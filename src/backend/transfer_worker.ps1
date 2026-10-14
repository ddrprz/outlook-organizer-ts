param (
    [string]$ConfigFile = "",
    [string]$AbortFile = "",
    [string]$PauseFile = ""
)

<#
.SYNOPSIS
    Motor de transferencia de correos entre buzones de Outlook / Exchange (Buzón Principal / Compartidos)
    con telemetría en tiempo real, deduplicación, filtrado temporal por años y meses, y protocolo de parada segura.
#>

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
$ErrorActionPreference    = "Stop"

try {
    [System.Diagnostics.Process]::GetCurrentProcess().PriorityClass = [System.Diagnostics.ProcessPriorityClass]::High
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
$script:lastTelemetryTime = $null
$script:globalProcessedItems = 0
$script:globalTotalItems = 0
$script:totalImported = [int64]0
$script:totalImportedBytes = [int64]0
$script:totalDuplicates = [int64]0
$script:totalErrors = [int64]0
$script:isAborted = $false

function Emit-ProgressTelemetry([int]$currentItems, [int]$totalItems, [string]$folderName, [DateTime]$startTime) {
    try {
        $now = [DateTime]::UtcNow
        if ($null -eq $script:transferStartTime -and $currentItems -gt 0) {
            $script:transferStartTime = $now
        }

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

        if ($speed -le 0.0 -and $script:transferStartTime) {
            $activeSec = ($now - $script:transferStartTime).TotalSeconds
            if ($activeSec -ge 1.0 -and $currentItems -gt 0) {
                $speed = [math]::Round($currentItems / $activeSec, 1)
            }
        }

        $calcSpeed = if ($speed -gt 0.0) { $speed } else { 5.0 }
        $remaining = [math]::Max(0, $totalItems - $currentItems)
        $eta = [math]::Round($remaining / $calcSpeed)

        Send-Telemetry @{
            type           = "progress"
            pst_index      = 1
            pst_total      = 1
            pst_name       = $folderName
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

function Check-And-Emit-Progress([int]$currentItems, [int]$totalItems, [string]$folderName, [DateTime]$startTime, [bool]$force = $false) {
    try {
        $now = [DateTime]::UtcNow
        $shouldEmit = $force -or ($currentItems -eq $totalItems) -or ($null -eq $script:lastTelemetryTime) -or (($now - $script:lastTelemetryTime).TotalMilliseconds -ge 200)
        if ($shouldEmit) {
            Emit-ProgressTelemetry $currentItems $totalItems $folderName $startTime
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
    if ([string]::IsNullOrWhiteSpace($relPath)) { return $destRoot }
    $parts = $relPath.Split("\")
    $curr = $destRoot
    foreach ($p in $parts) {
        if (-not [string]::IsNullOrWhiteSpace($p)) {
            $curr = Get-OrCreateFolder $curr $p.Trim()
        }
    }
    return $curr
}

function Collect-CandidateFolders($folder, [string]$parentPath, [System.Collections.Generic.List[hashtable]]$collector) {
    $fName = $folder.Name
    if ($fName -match "^(Yammer Root|Quick Step Settings|Conversation History|Social Activity Provider RSS Feeds|Sync Issues|Problemas de sincronización|Conflictos|Errores locales|Fallos del servidor|Journal|Diario|Notes|Notas|Suggested Contacts|Contactos sugeridos)$") {
        return
    }

    if ($fName -match "^(Top of Personal Folders|Top of Information Store|Elementos de nivel superior del almac[eé]n personal|IPM_SUBTREE|IPM_ROOT)$") {
        try {
            foreach ($sub in $folder.Folders) {
                Collect-CandidateFolders $sub $parentPath $collector
            }
        } catch {}
        return
    }

    $relPath = if ($parentPath) { "$parentPath\$fName" } else { $fName }
    $cnt = 0
    try { if ($folder.Items) { $cnt = [int]$folder.Items.Count } } catch { $cnt = 0 }

    if ($cnt -gt 0) {
        $collector.Add(@{
            Folder  = $folder
            Name    = $fName
            RelPath = $relPath
            Count   = $cnt
        })
    }

    try {
        foreach ($sub in $folder.Folders) {
            Collect-CandidateFolders $sub $relPath $collector
        }
    } catch {}
}

function Index-TargetFolderItems($targetFolder, [System.Collections.Generic.HashSet[string]]$seenSet) {
    try {
        $tbl = $targetFolder.GetTable()
        if ($tbl) {
            $colSubj = -1
            $colSnd = -1
            $colDt = -1
            try { $tbl.Columns.RemoveAll() } catch {}
            try { $tbl.Columns.Add("Subject") | Out-Null; $colSubj = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("SenderEmailAddress") | Out-Null; $colSnd = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("ReceivedTime") | Out-Null; $colDt = $tbl.Columns.Count - 1 } catch {}

            $batchSize = 5000
            while (-not $tbl.EndOfTable) {
                $arr = $tbl.GetArray($batchSize)
                $bRows = $arr.GetLength(0)
                $bCols = $arr.GetLength(1)
                if ($bRows -eq 0) { break }

                for ($i = 0; $i -lt $bRows; $i++) {
                    $s   = if ($colSubj -ge 0 -and $colSubj -lt $bCols) { $arr[$i, $colSubj] } else { $null }
                    $snd = if ($colSnd -ge 0 -and $colSnd -lt $bCols) { $arr[$i, $colSnd] } else { $null }
                    $dt  = if ($colDt -ge 0 -and $colDt -lt $bCols) { $arr[$i, $colDt] } else { $null }

                    if ($s -or $snd -or $dt) {
                        $dtStr = if ($dt -and ($dt -is [DateTime])) { $dt.ToString('yyyyMMddHHmmss') } else { "" }
                        $cKey = "$s|$snd|$dtStr"
                        [void]$seenSet.Add($cKey)
                    }
                }
            }
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tbl) | Out-Null } catch {}
        }
    } catch {}
}

$processedItemsList = New-Object 'System.Collections.Generic.List[hashtable]'
$errorsList = New-Object 'System.Collections.Generic.List[hashtable]'

function Record-ExecutionError([string]$errType, [string]$srcFolder, [string]$subj, [string]$snd, [string]$msg) {
    try {
        $nowStr = (Get-Date).ToString("yyyy-MM-dd HH:mm:ss")
        $errObj = @{
            timestamp     = $nowStr
            error_type    = $errType
            source_folder = $srcFolder
            subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
            sender        = if ($snd) { $snd } else { "(Desconocido)" }
            error_message = $msg
        }
        if ($script:errorsList.Count -lt 5000) {
            $script:errorsList.Add($errObj)
        }
        Log-Message "[$errType] $msg" "ERROR"
    } catch {}
}

$outlook = $null
$namespace = $null
$weStartedOutlook = $false
$sourceStore = $null
$targetStore = $null

try {
    Log-Message "Iniciando motor de transferencia entre buzones MAPI..."

    if (-not (Test-Path $ConfigFile)) {
        throw "El archivo de configuración '$ConfigFile' no existe."
    }

    $configJson = [System.IO.File]::ReadAllText($ConfigFile, [System.Text.Encoding]::UTF8)
    $config = $configJson | ConvertFrom-Json
    Log-Message "Configuración cargada. Origen: '$($config.source_mailbox)' -> Destino: '$($config.target_mailbox)'"

    # Conectar con Outlook COM
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
        Log-Message "Conectado a la instancia activa de Outlook."
    } catch {
        $outlook = New-Object -ComObject Outlook.Application
        $weStartedOutlook = $true
        Log-Message "Nueva instancia de Outlook COM iniciada en segundo plano."
    }

    $namespace = $outlook.GetNamespace("MAPI")
    if ($config.profile_name -and $config.profile_name.Trim() -ne "") {
        try { $namespace.Logon($config.profile_name.Trim(), "", $false, $false) } catch {}
    } else {
        try { $namespace.Logon("", "", $false, $false) } catch {}
    }

    # Desviar ActiveExplorer a tareas locales y minimizar para congelar UI en Modo Online
    try {
        $activeExp = $outlook.ActiveExplorer()
        if ($null -ne $activeExp) {
            $localTasks = $namespace.GetDefaultFolder(13)
            if ($null -ne $localTasks) {
                $activeExp.CurrentFolder = $localTasks
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($localTasks) | Out-Null } catch {}
            }
            $activeExp.WindowState = 1
            Log-Message "Vista activa de Outlook desviada a carpeta local y minimizada para acelerar transferencia."
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($activeExp) | Out-Null } catch {}
        }
    } catch {}

    # Localizar almacén de origen y destino
    $srcNorm = $config.source_mailbox.Trim().ToLowerInvariant()
    $dstNorm = $config.target_mailbox.Trim().ToLowerInvariant()

    foreach ($s in $namespace.Stores) {
        $dn = ""
        try { $dn = $s.DisplayName } catch {}
        if ($dn) {
            $dnNorm = $dn.Trim().ToLowerInvariant()
            if ($null -eq $sourceStore -and ($dnNorm -eq $srcNorm -or $dnNorm.Contains($srcNorm) -or $srcNorm.Contains($dnNorm))) {
                $sourceStore = $s
            }
            if ($null -eq $targetStore -and ($dnNorm -eq $dstNorm -or $dnNorm.Contains($dstNorm) -or $dstNorm.Contains($dnNorm))) {
                $targetStore = $s
            }
        }
    }

    if ($null -eq $sourceStore) { throw "No se encontró el almacén del buzón de origen: '$($config.source_mailbox)'." }
    if ($null -eq $targetStore) { throw "No se encontró el almacén del buzón de destino: '$($config.target_mailbox)'." }

    Log-Message "Buzón Origen confirmado: '$($sourceStore.DisplayName)'"
    Log-Message "Buzón Destino confirmado: '$($targetStore.DisplayName)'"

    $allowedYearsSet = $null
    if ($config.allowed_years -and $config.allowed_years.Count -gt 0) {
        $allowedYearsSet = New-Object 'System.Collections.Generic.HashSet[int]'
        foreach ($y in $config.allowed_years) { [void]$allowedYearsSet.Add([int]$y) }
        Log-Message "Filtro de años activo: $($config.allowed_years -join ', ')"
    }

    $allowedMonthsSet = $null
    if ($config.allowed_months -and $config.allowed_months.Count -gt 0) {
        $allowedMonthsSet = New-Object 'System.Collections.Generic.HashSet[int]'
        foreach ($m in $config.allowed_months) { [void]$allowedMonthsSet.Add([int]$m) }
        Log-Message "Filtro de meses activo: $($config.allowed_months -join ', ')"
    }

    $srcRoot = $sourceStore.GetRootFolder()
    $dstRoot = $targetStore.GetRootFolder()

    $candidateList = New-Object 'System.Collections.Generic.List[hashtable]'
    Collect-CandidateFolders $srcRoot "" $candidateList

    $totalCandidatesCount = 0
    foreach ($cf in $candidateList) { $totalCandidatesCount += $cf.Count }
    $script:globalTotalItems = $totalCandidatesCount

    Log-Message "Carpetas identificadas en buzón origen: $($candidateList.Count). Correos candidatos totales: $totalCandidatesCount"

    $startTime = [DateTime]::UtcNow
    $totalProcessed = 0
    $consecutiveThrottles = 0
    $destFolderCache = @{}
    $destSeenSets = @{}

    foreach ($cf in $candidateList) {
        if ($script:isAborted) { break }

        $srcFolder = $cf.Folder
        $relPath = $cf.RelPath
        $countInFolder = $cf.Count

        Log-Message "Procesando carpeta '$relPath' ($countInFolder correos)..."

        # Obtener o crear carpeta homóloga en el buzón destino
        $destFolder = if ($destFolderCache.ContainsKey($relPath)) {
            $destFolderCache[$relPath]
        } else {
            $df = Ensure-FolderHierarchy $dstRoot $relPath
            $destFolderCache[$relPath] = $df
            $df
        }

        # Indexar carpeta destino para deduplicación si está habilitada
        if ($config.deduplication_enabled -and -not $destSeenSets.ContainsKey($relPath)) {
            $sSet = New-Object 'System.Collections.Generic.HashSet[string]'
            Index-TargetFolderItems $destFolder $sSet
            $destSeenSets[$relPath] = $sSet
        }
        $seenSet = if ($destSeenSets.ContainsKey($relPath)) { $destSeenSets[$relPath] } else { $null }

        # Leer metadatos por lotes con MAPI Table
        $tbl = $null
        $tableEntries = New-Object 'System.Collections.Generic.List[hashtable]'
        try {
            $tbl = $srcFolder.GetTable()
            $colEid = -1
            $colDate = -1
            $colSubj = -1
            $colSnd = -1
            $colMid = -1
            $colSz = -1
            $colSent = -1

            try { $tbl.Columns.RemoveAll() } catch {}
            try { $tbl.Columns.Add("EntryID") | Out-Null; $colEid = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("ReceivedTime") | Out-Null; $colDate = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("Subject") | Out-Null; $colSubj = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("SenderEmailAddress") | Out-Null; $colSnd = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("http://schemas.microsoft.com/mapi/proptag/0x1035001E") | Out-Null; $colMid = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("Size") | Out-Null; $colSz = $tbl.Columns.Count - 1 } catch {}
            try { $tbl.Columns.Add("SentOn") | Out-Null; $colSent = $tbl.Columns.Count - 1 } catch {}

            $batchSize = 5000
            while (-not $tbl.EndOfTable) {
                $arr = $tbl.GetArray($batchSize)
                $bRows = $arr.GetLength(0)
                $bCols = $arr.GetLength(1)
                if ($bRows -eq 0) { break }

                for ($r = 0; $r -lt $bRows; $r++) {
                    $eId = if ($colEid -ge 0 -and $colEid -lt $bCols) { $arr[$r, $colEid] } else { $null }
                    if ($eId) {
                        $rTime = if ($colDate -ge 0 -and $colDate -lt $bCols) { $arr[$r, $colDate] } else { $null }
                        if (($null -eq $rTime -or -not ($rTime -is [DateTime]) -or $rTime.Year -lt 1980) -and $colSent -ge 0 -and $colSent -lt $bCols) {
                            $rTime = $arr[$r, $colSent]
                        }
                        if ($null -eq $rTime -or -not ($rTime -is [DateTime])) { $rTime = Get-Date }

                        $eSubj = if ($colSubj -ge 0 -and $colSubj -lt $bCols) { $arr[$r, $colSubj] } else { "" }
                        $eSnd  = if ($colSnd -ge 0 -and $colSnd -lt $bCols) { $arr[$r, $colSnd] } else { "" }
                        $eMid  = if ($colMid -ge 0 -and $colMid -lt $bCols) { $arr[$r, $colMid] } else { "" }
                        $eSz   = if ($colSz -ge 0 -and $colSz -lt $bCols) { $arr[$r, $colSz] } else { 0 }

                        $tableEntries.Add(@{
                            EntryID      = $eId
                            ReceivedTime = $rTime
                            Subject      = $eSubj
                            Sender       = $eSnd
                            MessageID    = $eMid
                            Size         = $eSz
                        })
                    }
                }
            }
            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tbl) | Out-Null } catch {}
            $tbl = $null
        } catch {
            Log-Message "Aviso al leer tabla MAPI de '$relPath': $_" "WARN"
        }

        # Procesar correos
        foreach ($entry in $tableEntries) {
            if ($AbortFile -and [System.IO.File]::Exists($AbortFile)) {
                Log-Message "Señal de parada segura recibida. Deteniendo transferencia..." "WARN"
                $script:isAborted = $true
                break
            }

            if ($PauseFile -and [System.IO.File]::Exists($PauseFile)) {
                Log-Message "Transferencia pausada. En espera de reanudación..." "WARN"
                while ($PauseFile -and [System.IO.File]::Exists($PauseFile)) {
                    if ($AbortFile -and [System.IO.File]::Exists($AbortFile)) { break }
                    Start-Sleep -Milliseconds 200
                }
                if (-not ($AbortFile -and [System.IO.File]::Exists($AbortFile))) {
                    Log-Message "Transferencia reanudada." "INFO"
                }
            }

            $totalProcessed++
            $script:globalProcessedItems++

            $rcvd = $entry.ReceivedTime
            $subj = $entry.Subject
            $sender = $entry.Sender
            $mid = $entry.MessageID

            # Filtros de año y mes
            if ($null -ne $allowedYearsSet -and $null -ne $rcvd -and -not $allowedYearsSet.Contains([int]$rcvd.Year)) {
                Check-And-Emit-Progress $totalProcessed $totalCandidatesCount $relPath $startTime
                continue
            }
            if ($null -ne $allowedMonthsSet -and $null -ne $rcvd -and -not $allowedMonthsSet.Contains([int]$rcvd.Month)) {
                Check-And-Emit-Progress $totalProcessed $totalCandidatesCount $relPath $startTime
                continue
            }

            # Deduplicación
            $dtStr = if ($rcvd) { $rcvd.ToString('yyyyMMddHHmmss') } else { "" }
            $cKey = "$subj|$sender|$dtStr"
            $isDuplicate = $false
            if ($config.deduplication_enabled -and $seenSet) {
                if ($seenSet.Contains($cKey)) {
                    $isDuplicate = $true
                } elseif ($mid -and ($mid -is [string]) -and $mid.Trim() -ne "" -and $seenSet.Contains($mid.Trim())) {
                    $isDuplicate = $true
                }
            }

            if ($isDuplicate) {
                $script:totalDuplicates++
                Check-And-Emit-Progress $totalProcessed $totalCandidatesCount $relPath $startTime
                continue
            }

            # Transferir ítem
            $item = $null
            $copy = $null
            try {
                $item = $namespace.GetItemFromID($entry.EntryID, $sourceStore.StoreID)
                if ($null -ne $item) {
                    $itemBytes = [int64]0
                    if ($entry.Size) { try { $itemBytes = [int64]$entry.Size } catch {} }

                    if ($config.transfer_mode -eq "Move") {
                        $moved = $item.Move($destFolder)
                        if ($null -ne $moved) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($moved) | Out-Null } catch {}
                            $moved = $null
                        }
                    } else {
                        $copy = $item.Copy()
                        $moved = $copy.Move($destFolder)
                        if ($null -ne $moved) {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($moved) | Out-Null } catch {}
                            $moved = $null
                        }
                        $copy = $null
                    }

                    $script:totalImported++
                    $script:totalImportedBytes += $itemBytes
                    $consecutiveThrottles = 0

                    if ($seenSet) {
                        [void]$seenSet.Add($cKey)
                        if ($mid -and ($mid -is [string]) -and $mid.Trim() -ne "") {
                            [void]$seenSet.Add($mid.Trim())
                        }
                    }

                    if ($processedItemsList.Count -lt 5000) {
                        $itemKb = [math]::Round([double]$itemBytes / 1024.0, 1)
                        $processedItemsList.Add(@{
                            subject       = if ($subj) { $subj } else { "(Sin Asunto)" }
                            sender        = if ($sender) { $sender } else { "(Desconocido)" }
                            date          = if ($rcvd) { $rcvd.ToString("yyyy-MM-dd HH:mm:ss") } else { "" }
                            source_folder = $relPath
                            dest_folder   = $destFolder.Name
                            status        = "Transferido"
                            size_kb       = $itemKb
                        })
                    }
                } else {
                    $script:totalErrors++
                }
            } catch {
                $errStr = "$_"
                $script:totalErrors++
                if ($errStr -match "0x8001010A|0x80040115|busy|throttle|retry|backoff|quota") {
                    $consecutiveThrottles = [math]::Min(4, $consecutiveThrottles + 1)
                }
                Record-ExecutionError "transfer_error" $relPath $subj $sender "Error al transferir correo: $errStr"
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

            if ($config.adaptive_throttling -and $consecutiveThrottles -gt 0) {
                $throttleSleep = [math]::Min(250, 50 * [math]::Pow(2, [math]::Min(3, $consecutiveThrottles)))
                Start-Sleep -Milliseconds $throttleSleep
            }

            Check-And-Emit-Progress $totalProcessed $totalCandidatesCount $relPath $startTime

            # Recolección periódica optimizada de memoria COM cada 500 correos para evitar saturación de RCWs sin micro-pausas
            if ($totalProcessed % 500 -eq 0) {
                [System.GC]::Collect()
                [System.GC]::WaitForPendingFinalizers()
            }
        }
    }

    # Guardar bitácora temporal JSON de elementos procesados y errores
    $itemsTempFile = Join-Path ([System.IO.Path]::GetTempPath()) "outlook_organizer_transfer_items.json"
    try {
        $itemsJson = if ($processedItemsList.Count -eq 0) { "[]" } else { $processedItemsList | ConvertTo-Json -Depth 4 -Compress }
        [System.IO.File]::WriteAllText($itemsTempFile, $itemsJson, [System.Text.Encoding]::UTF8)
    } catch {}

    $errorsTempFile = Join-Path ([System.IO.Path]::GetTempPath()) "outlook_organizer_transfer_errors.json"
    try {
        $errorsJson = if ($errorsList.Count -eq 0) { "[]" } else { $errorsList | ConvertTo-Json -Depth 4 -Compress }
        [System.IO.File]::WriteAllText($errorsTempFile, $errorsJson, [System.Text.Encoding]::UTF8)
    } catch {}

    $finalStatus = if ($script:isAborted) { "aborted" } else { "completed" }
    Send-Telemetry @{
        type           = "finished"
        status         = $finalStatus
        imported       = $script:totalImported
        imported_bytes = $script:totalImportedBytes
        duplicates     = $script:totalDuplicates
        errors         = $script:totalErrors
    }
    Log-Message "Transferencia finalizada con estado: $finalStatus. Total transferidos: $($script:totalImported), Duplicados omitidos: $($script:totalDuplicates), Errores: $($script:totalErrors)."
}
catch {
    Record-ExecutionError "fatal_error" "" "" "" "Error fatal en transferencia MAPI: $_"
    Send-Telemetry @{
        type           = "finished"
        status         = "failed"
        imported       = $script:totalImported
        imported_bytes = $script:totalImportedBytes
        duplicates     = $script:totalDuplicates
        errors         = ($script:totalErrors + 1)
    }
}
finally {
    Log-Message "Liberando recursos de transferencia..."

    # Restaurar ventana de Outlook si estaba abierta
    if (-not $weStartedOutlook -and $null -ne $outlook) {
        try {
            $exp = $outlook.ActiveExplorer()
            if ($null -ne $exp) {
                $exp.WindowState = 0
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($exp) | Out-Null } catch {}
            }
        } catch {}
    }

    if ($null -ne $sourceStore) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($sourceStore) | Out-Null } catch {}
        $sourceStore = $null
    }
    if ($null -ne $targetStore) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($targetStore) | Out-Null } catch {}
        $targetStore = $null
    }

    if ($weStartedOutlook -and $null -ne $outlook) {
        try { $outlook.Quit() } catch {}
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

    if ($AbortFile -and (Test-Path $AbortFile)) {
        try { Remove-Item -Path $AbortFile -Force -ErrorAction SilentlyContinue } catch {}
    }
    if ($PauseFile -and (Test-Path $PauseFile)) {
        try { Remove-Item -Path $PauseFile -Force -ErrorAction SilentlyContinue } catch {}
    }
}
