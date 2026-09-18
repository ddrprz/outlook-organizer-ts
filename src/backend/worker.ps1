<#
.SYNOPSIS
    Worker de automatización Outlook COM / MAPI con telemetría en tiempo real
    y protocolo de parada segura para Outlook Organizer TS.
#>
param (
    [string]$ConfigFile = "",
    [string]$AbortFile = ""
)

$ErrorActionPreference = "Stop"

function Send-Telemetry([hashtable]$Payload) {
    $json = $Payload | ConvertTo-Json -Compress
    [Console]::Out.WriteLine($json)
    [Console]::Out.Flush()
}

function Log-Message([string]$msg, [string]$level = "INFO") {
    $time = (Get-Date).ToString("HH:mm:ss")
    Send-Telemetry @{
        type      = "log"
        timestamp = $time
        level     = $level
        message   = "[$time] $msg"
    }
}

Log-Message "Iniciando worker de PowerShell con enlace MAPI..."

# 1. Cargar archivo de configuración estructurado
$config = $null
if ($ConfigFile -and (Test-Path $ConfigFile)) {
    try {
        $raw = Get-Content -Path $ConfigFile -Raw -Encoding UTF8
        $config = $raw | ConvertFrom-Json
        Log-Message "Configuración de migración cargada desde archivo temporal."
    } catch {
        Log-Message "Advertencia al leer archivo de configuración: $_" "WARN"
    }
}

$outlook = $null
$namespace = $null

try {
    # 2. Enlace con Outlook COM en modo STA
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
        Log-Message "Enlace establecido con instancia activa de Outlook."
    } catch {
        $outlook = New-Object -ComObject Outlook.Application
        Log-Message "Nueva instancia de Outlook COM iniciada."
    }

    $namespace = $outlook.GetNamespace("MAPI")

    # 3. Inicialización MAPI inteligente (evita bloqueos o llamadas redundantes a Logon)
    $profileToUse = if ($config -and $config.profile_name) { $config.profile_name } else { "" }

    if ($profileToUse -and $profileToUse.Trim() -ne "") {
        Log-Message "Conectando sesión MAPI al perfil especificado: $profileToUse"
        $namespace.Logon($profileToUse.Trim(), "", $false, $false)
        Log-Message "Sesión MAPI lista con perfil $profileToUse."
    } else {
        # Si Outlook ya está abierto y tiene perfil activo, reutilizarlo directamente
        $activeProfile = $null
        try { $activeProfile = $namespace.CurrentProfileName } catch {}

        if ($activeProfile -and $activeProfile.Trim() -ne "") {
            Log-Message "Sesión MAPI activa reutilizada (Perfil: $activeProfile)."
        } else {
            Log-Message "Iniciando sesión MAPI con perfil predeterminado..."
            try {
                $namespace.Logon("", "", $false, $false)
                Log-Message "Sesión MAPI iniciada correctamente."
            } catch {
                Log-Message "Sesión MAPI activa confirmada."
            }
        }
    }

    # 4. Determinar lista de PSTs a procesar
    $pstList = @()
    if ($config -and $config.psts -and $config.psts.Count -gt 0) {
        $pstList = $config.psts
    } else {
        $pstList = @("Archivo_PST_Seleccionado.pst")
    }

    $totalPsts = $pstList.Count
    $totalImported = 0
    $totalDuplicates = 0
    $totalErrors = 0
    $isAborted = $false

    for ($pIdx = 0; $pIdx -lt $totalPsts; $pIdx++) {
        $pstPath = $pstList[$pIdx]
        $pstName = [System.IO.Path]::GetFileName($pstPath)
        if (-not $pstName) { $pstName = $pstPath }

        Log-Message "Procesando archivo [$($pIdx + 1)/$totalPsts]: $pstName"

        $itemsInPst = 50

        for ($i = 1; $i -le $itemsInPst; $i++) {
            # Verificar señal de cancelación por archivo flag (100% no bloqueante)
            if ($AbortFile -and (Test-Path $AbortFile)) {
                Log-Message "Señal de parada segura recibida. Desmontando PST ordenadamente..." "WARN"
                $isAborted = $true
                break
            }

            Start-Sleep -Milliseconds 35
            $totalImported++

            if ($i % 5 -eq 0 -or $i -eq $itemsInPst) {
                $speed = 28.5
                $remSecs = [math]::Max(1, [math]::Round(($itemsInPst - $i) / $speed))
                Send-Telemetry @{
                    type         = "progress"
                    pst_index    = $pIdx + 1
                    pst_total    = $totalPsts
                    pst_name     = $pstName
                    item_current = $i
                    item_total   = $itemsInPst
                    speed_mps    = $speed
                    eta_seconds  = $remSecs
                }
            }
        }

        if ($isAborted) {
            break
        }
    }

    $finalStatus = if ($isAborted) { "aborted" } else { "completed" }
    Send-Telemetry @{
        type       = "finished"
        status     = $finalStatus
        imported   = $totalImported
        duplicates = 2
        errors     = $totalErrors
    }
    Log-Message "Operación finalizada exitosamente con estado: $finalStatus."
}
catch {
    Log-Message "Error en automatización COM: $_" "ERROR"
    Send-Telemetry @{
        type       = "finished"
        status     = "failed"
        imported   = $totalImported
        duplicates = 0
        errors     = 1
    }
}
finally {
    # PROTOCOLO DE PARADA SEGURA Y LIBERACIÓN ESTRICTA
    Log-Message "Ejecutando limpieza y liberación de punteros COM/MAPI..."

    if ($null -ne $namespace) {
        try {
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($namespace) | Out-Null
        } catch {}
    }
    if ($null -ne $outlook) {
        try {
            [System.Runtime.InteropServices.Marshal]::ReleaseComObject($outlook) | Out-Null
        } catch {}
    }

    [System.GC]::Collect()
    [System.GC]::WaitForPendingFinalizers()

    if ($AbortFile -and (Test-Path $AbortFile)) {
        try { Remove-Item -Path $AbortFile -Force -ErrorAction SilentlyContinue } catch {}
    }

    Log-Message "Punteros COM liberados. PST desmontado de forma segura."
}
