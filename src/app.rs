use std::{
    collections::VecDeque,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Welcome,
    FileExplorer,
    PstSource,
    Mailbox,
    FoldersMode,
    Routing,
    Deduplication,
    Filters,
    Summary,
    Execution,
    Completion,
    PstDetailView,
}

impl WizardStep {
    pub fn title(&self) -> &'static str {
        match self {
            WizardStep::Welcome => "Menú Principal",
            WizardStep::FileExplorer => "Explorador de Archivos PST",
            WizardStep::PstSource => "Selección de Archivos PST",
            WizardStep::Mailbox => "Selección de Buzón Destino",
            WizardStep::FoldersMode => "Carpetas y Modo de Transferencia",
            WizardStep::Routing => "Enrutamiento y Agrupación Temporal",
            WizardStep::Deduplication => "Deduplicación y Revisión Profunda",
            WizardStep::Filters => "Filtros de Fecha y Throttling",
            WizardStep::Summary => "Resumen Pre-Vuelo y Confirmación",
            WizardStep::Execution => "Procesando en Tiempo Real",
            WizardStep::Completion => "Operación Finalizada y Reportes",
            WizardStep::PstDetailView => "Detalle Analítico del Archivo PST",
        }
    }

    pub fn index(&self) -> usize {
        match self {
            WizardStep::Welcome => 0,
            WizardStep::FileExplorer => 1,
            WizardStep::PstSource => 1,
            WizardStep::PstDetailView => 1,
            WizardStep::Mailbox => 2,
            WizardStep::FoldersMode => 3,
            WizardStep::Routing => 4,
            WizardStep::Deduplication => 5,
            WizardStep::Filters => 6,
            WizardStep::Summary => 7,
            WizardStep::Execution => 7,
            WizardStep::Completion => 7,
        }
    }
}

#[derive(Debug, Clone)]
pub struct PstItem {
    pub path: String,
    pub name: String,
    pub size_mb: f64,
    pub selected: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PstFolderDetail {
    pub name: String,
    #[serde(default)]
    pub count: usize,
    #[serde(default)]
    pub path: String,
    #[serde(default)]
    pub parent_path: Option<String>,
    #[serde(default)]
    pub size_mb: f64,
    #[serde(default)]
    pub has_children: bool,
    #[serde(default)]
    pub years: Vec<u32>,
    #[serde(default)]
    pub year_months: std::collections::BTreeMap<String, Vec<u32>>,
    #[serde(default)]
    pub counts_by_year: std::collections::BTreeMap<String, usize>,
    #[serde(default)]
    pub counts_by_month: std::collections::BTreeMap<String, usize>,
    #[serde(default)]
    pub sizes_by_year_mb: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub sizes_by_month_mb: std::collections::BTreeMap<String, f64>,
}

impl Default for PstFolderDetail {
    fn default() -> Self {
        Self {
            name: String::new(),
            count: 0,
            path: String::new(),
            parent_path: None,
            size_mb: 0.0,
            has_children: false,
            years: Vec::new(),
            year_months: std::collections::BTreeMap::new(),
            counts_by_year: std::collections::BTreeMap::new(),
            counts_by_month: std::collections::BTreeMap::new(),
            sizes_by_year_mb: std::collections::BTreeMap::new(),
            sizes_by_month_mb: std::collections::BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct PstDetail {
    pub file_name: String,
    pub file_path: String,
    pub size_mb: f64,
    #[serde(default)]
    pub total_items: usize,
    pub last_email_date: Option<String>,
    pub first_email_date: Option<String>,
    pub folders: Vec<PstFolderDetail>,
    pub years: Vec<u32>,
    pub year_months: std::collections::BTreeMap<String, Vec<u32>>,
    #[serde(default)]
    pub counts_by_year: std::collections::BTreeMap<String, usize>,
    #[serde(default)]
    pub counts_by_month: std::collections::BTreeMap<String, usize>,
    #[serde(default)]
    pub sizes_by_year_mb: std::collections::BTreeMap<String, f64>,
    #[serde(default)]
    pub sizes_by_month_mb: std::collections::BTreeMap<String, f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PstFolderItemType {
    ParentDir,
    Folder,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PstFolderEntry {
    pub name: String,
    pub path: String,
    pub parent_path: Option<String>,
    pub count: usize,
    pub size_mb: f64,
    pub has_children: bool,
    pub item_type: PstFolderItemType,
}

#[derive(Debug, Clone, Default)]
pub struct PstFolderExplorerState {
    pub current_parent: Option<String>, // None = raíz de carpetas del PST
    pub selected_idx: usize,
    pub checked_folder_path: Option<String>, // Carpeta seleccionada con Espacio
}

impl PstFolderExplorerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.current_parent = None;
        self.selected_idx = 0;
        self.checked_folder_path = None;
    }

    /// Obtiene las entradas a listar en el nivel actual
    pub fn current_entries(&self, detail: &PstDetail) -> Vec<PstFolderEntry> {
        let mut entries = Vec::new();

        // Si estamos dentro de una subcarpeta, la primera opción es '..' para subir de nivel
        if let Some(ref parent) = self.current_parent {
            entries.push(PstFolderEntry {
                name: ".. (Subir de nivel)".to_string(),
                path: parent.clone(),
                parent_path: None,
                count: 0,
                size_mb: 0.0,
                has_children: false,
                item_type: PstFolderItemType::ParentDir,
            });
        }

        // Buscar las carpetas cuyo parent_path coincida con current_parent
        for f in &detail.folders {
            let is_match = match (&self.current_parent, &f.parent_path) {
                (None, None) => true,
                (None, Some(p)) if p.is_empty() => true,
                (Some(curr), Some(p)) => curr == p,
                _ => false,
            };

            if is_match {
                entries.push(PstFolderEntry {
                    name: f.name.clone(),
                    path: if f.path.is_empty() { f.name.clone() } else { f.path.clone() },
                    parent_path: f.parent_path.clone(),
                    count: f.count,
                    size_mb: f.size_mb,
                    has_children: f.has_children,
                    item_type: PstFolderItemType::Folder,
                });
            }
        }

        entries
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self, max_entries: usize) {
        if max_entries > 0 && self.selected_idx + 1 < max_entries {
            self.selected_idx += 1;
        }
    }

    pub fn navigate_into(&mut self, detail: &PstDetail) -> bool {
        let entries = self.current_entries(detail);
        if let Some(entry) = entries.get(self.selected_idx) {
            match entry.item_type {
                PstFolderItemType::ParentDir => {
                    self.navigate_up(detail);
                    return true;
                }
                PstFolderItemType::Folder => {
                    if entry.has_children {
                        self.current_parent = Some(entry.path.clone());
                        self.selected_idx = 0;
                        return true;
                    }
                }
            }
        }
        false
    }

    pub fn navigate_up(&mut self, detail: &PstDetail) -> bool {
        if let Some(ref current) = self.current_parent {
            // Buscar cuál es el padre del current_parent
            let parent_of_current = detail
                .folders
                .iter()
                .find(|f| {
                    let f_path = if f.path.is_empty() { &f.name } else { &f.path };
                    f_path == current
                })
                .and_then(|f| f.parent_path.clone())
                .filter(|p| !p.is_empty());

            self.current_parent = parent_of_current;
            self.selected_idx = 0;
            true
        } else {
            false
        }
    }

    /// Alterna la selección con la tecla Espacio
    pub fn toggle_select(&mut self, detail: &PstDetail) {
        let entries = self.current_entries(detail);
        if let Some(entry) = entries.get(self.selected_idx)
            && entry.item_type == PstFolderItemType::Folder
        {
            if self.checked_folder_path.as_deref() == Some(&entry.path) {
                // Si ya estaba seleccionada, deseleccionar para volver al resumen general
                self.checked_folder_path = None;
            } else {
                self.checked_folder_path = Some(entry.path.clone());
            }
        }
    }

    /// Obtiene los detalles de la carpeta seleccionada actualmente (si hay una)
    pub fn get_selected_folder_stats<'a>(&self, detail: &'a PstDetail) -> Option<&'a PstFolderDetail> {
        if let Some(ref path) = self.checked_folder_path {
            detail.folders.iter().find(|f| {
                let f_path = if f.path.is_empty() { &f.name } else { &f.path };
                f_path == path
            })
        } else {
            None
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum PstDetailModalState {
    Closed,
    Loading {
        pst_path: String,
        pst_name: String,
        current_folder: Option<String>,
        scanned_items: usize,
    },
    Loaded(Box<PstDetail>),
    Error { pst_name: String, message: String },
}

#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailboxItem {
    pub display_name: String,
    pub store_type: String,
    pub size_display: String,
    #[serde(default)]
    pub file_path: Option<String>,
    #[serde(default)]
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExplorerItemType {
    ParentDir,
    Drive,
    Directory,
    PstFile,
}

#[derive(Debug, Clone)]
pub struct ExplorerEntry {
    pub name: String,
    pub path: PathBuf,
    pub item_type: ExplorerItemType,
    pub size_mb: Option<f64>,
    pub selected: bool,
}

#[derive(Debug, Clone)]
pub struct FileExplorerState {
    pub current_path: PathBuf,
    pub entries: Vec<ExplorerEntry>,
    pub selected_idx: usize,
    pub is_drives_view: bool,
    pub warning_notice: Option<String>,
}

impl FileExplorerState {
    pub fn new(initial_path: PathBuf) -> Self {
        let mut state = Self {
            current_path: initial_path,
            entries: Vec::new(),
            selected_idx: 0,
            is_drives_view: false,
            warning_notice: None,
        };
        state.refresh();
        state
    }

    pub fn refresh(&mut self) {
        if self.is_drives_view || self.current_path.as_os_str().is_empty() || !self.current_path.exists() {
            self.entries = list_windows_drives();
            self.is_drives_view = true;
            self.selected_idx = 0;
        } else {
            let (entries, drives) = read_directory(&self.current_path);
            self.entries = entries;
            self.is_drives_view = drives;
            if self.selected_idx >= self.entries.len() {
                self.selected_idx = 0;
            }
        }
    }

    pub fn navigate_into_selected(&mut self) -> bool {
        if let Some(entry) = self.entries.get(self.selected_idx) {
            match entry.item_type {
                ExplorerItemType::ParentDir => {
                    self.navigate_up();
                    true
                }
                ExplorerItemType::Drive | ExplorerItemType::Directory => {
                    self.current_path = entry.path.clone();
                    self.is_drives_view = false;
                    self.selected_idx = 0;
                    self.refresh();
                    true
                }
                ExplorerItemType::PstFile => {
                    if let Some(e) = self.entries.get_mut(self.selected_idx) {
                        e.selected = !e.selected;
                    }
                    false
                }
            }
        } else {
            false
        }
    }

    pub fn navigate_up(&mut self) {
        if self.is_drives_view {
            return;
        }

        if let Some(parent) = self.current_path.parent() {
            if parent.as_os_str().is_empty() || parent == self.current_path {
                self.is_drives_view = true;
                self.current_path = PathBuf::new();
                self.refresh();
            } else {
                self.current_path = parent.to_path_buf();
                self.refresh();
            }
        } else {
            self.is_drives_view = true;
            self.current_path = PathBuf::new();
            self.refresh();
        }
    }

    pub fn collect_selected_psts(&self) -> Vec<PstItem> {
        let mut psts = Vec::new();
        for entry in &self.entries {
            if entry.item_type == ExplorerItemType::PstFile && entry.selected {
                psts.push(PstItem {
                    path: entry.path.to_string_lossy().to_string(),
                    name: entry.name.clone(),
                    size_mb: entry.size_mb.unwrap_or(0.0),
                    selected: true,
                });
            }
        }

        if psts.is_empty() && !self.is_drives_view && self.current_path.exists() {
            psts = scan_folder_for_psts(&self.current_path);
        }

        psts
    }
}

pub fn list_windows_drives() -> Vec<ExplorerEntry> {
    let mut drives = Vec::new();
    for letter in b'C'..=b'Z' {
        let drive_str = format!("{}:\\", letter as char);
        let path = PathBuf::from(&drive_str);
        if path.exists() {
            drives.push(ExplorerEntry {
                name: format!("Unidad de Disco ({}:)", letter as char),
                path,
                item_type: ExplorerItemType::Drive,
                size_mb: None,
                selected: false,
            });
        }
    }
    drives
}

pub fn read_directory(path: &Path) -> (Vec<ExplorerEntry>, bool) {
    let read_res = std::fs::read_dir(path);
    if read_res.is_err() {
        return (list_windows_drives(), true);
    }

    let mut entries = Vec::new();
    entries.push(ExplorerEntry {
        name: ".. (Subir de nivel)".to_string(),
        path: path.parent().map(|p| p.to_path_buf()).unwrap_or_else(|| PathBuf::from("")),
        item_type: ExplorerItemType::ParentDir,
        size_mb: None,
        selected: false,
    });

    let mut dir_entries = Vec::new();
    let mut file_entries = Vec::new();

    if let Ok(read_dir) = read_res {
        for entry in read_dir.flatten() {
            let file_type = match entry.file_type() {
                Ok(ft) => ft,
                Err(_) => continue,
            };
            let file_name = entry.file_name().to_string_lossy().to_string();

            if file_type.is_dir() {
                dir_entries.push(ExplorerEntry {
                    name: file_name,
                    path: entry.path(),
                    item_type: ExplorerItemType::Directory,
                    size_mb: None,
                    selected: false,
                });
            } else if file_type.is_file() {
                let is_pst = entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("pst"))
                    .unwrap_or(false);

                if is_pst {
                    let size_mb = entry.metadata().ok().map(|m| m.len() as f64 / (1024.0 * 1024.0)).unwrap_or(0.0);
                    file_entries.push(ExplorerEntry {
                        name: file_name,
                        path: entry.path(),
                        item_type: ExplorerItemType::PstFile,
                        size_mb: Some(size_mb),
                        selected: true,
                    });
                }
            }
        }
    }

    dir_entries.sort_by_key(|a| a.name.to_lowercase());
    file_entries.sort_by_key(|a| a.name.to_lowercase());

    entries.extend(dir_entries);
    entries.extend(file_entries);

    (entries, false)
}

pub fn scan_folder_for_psts(folder: &Path) -> Vec<PstItem> {
    let mut items = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(folder) {
        for entry in read_dir.flatten() {
            if let Ok(ft) = entry.file_type()
                && ft.is_file() {
                let is_pst = entry.path().extension()
                    .and_then(|ext| ext.to_str())
                    .map(|ext| ext.eq_ignore_ascii_case("pst"))
                    .unwrap_or(false);

                if is_pst {
                    let name = entry.file_name().to_string_lossy().to_string();
                    let size_mb = entry.metadata().ok().map(|m| m.len() as f64 / (1024.0 * 1024.0)).unwrap_or(0.0);
                    items.push(PstItem {
                        path: entry.path().to_string_lossy().to_string(),
                        name,
                        size_mb,
                        selected: true,
                    });
                }
            }
        }
    }
    items.sort_by_key(|a| a.name.to_lowercase());
    items
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RoutingGranularity {
    #[default]
    Mirror,
    Years,
    YearsAndMonths,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingModal {
    None,
    Criterion,     // Criterio de Enrutamiento (Años vs Meses)
    YearScope,     // Alcance de años (Todos los años vs Año específico)
    SpecificYear,  // Año específico (Input numérico)
    MonthScope,    // Alcance de meses (Todos los meses [contextual] vs Mes específico)
    SpecificMonth, // Mes específico (Selector horizontal)
}

#[derive(Debug, Clone)]
pub struct ProgressState {
    pub current_pst_name: String,
    pub current_pst_items: u64,
    pub current_pst_total: u64,
    pub global_items_processed: u64,
    pub global_items_total: u64,
    #[allow(dead_code)]
    pub current_pst_idx: usize,
    #[allow(dead_code)]
    pub total_psts: usize,
    pub speed_mps: f64,
    pub eta_seconds: u64,
    pub imported_count: u64,
    pub duplicates_skipped: u64,
    pub error_count: u64,
    pub throttling_active: bool,
    pub graceful_cancelling: bool,
}

impl Default for ProgressState {
    fn default() -> Self {
        Self {
            current_pst_name: String::new(),
            current_pst_items: 0,
            current_pst_total: 100,
            global_items_processed: 0,
            global_items_total: 100,
            current_pst_idx: 0,
            total_psts: 0,
            speed_mps: 0.0,
            eta_seconds: 0,
            imported_count: 0,
            duplicates_skipped: 0,
            error_count: 0,
            throttling_active: false,
            graceful_cancelling: false,
        }
    }
}

/// Estado global reactivo de la aplicación
pub struct AppState {
    pub step: WizardStep,
    pub should_quit: bool,
    pub welcome_menu_idx: usize,
    
    // Configuración Paso 1: Perfil
    pub use_default_profile: bool,
    pub custom_profile_name: String,
    pub is_editing_profile: bool,

    // Configuración Paso 2: Fuentes PST
    pub pst_scan_path: String,
    pub discovered_psts: Vec<PstItem>,
    pub selected_pst_table_idx: usize,

    // Configuración Paso 3: Buzones Destino MAPI
    pub discovered_mailboxes: Vec<MailboxItem>,
    pub selected_mailbox_idx: usize,
    pub is_loading_mailboxes: bool,
    pub mailbox_warning_notice: Option<String>,

    // Configuración Paso 4: Carpetas y Modo
    pub include_inbox: bool,
    pub include_sent: bool,
    pub include_deleted: bool,
    pub include_custom_folders: bool,
    pub transfer_mode: TransferMode,

    // Configuración Paso 5: Enrutamiento
    pub routing_enabled: bool,
    pub routing_granularity: RoutingGranularity,
    pub specific_year: Option<u32>,
    pub specific_month: Option<u32>,
    pub routing_all_years: bool,
    pub routing_all_months: bool,
    pub active_routing_modal: RoutingModal,
    pub routing_modal_criterion_idx: usize,
    pub routing_modal_year_scope_idx: usize,
    pub routing_modal_month_scope_idx: usize,
    pub routing_input_year: String,
    pub routing_input_month: u32,

    // Configuración Paso 6: Deduplicación
    pub deduplication_enabled: bool,
    pub deep_scan_enabled: bool,

    // Configuración Paso 7: Filtros y Throttling
    pub adaptive_throttling_enabled: bool,

    // Explorador de archivos interactivo
    pub explorer: FileExplorerState,

    // Métricas y progreso en tiempo real
    pub progress: ProgressState,
    pub activity_log: VecDeque<String>, // Buffer circular limitado (max 300)

    // Modal y vista completa de detalle de PST
    pub previous_step_before_detail: WizardStep,
    pub pst_folder_explorer: PstFolderExplorerState,
    pub pst_detail_modal: PstDetailModalState,
    pub pst_details_cache: std::collections::HashMap<String, PstDetail>,
}

pub fn default_fallback_mailboxes() -> Vec<MailboxItem> {
    vec![
        MailboxItem {
            display_name: "buzon.personal@empresa.com".to_string(),
            store_type: "ExchangeOnline".to_string(),
            size_display: "0 Bytes".to_string(),
            file_path: None,
            selected: true,
        },
    ]
}

impl AppState {
    pub fn new() -> Self {
        let default_correo = PathBuf::from(r"C:\Correo");
        let initial_psts = if default_correo.exists() {
            scan_folder_for_psts(&default_correo)
        } else {
            Vec::new()
        };
        let explorer = FileExplorerState::new(default_correo);

        Self {
            step: WizardStep::Welcome,
            should_quit: false,
            welcome_menu_idx: 0,
            use_default_profile: true,
            custom_profile_name: String::new(),
            is_editing_profile: false,
            pst_scan_path: r"C:\Correo".to_string(),
            discovered_psts: initial_psts,
            selected_pst_table_idx: 0,
            discovered_mailboxes: default_fallback_mailboxes(),
            selected_mailbox_idx: 0,
            is_loading_mailboxes: false,
            mailbox_warning_notice: None,
            include_inbox: true,
            include_sent: true,
            include_deleted: false,
            include_custom_folders: true,
            transfer_mode: TransferMode::Copy,
            routing_enabled: true,
            routing_granularity: RoutingGranularity::Mirror,
            specific_year: None,
            specific_month: None,
            routing_all_years: true,
            routing_all_months: true,
            active_routing_modal: RoutingModal::None,
            routing_modal_criterion_idx: 0,
            routing_modal_year_scope_idx: 0,
            routing_modal_month_scope_idx: 0,
            routing_input_year: "2024".to_string(),
            routing_input_month: 1,
            deduplication_enabled: true,
            deep_scan_enabled: true,
            adaptive_throttling_enabled: true,
            explorer,
            progress: ProgressState::default(),
            activity_log: VecDeque::with_capacity(300),
            previous_step_before_detail: WizardStep::PstSource,
            pst_folder_explorer: PstFolderExplorerState::new(),
            pst_detail_modal: PstDetailModalState::Closed,
            pst_details_cache: std::collections::HashMap::new(),
        }
    }

    pub fn selected_mailboxes(&self) -> Vec<&MailboxItem> {
        self.discovered_mailboxes.iter().filter(|m| m.selected).collect()
    }

    pub fn selected_mailboxes_display(&self) -> String {
        let selected = self.selected_mailboxes();
        if selected.is_empty() {
            "Ninguno seleccionado".to_string()
        } else if selected.len() == 1 {
            selected[0].display_name.clone()
        } else {
            format!("{} buzones seleccionados", selected.len())
        }
    }

    pub fn log_event(&mut self, event: String) {
        if self.activity_log.len() >= 300 {
            self.activity_log.pop_front();
        }
        self.activity_log.push_back(event);
    }

    pub fn open_pst_detail(&mut self, path: String, name: String) -> bool {
        if self.step != WizardStep::PstDetailView {
            self.previous_step_before_detail = self.step;
        }
        self.pst_folder_explorer.reset();
        self.step = WizardStep::PstDetailView;
        if let Some(cached) = self.pst_details_cache.get(&path) {
            self.pst_detail_modal = PstDetailModalState::Loaded(Box::new(cached.clone()));
            false
        } else {
            self.pst_detail_modal = PstDetailModalState::Loading {
                pst_path: path,
                pst_name: name,
                current_folder: None,
                scanned_items: 0,
            };
            true
        }
    }

    pub fn close_pst_detail(&mut self) {
        self.pst_detail_modal = PstDetailModalState::Closed;
        self.pst_folder_explorer.reset();
        self.step = self.previous_step_before_detail;
    }

    pub fn next_step(&mut self) {
        self.step = match self.step {
            WizardStep::Welcome => WizardStep::PstSource,
            WizardStep::FileExplorer => WizardStep::PstSource,
            WizardStep::PstSource => WizardStep::Mailbox,
            WizardStep::PstDetailView => self.previous_step_before_detail,
            WizardStep::Mailbox => WizardStep::FoldersMode,
            WizardStep::FoldersMode => {
                self.active_routing_modal = RoutingModal::Criterion;
                self.routing_modal_criterion_idx = match self.routing_granularity {
                    RoutingGranularity::Mirror => 0,
                    RoutingGranularity::Years => 1,
                    RoutingGranularity::YearsAndMonths => 2,
                };
                self.routing_modal_year_scope_idx = 0;
                self.routing_modal_month_scope_idx = 0;
                WizardStep::Routing
            }
            WizardStep::Routing => {
                self.active_routing_modal = RoutingModal::None;
                WizardStep::Deduplication
            }
            WizardStep::Deduplication => WizardStep::Filters,
            WizardStep::Filters => WizardStep::Summary,
            WizardStep::Summary => WizardStep::Execution,
            WizardStep::Execution => WizardStep::Completion,
            WizardStep::Completion => WizardStep::Completion,
        };
    }

    pub fn prev_step(&mut self) {
        self.step = match self.step {
            WizardStep::Welcome => WizardStep::Welcome,
            WizardStep::FileExplorer => WizardStep::Welcome,
            WizardStep::PstSource => WizardStep::Welcome,
            WizardStep::PstDetailView => self.previous_step_before_detail,
            WizardStep::Mailbox => WizardStep::PstSource,
            WizardStep::FoldersMode => WizardStep::Mailbox,
            WizardStep::Routing => {
                self.active_routing_modal = RoutingModal::None;
                WizardStep::FoldersMode
            }
            WizardStep::Deduplication => {
                self.active_routing_modal = RoutingModal::None;
                WizardStep::Routing
            }
            WizardStep::Filters => WizardStep::Deduplication,
            WizardStep::Summary => WizardStep::Filters,
            WizardStep::Execution => WizardStep::Summary,
            WizardStep::Completion => WizardStep::Summary,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use std::io::Write;

    #[test]
    fn test_list_windows_drives() {
        let drives = list_windows_drives();
        assert!(!drives.is_empty(), "Should detect at least one Windows drive");
        let has_c = drives.iter().any(|d| d.path == std::path::Path::new(r"C:\"));
        assert!(has_c, "Drive C:\\ should be present");
    }

    #[test]
    fn test_read_directory_and_pst_discovery() {
        let temp_dir = std::env::temp_dir().join("outlook_organizer_ts_test");
        let _ = std::fs::remove_dir_all(&temp_dir);
        std::fs::create_dir_all(&temp_dir).unwrap();

        let sub_dir = temp_dir.join("subfolder");
        std::fs::create_dir_all(&sub_dir).unwrap();

        let pst_file = temp_dir.join("test_archive.pst");
        {
            let mut f = File::create(&pst_file).unwrap();
            f.write_all(&vec![0u8; 1024 * 1024]).unwrap(); // 1MB file
            f.sync_all().unwrap();
        }

        let txt_file = temp_dir.join("notes.txt");
        {
            let mut f2 = File::create(&txt_file).unwrap();
            f2.write_all(b"sample").unwrap();
            f2.sync_all().unwrap();
        }

        let (entries, is_drives) = read_directory(&temp_dir);
        assert!(!is_drives);
        // entries: .. (Parent), subfolder (Dir), test_archive.pst (PstFile). notes.txt is ignored.
        assert!(entries.iter().any(|e| e.name.contains("..")));
        assert!(entries.iter().any(|e| e.name == "subfolder" && e.item_type == ExplorerItemType::Directory));
        let pst_entry = entries.iter().find(|e| e.name == "test_archive.pst");
        assert!(pst_entry.is_some());
        assert_eq!(pst_entry.unwrap().item_type, ExplorerItemType::PstFile);
        assert!(pst_entry.unwrap().size_mb.unwrap() >= 0.9);

        let psts = scan_folder_for_psts(&temp_dir);
        assert_eq!(psts.len(), 1);
        assert_eq!(psts[0].name, "test_archive.pst");

        let explorer = FileExplorerState::new(temp_dir.clone());
        let collected = explorer.collect_selected_psts();
        assert_eq!(collected.len(), 1);

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_wizard_step_transitions() {
        let mut state = AppState::new();
        assert_eq!(state.step, WizardStep::Welcome);
        state.next_step();
        assert_eq!(state.step, WizardStep::PstSource);
        state.prev_step();
        assert_eq!(state.step, WizardStep::Welcome);
    }

    #[test]
    fn test_mailbox_selection() {
        let mut state = AppState::new();
        assert_eq!(state.selected_mailboxes().len(), 1);
        assert_eq!(state.selected_mailboxes_display(), "buzon.personal@empresa.com");

        state.discovered_mailboxes.push(MailboxItem {
            display_name: "compartido@empresa.com".to_string(),
            store_type: "SharedMailbox".to_string(),
            size_display: "500 MB".to_string(),
            file_path: None,
            selected: true,
        });

        assert_eq!(state.selected_mailboxes().len(), 2);
        assert_eq!(state.selected_mailboxes_display(), "2 buzones seleccionados");

        state.discovered_mailboxes[0].selected = false;
        assert_eq!(state.selected_mailboxes().len(), 1);
        assert_eq!(state.selected_mailboxes_display(), "compartido@empresa.com");

        state.discovered_mailboxes[1].selected = false;
        assert_eq!(state.selected_mailboxes().len(), 0);
        assert_eq!(state.selected_mailboxes_display(), "Ninguno seleccionado");
    }

    #[test]
    fn test_routing_default_all_years_all_months() {
        let mut state = AppState::new();
        state.step = WizardStep::FoldersMode;
        state.next_step();
        assert_eq!(state.step, WizardStep::Routing);
        assert_eq!(state.active_routing_modal, RoutingModal::Criterion);

        // Seleccionar granularidad de Meses (Años y Meses)
        state.routing_modal_criterion_idx = 1;
        state.routing_granularity = RoutingGranularity::YearsAndMonths;
        state.active_routing_modal = RoutingModal::YearScope;

        // Año por defecto: Todos los años (índice 0)
        state.routing_modal_year_scope_idx = 0;
        state.specific_year = None;
        state.routing_all_years = true;
        state.active_routing_modal = RoutingModal::MonthScope;

        // Mes por defecto: Todos los meses (índice 0) -> Todos los meses de todos los años
        state.routing_modal_month_scope_idx = 0;
        state.specific_month = None;
        state.routing_all_months = true;
        state.active_routing_modal = RoutingModal::None;

        assert_eq!(state.specific_year, None, "Debe ser None para abarcar todos los años por defecto");
        assert_eq!(state.specific_month, None, "Debe ser None para abarcar todos los meses por defecto");
        assert!(state.routing_all_years);
        assert!(state.routing_all_months);
    }

    #[test]
    fn test_routing_specific_year_all_months() {
        let mut state = AppState::new();
        state.routing_granularity = RoutingGranularity::YearsAndMonths;

        // Elegir Año específico
        state.active_routing_modal = RoutingModal::YearScope;
        state.routing_modal_year_scope_idx = 1;
        state.active_routing_modal = RoutingModal::SpecificYear;
        state.routing_input_year = "2023".to_string();
        state.specific_year = Some(2023);
        state.routing_all_years = false;

        // Pasa a MonthScope: seleccionar Todos los meses (índice 0)
        state.active_routing_modal = RoutingModal::MonthScope;
        state.routing_modal_month_scope_idx = 0;
        state.specific_month = None; // Todos los meses del año 2023
        state.routing_all_months = true;
        state.active_routing_modal = RoutingModal::None;

        assert_eq!(state.specific_year, Some(2023));
        assert_eq!(state.specific_month, None, "Mes debe ser None indicando todos los meses del 2023");
        assert!(!state.routing_all_years);
        assert!(state.routing_all_months);
    }

    #[test]
    fn test_routing_specific_year_and_specific_month() {
        let mut state = AppState::new();
        state.routing_granularity = RoutingGranularity::YearsAndMonths;
        state.specific_year = Some(2023);
        state.routing_all_years = false;

        // Mes específico (ej. Mayo = 5)
        state.active_routing_modal = RoutingModal::MonthScope;
        state.routing_modal_month_scope_idx = 1;
        state.active_routing_modal = RoutingModal::SpecificMonth;
        state.routing_input_month = 5;
        state.specific_month = Some(5);
        state.routing_all_months = false;
        state.active_routing_modal = RoutingModal::None;

        assert_eq!(state.specific_year, Some(2023));
        assert_eq!(state.specific_month, Some(5));
        assert!(!state.routing_all_months);
    }

    #[test]
    fn test_routing_mirror_default_all_years_and_months() {
        let mut state = AppState::new();
        assert_eq!(state.routing_granularity, RoutingGranularity::Mirror);
        assert_eq!(state.routing_modal_criterion_idx, 0);

        state.step = WizardStep::FoldersMode;
        state.next_step();
        assert_eq!(state.step, WizardStep::Routing);
        assert_eq!(state.active_routing_modal, RoutingModal::Criterion);
        assert_eq!(state.routing_modal_criterion_idx, 0);

        // Confirmar criterio Espejo (índice 0) -> Pasa a YearScope
        state.active_routing_modal = RoutingModal::YearScope;
        state.routing_modal_year_scope_idx = 0; // Todos los años
        state.specific_year = None;
        state.routing_all_years = true;

        // En Espejo, pasa a MonthScope
        state.active_routing_modal = RoutingModal::MonthScope;
        state.routing_modal_month_scope_idx = 0; // Todos los meses
        state.specific_month = None;
        state.routing_all_months = true;
        state.active_routing_modal = RoutingModal::None;

        assert_eq!(state.routing_granularity, RoutingGranularity::Mirror);
        assert_eq!(state.specific_year, None);
        assert_eq!(state.specific_month, None);
        assert!(state.routing_all_years);
        assert!(state.routing_all_months);
    }

    #[test]
    fn test_routing_mirror_with_specific_year_and_month_filters() {
        let mut state = AppState::new();
        state.routing_granularity = RoutingGranularity::Mirror;

        // Año específico
        state.active_routing_modal = RoutingModal::YearScope;
        state.routing_modal_year_scope_idx = 1;
        state.active_routing_modal = RoutingModal::SpecificYear;
        state.routing_input_year = "2024".to_string();
        state.specific_year = Some(2024);
        state.routing_all_years = false;

        // Mes específico (Septiembre = 9)
        state.active_routing_modal = RoutingModal::MonthScope;
        state.routing_modal_month_scope_idx = 1;
        state.active_routing_modal = RoutingModal::SpecificMonth;
        state.routing_input_month = 9;
        state.specific_month = Some(9);
        state.routing_all_months = false;
        state.active_routing_modal = RoutingModal::None;

        assert_eq!(state.routing_granularity, RoutingGranularity::Mirror);
        assert_eq!(state.specific_year, Some(2024));
        assert_eq!(state.specific_month, Some(9));
        assert!(!state.routing_all_years);
        assert!(!state.routing_all_months);
    }

    #[test]
    fn test_pst_detail_modal_open_and_close() {
        let mut state = AppState::new();
        state.step = WizardStep::PstSource;
        assert_eq!(state.pst_detail_modal, PstDetailModalState::Closed);

        // Primera apertura: no está en caché -> retorna true para disparar inspección
        let should_trigger = state.open_pst_detail(r"C:\Correo\archivo.pst".to_string(), "archivo.pst".to_string());
        assert!(should_trigger);
        assert_eq!(state.step, WizardStep::PstDetailView);
        match &state.pst_detail_modal {
            PstDetailModalState::Loading { pst_path, pst_name, .. } => {
                assert_eq!(pst_path, r"C:\Correo\archivo.pst");
                assert_eq!(pst_name, "archivo.pst");
            }
            _ => panic!("Debería estar en estado Loading"),
        }

        // Simular que se guarda en caché
        let detail = PstDetail {
            file_name: "archivo.pst".to_string(),
            file_path: r"C:\Correo\archivo.pst".to_string(),
            size_mb: 250.5,
            total_items: 1200,
            last_email_date: Some("2024-05-15 14:30:00".to_string()),
            first_email_date: Some("2022-01-10 08:20:00".to_string()),
            folders: vec![PstFolderDetail {
                name: "Bandeja de entrada".to_string(),
                count: 1200,
                ..Default::default()
            }],
            years: vec![2022, 2023, 2024],
            year_months: std::collections::BTreeMap::new(),
            ..Default::default()
        };
        state.pst_details_cache.insert(r"C:\Correo\archivo.pst".to_string(), detail.clone());

        // Segunda apertura: ya en caché -> retorna false y pasa a Loaded de inmediato
        let should_trigger_cached = state.open_pst_detail(r"C:\Correo\archivo.pst".to_string(), "archivo.pst".to_string());
        assert!(!should_trigger_cached);
        match &state.pst_detail_modal {
            PstDetailModalState::Loaded(d) => {
                assert_eq!(d.total_items, 1200);
                assert_eq!(d.last_email_date.as_deref(), Some("2024-05-15 14:30:00"));
            }
            _ => panic!("Debería estar en estado Loaded"),
        }

        // Cerrar modal y vista completa
        state.close_pst_detail();
        assert_eq!(state.pst_detail_modal, PstDetailModalState::Closed);
        assert_eq!(state.step, WizardStep::PstSource);
    }

    #[test]
    fn test_pst_folder_explorer_navigation_and_selection() {
        let detail = PstDetail {
            file_name: "test.pst".to_string(),
            file_path: r"C:\Correo\test.pst".to_string(),
            size_mb: 150.0,
            total_items: 500,
            folders: vec![
                PstFolderDetail {
                    name: "Bandeja de entrada".to_string(),
                    path: "Bandeja de entrada".to_string(),
                    parent_path: None,
                    count: 300,
                    size_mb: 90.0,
                    has_children: true,
                    ..Default::default()
                },
                PstFolderDetail {
                    name: "Facturas 2024".to_string(),
                    path: r"Bandeja de entrada\Facturas 2024".to_string(),
                    parent_path: Some("Bandeja de entrada".to_string()),
                    count: 150,
                    size_mb: 45.0,
                    has_children: false,
                    ..Default::default()
                },
                PstFolderDetail {
                    name: "Elementos enviados".to_string(),
                    path: "Elementos enviados".to_string(),
                    parent_path: None,
                    count: 50,
                    size_mb: 15.0,
                    has_children: false,
                    ..Default::default()
                },
            ],
            years: vec![2024],
            ..Default::default()
        };

        let mut explorer = PstFolderExplorerState::new();

        // Nivel raíz: debe listar 2 carpetas principales
        let root_entries = explorer.current_entries(&detail);
        assert_eq!(root_entries.len(), 2);
        assert_eq!(root_entries[0].name, "Bandeja de entrada");
        assert_eq!(root_entries[1].name, "Elementos enviados");

        // Seleccionar Bandeja de entrada con Espacio
        explorer.toggle_select(&detail);
        assert_eq!(explorer.checked_folder_path, Some("Bandeja de entrada".to_string()));
        let selected_stats = explorer.get_selected_folder_stats(&detail);
        assert!(selected_stats.is_some());
        assert_eq!(selected_stats.unwrap().count, 300);

        // Deseleccionar con Espacio nuevamente
        explorer.toggle_select(&detail);
        assert_eq!(explorer.checked_folder_path, None);
        assert!(explorer.get_selected_folder_stats(&detail).is_none());

        // Entrar con 'E' a Bandeja de entrada
        let entered = explorer.navigate_into(&detail);
        assert!(entered);
        assert_eq!(explorer.current_parent, Some("Bandeja de entrada".to_string()));

        // En subnivel: debe haber ".. (Subir de nivel)" y "Facturas 2024"
        let sub_entries = explorer.current_entries(&detail);
        assert_eq!(sub_entries.len(), 2);
        assert_eq!(sub_entries[0].item_type, PstFolderItemType::ParentDir);
        assert_eq!(sub_entries[1].name, "Facturas 2024");

        // Subir de nivel con navigate_up
        let went_up = explorer.navigate_up(&detail);
        assert!(went_up);
        assert_eq!(explorer.current_parent, None);
    }

    #[test]
    fn test_pst_detail_deserialization_from_powershell_output() {
        let json_str = r#"{"file_path":"C:\\Correo\\test.pst","year_months":{"2026":[1,2,3,4,5,6,7]},"last_email_date":"2026-07-06 18:04:02","size_mb":573.03,"years":[2026],"first_email_date":"2026-01-19 09:25:29","file_name":"test.pst","counts_by_month":{"2026-07":82,"2026-06":339},"folders":[{"path":"Bandeja de entrada","parent_path":null,"years":[2026],"counts_by_year":{"2026":960},"has_children":true,"year_months":{"2026":[1,2,3,4,5,6,7]},"count":960,"name":"Bandeja de entrada","sizes_by_year_mb":{"2026":328.07},"size_mb":328.07,"sizes_by_month_mb":{"2026-03":52.12},"counts_by_month":{"2026-07":63}}],"sizes_by_year_mb":{"2026":510.22},"sizes_by_month_mb":{"2026-03":111.6},"total_items":1341,"counts_by_year":{"2026":1341}}"#;
        let detail: Result<PstDetail, _> = serde_json::from_str(json_str);
        assert!(detail.is_ok(), "Deserialization should succeed: {:?}", detail.err());
        let d = detail.unwrap();
        assert_eq!(d.file_name, "test.pst");
        assert_eq!(d.folders.len(), 1);
        assert_eq!(d.folders[0].name, "Bandeja de entrada");
        assert_eq!(d.folders[0].years, vec![2026]);
    }
}

