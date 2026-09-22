param (
    [string]$PstPath = "",
    [string]$ProfileName = ""
)

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
$ErrorActionPreference    = "Stop"

if (-not $PstPath -or -not (Test-Path $PstPath)) {
    $errObj = @{
        error = "El archivo PST no existe en la ruta especificada: '$PstPath'"
    }
    Write-Output ($errObj | ConvertTo-Json -Compress)
    exit 0
}

$fileItem = Get-Item $PstPath
$fileName = $fileItem.Name
$sizeMb = [math]::Round($fileItem.Length / 1MB, 2)

$outlook = $null
$namespace = $null
$pstStore = $null
$wasMountedByUs = $false

$foldersList = @()
$totalItems = 0
$yearsSet = @{}
$yearMonthsMap = @{}
$minDate = [DateTime]::MaxValue
$maxDate = [DateTime]::MinValue

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
        try {
            $namespace.Logon("", "", $false, $false)
        } catch {}
    }

    # Verificar si el PST ya está montado
    foreach ($s in $namespace.Stores) {
        if ($s.FilePath -and ($s.FilePath.Trim().ToLower() -eq $PstPath.Trim().ToLower())) {
            $pstStore = $s
            break
        }
    }

    if (-not $pstStore) {
        $namespace.AddStoreEx($PstPath, 1) # 1 = olStoreDefault (Unicode)
        Start-Sleep -Milliseconds 150
        foreach ($s in $namespace.Stores) {
            if ($s.FilePath -and ($s.FilePath.Trim().ToLower() -eq $PstPath.Trim().ToLower())) {
                $pstStore = $s
                $wasMountedByUs = $true
                break
            }
        }
    }

    if (-not $pstStore) {
        $errObj = @{
            error = "No se pudo montar el almacén MAPI para el archivo PST: '$fileName'"
        }
        Write-Output ($errObj | ConvertTo-Json -Compress)
        exit 0
    }

    $root = $pstStore.GetRootFolder()

    # Función recursiva para explorar carpetas
    function Inspect-Folder($folder) {
        $fName = $folder.Name
        $fCount = 0
        try {
            $fCount = $folder.Items.Count
        } catch {}

        if ($fCount -gt 0) {
            $script:totalItems += $fCount
            $script:foldersList += @{
                name  = $fName
                count = $fCount
            }

            # Extracción eficiente de fechas con Table o Items
            $tableSuccess = $false
            try {
                $table = $folder.GetTable()
                if ($table) {
                    $tableSuccess = $true
                    while (-not $table.EndOfTable) {
                        $row = $table.GetNextRow()
                        $rcvd = $null
                        try { $rcvd = $row["ReceivedTime"] } catch {}
                        if ($null -ne $rcvd -and ($rcvd -is [DateTime])) {
                            if ($rcvd.Year -ge 1980 -and $rcvd.Year -le 2050) {
                                $script:yearsSet[$rcvd.Year] = $true
                                $yStr = "$($rcvd.Year)"
                                if (-not $script:yearMonthsMap.ContainsKey($yStr)) {
                                    $script:yearMonthsMap[$yStr] = @{}
                                }
                                $script:yearMonthsMap[$yStr][$rcvd.Month] = $true

                                if ($rcvd -lt $script:minDate) { $script:minDate = $rcvd }
                                if ($rcvd -gt $script:maxDate) { $script:maxDate = $rcvd }
                            }
                        }
                    }
                }
            } catch {
                $tableSuccess = $false
            }

            # Fallback en caso de que GetTable no esté disponible en esta versión de MAPI
            if (-not $tableSuccess) {
                try {
                    $items = $folder.Items
                    try {
                        $items.Sort("[ReceivedTime]", $true) # Descendente: más reciente primero
                        $firstItem = $items.Item(1)
                        if ($firstItem) {
                            $t = $null
                            try { $t = $firstItem.ReceivedTime } catch {}
                            if ($t -is [DateTime] -and $t -gt $script:maxDate) { $script:maxDate = $t }
                            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($firstItem) | Out-Null
                        }
                    } catch {}

                    try {
                        $items.Sort("[ReceivedTime]", $false) # Ascendente: más antiguo primero
                        $lastItem = $items.Item(1)
                        if ($lastItem) {
                            $t = $null
                            try { $t = $lastItem.ReceivedTime } catch {}
                            if ($t -is [DateTime] -and $t -lt $script:minDate) { $script:minDate = $t }
                            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($lastItem) | Out-Null
                        }
                    } catch {}
                } catch {}
            }
        }

        # Subcarpetas
        try {
            foreach ($sub in $folder.Folders) {
                Inspect-Folder $sub
            }
        } catch {}
    }

    foreach ($subF in $root.Folders) {
        Inspect-Folder $subF
    }

    # Estructurar resultado
    $sortedYears = $yearsSet.Keys | ForEach-Object { [int]$_ } | Sort-Object
    $finalYearMonths = @{}
    foreach ($y in $sortedYears) {
        $yStr = "$y"
        if ($yearMonthsMap.ContainsKey($yStr)) {
            $mList = $yearMonthsMap[$yStr].Keys | ForEach-Object { [int]$_ } | Sort-Object
            $finalYearMonths[$yStr] = @($mList)
        } else {
            $finalYearMonths[$yStr] = @()
        }
    }

    $lastDateStr = if ($maxDate -gt [DateTime]::MinValue) { $maxDate.ToString("yyyy-MM-dd HH:mm:ss") } else { $null }
    $firstDateStr = if ($minDate -lt [DateTime]::MaxValue) { $minDate.ToString("yyyy-MM-dd HH:mm:ss") } else { $null }

    if ($sortedYears.Count -eq 0 -and $minDate -lt [DateTime]::MaxValue -and $maxDate -gt [DateTime]::MinValue) {
        $yMin = $minDate.Year
        $yMax = $maxDate.Year
        for ($y = $yMin; $y -le $yMax; $y++) {
            $sortedYears += $y
            $finalYearMonths["$y"] = @(1..12)
        }
    }

    $result = @{
        file_name        = $fileName
        file_path        = $PstPath
        size_mb          = $sizeMb
        total_items      = $totalItems
        last_email_date  = $lastDateStr
        first_email_date = $firstDateStr
        folders          = $foldersList
        years            = @($sortedYears)
        year_months      = $finalYearMonths
    }

    Write-Output ($result | ConvertTo-Json -Compress)
}
catch {
    $errObj = @{
        error = "Excepción al inspeccionar PST: $_"
    }
    Write-Output ($errObj | ConvertTo-Json -Compress)
}
finally {
    if ($wasMountedByUs -and $null -ne $pstStore) {
        try {
            $rootF = $pstStore.GetRootFolder()
            $namespace.RemoveStore($rootF)
        } catch {}
    }

    if ($null -ne $namespace) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($namespace) | Out-Null } catch {}
    }
    if ($null -ne $outlook) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($outlook) | Out-Null } catch {}
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()
}
