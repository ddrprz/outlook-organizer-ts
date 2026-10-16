param (
    [string]$MailboxName = "",
    [string]$ProfileName = ""
)

<#
.SYNOPSIS
    Inspecciona detalladamente un buzón de Microsoft Outlook / Exchange (Buzón Principal o Compartido)
    utilizando MAPI Table de alto rendimiento para extraer la jerarquía de carpetas, volumen de correos,
    tamaño y distribución temporal por años y meses para Outlook Organizer TS.
#>

[Console]::OutputEncoding = [System.Text.Encoding]::UTF8
[Console]::InputEncoding  = [System.Text.Encoding]::UTF8
$OutputEncoding           = [System.Text.Encoding]::UTF8
$ErrorActionPreference    = "Stop"

try {
    [System.Diagnostics.Process]::GetCurrentProcess().PriorityClass = [System.Diagnostics.ProcessPriorityClass]::High
} catch {}

$outlook = $null
$namespace = $null
$weStartedOutlook = $false

function Emit-Progress([string]$folder, [int]$items) {
    try {
        $p = @{
            folder = $folder
            items  = $items
        }
        $json = $p | ConvertTo-Json -Compress
        Write-Output "PROGRESS:$json"
    } catch {}
}

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

    # Localizar el Store del buzón
    $targetStore = $null
    $targetNameNorm = $MailboxName.Trim().ToLowerInvariant()

    foreach ($s in $namespace.Stores) {
        $dn = ""
        try { $dn = $s.DisplayName } catch {}
        if ($dn) {
            $dnNorm = $dn.Trim().ToLowerInvariant()
            if ($dnNorm -eq $targetNameNorm) {
                $targetStore = $s
                break
            }
        }
    }

    if ($null -eq $targetStore) {
        # Búsqueda secundaria por coincidencia parcial o email
        foreach ($s in $namespace.Stores) {
            $dn = ""
            try { $dn = $s.DisplayName } catch {}
            if ($dn) {
                $dnNorm = $dn.Trim().ToLowerInvariant()
                if ($dnNorm.Contains($targetNameNorm) -or $targetNameNorm.Contains($dnNorm)) {
                    $targetStore = $s
                    break
                }
            }
        }
    }

    if ($null -eq $targetStore) {
        $errObj = @{
            error = "No se encontró el almacén MAPI para el buzón: '$MailboxName'"
        }
        Write-Output ($errObj | ConvertTo-Json -Compress)
        exit 0
    }

    $root = $targetStore.GetRootFolder()

    # Función recursiva para explorar carpetas, jerarquía y métricas temporales
    function Inspect-Folder($folder, [string]$parentPath = "") {
        $fName = $folder.Name
        # Ignorar carpetas internas de sistema
        if ($fName -match "^(Yammer Root|Quick Step Settings|Conversation History|Social Activity Provider RSS Feeds|Sync Issues|Problemas de sincronización|Conflictos|Errores locales|Fallos del servidor|Journal|Diario|Notes|Notas|Suggested Contacts|Contactos sugeridos)$") {
            return
        }

        # Desplegar contenedores intermedios transparentes
        if ($fName -match "^(Top of Personal Folders|Top of Information Store|Elementos de nivel superior del almac[eé]n personal|IPM_SUBTREE|IPM_ROOT)$") {
            try {
                foreach ($sub in $folder.Folders) {
                    Inspect-Folder $sub $parentPath
                }
            } catch {}
            return
        }

        $relPath = if ($parentPath) { "$parentPath\$fName" } else { $fName }
        $fCount = 0
        try {
            if ($null -ne $folder.Items) {
                $fCount = [int]$folder.Items.Count
            }
        } catch { $fCount = 0 }

        $subCount = 0
        try {
            if ($null -ne $folder.Folders) {
                $subCount = [int]$folder.Folders.Count
            }
        } catch { $subCount = 0 }
        $hasChildren = [bool]($subCount -gt 0)

        # 1. Obtención instantánea del tamaño de la carpeta vía MAPI PropertyAccessor (0 ms)
        $folderSizeBytes = [int64]0
        try {
            $fPa = $folder.PropertyAccessor
            if ($null -ne $fPa) {
                try { $folderSizeBytes = [int64]$fPa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080014") } catch {}
                if ($folderSizeBytes -le 0) {
                    try { $folderSizeBytes = [int64]$fPa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x36E40014") } catch {}
                }
                if ($folderSizeBytes -le 0) {
                    try { $folderSizeBytes = [int64]$fPa.GetProperty("http://schemas.microsoft.com/mapi/proptag/0x0E080003") } catch {}
                }
            }
        } catch {}

        $fCountsByYear = @{}
        $fCountsByMonth = @{}
        $fSizesByYear = @{}
        $fSizesByMonth = @{}

        if ($fCount -gt 0) {
            Emit-Progress $relPath $script:totalEmails
            $script:totalEmails += $fCount

            # 2. Límites temporales ultrarrápidos (Min y Max Date) mediante índice del servidor
            $fMaxDate = $null
            $fMinDate = $null

            # Fecha más reciente (Top 1 descendente)
            try {
                $tblDesc = $folder.GetTable()
                if ($null -ne $tblDesc) {
                    try { $tblDesc.Columns.RemoveAll() } catch {}
                    try { $tblDesc.Columns.Add("ReceivedTime") | Out-Null } catch {}
                    try { $tblDesc.Sort("ReceivedTime", $true) | Out-Null } catch {}
                    if (-not $tblDesc.EndOfTable) {
                        $r = $tblDesc.GetRow()
                        $rt = $r["ReceivedTime"]
                        if ($rt -is [DateTime] -and $rt.Year -ge 1980 -and $rt.Year -le 2100) {
                            $fMaxDate = $rt
                            if ($null -eq $script:maxDate -or $rt -gt $script:maxDate) {
                                $script:maxDate = $rt
                            }
                        }
                    }
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tblDesc) | Out-Null } catch {}
                }
            } catch {}

            # Fecha más antigua (Top 1 ascendente)
            try {
                $tblAsc = $folder.GetTable()
                if ($null -ne $tblAsc) {
                    try { $tblAsc.Columns.RemoveAll() } catch {}
                    try { $tblAsc.Columns.Add("ReceivedTime") | Out-Null } catch {}
                    try { $tblAsc.Sort("ReceivedTime", $false) | Out-Null } catch {}
                    if (-not $tblAsc.EndOfTable) {
                        $r = $tblAsc.GetRow()
                        $rt = $r["ReceivedTime"]
                        if ($rt -is [DateTime] -and $rt.Year -ge 1980 -and $rt.Year -le 2100) {
                            $fMinDate = $rt
                            if ($null -eq $script:minDate -or $rt -lt $script:minDate) {
                                $script:minDate = $rt
                            }
                        }
                    }
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tblAsc) | Out-Null } catch {}
                }
            } catch {}

            # 3. Muestreo ágil de distribución (máximo 300 ítems para no penalizar la red)
            $sampleLimit = if ($fCount -gt 300) { 300 } else { $fCount }
            $tbl = $null
            $sampledCount = 0
            $sampledBytes = [int64]0

            try {
                $tbl = $folder.GetTable()
                if ($null -ne $tbl) {
                    $colDateIdx = -1
                    $colSizeIdx = -1
                    $colSentIdx = -1

                    try { $tbl.Columns.RemoveAll() } catch {}
                    try { $tbl.Columns.Add("ReceivedTime") | Out-Null; $colDateIdx = $tbl.Columns.Count - 1 } catch {}
                    try { $tbl.Columns.Add("Size") | Out-Null; $colSizeIdx = $tbl.Columns.Count - 1 } catch {}
                    try { $tbl.Columns.Add("SentOn") | Out-Null; $colSentIdx = $tbl.Columns.Count - 1 } catch {}

                    if ($fCount -gt 300) {
                        try { $tbl.Sort("ReceivedTime", $true) | Out-Null } catch {}
                    }

                    $arr = $tbl.GetArray($sampleLimit)
                    $bRows = $arr.GetLength(0)
                    $bCols = $arr.GetLength(1)

                    for ($i = 0; $i -lt $bRows; $i++) {
                        $rTime = if ($colDateIdx -ge 0 -and $colDateIdx -lt $bCols) { $arr[$i, $colDateIdx] } else { $null }
                        if (($null -eq $rTime -or -not ($rTime -is [DateTime]) -or $rTime.Year -lt 1980) -and $colSentIdx -ge 0 -and $colSentIdx -lt $bCols) {
                            $rTime = $arr[$i, $colSentIdx]
                        }

                        $sz = if ($colSizeIdx -ge 0 -and $colSizeIdx -lt $bCols) { $arr[$i, $colSizeIdx] } else { 0 }
                        $numSz = [int64]0
                        if ($sz) {
                            try { $numSz = [int64]$sz } catch { $numSz = 0 }
                        }
                        $sampledBytes += $numSz
                        $sampledCount++

                        if ($null -ne $rTime -and ($rTime -is [DateTime]) -and $rTime.Year -ge 1980 -and $rTime.Year -le 2100) {
                            $y = $rTime.Year
                            $m = $rTime.Month
                            $yKey = "$y"
                            $ymKey = "{0}-{1:D2}" -f $y, $m

                            $fCountsByYear[$yKey] = [int]($fCountsByYear[$yKey] + 1)
                            $fCountsByMonth[$ymKey] = [int]($fCountsByMonth[$ymKey] + 1)
                            $fSizesByYear[$yKey] = [int64]($fSizesByYear[$yKey] + $numSz)
                            $fSizesByMonth[$ymKey] = [int64]($fSizesByMonth[$ymKey] + $numSz)

                            if (-not $script:allYearsMap.ContainsKey($y)) {
                                $script:allYearsMap[$y] = $true
                            }
                            if (-not $script:allYearMonthsMap.ContainsKey($yKey)) {
                                $script:allYearMonthsMap[$yKey] = @{}
                            }
                            $script:allYearMonthsMap[$yKey][$m] = $true

                            if ($null -eq $script:minDate -or $rTime -lt $script:minDate) {
                                $script:minDate = $rTime
                            }
                            if ($null -eq $script:maxDate -or $rTime -gt $script:maxDate) {
                                $script:maxDate = $rTime
                            }
                        }
                    }
                }
            } catch {}
            finally {
                if ($null -ne $tbl) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tbl) | Out-Null } catch {}
                }
            }

            # Si el tamaño de la carpeta no se pudo leer directamente de MAPI, estimarlo del muestreo
            if ($folderSizeBytes -le 0) {
                if ($sampledCount -gt 0 -and $sampledBytes -gt 0) {
                    $avgItemSize = [double]$sampledBytes / [double]$sampledCount
                    $folderSizeBytes = [int64]($avgItemSize * $fCount)
                } else {
                    $folderSizeBytes = [int64]($fCount * 50KB)
                }
            }

            # Si se muestreó de forma acotada, escalar proporcionalmente los conteos y tamaños
            if ($fCount -gt $sampledCount -and $sampledCount -gt 0) {
                $scaleFactor = [double]$fCount / [double]$sampledCount
                foreach ($yKey in @($fCountsByYear.Keys)) {
                    $fCountsByYear[$yKey] = [int][Math]::Round($fCountsByYear[$yKey] * $scaleFactor)
                }
                foreach ($ymKey in @($fCountsByMonth.Keys)) {
                    $fCountsByMonth[$ymKey] = [int][Math]::Round($fCountsByMonth[$ymKey] * $scaleFactor)
                }
                if ($folderSizeBytes -gt 0) {
                    $totalSampledSizes = [int64]0
                    foreach ($yKey in $fSizesByYear.Keys) { $totalSampledSizes += $fSizesByYear[$yKey] }
                    if ($totalSampledSizes -gt 0) {
                        $sizeScale = [double]$folderSizeBytes / [double]$totalSampledSizes
                        foreach ($yKey in @($fSizesByYear.Keys)) {
                            $fSizesByYear[$yKey] = [int64][Math]::Round($fSizesByYear[$yKey] * $sizeScale)
                        }
                        foreach ($ymKey in @($fSizesByMonth.Keys)) {
                            $fSizesByMonth[$ymKey] = [int64][Math]::Round($fSizesByMonth[$ymKey] * $sizeScale)
                        }
                    }
                }
            }

            # Garantizar que todos los años y meses entre minDate y maxDate existan en el mapa de filtros
            if ($fMinDate -and $fMaxDate -and $fMinDate.Year -le $fMaxDate.Year) {
                for ($yr = $fMinDate.Year; $yr -le $fMaxDate.Year; $yr++) {
                    $yStr = "$yr"
                    if (-not $script:allYearsMap.ContainsKey($yr)) {
                        $script:allYearsMap[$yr] = $true
                    }
                    if (-not $script:allYearMonthsMap.ContainsKey($yStr)) {
                        $script:allYearMonthsMap[$yStr] = @{}
                    }
                    if (-not $fCountsByYear.ContainsKey($yStr)) {
                        $fCountsByYear[$yStr] = 0
                    }
                    $startMonth = if ($yr -eq $fMinDate.Year) { $fMinDate.Month } else { 1 }
                    $endMonth = if ($yr -eq $fMaxDate.Year) { $fMaxDate.Month } else { 12 }
                    for ($mo = $startMonth; $mo -le $endMonth; $mo++) {
                        $script:allYearMonthsMap[$yStr][$mo] = $true
                        $ymStr = "{0}-{1:D2}" -f $yr, $mo
                        if (-not $fCountsByMonth.ContainsKey($ymStr)) {
                            $fCountsByMonth[$ymStr] = 0
                        }
                    }
                }
            }
        }

        # Redondear tamaños de carpeta a MB
        $fSizesByYearMb = @{}
        foreach ($k in $fSizesByYear.Keys) {
            $fSizesByYearMb[$k] = [math]::Round($fSizesByYear[$k] / 1MB, 2)
        }
        $fSizesByMonthMb = @{}
        foreach ($k in $fSizesByMonth.Keys) {
            $fSizesByMonthMb[$k] = [math]::Round($fSizesByMonth[$k] / 1MB, 2)
        }

        $folderDetail = @{
            name              = $fName
            relative_path     = $relPath
            item_count        = $fCount
            size_mb           = [math]::Round($folderSizeBytes / 1MB, 2)
            has_children      = $hasChildren
            counts_by_year    = $fCountsByYear
            counts_by_month   = $fCountsByMonth
            sizes_by_year_mb  = $fSizesByYearMb
            sizes_by_month_mb = $fSizesByMonthMb
        }
        $script:foldersList.Add($folderDetail)

        # Recorrer subcarpetas
        try {
            foreach ($sub in $folder.Folders) {
                Inspect-Folder $sub $relPath
                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($sub) | Out-Null } catch {}
            }
        } catch {}
    }

    $script:totalEmails = 0
    $script:minDate = $null
    $script:maxDate = $null
    $script:foldersList = New-Object 'System.Collections.Generic.List[hashtable]'
    $script:allYearsMap = @{}
    $script:allYearMonthsMap = @{}

    foreach ($subF in $root.Folders) {
        Inspect-Folder $subF ""
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($subF) | Out-Null } catch {}
    }

    # Consolidar conteos y tamaños globales sumando los totales escalados de las carpetas
    $script:globalCountsByYear = @{}
    $script:globalCountsByMonth = @{}
    $script:globalSizesByYear = @{}
    $script:globalSizesByMonth = @{}

    foreach ($f in $script:foldersList) {
        if ($f.counts_by_year) {
            foreach ($y in $f.counts_by_year.Keys) {
                $script:globalCountsByYear[$y] = [int]($script:globalCountsByYear[$y] + $f.counts_by_year[$y])
            }
        }
        if ($f.counts_by_month) {
            foreach ($ym in $f.counts_by_month.Keys) {
                $script:globalCountsByMonth[$ym] = [int]($script:globalCountsByMonth[$ym] + $f.counts_by_month[$ym])
            }
        }
        if ($f.sizes_by_year_mb) {
            foreach ($y in $f.sizes_by_year_mb.Keys) {
                $script:globalSizesByYear[$y] = [double]($script:globalSizesByYear[$y] + $f.sizes_by_year_mb[$y])
            }
        }
        if ($f.sizes_by_month_mb) {
            foreach ($ym in $f.sizes_by_month_mb.Keys) {
                $script:globalSizesByMonth[$ym] = [double]($script:globalSizesByMonth[$ym] + $f.sizes_by_month_mb[$ym])
            }
        }
    }

    # Ordenar años y estructurar meses
    $sortedYears = @($script:allYearsMap.Keys | Sort-Object)
    $finalYearMonths = @{}
    foreach ($yKey in $script:allYearMonthsMap.Keys) {
        $mList = @($script:allYearMonthsMap[$yKey].Keys | Sort-Object)
        $finalYearMonths[$yKey] = $mList
    }

    $globalSizesByYearRounded = @{}
    foreach ($k in $script:globalSizesByYear.Keys) {
        $globalSizesByYearRounded[$k] = [math]::Round([double]$script:globalSizesByYear[$k], 2)
    }

    $globalSizesByMonthRounded = @{}
    foreach ($k in $script:globalSizesByMonth.Keys) {
        $globalSizesByMonthRounded[$k] = [math]::Round([double]$script:globalSizesByMonth[$k], 2)
    }

    # Calcular peso total
    $totalMb = 0.0
    foreach ($f in $script:foldersList) {
        $totalMb += [double]$f.size_mb
    }
    $totalMb = [math]::Round($totalMb, 2)

    $result = @{
        file_name         = $targetStore.DisplayName
        file_path         = if ($targetStore.FilePath) { $targetStore.FilePath } else { $targetStore.DisplayName }
        size_mb           = $totalMb
        total_items       = $script:totalEmails
        first_email_date  = if ($script:minDate) { $script:minDate.ToString("yyyy-MM-dd HH:mm:ss") } else { $null }
        last_email_date   = if ($script:maxDate) { $script:maxDate.ToString("yyyy-MM-dd HH:mm:ss") } else { $null }
        folders           = @($script:foldersList)
        years             = $sortedYears
        year_months       = $finalYearMonths
        counts_by_year    = $script:globalCountsByYear
        counts_by_month   = $script:globalCountsByMonth
        sizes_by_year_mb  = $globalSizesByYearRounded
        sizes_by_month_mb = $globalSizesByMonthRounded
    }

    Write-Output ($result | ConvertTo-Json -Depth 15 -Compress)
}
catch {
    $errObj = @{
        error = "Excepción al inspeccionar buzón: $_"
    }
    Write-Output ($errObj | ConvertTo-Json -Depth 5 -Compress)
}
finally {
    if ($targetStore) {
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($targetStore) | Out-Null } catch {}
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
}
