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

    # Función de alta velocidad para obtener rango de fechas usando ordenamiento indexado MAPI
    function Get-FolderDateRange($folder) {
        $dateProps = @(
            @{ Name = "ReceivedTime"; Dasl = "http://schemas.microsoft.com/mapi/proptag/0x0E060040" },
            @{ Name = "SentOn";       Dasl = "http://schemas.microsoft.com/mapi/proptag/0x00390040" }
        )
        if ($folder.Name -match "enviad|sent") {
            $dateProps = @(
                @{ Name = "SentOn";       Dasl = "http://schemas.microsoft.com/mapi/proptag/0x00390040" },
                @{ Name = "ReceivedTime"; Dasl = "http://schemas.microsoft.com/mapi/proptag/0x0E060040" }
            )
        }

        foreach ($dp in $dateProps) {
            $pName = $dp.Name
            $pDasl = $dp.Dasl
            try {
                $tblMax = $folder.GetTable()
                $tblMax.Columns.Add($pName) | Out-Null
                $tblMax.Sort($pName, $true)
                if (-not $tblMax.EndOfTable) {
                    $row = $tblMax.GetNextRow()
                    $valMax = $row.Item($pName)
                    if ($null -ne $valMax -and ($valMax -is [DateTime]) -and $valMax.Year -ge 1980 -and $valMax.Year -le 2050) {
                        $tblMin = $folder.GetTable()
                        $tblMin.Columns.Add($pName) | Out-Null
                        $tblMin.Sort($pName, $false)
                        if (-not $tblMin.EndOfTable) {
                            $rowMin = $tblMin.GetNextRow()
                            $valMin = $rowMin.Item($pName)
                            if ($null -ne $valMin -and ($valMin -is [DateTime]) -and $valMin.Year -ge 1980 -and $valMin.Year -le 2050) {
                                return @{
                                    MinDate  = $valMin
                                    MaxDate  = $valMax
                                    DaslProp = $pDasl
                                }
                            }
                        }
                    }
                }
            } catch {}
        }
        return $null
    }

    # Función recursiva para explorar carpetas de forma ultrarrápida
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

            $range = Get-FolderDateRange $folder
            if ($range) {
                if ($range.MinDate -lt $script:minDate) { $script:minDate = $range.MinDate }
                if ($range.MaxDate -gt $script:maxDate) { $script:maxDate = $range.MaxDate }

                $yMin = $range.MinDate.Year
                $yMax = $range.MaxDate.Year

                # Registrar siempre los años y meses de los extremos confirmados
                $script:yearsSet[$yMin] = $true
                $script:yearsSet[$yMax] = $true
                $yMinStr = "$yMin"
                $yMaxStr = "$yMax"
                if (-not $script:yearMonthsMap.ContainsKey($yMinStr)) { $script:yearMonthsMap[$yMinStr] = @{} }
                if (-not $script:yearMonthsMap.ContainsKey($yMaxStr)) { $script:yearMonthsMap[$yMaxStr] = @{} }
                $script:yearMonthsMap[$yMinStr][$range.MinDate.Month] = $true
                $script:yearMonthsMap[$yMaxStr][$range.MaxDate.Month] = $true

                # Si abarca varios años o meses, consultar la presencia mediante índices DASL instantáneos
                if ($yMin -ne $yMax -or $range.MinDate.Month -ne $range.MaxDate.Month) {
                    for ($y = $yMin; $y -le $yMax; $y++) {
                        $yStr = "$y"
                        $hasYear = $true
                        if ($y -ne $yMin -and $y -ne $yMax) {
                            $yStart = (Get-Date -Year $y -Month 1 -Day 1 -Hour 0 -Minute 0 -Second 0).ToString("yyyy-MM-dd HH:mm:ss")
                            $yEnd = (Get-Date -Year ($y + 1) -Month 1 -Day 1 -Hour 0 -Minute 0 -Second 0).ToString("yyyy-MM-dd HH:mm:ss")
                            $filterY = "@SQL=""$($range.DaslProp)"" >= '$yStart' AND ""$($range.DaslProp)"" < '$yEnd'"
                            try {
                                $tblY = $folder.GetTable($filterY)
                                $hasYear = ($tblY -and $tblY.GetRowCount() -gt 0)
                            } catch { $hasYear = $true }
                        }

                        if ($hasYear) {
                            $script:yearsSet[$y] = $true
                            if (-not $script:yearMonthsMap.ContainsKey($yStr)) {
                                $script:yearMonthsMap[$yStr] = @{}
                            }

                            $mStart = if ($y -eq $yMin) { $range.MinDate.Month } else { 1 }
                            $mEnd = if ($y -eq $yMax) { $range.MaxDate.Month } else { 12 }

                            for ($m = $mStart; $m -le $mEnd; $m++) {
                                # Si ya está registrado en este año, saltar consulta
                                if ($script:yearMonthsMap[$yStr].ContainsKey($m)) { continue }

                                $dtStart = (Get-Date -Year $y -Month $m -Day 1 -Hour 0 -Minute 0 -Second 0)
                                $dtEnd = $dtStart.AddMonths(1)
                                $filterM = "@SQL=""$($range.DaslProp)"" >= '$($dtStart.ToString("yyyy-MM-dd HH:mm:ss"))' AND ""$($range.DaslProp)"" < '$($dtEnd.ToString("yyyy-MM-dd HH:mm:ss"))'"
                                try {
                                    $tblM = $folder.GetTable($filterM)
                                    if ($tblM -and $tblM.GetRowCount() -gt 0) {
                                        $script:yearMonthsMap[$yStr][$m] = $true
                                    }
                                } catch {}
                            }
                        }
                    }
                }
            } else {
                # Fallback seguro con Items si GetTable no pudo ordenar por columnas
                try {
                    $items = $folder.Items
                    try {
                        $items.Sort("[ReceivedTime]", $true)
                        $firstItem = $items.Item(1)
                        if ($firstItem) {
                            $t = $null
                            try { $t = $firstItem.ReceivedTime } catch {}
                            if ($null -eq $t -or -not ($t -is [DateTime])) { try { $t = $firstItem.SentOn } catch {} }
                            if ($t -is [DateTime] -and $t.Year -ge 1980 -and $t.Year -le 2050) {
                                if ($t -gt $script:maxDate) { $script:maxDate = $t }
                                $script:yearsSet[$t.Year] = $true
                                $yStr = "$($t.Year)"
                                if (-not $script:yearMonthsMap.ContainsKey($yStr)) { $script:yearMonthsMap[$yStr] = @{} }
                                $script:yearMonthsMap[$yStr][$t.Month] = $true
                            }
                            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($firstItem) | Out-Null
                        }
                    } catch {}

                    try {
                        $items.Sort("[ReceivedTime]", $false)
                        $lastItem = $items.Item(1)
                        if ($lastItem) {
                            $t = $null
                            try { $t = $lastItem.ReceivedTime } catch {}
                            if ($null -eq $t -or -not ($t -is [DateTime])) { try { $t = $lastItem.SentOn } catch {} }
                            if ($t -is [DateTime] -and $t.Year -ge 1980 -and $t.Year -le 2050) {
                                if ($t -lt $script:minDate) { $script:minDate = $t }
                                $script:yearsSet[$t.Year] = $true
                                $yStr = "$($t.Year)"
                                if (-not $script:yearMonthsMap.ContainsKey($yStr)) { $script:yearMonthsMap[$yStr] = @{} }
                                $script:yearMonthsMap[$yStr][$t.Month] = $true
                            }
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
