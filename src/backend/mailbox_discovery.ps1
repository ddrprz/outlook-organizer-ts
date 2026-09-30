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
        if ($filePath -and (Test-Path $filePath)) {
            try {
                $len = (Get-Item $filePath).Length
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

        $key = if ($dispName) { $dispName.ToLower() } else { $filePath.ToLower() }
        if ($key -and -not $seenKeys.Contains($key)) {
            [void]$seenKeys.Add($key)
            $discovered.Add(@{
                display_name = $dispName
                file_path    = if ($filePath) { $filePath } else { $null }
                store_type   = $type
                size_display = $sizeStr
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
                if ($delFilePath -and (Test-Path $delFilePath)) {
                    try {
                        $len = (Get-Item $delFilePath).Length
                        if ($len -ge 1GB) {
                            $sizeStr = "{0:N2} GB" -f ($len / 1GB)
                        } elseif ($len -ge 1MB) {
                            $sizeStr = "{0:N2} MB" -f ($len / 1MB)
                        } else {
                            $sizeStr = "{0} Bytes" -f $len
                        }
                    } catch {}
                }

                [void]$seenKeys.Add($key)
                $discovered.Add(@{
                    display_name = $nameToUse
                    file_path    = $delFilePath
                    store_type   = $delType
                    size_display = $sizeStr
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

