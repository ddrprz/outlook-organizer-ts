param (
    [string]$ProfileName = ""
)

$ErrorActionPreference = "Stop"
$outlook = $null
$namespace = $null

try {
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
    } catch {
        $outlook = New-Object -ComObject Outlook.Application
    }

    $namespace = $outlook.GetNamespace("MAPI")
    if ($ProfileName -and $ProfileName.Trim() -ne "") {
        $namespace.Logon($ProfileName.Trim(), "", $false, $false)
    } else {
        $namespace.Logon("", "", $false, $false)
    }

    $stores = @()
    foreach ($s in $namespace.Stores) {
        $type = "Desconocido"
        switch ($s.ExchangeStoreType) {
            0 { $type = "ExchangeOnline" }
            1 { $type = "Delegate" }
            2 { $type = "PublicFolder" }
            3 {
                if ($s.FilePath -and $s.FilePath.ToLower().EndsWith(".pst")) {
                    $type = "PST"
                } else {
                    $type = "MAPI"
                }
            }
            4 { $type = "SharedMailbox" }
            default { $type = "Exchange" }
        }

        $sizeStr = "0 Bytes"
        if ($s.FilePath -and (Test-Path $s.FilePath)) {
            $len = (Get-Item $s.FilePath).Length
            if ($len -ge 1GB) {
                $sizeStr = "{0:N2} GB" -f ($len / 1GB)
            } elseif ($len -ge 1MB) {
                $sizeStr = "{0:N2} MB" -f ($len / 1MB)
            } else {
                $sizeStr = "{0} Bytes" -f $len
            }
        }

        $stores += [PSCustomObject]@{
            display_name = $s.DisplayName
            file_path    = $s.FilePath
            store_type   = $type
            size_display = $sizeStr
        }
    }

    ConvertTo-Json -InputObject @($stores) -Depth 5 -Compress
}
catch {
    Write-Output "[]"
}
finally {
    if ($null -ne $namespace) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($namespace) | Out-Null } catch {}
    }
    if ($null -ne $outlook) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {}
    }
    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}
