# DESIGN.md — Especificación de Diseño y Arquitectura de TUI

Este documento define la arquitectura visual, el diseño de la interfaz de terminal (TUI) con `ratatui` y `crossterm`, el sistema de diseño, la disposición de pantallas y la identidad de marca de **Outlook Organizer TS**.

---

## 1. Filosofía de Diseño y Experiencia de Usuario (UX)

- **Moderna, limpia y profesional**: Estética oscura con alto contraste accesible, bordes redondeados (`Rounded`), acentos visuales vibrantes (Cyan/Azul Outlook y Amber/Oro) y retroalimentación clara.
- **Flujo Guiado Paso a Paso (Wizard Pattern)**: El usuario avanza por etapas bien definidas con validaciones antes de pasar a la siguiente pantalla.
- **Seguridad en primer plano**: Advertencias visibles ante operaciones destructivas (modo Mover) y estados de parada controlada.
- **Teclas de navegación universales e intuitivas**:
  - `Tab` / `Shift+Tab`: Alternar foco entre paneles o campos.
  - `↑` / `↓` o `j` / `k`: Navegar por listas/tablas.
  - `Espacio`: Seleccionar/deseleccionar checkbox.
  - `Enter`: Confirmar selección o avanzar al siguiente paso.
  - `Esc` / `Backspace`: Retroceder al paso anterior.
  - `q` / `Ctrl+C`: Salir (o parada segura con confirmación si hay una operación activa).

---

## 2. Paleta de Colores y Sistema Visual

Basado en una paleta moderna, compatible con terminales TrueColor y degradación elegante a 256 colores:

| Rol Semántico | Color Hex / Ratatui | Aplicación |
| :--- | :--- | :--- |
| **Primary Accent** | `#00A4EF` / `Cyan` / `LightBlue` | Bordes activos, títulos destacados, selección principal |
| **Secondary Accent** | `#5B5FC7` / `Blue` / `Indexed(63)` | Acento Outlook / M365, pestañas secundarias |
| **Success** | `#107C41` / `#00CC6A` / `Green` | Pasos completados, ítems importados con éxito |
| **Warning / Caution** | `#FFB900` / `Yellow` / `LightYellow` | Advertencias de modo Mover, Throttling activo |
| **Danger / Critical** | `#D83B01` / `#E81123` / `Red` | Errores críticos, acción destructiva, cancelar |
| **Background** | `#1E1E1E` o `Reset` (terminal default) | Fondo general limpio |
| **Surface / Card** | `#252526` / `DarkGray` | Fondo de bloques y modales |
| **Text Main** | `#FFFFFF` / `White` | Texto estándar legible |
| **Text Muted** | `#8A8886` / `Gray` | Textos secundarios, atajos de teclado, metadata |
| **Brand Watermark** | `#00C7B7` / `#D4AF37` / Cyan-Gold | Marca de agua y logo corporativo *Timeless Support* |

---

## 3. Disposición General de la Pantalla (Layout Base)

La interfaz se divide verticalmente en 3 áreas principales más el pie con la marca de agua integrada:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│  ✉ OUTLOOK ORGANIZER TS  v0.1.0                   [Paso 2 de 8: Selección de PSTs]    │  <- Header (3 líneas)
├────────────────────────────────────────────────────────────────────────────────────────┤
│                                                                                        │
│                                                                                        │
│                               ÁREA DE CONTENIDO PRINCIPAL                              │
│                      (Cambia según el paso del Asistente / Wizard)                     │  <- Body (Flex/Min 15)
│                                                                                        │
│                                                                                        │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ [Tab] Cambiar Foco  [↑/↓] Navegar  [Espacio] Seleccionar  [Enter] Siguiente  [Esc] Atrás│  <- Footer (3 líneas)
│                                                                   ┌──────────────────┐ │
│                                                                   │ ⏳ TIMELESS      │ │  <- Watermark (Esquina inf. der.)
│                                                                   │    SUPPORT       │ │
└───────────────────────────────────────────────────────────────────┴──────────────────┘─┘
```

### Constraints de Layout:
```rust
Layout::default()
    .direction(Direction::Vertical)
    .constraints([
        Constraint::Length(3), // Header superior
        Constraint::Min(16),    // Pantalla activa del wizard
        Constraint::Length(4), // Footer de controles + Marca de agua
    ])
```

---

## 4. Marca de Agua y Logo Corporativo: "Timeless Support"

Ubicada fijada en la **esquina inferior derecha** del footer, con diseño elegante y discreto en dos líneas:

### Arte ASCII y Símbolo:
```text
  ⏳ TIMELESS
     SUPPORT
```
*Variante estilizada con glifo de tiempo infinito:*
```text
  ⧗ TIMELESS
    SUPPORT
```

### Estilo Visual del Watermark:
- **Línea 1 (`TIMELESS`)**: Color Cyan brillante (`#00E5FF`) o Dorado (`#FFD700`), tipografía en negrita (`Modifier::BOLD`).
- **Línea 2 (`SUPPORT`)**: Color Blanco suave (`#CCCCCC`) o Gris azulado (`#8BA8B7`), ligeramente indentado para equilibrio visual.
- **Símbolo / Icono**: Reloj de arena (`⏳` o `⧗` o `⌛`), que simboliza soporte continuo y estabilidad atemporal.
- **Posición**: Anclado en el cuadrante inferior derecho dentro del bloque del pie de página (`Alignment::Right`), sin tapar las teclas de navegación situadas a la izquierda.

---

## 5. Especificación de las Pantallas del Wizard

### Pantalla 1: Bienvenida y Perfil de Outlook
- **Propósito**: Comprobar conectividad MAPI/Outlook y elegir el perfil.
- **Componentes**:
  - Banner estilizado con título.
  - Radio buttons: `[●] Usar perfil predeterminado del sistema` / `[○] Especificar perfil manual`.
  - Campo de entrada de texto para nombre de perfil (se habilita si se marca manual).
  - Indicador de estado de Outlook (detectando si `OUTLOOK.EXE` está en ejecución).

### Pantalla 2: Origen y Escaneo de PSTs
- **Propósito**: Localizar y marcar los archivos PST a procesar.
- **Componentes**:
  - Selector de modo de escaneo:
    1. Carpeta predeterminada (`C:\Correo`) [Botón rápido].
    2. Ruta personalizada (con autocompletado básico o input de ruta).
    3. Escaneo completo de disco (muestra spinner mientras indexa).
  - Lista de PSTs encontrados con checkboxes `[x]`, tamaño en GB/MB, ruta y fecha de modificación.
  - Opciones masivas: `[a] Marcar todos`, `[n] Desmarcar todos`.

### Pantalla 3: Selección de Buzón(es) Destino
- **Propósito**: Indicar a qué buzón o buzones se transferirán los correos.
- **Componentes**:
  - Selector:
    - `[●] Buzón personal principal`.
    - `[○] Buzón compartido (Shared Mailbox) específico`.
    - `[○] Múltiples buzones compartidos`.
  - Tabla interactiva con buzones disponibles detectados en la sesión de Outlook.

### Pantalla 4: Carpetas a Importar y Modo de Transferencia
- **Propósito**: Filtrar qué tipos de correo importar y definir si se copian o mueven.
- **Componentes**:
  - Panel izquierdo (Carpetas):
    - `[x] Bandeja de entrada (Inbox)`
    - `[x] Elementos enviados (Sent Items)`
    - `[ ] Elementos eliminados (Deleted Items)`
    - `[ ] Borradores (Drafts)`
    - `[x] Carpetas personalizadas / subcarpetas`
  - Panel derecho (Modo de Transferencia):
    - `(●) Copiar correos [Recomendado]` (PST queda intacto).
    - `(○) Mover correos [Destructivo]` (Se borra del PST tras verificar éxito en destino).
    - *Alerta visual en ámbar/rojo si se selecciona "Mover"*.

### Pantalla 5: Enrutamiento y Agrupación Temporal
- **Propósito**: Decidir cómo se crearán las carpetas de archivo en destino.
- **Componentes**:
  - Switch: `[x] Activar Enrutamiento Inteligente por Fechas`.
  - Nivel de granularidad:
    - `(●) Por Años (ej. Inbox/2024)`
    - `(○) Por Años y Meses (ej. Inbox/2024/05-Mayo)`
  - Rango:
    - `(●) Todos los años/meses disponibles`
    - `(○) Filtrar un año específico` (Selector numérico: 2023, 2024, etc.)
    - `(○) Filtrar un mes específico`.

### Pantalla 6: Deduplicación y Revisión Profunda
- **Propósito**: Prevenir correos repetidos y ahorrar cuota de buzón.
- **Componentes**:
  - `[x] Omitir correos duplicados automáticamente`.
  - Criterio de duplicados: `[●] Message-ID / SearchKey` | `[○] Clave compuesta (Asunto + Remitente + Fecha)`.
  - `[x] Revisión profunda (Deep Scan)`:
    - *Nota*: "Indexa recursivamente subcarpetas existentes. Más lento pero detecta correos ya clasificados manualmente".

### Pantalla 7: Filtros de Fecha y Throttling Adaptativo
- **Propósito**: Afinar rendimiento y evitar bloqueos en la nube.
- **Componentes**:
  - Filtro de fecha: `Desde: [ AAAA-MM-DD ]` hasta `[ AAAA-MM-DD ]` (o en blanco para todo).
  - `[x] Throttling adaptativo (Anti-Throttling)`:
    - Detecta latencia y estados 429/Backoff de Exchange Online / Microsoft 365, regulando dinámicamente las llamadas COM.

### Pantalla 8: Resumen Prevuelo (Pre-flight Summary)
- **Propósito**: Revisión integral antes de iniciar cualquier cambio.
- **Componentes**:
  - Tabla de configuración consolidada (Origen, Destino, Modo Copiar/Mover, Deduplicación, etc.).
  - Total de PSTs seleccionados y volumen estimado.
  - Botones de acción destacados:
    - `[ INICIAR OPERACIÓN ]` (Color Verde brillante, requiere confirmación explícita con Enter o tecla dedicada).
    - `[ CANCELAR / SALIR ]` (Color Rojo).

---

## 6. Pantalla de Ejecución y Progreso en Vivo

Cuando la operación está en curso, la pantalla cambia a un **Dashboard de Monitoreo en Tiempo Real**:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│  ✉ PROCESANDO PSTs...                              [Throttling: Normal (12 ms/req)]    │
├────────────────────────────────────────────────────────────────────────────────────────┤
│ PST Actual (1/3): Archivo_2023.pst                                                     │
│ [████████████████████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 48% (1,450 / 3,020)  │
│                                                                                        │
│ Progreso Global:                                                                       │
│ [████████████████░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░░] 22% (Total ítems)    │
├──────────────────────────────────────┬─────────────────────────────────────────────────┤
│ Métricas en Vivo                     │ Registro de Actividad en Tiempo Real            │
│ • Importados con éxito:  1,412       │ [14:02:10] OK: 'Reunión Proyecto' -> Inbox/2023 │
│ • Duplicados omitidos:      38       │ [14:02:11] SKIP: Duplicate Message-ID <a1@b.c>  │
│ • Errores de lectura:        0       │ [14:02:12] RETRY: Latencia alta detectada (+2s) │
│ • Velocidad:         18 msgs/s       │ [14:02:13] OK: 'Factura 9481' -> Inbox/2023     │
├──────────────────────────────────────┴─────────────────────────────────────────────────┤
│ [Esc / Ctrl+C] Detener con Seguridad (Desmonta PST sin corromper)        ⏳ TIMELESS   │
│                                                                             SUPPORT    │
└────────────────────────────────────────────────────────────────────────────────────────┘
```

### Manejo de Parada Segura (Graceful Cancel):
- Si el usuario presiona `Esc` o `Ctrl+C`:
  - El título cambia a `[PARANDO CON SEGURIDAD... ESPERE A DESMONTAR PST]`.
  - El color de borde cambia a **Ámbar parpadeante / Amarillo**.
  - La TUI espera a que el worker reporte `PST_DISMOUNTED` antes de finalizar.

### 6.1. Especificación de Barras de Progreso Funcionales (Ratatui `Gauge` / `LineGauge`)
Las barras de progreso no son meramente cosméticas; están enlazadas a un canal asíncrono con telemetría en tiempo real:

1. **Barra 1: Progreso del PST Actual (`Gauge`)**:
   - **Widget**: `ratatui::widgets::Gauge`.
   - **Valor dinámico**: Porcentaje exacto calculado como `(items_procesados_pst as f64 / total_items_pst as f64) * 100.0`.
   - **Label dinámico**: Formato legible `"{percent}% ({items_pst} / {total_pst} correos) - Archivo: {pst_name}"`.
   - **Estilo visual**: Gradiente o acento Cyan `#00A4EF` con fondo `DarkGray` y bordes suaves.

2. **Barra 2: Progreso Global de la Operación (`Gauge` o `LineGauge`)**:
   - **Widget**: `ratatui::widgets::Gauge`.
   - **Valor dinámico**: `(items_totales_procesados as f64 / gran_total_items as f64) * 100.0` o ponderado por volumen en MB/GB si el conteo previo no está disponible de inmediato.
   - **Label dinámico**: Formato `"{global_percent}% - PST {current_pst_idx}/{total_psts} ({elapsed_time} transcurrido / ~{eta} restante)"`.
   - **Estilo visual**: Verde éxito `#00CC6A` o Azul Outlook `#5B5FC7`.

3. **Mecanismo de Actualización y Sincronización (Rust <-> PowerShell)**:
   - El script de PowerShell emite eventos periódicos por `stdout` en formato JSON Lines (cada *N* mensajes o cada 100 ms para no saturar la terminal):
     ```json
     {"type": "progress", "pst_index": 1, "pst_total": 3, "pst_name": "Inbox.pst", "item_current": 1450, "item_total": 3020, "speed_mps": 18.2, "status": "transferring"}
     ```
   - En Rust, un hilo/tarea asíncrona lee las líneas del pipe, actualiza de inmediato el struct `ProgressState` de la aplicación y dispara un re-render del frame en `ratatui`.
   - **Cálculo de ETA y Velocidad en vivo**: Media móvil de correos transferidos por segundo para proyectar el tiempo estimado de finalización con precisión.

---

## 7. Pantalla Final: Reportes y Cierre

1. **Tarjeta de Resumen Final**:
   - Estado: `Completado exitosamente` o `Interrumpido con seguridad por usuario`.
   - Métricas finales: Total procesados, duplicados omitidos, tiempo total transcurrido.
2. **Registro JSON**:
   - Notificación: *"Registro JSON guardado en: .\logs\2026-09-15\run_14-30-00.json"* (solo en versión `.exe`).
3. **Generación de Informe HTML**:
   - Cuadro modal interactivo:
     - `¿Desea generar un informe interactivo HTML? [S/N]`
     - Ruta: `[●] Ruta predeterminada` / `[○] Ruta personalizada: [ ___________ ]`
   - Al pulsar generar: crea una página web interactiva, moderna, con gráficos ligeros en CSS/SVG, tabla de eventos y métricas de la empresa.

---

## 8. Arquitectura del Código TUI en Rust

```
src/
├── main.rs                 # Inicialización de terminal, loop principal y teardown
├── app.rs                  # Máquina de estados (AppState, WizardStep, Métricas)
├── ui/
│   ├── mod.rs              # Enrutador de renderizado principal
│   ├── theme.rs            # Paleta de colores, estilos y constantes visuales
│   ├── header.rs           # Header con título y paso actual
│   ├── footer.rs           # Barra de atajos y marca de agua Timeless Support
│   └── screens/
│       ├── welcome.rs      # Paso 1
│       ├── pst_source.rs   # Paso 2
│       ├── mailbox.rs      # Paso 3
│       ├── folders_mode.rs # Paso 4
│       ├── routing.rs      # Paso 5
│       ├── deduplication.rs# Paso 6
│       ├── filters.rs      # Paso 7
│       ├── summary.rs      # Paso 8
│       ├── execution.rs    # Pantalla de monitoreo en tiempo real
│       └── completion.rs   # Pantalla de reporte final / HTML
├── backend/
│   ├── mod.rs              # Puente asíncrono Rust <-> PowerShell
│   ├── runner.rs           # Spawner de subproceso PowerShell con pipes
│   └── messages.rs         # Protocolo de telemetría (JSON lines en stdout)
└── report/
    ├── json.rs             # Generador de auditoría JSON
    └── html.rs             # Generador de plantilla HTML visual
```
