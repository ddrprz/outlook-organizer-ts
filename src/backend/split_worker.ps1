<#
.SYNOPSIS
    Motor de división y separación de archivos PST (PST Splitter) con automatización
    Outlook COM / MAPI, telemetría continua en tiempo real, optimización de escaneo MAPI Table
    y protocolo de parada segura (Graceful Shutdown) para Outlook Organizer TS.
#>
param (
    [string]$ConfigFile = "",
    [string]$AbortFile = "",
    [string]$PauseFile = ""
)

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
$ErrorActionPreference    = "Stop"

try {
    [System.Diagnostics.Process]::GetCurrentProcess().PriorityClass = [System.Diagnostics.ProcessPriorityClass]::BelowNormal
} catch {}

function Send-Telemetry([hashtable]$Payload) {
    $json = $Payload | ConvertTo-Json -Compress
    [Console]::Out.WriteLine($json)
    [Console]::Out.Flush()
}

function Log-Message([string]$msg, [string]$level = "INFO") {
    $time = (Get-Date).ToString("HH:mm:ss")
    Send-Telemetry @{
        type      = "log"
        timestamp = $time
        level     = $level
        message   = "[$time] $msg"
    }
}

function Check-And-Emit-Split-Progress([string]$currentPstName, [int]$currentItems, [int]$totalItems, [int]$transferred, [DateTime]$startTime, [bool]$force = $false) {
    try {
        $now = [DateTime]::UtcNow
        $shouldEmit = $force -or ($currentItems % 25 -eq 0) -or ($null -eq $script:lastTelemetryTime) -or (($now - $script:lastTelemetryTime).TotalMilliseconds -gt 150) -or ($currentItems -eq $totalItems)
        if ($shouldEmit) {
            $elapsedSec = ($now - $startTime).TotalSeconds
            $speed = if ($elapsedSec -gt 0) { [math]::Round($currentItems / $elapsedSec, 1) } else { 0.0 }
            $remaining = [math]::Max(0, $totalItems - $currentItems)
            $eta = if ($speed -gt 0) { [math]::Round($remaining / $speed) } else { 0 }

            Send-Telemetry @{
                type         = "progress"
                pst_index    = 1
                pst_total    = 1
                pst_name     = $currentPstName
                item_current = $currentItems
                item_total   = $totalItems
                speed_mps    = $speed
                eta_seconds  = $eta
                imported     = $transferred
                duplicates   = 0
                errors       = $script:totalErrors
            }
            $script:lastTelemetryTime = $now
        }
    } catch {}
}

function Get-OrCreateFolder($parentFolder, [string]$subfolderName) {
    try {
        return $parentFolder.Folders.Item($subfolderName)
    } catch {
        return $parentFolder.Folders.Add($subfolderName)
    }
}

function Ensure-FolderHierarchy($destRoot, [string]$relPath) {
    if ([string]::IsNullOrWhiteSpace($relPath)) {
        return $destRoot
    }
    $parts = $relPath.Split([char[]]@('\', '/'), [System.StringSplitOptions]::RemoveEmptyEntries)
    $curr = $destRoot
    foreach ($p in $parts) {
        $curr = Get-OrCreateFolder $curr $p
    }
    return $curr
}

function Get-FolderType([string]$folderName) {
    $norm = $folderName.Trim().ToLower()
    if ($norm -eq "bandeja de entrada" -or $norm -eq "inbox") { return "inbox" }
    if ($norm -eq "elementos enviados" -or $norm -eq "sent items" -or $norm -eq "sent") { return "sent" }
    if ($norm -eq "elementos eliminados" -or $norm -eq "deleted items" -or $norm -eq "trash") { return "deleted" }
    return "custom"
}

function Collect-CandidateFolders($folder, [string]$parentRelPath, [System.Collections.Generic.List[hashtable]]$list, $config) {
    $currentName = $folder.Name
    $relPath = if ([string]::IsNullOrEmpty($parentRelPath)) { $currentName } else { "$parentRelPath\$currentName" }
    $fType = Get-FolderType $currentName

    $include = $false
    switch ($fType) {
        "inbox"   { $include = [bool]$config.include_inbox }
        "sent"    { $include = [bool]$config.include_sent }
        "deleted" { $include = [bool]$config.include_deleted }
        "custom"  { $include = [bool]$config.include_custom_folders }
    }

    if ($include) {
        $list.Add(@{
            Folder  = $folder
            Name    = $currentName
            RelPath = $relPath
            Type    = $fType
        })
    }

    try {
        foreach ($sub in $folder.Folders) {
            Collect-CandidateFolders $sub $relPath $list $config
        }
    } catch {}
}

function Get-UniquePstPath([string]$dir, [string]$baseFileName) {
    $nameWithoutExt = [System.IO.Path]::GetFileNameWithoutExtension($baseFileName)
    $ext = [System.IO.Path]::GetExtension($baseFileName)
    if ([string]::IsNullOrEmpty($ext)) { $ext = ".pst" }

    $candidatePath = [System.IO.Path]::Combine($dir, "$nameWithoutExt$ext")
    if (-not (Test-Path $candidatePath)) {
        return $candidatePath
    }

    $idx = 1
    while ($true) {
        $candidatePath = [System.IO.Path]::Combine($dir, "$nameWithoutExt ($idx)$ext")
        if (-not (Test-Path $candidatePath)) {
            return $candidatePath
        }
        $idx++
    }
}

function Get-OrMountTargetStore($namespace, [string]$targetPstPath, [ref]$storesToUnmountRef) {
    $fullTarget = [System.IO.Path]::GetFullPath($targetPstPath).ToLowerInvariant()
    $targetStore = $null

    foreach ($st in $namespace.Stores) {
        $p = ""
        try { $p = $st.FilePath } catch {}
        if ($p) {
            try {
                if ([System.IO.Path]::GetFullPath($p).ToLowerInvariant() -eq $fullTarget) {
                    $targetStore = $st
                    break
                }
            } catch {}
        }
    }

    if ($null -eq $targetStore) {
        try {
            $namespace.AddStoreEx($targetPstPath, 3) # 3 = olStoreUnicode
        } catch {
            Log-Message "Error al invocar AddStoreEx para '$targetPstPath': $_" "ERROR"
            return $null
        }

        for ($retry = 0; $retry -lt 5; $retry++) {
            Start-Sleep -Milliseconds 150
            foreach ($st in $namespace.Stores) {
                $p = ""
                try { $p = $st.FilePath } catch {}
                if ($p) {
                    try {
                        if ([System.IO.Path]::GetFullPath($p).ToLowerInvariant() -eq $fullTarget) {
                            $targetStore = $st
                            break
                        }
                    } catch {}
                }
            }
            if ($null -ne $targetStore) { break }
        }

        if ($null -ne $targetStore) {
            $storesToUnmountRef.Value += $targetStore
        }
    }

    return $targetStore
}

# --- INICIO DEL FLUJO PRINCIPAL ---
$outlook = $null
$namespace = $null
$sourceStore = $null
$sourceRoot = $null
$storesToUnmount = @()
$candidateFolders = $null
$weStartedOutlook = $false
$isAborted = $false
$script:totalErrors = 0
$totalTransferred = 0
$totalProcessed = 0
$script:lastTelemetryTime = [DateTime]::UtcNow
$openTargetStores = @{}
$targetFolderCache = @{}
$targetPathCache = @{}
$generatedPsts = @{}

try {
    Log-Message "Iniciando motor de separación de PSTs (PST Splitter)..."

    # 1. Cargar archivo de configuración JSON
    if (-not (Test-Path $ConfigFile)) {
        throw "El archivo de configuración no existe: $ConfigFile"
    }
    $rawConfig = Get-Content -Path $ConfigFile -Raw -Encoding UTF8
    $config = $rawConfig | ConvertFrom-Json
    $sourcePstPaths = @()
    if ($config.source_pst_paths -and $config.source_pst_paths.Count -gt 0) {
        $sourcePstPaths = $config.source_pst_paths
    } elseif ($config.source_pst_path) {
        $sourcePstPaths = @($config.source_pst_path)
    }

    $validSourcePaths = @()
    foreach ($sp in $sourcePstPaths) {
        if (Test-Path $sp) {
            $validSourcePaths += $sp
        } else {
            Log-Message "Advertencia: El archivo PST '$sp' no existe en disco. Omitiendo." "WARN"
        }
    }

    if ($validSourcePaths.Count -eq 0) {
        throw "No se encontró ningún archivo PST de origen válido para procesar."
    }

    $isMultiSource = ($validSourcePaths.Count -gt 1)
    $defaultPrefix = if ($isMultiSource) {
        "Consolidado"
    } else {
        [System.IO.Path]::GetFileNameWithoutExtension($validSourcePaths[0])
    }

    $outputDir     = $config.output_dir
    $partitionMode = if ($config.partition_mode) { $config.partition_mode } else { "ByYear" }
    $transferMode  = if ($config.transfer_mode) { $config.transfer_mode } else { "Copy" }

    if (-not (Test-Path $outputDir)) {
        try {
            New-Item -ItemType Directory -Path $outputDir -Force | Out-Null
            Log-Message "Directorio de destino creado: $outputDir"
        } catch {
            throw "No se pudo crear el directorio de destino '$outputDir': $_"
        }
    }

    # Conjuntos de años y meses permitidos
    $allowedYears = $null
    if ($config.selected_years -and $config.selected_years.Count -gt 0) {
        $allowedYears = New-Object 'System.Collections.Generic.HashSet[int]'
        foreach ($y in $config.selected_years) { [void]$allowedYears.Add([int]$y) }
    }

    $allowedMonths = $null
    if ($config.selected_months -and $config.selected_months.Count -gt 0) {
        $allowedMonths = New-Object 'System.Collections.Generic.HashSet[int]'
        foreach ($m in $config.selected_months) { [void]$allowedMonths.Add([int]$m) }
    }

    # 2. Inicializar sesión MAPI
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
        Log-Message "Conectado a la instancia en ejecución de Microsoft Outlook."
    } catch {
        Log-Message "Outlook no está abierto en segundo plano. Iniciando sesión COM dedicada..."
        $outlook = New-Object -ComObject Outlook.Application
        $weStartedOutlook = $true
    }

    $namespace = $outlook.GetNamespace("MAPI")
    if ($config.profile_name -and $config.profile_name.Trim() -ne "") {
        Log-Message "Iniciando sesión con el perfil: $($config.profile_name)"
        $namespace.Logon($config.profile_name, $null, $false, $true)
    } else {
        $namespace.Logon("", $null, $false, $true)
    }

    $unmountRef = [ref]$storesToUnmount
    $startTime = [DateTime]::UtcNow

    # Paso A: Descubrir y contar total de correos de todas las fuentes
    $sourcesData = @()
    $totalCandidateItems = 0

    foreach ($srcPath in $validSourcePaths) {
        if ($isAborted) { break }
        $st = Get-OrMountTargetStore $namespace $srcPath $unmountRef
        if ($null -ne $st) {
            $root = $st.GetRootFolder()
            $cList = New-Object 'System.Collections.Generic.List[hashtable]'
            Collect-CandidateFolders $root "" $cList $config
            $fCount = 0
            foreach ($cf in $cList) {
                try { $fCount += $cf.Folder.Items.Count } catch {}
            }
            $totalCandidateItems += $fCount
            $sourcesData += @{
                Store       = $st
                Path        = $srcPath
                Folders     = $cList.ToArray()
                ItemsCount  = $fCount
            }
        }
    }

    if ($totalCandidateItems -eq 0) {
        $totalCandidateItems = 1
    }

    Log-Message "Total correos a examinar en $($validSourcePaths.Count) archivo(s) PST: $totalCandidateItems"
    Check-And-Emit-Split-Progress "Iniciando partición..." 0 $totalCandidateItems 0 $startTime $true

    # Paso B: Procesar correos de cada archivo PST de origen
    foreach ($sData in $sourcesData) {
        if ($isAborted) { break }
        $sourceStore = $sData.Store
        $sourcePath  = $sData.Path
        $curBaseName = [System.IO.Path]::GetFileNameWithoutExtension($sourcePath)
        Log-Message "Procesando archivo: $curBaseName.pst ($($sData.Folders.Count) carpetas)..."

        foreach ($cf in $sData.Folders) {
            if ($isAborted) { break }

            $srcFolder = $cf.Folder
            $itemCount = 0
            try {
                $itemCount = $srcFolder.Items.Count
            } catch {
                Log-Message "No se pudieron leer elementos de '$($cf.RelPath)': $_" "WARN"
                continue
            }

            if ($itemCount -eq 0) { continue }
            Log-Message "Examinando carpeta '$($cf.RelPath)' ($itemCount correos)..."

            # Intento de escaneo ultra-rápido con MAPI Table
            $tableEntries = New-Object 'System.Collections.Generic.List[hashtable]'
            $useTable = $true

            try {
                $tbl = $srcFolder.GetTable()
                try { $tbl.Columns.RemoveAll() } catch {}
                try { $tbl.Columns.Add("EntryID") | Out-Null } catch {}
                try { $tbl.Columns.Add("ReceivedTime") | Out-Null } catch {}

                $batchSize = 5000
                while (-not $tbl.EndOfTable) {
                    $arr = $tbl.GetArray($batchSize)
                    $batchRows = $arr.GetLength(0)
                    if ($batchRows -eq 0) { break }
                    for ($r = 0; $r -lt $batchRows; $r++) {
                        $eId = $arr[$r, 0]
                        $rTime = $arr[$r, 1]
                        if ($eId) {
                            $tableEntries.Add(@{
                                EntryID      = $eId
                                ReceivedTime = $rTime
                            })
                        }
                    }
                }
                if ($null -ne $tbl) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tbl) | Out-Null } catch {}
                }
            } catch {
                $useTable = $false
            }

            if ($useTable -and $tableEntries.Count -gt 0) {
                # === RUTA A: PROCESAMIENTO ACELERADO EN RAM CON MAPI TABLE ===
                foreach ($entry in $tableEntries) {
                    if ($AbortFile -and (Test-Path $AbortFile)) {
                        Log-Message "Señal de cancelación recibida. Deteniendo separación ordenadamente..." "WARN"
                        $isAborted = $true
                        break
                    }

                    if ($PauseFile -and (Test-Path $PauseFile)) {
                        Log-Message "Proceso de separación en pausa. En espera de reanudación..." "WARN"
                        while ($PauseFile -and (Test-Path $PauseFile)) {
                            if ($AbortFile -and (Test-Path $AbortFile)) { break }
                            Start-Sleep -Milliseconds 200
                        }
                        if (-not ($AbortFile -and (Test-Path $AbortFile))) {
                            Log-Message "Proceso de separación reanudado." "INFO"
                        }
                    }

                    $totalProcessed++

                    $rcvd = $entry.ReceivedTime
                    if ($null -eq $rcvd -or -not ($rcvd -is [DateTime])) {
                        $rcvd = Get-Date
                    }

                    $y = $rcvd.Year
                    $m = $rcvd.Month

                    if ($null -ne $allowedYears -and -not $allowedYears.Contains([int]$y)) {
                        Check-And-Emit-Split-Progress "Omitiendo por año..." $totalProcessed $totalCandidateItems $totalTransferred $startTime
                        continue
                    }
                    if ($null -ne $allowedMonths -and -not $allowedMonths.Contains([int]$m)) {
                        Check-And-Emit-Split-Progress "Omitiendo por mes..." $totalProcessed $totalCandidateItems $totalTransferred $startTime
                        continue
                    }

                    # Determinar nombre y ruta única del PST destino
                    $targetKey = ""
                    $baseFileName = ""
                    switch ($partitionMode) {
                        "ByYear" {
                            $targetKey = "$y"
                            $baseFileName = "${defaultPrefix}_$y.pst"
                        }
                        "ByYearMonth" {
                            $mPad = "{0:D2}" -f $m
                            $targetKey = "${y}_$mPad"
                            $baseFileName = "${defaultPrefix}_${y}_$mPad.pst"
                        }
                        default {
                            $targetKey = "single"
                            $baseFileName = "${defaultPrefix}_filtrado.pst"
                        }
                    }

                    # Resolver nombre único con (1) (2) si ya existe en disco
                    $targetPstPath = ""
                    if ($targetPathCache.ContainsKey($targetKey)) {
                        $targetPstPath = $targetPathCache[$targetKey]
                    } else {
                        $targetPstPath = Get-UniquePstPath $outputDir $baseFileName
                        $targetPathCache[$targetKey] = $targetPstPath
                    }
                    $targetFileName = [System.IO.Path]::GetFileName($targetPstPath)

                    # Montar o recuperar almacén destino de forma segura
                    $targetStore = $null
                    if ($openTargetStores.ContainsKey($targetPstPath)) {
                        $targetStore = $openTargetStores[$targetPstPath]
                    } else {
                        Log-Message "Inicializando archivo PST de salida: $targetFileName"
                        $targetStore = Get-OrMountTargetStore $namespace $targetPstPath $unmountRef
                        if ($null -eq $targetStore) {
                            $script:totalErrors++
                            continue
                        }
                        $openTargetStores[$targetPstPath] = $targetStore
                        $generatedPsts[$targetPstPath] = @{
                            file_path   = $targetPstPath
                            file_name   = $targetFileName
                            items_count = 0
                        }
                    }

                    # Carpeta equivalente en el PST destino
                    $targetRoot = $null
                    try { $targetRoot = $targetStore.GetRootFolder() } catch {}
                    if ($null -eq $targetRoot) {
                        $script:totalErrors++
                        continue
                    }

                    $cacheKey = "$targetPstPath|$($cf.RelPath)"
                    $destFolder = $null
                    if ($targetFolderCache.ContainsKey($cacheKey)) {
                        $destFolder = $targetFolderCache[$cacheKey]
                    } else {
                        $destFolder = Ensure-FolderHierarchy $targetRoot $cf.RelPath
                        $targetFolderCache[$cacheKey] = $destFolder
                    }

                    # Cargar el ítem específico por EntryID y transferir
                    $item = $null
                    try {
                        $item = $namespace.GetItemFromID($entry.EntryID, $sourceStore.StoreID)
                        if ($null -ne $item) {
                            if ($transferMode -eq "Move") {
                                $item.Move($destFolder) | Out-Null
                            } else {
                                $copy = $item.Copy()
                                $copy.Move($destFolder) | Out-Null
                                if ($null -ne $copy) {
                                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                                }
                            }
                            $totalTransferred++
                            $generatedPsts[$targetPstPath].items_count++
                        }
                    } catch {
                        Log-Message "Error al transferir correo a '$targetFileName': $_" "WARN"
                        $script:totalErrors++
                    } finally {
                        if ($null -ne $item) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                        }
                    }

                    Check-And-Emit-Split-Progress $targetFileName $totalProcessed $totalCandidateItems $totalTransferred $startTime
                }
            } else {
                # === RUTA B: FALLBACK TRADICIONAL SI LA CARPETA NO SOPORTA MAPI TABLE ===
                $folderItems = $null
                try { $folderItems = $srcFolder.Items } catch {}
                if ($null -ne $folderItems) {
                    for ($idx = $itemCount; $idx -ge 1; $idx--) {
                        if ($AbortFile -and (Test-Path $AbortFile)) {
                            Log-Message "Señal de cancelación recibida. Deteniendo separación ordenadamente..." "WARN"
                            $isAborted = $true
                            break
                        }

                        if ($PauseFile -and (Test-Path $PauseFile)) {
                            Log-Message "Proceso de separación en pausa. En espera de reanudación..." "WARN"
                            while ($PauseFile -and (Test-Path $PauseFile)) {
                                if ($AbortFile -and (Test-Path $AbortFile)) { break }
                                Start-Sleep -Milliseconds 200
                            }
                            if (-not ($AbortFile -and (Test-Path $AbortFile))) {
                                Log-Message "Proceso de separación reanudado." "INFO"
                            }
                        }

                        $totalProcessed++

                        $item = $null
                        try { $item = $folderItems.Item($idx) } catch { $script:totalErrors++; continue }
                        if ($null -eq $item) { continue }

                        $rcvd = $null
                        try { $rcvd = $item.ReceivedTime } catch {}
                        if ($null -eq $rcvd) { try { $rcvd = $item.SentOn } catch {} }
                        if ($null -eq $rcvd) { $rcvd = Get-Date }

                        $y = $rcvd.Year
                        $m = $rcvd.Month

                        if ($null -ne $allowedYears -and -not $allowedYears.Contains([int]$y)) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                            Check-And-Emit-Split-Progress "Omitiendo por año..." $totalProcessed $totalCandidateItems $totalTransferred $startTime
                            continue
                        }
                        if ($null -ne $allowedMonths -and -not $allowedMonths.Contains([int]$m)) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                            Check-And-Emit-Split-Progress "Omitiendo por mes..." $totalProcessed $totalCandidateItems $totalTransferred $startTime
                            continue
                        }

                        $targetKey = if ($partitionMode -eq "ByYear") { "$y" } elseif ($partitionMode -eq "ByYearMonth") { "${y}_{0:D2}" -f $m } else { "single" }
                        $baseFileName = if ($partitionMode -eq "ByYear") { "${defaultPrefix}_$y.pst" } elseif ($partitionMode -eq "ByYearMonth") { "${defaultPrefix}_${y}_{0:D2}.pst" -f $m } else { "${defaultPrefix}_filtrado.pst" }

                        $targetPstPath = if ($targetPathCache.ContainsKey($targetKey)) { $targetPathCache[$targetKey] } else { $p = Get-UniquePstPath $outputDir $baseFileName; $targetPathCache[$targetKey] = $p; $p }
                        $targetFileName = [System.IO.Path]::GetFileName($targetPstPath)

                        $targetStore = if ($openTargetStores.ContainsKey($targetPstPath)) { $openTargetStores[$targetPstPath] } else {
                            Log-Message "Inicializando archivo PST de salida: $targetFileName"
                            $st = Get-OrMountTargetStore $namespace $targetPstPath $unmountRef
                            if ($st) { $openTargetStores[$targetPstPath] = $st; $generatedPsts[$targetPstPath] = @{ file_path = $targetPstPath; file_name = $targetFileName; items_count = 0 }; $st } else { $null }
                        }

                        if ($null -eq $targetStore) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                            $script:totalErrors++
                            continue
                        }

                        $targetRoot = $null
                        try { $targetRoot = $targetStore.GetRootFolder() } catch {}
                        $cacheKey = "$targetPstPath|$($cf.RelPath)"
                        $destFolder = if ($targetFolderCache.ContainsKey($cacheKey)) { $targetFolderCache[$cacheKey] } else { $df = Ensure-FolderHierarchy $targetRoot $cf.RelPath; $targetFolderCache[$cacheKey] = $df; $df }

                        try {
                            if ($transferMode -eq "Move") {
                                $item.Move($destFolder) | Out-Null
                            } else {
                                $copy = $item.Copy()
                                $copy.Move($destFolder) | Out-Null
                                if ($null -ne $copy) { try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {} }
                            }
                            $totalTransferred++
                            $generatedPsts[$targetPstPath].items_count++
                        } catch {
                            $script:totalErrors++
                        } finally {
                            if ($null -ne $item) { try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {} }
                        }

                        Check-And-Emit-Split-Progress $targetFileName $totalProcessed $totalCandidateItems $totalTransferred $startTime
                    }
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($folderItems) | Out-Null } catch {}
                }
            }

            if ($null -ne $srcFolder) {
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($srcFolder) | Out-Null } catch {}
            }
        }
    }

    Check-And-Emit-Split-Progress "Finalizado" $totalProcessed $totalCandidateItems $totalTransferred $startTime $true

    $finalStatus = if ($isAborted) { "aborted" } else { "completed" }
    Log-Message "Separación finalizada. Estado: $finalStatus. Total correos transferidos: $totalTransferred."
}
catch {
    Log-Message "Error crítico en proceso de separación: $_" "ERROR"
    $finalStatus = "failed"
    $script:totalErrors++
}
finally {
    Log-Message "Desmontando archivos PST y liberando punteros COM..."

    # Calcular tamaños en disco de los PSTs generados
    $pstsReport = @()
    foreach ($k in $generatedPsts.Keys) {
        $info = $generatedPsts[$k]
        $sizeMb = 0.0
        if (Test-Path $info.file_path) {
            try {
                $fi = Get-Item $info.file_path
                $sizeMb = [math]::Round($fi.Length / 1MB, 2)
            } catch {}
        }
        $pstsReport += @{
            file_path   = $info.file_path
            file_name   = $info.file_name
            items_count = [int]$info.items_count
            size_mb     = [double]$sizeMb
        }
    }

    # Desmontar almacenes PST creados y el de origen
    foreach ($st in $storesToUnmount) {
        try {
            if ($null -ne $st -and $null -ne $namespace) {
                $namespace.RemoveStore($st.GetRootFolder()) | Out-Null
            }
        } catch {}
        if ($null -ne $st) {
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($st) | Out-Null } catch {}
        }
    }

    if ($null -ne $sourceRoot) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($sourceRoot) | Out-Null } catch {}
    }
    if ($null -ne $namespace) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($namespace) | Out-Null } catch {}
    }
    if ($weStartedOutlook -and ($null -ne $outlook)) {
        try { $outlook.Quit() | Out-Null } catch {}
    }
    if ($null -ne $outlook) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {}
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
    [System.GC]::Collect()

    Log-Message "Almacenes desmontados correctamente."

    Send-Telemetry @{
        type            = "split_finished"
        status          = $finalStatus
        total_items     = [uint64]$totalTransferred
        total_extracted = [uint64]$totalTransferred
        generated_psts  = $pstsReport
        errors          = [uint64]$script:totalErrors
    }

    if ($PauseFile -and (Test-Path $PauseFile)) {
        try { Remove-Item -Path $PauseFile -Force -ErrorAction SilentlyContinue } catch {}
    }
}
