//! Módulo de formateo dinámico de unidades para la TUI de Outlook Organizer TS.
//!
//! Convierte automáticamente:
//! - Tamaños: Bytes, Kilobytes, Megabytes, Gigabytes, Terabytes
//! - Duraciones / Tiempos: segundos -> minutos -> horas -> días

/// Formatea un tamaño expresado en Megabytes (f64) de forma dinámica:
/// - `<= 0.0 MB`: "0 MB"
/// - `< 1.0 MB`: Kilobytes ("X.X KB") o Bytes ("X B")
/// - `1.0 MB .. 1000.0 MB`: Megabytes ("X.X MB")
/// - `>= 1000.0 MB .. 1000.0 GB`: Gigabytes ("X.X GB")
/// - `>= 1000.0 GB`: Terabytes ("X.X TB")
pub fn format_size_mb(mb: f64) -> String {
    if mb <= 0.0 {
        return "0 MB".to_string();
    }

    if mb >= 1_000_000.0 {
        let tb = mb / 1_048_576.0;
        format!("{:.2} TB", tb)
    } else if mb >= 1000.0 {
        let gb = mb / 1024.0;
        if gb >= 100.0 {
            format!("{:.1} GB", gb)
        } else {
            format!("{:.2} GB", gb)
        }
    } else if mb >= 1.0 {
        if mb >= 100.0 {
            format!("{:.1} MB", mb)
        } else {
            format!("{:.2} MB", mb)
        }
    } else {
        let kb = mb * 1024.0;
        if kb >= 1.0 {
            if kb >= 100.0 {
                format!("{:.0} KB", kb)
            } else {
                format!("{:.1} KB", kb)
            }
        } else {
            let bytes = kb * 1024.0;
            format!("{:.0} B", bytes)
        }
    }
}

/// Variante para vistas de árbol donde 0.0 debe ser una cadena vacía
#[allow(dead_code)]
pub fn format_size_mb_or_empty(mb: f64) -> String {
    if mb <= 0.0 {
        String::new()
    } else {
        format_size_mb(mb)
    }
}

/// Formatea una duración en segundos en formato compacto (ideal para barras de progreso / gauges)
/// Ejemplos:
/// - 0..59s -> "45s"
/// - 60..3599s -> "2m 05s"
/// - 3600..86399s -> "1h 04m"
/// - >= 86400s -> "1d 3h"
pub fn format_duration_compact(seconds: u64) -> String {
    if seconds == 0 {
        return "0s".to_string();
    }

    if seconds < 60 {
        format!("{}s", seconds)
    } else if seconds < 3600 {
        let mins = seconds / 60;
        let secs = seconds % 60;
        if secs > 0 {
            format!("{}m {:02}s", mins, secs)
        } else {
            format!("{}m", mins)
        }
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        let rem = seconds % 3600;
        let mins = rem / 60;
        if mins > 0 {
            format!("{}h {:02}m", hours, mins)
        } else {
            format!("{}h", hours)
        }
    } else {
        let days = seconds / 86400;
        let rem = seconds % 86400;
        let hours = rem / 3600;
        if hours > 0 {
            format!("{}d {}h", days, hours)
        } else {
            format!("{}d", days)
        }
    }
}

/// Formatea una duración en segundos en formato descriptivo / detallado (ideal para tarjetas KPI)
/// Ejemplos:
/// - 0 -> "0 segundos"
/// - 1 -> "1 segundo"
/// - 45 -> "45 segundos"
/// - 125 -> "2 min 5 s"
/// - 3600 -> "1 h"
/// - 3725 -> "1 h 2 min 5 s"
pub fn format_duration_verbose(seconds: u64) -> String {
    if seconds == 0 {
        return "0 segundos".to_string();
    }
    if seconds == 1 {
        return "1 segundo".to_string();
    }
    if seconds < 60 {
        return format!("{} segundos", seconds);
    }

    if seconds < 3600 {
        let mins = seconds / 60;
        let secs = seconds % 60;
        if secs == 0 {
            format!("{} min", mins)
        } else {
            format!("{} min {} s", mins, secs)
        }
    } else if seconds < 86400 {
        let hours = seconds / 3600;
        let rem = seconds % 3600;
        let mins = rem / 60;
        let secs = rem % 60;
        if mins == 0 && secs == 0 {
            format!("{} h", hours)
        } else if secs == 0 {
            format!("{} h {} min", hours, mins)
        } else {
            format!("{} h {} min {} s", hours, mins, secs)
        }
    } else {
        let days = seconds / 86400;
        let rem = seconds % 86400;
        let hours = rem / 3600;
        let mins = (rem % 3600) / 60;
        if hours == 0 && mins == 0 {
            format!("{} días", days)
        } else if mins == 0 {
            format!("{} días {} h", days, hours)
        } else {
            format!("{} días {} h {} min", days, hours, mins)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_size_mb() {
        assert_eq!(format_size_mb(0.0), "0 MB");
        assert_eq!(format_size_mb(-5.0), "0 MB");
        assert_eq!(format_size_mb(0.5), "512 KB");
        assert_eq!(format_size_mb(45.2), "45.20 MB");
        assert_eq!(format_size_mb(150.0), "150.0 MB");
        assert_eq!(format_size_mb(1024.0), "1.00 GB");
        assert_eq!(format_size_mb(50_000.0), "48.83 GB");
        assert_eq!(format_size_mb(120_000.0), "117.2 GB");
        assert_eq!(format_size_mb(2_000_000.0), "1.91 TB");
    }

    #[test]
    fn test_format_size_mb_or_empty() {
        assert_eq!(format_size_mb_or_empty(0.0), "");
        assert_eq!(format_size_mb_or_empty(-1.0), "");
        assert_eq!(format_size_mb_or_empty(50.0), "50.00 MB");
    }

    #[test]
    fn test_format_duration_compact() {
        assert_eq!(format_duration_compact(0), "0s");
        assert_eq!(format_duration_compact(45), "45s");
        assert_eq!(format_duration_compact(60), "1m");
        assert_eq!(format_duration_compact(125), "2m 05s");
        assert_eq!(format_duration_compact(3600), "1h");
        assert_eq!(format_duration_compact(3665), "1h 01m");
        assert_eq!(format_duration_compact(90000), "1d 1h");
    }

    #[test]
    fn test_format_duration_verbose() {
        assert_eq!(format_duration_verbose(0), "0 segundos");
        assert_eq!(format_duration_verbose(1), "1 segundo");
        assert_eq!(format_duration_verbose(45), "45 segundos");
        assert_eq!(format_duration_verbose(60), "1 min");
        assert_eq!(format_duration_verbose(125), "2 min 5 s");
        assert_eq!(format_duration_verbose(3600), "1 h");
        assert_eq!(format_duration_verbose(3665), "1 h 1 min 5 s");
        assert_eq!(format_duration_verbose(90060), "1 días 1 h 1 min");
    }
}
