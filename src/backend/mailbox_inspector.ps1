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

        $folderSizeBytes = 0
        $fCountsByYear = @{}
        $fCountsByMonth = @{}
        $fSizesByYear = @{}
        $fSizesByMonth = @{}

        if ($fCount -gt 0) {
            Emit-Progress $relPath $script:totalEmails

            $tbl = $null
            $tableSuccess = $false
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

                    $batchSize = 5000
                    while (-not $tbl.EndOfTable) {
                        $arr = $tbl.GetArray($batchSize)
                        $bRows = $arr.GetLength(0)
                        $bCols = $arr.GetLength(1)
                        if ($bRows -eq 0) { break }

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
                            $folderSizeBytes += $numSz

                            if ($null -ne $rTime -and ($rTime -is [DateTime]) -and $rTime.Year -ge 1980 -and $rTime.Year -le 2100) {
                                $y = $rTime.Year
                                $m = $rTime.Month
                                $yKey = "$y"
                                $ymKey = "{0}-{1:D2}" -f $y, $m

                                $fCountsByYear[$yKey] = [int]($fCountsByYear[$yKey] + 1)
                                $fCountsByMonth[$ymKey] = [int]($fCountsByMonth[$ymKey] + 1)
                                $fSizesByYear[$yKey] = [int64]($fSizesByYear[$yKey] + $numSz)
                                $fSizesByMonth[$ymKey] = [int64]($fSizesByMonth[$ymKey] + $numSz)

                                $script:globalCountsByYear[$yKey] = [int]($script:globalCountsByYear[$yKey] + 1)
                                $script:globalCountsByMonth[$ymKey] = [int]($script:globalCountsByMonth[$ymKey] + 1)
                                $script:globalSizesByYear[$yKey] = [int64]($script:globalSizesByYear[$yKey] + $numSz)
                                $script:globalSizesByMonth[$ymKey] = [int64]($script:globalSizesByMonth[$ymKey] + $numSz)

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

                            $script:totalEmails++
                        }
                    }
                    $tableSuccess = $true
                }
            } catch {
                $tableSuccess = $false
            } finally {
                if ($null -ne $tbl) {
                    try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($tbl) | Out-Null } catch {}
                }
            }

            # Fallback seguro por si GetTable no estuvo disponible
            if (-not $tableSuccess) {
                $items = $null
                try {
                    $items = $folder.Items
                    $itemCount = $items.Count
                    for ($idx = 1; $idx -le $itemCount; $idx++) {
                        $itm = $null
                        try {
                            $itm = $items.Item($idx)
                            $rTime = $null
                            try { $rTime = $itm.ReceivedTime } catch {}
                            if ($null -eq $rTime) { try { $rTime = $itm.SentOn } catch {} }

                            $sz = 0
                            try { $sz = [int64]$itm.Size } catch {}
                            $folderSizeBytes += $sz

                            if ($null -ne $rTime -and ($rTime -is [DateTime]) -and $rTime.Year -ge 1980 -and $rTime.Year -le 2100) {
                                $y = $rTime.Year
                                $m = $rTime.Month
                                $yKey = "$y"
                                $ymKey = "{0}-{1:D2}" -f $y, $m

                                $fCountsByYear[$yKey] = [int]($fCountsByYear[$yKey] + 1)
                                $fCountsByMonth[$ymKey] = [int]($fCountsByMonth[$ymKey] + 1)
                                $fSizesByYear[$yKey] = [int64]($fSizesByYear[$yKey] + $sz)
                                $fSizesByMonth[$ymKey] = [int64]($fSizesByMonth[$ymKey] + $sz)

                                $script:globalCountsByYear[$yKey] = [int]($script:globalCountsByYear[$yKey] + 1)
                                $script:globalCountsByMonth[$ymKey] = [int]($script:globalCountsByMonth[$ymKey] + 1)
                                $script:globalSizesByYear[$yKey] = [int64]($script:globalSizesByYear[$yKey] + $sz)
                                $script:globalSizesByMonth[$ymKey] = [int64]($script:globalSizesByMonth[$ymKey] + $sz)

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
                            $script:totalEmails++
                        } catch {}
                        finally {
                            if ($null -ne $itm) {
                                try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($itm) | Out-Null } catch {}
                            }
                        }
                    }
                } catch {}
                finally {
                    if ($null -ne $items) {
                        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($items) | Out-Null } catch {}
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
    $script:globalCountsByYear = @{}
    $script:globalCountsByMonth = @{}
    $script:globalSizesByYear = @{}
    $script:globalSizesByMonth = @{}

    foreach ($subF in $root.Folders) {
        Inspect-Folder $subF ""
        try { [System.Runtime.InteropServices.Marshal]::ReleaseComObject($subF) | Out-Null } catch {}
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
        $globalSizesByYearRounded[$k] = [math]::Round($script:globalSizesByYear[$k] / 1MB, 2)
    }

    $globalSizesByMonthRounded = @{}
    foreach ($k in $script:globalSizesByMonth.Keys) {
        $globalSizesByMonthRounded[$k] = [math]::Round($script:globalSizesByMonth[$k] / 1MB, 2)
    }

    # Calcular peso total
    $totalMb = 0.0
    foreach ($k in $globalSizesByYearRounded.Keys) {
        $totalMb += [double]$globalSizesByYearRounded[$k]
    }
    $totalMb = [math]::Round($totalMb, 2)

    $result = @{
        file_name         = $targetStore.DisplayName
        file_path         = if ($targetStore.FilePath) { $targetStore.FilePath } else { $targetStore.DisplayName }
        size_mb           = $totalMb
        total_items       = $script:totalEmails
        first_email_date  = if ($script:minDate) { $script:minDate.ToString("yyyy-MM-dd HH:mm:ss") } else { $null }
        last_email_date   = if ($script:maxDate) { $script:maxDate.ToString("yyyy-MM-dd HH:mm:ss") } else { $null }
        folders           = @($foldersList)
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
}
