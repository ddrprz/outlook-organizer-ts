<#
.SYNOPSIS
    Motor de migración e importación Outlook COM / MAPI de alto rendimiento con
    telemetría en tiempo real, deduplicación, enrutamiento temporal y protocolo
    de parada segura (Graceful Shutdown) para Outlook Organizer TS.
#>
param (
    [string]$ConfigFile = "",
    [string]$AbortFile = ""
)

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
$ErrorActionPreference    = "Stop"

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
                    [System.Runtime.InteropServices.Marshal]::ReleaseComObject($existing) | Out-Null
                }
            }
        }

        if ($deepScan) {
            foreach ($sub in $targetFolder.Folders) {
                Index-TargetFolderItems $sub $seenSet $deepScan
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
$lastTelemetryTime = [DateTime]::MinValue
$targetSets = @{}

try {
    # 2. Conectar a Outlook COM en modo STA
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
        Log-Message "Enlace establecido con instancia activa de Outlook."
    } catch {
        $outlook = New-Object -ComObject Outlook.Application
        Log-Message "Nueva instancia de Outlook COM iniciada."
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
        }

        $pstProcessedCount = 0

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

            # Iterar elementos en orden inverso (seguro para Copy y Move)
            for ($idx = $itemCount; $idx -ge 1; $idx--) {
                # Comprobar protocolo de parada segura
                if ($AbortFile -and (Test-Path $AbortFile)) {
                    Log-Message "Señal de parada segura recibida. Deteniendo proceso ordenadamente..." "WARN"
                    $isAborted = $true
                    break
                }

                $item = $null
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

                    # Filtro por año específico si está configurado
                    if ($config -and $config.specific_year -and $rcvd.Year -ne [int]$config.specific_year) {
                        $pstProcessedCount++
                        continue
                    }

                    # Filtro por mes específico si está configurado
                    if ($config -and $config.specific_month -and $rcvd.Month -ne [int]$config.specific_month) {
                        $pstProcessedCount++
                        continue
                    }

                    # Determinar carpeta de destino base
                    $baseDest = Get-DestFolderByPath $destStore $cf.RelPath $cf.Type
                    $finalDest = $baseDest

                    # Enrutamiento jerárquico por fecha (solo cuando no es Espejo)
                    if ($config -and $config.routing_enabled) {
                        if ($config.routing_granularity -eq "YearsAndMonths") {
                            $yearName = "$($rcvd.Year)"
                            $yearFolder = Get-OrCreateFolder $baseDest $yearName
                            $mName = if ($monthNames.ContainsKey($rcvd.Month)) { $monthNames[$rcvd.Month] } else { "{0:D2}" -f $rcvd.Month }
                            $finalDest = Get-OrCreateFolder $yearFolder $mName
                        } elseif ($config.routing_granularity -eq "Years") {
                            $yearName = "$($rcvd.Year)"
                            $yearFolder = Get-OrCreateFolder $baseDest $yearName
                            $finalDest = $yearFolder
                        }
                        # Si es "Mirror", $finalDest permanece como $baseDest (estructura original espejo sin agrupar por fecha)
                    }

                    # Deduplicación inteligente
                    $destId = $finalDest.EntryID
                    if (-not $targetSets.ContainsKey($destId)) {
                        $targetSets[$destId] = New-Object 'System.Collections.Generic.HashSet[string]'
                        if ($config -and $config.deduplication_enabled) {
                            Index-TargetFolderItems $finalDest $targetSets[$destId] ([bool]$config.deep_scan_enabled)
                        }
                    }
                    $folderSet = $targetSets[$destId]

                    $mid = $null
                    try {
                        $mid = $item.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x1035001E")
                    } catch {}
                    $subj = $item.Subject
                    $sender = $item.SenderEmailAddress
                    $cKey = "$subj|$sender|$($rcvd.ToString('yyyyMMddHHmmss'))"

                    $isDuplicate = $false
                    if ($config -and $config.deduplication_enabled) {
                        if (($mid -and $folderSet.Contains($mid.Trim())) -or $folderSet.Contains($cKey)) {
                            $isDuplicate = $true
                        }
                    }

                    if ($isDuplicate) {
                        $totalDuplicates++
                        $pstProcessedCount++
                        continue
                    }

                    # Transferencia: Copiar (predeterminado) o Mover
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

                    # Registrar clave para deduplicar futuros correos de la misma sesión
                    if ($mid -and $mid.Trim() -ne "") {
                        [void]$folderSet.Add($mid.Trim())
                    }
                    [void]$folderSet.Add($cKey)

                    # Telemetría en vivo (cada 5 items o cada 250ms)
                    $now = [DateTime]::UtcNow
                    if ($pstProcessedCount % 5 -eq 0 -or ($now - $lastTelemetryTime).TotalMilliseconds -gt 250 -or $pstProcessedCount -eq $totalPstItems) {
                        $elapsedSec = ($now - $startTime).TotalSeconds
                        $speed = if ($elapsedSec -gt 0) { [math]::Round($totalImported / $elapsedSec, 1) } else { 0.0 }
                        $remaining = [math]::Max(0, $totalPstItems - $pstProcessedCount)
                        $eta = if ($speed -gt 0) { [math]::Round($remaining / $speed) } else { 0 }

                        Send-Telemetry @{
                            type         = "progress"
                            pst_index    = $pIdx + 1
                            pst_total    = $totalPsts
                            pst_name     = $pstName
                            item_current = $pstProcessedCount
                            item_total   = $totalPstItems
                            speed_mps    = $speed
                            eta_seconds  = $eta
                        }
                        $lastTelemetryTime = $now
                    }

                    # Throttling adaptativo
                    if ($config -and $config.adaptive_throttling) {
                        Start-Sleep -Milliseconds 12
                    }

                    if ($pstProcessedCount % 50 -eq 0) {
                        [System.GC]::Collect()
                    }
                }
                catch {
                    $totalErrors++
                    $pstProcessedCount++
                }
                finally {
                    if ($null -ne $item) {
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($item) | Out-Null } catch {}
                    }
                }
            }
        }

        # Desmontar PST si fue montado en esta ejecución
        if ($wasMountedByUs -and -not $isAborted) {
            try {
                $rootF = $pstStore.GetRootFolder()
                $namespace.RemoveStore($rootF)
                $storesToUnmount = $storesToUnmount | Where-Object { $_ -ne $pstStore }
                Log-Message "PST '$pstName' desmontado limpiamente."
            } catch {
                Log-Message "Aviso al desmontar PST: $_" "WARN"
            }
        }

        if ($isAborted) { break }
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

    # Garantizar desmontaje de almacenes montados
    if ($storesToUnmount -and $storesToUnmount.Count -gt 0) {
        foreach ($st in $storesToUnmount) {
            try {
                $rootF = $st.GetRootFolder()
                Log-Message "Desmontando PST residual: $($rootF.Name)..."
                $namespace.RemoveStore($rootF)
            } catch {}
        }
    }

    if ($null -ne $namespace) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($namespace) | Out-Null } catch {}
    }
    if ($null -ne $outlook) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {}
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    if ($AbortFile -and (Test-Path $AbortFile)) {
        try { Remove-Item -Path $AbortFile -Force -ErrorAction SilentlyContinue } catch {}
    }

    Log-Message "Punteros COM liberados y recursos finalizados con seguridad."
}
