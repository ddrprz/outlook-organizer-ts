use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WizardStep {
    Welcome,
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
            WizardStep::Welcome => "Bienvenida y Perfil MAPI",
            WizardStep::PstSource => "Origen y Escaneo de PSTs",
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
            WizardStep::Welcome => 1,
            WizardStep::PstSource => 2,
            WizardStep::Mailbox => 3,
            WizardStep::FoldersMode => 4,
            WizardStep::Routing => 5,
            WizardStep::Deduplication => 6,
            WizardStep::Filters => 7,
            WizardStep::Summary => 8,
            WizardStep::Execution => 8,
            WizardStep::Completion => 8,
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
    pub current_pst_idx: usize,
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
    
    // Configuración Paso 1: Perfil
    pub use_default_profile: bool,
    pub custom_profile_name: String,

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
    pub specific_year: Option<u32>,

    // Configuración Paso 6: Deduplicación
    pub deduplication_enabled: bool,
    pub deep_scan_enabled: bool,

    // Configuración Paso 7: Filtros y Throttling
    pub adaptive_throttling_enabled: bool,

    // Métricas y progreso en tiempo real
    pub progress: ProgressState,
    pub activity_log: VecDeque<String>, // Buffer circular limitado (max 300)
}

impl AppState {
    pub fn new() -> Self {
        let mut sample_psts = Vec::new();
        sample_psts.push(PstItem {
            path: r"C:\Correo\Archivo_2023.pst".to_string(),
            name: "Archivo_2023.pst".to_string(),
            size_mb: 2450.5,
            selected: true,
        });
        sample_psts.push(PstItem {
            path: r"C:\Correo\Historico_2024.pst".to_string(),
            name: "Historico_2024.pst".to_string(),
            size_mb: 4120.0,
            selected: true,
        });

        Self {
            step: WizardStep::Welcome,
            should_quit: false,
            use_default_profile: true,
            custom_profile_name: String::new(),
            pst_scan_path: r"C:\Correo".to_string(),
            discovered_psts: sample_psts,
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
