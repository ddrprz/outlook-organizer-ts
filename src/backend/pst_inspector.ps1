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
$globalCountsByYear = @{}
$globalCountsByMonth = @{}
$globalSizesByYear = @{}
$globalSizesByMonth = @{}
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

    # Función recursiva para explorar carpetas, jerarquía y métricas temporales
    function Inspect-Folder($folder, [string]$parentPath = "") {
        $fName = $folder.Name
        $relPath = if ($parentPath) { "$parentPath\$fName" } else { $fName }
        $fCount = 0
        try { $fCount = $folder.Items.Count } catch {}
        $subCount = 0
        try { $subCount = $folder.Folders.Count } catch {}
        $hasChildren = ($subCount -gt 0)

        $folderSizeBytes = 0
        $fCountsByYear = @{}
        $fCountsByMonth = @{}
        $fSizesByYear = @{}
        $fSizesByMonth = @{}
        $fYearsSet = @{}
        $fYearMonths = @{}

        if ($fCount -gt 0) {
            try {
                $tbl = $folder.GetTable()
                try { $tbl.Columns.Add("MessageSize") | Out-Null } catch {}
                try { $tbl.Columns.Add("ReceivedTime") | Out-Null } catch {}
                try { $tbl.Columns.Add("SentOn") | Out-Null } catch {}

                while (-not $tbl.EndOfTable) {
                    $row = $tbl.GetNextRow()
                    $sz = $row.Item("MessageSize")
                    if ($null -eq $sz -or $sz -lt 0) { $sz = 0 }
                    $folderSizeBytes += $sz

                    $dt = $row.Item("ReceivedTime")
                    if ($null -eq $dt -or -not ($dt -is [DateTime])) {
                        $dt = $row.Item("SentOn")
                    }
                    if ($null -ne $dt -and ($dt -is [DateTime]) -and $dt.Year -ge 1980 -and $dt.Year -le 2050) {
                        $y = $dt.Year
                        $m = $dt.Month
                        $yStr = "$y"
                        $ymStr = "$y-$("{0:D2}" -f $m)"

                        if ($dt -lt $script:minDate) { $script:minDate = $dt }
                        if ($dt -gt $script:maxDate) { $script:maxDate = $dt }

                        # Métricas a nivel de carpeta
                        $fYearsSet[$y] = $true
                        if (-not $fYearMonths.ContainsKey($yStr)) { $fYearMonths[$yStr] = @{} }
                        $fYearMonths[$yStr][$m] = $true

                        if (-not $fCountsByYear.ContainsKey($yStr)) { $fCountsByYear[$yStr] = 0; $fSizesByYear[$yStr] = 0.0 }
                        $fCountsByYear[$yStr]++
                        $fSizesByYear[$yStr] += ($sz / 1MB)

                        if (-not $fCountsByMonth.ContainsKey($ymStr)) { $fCountsByMonth[$ymStr] = 0; $fSizesByMonth[$ymStr] = 0.0 }
                        $fCountsByMonth[$ymStr]++
                        $fSizesByMonth[$ymStr] += ($sz / 1MB)

                        # Métricas globales del PST
                        $script:yearsSet[$y] = $true
                        if (-not $script:yearMonthsMap.ContainsKey($yStr)) { $script:yearMonthsMap[$yStr] = @{} }
                        $script:yearMonthsMap[$yStr][$m] = $true

                        if (-not $script:globalCountsByYear.ContainsKey($yStr)) { $script:globalCountsByYear[$yStr] = 0; $script:globalSizesByYear[$yStr] = 0.0 }
                        $script:globalCountsByYear[$yStr]++
                        $script:globalSizesByYear[$yStr] += ($sz / 1MB)

                        if (-not $script:globalCountsByMonth.ContainsKey($ymStr)) { $script:globalCountsByMonth[$ymStr] = 0; $script:globalSizesByMonth[$ymStr] = 0.0 }
                        $script:globalCountsByMonth[$ymStr]++
                        $script:globalSizesByMonth[$ymStr] += ($sz / 1MB)
                    }
                }
            } catch {
                # Fallback en caso de que GetTable encuentre alguna excepción MAPI
            }
        }

        # Redondear tamaños de carpeta
        $fSizesByYearRounded = @{}
        foreach ($k in $fSizesByYear.Keys) { $fSizesByYearRounded[$k] = [math]::Round($fSizesByYear[$k], 2) }
        $fSizesByMonthRounded = @{}
        foreach ($k in $fSizesByMonth.Keys) { $fSizesByMonthRounded[$k] = [math]::Round($fSizesByMonth[$k], 2) }

        $fSortedYears = $fYearsSet.Keys | ForEach-Object { [int]$_ } | Sort-Object
        $fFinalYearMonths = @{}
        foreach ($y in $fSortedYears) {
            $yStr = "$y"
            if ($fYearMonths.ContainsKey($yStr)) {
                $mList = $fYearMonths[$yStr].Keys | ForEach-Object { [int]$_ } | Sort-Object
                $fFinalYearMonths[$yStr] = @($mList)
            } else {
                $fFinalYearMonths[$yStr] = @()
            }
        }

        $script:totalItems += $fCount
        $script:foldersList += @{
            name              = $fName
            path              = $relPath
            parent_path       = if ($parentPath) { $parentPath } else { $null }
            total_items       = $fCount
            size_mb           = [math]::Round($folderSizeBytes / 1MB, 2)
            has_children      = $hasChildren
            years             = @($fSortedYears)
            year_months       = $fFinalYearMonths
            counts_by_year    = $fCountsByYear
            counts_by_month   = $fCountsByMonth
            sizes_by_year_mb  = $fSizesByYearRounded
            sizes_by_month_mb = $fSizesByMonthRounded
        }

        # Subcarpetas recursivas
        try {
            foreach ($sub in $folder.Folders) {
                Inspect-Folder $sub $relPath
            }
        } catch {}
    }

    foreach ($subF in $root.Folders) {
        Inspect-Folder $subF ""
    }

    # Estructurar resultado global
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

    $globalSizesByYearRounded = @{}
    foreach ($k in $script:globalSizesByYear.Keys) { $globalSizesByYearRounded[$k] = [math]::Round($script:globalSizesByYear[$k], 2) }
    $globalSizesByMonthRounded = @{}
    foreach ($k in $script:globalSizesByMonth.Keys) { $globalSizesByMonthRounded[$k] = [math]::Round($script:globalSizesByMonth[$k], 2) }

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
        file_name         = $fileName
        file_path         = $PstPath
        size_mb           = $sizeMb
        total_items       = $totalItems
        last_email_date   = $lastDateStr
        first_email_date  = $firstDateStr
        folders           = $foldersList
        years             = @($sortedYears)
        year_months       = $finalYearMonths
        counts_by_year    = $script:globalCountsByYear
        counts_by_month   = $script:globalCountsByMonth
        sizes_by_year_mb  = $globalSizesByYearRounded
        sizes_by_month_mb = $globalSizesByMonthRounded
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
