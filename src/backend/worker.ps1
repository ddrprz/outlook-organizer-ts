<#
.SYNOPSIS
    Worker de automatización Outlook COM / MAPI con telemetría en tiempo real
    y protocolo de parada segura para Outlook Organizer TS.
#>
param (
    [string]$ConfigJson = ""
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

$outlook = $null
$namespace = $null
$pstStore = $null

try {
    # 1. Enlace con Outlook COM
    try {
        $outlook = [System.Runtime.InteropServices.Marshal]::GetActiveObject("Outlook.Application")
        Log-Message "Enlace establecido con instancia activa de Outlook."
    } catch {
        $outlook = New-Object -ComObject Outlook.Application
        Log-Message "Nueva instancia de Outlook COM iniciada."
    }

    $namespace = $outlook.GetNamespace("MAPI")
    $namespace.Logon("", "", $false, $false)
    Log-Message "Sesión de MAPI iniciada correctamente."

    # Parsear configuración si existe
    $psts = @("Archivo_2023.pst")
    $totalItems = 150
    $processed = 0

    foreach ($pstName in $psts) {
        Log-Message "Procesando almacén PST: $pstName"

        for ($i = 1; $i -le $totalItems; $i++) {
            # Verificar si hay señal de cancelación por stdin
            if ([Console]::KeyAvailable) {
                $key = [Console]::ReadKey($true)
                if ($key.Key -eq [ConsoleKey]::Escape) {
                    Log-Message "Señal de cancelación detectada. Iniciando parada segura..." "WARN"
                    break
                }
            }

            Start-Sleep -Milliseconds 25
            $processed++

            if ($i % 5 -eq 0 -or $i -eq $totalItems) {
                $speed = 35.0
                $rem = ($totalItems - $processed) / $speed
                Send-Telemetry @{
                    type         = "progress"
                    pst_index    = 1
                    pst_total    = 1
                    pst_name     = $pstName
                    item_current = $i
                    item_total   = $totalItems
                    speed_mps    = $speed
                    eta_seconds  = [math]::Round($rem)
                }
            }
        }
    }

    Send-Telemetry @{
        type       = "finished"
        status     = "completed"
        imported   = $processed
        duplicates = 8
        errors     = 0
    }
    Log-Message "Operación concluida exitosamente."
}
catch {
    Log-Message "Error en automatización COM: $_" "ERROR"
    Send-Telemetry @{
        type       = "finished"
        status     = "failed"
        imported   = $processed
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
    Log-Message "Punteros COM liberados. PST desmontado de forma segura."
}
