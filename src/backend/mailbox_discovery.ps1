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

        # Cálculo de tamaño o estado Online
        $sizeStr = "Online (Nube)"
        $usedBytes = $null
        $totalBytes = 53687091200 # 50 GB estándar M365 (50 * 1024^3)
        $quotaStr = "50 GB"
        $usagePercent = 0.0

        if ($filePath -and (Test-Path $filePath)) {
            try {
                $len = (Get-Item $filePath).Length
                $usedBytes = [int64]$len
                if ($len -ge 1GB) {
                    $sizeStr = "{0:N2} GB" -f ($len / 1GB)
                } elseif ($len -ge 1MB) {
                    $sizeStr = "{0:N2} MB" -f ($len / 1MB)
                } else {
                    $sizeStr = "{0} Bytes" -f $len
                }
            } catch {
                $sizeStr = "Online (Nube)"
            }
        }

        # Intentar consultar cuota MAPI en el Store si está expuesta (PR_STORAGE_QUOTA_LIMIT = 0x34040003 en KB)
        try {
            $quotaKb = $s.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x34040003")
            if ($quotaKb -and $quotaKb -gt 0) {
                $totalBytes = [int64]$quotaKb * 1024
                $quotaStr = "{0:N0} GB" -f ($totalBytes / 1GB)
            }
        } catch {}

        # Si aún no tenemos bytes usados (modo online sin OST local), intentar consultar tamaño de la raíz (PR_MESSAGE_SIZE_EXTENDED = 0x0E080014)
        if ($null -eq $usedBytes) {
            try {
                $rootFolder = $s.GetRootFolder()
                if ($rootFolder) {
                    try {
                        $storeSize = $rootFolder.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080014")
                        if ($storeSize -and $storeSize -gt 0) {
                            $usedBytes = [int64]$storeSize
                            if ($usedBytes -ge 1GB) {
                                $sizeStr = "{0:N2} GB" -f ($usedBytes / 1GB)
                            } elseif ($usedBytes -ge 1MB) {
                                $sizeStr = "{0:N2} MB" -f ($usedBytes / 1MB)
                            } else {
                                $sizeStr = "{0} Bytes" -f $usedBytes
                            }
                        }
                    } catch {}
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($rootFolder) | Out-Null } catch {}
                }
            } catch {}
        }

        if ($usedBytes -and $totalBytes -gt 0) {
            $usagePercent = [Math]::Round(($usedBytes / $totalBytes) * 100, 1)
            if ($usagePercent -gt 100.0) { $usagePercent = 100.0 }
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

                $sizeStr = "Online (Nube)"
                $usedBytes = $null
                $totalBytes = 53687091200
                $quotaStr = "50 GB"
                $usagePercent = 0.0

                if ($delFilePath -and (Test-Path $delFilePath)) {
                    try {
                        $len = (Get-Item $delFilePath).Length
                        $usedBytes = [int64]$len
                        if ($len -ge 1GB) {
                            $sizeStr = "{0:N2} GB" -f ($len / 1GB)
                        } elseif ($len -ge 1MB) {
                            $sizeStr = "{0:N2} MB" -f ($len / 1MB)
                        } else {
                            $sizeStr = "{0} Bytes" -f $len
                        }
                    } catch {}
                }

                if ($delStore) {
                    try {
                        $quotaKb = $delStore.PropertyAccessor.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x34040003")
                        if ($quotaKb -and $quotaKb -gt 0) {
                            $totalBytes = [int64]$quotaKb * 1024
                            $quotaStr = "{0:N0} GB" -f ($totalBytes / 1GB)
                        }
                    } catch {}
                }

                if ($usedBytes -and $totalBytes -gt 0) {
                    $usagePercent = [Math]::Round(($usedBytes / $totalBytes) * 100, 1)
                    if ($usagePercent -gt 100.0) { $usagePercent = 100.0 }
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

