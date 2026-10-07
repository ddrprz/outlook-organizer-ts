<#
.SYNOPSIS
    Motor de migración e importación Outlook COM / MAPI de alto rendimiento con
    telemetría en tiempo real, deduplicación, enrutamiento temporal y protocolo
    de parada segura (Graceful Shutdown) para Outlook Organizer TS.
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

function Emit-ProgressTelemetry([int]$currentPstIdx, [int]$totalPsts, [string]$pstName, [int]$currentItems, [int]$totalItems, [DateTime]$startTime) {
    try {
        $now = [DateTime]::UtcNow
        $elapsedSec = ($now - $startTime).TotalSeconds
        $speed = if ($elapsedSec -gt 0) { [math]::Round($currentItems / $elapsedSec, 1) } else { 0.0 }
        $remaining = [math]::Max(0, $totalItems - $currentItems)
        $eta = if ($speed -gt 0) { [math]::Round($remaining / $speed) } else { 0 }

        Send-Telemetry @{
            type         = "progress"
            pst_index    = $currentPstIdx
            pst_total    = $totalPsts
            pst_name     = $pstName
            item_current = $currentItems
            item_total   = $totalItems
            speed_mps    = $speed
            eta_seconds  = $eta
            imported     = $script:totalImported
            duplicates   = $script:totalDuplicates
            errors       = $script:totalErrors
        }
        $script:lastTelemetryTime = $now
    } catch {}
}

function Check-And-Emit-Progress([int]$currentPstIdx, [int]$totalPsts, [string]$pstName, [int]$currentItems, [int]$totalItems, [DateTime]$startTime, [bool]$force = $false) {
    try {
        $now = [DateTime]::UtcNow
        $shouldEmit = $force -or ($currentItems % 15 -eq 0) -or ($null -eq $script:lastTelemetryTime) -or (($now - $script:lastTelemetryTime).TotalMilliseconds -gt 200) -or ($currentItems -eq $totalItems)
        if ($shouldEmit) {
            Emit-ProgressTelemetry $currentPstIdx $totalPsts $pstName $currentItems $totalItems $startTime
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

function Get-FolderType([string]$folderName) {
    $norm = $folderName.Trim().ToLower()
    if ($norm -eq "bandeja de entrada" -or $norm -eq "inbox") {
        return "inbox"
    }
    if ($norm -eq "elementos enviados" -or $norm -eq "sent items" -or $norm -eq "sent") {
        return "sent"
    }
    if ($norm -eq "elementos eliminados" -or $norm -eq "deleted items" -or $norm -eq "trash") {
        return "deleted"
    }
    return "custom"
}

function Get-DestBaseFolder($destStore, [string]$folderType, [string]$customName) {
    $root = $destStore.GetRootFolder()
    switch ($folderType) {
        "inbox" {
            try { return $destStore.GetDefaultFolder(6) } catch {}
            foreach ($f in $root.Folders) {
                if ($f.Name -match "^(Bandeja de entrada|Inbox)$") { return $f }
            }
            return Get-OrCreateFolder $root "Bandeja de entrada"
        }
        "sent" {
            try { return $destStore.GetDefaultFolder(5) } catch {}
            foreach ($f in $root.Folders) {
                if ($f.Name -match "^(Elementos enviados|Sent Items)$") { return $f }
            }
            return Get-OrCreateFolder $root "Elementos enviados"
        }
        "deleted" {
            try { return $destStore.GetDefaultFolder(3) } catch {}
            foreach ($f in $root.Folders) {
                if ($f.Name -match "^(Elementos eliminados|Deleted Items)$") { return $f }
            }
            return Get-OrCreateFolder $root "Elementos eliminados"
        }
        default {
            return Get-OrCreateFolder $root $customName
        }
    }
}

function Get-DestFolderByPath($destStore, [string]$relPath, [string]$folderType) {
    if (-not $relPath -or $relPath.Trim() -eq "") {
        return Get-DestBaseFolder $destStore $folderType ""
    }
    $parts = $relPath -split "[\\/]"
    if ($parts.Length -eq 0 -or [string]::IsNullOrWhiteSpace($parts[0])) {
        return Get-DestBaseFolder $destStore $folderType ""
    }
    $topName = $parts[0]
    $topType = Get-FolderType $topName
    $currentDest = Get-DestBaseFolder $destStore $topType $topName
    for ($i = 1; $i -lt $parts.Length; $i++) {
        $subName = $parts[$i]
        if (-not [string]::IsNullOrWhiteSpace($subName)) {
            $currentDest = Get-OrCreateFolder $currentDest $subName
        }
    }
    return $currentDest
}

function Get-CachedDestBaseFolder($destStore, [string]$relPath, [string]$folderType) {
    $cacheKey = "$relPath|$folderType"
    if ($script:destFolderCache.ContainsKey($cacheKey)) {
        return $script:destFolderCache[$cacheKey]
    }
    $f = Get-DestFolderByPath $destStore $relPath $folderType
    $script:destFolderCache[$cacheKey] = $f
    return $f
}

function Clear-FolderCaches {
    if ($script:destFolderCache) {
        foreach ($k in @($script:destFolderCache.Keys)) {
            $f = $script:destFolderCache[$k]
            if ($null -ne $f) {
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($f) | Out-Null } catch {}
            }
        }
        $script:destFolderCache.Clear()
    }
    if ($script:dateFolderCache) {
        foreach ($k in @($script:dateFolderCache.Keys)) {
            $f = $script:dateFolderCache[$k]
            if ($null -ne $f) {
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($f) | Out-Null } catch {}
            }
        }
        $script:dateFolderCache.Clear()
    }
}

function Collect-CandidateFolders($folder, [string]$currentPath, [System.Collections.Generic.List[hashtable]]$collector, $selectedPathsSet, [bool]$hasSubpaths, $config) {
    foreach ($sub in $folder.Folders) {
        $subName = $sub.Name
        # Ignorar carpetas internas de sistema irrelevantes
        if ($subName -match "^(Yammer Root|Quick Step Settings|Conversation History|Social Activity Provider RSS Feeds|Sync Issues|Problemas de sincronización|Conflictos|Errores locales|Fallos del servidor)$") {
            continue
        }

        $subPath = if ($currentPath -eq "") { $subName } else { "$currentPath\$subName" }
        $fType = Get-FolderType $subName

        $shouldInclude = $false
        if ($selectedPathsSet -and $selectedPathsSet.Count -gt 0) {
            if ($hasSubpaths) {
                # Modo árbol detallado: coincidencia exacta de ruta
                if ($selectedPathsSet.Contains($subPath)) {
                    $shouldInclude = $true
                }
            } else {
                # Modo básico: coincidencia por top-level o ancestro raíz
                $topAncestor = ($subPath -split "[\\/]")[0]
                if ($selectedPathsSet.Contains($topAncestor) -or $selectedPathsSet.Contains($subName)) {
                    $shouldInclude = $true
                }
            }
        } else {
            # Compatibilidad con flags heredados (legacy flags)
            switch ($fType) {
                "inbox"   { $shouldInclude = if ($config) { [bool]$config.include_inbox } else { $true } }
                "sent"    { $shouldInclude = if ($config) { [bool]$config.include_sent } else { $true } }
                "deleted" { $shouldInclude = if ($config) { [bool]$config.include_deleted } else { $false } }
                "custom"  { $shouldInclude = if ($config) { [bool]$config.include_custom_folders } else { $true } }
                default   { $shouldInclude = $true }
            }
        }

        if ($shouldInclude) {
            $collector.Add(@{
                Folder  = $sub
                Type    = $fType
                Name    = $subName
                RelPath = $subPath
            })
        }

        # Continuar la recolección recursiva en subcarpetas
        Collect-CandidateFolders $sub $subPath $collector $selectedPathsSet $hasSubpaths $config
    }
}

function Index-TargetFolderItems($targetFolder, [System.Collections.Generic.HashSet[string]]$seenSet, [bool]$deepScan) {
    try {
        $indexedWithTable = $false
        $tbl = $null
        try {
            $tbl = $targetFolder.GetTable()
            if ($tbl) {
                try { $tbl.Columns.RemoveAll() } catch {}
                try { $tbl.Columns.Add("Subject") | Out-Null } catch {}
                try { $tbl.Columns.Add("SenderEmailAddress") | Out-Null } catch {}
                try { $tbl.Columns.Add("ReceivedTime") | Out-Null } catch {}
                try { $tbl.Columns.Add("http://schemas.microsoft.com/mapi/proptag/0x1035001E") | Out-Null } catch {}

                $batchSize = 5000
                while (-not $tbl.EndOfTable) {
                    $arr = $tbl.GetArray($batchSize)
                    $rowsInBatch = $arr.GetLength(0)
                    if ($rowsInBatch -eq 0) { break }

                    for ($i = 0; $i -lt $rowsInBatch; $i++) {
                        $s = $arr[$i, 0]
                        $snd = $arr[$i, 1]
                        $dt = $arr[$i, 2]
                        $mid = $arr[$i, 3]

                        if ($mid -and ($mid -is [string]) -and $mid.Trim() -ne "") {
                            [void]$seenSet.Add($mid.Trim())
                        }
                        if ($s -or $snd -or $dt) {
                            $dtStr = if ($dt -and ($dt -is [DateTime])) { $dt.ToString('yyyyMMddHHmmss') } else { "" }
                            $cKey = "$s|$snd|$dtStr"
                            [void]$seenSet.Add($cKey)
                        }
                    }
                }
                $indexedWithTable = $true
            }
        } catch {
            $indexedWithTable = $false
        } finally {
            if ($null -ne $tbl) {
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tbl) | Out-Null } catch {}
            }
        }

        # Fallback a iteración clásica únicamente si GetTable no estuvo disponible
        if (-not $indexedWithTable) {
            $items = $targetFolder.Items
            $count = $items.Count
            for ($k = 1; $k -le $count; $k++) {
                $existing = $null
                try {
                    $existing = $items.Item($k)
                    $mid = $null
                    try {
                        $mid = $existing.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x1035001E")
                    } catch {}
                    if ($mid -and $mid.Trim() -ne "") {
                        [void]$seenSet.Add($mid.Trim())
                    }
                    $subj = $existing.Subject
                    $sender = $existing.SenderEmailAddress
                    $t = $null
                    try { $t = $existing.ReceivedTime } catch {}
                    if ($null -ne $t) {
                        $cKey = "$subj|$sender|$($t.ToString('yyyyMMddHHmmss'))"
                        [void]$seenSet.Add($cKey)
                    }
                } catch {}
                finally {
                    if ($null -ne $existing) {
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($existing) | Out-Null } catch {}
                    }
                }
            }
        }

        if ($deepScan) {
            $subFolders = $null
            try {
                $subFolders = $targetFolder.Folders
                foreach ($sub in $subFolders) {
                    Index-TargetFolderItems $sub $seenSet $deepScan
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($sub) | Out-Null } catch {}
                }
            } catch {}
            finally {
                if ($null -ne $subFolders) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($subFolders) | Out-Null } catch {}
                }
            }
        }
    } catch {}
}

$monthNames = @{
    1  = "01-Enero"
    2  = "02-Febrero"
    3  = "03-Marzo"
    4  = "04-Abril"
    5  = "05-Mayo"
    6  = "06-Junio"
    7  = "07-Julio"
    8  = "08-Agosto"
    9  = "09-Septiembre"
    10 = "10-Octubre"
    11 = "11-Noviembre"
    12 = "12-Diciembre"
}

Log-Message "Iniciando worker de PowerShell con enlace MAPI..."

# 1. Cargar archivo de configuración estructurado
$config = $null
if ($ConfigFile -and (Test-Path $ConfigFile)) {
    try {
        $raw = Get-Content -Path $ConfigFile -Raw -Encoding UTF8
        $config = $raw | ConvertFrom-Json
        Log-Message "Configuración cargada correctamente."
    } catch {
        Log-Message "Advertencia al leer configuración: $_" "WARN"
    }
}

# Inicializar conjuntos de filtros para búsqueda O(1) de años y meses permitidos
$allowedYearsSet = $null
if ($config -and $config.specific_years -and $config.specific_years.Count -gt 0) {
    $allowedYearsSet = New-Object 'System.Collections.Generic.HashSet[int]'
    foreach ($y in $config.specific_years) {
        if ($y) { [void]$allowedYearsSet.Add([int]$y) }
    }
} elseif ($config -and $config.specific_year) {
    $allowedYearsSet = New-Object 'System.Collections.Generic.HashSet[int]'
    [void]$allowedYearsSet.Add([int]$config.specific_year)
}

$allowedMonthsSet = $null
if ($config -and $config.specific_months -and $config.specific_months.Count -gt 0) {
    $allowedMonthsSet = New-Object 'System.Collections.Generic.HashSet[int]'
    foreach ($m in $config.specific_months) {
        if ($m) { [void]$allowedMonthsSet.Add([int]$m) }
    }
} elseif ($config -and $config.specific_month) {
    $allowedMonthsSet = New-Object 'System.Collections.Generic.HashSet[int]'
    [void]$allowedMonthsSet.Add([int]$config.specific_month)
}

$outlook = $null
$namespace = $null
$storesToUnmount = @()
$totalImported = 0
$totalDuplicates = 0
$totalErrors = 0
$isAborted = $false
$globalProcessedItems = 0
$globalTotalItems = 0
$startTime = [DateTime]::UtcNow
$lastTelemetryTime = [DateTime]::UtcNow
$targetSets = @{}
$destFolderCache = @{}
$dateFolderCache = @{}
$consecutiveThrottles = 0
$processedItemsList = New-Object 'System.Collections.Generic.List[hashtable]'

$weStartedOutlook = $false
try {
    # Configurar límites de tamaño máximo para archivos PST en el registro (47.5 GB = 48,640 MB)
    $maxLargeMb = 48640   # 47.5 GB exactos (47.5 * 1024 MB)
    $warnLargeMb = 46080  # 45.0 GB advertencia (45 * 1024 MB, margen seguro de 2.5 GB)

    $pstRegVersions = @("16.0", "15.0", "14.0", "12.0", "11.0")
    $regHives = @(
        "HKCU:\Software\Microsoft\Office",
        "HKCU:\Software\Policies\Microsoft\Office",
        "HKLM:\Software\Microsoft\Office",
        "HKLM:\Software\Policies\Microsoft\Office"
    )
    foreach ($ver in $pstRegVersions) {
        foreach ($hive in $regHives) {
            $regPstPath = "$hive\$ver\Outlook\PST"
            try {
                if (-not (Test-Path $regPstPath)) {
                    New-Item -Path $regPstPath -Force -ErrorAction SilentlyContinue | Out-Null
                }
                Set-ItemProperty -Path $regPstPath -Name "MaxLargeFileSize" -Value $maxLargeMb -Type DWord -Force -ErrorAction SilentlyContinue
                Set-ItemProperty -Path $regPstPath -Name "WarnLargeFileSize" -Value $warnLargeMb -Type DWord -Force -ErrorAction SilentlyContinue
            } catch {}
        }
    }

    # 2. Conectar a Outlook COM en modo STA
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
        Log-Message "Enlace establecido con instancia activa de Outlook."
    } catch {
        $outlook = New-Object -ComObject Outlook.Application
        $weStartedOutlook = $true
        Log-Message "Nueva instancia de Outlook COM iniciada en segundo plano."
    }

    $namespace = $outlook.GetNamespace("MAPI")

    # 3. Inicialización MAPI (reutilizar sesión o logon)
    $profileToUse = if ($config -and $config.profile_name) { $config.profile_name } else { "" }
    if ($profileToUse -and $profileToUse.Trim() -ne "") {
        Log-Message "Conectando sesión MAPI al perfil: $profileToUse"
        $namespace.Logon($profileToUse.Trim(), "", $false, $false)
        Log-Message "Sesión MAPI lista con perfil: $profileToUse."
    } else {
        $activeProfile = $null
        try { $activeProfile = $namespace.CurrentProfileName } catch {}

        if ($activeProfile -and $activeProfile.Trim() -ne "") {
            Log-Message "Sesión MAPI activa reutilizada (Perfil: $activeProfile)."
        } else {
            Log-Message "Iniciando sesión MAPI con perfil predeterminado..."
            try {
                $namespace.Logon("", "", $false, $false)
                Log-Message "Sesión MAPI iniciada correctamente."
            } catch {
                Log-Message "Sesión MAPI activa confirmada."
            }
        }
    }

    # 4. Localizar buzón de destino
    $destStore = $null
    $targetName = if ($config -and $config.target_mailboxes -and $config.target_mailboxes.Count -gt 0) {
        $config.target_mailboxes[0]
    } else {
        ""
    }

    if ($targetName -and $targetName.Trim() -ne "") {
        foreach ($s in $namespace.Stores) {
            if ($s.DisplayName -like "*$targetName*" -or ($s.ExchangeStoreType -ne $null -and $s.DisplayName -eq $targetName)) {
                $destStore = $s
                break
            }
        }
    }

    if ($null -eq $destStore) {
        $destStore = $namespace.DefaultStore
        Log-Message "Buzón destino: usando almacén predeterminado '$($destStore.DisplayName)'."
    } else {
        Log-Message "Buzón destino encontrado: '$($destStore.DisplayName)'."
    }

    # 5. Determinar lista de archivos PST a procesar
    $pstList = @()
    if ($config -and $config.psts -and $config.psts.Count -gt 0) {
        $pstList = $config.psts
    }

    if ($pstList.Count -eq 0) {
        Log-Message "No se especificaron archivos PST para procesar." "WARN"
    }

    $totalPsts = $pstList.Count

    # 6. Iterar sobre cada archivo PST
    for ($pIdx = 0; $pIdx -lt $totalPsts; $pIdx++) {
        $pstPath = $pstList[$pIdx]
        $pstName = [System.IO.Path]::GetFileName($pstPath)
        if (-not $pstName) { $pstName = $pstPath }

        if (-not (Test-Path $pstPath)) {
            Log-Message "Archivo PST no encontrado en disco: $pstPath" "ERROR"
            $totalErrors++
            continue
        }

        Log-Message "Montando archivo PST [$($pIdx + 1)/$totalPsts]: $pstName"

        # Verificar si ya está montado
        $pstStore = $null
        foreach ($s in $namespace.Stores) {
            if ($s.FilePath -and ($s.FilePath.Trim().ToLower() -eq $pstPath.Trim().ToLower())) {
                $pstStore = $s
                break
            }
        }

        $wasMountedByUs = $false
        if ($pstStore) {
            Log-Message "PST ya cargado en sesión MAPI: $($pstStore.DisplayName)."
        } else {
            try {
                $namespace.AddStoreEx($pstPath, 1) # 1 = olStoreDefault (Unicode)
                Start-Sleep -Milliseconds 150
                foreach ($s in $namespace.Stores) {
                    if ($s.FilePath -and ($s.FilePath.Trim().ToLower() -eq $pstPath.Trim().ToLower())) {
                        $pstStore = $s
                        break
                    }
                }
                if ($pstStore) {
                    $wasMountedByUs = $true
                    $storesToUnmount += $pstStore
                    Log-Message "PST montado exitosamente en MAPI."
                }
            } catch {
                Log-Message "Error al montar PST '$pstName': $_" "ERROR"
                $totalErrors++
                continue
            }
        }

        if ($null -eq $pstStore) {
            Log-Message "No se pudo acceder al almacén MAPI de: $pstName" "ERROR"
            $totalErrors++
            continue
        }

        # 7. Identificar carpetas de origen en el PST
        $pstRoot = $null
        try {
            $pstRoot = $pstStore.GetRootFolder()
        } catch {
            Log-Message "No se pudo obtener carpeta raíz de: $pstName" "ERROR"
            $totalErrors++
            continue
        }

        $selectedPathsSet = $null
        $hasSubpaths = $false
        if ($config -and $config.selected_folder_paths -and $config.selected_folder_paths.Count -gt 0) {
            $selectedPathsSet = New-Object 'System.Collections.Generic.HashSet[string]' ([System.StringComparer]::OrdinalIgnoreCase)
            foreach ($sp in $config.selected_folder_paths) {
                if ($sp -and $sp.Trim() -ne "") {
                    [void]$selectedPathsSet.Add($sp.Trim())
                    if ($sp.Contains("\") -or $sp.Contains("/")) {
                        $hasSubpaths = $true
                    }
                }
            }
        }

        $candidateList = New-Object 'System.Collections.Generic.List[hashtable]'
        Collect-CandidateFolders $pstRoot "" $candidateList $selectedPathsSet $hasSubpaths $config
        $candidateFolders = $candidateList.ToArray()

        # Pre-conteo de elementos para barra de progreso precisa
        $totalPstItems = 0
        foreach ($cf in $candidateFolders) {
            try { $totalPstItems += $cf.Folder.Items.Count } catch {}
        }

        Log-Message "PST '$pstName': $totalPstItems correos encontrados en carpetas seleccionadas."

        Send-Telemetry @{
            type         = "progress"
            pst_index    = $pIdx + 1
            pst_total    = $totalPsts
            pst_name     = $pstName
            item_current = 0
            item_total   = $totalPstItems
            speed_mps    = 0.0
            eta_seconds  = 0
            imported     = $totalImported
            duplicates   = $totalDuplicates
            errors       = $totalErrors
        }

        $pstProcessedCount = 0
        $lastTelemetryTime = [DateTime]::UtcNow

        # 8. Procesar cada carpeta seleccionada
        foreach ($cf in $candidateFolders) {
            if ($isAborted) { break }

            $srcFolder = $cf.Folder
            $folderItems = $null
            $itemCount = 0
            try {
                $folderItems = $srcFolder.Items
                $itemCount = $folderItems.Count
            } catch {
                Log-Message "No se pudieron leer elementos de carpeta '$($cf.Name)': $_" "WARN"
                continue
            }

            Log-Message "Procesando carpeta '$($cf.RelPath)' ($itemCount correos)..."

            # Pre-resolver carpeta base de destino una sola vez para toda la carpeta de origen
            $baseDest = Get-CachedDestBaseFolder $destStore $cf.RelPath $cf.Type
            $baseDestId = $null
            try { $baseDestId = $baseDest.EntryID } catch {}

            # Intentar extracción ultra-rápida por lotes con MAPI Table
            $tableEntries = New-Object 'System.Collections.Generic.List[hashtable]'
            $useTable = $true

            try {
                $tbl = $srcFolder.GetTable()
                try { $tbl.Columns.RemoveAll() } catch {}
                try { $tbl.Columns.Add("EntryID") | Out-Null } catch {}
                try { $tbl.Columns.Add("ReceivedTime") | Out-Null } catch {}
                try { $tbl.Columns.Add("Subject") | Out-Null } catch {}
                try { $tbl.Columns.Add("SenderEmailAddress") | Out-Null } catch {}
                try { $tbl.Columns.Add("http://schemas.microsoft.com/mapi/proptag/0x1035001E") | Out-Null } catch {}
                try { $tbl.Columns.Add("Size") | Out-Null } catch {}
                try { $tbl.Columns.Add("SentOn") | Out-Null } catch {}

                $batchSize = 5000
                while (-not $tbl.EndOfTable) {
                    $arr = $tbl.GetArray($batchSize)
                    $batchRows = $arr.GetLength(0)
                    if ($batchRows -eq 0) { break }
                    for ($r = 0; $r -lt $batchRows; $r++) {
                        $eId = $arr[$r, 0]
                        if ($eId) {
                            $rTime = $arr[$r, 1]
                            if ($null -eq $rTime -or -not ($rTime -is [DateTime]) -or $rTime.Year -lt 1980) {
                                $rTime = $arr[$r, 6]
                            }
                            if ($null -eq $rTime -or -not ($rTime -is [DateTime])) {
                                $rTime = Get-Date
                            }

                            $eSubj = $arr[$r, 2]
                            $eSender = $arr[$r, 3]
                            $eMid = $arr[$r, 4]
                            $eSize = $arr[$r, 5]

                            $tableEntries.Add(@{
                                EntryID      = $eId
                                ReceivedTime = $rTime
                                Subject      = $eSubj
                                Sender       = $eSender
                                MessageID    = $eMid
                                Size         = $eSize
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
                        Log-Message "Señal de parada segura recibida. Deteniendo proceso ordenadamente..." "WARN"
                        $isAborted = $true
                        break
                    }

                    if ($PauseFile -and (Test-Path $PauseFile)) {
                        Log-Message "Proceso de importación pausado por el usuario. En espera de reanudación..." "WARN"
                        while ($PauseFile -and (Test-Path $PauseFile)) {
                            if ($AbortFile -and (Test-Path $AbortFile)) { break }
                            Start-Sleep -Milliseconds 200
                        }
                        if (-not ($AbortFile -and (Test-Path $AbortFile))) {
                            Log-Message "Proceso de importación reanudado." "INFO"
                        }
                    }

                    $rcvd = $entry.ReceivedTime
                    if ($null -ne $allowedYearsSet -and -not $allowedYearsSet.Contains([int]$rcvd.Year)) {
                        $pstProcessedCount++
                        Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                        continue
                    }
                    if ($null -ne $allowedMonthsSet -and -not $allowedMonthsSet.Contains([int]$rcvd.Month)) {
                        $pstProcessedCount++
                        Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                        continue
                    }

                    # Determinar carpeta de destino final con enrutamiento y caché O(1)
                    $finalDest = $baseDest
                    if ($config -and $config.routing_enabled) {
                        $granularity = if ($config.routing_granularity) { $config.routing_granularity } else { "Mirror" }
                        if ($granularity -ne "Mirror") {
                            $y = $rcvd.Year
                            $m = $rcvd.Month
                            $dateKey = "$baseDestId|$granularity|$y|$m"
                            if ($script:dateFolderCache.ContainsKey($dateKey)) {
                                $finalDest = $script:dateFolderCache[$dateKey]
                            } else {
                                if ($granularity -eq "YearsAndMonths") {
                                    $yearName = "$y"
                                    $yearFolder = Get-OrCreateFolder $baseDest $yearName
                                    $mName = if ($monthNames.ContainsKey($m)) { $monthNames[$m] } else { "{0:D2}" -f $m }
                                    $finalDest = Get-OrCreateFolder $yearFolder $mName
                                } elseif ($granularity -eq "Years") {
                                    $yearName = "$y"
                                    $finalDest = Get-OrCreateFolder $baseDest $yearName
                                }
                                $script:dateFolderCache[$dateKey] = $finalDest
                            }
                        }
                    }

                    # Deduplicación inteligente en memoria
                    $destId = $finalDest.EntryID
                    if (-not $targetSets.ContainsKey($destId)) {
                        $targetSets[$destId] = New-Object 'System.Collections.Generic.HashSet[string]'
                        if ($config -and $config.deduplication_enabled) {
                            Index-TargetFolderItems $finalDest $targetSets[$destId] ([bool]$config.deep_scan_enabled)
                        }
                    }
                    $folderSet = $targetSets[$destId]

                    $subj = $entry.Subject
                    $sender = $entry.Sender
                    $mid = $entry.MessageID
                    $cKey = "$subj|$sender|$($rcvd.ToString('yyyyMMddHHmmss'))"

                    $isDuplicate = $false
                    if ($config -and $config.deduplication_enabled) {
                        if ($folderSet.Contains($cKey)) {
                            $isDuplicate = $true
                        } elseif ($mid -and ($mid -is [string]) -and $mid.Trim() -ne "" -and $folderSet.Contains($mid.Trim())) {
                            $isDuplicate = $true
                        }
                    }

                    if ($isDuplicate) {
                        $totalDuplicates++
                        $pstProcessedCount++
                        if ($processedItemsList.Count -lt 5000) {
                            $itemKb = 0.0
                            if ($entry.Size) { try { $itemKb = [math]::Round([double]$entry.Size / 1024.0, 1) } catch {} }
                            $processedItemsList.Add(@{
                                subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                date          = $rcvd.ToString("yyyy-MM-dd HH:mm:ss")
                                source_folder = $cf.RelPath
                                dest_folder   = $finalDest.Name
                                pst_name      = $pstName
                                status        = "Duplicado Omitido"
                                size_kb       = $itemKb
                            })
                        }
                        Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                        continue
                    }

                    # Transferencia directa O(1) vía GetItemFromID
                    $item = $null
                    $copy = $null
                    try {
                        $item = $namespace.GetItemFromID($entry.EntryID, $pstStore.StoreID)
                        if ($null -ne $item) {
                            if ($config -and $config.transfer_mode -eq "Move") {
                                $item.Move($finalDest) | Out-Null
                            } else {
                                $copy = $item.Copy()
                                $copy.Move($finalDest) | Out-Null
                                if ($null -ne $copy) {
                                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                                }
                            }
                            $totalImported++
                            $pstProcessedCount++

                            [void]$folderSet.Add($cKey)
                            if ($mid -and ($mid -is [string]) -and $mid.Trim() -ne "") {
                                [void]$folderSet.Add($mid.Trim())
                            }

                            if ($processedItemsList.Count -lt 5000) {
                                $itemKb = 0.0
                                if ($entry.Size) { try { $itemKb = [math]::Round([double]$entry.Size / 1024.0, 1) } catch {} }
                                $processedItemsList.Add(@{
                                    subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                    sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                    date          = $rcvd.ToString("yyyy-MM-dd HH:mm:ss")
                                    source_folder = $cf.RelPath
                                    dest_folder   = $finalDest.Name
                                    pst_name      = $pstName
                                    status        = "Importado"
                                    size_kb       = $itemKb
                                })
                            }
                        }
                    } catch {
                        $totalErrors++
                        $pstProcessedCount++
                        $consecutiveThrottles = [math]::Min(15, $consecutiveThrottles + 2)
                        Log-Message "Aviso al transferir correo de carpeta '$($cf.RelPath)': $_" "WARN"
                    } finally {
                        if ($null -ne $copy) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                        }
                        if ($null -ne $item) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                        }
                    }

                    if ($config -and $config.adaptive_throttling -and $consecutiveThrottles -gt 0) {
                        Start-Sleep -Milliseconds ([math]::Min(500, 25 * $consecutiveThrottles))
                        $consecutiveThrottles--
                    }

                    Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime

                    if ($pstProcessedCount % 100 -eq 0) {
                        [System.GC]::Collect()
                        [System.GC]::WaitForPendingFinalizers()
                    }
                }
            } else {
                # === RUTA B: FALLBACK CLÁSICO SI LA CARPETA NO SOPORTA MAPI TABLE ===
                for ($idx = $itemCount; $idx -ge 1; $idx--) {
                # Comprobar protocolo de parada segura
                if ($AbortFile -and (Test-Path $AbortFile)) {
                    Log-Message "Señal de parada segura recibida. Deteniendo proceso ordenadamente..." "WARN"
                    $isAborted = $true
                    break
                }

                # Comprobar pausa solicitada por el usuario
                if ($PauseFile -and (Test-Path $PauseFile)) {
                    Log-Message "Proceso de importación pausado por el usuario. En espera de reanudación..." "WARN"
                    while ($PauseFile -and (Test-Path $PauseFile)) {
                        if ($AbortFile -and (Test-Path $AbortFile)) { break }
                        Start-Sleep -Milliseconds 200
                    }
                    if (-not ($AbortFile -and (Test-Path $AbortFile))) {
                        Log-Message "Proceso de importación reanudado." "INFO"
                    }
                }

                $item = $null
                $copy = $null
                try {
                    $item = $folderItems.Item($idx)
                } catch {
                    $totalErrors++
                    $pstProcessedCount++
                    continue
                }

                if ($null -eq $item) {
                    $pstProcessedCount++
                    continue
                }

                try {
                    # Extraer fecha del correo
                    $rcvd = $null
                    try { $rcvd = $item.ReceivedTime } catch {}
                    if ($null -eq $rcvd -or $rcvd.Year -lt 1980) {
                        try { $rcvd = $item.SentOn } catch {}
                    }
                    if ($null -eq $rcvd) {
                        $rcvd = Get-Date
                    }

                    # Filtro por años específicos si está configurado
                    if ($null -ne $allowedYearsSet -and -not $allowedYearsSet.Contains([int]$rcvd.Year)) {
                        $pstProcessedCount++
                        Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                        continue
                    }

                    # Filtro por meses específicos si está configurado
                    if ($null -ne $allowedMonthsSet -and -not $allowedMonthsSet.Contains([int]$rcvd.Month)) {
                        $pstProcessedCount++
                        Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                        continue
                    }

                    # Determinar carpeta de destino final con enrutamiento y caché O(1)
                    $finalDest = $baseDest
                    if ($config -and $config.routing_enabled) {
                        $granularity = if ($config.routing_granularity) { $config.routing_granularity } else { "Mirror" }
                        if ($granularity -ne "Mirror") {
                            $y = $rcvd.Year
                            $m = $rcvd.Month
                            $dateKey = "$baseDestId|$granularity|$y|$m"
                            if ($script:dateFolderCache.ContainsKey($dateKey)) {
                                $finalDest = $script:dateFolderCache[$dateKey]
                            } else {
                                if ($granularity -eq "YearsAndMonths") {
                                    $yearName = "$y"
                                    $yearFolder = Get-OrCreateFolder $baseDest $yearName
                                    $mName = if ($monthNames.ContainsKey($m)) { $monthNames[$m] } else { "{0:D2}" -f $m }
                                    $finalDest = Get-OrCreateFolder $yearFolder $mName
                                } elseif ($granularity -eq "Years") {
                                    $yearName = "$y"
                                    $finalDest = Get-OrCreateFolder $baseDest $yearName
                                }
                                $script:dateFolderCache[$dateKey] = $finalDest
                            }
                        }
                    }

                    # Deduplicación inteligente con MAPI Table
                    $destId = $finalDest.EntryID
                    if (-not $targetSets.ContainsKey($destId)) {
                        $targetSets[$destId] = New-Object 'System.Collections.Generic.HashSet[string]'
                        if ($config -and $config.deduplication_enabled) {
                            Index-TargetFolderItems $finalDest $targetSets[$destId] ([bool]$config.deep_scan_enabled)
                        }
                    }
                    $folderSet = $targetSets[$destId]

                    $subj = $item.Subject
                    $sender = $item.SenderEmailAddress
                    $cKey = "$subj|$sender|$($rcvd.ToString('yyyyMMddHHmmss'))"

                    $isDuplicate = $false
                    $mid = $null
                    if ($config -and $config.deduplication_enabled) {
                        # 1. Comprobar clave compuesta primero (instantáneo en RAM, 0 llamadas COM)
                        if ($folderSet.Contains($cKey)) {
                            $isDuplicate = $true
                        } else {
                            # 2. Solo si no coincide la clave compuesta, consultar Message-ID MAPI
                            try {
                                $mid = $item.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x1035001E")
                            } catch {}
                            if ($mid -and ($mid -is [string]) -and $folderSet.Contains($mid.Trim())) {
                                $isDuplicate = $true
                            }
                        }
                    }

                    if ($isDuplicate) {
                        $totalDuplicates++
                        $pstProcessedCount++
                        if ($processedItemsList.Count -lt 5000) {
                            $itemKb = 0.0
                            try { $itemKb = [math]::Round($item.Size / 1024.0, 1) } catch {}
                            $processedItemsList.Add(@{
                                subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                date          = $rcvd.ToString("yyyy-MM-dd HH:mm:ss")
                                source_folder = $cf.RelPath
                                dest_folder   = $finalDest.Name
                                pst_name      = $pstName
                                status        = "Duplicado Omitido"
                                size_kb       = $itemKb
                            })
                        }
                    } else {
                        # Transferencia: Copiar (predeterminado) o Mover
                        $itemKb = 0.0
                        try { $itemKb = [math]::Round($item.Size / 1024.0, 1) } catch {}

                        if ($config -and $config.transfer_mode -eq "Move") {
                            $item.Move($finalDest) | Out-Null
                            $totalImported++
                            $pstProcessedCount++
                        } else {
                            $copy = $item.Copy()
                            $copy.Move($finalDest) | Out-Null
                            if ($null -ne $copy) {
                                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                            }
                            $totalImported++
                            $pstProcessedCount++
                        }

                        if ($processedItemsList.Count -lt 5000) {
                            $processedItemsList.Add(@{
                                subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                date          = $rcvd.ToString("yyyy-MM-dd HH:mm:ss")
                                source_folder = $cf.RelPath
                                dest_folder   = $finalDest.Name
                                pst_name      = $pstName
                                status        = "Importado"
                                size_kb       = $itemKb
                            })
                        }

                        # Registrar claves para deduplicar futuros correos de la misma sesión
                        [void]$folderSet.Add($cKey)
                        if ($mid -and ($mid -is [string]) -and $mid.Trim() -ne "") {
                            [void]$folderSet.Add($mid.Trim())
                        }

                        # Throttling adaptativo inteligente (dynamic backoff) y micro-pausa cooperativa
                        if ($config -and $config.adaptive_throttling) {
                            if ($consecutiveThrottles -gt 0) {
                                Start-Sleep -Milliseconds ([math]::Min(500, 25 * $consecutiveThrottles))
                                $consecutiveThrottles--
                            } elseif ($pstProcessedCount % 10 -eq 0) {
                                [System.Threading.Thread]::Sleep(2)
                            }
                        } elseif ($pstProcessedCount % 5 -eq 0) {
                            [System.Threading.Thread]::Sleep(1)
                        }
                    }

                    # Pausa periódica cooperativa cada 10 correos para ceder CPU y no saturar Windows Explorer
                    if ($pstProcessedCount % 10 -eq 0) {
                        [System.Threading.Thread]::Sleep(1)
                    }

                    Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime

                    if ($pstProcessedCount % 100 -eq 0) {
                        [System.GC]::Collect()
                        [System.GC]::WaitForPendingFinalizers()
                    }
                }
                catch {
                    $totalErrors++
                    $pstProcessedCount++
                    $consecutiveThrottles = [math]::Min(15, $consecutiveThrottles + 2)
                    Log-Message "Aviso al procesar correo #$idx de carpeta '$($cf.RelPath)': $_" "WARN"
                    if ($processedItemsList.Count -lt 5000) {
                        $processedItemsList.Add(@{
                            subject       = "(Error al leer correo: $($_.Exception.Message))"
                            sender        = "-"
                            date          = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
                            source_folder = $cf.RelPath
                            dest_folder   = "-"
                            pst_name      = $pstName
                            status        = "Error"
                            size_kb       = 0.0
                        })
                    }

                    Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                }
                finally {
                    if ($null -ne $copy) {
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                    }
                    if ($null -ne $item) {
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                    }
                }
            }
            }

            # Liberar colección de elementos y carpeta de origen al concluir la carpeta
            if ($null -ne $folderItems) {
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($folderItems) | Out-Null } catch {}
                $folderItems = $null
            }
            if ($null -ne $srcFolder) {
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($srcFolder) | Out-Null } catch {}
                $srcFolder = $null
            }
        }

        # 1. Liberar todas las referencias COM de las carpetas candidatas del PST
        if ($candidateFolders) {
            foreach ($cf in $candidateFolders) {
                if ($cf -and $cf.Folder) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($cf.Folder) | Out-Null } catch {}
                    $cf.Folder = $null
                }
            }
        }
        $candidateFolders = $null

        # 2. Liberar raíz del PST
        if ($null -ne $pstRoot) {
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($pstRoot) | Out-Null } catch {}
            $pstRoot = $null
        }

        # 3. Forzar limpieza de punteros COM para que MAPI no mantenga bloqueos en el archivo PST
        [System.GC]::Collect()
        [System.GC]::WaitForPendingFinalizers()
        [System.GC]::Collect()
        [System.GC]::WaitForPendingFinalizers()

        # 4. Desmontar PST si fue montado en esta ejecución
        if ($wasMountedByUs -and -not $isAborted) {
            try {
                $rootF = $pstStore.GetRootFolder()
                $namespace.RemoveStore($rootF)
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($rootF) | Out-Null } catch {}
                $storesToUnmount = $storesToUnmount | Where-Object { $_ -ne $pstStore }
                Log-Message "PST '$pstName' desmontado limpiamente."
            } catch {
                Log-Message "Aviso al desmontar PST: $_" "WARN"
            }
        }

        if ($null -ne $pstStore) {
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($pstStore) | Out-Null } catch {}
            $pstStore = $null
        }

        if ($isAborted) { break }
    }

    # Guardar reporte de elementos procesados en archivo temporal JSON para el informe interactivo HTML
    $itemsTempFile = Join-Path ([System.IO.Path]::GetTempPath()) "outlook_organizer_items.json"
    try {
        $itemsJson = $processedItemsList | ConvertTo-Json -Depth 3 -Compress
        [System.IO.File]::WriteAllText($itemsTempFile, $itemsJson, [System.Text.Encoding]::UTF8)
        Log-Message "Historial de correos ($($processedItemsList.Count) items) preparado para informe interactivo."
    } catch {
        Log-Message "Aviso al escribir historial temporal de correos: $_" "WARN"
    }

    $finalStatus = if ($isAborted) { "aborted" } else { "completed" }
    Send-Telemetry @{
        type       = "finished"
        status     = $finalStatus
        imported   = $totalImported
        duplicates = $totalDuplicates
        errors     = $totalErrors
    }
    Log-Message "Operación finalizada con estado: $finalStatus. Total importados: $totalImported, Duplicados: $totalDuplicates, Errores: $totalErrors."
}
catch {
    Log-Message "Error fatal en automatización COM: $_" "ERROR"
    Send-Telemetry @{
        type       = "finished"
        status     = "failed"
        imported   = $totalImported
        duplicates = $totalDuplicates
        errors     = ($totalErrors + 1)
    }
}
finally {
    Log-Message "Ejecutando limpieza y liberación de punteros COM/MAPI..."

    # 1. Limpiar cachés de carpetas
    Clear-FolderCaches

    # 2. Liberar carpetas candidatas residuales
    if ($candidateFolders) {
        foreach ($cf in $candidateFolders) {
            if ($cf -and $cf.Folder) {
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($cf.Folder) | Out-Null } catch {}
            }
        }
        $candidateFolders = $null
    }

    # 3. Garantizar desmontaje de almacenes montados
    if ($storesToUnmount -and $storesToUnmount.Count -gt 0) {
        foreach ($st in $storesToUnmount) {
            try {
                $rootF = $st.GetRootFolder()
                Log-Message "Desmontando PST residual: $($rootF.Name)..."
                $namespace.RemoveStore($rootF)
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($rootF) | Out-Null } catch {}
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($st) | Out-Null } catch {}
            } catch {}
        }
        $storesToUnmount = @()
    }

    if ($null -ne $destStore) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($destStore) | Out-Null } catch {}
        $destStore = $null
    }

    if ($weStartedOutlook -and $null -ne $outlook) {
        try {
            Log-Message "Cerrando instancia secundaria de Outlook iniciada para la migración..."
            $outlook.Quit()
        } catch {}
    }

    if ($null -ne $namespace) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($namespace) | Out-Null } catch {}
        $namespace = $null
    }
    if ($null -ne $outlook) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {}
        $outlook = $null
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    if ($AbortFile -and (Test-Path $AbortFile)) {
        try { Remove-Item -Path $AbortFile -Force -ErrorAction SilentlyContinue } catch {}
    }
    if ($PauseFile -and (Test-Path $PauseFile)) {
        try { Remove-Item -Path $PauseFile -Force -ErrorAction SilentlyContinue } catch {}
    }

    Log-Message "Punteros COM liberados, procesos desacoplados y recursos finalizados con seguridad."
}
