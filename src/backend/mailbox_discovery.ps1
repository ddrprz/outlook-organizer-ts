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

    function Get-FastStoreMetrics($store, $filePath) {
        $pa = $null
        try { $pa = $store.PropertyAccessor } catch {}

        # 1. Tamaño usado real reportado por Exchange (en bytes)
        $usedBytes = [int64]0
        if ($pa) {
            try {
                $val = $pa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080014") # PR_MESSAGE_SIZE_EXTENDED
                if ($val -and [int64]$val -gt 0) { $usedBytes = [int64]$val }
            } catch {}
            if ($usedBytes -le 0) {
                try {
                    $val = $pa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080003") # PR_MESSAGE_SIZE (32-bit fallback)
                    if ($val -and [int64]$val -gt 0) { $usedBytes = [int64]$val }
                } catch {}
            }
        }

        # Si no se pudo obtener del almacén raíz, sumar carpetas de primer nivel
        if ($usedBytes -le 0) {
            try {
                $rf = $store.GetRootFolder()
                if ($rf -and $rf.Folders) {
                    $count = [Math]::Min($rf.Folders.Count, 30)
                    for ($i = 1; $i -le $count; $i++) {
                        try {
                            $f = $rf.Folders.Item($i)
                            $fPa = $f.PropertyAccessor
                            $fSz = [int64]0
                            try { $fSz = [int64]$fPa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080014") } catch {}
                            if ($fSz -le 0) {
                                try { $fSz = [int64]$fPa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x36E40014") } catch {}
                            }
                            if ($fSz -gt 0) { $usedBytes += $fSz }
                        } catch {}
                    }
                }
            } catch {}
        }

        # Como último recurso (por ejemplo almacén desconectado), usar tamaño de archivo .ost
        if ($usedBytes -le 0 -and $filePath -and (Test-Path $filePath)) {
            try { $usedBytes = [int64](Get-Item $filePath).Length } catch {}
        }

        # 2. Cuota de almacenamiento dinámica reportada por Exchange (en KB)
        $quotaKb = [int64]0
        if ($pa) {
            foreach ($qTag in @(
                "http://schemas.microsoft.com/mapi/proptag/0x341C0003", # PR_QUOTA_SEND_THRESHOLD
                "http://schemas.microsoft.com/mapi/proptag/0x341A0003", # PR_STORAGE_QUOTA_LIMIT
                "http://schemas.microsoft.com/mapi/proptag/0x341B0003"  # PR_QUOTA_WARNING_THRESHOLD
            )) {
                try {
                    $qVal = $pa.GetProperty($qTag)
                    if ($qVal -and [int64]$qVal -gt $quotaKb) {
                        $quotaKb = [int64]$qVal
                    }
                } catch {}
            }
        }

        $totalBytes = [int64]0
        if ($quotaKb -gt 0) {
            $totalBytes = [int64]($quotaKb * 1024)
        } else {
            # Si Exchange no expone cuotas explícitas, inferir cuota dinámica según uso
            if ($usedBytes -gt (50GB)) {
                $totalBytes = [int64](100GB)
            } else {
                $totalBytes = [int64](50GB)
            }
        }

        # Detección de restricción de almacenamiento (bloqueo real por cuota)
        $isRestricted = $false
        if ($pa) {
            try {
                $state = [int]$pa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x34130003")
                if ($state -ge 3) { $isRestricted = $true }
            } catch {}
        }

        if ($isRestricted -and $usedBytes -lt $totalBytes) {
            $usedBytes = $totalBytes
        }

        # Porcentaje de ocupación
        $usagePercent = 0.0
        if ($totalBytes -gt 0) {
            $usagePercent = [Math]::Round(([double]$usedBytes / [double]$totalBytes) * 100.0, 1)
            if ($usagePercent -gt 100.0) { $usagePercent = 100.0 }
            if ($usagePercent -lt 0.0) { $usagePercent = 0.0 }
        }

        $quotaDisplay = if ($totalBytes -ge 1GB) {
            $gbVal = [Math]::Round($totalBytes / 1GB, 1)
            if ($gbVal -eq [Math]::Floor($gbVal)) {
                "{0:N0} GB" -f $gbVal
            } else {
                "{0:N1} GB" -f $gbVal
            }
        } else {
            "{0:N0} MB" -f ($totalBytes / 1MB)
        }

        $sizeDisplay = if ($usedBytes -ge 1GB) {
            "{0:N1} GB" -f ($usedBytes / 1GB)
        } elseif ($usedBytes -ge 1MB) {
            "{0:N1} MB" -f ($usedBytes / 1MB)
        } else {
            "0.0 GB"
        }

        return @{
            used_bytes    = $usedBytes
            total_bytes   = $totalBytes
            size_display  = $sizeDisplay
            quota_display = $quotaDisplay
            usage_percent = $usagePercent
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

        # Cálculo dinámico de tamaño y cuota
        $metrics = Get-FastStoreMetrics $s $filePath

        $key = if ($dispName) { $dispName.ToLower() } else { $filePath.ToLower() }
        if ($key -and -not $seenKeys.Contains($key)) {
            [void]$seenKeys.Add($key)
            $discovered.Add(@{
                display_name  = $dispName
                file_path     = if ($filePath) { $filePath } else { $null }
                store_type    = $type
                size_display  = $metrics.size_display
                used_bytes    = $metrics.used_bytes
                total_bytes   = $metrics.total_bytes
                quota_display = $metrics.quota_display
                usage_percent = $metrics.usage_percent
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

                $metrics = if ($delStore) {
                    Get-FastStoreMetrics $delStore $delFilePath
                } else {
                    @{
                        used_bytes    = [int64]0
                        total_bytes   = [int64]107374182400
                        size_display  = "0.0 GB"
                        quota_display = "100 GB"
                        usage_percent = 0.0
                    }
                }

                [void]$seenKeys.Add($key)
                $discovered.Add(@{
                    display_name  = $nameToUse
                    file_path     = $delFilePath
                    store_type    = $delType
                    size_display  = $metrics.size_display
                    used_bytes    = $metrics.used_bytes
                    total_bytes   = $metrics.total_bytes
                    quota_display = $metrics.quota_display
                    usage_percent = $metrics.usage_percent
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

