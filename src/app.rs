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
        }
    }

    pub fn index(&self) -> usize {
        match self {
            WizardStep::Welcome => 0,
            WizardStep::FileExplorer => 1,
            WizardStep::PstSource => 1,
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

    dir_entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    file_entries.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    entries.extend(dir_entries);
    entries.extend(file_entries);

    (entries, false)
}

pub fn scan_folder_for_psts(folder: &Path) -> Vec<PstItem> {
    let mut items = Vec::new();
    if let Ok(read_dir) = std::fs::read_dir(folder) {
        for entry in read_dir.flatten() {
            if let Ok(ft) = entry.file_type() {
                if ft.is_file() {
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
    }
    items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));
    items
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TransferMode {
    Copy,
    Move,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RoutingGranularity {
    Years,
    YearsAndMonths,
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

    // Configuración Paso 3: Buzón
    pub is_shared_mailbox: bool,
    pub target_mailbox: String,

    // Configuración Paso 4: Carpetas y Modo
    pub include_inbox: bool,
    pub include_sent: bool,
    pub include_deleted: bool,
    pub include_custom_folders: bool,
    pub transfer_mode: TransferMode,

    // Configuración Paso 5: Enrutamiento
    pub routing_enabled: bool,
    pub routing_granularity: RoutingGranularity,
    #[allow(dead_code)]
    pub specific_year: Option<u32>,

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
            is_shared_mailbox: false,
            target_mailbox: "buzon.personal@empresa.com".to_string(),
            include_inbox: true,
            include_sent: true,
            include_deleted: false,
            include_custom_folders: true,
            transfer_mode: TransferMode::Copy,
            routing_enabled: true,
            routing_granularity: RoutingGranularity::YearsAndMonths,
            specific_year: None,
            deduplication_enabled: true,
            deep_scan_enabled: true,
            adaptive_throttling_enabled: true,
            explorer,
            progress: ProgressState::default(),
            activity_log: VecDeque::with_capacity(300),
        }
    }

    pub fn log_event(&mut self, event: String) {
        if self.activity_log.len() >= 300 {
            self.activity_log.pop_front();
        }
        self.activity_log.push_back(event);
    }

    pub fn next_step(&mut self) {
        self.step = match self.step {
            WizardStep::Welcome => WizardStep::PstSource,
            WizardStep::FileExplorer => WizardStep::PstSource,
            WizardStep::PstSource => WizardStep::Mailbox,
            WizardStep::Mailbox => WizardStep::FoldersMode,
            WizardStep::FoldersMode => WizardStep::Routing,
            WizardStep::Routing => WizardStep::Deduplication,
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
            WizardStep::Mailbox => WizardStep::PstSource,
            WizardStep::FoldersMode => WizardStep::Mailbox,
            WizardStep::Routing => WizardStep::FoldersMode,
            WizardStep::Deduplication => WizardStep::Routing,
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
        let has_c = drives.iter().any(|d| d.path == PathBuf::from(r"C:\"));
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
}
