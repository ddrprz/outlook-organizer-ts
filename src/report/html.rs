use std::{fs, path::PathBuf};
use chrono::Local;

use crate::app::AppState;

/// Genera un informe interactivo, visual, moderno y responsive en HTML
pub fn generate_html_report(state: &AppState, custom_path: Option<PathBuf>) -> Result<PathBuf, std::io::Error> {
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H:%M:%S").to_string();

    let target_path = if let Some(path) = custom_path {
        path
    } else {
        let logs_dir = PathBuf::from("logs").join(&date_str);
        fs::create_dir_all(&logs_dir)?;
        logs_dir.join(format!("report_{}.html", now.format("%H-%M-%S")))
    };

    let selected_psts_rows: String = state
        .discovered_psts
        .iter()
        .filter(|p| p.selected)
        .map(|p| {
            format!(
                "<tr><td>📁 {}</td><td>{:.1} MB</td><td><span class='badge badge-success'>Completado</span></td></tr>",
                p.name, p.size_mb
            )
        })
        .collect();

    let html_content = format!(
        r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Informe de Migración — Outlook Organizer TS</title>
  <style>
    :root {{
      --bg: #0f172a;
      --card-bg: #1e293b;
      --accent: #00a4ef;
      --accent-secondary: #5b5fc7;
      --success: #10b981;
      --warning: #f59e0b;
      --danger: #ef4444;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --border: #334155;
    }}
    * {{ box-sizing: border-box; margin: 0; padding: 0; }}
    body {{
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.6;
      padding: 2.5rem 1.5rem;
    }}
    .container {{
      max-width: 1000px;
      margin: 0 auto;
    }}
    .header {{
      display: flex;
      justify-content: space-between;
      align-items: center;
      border-bottom: 2px solid var(--border);
      padding-bottom: 1.5rem;
      margin-bottom: 2rem;
    }}
    .header h1 {{
      font-size: 1.8rem;
      color: var(--accent);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }}
    .brand {{
      text-align: right;
      font-weight: 700;
      color: #00e5ff;
      letter-spacing: 1px;
    }}
    .brand small {{
      display: block;
      color: var(--text-muted);
      font-size: 0.75rem;
      font-weight: 400;
    }}
    .grid {{
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(220px, 1fr));
      gap: 1.25rem;
      margin-bottom: 2.5rem;
    }}
    .card {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 1.5rem;
      box-shadow: 0 4px 6px -1px rgba(0,0,0,0.1);
    }}
    .card .title {{
      font-size: 0.85rem;
      color: var(--text-muted);
      text-transform: uppercase;
      font-weight: 600;
      margin-bottom: 0.5rem;
    }}
    .card .value {{
      font-size: 2rem;
      font-weight: 800;
      color: var(--text);
    }}
    .card .value.success {{ color: var(--success); }}
    .card .value.accent {{ color: var(--accent); }}
    .card .value.danger {{ color: var(--danger); }}
    .table-container {{
      background: var(--card-bg);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 1.5rem;
      margin-bottom: 2rem;
    }}
    .table-container h2 {{
      font-size: 1.2rem;
      margin-bottom: 1rem;
      color: var(--accent);
    }}
    table {{
      width: 100%;
      border-collapse: collapse;
      text-align: left;
    }}
    th, td {{
      padding: 0.75rem 1rem;
      border-bottom: 1px solid var(--border);
    }}
    th {{
      color: var(--text-muted);
      font-size: 0.85rem;
      text-transform: uppercase;
    }}
    .badge {{
      display: inline-block;
      padding: 0.25rem 0.6rem;
      border-radius: 9999px;
      font-size: 0.75rem;
      font-weight: 600;
    }}
    .badge-success {{ background: rgba(16, 185, 129, 0.2); color: var(--success); }}
    .footer {{
      text-align: center;
      color: var(--text-muted);
      font-size: 0.85rem;
      margin-top: 3rem;
      border-top: 1px solid var(--border);
      padding-top: 1.5rem;
    }}
  </style>
</head>
<body>
  <div class="container">
    <div class="header">
      <div>
        <h1>◈ Informe de Migración de PSTs</h1>
        <p style="color: var(--text-muted); font-size: 0.9rem;">Fecha: {} &bull; Hora: {}</p>
      </div>
      <div class="brand">
        ◈ TIMELESS
        <small>SUPPORT</small>
      </div>
    </div>

    <div class="grid">
      <div class="card">
        <div class="title">Correos Importados</div>
        <div class="value success">{}</div>
      </div>
      <div class="card">
        <div class="title">Duplicados Omitidos</div>
        <div class="value accent">{}</div>
      </div>
      <div class="card">
        <div class="title">Errores de Lectura</div>
        <div class="value danger">{}</div>
      </div>
      <div class="card">
        <div class="title">Buzón Destino</div>
        <div class="value" style="font-size: 1.1rem; word-break: break-all; margin-top: 0.5rem;">{}</div>
      </div>
    </div>

    <div class="table-container">
      <h2>Archivos PST Procesados</h2>
      <table>
        <thead>
          <tr>
            <th>Nombre del Archivo</th>
            <th>Tamaño</th>
            <th>Estado</th>
          </tr>
        </thead>
        <tbody>
          {}
        </tbody>
      </table>
    </div>

    <div class="footer">
      Generado automáticamente por <strong>Outlook Organizer TS</strong> &bull; &copy; {} Timeless Support
    </div>
  </div>
</body>
</html>"#,
        date_str,
        time_str,
        state.progress.imported_count,
        state.progress.duplicates_skipped,
        state.progress.error_count,
        state.target_mailbox,
        selected_psts_rows,
        now.format("%Y")
    );

    fs::write(&target_path, html_content)?;
    Ok(target_path)
}
