param (
    [string]$ProfileName = ""
)

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
$ErrorActionPreference    = "Stop"

try {
    [System.Diagnostics.Process]::GetCurrentProcess().PriorityClass = [System.Diagnostics.ProcessPriorityClass]::BelowNormal
} catch {}

$outlook = $null
$namespace = $null
$weStartedOutlook = $false
$createdPlaceholders = @()

# Auto-reparación preventiva: buscar PSTs huérfanos/inexistentes en el perfil de Outlook
# que causarían diálogos modales de error ("No se encuentra el archivo...") al iniciar la sesión MAPI
try {
    $profileBase = "HKCU:\Software\Microsoft\Office"
    $regKeys = Get-ChildItem $profileBase -Recurse -ErrorAction SilentlyContinue | Where-Object { $_.Name -like "*\Outlook\Profiles\*" }
    foreach ($k in $regKeys) {
        $p = $k.GetValue("001f6700")
        if ($p) {
            $path = if ($p -is [byte[]]) { [System.Text.Encoding]::Unicode.GetString($p).Trim([char]0) } else { [string]$p }
            if ($path -like "*.pst" -and -not (Test-Path $path)) {
                try {
                    $dir = [System.IO.Path]::GetDirectoryName($path)
                    if ($dir -and -not (Test-Path $dir)) {
                        New-Item -ItemType Directory -Path $dir -Force -ErrorAction SilentlyContinue | Out-Null
                    }
                    if (-not (Test-Path $path)) {
                        # Escribir cabecera mínima de 4 bytes para evitar el cuadro de diálogo de Outlook en el arranque
                        [System.IO.File]::WriteAllBytes($path, @(0x21, 0x42, 0x44, 0x4E))
                        $createdPlaceholders += $path
                    }
                } catch {}
            }
        }
    }
} catch {}

try {
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
    } catch {
        $outlook = New-Object -ComObject Outlook.Application
        $weStartedOutlook = $true
    }

    $namespace = $outlook.GetNamespace("MAPI")
    if ($ProfileName -and $ProfileName.Trim() -ne "") {
        try { $namespace.Logon($ProfileName.Trim(), "", $false, $false) } catch {}
    } else {
        try { $namespace.Logon("", "", $false, $false) } catch {}
    }

    # Desmontar limpiamente del perfil cualquier almacén PST fantasma que haya sido detectado
    if ($createdPlaceholders -and $createdPlaceholders.Count -gt 0) {
        foreach ($ph in $createdPlaceholders) {
            $fullPh = [System.IO.Path]::GetFullPath($ph).ToLowerInvariant()
            foreach ($s in $namespace.Stores) {
                $sp = ""
                try { $sp = $s.FilePath } catch {}
                if ($sp -and [System.IO.Path]::GetFullPath($sp).ToLowerInvariant() -eq $fullPh) {
                    try {
                        $rf = $s.GetRootFolder()
                        $namespace.RemoveStore($rf)
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($rf) | Out-Null } catch {}
                    } catch {}
                    break
                }
            }
            try {
                if (Test-Path $ph) {
                    $fi = Get-Item $ph -ErrorAction SilentlyContinue
                    if ($fi -and $fi.Length -le 4) {
                        Remove-Item $ph -Force -ErrorAction SilentlyContinue
                    }
                }
            } catch {}
        }
    }

    function Get-FastStoreUsage($store) {
        $used = [int64]0
        try {
            $root = $store.GetRootFolder()
            if ($root) {
                $queue = [System.Collections.ArrayList]::new()
                $seenFolderIds = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

                # Priorizar carpetas por defecto críticas de MAPI
                # (3 = Eliminados, 6 = Bandeja de entrada, 5 = Enviados, 16 = Borradores, 23 = Correo no deseado)
                $defIds = @(3, 6, 5, 16, 23)
                foreach ($did in $defIds) {
                    try {
                        $df = $store.GetDefaultFolder($did)
                        if ($df) {
                            $fid = ""
                            try { $fid = $df.EntryID } catch {}
                            if ($fid -and -not $seenFolderIds.Contains($fid)) {
                                [void]$seenFolderIds.Add($fid)
                                [void]$queue.Add($df)
                            }
                        }
                    } catch {}
                }

                # Agregar carpetas directas de la raíz del almacén
                try {
                    $rfFolders = $root.Folders
                    if ($rfFolders) {
                        for ($i = 1; $i -le $rfFolders.Count; $i++) {
                            try {
                                $f = $rfFolders.Item($i)
                                $fid = ""
                                try { $fid = $f.EntryID } catch {}
                                if ($fid -and -not $seenFolderIds.Contains($fid)) {
                                    [void]$seenFolderIds.Add($fid)
                                    [void]$queue.Add($f)
                                } elseif (-not $fid) {
                                    [void]$queue.Add($f)
                                }
                            } catch {}
                        }
                    }
                } catch {}

                # Procesar en anchura (BFS) sumando tamaños de forma ultra veloz
                $idx = 0
                while ($idx -lt $queue.Count -and $idx -lt 250) {
                    $current = $queue[$idx]
                    $idx++

                    $fExtSize = [int64]0
                    $fMsgSize = [int64]0

                    # 1. Probar PR_EXTENDED_FOLDER_SIZE (0x36E40014: tamaño acumulado de carpeta + subcarpetas)
                    try {
                        $val = $current.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x36E40014")
                        if ($val -and [int64]$val -gt 0) {
                            $fExtSize = [int64]$val
                        }
                    } catch {}

                    # 2. Probar PR_MESSAGE_SIZE_EXTENDED (0x0E080014: tamaño directo de mensajes en la carpeta)
                    try {
                        $val = $current.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080014")
                        if ($val -and [int64]$val -gt 0) {
                            $fMsgSize = [int64]$val
                        }
                    } catch {}

                    # Si tenemos tamaño extendido que abarca subcarpetas, lo sumamos y evitamos explorar hijos
                    if ($fExtSize -gt 0 -and $fExtSize -ge $fMsgSize) {
                        $used += $fExtSize
                    } else {
                        if ($fMsgSize -gt 0) {
                            $used += $fMsgSize
                        }

                        # Encolar subcarpetas hijas para no omitir datos
                        try {
                            $subs = $current.Folders
                            if ($subs -and $subs.Count -gt 0) {
                                for ($i = 1; $i -le $subs.Count; $i++) {
                                    if ($queue.Count -lt 250) {
                                        try {
                                            $sf = $subs.Item($i)
                                            $sfid = ""
                                            try { $sfid = $sf.EntryID } catch {}
                                            if ($sfid -and -not $seenFolderIds.Contains($sfid)) {
                                                [void]$seenFolderIds.Add($sfid)
                                                [void]$queue.Add($sf)
                                            } elseif (-not $sfid) {
                                                [void]$queue.Add($sf)
                                            }
                                        } catch {}
                                    }
                                }
                            }
                        } catch {}
                    }
                }
            }
        } catch {}

        return $used
    }

    function Get-FastStoreQuota($store) {
        $quotaBytes = [int64]53150220288 # 49.5 GB (cuota estándar Microsoft 365)
        $quotaDisplay = "49.5 GB"
        $isOverQuota = $false
        $excessBytes = [int64]0

        $targets = @($store)
        try {
            $rf = $store.GetRootFolder()
            if ($rf) { $targets += $rf }
        } catch {}

        foreach ($obj in $targets) {
            try {
                $pa = $obj.PropertyAccessor
                if ($null -eq $pa) { continue }

                # 1. Comprobar exceso de almacenamiento (PR_EXCESS_STORAGE_USED = 0x340E0003, en KB)
                try {
                    $excessKb = [int64]$pa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x340E0003")
                    if ($excessKb -gt 0) {
                        $excessBytes = $excessKb * 1024
                        $isOverQuota = $true
                    }
                } catch {}

                # 2. Comprobar restricción de almacenamiento (PR_STORAGE_RESTRICTION_STATE = 0x34130003)
                # 1 = Warning, 2 = Warning, 3 = ProhibitSend, 4 = ProhibitReceive (Lleno total)
                try {
                    $state = [int]$pa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x34130003")
                    if ($state -ge 3) {
                        $isOverQuota = $true
                    }
                } catch {}

                # 3. Comprobar bandera de sobrecuota del servidor (PR_SVR_OVER_QUOTA = 0x340F0003)
                try {
                    $svrOver = $pa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x340F0003")
                    if ($svrOver -eq $true -or [int]$svrOver -eq 1) {
                        $isOverQuota = $true
                    }
                } catch {}

                # 4. Leer límite de cuota (en KB)
                $quotaTags = @(
                    "http://schemas.microsoft.com/mapi/proptag/0x34040003",
                    "http://schemas.microsoft.com/mapi/proptag/0x340D0003",
                    "http://schemas.microsoft.com/mapi/proptag/0x341A0003",
                    "http://schemas.microsoft.com/mapi/proptag/0x34070003"
                )
                foreach ($t in $quotaTags) {
                    try {
                        $val = [int64]$pa.GetProperty($t)
                        if ($val -ge 10485760) { # al menos 10 GB en KB
                            $candidate = $val * 1024
                            $gb = [Math]::Round($candidate / 1GB, 1)
                            if ($gb -ge 10.0) {
                                $quotaBytes = $candidate
                                $quotaDisplay = ("{0:N1} GB" -f $gb)
                                break
                            }
                        }
                    } catch {}
                }
            } catch {}
        }

        return @{
            TotalBytes   = $quotaBytes
            QuotaDisplay = $quotaDisplay
            IsOverQuota  = $isOverQuota
            ExcessBytes  = $excessBytes
        }
    }

    $discovered = [System.Collections.Generic.List[hashtable]]::new()
    $seenKeys = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

    # 1. Explorar Almacenes (Stores) MAPI
    foreach ($s in $namespace.Stores) {
        $filePath = if ($s.FilePath) { $s.FilePath.Trim() } else { "" }
        $dispName = if ($s.DisplayName) { $s.DisplayName.Trim() } else { "" }

        # EXCLUIR ARCHIVOS PST: Un PST es un archivo de origen o archivo local, NUNCA un buzón de destino MAPI
        if ($filePath -and $filePath.ToLower().EndsWith(".pst")) {
            continue
        }
        if ($s.ExchangeStoreType -eq 3 -and ($filePath -like "*.pst" -or $dispName.ToLower().EndsWith(".pst"))) {
            continue
        }

        # EXCLUIR CARPETAS PÚBLICAS
        if ($s.ExchangeStoreType -eq 2) {
            continue
        }

        # Clasificación del tipo de buzón destino
        $type = switch ($s.ExchangeStoreType) {
            0 { "ExchangeOnline" }
            1 { "Delegate" }
            4 { "SharedMailbox" }
            default {
                if ($filePath.ToLower().EndsWith(".ost")) {
                    "Exchange"
                } else {
                    "MAPI"
                }
            }
        }

        # Cálculo de tamaño usado
        $usedBytes = [int64]0
        if ($filePath -and (Test-Path $filePath)) {
            try { $usedBytes = [int64](Get-Item $filePath).Length } catch {}
        }
        if ($usedBytes -le 0) {
            $usedBytes = Get-FastStoreUsage $s
        }

        # Cálculo de cuota y detección de sobrecuota
        $quotaData = Get-FastStoreQuota $s
        $totalBytes = $quotaData.TotalBytes
        $quotaStr   = $quotaData.QuotaDisplay

        if ($quotaData.IsOverQuota) {
            if ($quotaData.ExcessBytes -gt 0) {
                $usedBytes = [Math]::Max($usedBytes, ($totalBytes + $quotaData.ExcessBytes))
            } else {
                $usedBytes = [Math]::Max($usedBytes, $totalBytes)
            }
        }

        # Porcentaje de ocupación
        $usagePercent = 0.0
        if ($totalBytes -gt 0) {
            $usagePercent = [Math]::Round(($usedBytes / $totalBytes) * 100, 1)
            if ($usagePercent -gt 100.0) { $usagePercent = 100.0 }
            if ($usagePercent -lt 0.0) { $usagePercent = 0.0 }
        }

        $sizeStr = if ($usedBytes -ge 1GB) {
            "{0:N1} GB" -f ($usedBytes / 1GB)
        } elseif ($usedBytes -ge 1MB) {
            "{0:N1} MB" -f ($usedBytes / 1MB)
        } else {
            "0.0 GB"
        }

        $key = if ($dispName) { $dispName.ToLower() } else { $filePath.ToLower() }
        if ($key -and -not $seenKeys.Contains($key)) {
            [void]$seenKeys.Add($key)
            $discovered.Add(@{
                display_name  = $dispName
                file_path     = if ($filePath) { $filePath } else { $null }
                store_type    = $type
                size_display  = $sizeStr
                used_bytes    = $usedBytes
                total_bytes   = $totalBytes
                quota_display = $quotaStr
                usage_percent = $usagePercent
            })
        }
    }

    # 2. Explorar Cuentas configuradas (Accounts) para capturar buzones adicionales o buzones delegados
    try {
        foreach ($acc in $namespace.Accounts) {
            $accName = if ($acc.DisplayName) { $acc.DisplayName.Trim() } else { "" }
            $smtp = if ($acc.SmtpAddress) { $acc.SmtpAddress.Trim() } else { "" }
            $nameToUse = if ($smtp) { $smtp } else { $accName }

            $key = $nameToUse.ToLower()
            if ($nameToUse -and -not $seenKeys.Contains($key)) {
                $delStore = $null
                try { $delStore = $acc.DeliveryStore } catch {}

                $delFilePath = if ($delStore -and $delStore.FilePath) { $delStore.FilePath.Trim() } else { $null }
                if ($delFilePath -and $delFilePath.ToLower().EndsWith(".pst")) {
                    continue
                }

                $delType = if ($delStore) {
                    switch ($delStore.ExchangeStoreType) {
                        0 { "ExchangeOnline" }
                        1 { "Delegate" }
                        4 { "SharedMailbox" }
                        default { "Exchange" }
                    }
                } else {
                    "ExchangeOnline"
                }

                $usedBytes = [int64]0
                if ($delFilePath -and (Test-Path $delFilePath)) {
                    try { $usedBytes = [int64](Get-Item $delFilePath).Length } catch {}
                }
                if ($delStore -and $usedBytes -le 0) {
                    $usedBytes = Get-FastStoreUsage $delStore
                }

                $quotaData = if ($delStore) { Get-FastStoreQuota $delStore } else { @{ TotalBytes = [int64]53150220288; QuotaDisplay = "49.5 GB"; IsOverQuota = $false; ExcessBytes = [int64]0 } }
                $totalBytes = $quotaData.TotalBytes
                $quotaStr   = $quotaData.QuotaDisplay

                if ($quotaData.IsOverQuota) {
                    if ($quotaData.ExcessBytes -gt 0) {
                        $usedBytes = [Math]::Max($usedBytes, ($totalBytes + $quotaData.ExcessBytes))
                    } else {
                        $usedBytes = [Math]::Max($usedBytes, $totalBytes)
                    }
                }

                $usagePercent = 0.0
                if ($totalBytes -gt 0) {
                    $usagePercent = [Math]::Round(($usedBytes / $totalBytes) * 100, 1)
                    if ($usagePercent -gt 100.0) { $usagePercent = 100.0 }
                    if ($usagePercent -lt 0.0) { $usagePercent = 0.0 }
                }

                $sizeStr = if ($usedBytes -ge 1GB) {
                    "{0:N1} GB" -f ($usedBytes / 1GB)
                } elseif ($usedBytes -ge 1MB) {
                    "{0:N1} MB" -f ($usedBytes / 1MB)
                } else {
                    "0.0 GB"
                }

                [void]$seenKeys.Add($key)
                $discovered.Add(@{
                    display_name  = $nameToUse
                    file_path     = $delFilePath
                    store_type    = $delType
                    size_display  = $sizeStr
                    used_bytes    = $usedBytes
                    total_bytes   = $totalBytes
                    quota_display = $quotaStr
                    usage_percent = $usagePercent
                })
            }
        }
    } catch {}

    $arr = @($discovered)
    ConvertTo-Json -InputObject $arr -Depth 5 -Compress
}
catch {
    Write-Output "[]"
}
finally {
    if ($createdPlaceholders -and $createdPlaceholders.Count -gt 0) {
        foreach ($ph in $createdPlaceholders) {
            try {
                if (Test-Path $ph) {
                    $fi = Get-Item $ph -ErrorAction SilentlyContinue
                    if ($fi -and $fi.Length -le 4) {
                        Remove-Item $ph -Force -ErrorAction SilentlyContinue
                    }
                }
            } catch {}
        }
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
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}

