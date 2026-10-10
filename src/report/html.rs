use std::{fs, path::PathBuf};
use chrono::Local;

use crate::app::{AppState, RoutingGranularity, TransferMode};

/// Genera un informe interactivo, visual, detallado, moderno y responsive en HTML
pub fn generate_html_report(state: &AppState, custom_path: Option<PathBuf>) -> Result<PathBuf, std::io::Error> {
    let now = Local::now();
    let date_str = now.format("%Y-%m-%d").to_string();
    let time_str = now.format("%H:%M:%S").to_string();
    let year_str = now.format("%Y").to_string();

    let target_path = if let Some(path) = custom_path {
        path
    } else {
        let logs_dir = PathBuf::from("logs").join(&date_str);
        fs::create_dir_all(&logs_dir)?;
        logs_dir.join(format!("report_{}.html", now.format("%H-%M-%S")))
    };

    let imported_count = state.progress.imported_count;
    let duplicates_count = state.progress.duplicates_skipped;
    let errors_count = state.progress.error_count;
    let total_count = imported_count + duplicates_count + errors_count;

    let imported_pct = if total_count > 0 {
        format!("{:.1}%", (imported_count as f64 / total_count as f64) * 100.0)
    } else {
        "100%".to_string()
    };

    let duplicates_pct = if total_count > 0 {
        format!("{:.1}%", (duplicates_count as f64 / total_count as f64) * 100.0)
    } else {
        "0%".to_string()
    };

    let errors_pct = if total_count > 0 {
        format!("{:.1}%", (errors_count as f64 / total_count as f64) * 100.0)
    } else {
        "0%".to_string()
    };

    let transfer_mode_str = match state.transfer_mode {
        TransferMode::Copy => "Copy (Non-destructive - original PST remains intact)",
        TransferMode::Move => "Move (Transfer and remove from PST)",
    };

    let routing_str = if state.routing_enabled {
        match state.routing_granularity {
            RoutingGranularity::Mirror => "Mirror Structure (Identical to original PST)",
            RoutingGranularity::Years => "Temporal Grouping by Years (Inbox / YYYY)",
            RoutingGranularity::YearsAndMonths => "Grouping by Years and Months (Inbox / YYYY / MM-Month)",
        }
    } else {
        "Direct (No hierarchical routing)"
    };

    let dedup_str = if state.deduplication_enabled {
        if state.deep_scan_enabled {
            "Enabled (Message-ID / Composite Key + Recursive Deep Scan)"
        } else {
            "Enabled (Standard in destination folder)"
        }
    } else {
        "Disabled"
    };

    let throttling_str = if state.adaptive_throttling_enabled {
        "Enabled (Exchange MAPI Anti-Throttling Protection)"
    } else {
        "Disabled (Direct maximum speed)"
    };

    let profile_str = if state.use_default_profile {
        "Default Windows / Outlook MAPI profile".to_string()
    } else {
        format!("Custom MAPI profile: {}", state.custom_profile_name)
    };

    let date_filter_str = if state.routing_all_years && state.routing_all_months {
        "Full History (No date filter)".to_string()
    } else {
        format!("{} | {}", state.format_years_filter_display(), state.format_months_filter_display())
    };

    let selected_psts_rows: String = state
        .discovered_psts
        .iter()
        .filter(|p| p.selected)
        .map(|p| {
            format!(
                r#"<tr><td><svg class="cell-svg" viewBox="0 0 24 24" fill="none" stroke="var(--cyan)" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg> <strong>{}</strong></td><td class="muted-cell">{}</td><td>{}</td><td><span class="badge badge-success"><svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg> Processed</span></td></tr>"#,
                p.name, p.path, crate::ui::format::format_size_mb(p.size_mb)
            )
        })
        .collect();

    let folders = state.get_imported_folders_summary();
    let emails = state.get_effective_email_items();
    let logs: Vec<String> = state.activity_log.iter().cloned().collect();

    let folders_json = serde_json::to_string(&folders).unwrap_or_else(|_| "[]".to_string());
    let emails_json = serde_json::to_string(&emails).unwrap_or_else(|_| "[]".to_string());
    let logs_json = serde_json::to_string(&logs).unwrap_or_else(|_| "[]".to_string());

    let template = get_html_template();

    let rendered = template
        .replace("__DATE_STR__", &date_str)
        .replace("__TIME_STR__", &time_str)
        .replace("__YEAR__", &year_str)
        .replace("__IMPORTED_COUNT__", &imported_count.to_string())
        .replace("__DUPLICATES_COUNT__", &duplicates_count.to_string())
        .replace("__ERRORS_COUNT__", &errors_count.to_string())
        .replace("__TOTAL_COUNT__", &total_count.to_string())
        .replace("__IMPORTED_PCT__", &imported_pct)
        .replace("__DUPLICATES_PCT__", &duplicates_pct)
        .replace("__ERRORS_PCT__", &errors_pct)
        .replace("__MAILBOX_NAME__", &state.selected_mailboxes_display())
        .replace("__TRANSFER_MODE__", transfer_mode_str)
        .replace("__ROUTING_INFO__", routing_str)
        .replace("__DEDUP_INFO__", dedup_str)
        .replace("__THROTTLING_INFO__", throttling_str)
        .replace("__PROFILE_INFO__", &profile_str)
        .replace("__DATE_FILTER_INFO__", &date_filter_str)
        .replace("__PST_ROWS__", &selected_psts_rows)
        .replace("__STORAGE_IMPACT_SECTION__", &build_storage_impact_html(state))
        .replace("__FOLDERS_JSON__", &folders_json)
        .replace("__EMAILS_JSON__", &emails_json)
        .replace("__LOGS_JSON__", &logs_json);

    fs::write(&target_path, rendered)?;
    Ok(target_path)
}

fn format_storage_delta_display(delta_gb: f64) -> String {
    if delta_gb >= 0.01 {
        format!("+{:.2} GB", delta_gb)
    } else if delta_gb > 0.0 {
        let mb = delta_gb * 1024.0;
        if mb >= 0.1 {
            format!("+{:.1} MB", mb)
        } else {
            format!("+{:.2} MB", mb)
        }
    } else {
        "+0.00 GB".to_string()
    }
}

fn build_storage_impact_html(state: &AppState) -> String {
    let impacts = state.get_mailbox_storage_impacts();
    if impacts.is_empty() {
        return r#"<div class="storage-impact-panel"><p style="color: var(--text-muted); text-align: center; padding: 1.5rem;">No se detectaron buzones destino seleccionados para auditar el impacto en almacenamiento.</p></div>"#.to_string();
    }

    let impacts_count = impacts.len();
    let total_quota_gb: f64 = impacts.iter().map(|i| i.total_gb).sum();
    let total_used_before_gb: f64 = impacts.iter().map(|i| i.used_before_gb).sum();
    let total_used_after_gb: f64 = impacts.iter().map(|i| i.used_after_gb).sum();
    let total_imported_gb: f64 = impacts.iter().map(|i| i.imported_gb).sum();
    let total_free_after_gb: f64 = impacts.iter().map(|i| i.free_after_gb).sum();
    let total_imported_items: u64 = state.progress.imported_count;

    let mut cards_html = String::new();
    let mut table_rows_html = String::new();

    for item in &impacts {
        let health_color_before = match item.health_before.as_str() {
            "Lleno" | "Crítico" => "var(--danger)",
            "Atención" => "var(--warning)",
            _ => "var(--success)",
        };
        let health_color_after = match item.health_after.as_str() {
            "Lleno" | "Crítico" => "var(--danger)",
            "Atención" => "var(--warning)",
            _ => "var(--success)",
        };

        let card = format!(
            r#"<div class="mailbox-impact-card">
        <div class="mbx-card-header">
          <div class="mbx-name-group">
            <svg class="icon icon-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/><polyline points="22,6 12,13 2,6"/></svg>
            <strong class="mbx-name">{}</strong>
            <span class="store-type-tag">({})</span>
          </div>
          <div class="quota-pill">
            <span>Cuota Asignada:</span> <strong>{}</strong>
          </div>
        </div>

        <div class="mbx-comparison-grid">
          <!-- Columna Antes -->
          <div class="comparison-col before-col">
            <div class="phase-header">
              <span class="phase-badge before-badge">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 8 14"/></svg>
                ESTADO ANTES DE IMPORTAR
              </span>
              <span class="health-indicator" style="color: {};">● {}</span>
            </div>

            <div class="storage-metric-row">
              <div class="storage-box">
                <span class="box-label">Espacio Usado Inicial</span>
                <span class="box-value">{:.2} GB</span>
                <span class="box-sub">{:.1}% de la cuota</span>
              </div>
              <div class="storage-box free-box">
                <span class="box-label">Espacio Libre Inicial</span>
                <span class="box-value free-val">{:.2} GB</span>
                <span class="box-sub">disponible</span>
              </div>
            </div>

            <div class="impact-bar-wrapper">
              <div class="impact-bar-outer">
                <div class="impact-bar-fill before-fill" style="width: {:.1}%;"></div>
              </div>
              <div class="impact-bar-labels">
                <span>Ocupación: <strong>{:.1}%</strong></span>
                <span>Restante: <strong>{:.2} GB</strong></span>
              </div>
            </div>
          </div>

          <!-- Separador de Incremento / Delta -->
          <div class="comparison-arrow-col">
            <div class="delta-badge">
              <span class="delta-title">TRANSFERIDO</span>
              <span class="delta-value">{}</span>
              <span class="delta-sub">+{:.1}% incremento</span>
              <span class="delta-items">{} correos</span>
            </div>
            <div class="arrow-svg-wrapper">
              <svg width="28" height="28" viewBox="0 0 24 24" fill="none" stroke="var(--cyan)" stroke-width="2.5">
                <line x1="5" y1="12" x2="19" y2="12"></line>
                <polyline points="12 5 19 12 12 19"></polyline>
              </svg>
            </div>
          </div>

          <!-- Columna Después -->
          <div class="comparison-col after-col">
            <div class="phase-header">
              <span class="phase-badge after-badge">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><path d="M22 11.08V12a10 10 0 1 1-5.93-9.14"/><polyline points="22 4 12 14.01 9 11.01"/></svg>
                ESTADO DESPUÉS DE IMPORTAR
              </span>
              <span class="health-indicator" style="color: {};">● {}</span>
            </div>

            <div class="storage-metric-row">
              <div class="storage-box">
                <span class="box-label">Espacio Usado Final</span>
                <span class="box-value after-val">{:.2} GB</span>
                <span class="box-sub">{:.1}% de la cuota</span>
              </div>
              <div class="storage-box free-box">
                <span class="box-label">Espacio Libre Final</span>
                <span class="box-value free-val-after">{:.2} GB</span>
                <span class="box-sub">remanente neto</span>
              </div>
            </div>

            <div class="impact-bar-wrapper">
              <div class="impact-bar-outer">
                <div class="impact-bar-fill base-fill" style="width: {:.1}%;"></div>
                <div class="impact-bar-fill delta-fill" style="width: {:.1}%;"></div>
              </div>
              <div class="impact-bar-labels">
                <span>Ocupación: <strong>{:.1}%</strong></span>
                <span>Restante: <strong>{:.2} GB</strong></span>
              </div>
            </div>
          </div>
        </div>
      </div>"#,
            item.display_name,
            item.store_type,
            item.quota_display,
            health_color_before,
            item.health_before,
            item.used_before_gb,
            item.percent_before,
            item.free_before_gb,
            item.percent_before,
            item.percent_before,
            item.free_before_gb,
            format_storage_delta_display(item.delta_gb),
            item.delta_percent,
            item.imported_items,
            health_color_after,
            item.health_after,
            item.used_after_gb,
            item.percent_after,
            item.free_after_gb,
            item.percent_before,
            item.delta_percent,
            item.percent_after,
            item.free_after_gb
        );
        cards_html.push_str(&card);

        let row = format!(
            r#"<tr>
          <td><strong>{}</strong> <span class="muted-cell">({})</span></td>
          <td><strong>{}</strong></td>
          <td>{:.2} GB <span class="muted-cell">({:.1}%)</span></td>
          <td><span style="color: #34d399; font-weight: 700;">{:.2} GB libres</span></td>
          <td><span class="badge badge-duplicate">{}</span></td>
          <td><strong>{:.2} GB</strong> <span class="muted-cell">({:.1}%)</span></td>
          <td><strong style="color: var(--cyan);">{:.2} GB libres</strong></td>
          <td><span class="health-pill" style="color: {};">● {}</span></td>
        </tr>"#,
            item.display_name,
            item.store_type,
            item.quota_display,
            item.used_before_gb,
            item.percent_before,
            item.free_before_gb,
            format_storage_delta_display(item.delta_gb),
            item.used_after_gb,
            item.percent_after,
            item.free_after_gb,
            health_color_after,
            item.health_after
        );
        table_rows_html.push_str(&row);
    }

    format!(
        r#"<div class="storage-impact-panel">
      <div class="storage-panel-header">
        <div class="storage-title-area">
          <div class="storage-icon-badge">
            <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.2"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/><polyline points="22,6 12,13 2,6"/></svg>
          </div>
          <div>
            <h2>Auditoría de Almacenamiento MAPI: Antes vs. Después de la Importación</h2>
            <p class="storage-subtitle">Balance de capacidad y consumo de cuota por ingesta de correo en Microsoft Outlook / Exchange</p>
          </div>
        </div>
        <div class="storage-header-tags">
          <span class="meta-pill"><svg class="icon icon-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="20 6 9 17 4 12"/></svg> Auditoría MAPI</span>
          <span class="meta-pill"><strong>{}</strong> Buzón(es)</span>
        </div>
      </div>

      <div class="storage-kpis-grid">
        <div class="storage-kpi-box">
          <span class="kpi-label">Capacidad Total Combinada</span>
          <span class="kpi-val cyan">{:.1} GB</span>
          <span class="kpi-hint">Cuota asignada</span>
        </div>
        <div class="storage-kpi-box">
          <span class="kpi-label">Espacio Ocupado Antes</span>
          <span class="kpi-val">{:.2} GB</span>
          <span class="kpi-hint">Línea base previa</span>
        </div>
        <div class="storage-kpi-box delta-box">
          <span class="kpi-label">Volumen Transferido (Delta)</span>
          <span class="kpi-val green">{}</span>
          <span class="kpi-hint">+{} correos importados</span>
        </div>
        <div class="storage-kpi-box">
          <span class="kpi-label">Espacio Ocupado Después</span>
          <span class="kpi-val purple">{:.2} GB</span>
          <span class="kpi-hint">{:.2} GB libres restantes</span>
        </div>
      </div>

      {}

      <div class="storage-table-card">
        <div class="storage-table-header">
          <h3>
            <svg class="icon icon-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="3" width="18" height="18" rx="2" ry="2"/><line x1="3" y1="9" x2="21" y2="9"/><line x1="9" y1="21" x2="9" y2="9"/></svg>
            Desglose Comparativo de Almacenamiento por Buzón
          </h3>
        </div>
        <div class="table-responsive">
          <table class="storage-comparison-table">
            <thead>
              <tr>
                <th>Buzón Destino</th>
                <th>Cuota</th>
                <th>Antes (Usado)</th>
                <th>Antes (Libre)</th>
                <th>Transferido</th>
                <th>Después (Usado)</th>
                <th>Después (Libre)</th>
                <th>Salud Final</th>
              </tr>
            </thead>
            <tbody>
              {}
            </tbody>
          </table>
        </div>
      </div>
    </div>"#,
        impacts_count,
        total_quota_gb,
        total_used_before_gb,
        format_storage_delta_display(total_imported_gb),
        total_imported_items,
        total_used_after_gb,
        total_free_after_gb,
        cards_html,
        table_rows_html
    )
}

fn get_html_template() -> &'static str {
    r#"<!DOCTYPE html>
<html lang="es">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Informe de Migración & Auditoría — Outlook Organizer TS</title>
  <style>
    :root {
      --bg: #0a0f1d;
      --surface: #0f172a;
      --card: #1e293b;
      --card-hover: #27354f;
      --border: #334155;
      --accent: #00a4ef;
      --cyan: #00e5ff;
      --success: #10b981;
      --warning: #f59e0b;
      --danger: #ef4444;
      --text: #f8fafc;
      --text-muted: #94a3b8;
      --text-dim: #64748b;
    }
    * { box-sizing: border-box; margin: 0; padding: 0; }
    body {
      font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, Helvetica, Arial, sans-serif;
      background: var(--bg);
      color: var(--text);
      line-height: 1.5;
      padding: 2rem 1.5rem;
    }
    .container {
      max-width: 1200px;
      margin: 0 auto;
    }

    /* Iconos Vectoriales SVG */
    .icon {
      width: 16px;
      height: 16px;
      display: inline-block;
      vertical-align: -2px;
      flex-shrink: 0;
    }
    .icon-cyan {
      stroke: var(--cyan);
    }
    .header-title-svg {
      width: 26px;
      height: 26px;
      display: inline-block;
      vertical-align: -4px;
      margin-right: 0.35rem;
      filter: drop-shadow(0 0 8px rgba(0, 229, 255, 0.6));
    }
    .badge-svg {
      width: 12px;
      height: 12px;
      display: inline-block;
      vertical-align: -1px;
      flex-shrink: 0;
    }
    .cell-svg {
      width: 16px;
      height: 16px;
      display: inline-block;
      vertical-align: -3px;
      margin-right: 5px;
    }
    .path-arrow {
      width: 14px;
      height: 14px;
      display: inline-block;
      vertical-align: -2px;
      margin-right: 4px;
    }

    /* Header */
    .header {
      background: linear-gradient(135deg, rgba(30, 41, 59, 0.8) 0%, rgba(15, 23, 42, 0.9) 100%);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 1.75rem 2rem;
      margin-bottom: 2rem;
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 1.5rem;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.3);
    }
    .header-left {
      flex: 1;
      min-width: 300px;
    }
    .header-left h1 {
      font-size: 1.85rem;
      color: var(--text);
      display: flex;
      align-items: center;
      gap: 0.5rem;
      letter-spacing: -0.5px;
    }
    .header-meta {
      display: flex;
      gap: 0.65rem;
      margin-top: 0.75rem;
      font-size: 0.82rem;
      color: var(--text-muted);
      flex-wrap: wrap;
    }
    .meta-pill {
      background: rgba(15, 23, 42, 0.7);
      border: 1px solid var(--border);
      padding: 0.25rem 0.65rem;
      border-radius: 6px;
      display: inline-flex;
      align-items: center;
      gap: 0.45rem;
    }
    .meta-pill strong {
      color: var(--text);
    }

    .header-right {
      display: flex;
      flex-direction: column;
      align-items: flex-end;
      justify-content: center;
      gap: 0.9rem;
    }
    .brand-box {
      display: flex;
      justify-content: flex-end;
      width: 100%;
    }
    .brand-badge {
      display: inline-flex;
      align-items: center;
      justify-content: center;
      gap: 0.75rem;
      background: rgba(0, 229, 255, 0.05);
      border: 1px solid rgba(0, 229, 255, 0.3);
      padding: 0.55rem 1.4rem;
      border-radius: 12px;
      box-shadow: 0 4px 15px rgba(0, 229, 255, 0.08);
      text-align: center;
    }
    .brand-text {
      display: flex;
      flex-direction: column;
      align-items: center;
    }
    .brand-title {
      font-size: 1.1rem;
      font-weight: 800;
      letter-spacing: 2.5px;
      color: var(--cyan);
      text-shadow: 0 0 12px rgba(0, 229, 255, 0.45);
      line-height: 1;
    }
    .brand-subtitle {
      font-size: 0.68rem;
      font-weight: 600;
      letter-spacing: 2px;
      color: var(--text-muted);
      line-height: 1;
      margin-top: 3px;
    }

    .action-bar {
      display: flex;
      gap: 0.5rem;
    }
    .btn {
      background: var(--card);
      border: 1px solid var(--border);
      color: var(--text);
      padding: 0.45rem 0.85rem;
      border-radius: 8px;
      font-size: 0.8rem;
      font-weight: 600;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
      transition: all 0.2s ease;
    }
    .btn:hover {
      background: var(--card-hover);
      border-color: var(--accent);
      color: #fff;
      transform: translateY(-1px);
    }
    .btn-primary {
      background: rgba(0, 164, 239, 0.15);
      border-color: var(--accent);
      color: var(--cyan);
    }
    .btn-primary:hover {
      background: var(--accent);
      color: #fff;
    }

    /* KPI Grid */
    .kpi-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(240px, 1fr));
      gap: 1.25rem;
      margin-bottom: 2rem;
    }
    .kpi-card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 14px;
      padding: 1.4rem;
      position: relative;
      overflow: hidden;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.15);
      transition: transform 0.2s ease, border-color 0.2s ease;
    }
    .kpi-card:hover {
      transform: translateY(-2px);
      border-color: var(--cyan);
    }
    .kpi-card::before {
      content: "";
      position: absolute;
      top: 0;
      left: 0;
      width: 100%;
      height: 3px;
    }
    .kpi-card.kpi-imported::before { background: linear-gradient(90deg, var(--success), #34d399); }
    .kpi-card.kpi-duplicates::before { background: linear-gradient(90deg, var(--accent), var(--cyan)); }
    .kpi-card.kpi-errors::before { background: linear-gradient(90deg, var(--danger), #f87171); }
    .kpi-card.kpi-target::before { background: linear-gradient(90deg, #8b5cf6, var(--accent)); }
    .kpi-title {
      font-size: 0.8rem;
      text-transform: uppercase;
      font-weight: 700;
      letter-spacing: 0.5px;
      color: var(--text-muted);
      margin-bottom: 0.4rem;
      display: flex;
      justify-content: space-between;
      align-items: center;
    }
    .kpi-value {
      font-size: 2.2rem;
      font-weight: 800;
      line-height: 1.1;
      margin-bottom: 0.35rem;
    }
    .kpi-value.success { color: var(--success); }
    .kpi-value.cyan { color: var(--cyan); }
    .kpi-value.danger { color: var(--danger); }
    .kpi-value.purple { color: #a78bfa; font-size: 1.15rem; word-break: break-all; margin-top: 0.5rem; }
    .kpi-sub {
      font-size: 0.78rem;
      color: var(--text-muted);
    }

    /* Distribution Bar */
    .distribution-card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 14px;
      padding: 1.25rem 1.5rem;
      margin-bottom: 2rem;
    }
    .distribution-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.75rem;
      font-size: 0.85rem;
      font-weight: 600;
    }
    .ratio-bar {
      height: 10px;
      border-radius: 9999px;
      background: #0f172a;
      overflow: hidden;
      display: flex;
      margin-bottom: 0.75rem;
    }
    .ratio-fill-imported { background: var(--success); }
    .ratio-fill-duplicates { background: var(--cyan); }
    .ratio-fill-errors { background: var(--danger); }
    .legend-row {
      display: flex;
      gap: 1.5rem;
      font-size: 0.8rem;
      color: var(--text-muted);
      flex-wrap: wrap;
    }
    .legend-item {
      display: inline-flex;
      align-items: center;
      gap: 0.4rem;
    }
    .dot {
      width: 8px;
      height: 8px;
      border-radius: 50%;
      display: inline-block;
    }

    /* Tabs Navigation */
    .tabs-nav {
      display: flex;
      gap: 0.5rem;
      border-bottom: 1px solid var(--border);
      margin-bottom: 1.5rem;
      overflow-x: auto;
      padding-bottom: 2px;
    }
    .tab-btn {
      background: transparent;
      border: none;
      color: var(--text-muted);
      padding: 0.75rem 1.25rem;
      font-size: 0.9rem;
      font-weight: 600;
      cursor: pointer;
      border-bottom: 2px solid transparent;
      display: inline-flex;
      align-items: center;
      gap: 0.5rem;
      transition: all 0.2s ease;
      white-space: nowrap;
    }
    .tab-btn:hover {
      color: var(--text);
    }
    .tab-btn.active {
      color: var(--cyan);
      border-bottom-color: var(--cyan);
    }
    .tab-badge {
      background: rgba(255, 255, 255, 0.08);
      padding: 0.15rem 0.45rem;
      border-radius: 9999px;
      font-size: 0.72rem;
    }
    .tab-content {
      display: none;
      animation: fadeIn 0.25s ease;
    }
    .tab-content.active {
      display: block;
    }
    @keyframes fadeIn {
      from { opacity: 0; transform: translateY(4px); }
      to { opacity: 1; transform: translateY(0); }
    }

    /* Filter & Search Bar */
    .filter-panel {
      background: var(--surface);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 1rem 1.25rem;
      margin-bottom: 1.25rem;
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 1rem;
    }
    .search-group {
      position: relative;
      flex: 1;
      min-width: 260px;
    }
    .search-icon-wrapper {
      position: absolute;
      left: 1rem;
      top: 50%;
      transform: translateY(-50%);
      color: var(--text-dim);
      display: flex;
      align-items: center;
      pointer-events: none;
    }
    .search-input {
      width: 100%;
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 8px;
      padding: 0.65rem 2.2rem 0.65rem 2.5rem;
      color: var(--text);
      font-size: 0.88rem;
      outline: none;
      transition: border-color 0.2s ease;
    }
    .search-input:focus {
      border-color: var(--cyan);
      box-shadow: 0 0 0 2px rgba(0, 229, 255, 0.15);
    }
    .search-clear {
      position: absolute;
      right: 0.75rem;
      top: 50%;
      transform: translateY(-50%);
      background: transparent;
      border: none;
      color: var(--text-dim);
      cursor: pointer;
      display: none;
      padding: 4px;
    }
    .search-clear:hover { color: var(--text); }
    .filter-chips {
      display: flex;
      gap: 0.5rem;
      flex-wrap: wrap;
    }
    .chip {
      background: var(--card);
      border: 1px solid var(--border);
      color: var(--text-muted);
      padding: 0.4rem 0.75rem;
      border-radius: 9999px;
      font-size: 0.78rem;
      font-weight: 600;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      transition: all 0.2s ease;
    }
    .chip:hover {
      border-color: var(--accent);
      color: var(--text);
    }
    .chip.active {
      background: rgba(0, 229, 255, 0.15);
      border-color: var(--cyan);
      color: var(--cyan);
    }

    /* Tables */
    .table-card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 14px;
      overflow: hidden;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.15);
      margin-bottom: 1.5rem;
    }
    .table-responsive {
      overflow-x: auto;
    }
    table {
      width: 100%;
      border-collapse: collapse;
      text-align: left;
      font-size: 0.88rem;
    }
    th {
      background: rgba(15, 23, 42, 0.7);
      color: var(--text-muted);
      font-size: 0.75rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      padding: 0.85rem 1rem;
      border-bottom: 1px solid var(--border);
      user-select: none;
      white-space: nowrap;
    }
    th.sortable {
      cursor: pointer;
    }
    th.sortable:hover {
      color: var(--cyan);
    }
    td {
      padding: 0.85rem 1rem;
      border-bottom: 1px solid rgba(51, 65, 85, 0.5);
      color: var(--text);
    }
    tr:last-child td {
      border-bottom: none;
    }
    tr:hover td {
      background: rgba(255, 255, 255, 0.02);
    }
    .muted-cell {
      color: var(--text-muted);
      font-size: 0.82rem;
    }
    .highlight {
      background: rgba(0, 229, 255, 0.2);
      color: var(--cyan);
      font-weight: 700;
      padding: 0 2px;
      border-radius: 2px;
    }

    /* Badges */
    .badge {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      padding: 0.25rem 0.65rem;
      border-radius: 9999px;
      font-size: 0.72rem;
      font-weight: 700;
      letter-spacing: 0.2px;
    }
    .badge-success { background: rgba(16, 185, 129, 0.15); color: #34d399; border: 1px solid rgba(16, 185, 129, 0.3); }
    .badge-duplicate { background: rgba(0, 164, 239, 0.15); color: var(--cyan); border: 1px solid rgba(0, 164, 239, 0.3); }
    .badge-error { background: rgba(239, 68, 68, 0.15); color: #f87171; border: 1px solid rgba(239, 68, 68, 0.3); }

    /* Pagination */
    .pagination-bar {
      display: flex;
      justify-content: space-between;
      align-items: center;
      padding: 0.85rem 1.25rem;
      background: rgba(15, 23, 42, 0.5);
      border-top: 1px solid var(--border);
      flex-wrap: wrap;
      gap: 0.75rem;
      font-size: 0.82rem;
      color: var(--text-muted);
    }
    .page-controls {
      display: flex;
      gap: 0.35rem;
    }
    .page-btn {
      background: var(--card);
      border: 1px solid var(--border);
      color: var(--text);
      padding: 0.35rem 0.75rem;
      border-radius: 6px;
      font-size: 0.8rem;
      cursor: pointer;
      display: inline-flex;
      align-items: center;
      gap: 0.3rem;
    }
    .page-btn:disabled {
      opacity: 0.4;
      cursor: not-allowed;
    }
    .page-btn:not(:disabled):hover {
      border-color: var(--cyan);
      color: var(--cyan);
    }

    /* Config Matrix & Details */
    .audit-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(320px, 1fr));
      gap: 1.25rem;
      margin-bottom: 2rem;
    }
    .info-card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 14px;
      padding: 1.4rem;
    }
    .info-card h3 {
      font-size: 1rem;
      color: var(--cyan);
      margin-bottom: 1rem;
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    .info-row {
      display: flex;
      justify-content: space-between;
      padding: 0.65rem 0;
      border-bottom: 1px solid rgba(51, 65, 85, 0.4);
      font-size: 0.85rem;
    }
    .info-row:last-child {
      border-bottom: none;
    }
    .info-label {
      color: var(--text-muted);
    }
    .info-val {
      font-weight: 600;
      color: var(--text);
      text-align: right;
    }

    /* Logs Terminal View */
    .terminal-box {
      background: #060a14;
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 1.25rem;
      font-family: ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, "Liberation Mono", "Courier New", monospace;
      font-size: 0.82rem;
      max-height: 480px;
      overflow-y: auto;
      line-height: 1.6;
    }
    .log-line {
      margin-bottom: 0.35rem;
      word-break: break-all;
    }
    .log-info { color: #94a3b8; }
    .log-warn { color: var(--warning); }
    .log-error { color: var(--danger); font-weight: 700; }
    .log-success { color: var(--success); }

    /* Storage Impact Section Styles */
    .storage-impact-panel {
      background: linear-gradient(135deg, rgba(30, 41, 59, 0.7) 0%, rgba(15, 23, 42, 0.9) 100%);
      border: 1px solid var(--border);
      border-radius: 16px;
      padding: 1.75rem;
      margin-bottom: 2rem;
      box-shadow: 0 10px 25px -5px rgba(0, 0, 0, 0.3);
    }
    .storage-panel-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 1rem;
      margin-bottom: 1.5rem;
      border-bottom: 1px solid rgba(51, 65, 85, 0.6);
      padding-bottom: 1.25rem;
    }
    .storage-title-area {
      display: flex;
      align-items: center;
      gap: 0.85rem;
    }
    .storage-icon-badge {
      width: 44px;
      height: 44px;
      border-radius: 12px;
      background: rgba(0, 229, 255, 0.12);
      border: 1px solid rgba(0, 229, 255, 0.3);
      display: flex;
      align-items: center;
      justify-content: center;
      color: var(--cyan);
      flex-shrink: 0;
    }
    .storage-icon-badge svg {
      width: 22px;
      height: 22px;
    }
    .storage-title-area h2 {
      font-size: 1.25rem;
      color: var(--text);
      letter-spacing: -0.3px;
    }
    .storage-subtitle {
      font-size: 0.82rem;
      color: var(--text-muted);
      margin-top: 0.2rem;
    }
    .storage-header-tags {
      display: flex;
      gap: 0.5rem;
      align-items: center;
    }
    .storage-kpis-grid {
      display: grid;
      grid-template-columns: repeat(auto-fit, minmax(200px, 1fr));
      gap: 1rem;
      margin-bottom: 1.5rem;
    }
    .storage-kpi-box {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 1rem 1.15rem;
      display: flex;
      flex-direction: column;
      gap: 0.25rem;
      transition: transform 0.2s ease, border-color 0.2s ease;
    }
    .storage-kpi-box:hover {
      transform: translateY(-2px);
      border-color: var(--cyan);
    }
    .storage-kpi-box .kpi-label {
      font-size: 0.72rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: var(--text-muted);
      font-weight: 600;
    }
    .storage-kpi-box .kpi-val {
      font-size: 1.45rem;
      font-weight: 800;
      color: var(--text);
      letter-spacing: -0.5px;
    }
    .storage-kpi-box .kpi-val.cyan { color: var(--cyan); }
    .storage-kpi-box .kpi-val.green { color: #34d399; }
    .storage-kpi-box .kpi-val.purple { color: #a78bfa; }
    .storage-kpi-box .kpi-hint {
      font-size: 0.75rem;
      color: var(--text-dim);
    }
    .storage-kpi-box.delta-box {
      background: linear-gradient(135deg, rgba(16, 185, 129, 0.08) 0%, rgba(15, 23, 42, 0.8) 100%);
      border-color: rgba(16, 185, 129, 0.35);
    }
    .mailbox-impact-card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 14px;
      padding: 1.35rem;
      margin-bottom: 1.5rem;
      box-shadow: 0 4px 6px -1px rgba(0, 0, 0, 0.15);
    }
    .mbx-card-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      flex-wrap: wrap;
      gap: 0.75rem;
      margin-bottom: 1.25rem;
      border-bottom: 1px solid rgba(51, 65, 85, 0.4);
      padding-bottom: 0.85rem;
    }
    .mbx-name-group {
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    .mbx-name {
      font-size: 1rem;
      color: var(--cyan);
    }
    .store-type-tag {
      font-size: 0.78rem;
      color: var(--text-muted);
      background: rgba(15, 23, 42, 0.6);
      padding: 0.15rem 0.5rem;
      border-radius: 6px;
      border: 1px solid var(--border);
    }
    .quota-pill {
      font-size: 0.82rem;
      color: var(--text-muted);
      background: rgba(15, 23, 42, 0.7);
      padding: 0.3rem 0.75rem;
      border-radius: 9999px;
      border: 1px solid var(--border);
    }
    .quota-pill strong {
      color: var(--text);
    }
    .mbx-comparison-grid {
      display: grid;
      grid-template-columns: 1fr auto 1fr;
      gap: 1.25rem;
      align-items: center;
    }
    @media (max-width: 860px) {
      .mbx-comparison-grid {
        grid-template-columns: 1fr;
      }
      .comparison-arrow-col {
        padding: 0.5rem 0;
        transform: rotate(90deg);
      }
    }
    .comparison-col {
      background: rgba(15, 23, 42, 0.55);
      border: 1px solid var(--border);
      border-radius: 12px;
      padding: 1.15rem;
    }
    .before-col {
      border-left: 3px solid #64748b;
    }
    .after-col {
      border-left: 3px solid var(--cyan);
      background: linear-gradient(135deg, rgba(0, 229, 255, 0.03) 0%, rgba(15, 23, 42, 0.6) 100%);
    }
    .phase-header {
      display: flex;
      justify-content: space-between;
      align-items: center;
      margin-bottom: 0.85rem;
    }
    .phase-badge {
      display: inline-flex;
      align-items: center;
      gap: 0.35rem;
      font-size: 0.72rem;
      font-weight: 700;
      letter-spacing: 0.5px;
      text-transform: uppercase;
      padding: 0.2rem 0.55rem;
      border-radius: 6px;
    }
    .before-badge {
      background: rgba(100, 116, 139, 0.2);
      color: #94a3b8;
      border: 1px solid rgba(100, 116, 139, 0.3);
    }
    .after-badge {
      background: rgba(0, 229, 255, 0.15);
      color: var(--cyan);
      border: 1px solid rgba(0, 229, 255, 0.3);
    }
    .health-indicator {
      font-size: 0.78rem;
      font-weight: 700;
    }
    .storage-metric-row {
      display: flex;
      justify-content: space-between;
      gap: 0.75rem;
      margin-bottom: 1rem;
    }
    .storage-box {
      flex: 1;
      display: flex;
      flex-direction: column;
      gap: 0.15rem;
    }
    .storage-box.free-box {
      text-align: right;
    }
    .box-label {
      font-size: 0.72rem;
      color: var(--text-muted);
      text-transform: uppercase;
    }
    .box-value {
      font-size: 1.2rem;
      font-weight: 700;
      color: var(--text);
    }
    .box-value.after-val {
      color: #f8fafc;
    }
    .free-val {
      color: #34d399;
    }
    .free-val-after {
      color: var(--cyan);
    }
    .box-sub {
      font-size: 0.7rem;
      color: var(--text-dim);
    }
    .impact-bar-wrapper {
      margin-top: 0.5rem;
    }
    .impact-bar-outer {
      height: 10px;
      background: rgba(15, 23, 42, 0.9);
      border-radius: 9999px;
      overflow: hidden;
      border: 1px solid rgba(51, 65, 85, 0.6);
      display: flex;
    }
    .impact-bar-fill.before-fill {
      background: linear-gradient(90deg, #3b82f6, #06b6d4);
      height: 100%;
      border-radius: 9999px;
      transition: width 0.6s ease;
    }
    .impact-bar-fill.base-fill {
      background: linear-gradient(90deg, #3b82f6, #06b6d4);
      height: 100%;
    }
    .impact-bar-fill.delta-fill {
      background: linear-gradient(90deg, #10b981, #34d399);
      height: 100%;
      box-shadow: 0 0 8px rgba(16, 185, 129, 0.6);
    }
    .impact-bar-labels {
      display: flex;
      justify-content: space-between;
      margin-top: 0.35rem;
      font-size: 0.72rem;
      color: var(--text-muted);
    }
    .comparison-arrow-col {
      display: flex;
      flex-direction: column;
      align-items: center;
      justify-content: center;
      gap: 0.65rem;
    }
    .delta-badge {
      background: linear-gradient(135deg, rgba(16, 185, 129, 0.15) 0%, rgba(15, 23, 42, 0.8) 100%);
      border: 1px solid rgba(16, 185, 129, 0.4);
      border-radius: 12px;
      padding: 0.65rem 0.95rem;
      display: flex;
      flex-direction: column;
      align-items: center;
      gap: 0.15rem;
      text-align: center;
      box-shadow: 0 4px 12px rgba(16, 185, 129, 0.15);
    }
    .delta-title {
      font-size: 0.65rem;
      text-transform: uppercase;
      letter-spacing: 0.5px;
      color: #34d399;
      font-weight: 700;
    }
    .delta-value {
      font-size: 1.15rem;
      font-weight: 800;
      color: #34d399;
    }
    .delta-sub {
      font-size: 0.7rem;
      color: var(--text-muted);
    }
    .delta-items {
      font-size: 0.68rem;
      color: var(--cyan);
      font-weight: 600;
    }
    .arrow-svg-wrapper {
      filter: drop-shadow(0 0 6px rgba(0, 229, 255, 0.5));
    }
    .storage-table-card {
      background: var(--card);
      border: 1px solid var(--border);
      border-radius: 14px;
      overflow: hidden;
      margin-top: 1.25rem;
    }
    .storage-table-header {
      padding: 1rem 1.25rem;
      border-bottom: 1px solid var(--border);
      background: rgba(15, 23, 42, 0.6);
    }
    .storage-table-header h3 {
      font-size: 0.95rem;
      color: var(--cyan);
      display: flex;
      align-items: center;
      gap: 0.5rem;
    }
    .storage-comparison-table th {
      font-size: 0.72rem;
    }
    .health-pill {
      font-weight: 700;
      font-size: 0.78rem;
    }

    /* Footer */
    .footer {
      text-align: center;
      color: var(--text-muted);
      font-size: 0.82rem;
      margin-top: 3rem;
      border-top: 1px solid var(--border);
      padding-top: 1.75rem;
      display: flex;
      justify-content: space-between;
      flex-wrap: wrap;
      gap: 1rem;
    }
    .footer strong {
      color: var(--cyan);
    }

    /* Toast */
    #toast {
      position: fixed;
      bottom: 2rem;
      right: 2rem;
      background: var(--surface);
      border: 1px solid var(--cyan);
      color: var(--text);
      padding: 0.75rem 1.25rem;
      border-radius: 8px;
      box-shadow: 0 10px 25px rgba(0, 0, 0, 0.5);
      display: none;
      z-index: 1000;
      font-size: 0.85rem;
      font-weight: 600;
    }

    @media print {
      body { background: #fff; color: #000; padding: 0; }
      .header, .kpi-card, .info-card, .table-card { border-color: #ccc; background: #fff; box-shadow: none; color: #000; }
      .btn, .filter-panel, .tabs-nav { display: none; }
      .tab-content { display: block !important; margin-bottom: 2rem; }
    }
  </style>
</head>
<body>
  <div class="container">
    <!-- Header -->
    <header class="header">
      <div class="header-left">
        <h1>
          <svg class="header-title-svg" viewBox="0 0 24 24" fill="none" stroke="var(--cyan)" stroke-width="2.5">
            <polygon points="12 2 22 12 12 22 2 12"/>
          </svg>
          Informe de Migración &amp; Auditoría
        </h1>
        <div class="header-meta">
          <span class="meta-pill">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="3" y="4" width="18" height="18" rx="2" ry="2"/><line x1="16" y1="2" x2="16" y2="6"/><line x1="8" y1="2" x2="8" y2="6"/><line x1="3" y1="10" x2="21" y2="10"/></svg>
            Fecha: <strong>__DATE_STR__</strong>
          </span>
          <span class="meta-pill">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/></svg>
            Hora: <strong>__TIME_STR__</strong>
          </span>
          <span class="meta-pill">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polygon points="13 2 3 14 12 14 11 22 21 10 12 10 13 2"/></svg>
            Motor: <strong>Outlook COM / MAPI</strong>
          </span>
          <span class="meta-pill">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2"/><circle cx="12" cy="7" r="4"/></svg>
            Perfil: <strong>__PROFILE_INFO__</strong>
          </span>
        </div>
      </div>
      <div class="header-right">
        <div class="brand-box">
          <div class="brand-badge">
            <svg width="24" height="24" viewBox="0 0 24 24" fill="none" stroke="var(--cyan)" stroke-width="2.5">
              <polygon points="12 2 22 12 12 22 2 12" />
            </svg>
            <div class="brand-text">
              <span class="brand-title">TIMELESS</span>
              <span class="brand-subtitle">SUPPORT</span>
            </div>
          </div>
        </div>
        <div class="action-bar">
          <button class="btn btn-primary" onclick="window.print()">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="6 9 6 2 18 2 18 9"/><path d="M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2"/><rect x="6" y="14" width="12" height="8"/></svg>
            Imprimir / PDF
          </button>
          <button class="btn" onclick="copyExecutiveSummary()">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2"/><rect x="8" y="2" width="8" height="4" rx="1" ry="1"/></svg>
            Copiar Resumen
          </button>
          <button class="btn" onclick="exportEmailsToCsv()">
            <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
            Exportar CSV
          </button>
        </div>
      </div>
    </header>

    <!-- Executive KPI Grid -->
    <div class="kpi-grid">
      <div class="kpi-card kpi-imported">
        <div class="kpi-title">
          <span>Correos Importados</span>
          <span class="badge badge-success"><svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg> Transferidos</span>
        </div>
        <div class="kpi-value success">__IMPORTED_COUNT__</div>
        <div class="kpi-sub">__IMPORTED_PCT__ del total de elementos procesados</div>
      </div>

      <div class="kpi-card kpi-duplicates">
        <div class="kpi-title">
          <span>Duplicados Omitidos</span>
          <span class="badge badge-duplicate"><svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg> Conservados</span>
        </div>
        <div class="kpi-value cyan">__DUPLICATES_COUNT__</div>
        <div class="kpi-sub">__DUPLICATES_PCT__ detectados por Message-ID / Hash</div>
      </div>

      <div class="kpi-card kpi-errors">
        <div class="kpi-title">
          <span>Errores de Lectura</span>
          <span class="badge badge-error"><svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg> Incidentes</span>
        </div>
        <div class="kpi-value danger">__ERRORS_COUNT__</div>
        <div class="kpi-sub">__ERRORS_PCT__ de fallos MAPI registrados</div>
      </div>

      <div class="kpi-card kpi-target">
        <div class="kpi-title">
          <span>Buzón(es) Destino</span>
          <span class="badge badge-duplicate"><svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/><polyline points="22,6 12,13 2,6"/></svg> MAPI Store</span>
        </div>
        <div class="kpi-value purple">__MAILBOX_NAME__</div>
        <div class="kpi-sub">Modo: <strong>__TRANSFER_MODE__</strong></div>
      </div>
    </div>

    <!-- Distribution Bar -->
    <div class="distribution-card">
      <div class="distribution-header">
        <span>Distribución de Correos Procesados (Total: __TOTAL_COUNT__)</span>
        <span style="color: var(--cyan); font-weight: 700;">100% Analizado</span>
      </div>
      <div class="ratio-bar">
        <div class="ratio-fill-imported" style="width: __IMPORTED_PCT__;" title="Importados: __IMPORTED_COUNT__"></div>
        <div class="ratio-fill-duplicates" style="width: __DUPLICATES_PCT__;" title="Duplicados: __DUPLICATES_COUNT__"></div>
        <div class="ratio-fill-errors" style="width: __ERRORS_PCT__;" title="Errores: __ERRORS_COUNT__"></div>
      </div>
      <div class="legend-row">
        <div class="legend-item"><span class="dot" style="background: var(--success);"></span> Importados: <strong>__IMPORTED_COUNT__ (__IMPORTED_PCT__)</strong></div>
        <div class="legend-item"><span class="dot" style="background: var(--cyan);"></span> Duplicados Omitidos: <strong>__DUPLICATES_COUNT__ (__DUPLICATES_PCT__)</strong></div>
        <div class="legend-item"><span class="dot" style="background: var(--danger);"></span> Errores de Lectura: <strong>__ERRORS_COUNT__ (__ERRORS_PCT__)</strong></div>
      </div>
    </div>

    <!-- Navigation Tabs -->
    <nav class="tabs-nav">
      <button class="tab-btn active" onclick="switchTab('tab-summary')">
        <svg class="icon icon-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polygon points="12 2 22 12 12 22 2 12"/></svg>
        Resumen &amp; Auditoría
      </button>
      <button class="tab-btn" onclick="switchTab('tab-folders')">
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
        Carpetas Importadas <span class="tab-badge" id="folderTabCount">0</span>
      </button>
      <button class="tab-btn" onclick="switchTab('tab-emails')">
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M4 4h16c1.1 0 2 .9 2 2v12c0 1.1-.9 2-2 2H4c-1.1 0-2-.9-2-2V6c0-1.1.9-2 2-2z"/><polyline points="22,6 12,13 2,6"/></svg>
        Explorador de Correos <span class="tab-badge" id="emailTabCount">0</span>
      </button>
      <button class="tab-btn" onclick="switchTab('tab-logs')">
        <svg class="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><polyline points="4 17 10 11 4 5"/><line x1="12" y1="19" x2="20" y2="19"/></svg>
        Registro de Telemetría <span class="tab-badge" id="logTabCount">0</span>
      </button>
    </nav>

    <!-- TAB 1: Resumen & Auditoría -->
    <section id="tab-summary" class="tab-content active">
      __STORAGE_IMPACT_SECTION__

      <div class="table-card" style="margin-bottom: 2rem;">
        <div style="padding: 1.25rem 1.5rem; border-bottom: 1px solid var(--border);">
          <h2 style="font-size: 1.15rem; color: var(--cyan); display: flex; align-items: center; gap: 0.5rem;">
            <svg class="icon icon-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>
            Archivos PST Procesados en la Sesión
          </h2>
        </div>
        <div class="table-responsive">
          <table>
            <thead>
              <tr>
                <th>Archivo PST</th>
                <th>Ruta en Disco</th>
                <th>Tamaño</th>
                <th>Estado MAPI</th>
              </tr>
            </thead>
            <tbody>
              __PST_ROWS__
            </tbody>
          </table>
        </div>
      </div>

      <div class="audit-grid">
        <div class="info-card">
          <h3>
            <svg class="icon icon-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="3"/><path d="M19.4 15a1.65 1.65 0 0 0 .33 1.82l.06.06a2 2 0 0 1 0 2.83 2 2 0 0 1-2.83 0l-.06-.06a1.65 1.65 0 0 0-1.82-.33 1.65 1.65 0 0 0-1 1.51V21a2 2 0 0 1-2 2 2 2 0 0 1-2-2v-.09A1.65 1.65 0 0 0 9 19.4a1.65 1.65 0 0 0-1.82.33l-.06.06a2 2 0 0 1-2.83 0 2 2 0 0 1 0-2.83l.06-.06a1.65 1.65 0 0 0 .33-1.82 1.65 1.65 0 0 0-1.51-1H3a2 2 0 0 1-2-2 2 2 0 0 1 2-2h.09A1.65 1.65 0 0 0 4.6 9a1.65 1.65 0 0 0-.33-1.82l-.06-.06a2 2 0 0 1 0-2.83 2 2 0 0 1 2.83 0l.06.06a1.65 1.65 0 0 0 1.82.33H9a1.65 1.65 0 0 0 1-1.51V3a2 2 0 0 1 2-2 2 2 0 0 1 2 2v.09a1.65 1.65 0 0 0 1 1.51 1.65 1.65 0 0 0 1.82-.33l.06-.06a2 2 0 0 1 2.83 0 2 2 0 0 1 0 2.83l-.06.06a1.65 1.65 0 0 0-.33 1.82V9a1.65 1.65 0 0 0 1.51 1H21a2 2 0 0 1 2 2 2 2 0 0 1-2 2h-.09a1.65 1.65 0 0 0-1.51 1z"/></svg>
            Políticas de Transferencia
          </h3>
          <div class="info-row">
            <span class="info-label">Acción sobre el Correo:</span>
            <span class="info-val">__TRANSFER_MODE__</span>
          </div>
          <div class="info-row">
            <span class="info-label">Estrategia de Enrutamiento:</span>
            <span class="info-val">__ROUTING_INFO__</span>
          </div>
          <div class="info-row">
            <span class="info-label">Detección de Duplicados:</span>
            <span class="info-val">__DEDUP_INFO__</span>
          </div>
          <div class="info-row">
            <span class="info-label">Filtro de Rango Temporal:</span>
            <span class="info-val">__DATE_FILTER_INFO__</span>
          </div>
        </div>

        <div class="info-card">
          <h3>
            <svg class="icon icon-cyan" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><path d="M12 22s8-4 8-10V5l-8-3-8 3v7c0 6 8 10 8 10z"/></svg>
            Seguridad &amp; Rendimiento MAPI
          </h3>
          <div class="info-row">
            <span class="info-label">Throttling Adaptativo:</span>
            <span class="info-val">__THROTTLING_INFO__</span>
          </div>
          <div class="info-row">
            <span class="info-label">Protocolo de Cancelación:</span>
            <span class="info-val">Graceful Shutdown Activo</span>
          </div>
          <div class="info-row">
            <span class="info-label">Desmontaje Seguro (RemoveStore):</span>
            <span class="info-val">Garantizado en Finally</span>
          </div>
          <div class="info-row">
            <span class="info-label">Gestión de Memoria COM:</span>
            <span class="info-val">ReleaseComObject + GC Forzado</span>
          </div>
        </div>
      </div>
    </section>

    <!-- TAB 2: Carpetas Importadas -->
    <section id="tab-folders" class="tab-content">
      <div class="filter-panel">
        <div class="search-group">
          <span class="search-icon-wrapper">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          </span>
          <input type="text" id="folderSearch" class="search-input" placeholder="Buscar carpeta origen, PST o ruta de destino..." oninput="onFolderSearchChange()">
          <button id="folderSearchClear" class="search-clear" onclick="clearFolderSearch()">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
        <div style="font-size: 0.85rem; color: var(--text-muted);">
          Mostrando <strong id="folderMatchCount" style="color: var(--cyan);">0</strong> de <span id="folderTotalCount">0</span> carpetas
        </div>
      </div>

      <div class="table-card">
        <div class="table-responsive">
          <table id="foldersTable">
            <thead>
              <tr>
                <th class="sortable" onclick="sortFolders('pst_name')">Archivo PST ⇅</th>
                <th class="sortable" onclick="sortFolders('source_folder')">Carpeta Origen (PST) ⇅</th>
                <th class="sortable" onclick="sortFolders('dest_folder')">Destino (Outlook MAPI) ⇅</th>
                <th class="sortable" onclick="sortFolders('total_items')">Correos Procesados ⇅</th>
                <th class="sortable" onclick="sortFolders('size_mb')">Tamaño ⇅</th>
                <th>Estado</th>
              </tr>
            </thead>
            <tbody id="foldersTableBody">
              <!-- Renderizado dinámico vía JavaScript -->
            </tbody>
          </table>
        </div>
      </div>
    </section>

    <!-- TAB 3: Explorador de Correos (Búsqueda por Título y Metadatos) -->
    <section id="tab-emails" class="tab-content">
      <div class="filter-panel">
        <div class="search-group">
          <span class="search-icon-wrapper">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          </span>
          <input type="text" id="emailSearch" class="search-input" placeholder="Buscar por Asunto / Título del correo, remitente o carpeta..." oninput="onEmailSearchChange()">
          <button id="emailSearchClear" class="search-clear" onclick="clearEmailSearch()">
            <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><line x1="18" y1="6" x2="6" y2="18"/><line x1="6" y1="6" x2="18" y2="18"/></svg>
          </button>
        </div>
        <div class="filter-chips">
          <button class="chip active" id="filter-all" onclick="setEmailStatusFilter('all')">
            Todos (<span id="count-all">0</span>)
          </button>
          <button class="chip" id="filter-imported" onclick="setEmailStatusFilter('Importado')">
            <svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>
            Importados (<span id="count-imported">0</span>)
          </button>
          <button class="chip" id="filter-dup" onclick="setEmailStatusFilter('Duplicado Omitido')">
            <svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>
            Duplicados (<span id="count-dup">0</span>)
          </button>
          <button class="chip" id="filter-err" onclick="setEmailStatusFilter('Error')">
            <svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>
            Errores (<span id="count-err">0</span>)
          </button>
        </div>
      </div>

      <div class="table-card">
        <div class="table-responsive">
          <table id="emailsTable">
            <thead>
              <tr>
                <th style="width: 45px;">#</th>
                <th class="sortable" onclick="sortEmails('subject')">Asunto / Título del Correo ⇅</th>
                <th class="sortable" onclick="sortEmails('sender')">De / Remitente ⇅</th>
                <th class="sortable" onclick="sortEmails('date')">Fecha y Hora ⇅</th>
                <th class="sortable" onclick="sortEmails('source_folder')">Carpeta Origen ⇅</th>
                <th class="sortable" onclick="sortEmails('dest_folder')">Destino MAPI ⇅</th>
                <th class="sortable" onclick="sortEmails('size_kb')">Tamaño ⇅</th>
                <th>Estado</th>
              </tr>
            </thead>
            <tbody id="emailsTableBody">
              <!-- Renderizado dinámico vía JavaScript -->
            </tbody>
          </table>
        </div>

        <div class="pagination-bar">
          <div>
            Mostrando <strong id="emailPageStart">0</strong> - <strong id="emailPageEnd">0</strong> de <strong id="emailMatchTotal">0</strong> correos encontrados
          </div>
          <div style="display: flex; align-items: center; gap: 1rem;">
            <div>
              Filas por página:
              <select id="rowsPerPageSelect" onchange="changeRowsPerPage(this.value)" style="background: var(--surface); color: var(--text); border: 1px solid var(--border); border-radius: 4px; padding: 2px 6px;">
                <option value="25" selected>25</option>
                <option value="50">50</option>
                <option value="100">100</option>
                <option value="250">250</option>
              </select>
            </div>
            <div class="page-controls">
              <button id="prevPageBtn" class="page-btn" onclick="prevEmailPage()">
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="15 18 9 12 15 6"/></svg>
                Anterior
              </button>
              <span id="pageIndicator" style="line-height: 2; padding: 0 4px;">Página 1</span>
              <button id="nextPageBtn" class="page-btn" onclick="nextEmailPage()">
                Siguiente
                <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="9 18 15 12 9 6"/></svg>
              </button>
            </div>
          </div>
        </div>
      </div>
    </section>

    <!-- TAB 4: Logs de Telemetría -->
    <section id="tab-logs" class="tab-content">
      <div class="filter-panel">
        <div class="search-group">
          <span class="search-icon-wrapper">
            <svg width="16" height="16" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="11" cy="11" r="8"/><line x1="21" y1="21" x2="16.65" y2="16.65"/></svg>
          </span>
          <input type="text" id="logSearch" class="search-input" placeholder="Filtrar eventos y registros del log..." oninput="onLogSearchChange()">
        </div>
        <div class="filter-chips">
          <button class="chip active" id="log-filter-all" onclick="setLogFilter('all')">Todos</button>
          <button class="chip" id="log-filter-info" onclick="setLogFilter('INFO')">Info</button>
          <button class="chip" id="log-filter-warn" onclick="setLogFilter('WARN')">Avisos</button>
          <button class="chip" id="log-filter-error" onclick="setLogFilter('ERROR')">Errores</button>
        </div>
      </div>
      <div class="terminal-box" id="terminalContainer">
        <!-- Renderizado dinámico vía JavaScript -->
      </div>
    </section>

    <!-- Footer -->
    <footer class="footer">
      <div>
        Generado automáticamente por <strong>Outlook Organizer TS</strong> &bull; Edición Rust 2024
      </div>
      <div>
        &copy; __YEAR__ <strong>Timeless Support</strong> &bull; Soporte &amp; Infraestructura de TI
      </div>
    </footer>
  </div>

  <div id="toast">Notificación</div>

  <!-- Inyección de Datos Estructurados JSON -->
  <script type="application/json" id="rawFoldersData">__FOLDERS_JSON__</script>
  <script type="application/json" id="rawEmailsData">__EMAILS_JSON__</script>
  <script type="application/json" id="rawLogsData">__LOGS_JSON__</script>

  <!-- Lógica Interactiva en Vanilla JavaScript -->
  <script>
    let folders = [];
    let emails = [];
    let logs = [];

    try {
      folders = JSON.parse(document.getElementById("rawFoldersData").textContent || "[]");
      emails = JSON.parse(document.getElementById("rawEmailsData").textContent || "[]");
      logs = JSON.parse(document.getElementById("rawLogsData").textContent || "[]");
    } catch(e) {
      console.error("Error al cargar datos JSON:", e);
    }

    // Inicializar contadores en badges de navegación
    document.getElementById("folderTabCount").innerText = folders.length.toLocaleString();
    document.getElementById("emailTabCount").innerText = emails.length.toLocaleString();
    document.getElementById("logTabCount").innerText = logs.length.toLocaleString();

    // Estado de Pestañas
    function switchTab(tabId) {
      document.querySelectorAll(".tab-btn").forEach(btn => btn.classList.remove("active"));
      document.querySelectorAll(".tab-content").forEach(c => c.classList.remove("active"));

      const targetContent = document.getElementById(tabId);
      if (targetContent) {
        targetContent.classList.add("active");
      }

      const activeBtn = Array.from(document.querySelectorAll(".tab-btn")).find(btn => {
        return btn.getAttribute("onclick") && btn.getAttribute("onclick").includes(tabId);
      });
      if (activeBtn) activeBtn.classList.add("active");
    }

    // Funciones dinámicas de formateo de almacenamiento
    function formatSizeMb(mb) {
      if (mb === undefined || mb === null || mb <= 0) return "0 MB";
      if (mb >= 1000000) return (mb / 1048576).toFixed(2) + " TB";
      if (mb >= 1000) return (mb / 1024).toFixed(2) + " GB";
      if (mb >= 1) return mb >= 100 ? mb.toFixed(1) + " MB" : mb.toFixed(2) + " MB";
      const kb = mb * 1024;
      return kb >= 1 ? (kb >= 100 ? kb.toFixed(0) + " KB" : kb.toFixed(1) + " KB") : (kb * 1024).toFixed(0) + " B";
    }

    function formatSizeKb(kb) {
      if (kb === undefined || kb === null || kb <= 0) return "-";
      if (kb >= 1024 * 1000000) return (kb / (1024 * 1048576)).toFixed(2) + " TB";
      if (kb >= 1024 * 1000) return (kb / (1024 * 1024)).toFixed(2) + " GB";
      if (kb >= 1000) return (kb / 1024).toFixed(2) + " MB";
      if (kb < 1) return (kb * 1024).toFixed(0) + " B";
      return kb >= 100 ? kb.toFixed(0) + " KB" : kb.toFixed(1) + " KB";
    }

    // ==========================================
    // TAB 2: Lógica de Carpetas
    // ==========================================
    let folderSortColumn = "source_folder";
    let folderSortAsc = true;

    function renderFoldersTable() {
      const q = (document.getElementById("folderSearch").value || "").trim().toLowerCase();
      const filtered = folders.filter(f => {
        if (!q) return true;
        return (f.pst_name && f.pst_name.toLowerCase().includes(q)) ||
               (f.source_folder && f.source_folder.toLowerCase().includes(q)) ||
               (f.dest_folder && f.dest_folder.toLowerCase().includes(q));
      });

      filtered.sort((a, b) => {
        let valA = a[folderSortColumn];
        let valB = b[folderSortColumn];
        if (typeof valA === "string") valA = valA.toLowerCase();
        if (typeof valB === "string") valB = valB.toLowerCase();
        if (valA < valB) return folderSortAsc ? -1 : 1;
        if (valA > valB) return folderSortAsc ? 1 : -1;
        return 0;
      });

      const tbody = document.getElementById("foldersTableBody");
      document.getElementById("folderMatchCount").innerText = filtered.length;
      document.getElementById("folderTotalCount").innerText = folders.length;

      if (filtered.length === 0) {
        tbody.innerHTML = `<tr><td colspan="6" style="text-align: center; padding: 2.5rem; color: var(--text-muted);">No se encontraron carpetas coincidentes con el criterio de búsqueda.</td></tr>`;
        return;
      }

      const folderSvg = `<svg class="cell-svg" viewBox="0 0 24 24" fill="none" stroke="var(--cyan)" stroke-width="2"><path d="M22 19a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V5a2 2 0 0 1 2-2h5l2 3h9a2 2 0 0 1 2 2z"/></svg>`;
      const arrowRightSvg = `<svg class="path-arrow" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>`;
      const arrowDestSvg = `<svg class="path-arrow" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="9 10 4 15 9 20"/><path d="M20 4v7a4 4 0 0 1-4 4H4"/></svg>`;
      const checkSvg = `<svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>`;

      tbody.innerHTML = filtered.map(f => `
        <tr>
          <td>${folderSvg} <strong>${escapeHtml(f.pst_name)}</strong></td>
          <td>${arrowRightSvg} ${escapeHtml(f.source_folder)}</td>
          <td>${arrowDestSvg} ${escapeHtml(f.dest_folder)}</td>
          <td><strong>${f.total_items.toLocaleString()}</strong> correos</td>
          <td>${formatSizeMb(f.size_mb)}</td>
          <td><span class="badge badge-success">${checkSvg} ${escapeHtml(f.status || 'Completado')}</span></td>
        </tr>
      `).join("");
    }

    function onFolderSearchChange() {
      const q = document.getElementById("folderSearch").value;
      document.getElementById("folderSearchClear").style.display = q ? "block" : "none";
      renderFoldersTable();
    }

    function clearFolderSearch() {
      document.getElementById("folderSearch").value = "";
      document.getElementById("folderSearchClear").style.display = "none";
      renderFoldersTable();
    }

    function sortFolders(col) {
      if (folderSortColumn === col) {
        folderSortAsc = !folderSortAsc;
      } else {
        folderSortColumn = col;
        folderSortAsc = true;
      }
      renderFoldersTable();
    }

    // ==========================================
    // TAB 3: Lógica del Explorador de Correos
    // ==========================================
    let emailStatusFilter = "all";
    let emailPage = 1;
    let emailRowsPerPage = 25;
    let emailSortCol = "date";
    let emailSortAsc = false;

    function setEmailStatusFilter(status) {
      emailStatusFilter = status;
      emailPage = 1;
      document.querySelectorAll(".chip").forEach(c => {
        if (c.id && c.id.startsWith("filter-")) c.classList.remove("active");
      });
      if (status === "all") document.getElementById("filter-all").classList.add("active");
      else if (status === "Importado") document.getElementById("filter-imported").classList.add("active");
      else if (status === "Duplicado Omitido") document.getElementById("filter-dup").classList.add("active");
      else if (status === "Error") document.getElementById("filter-err").classList.add("active");

      renderEmailsTable();
    }

    function onEmailSearchChange() {
      const q = document.getElementById("emailSearch").value;
      document.getElementById("emailSearchClear").style.display = q ? "block" : "none";
      emailPage = 1;
      renderEmailsTable();
    }

    function clearEmailSearch() {
      document.getElementById("emailSearch").value = "";
      document.getElementById("emailSearchClear").style.display = "none";
      emailPage = 1;
      renderEmailsTable();
    }

    function sortEmails(col) {
      if (emailSortCol === col) {
        emailSortAsc = !emailSortAsc;
      } else {
        emailSortCol = col;
        emailSortAsc = true;
      }
      renderEmailsTable();
    }

    function changeRowsPerPage(val) {
      emailRowsPerPage = parseInt(val, 10) || 25;
      emailPage = 1;
      renderEmailsTable();
    }

    function prevEmailPage() {
      if (emailPage > 1) {
        emailPage--;
        renderEmailsTable();
      }
    }

    function nextEmailPage() {
      emailPage++;
      renderEmailsTable();
    }

    function renderEmailsTable() {
      let countAll = emails.length;
      let countImported = 0;
      let countDup = 0;
      let countErr = 0;

      for (let i = 0; i < emails.length; i++) {
        const st = emails[i].status;
        if (st === "Importado") countImported++;
        else if (st === "Duplicado Omitido") countDup++;
        else if (st === "Error") countErr++;
      }

      document.getElementById("count-all").innerText = countAll.toLocaleString();
      document.getElementById("count-imported").innerText = countImported.toLocaleString();
      document.getElementById("count-dup").innerText = countDup.toLocaleString();
      document.getElementById("count-err").innerText = countErr.toLocaleString();

      const q = (document.getElementById("emailSearch").value || "").trim().toLowerCase();

      const filtered = emails.filter(item => {
        if (emailStatusFilter !== "all" && item.status !== emailStatusFilter) {
          return false;
        }
        if (!q) return true;
        return (item.subject && item.subject.toLowerCase().includes(q)) ||
               (item.sender && item.sender.toLowerCase().includes(q)) ||
               (item.source_folder && item.source_folder.toLowerCase().includes(q)) ||
               (item.dest_folder && item.dest_folder.toLowerCase().includes(q));
      });

      filtered.sort((a, b) => {
        let valA = a[emailSortCol];
        let valB = b[emailSortCol];
        if (typeof valA === "string") valA = valA.toLowerCase();
        if (typeof valB === "string") valB = valB.toLowerCase();
        if (valA < valB) return emailSortAsc ? -1 : 1;
        if (valA > valB) return emailSortAsc ? 1 : -1;
        return 0;
      });

      const totalMatches = filtered.length;
      const totalPages = Math.ceil(totalMatches / emailRowsPerPage) || 1;
      if (emailPage > totalPages) emailPage = totalPages;
      if (emailPage < 1) emailPage = 1;

      const startIndex = (emailPage - 1) * emailRowsPerPage;
      const pageItems = filtered.slice(startIndex, startIndex + emailRowsPerPage);

      document.getElementById("emailMatchTotal").innerText = totalMatches.toLocaleString();
      document.getElementById("emailPageStart").innerText = totalMatches > 0 ? (startIndex + 1).toLocaleString() : 0;
      document.getElementById("emailPageEnd").innerText = Math.min(startIndex + emailRowsPerPage, totalMatches).toLocaleString();
      document.getElementById("pageIndicator").innerText = `Página ${emailPage} de ${totalPages}`;
      document.getElementById("prevPageBtn").disabled = (emailPage <= 1);
      document.getElementById("nextPageBtn").disabled = (emailPage >= totalPages);

      const tbody = document.getElementById("emailsTableBody");
      if (pageItems.length === 0) {
        tbody.innerHTML = `<tr><td colspan="8" style="text-align: center; padding: 2.5rem; color: var(--text-muted);">No se encontraron correos que coincidan con los filtros aplicados.</td></tr>`;
        return;
      }

      const checkSvg = `<svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5"><polyline points="20 6 9 17 4 12"/></svg>`;
      const dupSvg = `<svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><rect x="9" y="9" width="13" height="13" rx="2" ry="2"/><path d="M5 15H4a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h9a2 2 0 0 1 2 2v1"/></svg>`;
      const errSvg = `<svg class="badge-svg" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2"><circle cx="12" cy="12" r="10"/><line x1="12" y1="8" x2="12" y2="12"/><line x1="12" y1="16" x2="12.01" y2="16"/></svg>`;
      const arrowRightSvg = `<svg class="path-arrow" viewBox="0 0 24 24" fill="none" stroke="var(--accent)" stroke-width="2"><polyline points="9 18 15 12 9 6"/></svg>`;
      const arrowDestSvg = `<svg class="path-arrow" viewBox="0 0 24 24" fill="none" stroke="var(--success)" stroke-width="2"><polyline points="9 10 4 15 9 20"/><path d="M20 4v7a4 4 0 0 1-4 4H4"/></svg>`;

      tbody.innerHTML = pageItems.map((item, idx) => {
        const itemNumber = startIndex + idx + 1;
        let badgeClass = "badge-success";
        let badgeIcon = checkSvg;
        if (item.status === "Duplicado Omitido") {
          badgeClass = "badge-duplicate";
          badgeIcon = dupSvg;
        } else if (item.status === "Error") {
          badgeClass = "badge-error";
          badgeIcon = errSvg;
        }

        const highlightedSubject = highlightText(item.subject || "(Sin Asunto)", q);
        const highlightedSender = highlightText(item.sender || "-", q);
        const sizeStr = item.size_kb ? formatSizeKb(item.size_kb) : "-";

        return `
          <tr>
            <td style="color: var(--text-dim); font-size: 0.78rem;">${itemNumber}</td>
            <td><strong>${highlightedSubject}</strong></td>
            <td class="muted-cell">${highlightedSender}</td>
            <td class="muted-cell" style="white-space: nowrap;">${escapeHtml(item.date || '-')}</td>
            <td>${arrowRightSvg} ${escapeHtml(item.source_folder || '-')}</td>
            <td>${arrowDestSvg} ${escapeHtml(item.dest_folder || '-')}</td>
            <td class="muted-cell">${sizeStr}</td>
            <td><span class="badge ${badgeClass}">${badgeIcon} ${escapeHtml(item.status)}</span></td>
          </tr>
        `;
      }).join("");
    }

    // ==========================================
    // TAB 4: Lógica de Logs
    // ==========================================
    let currentLogLevel = "all";

    function setLogFilter(level) {
      currentLogLevel = level;
      document.querySelectorAll(".chip").forEach(c => {
        if (c.id && c.id.startsWith("log-filter-")) c.classList.remove("active");
      });
      if (level === "all") document.getElementById("log-filter-all").classList.add("active");
      else if (level === "INFO") document.getElementById("log-filter-info").classList.add("active");
      else if (level === "WARN") document.getElementById("log-filter-warn").classList.add("active");
      else if (level === "ERROR") document.getElementById("log-filter-error").classList.add("active");
      renderLogs();
    }

    function onLogSearchChange() {
      renderLogs();
    }

    function renderLogs() {
      const q = (document.getElementById("logSearch").value || "").trim().toLowerCase();
      const container = document.getElementById("terminalContainer");

      const filtered = logs.filter(line => {
        if (currentLogLevel === "INFO" && !line.includes("INFO") && line.includes("WARN")) return false;
        if (currentLogLevel === "WARN" && !line.includes("WARN")) return false;
        if (currentLogLevel === "ERROR" && !line.includes("ERROR") && !line.includes("Fallo")) return false;
        if (!q) return true;
        return line.toLowerCase().includes(q);
      });

      if (filtered.length === 0) {
        container.innerHTML = `<div style="color: var(--text-dim); text-align: center; padding: 2rem;">No hay registros para este filtro.</div>`;
        return;
      }

      container.innerHTML = filtered.map(l => {
        let cls = "log-info";
        if (l.includes("WARN") || l.includes("Aviso")) cls = "log-warn";
        else if (l.includes("ERROR") || l.includes("Fallo")) cls = "log-error";
        else if (l.includes("COMPLETADO") || l.includes("[FIN]") || l.includes("exitosamente")) cls = "log-success";
        return `<div class="log-line ${cls}">${escapeHtml(l)}</div>`;
      }).join("");
    }

    // ==========================================
    // Utilidades (Highlight, Escape, Export CSV)
    // ==========================================
    function escapeHtml(text) {
      if (!text) return "";
      return text.toString()
        .replace(/&/g, "&amp;")
        .replace(/</g, "&lt;")
        .replace(/>/g, "&gt;")
        .replace(/"/g, "&quot;")
        .replace(/'/g, "&#039;");
    }

    function highlightText(text, query) {
      if (!query || !text) return escapeHtml(text);
      const escapedQuery = query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const regex = new RegExp(`(${escapedQuery})`, "gi");
      const parts = text.split(regex);
      return parts.map(part => {
        if (part.toLowerCase() === query.toLowerCase()) {
          return `<span class="highlight">${escapeHtml(part)}</span>`;
        }
        return escapeHtml(part);
      }).join("");
    }

    function showToast(msg) {
      const t = document.getElementById("toast");
      t.innerText = msg;
      t.style.display = "block";
      setTimeout(() => { t.style.display = "none"; }, 3000);
    }

    function copyExecutiveSummary() {
      const summary = `Informe de Migración - Outlook Organizer TS\n` +
        `Fecha: __DATE_STR__ __TIME_STR__\n` +
        `Buzón Destino: __MAILBOX_NAME__\n` +
        `Importados: __IMPORTED_COUNT__ (__IMPORTED_PCT__)\n` +
        `Duplicados Omitidos: __DUPLICATES_COUNT__ (__DUPLICATES_PCT__)\n` +
        `Errores: __ERRORS_COUNT__ (__ERRORS_PCT__)\n` +
        `Modo: __TRANSFER_MODE__`;
      navigator.clipboard.writeText(summary).then(() => {
        showToast("✓ Resumen copiado al portapapeles");
      }).catch(() => {
        showToast("Aviso: no se pudo acceder al portapapeles.");
      });
    }

    function exportEmailsToCsv() {
      if (!emails || emails.length === 0) {
        showToast("No hay correos para exportar.");
        return;
      }
      let csvContent = "\uFEFFAsunto,Remitente,Fecha,Carpeta_Origen,Destino_MAPI,Tamano_KB,Estado,PST\n";
      emails.forEach(e => {
        const row = [
          `"${(e.subject || '').replace(/"/g, '""')}"`,
          `"${(e.sender || '').replace(/"/g, '""')}"`,
          `"${(e.date || '').replace(/"/g, '""')}"`,
          `"${(e.source_folder || '').replace(/"/g, '""')}"`,
          `"${(e.dest_folder || '').replace(/"/g, '""')}"`,
          e.size_kb || 0,
          `"${(e.status || '').replace(/"/g, '""')}"`,
          `"${(e.pst_name || '').replace(/"/g, '""')}"`
        ];
        csvContent += row.join(",") + "\n";
      });

      const blob = new Blob([csvContent], { type: "text/csv;charset=utf-8;" });
      const url = URL.createObjectURL(blob);
      const link = document.createElement("a");
      link.setAttribute("href", url);
      link.setAttribute("download", `migracion_correos___DATE_STR__.csv`);
      document.body.appendChild(link);
      link.click();
      document.body.removeChild(link);
      showToast("✓ Archivo CSV descargado");
    }

    // Inicialización al cargar la página
    renderFoldersTable();
    renderEmailsTable();
    renderLogs();
  </script>
</body>
</html>"#
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::{AppState, PstItem};

    #[test]
    fn test_generate_html_report_creates_valid_file() {
        let mut state = AppState::new();
        state.progress.imported_count = 120;
        state.progress.duplicates_skipped = 15;
        state.progress.error_count = 0;
        state.discovered_psts.push(PstItem {
            path: r"C:\Correo\ArchivoTest.pst".to_string(),
            name: "ArchivoTest.pst".to_string(),
            size_mb: 250.0,
            selected: true,
        });

        let temp_dir = std::env::temp_dir();
        let test_output = temp_dir.join("test_report.html");

        let res = generate_html_report(&state, Some(test_output.clone()));
        assert!(res.is_ok(), "generate_html_report falló: {:?}", res.err());
        assert!(test_output.exists(), "El archivo HTML de prueba no fue generado");

        let content = fs::read_to_string(&test_output).expect("Error al leer archivo generado");
        assert!(content.contains("<!DOCTYPE html>"));
        assert!(content.contains("Outlook Organizer TS"));
        assert!(content.contains("120")); // Correos importados
        assert!(content.contains("Carpetas Importadas"));
        assert!(content.contains("Explorador de Correos"));
        assert!(content.contains("rawFoldersData"));
        assert!(content.contains("Auditoría de Almacenamiento MAPI: Antes vs. Después"));
        assert!(content.contains("ESTADO ANTES DE IMPORTAR"));
        assert!(content.contains("ESTADO DESPUÉS DE IMPORTAR"));
        let logs_dir = PathBuf::from("logs");
        let _ = fs::create_dir_all(&logs_dir);
        let preview_path = logs_dir.join("preview_report.html");
        let _ = generate_html_report(&state, Some(preview_path));

        let _ = fs::remove_file(test_output);
    }

    #[test]
    fn test_deserialize_real_temp_items() {
        let temp_items = std::env::temp_dir().join("outlook_organizer_items.json");
        if temp_items.exists() {
            let content = fs::read_to_string(&temp_items).unwrap();
            let trimmed = content.trim_start_matches('\u{feff}');
            match serde_json::from_str::<Vec<crate::app::ProcessedEmailItem>>(trimmed) {
                Ok(items) => {
                    println!("SUCCESS: Deserialized {} items!", items.len());
                    assert!(!items.is_empty());
                }
                Err(e) => {
                    panic!("DESERIALIZATION ERROR: {:?}", e);
                }
            }
        }
    }
}
