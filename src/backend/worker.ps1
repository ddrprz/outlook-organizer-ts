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
    [System.Diagnostics.Process]::GetCurrentProcess().PriorityClass = [System.Diagnostics.ProcessPriorityClass]::Normal
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

$script:transferStartTime = $null
$script:rollingHistory = New-Object 'System.Collections.Generic.Queue[hashtable]'

function Emit-ProgressTelemetry([int]$currentPstIdx, [int]$totalPsts, [string]$pstName, [int]$currentItems, [int]$totalItems, [DateTime]$startTime) {
    try {
        $now = [DateTime]::UtcNow
        if ($null -eq $script:transferStartTime -and $currentItems -gt 0) {
            $script:transferStartTime = $now
        }

        # Medidor rodante de velocidad sobre ventana de 15 segundos
        if ($script:rollingHistory -is [System.Collections.Generic.Queue[hashtable]]) {
            $script:rollingHistory.Enqueue(@{ Time = $now; Count = $currentItems })
            while ($script:rollingHistory.Count -gt 1) {
                $oldest = $script:rollingHistory.Peek()
                if (($now - $oldest.Time).TotalSeconds -gt 15.0) {
                    [void]$script:rollingHistory.Dequeue()
                } else {
                    break
                }
            }
        }

        $speed = 0.0
        if ($script:rollingHistory -and $script:rollingHistory.Count -ge 2) {
            $oldest = $script:rollingHistory.Peek()
            $dt = ($now - $oldest.Time).TotalSeconds
            $dc = $currentItems - $oldest.Count
            if ($dt -ge 1.0 -and $dc -gt 0) {
                $speed = [math]::Round($dc / $dt, 1)
            }
        }

        # Fallback al tiempo activo desde el primer correo transferido si la ventana rodante es incipiente
        if ($speed -le 0.0 -and $script:transferStartTime) {
            $activeSec = ($now - $script:transferStartTime).TotalSeconds
            if ($activeSec -ge 1.0 -and $currentItems -gt 0) {
                $speed = [math]::Round($currentItems / $activeSec, 1)
            }
        }

        # Estimación realista de ETA basada en rendimiento activo (evita distorsión por tiempo inicial de logon MAPI)
        $calcSpeed = if ($speed -gt 0.0) { $speed } else { 5.0 }
        $remaining = [math]::Max(0, $totalItems - $currentItems)
        $eta = [math]::Round($remaining / $calcSpeed)

        Send-Telemetry @{
            type           = "progress"
            pst_index      = $currentPstIdx
            pst_total      = $totalPsts
            pst_name       = $pstName
            item_current   = $currentItems
            item_total     = $totalItems
            global_current = $script:globalProcessedItems
            global_total   = $script:globalTotalItems
            speed_mps      = if ($speed -gt 0.0) { $speed } else { 0.0 }
            eta_seconds    = $eta
            imported       = $script:totalImported
            imported_bytes = $script:totalImportedBytes
            duplicates     = $script:totalDuplicates
            errors         = $script:totalErrors
        }
        $script:lastTelemetryTime = $now
    } catch {}
}

function Check-And-Emit-Progress([int]$currentPstIdx, [int]$totalPsts, [string]$pstName, [int]$currentItems, [int]$totalItems, [DateTime]$startTime, [bool]$force = $false) {
    try {
        $now = [DateTime]::UtcNow
        $shouldEmit = $force -or ($currentItems -eq $totalItems) -or ($null -eq $script:lastTelemetryTime) -or (($now - $script:lastTelemetryTime).TotalMilliseconds -ge 200)
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
    if ($cacheKey -and $script:destFolderCache.ContainsKey($cacheKey)) {
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
    if ($script:dateFolderIdCache) { $script:dateFolderIdCache.Clear() }
    if ($script:dateFolderNameCache) { $script:dateFolderNameCache.Clear() }
    $script:transferStartTime = $null
    if ($script:rollingHistory) { $script:rollingHistory.Clear() }
}

function Collect-CandidateFolders($folder, [string]$currentPath, [System.Collections.Generic.List[hashtable]]$collector, $selectedPathsSet, [bool]$hasSubpaths, $config) {
    foreach ($sub in $folder.Folders) {
        $subName = $sub.Name
        # Ignorar carpetas internas de sistema irrelevantes
        if ($subName -match "^(Yammer Root|Quick Step Settings|Conversation History|Social Activity Provider RSS Feeds|Sync Issues|Problemas de sincronización|Conflictos|Errores locales|Fallos del servidor|Journal|Diario|Notes|Notas|Suggested Contacts|Contactos sugeridos)$") {
            continue
        }

        # Manejar contenedores intermedios transparentes de Outlook / Exchange
        $pstBaseName = [System.IO.Path]::GetFileNameWithoutExtension($pstPath)
        if ($subName -match "^(Top of Personal Folders|Top of Information Store|Elementos de nivel superior del almac[eé]n personal|IPM_SUBTREE|IPM_ROOT|Carpetas personales|Personal Folders|Archivo de datos de Outlook|Outlook Data File|Archivo de datos|Datos de Outlook|Almac[eé]n personal|Personal Storage)$" -or ($pstBaseName -and $subName -eq $pstBaseName)) {
            Collect-CandidateFolders $sub "" $collector $selectedPathsSet $hasSubpaths $config
            continue
        }

        $subPath = if ($currentPath -eq "") { $subName } else { "$currentPath\$subName" }
        $fType = Get-FolderType $subName

        # Configuración de permisos
        $allowInbox = if ($config -and $null -ne $config.include_inbox) { [bool]$config.include_inbox } else { $true }
        $allowSent = if ($config -and $null -ne $config.include_sent) { [bool]$config.include_sent } else { $true }
        $allowDeleted = if ($config -and $null -ne $config.include_deleted) { [bool]$config.include_deleted } else { $false }
        $allowCustom = if ($config -and $null -ne $config.include_custom_folders) { [bool]$config.include_custom_folders } else { $true }

        if ($selectedPathsSet -and $selectedPathsSet.Count -gt 0) {
            if ($selectedPathsSet.Contains("Carpetas personalizadas") -or $selectedPathsSet.Contains("Carpetas personalizadas / subcarpetas") -or $selectedPathsSet.Contains("Carpetas personalizadas / subcarpetas adicionales")) {
                $allowCustom = $true
            }
        }

        # Si algún ancestro es 'deleted' y no está permitido deleted, omitir
        if (-not $allowDeleted) {
            $pathParts = $subPath -split "[\\/]"
            $isUnderDeleted = $false
            foreach ($p in $pathParts) {
                if ($p -match "^(Elementos eliminados|Deleted Items|Trash)$") {
                    $isUnderDeleted = $true
                    break
                }
            }
            if ($isUnderDeleted) { continue }
        }

        # Si es deleted y no está permitido, omitir
        if ($fType -eq "deleted" -and -not $allowDeleted) {
            continue
        }

        $shouldInclude = $false

        # Si no hay filtros específicos de rutas, incluir según flags generales
        if ($null -eq $selectedPathsSet -or $selectedPathsSet.Count -eq 0) {
            switch ($fType) {
                "inbox"   { $shouldInclude = $allowInbox }
                "sent"    { $shouldInclude = $allowSent }
                "deleted" { $shouldInclude = $allowDeleted }
                default   { $shouldInclude = $allowCustom }
            }
        } else {
            # Comprobación de coincidencia con selectedPathsSet
            if ($selectedPathsSet.Contains($subPath) -or $selectedPathsSet.Contains($subName)) {
                $shouldInclude = $true
            } else {
                $parts = $subPath -split "[\\/]"
                $running = ""
                foreach ($part in $parts) {
                    $running = if ($running -eq "") { $part } else { "$running\$part" }
                    if ($selectedPathsSet.Contains($running)) {
                        $shouldInclude = $true
                        break
                    }
                }
                if (-not $shouldInclude) {
                    $topAncestor = $parts[0]
                    if ($selectedPathsSet.Contains($topAncestor)) {
                        $shouldInclude = $true
                    }
                }
            }

            # Si no hubo coincidencia directa por ruta, evaluar por tipo y permisos generales
            if (-not $shouldInclude) {
                switch ($fType) {
                    "inbox" {
                        if ($allowInbox -or $selectedPathsSet.Contains("Bandeja de entrada") -or $selectedPathsSet.Contains("Inbox")) {
                            $shouldInclude = $true
                        }
                    }
                    "sent" {
                        if ($allowSent -or $selectedPathsSet.Contains("Elementos enviados") -or $selectedPathsSet.Contains("Sent Items") -or $selectedPathsSet.Contains("Sent")) {
                            $shouldInclude = $true
                        }
                    }
                    "deleted" {
                        if ($allowDeleted -or $selectedPathsSet.Contains("Elementos eliminados") -or $selectedPathsSet.Contains("Deleted Items")) {
                            $shouldInclude = $true
                        }
                    }
                    default {
                        if ($allowCustom) {
                            $shouldInclude = $true
                        }
                    }
                }
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
                $colSubjectIdx = -1
                $colSenderIdx = -1
                $colDateIdx = -1
                try { $tbl.Columns.RemoveAll() } catch {}
                try { $tbl.Columns.Add("Subject") | Out-Null; $colSubjectIdx = $tbl.Columns.Count - 1 } catch {}
                try { $tbl.Columns.Add("SenderEmailAddress") | Out-Null; $colSenderIdx = $tbl.Columns.Count - 1 } catch {}
                try { $tbl.Columns.Add("ReceivedTime") | Out-Null; $colDateIdx = $tbl.Columns.Count - 1 } catch {}

                $batchSize = 5000
                while (-not $tbl.EndOfTable) {
                    $arr = $tbl.GetArray($batchSize)
                    $rowsInBatch = $arr.GetLength(0)
                    $colsInBatch = $arr.GetLength(1)
                    if ($rowsInBatch -eq 0) { break }

                    for ($i = 0; $i -lt $rowsInBatch; $i++) {
                        $s   = if ($colSubjectIdx -ge 0 -and $colSubjectIdx -lt $colsInBatch) { $arr[$i, $colSubjectIdx] } else { $null }
                        $snd = if ($colSenderIdx -ge 0 -and $colSenderIdx -lt $colsInBatch) { $arr[$i, $colSenderIdx] } else { $null }
                        $dt  = if ($colDateIdx -ge 0 -and $colDateIdx -lt $colsInBatch) { $arr[$i, $colDateIdx] } else { $null }

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
            $items = $null
            try {
                $items = $targetFolder.Items
                $count = $items.Count
                $maxInspect = [math]::Min($count, 1500)
                for ($k = 1; $k -le $maxInspect; $k++) {
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
            } catch {}
            finally {
                if ($null -ne $items) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($items) | Out-Null } catch {}
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
$totalImportedBytes = [int64]0
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
$dateFolderIdCache = @{}
$dateFolderNameCache = @{}
$consecutiveThrottles = 0
$processedItemsList = New-Object 'System.Collections.Generic.List[hashtable]'
$duplicateSampleCount = 0
$errorsList = New-Object 'System.Collections.Generic.List[hashtable]'

function Record-ExecutionError([string]$errType, [string]$pst, [string]$srcFolder, [string]$subj, [string]$snd, [string]$msg) {
    try {
        $nowStr = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
        $errObj = @{
            timestamp     = $nowStr
            error_type    = if ($errType) { $errType } else { "item_error" }
            pst_name      = if ($pst) { $pst } else { "" }
            source_folder = if ($srcFolder) { $srcFolder } else { "" }
            subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
            sender        = if ($snd) { $snd } else { "(Desconocido)" }
            error_message = if ($msg) { $msg } else { "" }
        }
        if ($script:errorsList.Count -lt 5000) {
            $script:errorsList.Add($errObj)
        }
        Log-Message "[$errType] $msg" "ERROR"
    } catch {}
}

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

    # Minimizar ventana activa de Outlook para evitar que el repintado de la lista gráfica sature la CPU
    try {
        if ($outlook.ActiveExplorer()) {
            $outlook.ActiveExplorer().WindowState = 1 # olMinimized
        }
    } catch {}

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
    function Get-TargetMailboxStore($ns, $cfg) {
        $tStore = $null
        $targetName = if ($cfg -and $cfg.target_mailboxes -and $cfg.target_mailboxes.Count -gt 0) {
            $cfg.target_mailboxes[0]
        } else {
            ""
        }

        if ($targetName -and $targetName.Trim() -ne "") {
            foreach ($s in $ns.Stores) {
                $dName = ""
                try { $dName = $s.DisplayName } catch {}
                if ($dName -and ($dName -like "*$targetName*" -or ($s.ExchangeStoreType -ne $null -and $dName -eq $targetName))) {
                    $tStore = $s
                    break
                }
            }
        }

        if ($null -eq $tStore) {
            try { $tStore = $ns.DefaultStore } catch {}
        }
        return $tStore
    }

    $destStore = Get-TargetMailboxStore $namespace $config
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
            Record-ExecutionError "pst_error" $pstPath "" "" "" "Archivo PST no encontrado en disco: $pstPath"
            $totalErrors++
            continue
        }

        Log-Message "Montando archivo PST [$($pIdx + 1)/$totalPsts]: $pstName"

        # Limpiar cachés de carpetas para que cada PST resuelva referencias COM limpias y frescas
        Clear-FolderCaches
        if ($null -eq $destStore) {
            $destStore = Get-TargetMailboxStore $namespace $config
        } else {
            try { $null = $destStore.DisplayName } catch { $destStore = Get-TargetMailboxStore $namespace $config }
        }

        function Find-PstStore($ns, [string]$targetPath, [string]$targetName) {
            $fullTarget = [System.IO.Path]::GetFullPath($targetPath).ToLowerInvariant()
            $fileTarget = [System.IO.Path]::GetFileName($targetPath).ToLowerInvariant()
            $baseTarget = [System.IO.Path]::GetFileNameWithoutExtension($targetPath).ToLowerInvariant()
            $nameTarget = $targetName.ToLowerInvariant()

            # 1. Búsqueda exacta por FilePath
            foreach ($s in $ns.Stores) {
                $p = ""
                try { $p = $s.FilePath } catch {}
                if ($p) {
                    try {
                        $fullP = [System.IO.Path]::GetFullPath($p).ToLowerInvariant()
                        if ($fullP -eq $fullTarget) { return $s }
                        if ([System.IO.Path]::GetFileName($fullP).ToLowerInvariant() -eq $fileTarget) { return $s }
                    } catch {}
                }
            }

            # 2. Búsqueda por DisplayName del almacén
            foreach ($s in $ns.Stores) {
                $dn = ""
                try { $dn = $s.DisplayName } catch {}
                if ($dn) {
                    $dnLow = $dn.Trim().ToLowerInvariant()
                    if ($dnLow -eq $baseTarget -or $dnLow -eq $nameTarget -or $dnLow -eq $fileTarget) {
                        return $s
                    }
                }
            }
            return $null
        }

        # Verificar si ya está montado
        $pstStore = Find-PstStore $namespace $pstPath $pstName
        $wasMountedByUs = $false

        if ($pstStore) {
            Log-Message "PST ya cargado en sesión MAPI: $($pstStore.DisplayName)."
        } else {
            # Registrar StoreIDs previos para detección inequívoca tras AddStoreEx
            $storesBefore = @{}
            foreach ($s in $namespace.Stores) {
                try {
                    $sId = $s.StoreID
                    if ($sId) { $storesBefore[$sId] = $true }
                } catch {}
            }

            try {
                $namespace.AddStoreEx($pstPath, 1) # 1 = olStoreDefault
            } catch {
                try {
                    $namespace.AddStoreEx($pstPath, 2) # fallback 2 = olStoreUnicode
                } catch {
                    Record-ExecutionError "pst_error" $pstName "" "" "" "Error al montar PST '$pstName': $_"
                    $totalErrors++
                    continue
                }
            }

            for ($retry = 0; $retry -lt 25; $retry++) {
                Start-Sleep -Milliseconds 200
                $pstStore = Find-PstStore $namespace $pstPath $pstName
                if ($null -ne $pstStore) { break }

                foreach ($s in $namespace.Stores) {
                    try {
                        $sId = $s.StoreID
                        if ($sId -and -not $storesBefore.ContainsKey($sId)) {
                            $pstStore = $s
                            break
                        }
                    } catch {}
                }
                if ($null -ne $pstStore) { break }
            }

            if ($pstStore) {
                $wasMountedByUs = $true
                $storesToUnmount += $pstStore
                Log-Message "PST montado exitosamente en MAPI: $($pstStore.DisplayName)."
            }
        }

        if ($null -eq $pstStore) {
            Record-ExecutionError "pst_error" $pstName "" "" "" "No se pudo acceder al almacén MAPI de: $pstName tras reintentos."
            $totalErrors++
            continue
        }

        # 7. Identificar carpetas de origen en el PST
        $pstRoot = $null
        try {
            $pstRoot = $pstStore.GetRootFolder()
        } catch {
            Record-ExecutionError "pst_error" $pstName "" "" "" "No se pudo obtener carpeta raíz de: $pstName"
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
            $cItems = 0
            try { $cItems = $cf.Folder.Items.Count } catch {}
            $totalPstItems += $cItems
            Log-Message "  -> Carpeta identificada: '$($cf.RelPath)' ($cItems correos)"
        }
        $script:globalTotalItems += $totalPstItems

        Log-Message "PST '$pstName': $($candidateFolders.Length) carpetas identificadas, $totalPstItems correos totales."

        Send-Telemetry @{
            type           = "progress"
            pst_index      = $pIdx + 1
            pst_total      = $totalPsts
            pst_name       = $pstName
            item_current   = 0
            item_total     = $totalPstItems
            global_current = $script:globalProcessedItems
            global_total   = $script:globalTotalItems
            speed_mps      = 0.0
            eta_seconds    = 0
            imported       = $totalImported
            imported_bytes = $totalImportedBytes
            duplicates     = $totalDuplicates
            errors         = $totalErrors
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
                Record-ExecutionError "folder_error" $pstName $cf.RelPath "" "" "No se pudieron leer elementos de carpeta '$($cf.Name)': $_"
                continue
            }

            Log-Message "Procesando carpeta '$($cf.RelPath)' ($itemCount correos)..."

            # Pre-resolver carpeta base de destino una sola vez para toda la carpeta de origen
            $baseDest = Get-CachedDestBaseFolder $destStore $cf.RelPath $cf.Type
            $baseDestId = ""
            $baseDestName = ""
            if ($baseDest) {
                try { $baseDestId = [string]$baseDest.EntryID } catch {}
                try { $baseDestName = [string]$baseDest.Name } catch {}
            }
            if ([string]::IsNullOrWhiteSpace($baseDestId)) { $baseDestId = "DEFAULT_DEST" }
            if ([string]::IsNullOrWhiteSpace($baseDestName)) { $baseDestName = if ($cf.Name) { $cf.Name } else { "-" } }

            # Intentar extracción ultra-rápida por lotes con MAPI Table
            $tableEntries = New-Object 'System.Collections.Generic.List[hashtable]'
            $useTable = $true

            try {
                $tbl = $srcFolder.GetTable()
                $colEidIdx = -1
                $colDateIdx = -1
                $colSubjIdx = -1
                $colSenderIdx = -1
                $colSizeIdx = -1
                $colSentOnIdx = -1

                try { $tbl.Columns.RemoveAll() } catch {}
                try { $tbl.Columns.Add("EntryID") | Out-Null; $colEidIdx = $tbl.Columns.Count - 1 } catch {}
                try { $tbl.Columns.Add("ReceivedTime") | Out-Null; $colDateIdx = $tbl.Columns.Count - 1 } catch {}
                try { $tbl.Columns.Add("Subject") | Out-Null; $colSubjIdx = $tbl.Columns.Count - 1 } catch {}
                try { $tbl.Columns.Add("SenderEmailAddress") | Out-Null; $colSenderIdx = $tbl.Columns.Count - 1 } catch {}
                try { $tbl.Columns.Add("Size") | Out-Null; $colSizeIdx = $tbl.Columns.Count - 1 } catch {}
                try { $tbl.Columns.Add("SentOn") | Out-Null; $colSentOnIdx = $tbl.Columns.Count - 1 } catch {}

                $batchSize = 5000
                while (-not $tbl.EndOfTable) {
                    $arr = $tbl.GetArray($batchSize)
                    $batchRows = $arr.GetLength(0)
                    $batchCols = $arr.GetLength(1)
                    if ($batchRows -eq 0) { break }
                    for ($r = 0; $r -lt $batchRows; $r++) {
                        $eId = if ($colEidIdx -ge 0 -and $colEidIdx -lt $batchCols) { $arr[$r, $colEidIdx] } else { $null }
                        if ($eId) {
                            $rTime = if ($colDateIdx -ge 0 -and $colDateIdx -lt $batchCols) { $arr[$r, $colDateIdx] } else { $null }
                            if (($null -eq $rTime -or -not ($rTime -is [DateTime]) -or $rTime.Year -lt 1980) -and $colSentOnIdx -ge 0 -and $colSentOnIdx -lt $batchCols) {
                                $rTime = $arr[$r, $colSentOnIdx]
                            }
                            if ($null -eq $rTime -or -not ($rTime -is [DateTime])) {
                                $rTime = Get-Date
                            }

                            $eSubj   = if ($colSubjIdx -ge 0 -and $colSubjIdx -lt $batchCols) { $arr[$r, $colSubjIdx] } else { "" }
                            $eSender = if ($colSenderIdx -ge 0 -and $colSenderIdx -lt $batchCols) { $arr[$r, $colSenderIdx] } else { "" }
                            $eSize   = if ($colSizeIdx -ge 0 -and $colSizeIdx -lt $batchCols) { $arr[$r, $colSizeIdx] } else { 0 }

                            $tableEntries.Add(@{
                                EntryID      = $eId
                                ReceivedTime = $rTime
                                Subject      = $eSubj
                                Sender       = $eSender
                                Size         = $eSize
                            })
                        }
                    }
                }
                if ($null -ne $tbl) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tbl) | Out-Null } catch {}
                }
            } catch {
                Log-Message "Aviso: MAPI Table no disponible en '$($cf.RelPath)' ($_). Usando iteración rápida..." "WARN"
                $useTable = $false
            }

            if ($useTable -and $tableEntries.Count -gt 0) {
                Log-Message "Procesando $($tableEntries.Count) elementos con extracción acelerada en RAM..."
                # === RUTA A: PROCESAMIENTO ACELERADO EN RAM CON MAPI TABLE ===
                foreach ($entry in $tableEntries) {
                    if ($pstProcessedCount % 25 -eq 0) {
                        if ($AbortFile -and [System.IO.File]::Exists($AbortFile)) {
                            Log-Message "Señal de parada segura recibida. Deteniendo proceso ordenadamente..." "WARN"
                            $isAborted = $true
                            break
                        }

                        if ($PauseFile -and [System.IO.File]::Exists($PauseFile)) {
                            Log-Message "Proceso de importación pausado por el usuario. En espera de reanudación..." "WARN"
                            while ($PauseFile -and [System.IO.File]::Exists($PauseFile)) {
                                if ($AbortFile -and [System.IO.File]::Exists($AbortFile)) { break }
                                Start-Sleep -Milliseconds 200
                            }
                            if (-not ($AbortFile -and [System.IO.File]::Exists($AbortFile))) {
                                Log-Message "Proceso de importación reanudado." "INFO"
                            }
                        }
                    }

                    $item = $null
                    $copy = $null
                    $subj = if ($entry.Subject) { $entry.Subject } else { "" }
                    $sender = if ($entry.Sender) { $entry.Sender } else { "" }
                    $rcvd = $entry.ReceivedTime

                    try {
                        if ($null -ne $allowedYearsSet -and $null -ne $rcvd -and -not $allowedYearsSet.Contains([int]$rcvd.Year)) {
                            $pstProcessedCount++
                            $script:globalProcessedItems++
                            Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                            continue
                        }
                        if ($null -ne $allowedMonthsSet -and $null -ne $rcvd -and -not $allowedMonthsSet.Contains([int]$rcvd.Month)) {
                            $pstProcessedCount++
                            $script:globalProcessedItems++
                            Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                            continue
                        }

                        # Determinar carpeta de destino final con enrutamiento y caché O(1) de objeto, ID y Nombre
                        $finalDest = $baseDest
                        $destId = $baseDestId
                        $destName = $baseDestName

                        if ($config -and $config.routing_enabled -and $rcvd) {
                            $granularity = if ($config.routing_granularity) { $config.routing_granularity } else { "Mirror" }
                            if ($granularity -ne "Mirror") {
                                $y = $rcvd.Year
                                $m = $rcvd.Month
                                $dateKey = "$baseDestId|$granularity|$y|$m"
                                if ($dateKey -and $script:dateFolderCache.ContainsKey($dateKey)) {
                                    $cachedDest = $script:dateFolderCache[$dateKey]
                                    if ($null -ne $cachedDest) {
                                        $finalDest = $cachedDest
                                        if ($script:dateFolderIdCache.ContainsKey($dateKey)) { $destId = $script:dateFolderIdCache[$dateKey] }
                                        if ($script:dateFolderNameCache.ContainsKey($dateKey)) { $destName = $script:dateFolderNameCache[$dateKey] }
                                    }
                                } else {
                                    $routedDest = $null
                                    if ($granularity -eq "YearsAndMonths") {
                                        $yearName = "$y"
                                        $yearFolder = if ($baseDest) { Get-OrCreateFolder $baseDest $yearName } else { $null }
                                        $mName = if ($m -and $monthNames.ContainsKey($m)) { $monthNames[$m] } else { "{0:D2}" -f $m }
                                        if ($yearFolder) {
                                            $routedDest = Get-OrCreateFolder $yearFolder $mName
                                        }
                                    } elseif ($granularity -eq "Years") {
                                        $yearName = "$y"
                                        if ($baseDest) {
                                            $routedDest = Get-OrCreateFolder $baseDest $yearName
                                        }
                                    }
                                    if ($routedDest) {
                                        $finalDest = $routedDest
                                        $dId = ""
                                        $dName = ""
                                        try { $dId = [string]$routedDest.EntryID } catch {}
                                        try { $dName = [string]$routedDest.Name } catch {}
                                        if ([string]::IsNullOrWhiteSpace($dId)) { $dId = if ($baseDestId) { $baseDestId } else { "DEFAULT_DEST" } }
                                        if ([string]::IsNullOrWhiteSpace($dName)) { $dName = if ($baseDestName) { $baseDestName } else { "-" } }
                                        $destId = $dId
                                        $destName = $dName
                                        if ($dateKey) {
                                            $script:dateFolderCache[$dateKey] = $routedDest
                                            $script:dateFolderIdCache[$dateKey] = $dId
                                            $script:dateFolderNameCache[$dateKey] = $dName
                                        }
                                    }
                                }
                            }
                        }

                        # Recuperar conjunto de claves de deduplicación directamente sin desenrollado
                        if (-not $targetSets.ContainsKey($destId) -or $null -eq $targetSets[$destId]) {
                            $targetSets[$destId] = New-Object 'System.Collections.Generic.HashSet[string]'
                            if ($config -and $config.deduplication_enabled -and $finalDest) {
                                Index-TargetFolderItems $finalDest $targetSets[$destId] ([bool]$config.deep_scan_enabled)
                            }
                        }
                        $folderSet = $targetSets[$destId]
                        if ($folderSet -isnot [System.Collections.Generic.HashSet[string]]) {
                            $folderSet = New-Object 'System.Collections.Generic.HashSet[string]'
                            $targetSets[$destId] = $folderSet
                        }

                        $dtStr = if ($rcvd) { $rcvd.ToString('yyyyMMddHHmmss') } else { "" }
                        $cKey = "$subj|$sender|$dtStr"

                        $isDuplicate = $false
                        if ($config -and $config.deduplication_enabled) {
                            if ($folderSet.Contains($cKey)) {
                                $isDuplicate = $true
                            }
                        }

                        if ($isDuplicate) {
                            $totalDuplicates++
                            $pstProcessedCount++
                            $script:globalProcessedItems++
                            if ($script:duplicateSampleCount -lt 500 -and $processedItemsList.Count -lt 5000) {
                                $script:duplicateSampleCount++
                                $itemKb = 0.0
                                if ($entry.Size) { try { $itemKb = [math]::Round([double]$entry.Size / 1024.0, 1) } catch {} }
                                $processedItemsList.Add(@{
                                    subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                    sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                    date          = if ($rcvd) { $rcvd.ToString("yyyy-MM-dd HH:mm:ss") } else { "" }
                                    source_folder = $cf.RelPath
                                    dest_folder   = $destName
                                    pst_name      = $pstName
                                    status        = "Duplicado Omitido"
                                    size_kb       = $itemKb
                                })
                            }
                            Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                            continue
                        }

                        # Transferencia directa O(1) vía GetItemFromID
                        $item = $namespace.GetItemFromID($entry.EntryID, $pstStore.StoreID)
                        if ($null -ne $item) {
                            $itemBytes = [int64]0
                            if ($entry.Size) { try { $itemBytes = [int64]$entry.Size } catch {} }
                            elseif ($item.Size) { try { $itemBytes = [int64]$item.Size } catch {} }

                            if ($config -and $config.transfer_mode -eq "Move") {
                                $item.Move($finalDest) | Out-Null
                            } else {
                                $copy = $item.Copy()
                                $copy.Move($finalDest) | Out-Null
                                if ($null -ne $copy) {
                                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                                    $copy = $null
                                }
                            }
                            $totalImported++
                            $totalImportedBytes += $itemBytes
                            $pstProcessedCount++
                            $script:globalProcessedItems++

                            if ($folderSet -is [System.Collections.Generic.HashSet[string]]) {
                                [void]$folderSet.Add($cKey)
                            }

                            if ($processedItemsList.Count -lt 5000) {
                                $itemKb = [math]::Round([double]$itemBytes / 1024.0, 1)
                                $processedItemsList.Add(@{
                                    subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                    sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                    date          = if ($rcvd) { $rcvd.ToString("yyyy-MM-dd HH:mm:ss") } else { "" }
                                    source_folder = $cf.RelPath
                                    dest_folder   = $destName
                                    pst_name      = $pstName
                                    status        = "Importado"
                                    size_kb       = $itemKb
                                })
                            }
                        }
                    } catch {
                        $errStr = "$_"
                        $totalErrors++
                        $pstProcessedCount++
                        $script:globalProcessedItems++
                        if ($errStr -match "0x8001010A|0x80040115|busy|throttle|retry|backoff|quota") {
                            $consecutiveThrottles = [math]::Min(10, $consecutiveThrottles + 1)
                        }
                        $safeSubj = if ($subj) { $subj } else { "(Sin Asunto)" }
                        $safeSender = if ($sender) { $sender } else { "(Desconocido)" }
                        try {
                            Record-ExecutionError "item_error" $pstName $cf.RelPath $safeSubj $safeSender "Aviso al procesar correo: $errStr"
                        } catch {}
                        try {
                            if ($processedItemsList.Count -lt 5000) {
                                $processedItemsList.Add(@{
                                    subject       = $safeSubj
                                    sender        = $safeSender
                                    date          = if ($rcvd) { $rcvd.ToString("yyyy-MM-dd HH:mm:ss") } else { "" }
                                    source_folder = $cf.RelPath
                                    dest_folder   = $destName
                                    pst_name      = $pstName
                                    status        = "Error"
                                    size_kb       = 0.0
                                    error_message = $errStr
                                })
                            }
                        } catch {}
                    } finally {
                        if ($null -ne $copy) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                            $copy = $null
                        }
                        if ($null -ne $item) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                            $item = $null
                        }
                    }

                    if ($config -and $config.adaptive_throttling -and $consecutiveThrottles -gt 0) {
                        Start-Sleep -Milliseconds ([math]::Min(250, 25 * $consecutiveThrottles))
                        $consecutiveThrottles--
                    }

                    Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime

                    if ($pstProcessedCount % 1000 -eq 0) {
                        [System.GC]::Collect(1, [System.GCCollectionMode]::Optimized)
                    }
                }
            } else {
                # === RUTA B: FALLBACK ULTRA-RÁPIDO CON LISTA DIRECTA DE ENTRYIDS EN RAM ===
                $entryIds = New-Object 'System.Collections.Generic.List[string]'
                try {
                    foreach ($it in $folderItems) {
                        try {
                            $eIdVal = [string]$it.EntryID
                            if (-not [string]::IsNullOrWhiteSpace($eIdVal)) {
                                $entryIds.Add($eIdVal)
                            }
                        } catch {}
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($it) | Out-Null } catch {}
                    }
                } catch {}

                Log-Message "Procesando $($entryIds.Count) elementos con iterador optimizado O(1)..."

                foreach ($eId in $entryIds) {
                    if ($pstProcessedCount % 25 -eq 0) {
                        if ($AbortFile -and [System.IO.File]::Exists($AbortFile)) {
                            Log-Message "Señal de parada segura recibida. Deteniendo proceso ordenadamente..." "WARN"
                            $isAborted = $true
                            break
                        }

                        if ($PauseFile -and [System.IO.File]::Exists($PauseFile)) {
                            Log-Message "Proceso de importación pausado por el usuario. En espera de reanudación..." "WARN"
                            while ($PauseFile -and [System.IO.File]::Exists($PauseFile)) {
                                if ($AbortFile -and [System.IO.File]::Exists($AbortFile)) { break }
                                Start-Sleep -Milliseconds 200
                            }
                            if (-not ($AbortFile -and [System.IO.File]::Exists($AbortFile))) {
                                Log-Message "Proceso de importación reanudado." "INFO"
                            }
                        }
                    }

                    $item = $null
                    $copy = $null
                    try {
                        $item = $namespace.GetItemFromID($eId, $pstStore.StoreID)
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
                            $script:globalProcessedItems++
                            Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                            continue
                        }

                        # Filtro por meses específicos si está configurado
                        if ($null -ne $allowedMonthsSet -and -not $allowedMonthsSet.Contains([int]$rcvd.Month)) {
                            $pstProcessedCount++
                            $script:globalProcessedItems++
                            Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                            continue
                        }

                        # Determinar carpeta de destino final con enrutamiento y caché O(1) de objeto, ID y Nombre
                        $finalDest = $baseDest
                        $destId = $baseDestId
                        $destName = $baseDestName

                        if ($config -and $config.routing_enabled -and $rcvd) {
                            $granularity = if ($config.routing_granularity) { $config.routing_granularity } else { "Mirror" }
                            if ($granularity -ne "Mirror") {
                                $y = $rcvd.Year
                                $m = $rcvd.Month
                                $dateKey = "$baseDestId|$granularity|$y|$m"
                                if ($dateKey -and $script:dateFolderCache.ContainsKey($dateKey)) {
                                    $cachedDest = $script:dateFolderCache[$dateKey]
                                    if ($null -ne $cachedDest) {
                                        $finalDest = $cachedDest
                                        if ($script:dateFolderIdCache.ContainsKey($dateKey)) { $destId = $script:dateFolderIdCache[$dateKey] }
                                        if ($script:dateFolderNameCache.ContainsKey($dateKey)) { $destName = $script:dateFolderNameCache[$dateKey] }
                                    }
                                } else {
                                    $routedDest = $null
                                    if ($granularity -eq "YearsAndMonths") {
                                        $yearName = "$y"
                                        $yearFolder = if ($baseDest) { Get-OrCreateFolder $baseDest $yearName } else { $null }
                                        $mName = if ($m -and $monthNames.ContainsKey($m)) { $monthNames[$m] } else { "{0:D2}" -f $m }
                                        if ($yearFolder) {
                                            $routedDest = Get-OrCreateFolder $yearFolder $mName
                                        }
                                    } elseif ($granularity -eq "Years") {
                                        $yearName = "$y"
                                        if ($baseDest) {
                                            $routedDest = Get-OrCreateFolder $baseDest $yearName
                                        }
                                    }
                                    if ($routedDest) {
                                        $finalDest = $routedDest
                                        $dId = ""
                                        $dName = ""
                                        try { $dId = [string]$routedDest.EntryID } catch {}
                                        try { $dName = [string]$routedDest.Name } catch {}
                                        if ([string]::IsNullOrWhiteSpace($dId)) { $dId = if ($baseDestId) { $baseDestId } else { "DEFAULT_DEST" } }
                                        if ([string]::IsNullOrWhiteSpace($dName)) { $dName = if ($baseDestName) { $baseDestName } else { "-" } }
                                        $destId = $dId
                                        $destName = $dName
                                        if ($dateKey) {
                                            $script:dateFolderCache[$dateKey] = $routedDest
                                            $script:dateFolderIdCache[$dateKey] = $dId
                                            $script:dateFolderNameCache[$dateKey] = $dName
                                        }
                                    }
                                }
                            }
                        }

                        if (-not $targetSets.ContainsKey($destId) -or $null -eq $targetSets[$destId]) {
                            $targetSets[$destId] = New-Object 'System.Collections.Generic.HashSet[string]'
                            if ($config -and $config.deduplication_enabled -and $finalDest) {
                                Index-TargetFolderItems $finalDest $targetSets[$destId] ([bool]$config.deep_scan_enabled)
                            }
                        }
                        $folderSet = $targetSets[$destId]
                        if ($folderSet -isnot [System.Collections.Generic.HashSet[string]]) {
                            $folderSet = New-Object 'System.Collections.Generic.HashSet[string]'
                            $targetSets[$destId] = $folderSet
                        }

                        $subj = $item.Subject
                        $sender = $item.SenderEmailAddress
                        $cKey = "$subj|$sender|$($rcvd.ToString('yyyyMMddHHmmss'))"

                        $isDuplicate = $false
                        if ($config -and $config.deduplication_enabled) {
                            if ($folderSet.Contains($cKey)) {
                                $isDuplicate = $true
                            }
                        }

                        if ($isDuplicate) {
                            $totalDuplicates++
                            $pstProcessedCount++
                            $script:globalProcessedItems++
                            if ($script:duplicateSampleCount -lt 500 -and $processedItemsList.Count -lt 5000) {
                                $script:duplicateSampleCount++
                                $itemKb = 0.0
                                try { $itemKb = [math]::Round($item.Size / 1024.0, 1) } catch {}
                                $processedItemsList.Add(@{
                                    subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                    sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                    date          = $rcvd.ToString("yyyy-MM-dd HH:mm:ss")
                                    source_folder = $cf.RelPath
                                    dest_folder   = $destName
                                    pst_name      = $pstName
                                    status        = "Duplicado Omitido"
                                    size_kb       = $itemKb
                                })
                            }
                            Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                            continue
                        }

                        # Transferencia: Copiar (predeterminado) o Mover
                        $itemBytes = [int64]0
                        try { if ($item.Size) { $itemBytes = [int64]$item.Size } } catch {}
                        $itemKb = [math]::Round([double]$itemBytes / 1024.0, 1)

                        if ($config -and $config.transfer_mode -eq "Move") {
                            $item.Move($finalDest) | Out-Null
                        } else {
                            $copy = $item.Copy()
                            $copy.Move($finalDest) | Out-Null
                            if ($null -ne $copy) {
                                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                                $copy = $null
                            }
                        }
                        $totalImported++
                        $totalImportedBytes += $itemBytes
                        $pstProcessedCount++
                        $script:globalProcessedItems++

                        if ($processedItemsList.Count -lt 5000) {
                            $processedItemsList.Add(@{
                                subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                                sender        = if ($sender) { $sender } else { "(Desconocido)" }
                                date          = $rcvd.ToString("yyyy-MM-dd HH:mm:ss")
                                source_folder = $cf.RelPath
                                dest_folder   = $destName
                                pst_name      = $pstName
                                status        = "Importado"
                                size_kb       = $itemKb
                            })
                        }

                        if ($folderSet -is [System.Collections.Generic.HashSet[string]]) {
                            [void]$folderSet.Add($cKey)
                        }

                        if ($config -and $config.adaptive_throttling -and $consecutiveThrottles -gt 0) {
                            Start-Sleep -Milliseconds ([math]::Min(250, 25 * $consecutiveThrottles))
                            $consecutiveThrottles--
                        }

                        Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime

                        if ($pstProcessedCount % 1000 -eq 0) {
                            [System.GC]::Collect(1, [System.GCCollectionMode]::Optimized)
                        }
                    }
                    catch {
                        $errStr = "$_"
                        $totalErrors++
                        $pstProcessedCount++
                        $script:globalProcessedItems++
                        if ($errStr -match "0x8001010A|0x80040115|busy|throttle|retry|backoff|quota") {
                            $consecutiveThrottles = [math]::Min(10, $consecutiveThrottles + 1)
                        }
                        $safeSubj = if ($subj) { $subj } else { "(Sin Asunto)" }
                        $safeSender = if ($sender) { $sender } else { "(Desconocido)" }
                        try {
                            Record-ExecutionError "item_error" $pstName $cf.RelPath $safeSubj $safeSender "Aviso al procesar correo: $errStr"
                        } catch {}
                        try {
                            if ($processedItemsList.Count -lt 5000) {
                                $processedItemsList.Add(@{
                                    subject       = $safeSubj
                                    sender        = $safeSender
                                    date          = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
                                    source_folder = $cf.RelPath
                                    dest_folder   = "-"
                                    pst_name      = $pstName
                                    status        = "Error"
                                    size_kb       = 0.0
                                    error_message = $errStr
                                })
                            }
                        } catch {}

                        Check-And-Emit-Progress ($pIdx + 1) $totalPsts $pstName $pstProcessedCount $totalPstItems $startTime
                    }
                    finally {
                        if ($null -ne $copy) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($copy) | Out-Null } catch {}
                            $copy = $null
                        }
                        if ($null -ne $item) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                            $item = $null
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

        Clear-FolderCaches

        if ($isAborted) { break }
    }

    # Guardar reporte de elementos procesados y registro de errores en archivos temporales JSON para el informe interactivo HTML y auditoría
    $itemsTempFile = Join-Path ([System.IO.Path]::GetTempPath()) "outlook_organizer_items.json"
    try {
        $itemsJson = if ($processedItemsList.Count -eq 0) {
            "[]"
        } elseif ($processedItemsList.Count -eq 1) {
            "[" + ($processedItemsList[0] | ConvertTo-Json -Depth 4 -Compress) + "]"
        } else {
            $processedItemsList | ConvertTo-Json -Depth 4 -Compress
        }
        [System.IO.File]::WriteAllText($itemsTempFile, $itemsJson, [System.Text.Encoding]::UTF8)
        Log-Message "Historial de correos ($($processedItemsList.Count) items) preparado para informe interactivo."
    } catch {
        Log-Message "Aviso al escribir historial temporal de correos: $_" "WARN"
    }

    $errorsTempFile = Join-Path ([System.IO.Path]::GetTempPath()) "outlook_organizer_errors.json"
    try {
        $errorsJson = if ($errorsList.Count -eq 0) {
            "[]"
        } elseif ($errorsList.Count -eq 1) {
            "[" + ($errorsList[0] | ConvertTo-Json -Depth 4 -Compress) + "]"
        } else {
            $errorsList | ConvertTo-Json -Depth 4 -Compress
        }
        [System.IO.File]::WriteAllText($errorsTempFile, $errorsJson, [System.Text.Encoding]::UTF8)
        Log-Message "Registro de incidencias ($($errorsList.Count) errores) preparado para auditoría e informe."
    } catch {
        Log-Message "Aviso al escribir registro temporal de errores: $_" "WARN"
    }

    $finalStatus = if ($isAborted) { "aborted" } else { "completed" }
    Send-Telemetry @{
        type           = "finished"
        status         = $finalStatus
        imported       = $totalImported
        imported_bytes = $totalImportedBytes
        duplicates     = $totalDuplicates
        errors         = $totalErrors
    }
    Log-Message "Operación finalizada con estado: $finalStatus. Total importados: $totalImported, Duplicados: $totalDuplicates, Errores: $totalErrors."
}
catch {
    Record-ExecutionError "fatal_error" "" "" "" "" "Error fatal en automatización COM: $_"
    Send-Telemetry @{
        type           = "finished"
        status         = "failed"
        imported       = $totalImported
        imported_bytes = $totalImportedBytes
        duplicates     = $totalDuplicates
        errors         = ($totalErrors + 1)
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
