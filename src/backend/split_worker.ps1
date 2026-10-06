<#
.SYNOPSIS
    Motor de división y separación de archivos PST (PST Splitter) con automatización
    Outlook COM / MAPI, telemetría en tiempo real y protocolo de parada segura
    (Graceful Shutdown) para Outlook Organizer TS.
#>
param (
    [string]$ConfigFile = "",
    [string]$AbortFile = ""
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

function Emit-ProgressTelemetry([string]$currentPstName, [int]$currentItems, [int]$totalItems, [int]$transferred, [DateTime]$startTime) {
    try {
        $now = [DateTime]::UtcNow
        $elapsedSec = ($now - $startTime).TotalSeconds
        $speed = if ($elapsedSec -gt 0) { [math]::Round($transferred / $elapsedSec, 1) } else { 0.0 }
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
$generatedPsts = @{}

try {
    Log-Message "Iniciando motor de separación de PSTs (PST Splitter)..."

    # 1. Cargar archivo de configuración JSON
    if (-not (Test-Path $ConfigFile)) {
        throw "El archivo de configuración no existe: $ConfigFile"
    }
    $rawConfig = [System.IO.File]::ReadAllText($ConfigFile, [System.Text.Encoding]::UTF8)
    $config = $rawConfig | ConvertFrom-Json

    $sourcePstPath = $config.source_pst_path
    $outputDir     = $config.output_dir
    $partitionMode = if ($config.partition_mode) { $config.partition_mode } else { "ByYear" }
    $transferMode  = if ($config.transfer_mode) { $config.transfer_mode } else { "Copy" }

    if (-not (Test-Path $sourcePstPath)) {
        throw "El archivo PST de origen no existe: $sourcePstPath"
    }

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

    # 3. Montar PST de origen
    $resolvedSourcePath = (Resolve-Path $sourcePstPath).Path
    $sourceStore = $null
    foreach ($st in $namespace.Stores) {
        try {
            if ($st.FilePath -and ((Resolve-Path $st.FilePath -ErrorAction SilentlyContinue).Path -eq $resolvedSourcePath)) {
                $sourceStore = $st
                break
            }
        } catch {}
    }

    if ($null -eq $sourceStore) {
        Log-Message "Montando PST de origen: $sourcePstPath"
        $namespace.AddStoreEx($resolvedSourcePath, 3) # 3 = olStoreUnicode
        foreach ($st in $namespace.Stores) {
            try {
                if ($st.FilePath -and ((Resolve-Path $st.FilePath -ErrorAction SilentlyContinue).Path -eq $resolvedSourcePath)) {
                    $sourceStore = $st
                    break
                }
            } catch {}
        }
        if ($sourceStore) {
            $storesToUnmount += $sourceStore
        }
    }

    if ($null -eq $sourceStore) {
        throw "No se pudo montar ni acceder al archivo PST de origen en MAPI: $sourcePstPath"
    }

    $sourceRoot = $sourceStore.GetRootFolder()
    $sourceBaseName = [System.IO.Path]::GetFileNameWithoutExtension($resolvedSourcePath)

    # 4. Descubrir carpetas de origen
    $candidateList = New-Object 'System.Collections.Generic.List[hashtable]'
    Collect-CandidateFolders $sourceRoot "" $candidateList $config
    $candidateFolders = $candidateList.ToArray()

    $totalCandidateItems = 0
    foreach ($cf in $candidateFolders) {
        try { $totalCandidateItems += $cf.Folder.Items.Count } catch {}
    }
    Log-Message "PST Origen: $totalCandidateItems correos encontrados en las carpetas a procesar."

    $startTime = [DateTime]::UtcNow
    Emit-ProgressTelemetry "Preparando partición..." 0 $totalCandidateItems 0 $startTime

    # 5. Procesar correos y generar archivos PST
    foreach ($cf in $candidateFolders) {
        if ($isAborted) { break }

        $srcFolder = $cf.Folder
        $folderItems = $null
        $itemCount = 0
        try {
            $folderItems = $srcFolder.Items
            $itemCount = $folderItems.Count
        } catch {
            Log-Message "No se pudieron leer elementos de '$($cf.RelPath)': $_" "WARN"
            continue
        }

        if ($itemCount -eq 0) { continue }
        Log-Message "Examinando carpeta '$($cf.RelPath)' ($itemCount correos)..."

        for ($idx = $itemCount; $idx -ge 1; $idx--) {
            if ($AbortFile -and (Test-Path $AbortFile)) {
                Log-Message "Señal de cancelación recibida. Deteniendo separación ordenadamente..." "WARN"
                $isAborted = $true
                break
            }

            $item = $null
            try {
                $item = $folderItems.Item($idx)
            } catch {
                $script:totalErrors++
                continue
            }

            if ($null -eq $item) { continue }

            # Obtener fecha de recepción
            $rcvd = $null
            try { $rcvd = $item.ReceivedTime } catch {}
            if ($null -eq $rcvd) {
                try { $rcvd = $item.SentOn } catch {}
            }
            if ($null -eq $rcvd) {
                $rcvd = Get-Date
            }

            $y = $rcvd.Year
            $m = $rcvd.Month

            # Filtrar por años y meses
            if ($null -ne $allowedYears -and -not $allowedYears.Contains([int]$y)) {
                $totalProcessed++
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                continue
            }
            if ($null -ne $allowedMonths -and -not $allowedMonths.Contains([int]$m)) {
                $totalProcessed++
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                continue
            }

            # Determinar nombre y ruta del PST destino
            $targetFileName = ""
            switch ($partitionMode) {
                "ByYear" {
                    $targetFileName = "${sourceBaseName}_$y.pst"
                }
                "ByYearMonth" {
                    $mPad = "{0:D2}" -f $m
                    $targetFileName = "${sourceBaseName}_${y}_$mPad.pst"
                }
                default {
                    $targetFileName = "${sourceBaseName}_filtrado.pst"
                }
            }
            $targetPstPath = Join-Path $outputDir $targetFileName

            # Montar o recuperar almacén de destino
            $targetStore = $null
            if ($openTargetStores.ContainsKey($targetPstPath)) {
                $targetStore = $openTargetStores[$targetPstPath]
            } else {
                Log-Message "Inicializando archivo PST de salida: $targetFileName"
                try {
                    $namespace.AddStoreEx($targetPstPath, 3) # 3 = olStoreUnicode
                } catch {
                    Log-Message "Error al crear/abrir PST '$targetPstPath': $_" "ERROR"
                    $script:totalErrors++
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                    continue
                }

                foreach ($st in $namespace.Stores) {
                    try {
                        if ($st.FilePath -and ((Resolve-Path $st.FilePath -ErrorAction SilentlyContinue).Path -eq (Resolve-Path $targetPstPath -ErrorAction SilentlyContinue).Path)) {
                            $targetStore = $st
                            break
                        }
                    } catch {}
                }

                if ($targetStore) {
                    $openTargetStores[$targetPstPath] = $targetStore
                    $storesToUnmount += $targetStore
                } else {
                    Log-Message "No se pudo recuperar el almacén MAPI para '$targetFileName'" "ERROR"
                    $script:totalErrors++
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                    continue
                }

                if (-not $generatedPsts.ContainsKey($targetPstPath)) {
                    $generatedPsts[$targetPstPath] = @{
                        file_path   = $targetPstPath
                        file_name   = $targetFileName
                        items_count = 0
                    }
                }
            }

            # Obtener carpeta equivalente en el PST destino
            $targetRoot = $null
            try { $targetRoot = $targetStore.GetRootFolder() } catch {}
            if ($null -eq $targetRoot) {
                $script:totalErrors++
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
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

            # Transferencia de ítem (Copiar o Mover)
            try {
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
                $totalProcessed++
                $generatedPsts[$targetPstPath].items_count++
            } catch {
                Log-Message "Error al transferir correo a '$targetFileName': $_" "WARN"
                $script:totalErrors++
            } finally {
                if ($null -ne $item) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                }
            }

            # Telemetría cada 15 correos o cada 200 ms
            $now = [DateTime]::UtcNow
            if (($totalProcessed % 15 -eq 0) -or (($now - $script:lastTelemetryTime).TotalMilliseconds -gt 200)) {
                Emit-ProgressTelemetry $targetFileName $totalProcessed $totalCandidateItems $totalTransferred $startTime
            }
        }

        if ($null -ne $folderItems) {
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($folderItems) | Out-Null } catch {}
        }
        if ($null -ne $srcFolder) {
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($srcFolder) | Out-Null } catch {}
        }
    }

    Emit-ProgressTelemetry "Finalizado" $totalProcessed $totalCandidateItems $totalTransferred $startTime

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

    # Desmontar almacenes generados y origen
    foreach ($st in $storesToUnmount) {
        try {
            $root = $st.GetRootFolder()
            Log-Message "Desmontando almacén: $($root.Name)"
            $namespace.RemoveStore($root)
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($root) | Out-Null } catch {}
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($st) | Out-Null } catch {}
        } catch {}
    }

    if ($weStartedOutlook -and $null -ne $outlook) {
        try { $outlook.Quit() } catch {}
    }

    if ($null -ne $namespace) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($namespace) | Out-Null } catch {}
    }
    if ($null -ne $outlook) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {}
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    if ($AbortFile -and (Test-Path $AbortFile)) {
        try { Remove-Item -Path $AbortFile -Force -ErrorAction SilentlyContinue } catch {}
    }

    # Emitir reporte final de separación
    Send-Telemetry @{
        type           = "split_finished"
        status         = if ($finalStatus) { $finalStatus } else { "completed" }
        total_items    = $totalTransferred
        generated_psts = $pstsReport
        errors         = $script:totalErrors
    }

    Log-Message "Proceso de separación de PSTs concluido con seguridad."
}
