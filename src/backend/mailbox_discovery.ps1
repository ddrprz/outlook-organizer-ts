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

    function Get-StoreDataSize($store) {
        $bytes = [int64]0
        try {
            $root = $store.GetRootFolder()
            if ($root) {
                # 1. Intentar leer tamaño extendido total en la raíz (PR_EXTENDED_FOLDER_SIZE = 0x36E40014)
                try {
                    $ext = $root.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x36E40014")
                    if ($ext -and [int64]$ext -gt 1MB) {
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($root) | Out-Null } catch {}
                        return [int64]$ext
                    }
                } catch {}

                # 2. Recorrer las carpetas principales del buzón (Bandeja de entrada, Enviados, Eliminados, etc.)
                $folders = $null
                try { $folders = $root.Folders } catch {}
                if ($folders) {
                    foreach ($f in $folders) {
                        try {
                            $fSize = $null
                            # PR_EXTENDED_FOLDER_SIZE (0x36E40014): tamaño acumulado con subcarpetas
                            try { $fSize = $f.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x36E40014") } catch {}
                            if ($fSize -and [int64]$fSize -gt 0) {
                                $bytes += [int64]$fSize
                            } else {
                                # PR_MESSAGE_SIZE_EXTENDED (0x0E080014): tamaño de mensajes directos
                                try {
                                    $mSize = $f.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080014")
                                    if ($mSize -and [int64]$mSize -gt 0) {
                                        $bytes += [int64]$mSize
                                    }
                                } catch {}

                                # Subcarpetas de segundo nivel
                                try {
                                    foreach ($sub in $f.Folders) {
                                        try {
                                            $subSize = $sub.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080014")
                                            if ($subSize -and [int64]$subSize -gt 0) {
                                                $bytes += [int64]$subSize
                                            }
                                        } catch {}
                                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($sub) | Out-Null } catch {}
                                    }
                                } catch {}
                            }
                        } catch {}
                        finally {
                            try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($f) | Out-Null } catch {}
                        }
                    }
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($folders) | Out-Null } catch {}
                }
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($root) | Out-Null } catch {}
            }
        } catch {}
        return $bytes
    }

    function Get-StoreQuota($store) {
        # Cuota estándar en Microsoft 365 (49.5 GB utilizables para SharedMailbox / Exchange)
        $standardBytes = [int64]53150220288 # 49.5 GB
        $standardStr   = "49.5 GB"

        $propTags = @(
            "http://schemas.microsoft.com/mapi/proptag/0x34040003", # PR_STORAGE_QUOTA_LIMIT (KB)
            "http://schemas.microsoft.com/mapi/proptag/0x341A0003", # PR_STORAGE_QUOTA_LIMIT_EXTENDED (KB)
            "http://schemas.microsoft.com/mapi/proptag/0x34050003"  # PR_QUOTA_WARNING (KB)
        )

        foreach ($tag in $propTags) {
            try {
                $val = $store.PropertyAccessor.GetProperty($tag)
                if ($val) {
                    $kb = [int64]$val
                    # Si la cuota reportada es >= 10 GB (10,485,760 KB), aceptarla (ej. 49.5 GB, 50 GB, 100 GB)
                    if ($kb -ge 10485760) {
                        $bytes = $kb * 1024
                        $gb = [Math]::Round($bytes / 1GB, 1)
                        return @{ TotalBytes = $bytes; QuotaDisplay = ("{0:N1} GB" -f $gb) }
                    }
                }
            } catch {}
        }

        return @{ TotalBytes = $standardBytes; QuotaDisplay = $standardStr }
    }

    $discovered = [System.Collections.Generic.List[hashtable]]::new()
    $seenKeys = [System.Collections.Generic.HashSet[string]]::new([System.StringComparer]::OrdinalIgnoreCase)

    # 1. Explorar Almacenes (Stores) MAPI
    foreach ($s in $namespace.Stores) {
        $filePath = if ($s.FilePath) { $s.FilePath.Trim() } else { "" }
        $dispName = if ($s.DisplayName) { $dispName = $s.DisplayName.Trim(); $dispName } else { "" }

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
            $usedBytes = Get-StoreDataSize $s
        }

        # Cálculo de cuota
        $quotaInfo = Get-StoreQuota $s
        $totalBytes = $quotaInfo.TotalBytes
        $quotaStr   = $quotaInfo.QuotaDisplay

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
                    $usedBytes = Get-StoreDataSize $delStore
                }

                $quotaInfo = if ($delStore) { Get-StoreQuota $delStore } else { @{ TotalBytes = [int64]53150220288; QuotaDisplay = "49.5 GB" } }
                $totalBytes = $quotaInfo.TotalBytes
                $quotaStr   = $quotaInfo.QuotaDisplay

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

