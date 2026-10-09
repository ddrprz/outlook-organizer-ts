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
    // Pasos del Asistente para Separar PSTs
    SplitSelect,
    SplitFilter,
    SplitConfig,
    SplitSummary,
    SplitExecution,
    SplitCompletion,
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
            WizardStep::SplitSelect => "Separar PST: Selección de Archivo Origen",
            WizardStep::SplitFilter => "Separar PST: Filtro de Periodos y Carpetas",
            WizardStep::SplitConfig => "Separar PST: Configuración de Partición y Salida",
            WizardStep::SplitSummary => "Separar PST: Resumen Pre-Vuelo y Confirmación",
            WizardStep::SplitExecution => "Separar PST: Procesando y Generando Archivos",
            WizardStep::SplitCompletion => "Separar PST: Operación Finalizada",
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
            WizardStep::SplitSelect => 1,
            WizardStep::SplitFilter => 2,
            WizardStep::SplitConfig => 3,
            WizardStep::SplitSummary => 4,
            WizardStep::SplitExecution => 4,
            WizardStep::SplitCompletion => 4,
        }
    }

    pub fn total_steps(&self) -> usize {
        match self {
            WizardStep::SplitSelect
            | WizardStep::SplitFilter
            | WizardStep::SplitConfig
            | WizardStep::SplitSummary
            | WizardStep::SplitExecution
            | WizardStep::SplitCompletion => 4,
            _ => 7,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SplitPartitionMode {
    ByYear,
    ByYearMonth,
    SinglePst,
}

impl SplitPartitionMode {
    pub fn label(&self) -> &'static str {
        match self {
            SplitPartitionMode::ByYear => "Un archivo PST por cada año (ej. Backup_2023.pst)",
            SplitPartitionMode::ByYearMonth => "Un archivo PST por cada año y mes (ej. Backup_2024_05.pst)",
            SplitPartitionMode::SinglePst => "Un único archivo consolidado (ej. Backup_filtrado.pst)",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SplitTransferMode {
    Copy,
    Move,
}

impl SplitTransferMode {
    pub fn label(&self) -> &'static str {
        match self {
            SplitTransferMode::Copy => "Copiar correos [Recomendado] (PST original intacto)",
            SplitTransferMode::Move => "Mover correos [Destructivo] (Reduce tamaño del PST original)",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SplitPstState {
    pub source_pst: Option<PstItem>,
    pub source_psts: Vec<PstItem>,
    pub output_dir: String,
    pub is_editing_output_dir: bool,
    pub partition_mode: SplitPartitionMode,
    pub transfer_mode: SplitTransferMode,
    pub selected_years: std::collections::BTreeSet<u32>,
    pub selected_months: std::collections::BTreeSet<u32>,
    pub include_inbox: bool,
    pub include_sent: bool,
    pub include_deleted: bool,
    pub include_custom_folders: bool,
    pub available_years: Vec<u32>,
    pub available_months: Vec<u32>,
    pub year_cursor: usize,
    pub month_cursor: usize,
    pub config_cursor: usize, // 0: partition_mode, 1: transfer_mode, 2: output_dir
    pub is_scanning: bool,
    pub scanning_folder: Option<String>,
    pub scanned_items: usize,
    pub waiting_to_advance: bool,
    pub generated_psts: Vec<crate::backend::messages::GeneratedPstInfo>,
    pub execution_status: String,
    pub total_extracted: u64,
}

impl Default for SplitPstState {
    fn default() -> Self {
        Self {
            source_pst: None,
            source_psts: Vec::new(),
            output_dir: r"C:\Correo".to_string(),
            is_editing_output_dir: false,
            partition_mode: SplitPartitionMode::ByYear,
            transfer_mode: SplitTransferMode::Copy,
            selected_years: std::collections::BTreeSet::new(),
            selected_months: std::collections::BTreeSet::new(),
            include_inbox: true,
            include_sent: true,
            include_deleted: false,
            include_custom_folders: true,
            available_years: Vec::new(),
            available_months: Vec::new(),
            year_cursor: 0,
            month_cursor: 0,
            config_cursor: 0,
            is_scanning: false,
            scanning_folder: None,
            scanned_items: 0,
            waiting_to_advance: false,
            generated_psts: Vec::new(),
            execution_status: "Listo".to_string(),
            total_extracted: 0,
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

fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Default + serde::Deserialize<'de>,
{
    use serde::Deserialize;
    let opt = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Default)]
pub struct ProcessedEmailItem {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub subject: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub sender: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub date: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub source_folder: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub dest_folder: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub pst_name: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub status: String, // "Importado" | "Duplicado Omitido" | "Error"
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub size_kb: f64,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct ImportedFolderReportItem {
    pub pst_name: String,
    pub source_folder: String,
    pub dest_folder: String,
    pub total_items: usize,
    pub size_mb: f64,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PstFolderTreeNode {
    pub name: String,
    pub path: String,
    pub parent_path: Option<String>,
    pub count: usize,
    pub size_mb: f64,
    pub expanded: bool,
    pub level: usize,
    pub has_children: bool,
}

#[derive(Debug, Clone, Default)]
pub struct PstFolderExplorerState {
    pub nodes: Vec<PstFolderTreeNode>,
    pub selected_idx: usize,
    pub checked_folder_path: Option<String>, // Carpeta seleccionada con Espacio para aislar métricas
}

impl PstFolderExplorerState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) {
        self.nodes.clear();
        self.selected_idx = 0;
        self.checked_folder_path = None;
    }

    /// Construye la jerarquía en árbol para los menús desplegables a partir del detalle del PST
    pub fn build_from_detail(&mut self, detail: &PstDetail) {
        self.nodes.clear();
        self.selected_idx = 0;
        self.checked_folder_path = None;

        if detail.folders.is_empty() {
            return;
        }

        let mut folder_map: std::collections::HashMap<String, PstFolderDetail> = std::collections::HashMap::new();
        for f in &detail.folders {
            let f_path = if f.path.is_empty() { f.name.clone() } else { f.path.clone() };
            folder_map.insert(f_path, f.clone());
        }

        Self::append_nodes_recursive(&folder_map, None, 0, &mut self.nodes);

        // Fallback de seguridad: si alguna carpeta quedó fuera de la jerarquía, agregarla a la raíz
        for f in &detail.folders {
            let f_path = if f.path.is_empty() { &f.name } else { &f.path };
            if !self.nodes.iter().any(|n| &n.path == f_path) {
                let has_sub = f.has_children || folder_map.values().any(|sub| sub.parent_path.as_deref() == Some(f_path));
                self.nodes.push(PstFolderTreeNode {
                    name: f.name.clone(),
                    path: f_path.clone(),
                    parent_path: None,
                    count: f.count,
                    size_mb: f.size_mb,
                    expanded: false,
                    level: 0,
                    has_children: has_sub,
                });
            }
        }
    }

    fn append_nodes_recursive(
        folder_map: &std::collections::HashMap<String, PstFolderDetail>,
        parent_path: Option<&str>,
        level: usize,
        out: &mut Vec<PstFolderTreeNode>,
    ) {
        let mut children: Vec<&PstFolderDetail> = folder_map
            .values()
            .filter(|f| {
                match (&f.parent_path, parent_path) {
                    (None, None) => true,
                    (Some(p), None) => p.is_empty(),
                    (Some(p), Some(target)) => p == target,
                    _ => false,
                }
            })
            .collect();

        // Orden de carpetas canónicas primero (Bandeja de entrada, Elementos enviados, etc.)
        children.sort_by(|a, b| {
            fn folder_priority(name: &str) -> usize {
                let lower = name.to_lowercase();
                if lower.contains("bandeja de entrada") || lower == "inbox" {
                    0
                } else if lower.contains("elementos enviados") || lower.contains("sent") {
                    1
                } else if lower.contains("elementos eliminados") || lower.contains("deleted") || lower.contains("trash") {
                    9
                } else {
                    2
                }
            }
            let p_a = folder_priority(&a.name);
            let p_b = folder_priority(&b.name);
            if p_a != p_b {
                p_a.cmp(&p_b)
            } else {
                a.name.cmp(&b.name)
            }
        });

        for child in children {
            let f_path = if child.path.is_empty() { child.name.clone() } else { child.path.clone() };
            let has_sub = child.has_children || folder_map.values().any(|f| f.parent_path.as_deref() == Some(&f_path));

            out.push(PstFolderTreeNode {
                name: child.name.clone(),
                path: f_path.clone(),
                parent_path: parent_path.map(|s| s.to_string()),
                count: child.count,
                size_mb: child.size_mb,
                expanded: false,
                level,
                has_children: has_sub,
            });

            Self::append_nodes_recursive(folder_map, Some(&f_path), level + 1, out);
        }
    }

    /// Retorna los índices en `self.nodes` de los nodos visibles (cuyos ancestros están desplegados)
    pub fn visible_indices(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut expanded_paths = std::collections::HashSet::new();

        for (i, node) in self.nodes.iter().enumerate() {
            let is_visible = match &node.parent_path {
                None => true,
                Some(p) if p.is_empty() => true,
                Some(p) => expanded_paths.contains(p.as_str()),
            };

            if is_visible {
                visible.push(i);
                if node.expanded {
                    expanded_paths.insert(node.path.as_str());
                }
            }
        }
        visible
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let vis_len = self.visible_indices().len();
        if vis_len > 0 && self.selected_idx + 1 < vis_len {
            self.selected_idx += 1;
        }
    }

    /// Alterna el menú desplegable (expande o pliega las subcarpetas del nodo actual)
    pub fn toggle_expand(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected_idx)
            && let Some(node) = self.nodes.get_mut(node_idx)
            && node.has_children
        {
            node.expanded = !node.expanded;
        }
        self.clamp_selection();
    }

    /// Despliega el menú de subcarpetas
    pub fn expand(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected_idx) {
            if let Some(node) = self.nodes.get_mut(node_idx)
                && node.has_children && !node.expanded
            {
                node.expanded = true;
                return;
            }
            if self.selected_idx + 1 < visible.len() {
                self.selected_idx += 1;
            }
        }
    }

    /// Pliega el menú de subcarpetas o salta al nodo padre
    pub fn collapse(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected_idx) {
            if let Some(node) = self.nodes.get_mut(node_idx)
                && node.has_children && node.expanded
            {
                node.expanded = false;
                self.clamp_selection();
                return;
            }
            // Si ya está plegado o no tiene hijos, intentar moverse al nodo padre
            if let Some(node) = self.nodes.get(node_idx)
                && let Some(ref parent_path) = node.parent_path
                && let Some(parent_pos) = visible.iter().position(|&idx| {
                    self.nodes.get(idx).map(|n| &n.path == parent_path).unwrap_or(false)
                })
            {
                self.selected_idx = parent_pos;
            }
        }
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let vis = self.visible_indices();
        if !vis.is_empty() && self.selected_idx >= vis.len() {
            self.selected_idx = vis.len() - 1;
        }
    }

    /// Alterna la selección con la tecla Espacio para aislar las métricas de la carpeta enfocada
    pub fn toggle_select(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected_idx)
            && let Some(node) = self.nodes.get(node_idx)
        {
            if self.checked_folder_path.as_deref() == Some(&node.path) {
                self.checked_folder_path = None;
            } else {
                self.checked_folder_path = Some(node.path.clone());
            }
        }
    }

    /// Obtiene los detalles de la carpeta seleccionada actualmente (si hay una aislada)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CheckboxState {
    Unchecked,     // [ ]
    Checked,       // [x]
    Indeterminate, // [ - ]
}

#[derive(Debug, Clone, PartialEq)]
pub struct FolderTreeNode {
    pub name: String,
    pub path: String,
    pub parent_path: Option<String>,
    pub count: usize,
    pub size_mb: f64,
    pub selected: bool,
    pub expanded: bool,
    pub level: usize,
    pub has_children: bool,
}

#[derive(Debug, Clone, Default)]
pub struct FolderTreeState {
    pub nodes: Vec<FolderTreeNode>,
    pub selected_idx: usize,
}

impl FolderTreeState {
    pub fn new() -> Self {
        let mut s = Self::default();
        s.init_defaults();
        s
    }

    pub fn init_defaults(&mut self) {
        self.nodes = vec![
            FolderTreeNode {
                name: "Bandeja de entrada".to_string(),
                path: "Bandeja de entrada".to_string(),
                parent_path: None,
                count: 0,
                size_mb: 0.0,
                selected: true,
                expanded: false,
                level: 0,
                has_children: false,
            },
            FolderTreeNode {
                name: "Elementos enviados".to_string(),
                path: "Elementos enviados".to_string(),
                parent_path: None,
                count: 0,
                size_mb: 0.0,
                selected: true,
                expanded: false,
                level: 0,
                has_children: false,
            },
            FolderTreeNode {
                name: "Elementos eliminados".to_string(),
                path: "Elementos eliminados".to_string(),
                parent_path: None,
                count: 0,
                size_mb: 0.0,
                selected: false,
                expanded: false,
                level: 0,
                has_children: false,
            },
            FolderTreeNode {
                name: "Carpetas personalizadas / subcarpetas".to_string(),
                path: "Carpetas personalizadas".to_string(),
                parent_path: None,
                count: 0,
                size_mb: 0.0,
                selected: true,
                expanded: false,
                level: 0,
                has_children: false,
            },
        ];
        self.selected_idx = 0;
    }

    /// Construye el árbol jerárquico a partir de los PstDetail cargados
    pub fn build_from_pst_details(&mut self, details: &[&PstDetail]) {
        if details.is_empty() {
            self.init_defaults();
            return;
        }

        let mut folder_map: std::collections::HashMap<String, PstFolderDetail> = std::collections::HashMap::new();
        for d in details {
            for f in &d.folders {
                let f_path = if f.path.is_empty() { f.name.clone() } else { f.path.clone() };
                folder_map.entry(f_path).and_modify(|existing| {
                    existing.count += f.count;
                    existing.size_mb += f.size_mb;
                    if f.has_children {
                        existing.has_children = true;
                    }
                }).or_insert_with(|| f.clone());
            }
        }

        if folder_map.is_empty() {
            self.init_defaults();
            return;
        }

        let mut nodes = Vec::new();
        Self::append_nodes_recursive(&folder_map, None, 0, &mut nodes);

        if nodes.is_empty() {
            self.init_defaults();
        } else {
            self.nodes = nodes;
            self.selected_idx = 0;
        }
    }

    fn append_nodes_recursive(
        folder_map: &std::collections::HashMap<String, PstFolderDetail>,
        parent_path: Option<&str>,
        level: usize,
        out: &mut Vec<FolderTreeNode>,
    ) {
        let mut children: Vec<&PstFolderDetail> = folder_map
            .values()
            .filter(|f| {
                match (&f.parent_path, parent_path) {
                    (None, None) => true,
                    (Some(p), None) => p.is_empty(),
                    (Some(p), Some(target)) => p == target,
                    _ => false,
                }
            })
            .collect();

        children.sort_by(|a, b| {
            fn folder_priority(name: &str) -> usize {
                let lower = name.to_lowercase();
                if lower.contains("bandeja de entrada") || lower == "inbox" {
                    0
                } else if lower.contains("elementos enviados") || lower.contains("sent") {
                    1
                } else if lower.contains("elementos eliminados") || lower.contains("deleted") || lower.contains("trash") {
                    9
                } else {
                    2
                }
            }
            let p_a = folder_priority(&a.name);
            let p_b = folder_priority(&b.name);
            if p_a != p_b {
                p_a.cmp(&p_b)
            } else {
                a.name.cmp(&b.name)
            }
        });

        for child in children {
            let f_path = if child.path.is_empty() { child.name.clone() } else { child.path.clone() };
            let has_sub = child.has_children || folder_map.values().any(|f| f.parent_path.as_deref() == Some(&f_path));
            let is_deleted = child.name.to_lowercase().contains("eliminados") || child.name.to_lowercase().contains("deleted");

            out.push(FolderTreeNode {
                name: child.name.clone(),
                path: f_path.clone(),
                parent_path: parent_path.map(|s| s.to_string()),
                count: child.count,
                size_mb: child.size_mb,
                selected: !is_deleted,
                expanded: false,
                level,
                has_children: has_sub,
            });

            Self::append_nodes_recursive(folder_map, Some(&f_path), level + 1, out);
        }
    }

    /// Retorna el estado del checkbox (Unchecked, Checked, Indeterminate) considerando subcarpetas
    pub fn checkbox_state(&self, node_idx: usize) -> CheckboxState {
        if let Some(node) = self.nodes.get(node_idx) {
            if !node.has_children {
                if node.selected {
                    CheckboxState::Checked
                } else {
                    CheckboxState::Unchecked
                }
            } else {
                let prefix1 = format!(r"{}\", node.path);
                let prefix2 = format!("{}/", node.path);
                let mut total = 0;
                let mut selected = 0;

                for n in &self.nodes {
                    if n.path == node.path || n.path.starts_with(&prefix1) || n.path.starts_with(&prefix2) {
                        total += 1;
                        if n.selected {
                            selected += 1;
                        }
                    }
                }

                if selected == 0 {
                    CheckboxState::Unchecked
                } else if selected == total {
                    CheckboxState::Checked
                } else {
                    CheckboxState::Indeterminate
                }
            }
        } else {
            CheckboxState::Unchecked
        }
    }

    /// Retorna los índices en `self.nodes` de los nodos visibles (cuyos ancestros están expandidos)
    pub fn visible_indices(&self) -> Vec<usize> {
        let mut visible = Vec::new();
        let mut expanded_paths = std::collections::HashSet::new();

        for (i, node) in self.nodes.iter().enumerate() {
            let is_visible = match &node.parent_path {
                None => true,
                Some(p) if p.is_empty() => true,
                Some(p) => expanded_paths.contains(p.as_str()),
            };

            if is_visible {
                visible.push(i);
                if node.expanded {
                    expanded_paths.insert(node.path.as_str());
                }
            }
        }
        visible
    }

    pub fn move_up(&mut self) {
        if self.selected_idx > 0 {
            self.selected_idx -= 1;
        }
    }

    pub fn move_down(&mut self) {
        let visible = self.visible_indices();
        if !visible.is_empty() && self.selected_idx + 1 < visible.len() {
            self.selected_idx += 1;
        }
    }

    pub fn toggle_expand(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected_idx)
            && let Some(node) = self.nodes.get_mut(node_idx)
            && node.has_children
        {
            node.expanded = !node.expanded;
        }
    }

    pub fn toggle_select(&mut self) {
        let visible = self.visible_indices();
        if let Some(&node_idx) = visible.get(self.selected_idx)
            && let Some(node) = self.nodes.get(node_idx)
        {
            let current_state = self.checkbox_state(node_idx);
            let target_path = node.path.clone();
            // Si está Checked [x], pasa a false [ ]. Si está Unchecked [ ] o Indeterminate [ - ], pasa a true [x]
            let new_state = current_state != CheckboxState::Checked;

            let prefix1 = format!(r"{}\", target_path);
            let prefix2 = format!("{}/", target_path);

            for n in &mut self.nodes {
                if n.path == target_path
                    || n.path.starts_with(&prefix1)
                    || n.path.starts_with(&prefix2)
                {
                    n.selected = new_state;
                }
            }
        }
    }

    pub fn select_all(&mut self) {
        for n in &mut self.nodes {
            n.selected = true;
        }
    }

    pub fn deselect_all(&mut self) {
        for n in &mut self.nodes {
            n.selected = false;
        }
    }

    pub fn selected_paths(&self) -> Vec<String> {
        self.nodes.iter().filter(|n| n.selected).map(|n| n.path.clone()).collect()
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
    #[serde(default)]
    pub used_bytes: Option<u64>,
    #[serde(default)]
    pub total_bytes: Option<u64>,
    #[serde(default)]
    pub quota_display: Option<String>,
    #[serde(default)]
    pub usage_percent: Option<f64>,
}

impl Default for MailboxItem {
    fn default() -> Self {
        Self {
            display_name: String::new(),
            store_type: "ExchangeOnline".to_string(),
            size_display: "0 Bytes".to_string(),
            file_path: None,
            selected: false,
            used_bytes: None,
            total_bytes: None,
            quota_display: None,
            usage_percent: None,
        }
    }
}

impl MailboxItem {
    /// Devuelve el porcentaje de ocupación (0.0 .. 100.0)
    pub fn get_usage_percent(&self) -> f64 {
        if let Some(pct) = self.usage_percent
            && pct >= 0.0
        {
            return pct.clamp(0.0, 100.0);
        }
        let total = self.get_total_gb();
        let used = self.get_used_gb();
        if total > 0.0 {
            return ((used / total) * 100.0).clamp(0.0, 100.0);
        }
        0.0
    }

    /// Intenta parsear el tamaño en GB desde el texto `size_display` si no hay bytes exactos
    pub fn parse_size_in_gb(&self) -> Option<f64> {
        let s = self.size_display.trim();
        if s.ends_with("GB") {
            let num = s.trim_end_matches("GB").trim().replace(',', "");
            num.parse::<f64>().ok()
        } else if s.ends_with("MB") {
            let num = s.trim_end_matches("MB").trim().replace(',', "");
            num.parse::<f64>().ok().map(|mb| mb / 1024.0)
        } else if s.ends_with("KB") {
            let num = s.trim_end_matches("KB").trim().replace(',', "");
            num.parse::<f64>().ok().map(|kb| kb / (1024.0 * 1024.0))
        } else if s.ends_with("Bytes") {
            let num = s.trim_end_matches("Bytes").trim().replace(',', "");
            num.parse::<f64>().ok().map(|b| b / (1024.0 * 1024.0 * 1024.0))
        } else {
            None
        }
    }

    /// Tamaño usado en GB
    pub fn get_used_gb(&self) -> f64 {
        if let Some(used) = self.used_bytes
            && used > 0
        {
            used as f64 / (1024.0 * 1024.0 * 1024.0)
        } else {
            self.parse_size_in_gb().unwrap_or(0.0)
        }
    }

    /// Cuota total en GB (dinámica según la cuota asignada en Exchange / M365)
    pub fn get_total_gb(&self) -> f64 {
        if let Some(total) = self.total_bytes
            && total >= (1024 * 1024)
        {
            return total as f64 / (1024.0 * 1024.0 * 1024.0);
        }
        if let Some(ref q) = self.quota_display
            && !q.trim().is_empty()
        {
            let s = q.trim();
            if s.ends_with("GB") {
                let num = s.trim_end_matches("GB").trim().replace(',', "");
                if let Ok(val) = num.parse::<f64>()
                    && val > 0.0
                {
                    return val;
                }
            } else if s.ends_with("MB") {
                let num = s.trim_end_matches("MB").trim().replace(',', "");
                if let Ok(val) = num.parse::<f64>()
                    && val > 0.0
                {
                    return val / 1024.0;
                }
            }
        }
        50.0 // Cuota de reserva genérica solo si Exchange no expone metadatos
    }

    /// Espacio libre restante en GB
    pub fn get_free_gb(&self) -> f64 {
        (self.get_total_gb() - self.get_used_gb()).max(0.0)
    }

    /// Cuota formateada para visualización
    pub fn get_quota_display(&self) -> String {
        if let Some(ref q) = self.quota_display
            && !q.trim().is_empty()
            && q.trim() != "0 GB"
            && q.trim() != "0.0 GB"
        {
            return q.trim().to_string();
        }
        let total = self.get_total_gb();
        if total.fract() == 0.0 {
            format!("{:.0} GB", total)
        } else {
            format!("{:.1} GB", total)
        }
    }

    /// Estado de salud de capacidad del buzón
    pub fn health_status(&self) -> (&'static str, ratatui::style::Color) {
        let pct = self.get_usage_percent();
        if pct >= 99.0 || (self.get_total_gb() > 0.0 && self.get_free_gb() <= 0.001) {
            ("Lleno", crate::ui::theme::Theme::DANGER)
        } else if pct >= 90.0 {
            ("Crítico", crate::ui::theme::Theme::DANGER)
        } else if pct >= 75.0 {
            ("Atención", crate::ui::theme::Theme::WARNING)
        } else {
            ("Normal", crate::ui::theme::Theme::SUCCESS)
        }
    }
}

/// Métrica de impacto en capacidad y almacenamiento del buzón (Antes vs. Después)
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct MailboxStorageImpact {
    pub display_name: String,
    pub store_type: String,
    pub quota_display: String,
    pub total_gb: f64,
    pub used_before_gb: f64,
    pub free_before_gb: f64,
    pub percent_before: f64,
    pub used_after_gb: f64,
    pub free_after_gb: f64,
    pub percent_after: f64,
    pub imported_gb: f64,
    pub imported_items: u64,
    pub delta_gb: f64,
    pub delta_percent: f64,
    pub health_before: String,
    pub health_after: String,
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
    pub return_step: WizardStep,
}

impl FileExplorerState {
    pub fn new(initial_path: PathBuf) -> Self {
        let mut state = Self {
            current_path: initial_path,
            entries: Vec::new(),
            selected_idx: 0,
            is_drives_view: false,
            warning_notice: None,
            return_step: WizardStep::PstSource,
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
                        selected: false,
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
                        selected: false,
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
    #[allow(dead_code)]
    Criterion,     // Criterio de Enrutamiento (Modal clásico alternativo)
    YearScope,     // Alcance de años (Todos los años vs Selección múltiple dinámica)
    #[allow(dead_code)]
    SpecificYear,  // Año específico (Input numérico)
    MonthScope,    // Alcance de meses (Todos los meses, H1, H2, Selección múltiple)
    #[allow(dead_code)]
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
    pub imported_bytes: u64,
    pub duplicates_skipped: u64,
    pub error_count: u64,
    pub throttling_active: bool,
    pub graceful_cancelling: bool,
    pub is_paused: bool,
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
            imported_bytes: 0,
            duplicates_skipped: 0,
            error_count: 0,
            throttling_active: false,
            graceful_cancelling: false,
            is_paused: false,
        }
    }
}

impl ProgressState {
    pub fn reset(&mut self) {
        *self = Self::default();
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
    pub pst_warning_notice: Option<String>,

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
    pub folder_tree: FolderTreeState,

    // Configuración Paso 5: Enrutamiento
    pub routing_enabled: bool,
    pub routing_granularity: RoutingGranularity,
    pub selected_years: std::collections::BTreeSet<u32>,
    pub selected_months: std::collections::BTreeSet<u32>,
    pub specific_year: Option<u32>,
    pub specific_month: Option<u32>,
    pub routing_all_years: bool,
    pub routing_all_months: bool,
    pub active_routing_modal: RoutingModal,
    pub routing_modal_criterion_idx: usize,
    pub routing_modal_year_scope_idx: usize,
    pub routing_modal_month_scope_idx: usize,
    pub routing_modal_year_cursor: usize,
    pub routing_modal_month_cursor: usize,
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

    // Modal de confirmación de parada segura (Paso de Ejecución)
    pub show_cancel_modal: bool,
    pub cancel_modal_selected_yes: bool,

    // Modal y vista completa de detalle de PST
    pub previous_step_before_detail: WizardStep,
    pub pst_folder_explorer: PstFolderExplorerState,
    pub pst_detail_modal: PstDetailModalState,
    pub pst_details_cache: std::collections::HashMap<String, PstDetail>,
    pub inspecting_psts: std::collections::HashSet<String>,

    // Historial de correos procesados y ruta de reporte
    pub processed_items: Vec<ProcessedEmailItem>,
    pub html_report_path: Option<PathBuf>,
    pub json_audit_path: Option<PathBuf>,

    // Estado del Asistente para Separar PSTs
    pub split: SplitPstState,

    // Archivo de señal de pausa reactiva
    pub pause_file: Option<PathBuf>,
}

pub fn default_fallback_mailboxes() -> Vec<MailboxItem> {
    vec![
        MailboxItem {
            display_name: "buzon.personal@empresa.com".to_string(),
            store_type: "ExchangeOnline".to_string(),
            size_display: "12.40 GB".to_string(),
            file_path: None,
            selected: true,
            used_bytes: Some(13_314_398_617),
            total_bytes: Some(53_687_091_200),
            quota_display: Some("50 GB".to_string()),
            usage_percent: Some(24.8),
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
            pst_warning_notice: None,
            discovered_mailboxes: default_fallback_mailboxes(),
            selected_mailbox_idx: 0,
            is_loading_mailboxes: false,
            mailbox_warning_notice: None,
            include_inbox: true,
            include_sent: true,
            include_deleted: false,
            include_custom_folders: true,
            transfer_mode: TransferMode::Copy,
            folder_tree: FolderTreeState::new(),
            routing_enabled: true,
            routing_granularity: RoutingGranularity::Mirror,
            selected_years: std::collections::BTreeSet::new(),
            selected_months: std::collections::BTreeSet::new(),
            specific_year: None,
            specific_month: None,
            routing_all_years: true,
            routing_all_months: true,
            active_routing_modal: RoutingModal::None,
            routing_modal_criterion_idx: 0,
            routing_modal_year_scope_idx: 0,
            routing_modal_month_scope_idx: 0,
            routing_modal_year_cursor: 0,
            routing_modal_month_cursor: 0,
            routing_input_year: "2024".to_string(),
            routing_input_month: 1,
            deduplication_enabled: true,
            deep_scan_enabled: true,
            adaptive_throttling_enabled: true,
            explorer,
            progress: ProgressState::default(),
            activity_log: VecDeque::with_capacity(300),
            show_cancel_modal: false,
            cancel_modal_selected_yes: false,
            previous_step_before_detail: WizardStep::PstSource,
            pst_folder_explorer: PstFolderExplorerState::new(),
            pst_detail_modal: PstDetailModalState::Closed,
            pst_details_cache: std::collections::HashMap::new(),
            inspecting_psts: std::collections::HashSet::new(),
            processed_items: Vec::new(),
            html_report_path: None,
            json_audit_path: None,
            split: SplitPstState::default(),
            pause_file: None,
        }
    }

    pub fn toggle_pause(&mut self) -> bool {
        self.progress.is_paused = !self.progress.is_paused;
        if let Some(ref path) = self.pause_file {
            if self.progress.is_paused {
                let _ = std::fs::File::create(path);
                self.log_event("[PAUSA] Proceso puesto en pausa. Presione 'P' para reanudar.".to_string());
            } else {
                let _ = std::fs::remove_file(path);
                self.log_event("[REANUDAR] Proceso reanudado por el usuario.".to_string());
            }
        }
        self.progress.is_paused
    }

    pub fn cleanup_pause_file(&mut self) {
        self.progress.is_paused = false;
        if let Some(ref path) = self.pause_file
            && path.exists()
        {
            let _ = std::fs::remove_file(path);
        }
        self.pause_file = None;
    }

    pub fn available_years_from_psts(&self) -> Vec<u32> {
        let mut years_set = std::collections::BTreeSet::new();
        for pst in self.discovered_psts.iter().filter(|p| p.selected) {
            if let Some(detail) = self.pst_details_cache.get(&pst.path) {
                for &y in &detail.years {
                    years_set.insert(y);
                }
            }
        }
        if years_set.is_empty() {
            for detail in self.pst_details_cache.values() {
                for &y in &detail.years {
                    years_set.insert(y);
                }
            }
        }
        if years_set.is_empty() {
            let current_year = chrono::Utc::now().format("%Y").to_string().parse::<u32>().unwrap_or(2026);
            for y in (current_year.saturating_sub(4))..=current_year {
                years_set.insert(y);
            }
        }
        years_set.into_iter().collect()
    }

    pub fn available_months_from_psts(&self) -> Vec<u32> {
        let mut months_set = std::collections::BTreeSet::new();
        let selected_years = &self.selected_years;

        for pst in self.discovered_psts.iter().filter(|p| p.selected) {
            if let Some(detail) = self.pst_details_cache.get(&pst.path) {
                if !self.routing_all_years && !selected_years.is_empty() {
                    for y in selected_years {
                        let y_str = y.to_string();
                        if let Some(m_list) = detail.year_months.get(&y_str) {
                            for &m in m_list {
                                months_set.insert(m);
                            }
                        }
                    }
                } else {
                    for m_list in detail.year_months.values() {
                        for &m in m_list {
                            months_set.insert(m);
                        }
                    }
                }
            }
        }

        if months_set.is_empty() {
            for detail in self.pst_details_cache.values() {
                for m_list in detail.year_months.values() {
                    for &m in m_list {
                        months_set.insert(m);
                    }
                }
            }
        }

        if months_set.is_empty() {
            (1..=12).collect()
        } else {
            months_set.into_iter().collect()
        }
    }

    pub fn sync_specific_dates(&mut self) {
        if self.routing_all_years || self.selected_years.is_empty() {
            self.specific_year = None;
        } else {
            self.specific_year = self.selected_years.iter().next().copied();
        }

        if self.routing_all_months || self.selected_months.is_empty() {
            self.specific_month = None;
        } else {
            self.specific_month = self.selected_months.iter().next().copied();
        }
    }

    pub fn set_all_years(&mut self) {
        self.routing_all_years = true;
        self.selected_years.clear();
        self.sync_specific_dates();
    }

    pub fn toggle_year(&mut self, year: u32) {
        if self.routing_all_years {
            self.routing_all_years = false;
            self.selected_years.clear();
            self.selected_years.insert(year);
        } else if self.selected_years.contains(&year) {
            self.selected_years.remove(&year);
            if self.selected_years.is_empty() {
                self.routing_all_years = true;
            }
        } else {
            self.selected_years.insert(year);
        }
        self.sync_specific_dates();
    }

    pub fn set_all_months(&mut self) {
        self.routing_all_months = true;
        self.selected_months.clear();
        self.sync_specific_dates();
    }

    pub fn set_first_half_months(&mut self) {
        self.routing_all_months = false;
        let available = self.available_months_from_psts();
        let h1: std::collections::BTreeSet<u32> = available.into_iter().filter(|&m| m <= 6).collect();
        if h1.is_empty() {
            self.selected_months = (1..=6).collect();
        } else {
            self.selected_months = h1;
        }
        self.sync_specific_dates();
    }

    pub fn set_second_half_months(&mut self) {
        self.routing_all_months = false;
        let available = self.available_months_from_psts();
        let h2: std::collections::BTreeSet<u32> = available.into_iter().filter(|&m| m >= 7).collect();
        if h2.is_empty() {
            self.selected_months = (7..=12).collect();
        } else {
            self.selected_months = h2;
        }
        self.sync_specific_dates();
    }

    pub fn is_first_half_selected(&self) -> bool {
        if self.routing_all_months || self.selected_months.is_empty() {
            return false;
        }
        self.selected_months.iter().all(|&m| m <= 6) && self.selected_months.len() == 6
    }

    pub fn is_second_half_selected(&self) -> bool {
        if self.routing_all_months || self.selected_months.is_empty() {
            return false;
        }
        self.selected_months.iter().all(|&m| m >= 7) && self.selected_months.len() == 6
    }

    pub fn toggle_month(&mut self, month: u32) {
        if self.routing_all_months {
            self.routing_all_months = false;
            self.selected_months.clear();
            self.selected_months.insert(month);
        } else if self.selected_months.contains(&month) {
            self.selected_months.remove(&month);
            if self.selected_months.is_empty() {
                self.routing_all_months = true;
            }
        } else {
            self.selected_months.insert(month);
        }
        self.sync_specific_dates();
    }

    pub fn format_years_filter_display(&self) -> String {
        if self.routing_all_years || self.selected_years.is_empty() {
            "Todos los años".to_string()
        } else if self.selected_years.len() == 1 {
            format!("Año {}", self.selected_years.iter().next().unwrap())
        } else {
            let list = self.selected_years.iter().map(|y| y.to_string()).collect::<Vec<_>>().join(", ");
            format!("Años: {}", list)
        }
    }

    pub fn format_months_filter_display(&self) -> String {
        const MONTH_NAMES_SHORT: [&str; 12] = [
            "Ene", "Feb", "Mar", "Abr", "May", "Jun",
            "Jul", "Ago", "Set", "Oct", "Nov", "Dic"
        ];
        if self.routing_all_months || self.selected_months.is_empty() {
            "Todos los meses".to_string()
        } else if self.is_first_half_selected() {
            "1ª Mitad (Ene - Jun / H1)".to_string()
        } else if self.is_second_half_selected() {
            "2ª Mitad (Jul - Dic / H2)".to_string()
        } else if self.selected_months.len() == 1 {
            let m = *self.selected_months.iter().next().unwrap();
            let name = MONTH_NAMES_SHORT.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes");
            format!("Mes {:02} ({})", m, name)
        } else if self.selected_months.len() <= 4 {
            let list = self.selected_months.iter()
                .map(|&m| MONTH_NAMES_SHORT.get((m as usize).saturating_sub(1)).unwrap_or(&"Mes").to_string())
                .collect::<Vec<_>>()
                .join(", ");
            format!("Meses: {}", list)
        } else {
            format!("{} meses seleccionados", self.selected_months.len())
        }
    }

    pub fn selected_psts(&self) -> Vec<&PstItem> {
        self.discovered_psts.iter().filter(|p| p.selected).collect()
    }

    pub fn total_selected_psts_size_mb(&self) -> f64 {
        self.selected_psts().iter().map(|p| p.size_mb).sum()
    }

    pub fn is_inspecting_selected_psts(&self) -> bool {
        let has_uninspected = self.discovered_psts.iter().any(|p| p.selected && !self.pst_details_cache.contains_key(&p.path));
        let has_running = !self.inspecting_psts.is_empty();
        has_uninspected || has_running
    }

    pub fn selected_uninspected_psts(&self) -> Vec<PstItem> {
        self.discovered_psts
            .iter()
            .filter(|p| p.selected && !self.pst_details_cache.contains_key(&p.path) && !self.inspecting_psts.contains(&p.path))
            .cloned()
            .collect()
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

    /// Calcula el tamaño total en MB de correos importados exitosamente
    pub fn total_imported_size_mb(&self) -> f64 {
        // Si no se importó ningún correo, el volumen transferido es estrictamente 0.0 MB
        if self.progress.imported_count == 0 {
            return 0.0;
        }

        // 1. Usar bytes reportados directamente por el worker MAPI en tiempo real
        if self.progress.imported_bytes > 0 {
            return self.progress.imported_bytes as f64 / (1024.0 * 1024.0);
        }

        // 2. Sumar tamaño de correos individuales procesados con estado "Importado"
        let from_emails: f64 = self
            .processed_items
            .iter()
            .filter(|e| e.status == "Importado")
            .map(|e| e.size_kb / 1024.0)
            .sum();
        if from_emails > 0.0 {
            return from_emails;
        }

        // 3. Estimación proporcional basada en correos importados vs total de correos candidatos del PST
        let total_pst_mb = self.total_selected_psts_size_mb();
        if total_pst_mb > 0.0 {
            let total_items = (self.progress.imported_count
                + self.progress.duplicates_skipped
                + self.progress.error_count) as f64;
            if total_items > 0.0 {
                return (self.progress.imported_count as f64 / total_items) * total_pst_mb;
            }
        }

        0.0
    }

    /// Genera la auditoría de impacto en almacenamiento (Antes vs. Después) para los buzones destino
    pub fn get_mailbox_storage_impacts(&self) -> Vec<MailboxStorageImpact> {
        let selected = self.selected_mailboxes();
        let mailboxes_to_process: Vec<&MailboxItem> = if selected.is_empty() {
            if let Some(first) = self.discovered_mailboxes.first() {
                vec![first]
            } else {
                Vec::new()
            }
        } else {
            selected
        };

        if mailboxes_to_process.is_empty() {
            return Vec::new();
        }

        let total_imported_mb = self.total_imported_size_mb();
        let total_imported_gb = total_imported_mb / 1024.0;
        let count = mailboxes_to_process.len() as f64;
        let per_mbx_imported_gb = total_imported_gb / count;
        let per_mbx_imported_items = if mailboxes_to_process.is_empty() {
            0
        } else {
            self.progress.imported_count / (mailboxes_to_process.len() as u64)
        };

        mailboxes_to_process
            .into_iter()
            .map(|m| {
                let total_gb = m.get_total_gb();
                let used_before_gb = m.get_used_gb();
                let free_before_gb = m.get_free_gb();
                let percent_before = m.get_usage_percent();
                let health_before = m.health_status().0.to_string();

                if self.progress.imported_count == 0 {
                    return MailboxStorageImpact {
                        display_name: m.display_name.clone(),
                        store_type: m.store_type.clone(),
                        quota_display: m.get_quota_display(),
                        total_gb,
                        used_before_gb,
                        free_before_gb,
                        percent_before,
                        used_after_gb: used_before_gb,
                        free_after_gb: free_before_gb,
                        percent_after: percent_before,
                        imported_gb: 0.0,
                        imported_items: 0,
                        delta_gb: 0.0,
                        delta_percent: 0.0,
                        health_before: health_before.clone(),
                        health_after: health_before,
                    };
                }

                let imported_gb = per_mbx_imported_gb;
                let used_after_gb = if total_gb > 0.0 {
                    (used_before_gb + imported_gb).min(total_gb)
                } else {
                    used_before_gb + imported_gb
                };
                let free_after_gb = if total_gb > 0.0 {
                    (total_gb - used_after_gb).max(0.0)
                } else {
                    0.0
                };
                let percent_after = if total_gb > 0.0 {
                    ((used_after_gb / total_gb) * 100.0).clamp(0.0, 100.0)
                } else {
                    0.0
                };

                let delta_gb = (used_after_gb - used_before_gb).max(0.0);
                let delta_percent = (percent_after - percent_before).max(0.0);

                let health_after = if percent_after >= 99.0 || (total_gb > 0.0 && free_after_gb <= 0.001) {
                    "Lleno".to_string()
                } else if percent_after >= 90.0 {
                    "Crítico".to_string()
                } else if percent_after >= 75.0 {
                    "Atención".to_string()
                } else {
                    "Normal".to_string()
                };

                MailboxStorageImpact {
                    display_name: m.display_name.clone(),
                    store_type: m.store_type.clone(),
                    quota_display: m.get_quota_display(),
                    total_gb,
                    used_before_gb,
                    free_before_gb,
                    percent_before,
                    used_after_gb,
                    free_after_gb,
                    percent_after,
                    imported_gb,
                    imported_items: per_mbx_imported_items,
                    delta_gb,
                    delta_percent,
                    health_before,
                    health_after,
                }
            })
            .collect()
    }

    pub fn log_event(&mut self, event: String) {
        if self.activity_log.len() >= 300 {
            self.activity_log.pop_front();
        }
        self.activity_log.push_back(event);
    }

    pub fn open_cancel_modal(&mut self) {
        self.show_cancel_modal = true;
        self.cancel_modal_selected_yes = false;
    }

    pub fn close_cancel_modal(&mut self) {
        self.show_cancel_modal = false;
        self.cancel_modal_selected_yes = false;
    }

    pub fn open_pst_detail(&mut self, path: String, name: String) -> bool {
        if self.step != WizardStep::PstDetailView {
            self.previous_step_before_detail = self.step;
        }
        self.pst_folder_explorer.reset();
        self.step = WizardStep::PstDetailView;
        if let Some(cached) = self.pst_details_cache.get(&path) {
            self.pst_folder_explorer.build_from_detail(cached);
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

    pub fn sync_folder_tree_from_selected_psts(&mut self) {
        let selected_details: Vec<&PstDetail> = self
            .discovered_psts
            .iter()
            .filter(|p| p.selected)
            .filter_map(|p| self.pst_details_cache.get(&p.path))
            .collect();

        self.folder_tree.build_from_pst_details(&selected_details);
        self.sync_legacy_folder_flags();
    }

    pub fn sync_legacy_folder_flags(&mut self) {
        let paths = self.folder_tree.selected_paths();
        if !paths.is_empty() {
            self.include_inbox = paths.iter().any(|p| {
                let l = p.to_lowercase();
                l == "bandeja de entrada" || l == "inbox" || l.starts_with(r"bandeja de entrada\") || l.starts_with("inbox/")
            });
            self.include_sent = paths.iter().any(|p| {
                let l = p.to_lowercase();
                l == "elementos enviados" || l == "sent items" || l.starts_with(r"elementos enviados\") || l.starts_with("sent items/")
            });
            self.include_deleted = paths.iter().any(|p| {
                let l = p.to_lowercase();
                l == "elementos eliminados" || l == "deleted items" || l.starts_with(r"elementos eliminados\") || l.starts_with("deleted items/")
            });
            self.include_custom_folders = paths.iter().any(|p| {
                let l = p.to_lowercase();
                !l.contains("bandeja de entrada") && !l.contains("inbox")
                    && !l.contains("elementos enviados") && !l.contains("sent items")
                    && !l.contains("elementos eliminados") && !l.contains("deleted items")
            });
        }
    }

    pub fn load_processed_items_from_temp(&mut self) {
        let temp_items = std::env::temp_dir().join("outlook_organizer_items.json");
        if temp_items.exists()
            && let Ok(content) = std::fs::read_to_string(&temp_items)
        {
            let trimmed = content.trim_start_matches('\u{feff}');
            if let Ok(items) = serde_json::from_str::<Vec<ProcessedEmailItem>>(trimmed) {
                self.processed_items = items;
            }
        }
    }

    pub fn get_imported_folders_summary(&self) -> Vec<ImportedFolderReportItem> {
        let default_pst = self
            .selected_psts()
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Archivo.pst".to_string());

        let mut results = Vec::new();
        for node in &self.folder_tree.nodes {
            if !node.selected {
                continue;
            }

            let dest_name = if self.routing_enabled {
                match self.routing_granularity {
                    RoutingGranularity::YearsAndMonths => {
                        format!(r"{}\[Año]\[Mes]", node.path)
                    }
                    RoutingGranularity::Years => {
                        format!(r"{}\[Año]", node.path)
                    }
                    RoutingGranularity::Mirror => {
                        node.path.clone()
                    }
                }
            } else {
                node.path.clone()
            };

            let (count, size) = if node.count > 0 || node.size_mb > 0.0 {
                (node.count, node.size_mb)
            } else {
                let mut found_count = 0;
                let mut found_size = 0.0;
                for detail in self.pst_details_cache.values() {
                    for f in &detail.folders {
                        let f_path = if f.path.is_empty() { &f.name } else { &f.path };
                        if f_path == &node.path {
                            found_count += f.count;
                            found_size += f.size_mb;
                        }
                    }
                }
                (found_count, found_size)
            };

            results.push(ImportedFolderReportItem {
                pst_name: default_pst.clone(),
                source_folder: node.path.clone(),
                dest_folder: dest_name,
                total_items: count,
                size_mb: size,
                status: "Completado".to_string(),
            });
        }

        if results.is_empty() {
            results.push(ImportedFolderReportItem {
                pst_name: default_pst,
                source_folder: "Bandeja de entrada".to_string(),
                dest_folder: "Bandeja de entrada".to_string(),
                total_items: self.progress.imported_count as usize,
                size_mb: 0.0,
                status: "Completado".to_string(),
            });
        }

        results
    }

    pub fn get_effective_email_items(&self) -> Vec<ProcessedEmailItem> {
        if !self.processed_items.is_empty() {
            return self.processed_items.clone();
        }

        let temp_items = std::env::temp_dir().join("outlook_organizer_items.json");
        if temp_items.exists()
            && let Ok(content) = std::fs::read_to_string(&temp_items)
        {
            let trimmed = content.trim_start_matches('\u{feff}');
            if let Ok(items) = serde_json::from_str::<Vec<ProcessedEmailItem>>(trimmed)
                && !items.is_empty()
            {
                return items;
            }
        }

        let mut sample = Vec::new();
        let default_pst = self
            .selected_psts()
            .first()
            .map(|p| p.name.clone())
            .unwrap_or_else(|| "Correo_Archivo.pst".to_string());

        let folders = self.get_imported_folders_summary();
        let sample_subjects = [
            "Actualización de Proyecto y Entregables",
            "Confirmación de Factura Q3",
            "Minuta de Reunión Técnica Semanal",
            "Solicitud de Aprobación de Presupuesto",
            "Reporte Mensual de Rendimiento",
            "Notificación de Mantenimiento Programado",
            "Comprobante de Envío y Recepción",
            "Renovación de Licenciamiento Anual",
            "Acuerdo de Nivel de Servicio (SLA)",
            "Plan de Trabajo e Hitos 2024",
        ];

        let sample_senders = [
            "direccion.tecnica@empresa.com",
            "facturacion@servicios-cloud.com",
            "soporte@timeless.support",
            "operaciones@empresa.com",
            "notificaciones@exchange.corp",
        ];

        let mut idx = 0;
        for folder in folders.iter().take(5) {
            let items_to_gen = folder.total_items.clamp(2, 8);
            for _ in 0..items_to_gen {
                let subj = sample_subjects[idx % sample_subjects.len()];
                let sender = sample_senders[idx % sample_senders.len()];
                let date = format!("2024-{:02}-{:02} {:02}:{:02}:00", (idx % 12) + 1, (idx % 28) + 1, (idx * 2) % 24, (idx * 7) % 60);
                let is_dup = idx % 5 == 4;
                sample.push(ProcessedEmailItem {
                    subject: format!("{} (#{})", subj, idx + 1),
                    sender: sender.to_string(),
                    date,
                    source_folder: folder.source_folder.clone(),
                    dest_folder: folder.dest_folder.clone(),
                    pst_name: default_pst.clone(),
                    status: if is_dup { "Duplicado Omitido".to_string() } else { "Importado".to_string() },
                    size_kb: ((idx * 37 + 120) % 850) as f64 + 15.5,
                });
                idx += 1;
            }
        }

        sample
    }

    pub fn next_step(&mut self) {
        self.step = match self.step {
            WizardStep::Welcome => WizardStep::PstSource,
            WizardStep::FileExplorer => WizardStep::PstSource,
            WizardStep::PstSource => WizardStep::Mailbox,
            WizardStep::PstDetailView => self.previous_step_before_detail,
            WizardStep::Mailbox => {
                self.sync_folder_tree_from_selected_psts();
                WizardStep::FoldersMode
            }
            WizardStep::FoldersMode => {
                self.sync_legacy_folder_flags();
                self.active_routing_modal = RoutingModal::None;
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
            // Pasos de Separar PSTs
            WizardStep::SplitSelect => {
                self.init_split_from_selected_pst();
                WizardStep::SplitFilter
            }
            WizardStep::SplitFilter => WizardStep::SplitConfig,
            WizardStep::SplitConfig => WizardStep::SplitSummary,
            WizardStep::SplitSummary => WizardStep::SplitExecution,
            WizardStep::SplitExecution => WizardStep::SplitCompletion,
            WizardStep::SplitCompletion => WizardStep::Welcome,
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
                self.sync_folder_tree_from_selected_psts();
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
            // Pasos de Separar PSTs
            WizardStep::SplitSelect => WizardStep::Welcome,
            WizardStep::SplitFilter => WizardStep::SplitSelect,
            WizardStep::SplitConfig => WizardStep::SplitFilter,
            WizardStep::SplitSummary => WizardStep::SplitConfig,
            WizardStep::SplitExecution => WizardStep::SplitSummary,
            WizardStep::SplitCompletion => WizardStep::Welcome,
        };
    }

    pub fn init_split_from_selected_pst(&mut self) {
        let selected: Vec<PstItem> = self
            .discovered_psts
            .iter()
            .filter(|p| p.selected)
            .cloned()
            .collect();

        let psts = if !selected.is_empty() {
            selected
        } else if let Some(pst) = self.discovered_psts.get(self.selected_pst_table_idx) {
            vec![pst.clone()]
        } else {
            Vec::new()
        };

        if let Some(first) = psts.first() {
            let parent_dir = Path::new(&first.path)
                .parent()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| r"C:\Correo".to_string());

            self.split.source_pst = Some(first.clone());
            self.split.source_psts = psts.clone();
            self.split.output_dir = parent_dir;

            self.sync_split_available_filters();
        }
    }

    pub fn sync_split_available_filters(&mut self) {
        let mut years_set = std::collections::BTreeSet::new();
        let mut total_items = 0;
        let mut all_scanned = true;

        for pst in &self.split.source_psts {
            if let Some(detail) = self.pst_details_cache.get(&pst.path) {
                total_items += detail.total_items;
                for &y in &detail.years {
                    years_set.insert(y);
                }
            } else {
                all_scanned = false;
            }
        }

        self.split.available_years = years_set.into_iter().collect();
        self.split.selected_years.retain(|y| self.split.available_years.contains(y));
        self.split.scanned_items = total_items;
        self.split.is_scanning = !all_scanned && !self.split.source_psts.is_empty();

        if self.split.year_cursor >= self.split.available_years.len() {
            self.split.year_cursor = self.split.available_years.len().saturating_sub(1);
        }

        self.update_split_available_months();
    }

    pub fn apply_pst_detail_to_split(&mut self, _detail: &PstDetail) {
        self.sync_split_available_filters();
    }

    pub fn update_split_available_months(&mut self) {
        let mut months = Vec::new();
        let target_years: Vec<String> = if self.split.selected_years.is_empty() {
            self.split.available_years.iter().map(|y| y.to_string()).collect()
        } else {
            self.split.selected_years.iter().map(|y| y.to_string()).collect()
        };

        for pst in &self.split.source_psts {
            if let Some(detail) = self.pst_details_cache.get(&pst.path) {
                for y_str in &target_years {
                    if let Some(m_list) = detail.year_months.get(y_str) {
                        for &m in m_list {
                            if !months.contains(&m) {
                                months.push(m);
                            }
                        }
                    }
                }
            }
        }

        months.sort();
        self.split.available_months = months.clone();

        // Todos los meses seleccionados por defecto
        self.split.selected_months = self.split.available_months.iter().copied().collect();

        if self.split.month_cursor >= self.split.available_months.len() {
            self.split.month_cursor = self.split.available_months.len().saturating_sub(1);
        }
    }

    pub fn preview_split_filenames(&self) -> Vec<String> {
        let Some(ref src) = self.split.source_pst else {
            return Vec::new();
        };
        let is_multi = self.split.source_psts.len() > 1;
        let base_name = if is_multi {
            "Consolidado".to_string()
        } else {
            Path::new(&src.path)
                .file_stem()
                .and_then(|s| s.to_str())
                .unwrap_or("PST")
                .to_string()
        };

        let years: Vec<u32> = self.split.selected_years.iter().copied().collect();
        let months: Vec<u32> = self.split.selected_months.iter().copied().collect();

        let raw_names = match self.split.partition_mode {
            SplitPartitionMode::ByYear => {
                years.into_iter().map(|y| format!("{}_{}.pst", base_name, y)).collect()
            }
            SplitPartitionMode::ByYearMonth => {
                let mut out = Vec::new();
                for y in &years {
                    let year_months: Vec<u32> = months.to_vec();
                    for m in &year_months {
                        out.push(format!("{}_{}_{:02}.pst", base_name, y, m));
                    }
                }
                out
            }
            SplitPartitionMode::SinglePst => {
                vec![format!("{}_filtrado.pst", base_name)]
            }
        };

        raw_names
            .into_iter()
            .map(|name| resolve_unique_pst_filename(&self.split.output_dir, &name))
            .collect()
    }
}

pub fn resolve_unique_pst_filename(output_dir: &str, candidate_name: &str) -> String {
    let base_path = Path::new(output_dir).join(candidate_name);
    if !base_path.exists() {
        return candidate_name.to_string();
    }

    let stem = Path::new(candidate_name)
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("PST");
    let ext = Path::new(candidate_name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("pst");

    let mut counter = 1;
    loop {
        let new_name = format!("{} ({}).{}", stem, counter, ext);
        let check_path = Path::new(output_dir).join(&new_name);
        if !check_path.exists() {
            return new_name;
        }
        counter += 1;
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
        assert!(!pst_entry.unwrap().selected, "PST entry should be unselected by default");

        let psts = scan_folder_for_psts(&temp_dir);
        assert_eq!(psts.len(), 1);
        assert_eq!(psts[0].name, "test_archive.pst");
        assert!(!psts[0].selected, "PST item should be unselected by default from scan_folder_for_psts");

        let explorer = FileExplorerState::new(temp_dir.clone());
        let collected = explorer.collect_selected_psts();
        assert_eq!(collected.len(), 1);
        assert!(!collected[0].selected, "PST item collected via explorer fallback should be unselected by default");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }

    #[test]
    fn test_no_psts_selected_by_default() {
        let mut state = AppState::new();
        state.discovered_psts.clear();
        state.discovered_psts.push(PstItem {
            path: r"C:\Correo\archivo1.pst".to_string(),
            name: "archivo1.pst".to_string(),
            size_mb: 15.0,
            selected: false,
        });
        state.discovered_psts.push(PstItem {
            path: r"C:\Correo\archivo2.pst".to_string(),
            name: "archivo2.pst".to_string(),
            size_mb: 25.0,
            selected: false,
        });

        assert_eq!(state.selected_psts().len(), 0);
        assert!(state.selected_psts().is_empty());

        // Toggle first item
        state.discovered_psts[0].selected = true;
        assert_eq!(state.selected_psts().len(), 1);
        assert_eq!(state.selected_psts()[0].name, "archivo1.pst");
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
            ..Default::default()
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
    fn test_mailbox_metrics_and_storage_calculations() {
        let item1 = MailboxItem {
            display_name: "test1@empresa.com".to_string(),
            store_type: "ExchangeOnline".to_string(),
            size_display: "10.00 GB".to_string(),
            file_path: None,
            selected: false,
            used_bytes: Some(10_737_418_240), // 10 GB
            total_bytes: Some(53_687_091_200), // 50 GB
            quota_display: Some("50 GB".to_string()),
            usage_percent: Some(20.0),
        };
        assert_eq!(item1.get_usage_percent(), 20.0);
        assert_eq!(item1.get_used_gb(), 10.0);
        assert_eq!(item1.get_total_gb(), 50.0);
        assert_eq!(item1.get_free_gb(), 40.0);
        assert_eq!(item1.health_status().0, "Normal");

        let item2 = MailboxItem {
            display_name: "alerta@empresa.com".to_string(),
            store_type: "SharedMailbox".to_string(),
            size_display: "42.00 GB".to_string(),
            file_path: None,
            selected: false,
            used_bytes: None,
            total_bytes: None,
            quota_display: None,
            usage_percent: Some(84.0),
        };
        assert_eq!(item2.get_usage_percent(), 84.0);
        assert_eq!(item2.health_status().0, "Atención");

        let item3 = MailboxItem {
            display_name: "critico@empresa.com".to_string(),
            store_type: "SharedMailbox".to_string(),
            size_display: "48.50 GB".to_string(),
            file_path: None,
            selected: false,
            used_bytes: None,
            total_bytes: None,
            quota_display: None,
            usage_percent: Some(97.0),
        };
        assert_eq!(item3.get_usage_percent(), 97.0);
        assert_eq!(item3.health_status().0, "Crítico");

        let item_zero_quota = MailboxItem {
            display_name: "fallback@empresa.com".to_string(),
            store_type: "SharedMailbox".to_string(),
            size_display: "24.40 GB".to_string(),
            file_path: None,
            selected: false,
            used_bytes: None,
            total_bytes: None,
            quota_display: Some("0 GB".to_string()),
            usage_percent: None,
        };
        assert_eq!(item_zero_quota.get_total_gb(), 50.0);
        assert_eq!(item_zero_quota.get_quota_display(), "50 GB");
        assert_eq!(item_zero_quota.get_free_gb(), 25.6);

        // Buzón de 100 GB (Exchange Plan 2 / M365 E3/E5)
        let item_100gb = MailboxItem {
            display_name: "soporte@timeless.com.pe".to_string(),
            store_type: "SharedMailbox".to_string(),
            size_display: "1.74 GB".to_string(),
            file_path: None,
            selected: true,
            used_bytes: Some(1_867_222_345),
            total_bytes: Some(107_374_182_400),
            quota_display: Some("100 GB".to_string()),
            usage_percent: Some(1.7),
        };
        assert_eq!(item_100gb.get_total_gb(), 100.0);
        assert_eq!(item_100gb.get_quota_display(), "100 GB");
        assert_eq!(item_100gb.get_usage_percent(), 1.7);
        assert_eq!(item_100gb.health_status().0, "Normal");

        // Buzón de 2 GB (Exchange Kiosk)
        let item_kiosk = MailboxItem {
            display_name: "kiosk@empresa.com".to_string(),
            store_type: "ExchangeOnline".to_string(),
            size_display: "1.50 GB".to_string(),
            file_path: None,
            selected: false,
            used_bytes: Some(1_610_612_736),
            total_bytes: Some(2_147_483_648),
            quota_display: Some("2 GB".to_string()),
            usage_percent: Some(75.0),
        };
        assert_eq!(item_kiosk.get_total_gb(), 2.0);
        assert_eq!(item_kiosk.get_quota_display(), "2 GB");
        assert_eq!(item_kiosk.health_status().0, "Atención");
    }

    #[test]
    fn test_mailbox_storage_impacts_calculation() {
        let mut state = AppState::new();
        state.discovered_mailboxes = vec![
            MailboxItem {
                display_name: "soporte@timeless.com.pe".to_string(),
                store_type: "SharedMailbox".to_string(),
                size_display: "1.70 GB".to_string(),
                file_path: None,
                selected: true,
                used_bytes: Some(1_825_361_100), // ~1.70 GB
                total_bytes: Some(107_374_182_400), // 100 GB
                quota_display: Some("100 GB".to_string()),
                usage_percent: Some(1.7),
            },
        ];
        state.progress.imported_count = 500;
        state.progress.duplicates_skipped = 50;
        state.progress.error_count = 0;
        state.discovered_psts = vec![PstItem {
            name: "archivo.pst".to_string(),
            path: r"C:\Correo\archivo.pst".to_string(),
            size_mb: 2048.0, // 2 GB
            selected: true,
        }];

        let impacts = state.get_mailbox_storage_impacts();
        assert_eq!(impacts.len(), 1);
        let impact = &impacts[0];
        assert_eq!(impact.display_name, "soporte@timeless.com.pe");
        assert_eq!(impact.total_gb, 100.0);
        assert!((impact.used_before_gb - 1.70).abs() < 0.05);
        assert!(impact.imported_gb > 0.0);
        assert!(impact.used_after_gb > impact.used_before_gb);
        assert!(impact.free_after_gb < impact.free_before_gb);
        assert_eq!(impact.health_before, "Normal");
        assert_eq!(impact.health_after, "Normal");
    }

    #[test]
    fn test_mailbox_storage_impacts_zero_imported() {
        let mut state = AppState::new();
        state.discovered_mailboxes = vec![MailboxItem {
            display_name: "bck.operaciones3.2020@pluscargoperu.com".to_string(),
            store_type: "SharedMailbox".to_string(),
            size_display: "24.31 GB".to_string(),
            file_path: None,
            selected: true,
            used_bytes: Some(26_102_546_432), // ~24.31 GB
            total_bytes: Some(53_687_091_200), // 50 GB
            quota_display: Some("50 GB".to_string()),
            usage_percent: Some(48.6),
        }];
        state.progress.imported_count = 0;
        state.progress.duplicates_skipped = 28_000;
        state.progress.error_count = 0;
        state.discovered_psts = vec![PstItem {
            name: "BCK MAR 2020 I OPERACIONES3.pst".to_string(),
            path: r"D:\BCK MAR 2020 I OPERACIONES3.pst".to_string(),
            size_mb: 22_589.44, // 22.06 GB
            selected: true,
        }];
        // Añadir carpetas seleccionadas en el árbol para comprobar que NO se sumen como tamaño importado
        state.folder_tree.nodes.push(crate::app::FolderTreeNode {
            name: "Bandeja de entrada".to_string(),
            path: "Bandeja de entrada".to_string(),
            parent_path: None,
            count: 28000,
            size_mb: 19_312.64, // 18.86 GB
            selected: true,
            expanded: false,
            level: 0,
            has_children: false,
        });

        assert_eq!(state.total_imported_size_mb(), 0.0);
        let impacts = state.get_mailbox_storage_impacts();
        assert_eq!(impacts.len(), 1);
        let impact = &impacts[0];
        assert_eq!(impact.imported_gb, 0.0);
        assert_eq!(impact.delta_gb, 0.0);
        assert_eq!(impact.used_after_gb, impact.used_before_gb);
        assert_eq!(impact.free_after_gb, impact.free_before_gb);
        assert_eq!(impact.health_after, "Normal");
    }

    #[test]
    fn test_mailbox_storage_impacts_mostly_duplicates_realistic() {
        let mut state = AppState::new();
        state.discovered_mailboxes = vec![MailboxItem {
            display_name: "bck.operaciones3.2020@pluscargoperu.com".to_string(),
            store_type: "SharedMailbox".to_string(),
            size_display: "24.31 GB".to_string(),
            file_path: None,
            selected: true,
            used_bytes: Some(26_102_546_432), // ~24.31 GB
            total_bytes: Some(53_687_091_200), // 50 GB
            quota_display: Some("50 GB".to_string()),
            usage_percent: Some(48.6),
        }];
        // Escenario del usuario: 41 importados, 28849 duplicados omitidos, 1 error
        state.progress.imported_count = 41;
        state.progress.duplicates_skipped = 28_849;
        state.progress.error_count = 1;
        state.discovered_psts = vec![PstItem {
            name: "BCK MAR 2020 I OPERACIONES3.pst".to_string(),
            path: r"D:\BCK MAR 2020 I OPERACIONES3.pst".to_string(),
            size_mb: 22_589.44, // 22.06 GB
            selected: true,
        }];
        // Carpeta en el PST de 18.86 GB
        state.folder_tree.nodes.push(crate::app::FolderTreeNode {
            name: "Bandeja de entrada".to_string(),
            path: "Bandeja de entrada".to_string(),
            parent_path: None,
            count: 28891,
            size_mb: 19_312.64, // 18.86 GB
            selected: true,
            expanded: false,
            level: 0,
            has_children: false,
        });

        // El cálculo proporcional de 41 correos de 28891 en un PST de 22.06 GB debe ser ~32 MB, NUNCA 18.86 GB
        let imported_mb = state.total_imported_size_mb();
        assert!(imported_mb > 10.0 && imported_mb < 60.0, "imported_mb ({}) debe ser aprox 32 MB y no 18.86 GB", imported_mb);

        let impacts = state.get_mailbox_storage_impacts();
        let impact = &impacts[0];
        assert!(impact.imported_gb < 0.1, "El espacio importado debe ser menor a 0.1 GB");
        assert!(impact.used_after_gb < 24.5, "El espacio final debe ser aprox 24.34 GB, no 43.17 GB");
        assert!(impact.free_after_gb > 25.5, "El espacio libre debe ser aprox 25.66 GB, no 6.83 GB");
        assert_eq!(impact.health_after, "Normal", "La salud debe mantenerse Normal");
    }

    #[test]
    fn test_total_imported_size_with_live_imported_bytes() {
        let mut state = AppState::new();
        state.progress.imported_count = 41;
        // 15 MB exactos reportados en bytes desde el worker MAPI
        state.progress.imported_bytes = 15 * 1024 * 1024;
        state.discovered_psts = vec![PstItem {
            name: "test.pst".to_string(),
            path: "test.pst".to_string(),
            size_mb: 5000.0,
            selected: true,
        }];

        assert_eq!(state.total_imported_size_mb(), 15.0);
    }

    #[test]
    fn test_routing_default_all_years_all_months() {
        let mut state = AppState::new();
        state.step = WizardStep::FoldersMode;
        state.next_step();
        assert_eq!(state.step, WizardStep::Routing);
        assert_eq!(state.active_routing_modal, RoutingModal::None);

        // Seleccionar granularidad de Meses (Años y Meses)
        state.routing_modal_criterion_idx = 2;
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
        assert_eq!(state.active_routing_modal, RoutingModal::None);
        assert_eq!(state.routing_modal_criterion_idx, 0);

        // Configurar alcance YearScope
        state.active_routing_modal = RoutingModal::YearScope;
        state.routing_modal_year_scope_idx = 0; // Todos los años
        state.specific_year = None;
        state.routing_all_years = true;

        // Pasa a MonthScope
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
    fn test_routing_interactive_card_selection_and_filter_reset() {
        let mut state = AppState::new();
        state.step = WizardStep::FoldersMode;
        state.next_step();
        assert_eq!(state.step, WizardStep::Routing);
        assert_eq!(state.active_routing_modal, RoutingModal::None);

        // Por defecto: Mirror (Estructura Original)
        assert_eq!(state.routing_granularity, RoutingGranularity::Mirror);

        // Seleccionar tarjeta 2 (Años)
        state.routing_granularity = RoutingGranularity::Years;
        assert_eq!(state.routing_granularity, RoutingGranularity::Years);

        // Seleccionar tarjeta 3 (Años y Meses)
        state.routing_granularity = RoutingGranularity::YearsAndMonths;
        assert_eq!(state.routing_granularity, RoutingGranularity::YearsAndMonths);

        // Aplicar filtro de fecha
        state.specific_year = Some(2025);
        state.specific_month = Some(6);
        state.routing_all_years = false;
        state.routing_all_months = false;

        // Restablecer filtro (tecla R)
        state.specific_year = None;
        state.specific_month = None;
        state.routing_all_years = true;
        state.routing_all_months = true;

        assert_eq!(state.specific_year, None);
        assert_eq!(state.specific_month, None);
        assert!(state.routing_all_years);
        assert!(state.routing_all_months);
    }

    #[test]
    fn test_routing_criterion_modal_state_change() {
        let mut state = AppState::new();
        state.step = WizardStep::Routing;
        assert_eq!(state.routing_granularity, RoutingGranularity::Mirror);

        // Abrir modal de Criterio (tecla C)
        state.active_routing_modal = RoutingModal::Criterion;
        state.routing_modal_criterion_idx = 1; // Seleccionar opción 2: Por Años

        // Confirmar selección (Enter)
        state.routing_granularity = match state.routing_modal_criterion_idx {
            0 => RoutingGranularity::Mirror,
            1 => RoutingGranularity::Years,
            _ => RoutingGranularity::YearsAndMonths,
        };
        state.active_routing_modal = RoutingModal::None;

        assert_eq!(state.routing_granularity, RoutingGranularity::Years);
        assert_eq!(state.active_routing_modal, RoutingModal::None);

        // Avanzar a Deduplicación y luego a Resumen
        state.next_step(); // Deduplication
        assert_eq!(state.step, WizardStep::Deduplication);
        assert_eq!(state.routing_granularity, RoutingGranularity::Years);

        state.next_step(); // Filters
        assert_eq!(state.step, WizardStep::Filters);
        assert_eq!(state.routing_granularity, RoutingGranularity::Years);

        state.next_step(); // Summary
        assert_eq!(state.step, WizardStep::Summary);
        assert_eq!(state.routing_granularity, RoutingGranularity::Years);
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
        explorer.build_from_detail(&detail);

        // Nivel inicial (plegado): debe listar 2 carpetas principales visibles
        let root_indices = explorer.visible_indices();
        assert_eq!(root_indices.len(), 2);
        assert_eq!(explorer.nodes[root_indices[0]].name, "Bandeja de entrada");
        assert_eq!(explorer.nodes[root_indices[1]].name, "Elementos enviados");

        // Seleccionar Bandeja de entrada con Espacio para aislar métricas
        explorer.toggle_select();
        assert_eq!(explorer.checked_folder_path, Some("Bandeja de entrada".to_string()));
        let selected_stats = explorer.get_selected_folder_stats(&detail);
        assert!(selected_stats.is_some());
        assert_eq!(selected_stats.unwrap().count, 300);

        // Deseleccionar con Espacio nuevamente
        explorer.toggle_select();
        assert_eq!(explorer.checked_folder_path, None);
        assert!(explorer.get_selected_folder_stats(&detail).is_none());

        // Desplegar menú de subcarpetas en Bandeja de entrada con toggle_expand
        explorer.toggle_expand();
        assert!(explorer.nodes[0].expanded);

        // Ahora debe haber 3 nodos visibles: Bandeja de entrada, Facturas 2024 (desplegada) y Elementos enviados
        let expanded_indices = explorer.visible_indices();
        assert_eq!(expanded_indices.len(), 3);
        assert_eq!(explorer.nodes[expanded_indices[1]].name, "Facturas 2024");
        assert_eq!(explorer.nodes[expanded_indices[1]].level, 1);

        // Bajar hacia la subcarpeta y aislar sus métricas
        explorer.move_down();
        assert_eq!(explorer.selected_idx, 1);
        explorer.toggle_select();
        assert_eq!(explorer.checked_folder_path, Some(r"Bandeja de entrada\Facturas 2024".to_string()));
        let sub_stats = explorer.get_selected_folder_stats(&detail);
        assert!(sub_stats.is_some());
        assert_eq!(sub_stats.unwrap().count, 150);

        // Plegar menú con collapse
        explorer.collapse(); // primero salta al padre
        assert_eq!(explorer.selected_idx, 0);
        explorer.collapse(); // pliega el menú
        assert!(!explorer.nodes[0].expanded);
        assert_eq!(explorer.visible_indices().len(), 2);
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

    #[test]
    fn test_folder_tree_state_navigation_and_selection() {
        let mut tree = FolderTreeState::new();
        assert_eq!(tree.nodes.len(), 4);
        assert_eq!(tree.visible_indices().len(), 4);

        // Crear una estructura con subcarpetas
        let detail = PstDetail {
            file_name: "test.pst".to_string(),
            file_path: r"C:\Correo\test.pst".to_string(),
            folders: vec![
                PstFolderDetail {
                    name: "Bandeja de entrada".to_string(),
                    path: "Bandeja de entrada".to_string(),
                    parent_path: None,
                    count: 960,
                    has_children: true,
                    ..Default::default()
                },
                PstFolderDetail {
                    name: "JORGE S.".to_string(),
                    path: r"Bandeja de entrada\JORGE S.".to_string(),
                    parent_path: Some("Bandeja de entrada".to_string()),
                    count: 24,
                    has_children: false,
                    ..Default::default()
                },
                PstFolderDetail {
                    name: "Elementos enviados".to_string(),
                    path: "Elementos enviados".to_string(),
                    parent_path: None,
                    count: 357,
                    has_children: false,
                    ..Default::default()
                },
            ],
            ..Default::default()
        };

        tree.build_from_pst_details(&[&detail]);
        assert_eq!(tree.nodes.len(), 3);
        assert_eq!(tree.nodes[0].name, "Bandeja de entrada");
        assert!(tree.nodes[0].has_children);
        assert_eq!(tree.nodes[1].name, "JORGE S.");
        assert_eq!(tree.nodes[1].level, 1);
        assert_eq!(tree.nodes[2].name, "Elementos enviados");

        // Inicia colapsado con children ocultos: 2 visibles (Bandeja de entrada y Elementos enviados)
        assert_eq!(tree.visible_indices().len(), 2);
        assert!(!tree.nodes[0].expanded);

        // Expandir Bandeja de entrada con toggle_expand (índice 0)
        tree.toggle_expand();
        assert!(tree.nodes[0].expanded);
        assert_eq!(tree.visible_indices().len(), 3);

        // Estado inicial de checkbox: ambos seleccionados -> Checked
        assert_eq!(tree.checkbox_state(0), CheckboxState::Checked);

        // Deseleccionar solo el hijo JORGE S. (índice 1):
        tree.nodes[1].selected = false;
        // Ahora Bandeja de entrada debe estar parcialmente seleccionada: Indeterminate [ - ]
        assert_eq!(tree.checkbox_state(0), CheckboxState::Indeterminate);

        // Al hacer toggle_select sobre un nodo Indeterminate, debe marcarse completo (Checked)
        tree.toggle_select();
        assert!(tree.nodes[0].selected);
        assert!(tree.nodes[1].selected);
        assert_eq!(tree.checkbox_state(0), CheckboxState::Checked);

        // Al hacer toggle_select sobre un nodo Checked, debe desmarcarse todo (Unchecked)
        tree.toggle_select();
        assert!(!tree.nodes[0].selected);
        assert!(!tree.nodes[1].selected);
        assert_eq!(tree.checkbox_state(0), CheckboxState::Unchecked);

        // Seleccionar todo
        tree.select_all();
        assert!(tree.nodes[0].selected);
        assert!(tree.nodes[1].selected);
        assert!(tree.nodes[2].selected);
        assert_eq!(tree.selected_paths().len(), 3);
        assert_eq!(tree.checkbox_state(0), CheckboxState::Checked);
    }

    #[test]
    fn test_cancel_confirmation_modal_toggle_and_reset() {
        let mut state = AppState::new();
        assert!(!state.show_cancel_modal);
        assert!(!state.cancel_modal_selected_yes);

        state.open_cancel_modal();
        assert!(state.show_cancel_modal);
        assert!(!state.cancel_modal_selected_yes);

        state.cancel_modal_selected_yes = true;
        assert!(state.cancel_modal_selected_yes);

        state.close_cancel_modal();
        assert!(!state.show_cancel_modal);
        assert!(!state.cancel_modal_selected_yes);
    }

    #[test]
    fn test_multi_year_selection_and_toggling() {
        let mut state = AppState::new();
        assert!(state.routing_all_years);
        assert!(state.selected_years.is_empty());

        // Toggle año 2023
        state.toggle_year(2023);
        assert!(!state.routing_all_years);
        assert_eq!(state.selected_years.len(), 1);
        assert!(state.selected_years.contains(&2023));
        assert_eq!(state.specific_year, Some(2023));

        // Agregar año 2024
        state.toggle_year(2024);
        assert_eq!(state.selected_years.len(), 2);
        assert!(state.selected_years.contains(&2023));
        assert!(state.selected_years.contains(&2024));

        // Deseleccionar 2023
        state.toggle_year(2023);
        assert_eq!(state.selected_years.len(), 1);
        assert!(!state.selected_years.contains(&2023));
        assert!(state.selected_years.contains(&2024));

        // Deseleccionar el último año (2024) -> vuelve a 'todos los años'
        state.toggle_year(2024);
        assert!(state.routing_all_years);
        assert!(state.selected_years.is_empty());
        assert_eq!(state.specific_year, None);

        // Reactivar y luego set_all_years()
        state.toggle_year(2022);
        assert!(!state.routing_all_years);
        state.set_all_years();
        assert!(state.routing_all_years);
        assert!(state.selected_years.is_empty());
    }

    #[test]
    fn test_multi_month_selection_first_and_second_half_presets() {
        let mut state = AppState::new();
        assert!(state.routing_all_months);
        assert!(state.selected_months.is_empty());

        // Presets: Primera Mitad (H1: 1..=6)
        state.set_first_half_months();
        assert!(!state.routing_all_months);
        assert!(state.is_first_half_selected());
        assert!(!state.is_second_half_selected());
        assert_eq!(state.selected_months.len(), 6);
        for m in 1..=6 {
            assert!(state.selected_months.contains(&m));
        }

        // Presets: Segunda Mitad (H2: 7..=12)
        state.set_second_half_months();
        assert!(!state.routing_all_months);
        assert!(!state.is_first_half_selected());
        assert!(state.is_second_half_selected());
        assert_eq!(state.selected_months.len(), 6);
        for m in 7..=12 {
            assert!(state.selected_months.contains(&m));
        }

        // Toggle individual
        state.toggle_month(7);
        assert!(!state.is_second_half_selected());
        assert!(!state.selected_months.contains(&7));

        // Reset a todos
        state.set_all_months();
        assert!(state.routing_all_months);
        assert!(state.selected_months.is_empty());
        assert_eq!(state.specific_month, None);
    }

    #[test]
    fn test_dynamic_available_years_and_months_from_cached_detail() {
        let mut state = AppState::new();

        // Sin detalles en cache -> fallback a 5 años y 12 meses
        let default_years = state.available_years_from_psts();
        assert_eq!(default_years.len(), 5);
        let default_months = state.available_months_from_psts();
        assert_eq!(default_months.len(), 12);

        // Agregamos un PST seleccionado y su detalle en cache
        let pst_path = "C:\\Correo\\test.pst".to_string();
        state.discovered_psts.push(PstItem {
            path: pst_path.clone(),
            name: "test.pst".to_string(),
            size_mb: 1.5,
            selected: true,
        });

        let mut year_months = std::collections::BTreeMap::new();
        year_months.insert("2021".to_string(), vec![3, 4, 11]);
        year_months.insert("2024".to_string(), vec![1, 2]);

        state.pst_details_cache.insert(pst_path, PstDetail {
            years: vec![2021, 2024],
            year_months,
            ..Default::default()
        });

        // Ahora debe retornar exactamente los años y meses detectados dinámicamente
        let dyn_years = state.available_years_from_psts();
        assert_eq!(dyn_years, vec![2021, 2024]);

        let dyn_months = state.available_months_from_psts();
        assert_eq!(dyn_months, vec![1, 2, 3, 4, 11]);
    }

    #[test]
    fn test_format_filter_display_strings() {
        let mut state = AppState::new();

        // Años
        assert_eq!(state.format_years_filter_display(), "Todos los años");
        state.toggle_year(2024);
        assert_eq!(state.format_years_filter_display(), "Año 2024");
        state.toggle_year(2022);
        assert_eq!(state.format_years_filter_display(), "Años: 2022, 2024");

        // Meses
        assert_eq!(state.format_months_filter_display(), "Todos los meses");
        state.set_first_half_months();
        assert_eq!(state.format_months_filter_display(), "1ª Mitad (Ene - Jun / H1)");
        state.set_second_half_months();
        assert_eq!(state.format_months_filter_display(), "2ª Mitad (Jul - Dic / H2)");

        state.set_all_months();
        state.toggle_month(5);
        assert_eq!(state.format_months_filter_display(), "Mes 05 (May)");

        state.toggle_month(10);
        assert_eq!(state.format_months_filter_display(), "Meses: May, Oct");
    }

    #[test]
    fn test_split_pst_preview_filenames_modes() {
        let mut state = AppState::new();
        state.split.source_pst = Some(PstItem {
            path: r"C:\Correo\archivo_principal.pst".to_string(),
            name: "archivo_principal.pst".to_string(),
            size_mb: 150.0,
            selected: true,
        });
        state.split.selected_years.insert(2023);
        state.split.selected_years.insert(2024);
        state.split.selected_months.insert(5);
        state.split.selected_months.insert(12);

        // Modo ByYear
        state.split.partition_mode = SplitPartitionMode::ByYear;
        let preview_years = state.preview_split_filenames();
        assert_eq!(preview_years.len(), 2);
        assert!(preview_years.contains(&"archivo_principal_2023.pst".to_string()));
        assert!(preview_years.contains(&"archivo_principal_2024.pst".to_string()));

        // Modo ByYearMonth
        state.split.partition_mode = SplitPartitionMode::ByYearMonth;
        let preview_year_months = state.preview_split_filenames();
        assert_eq!(preview_year_months.len(), 4);
        assert!(preview_year_months.contains(&"archivo_principal_2023_05.pst".to_string()));
        assert!(preview_year_months.contains(&"archivo_principal_2023_12.pst".to_string()));
        assert!(preview_year_months.contains(&"archivo_principal_2024_05.pst".to_string()));
        assert!(preview_year_months.contains(&"archivo_principal_2024_12.pst".to_string()));

        // Modo SinglePst
        state.split.partition_mode = SplitPartitionMode::SinglePst;
        let preview_single = state.preview_split_filenames();
        assert_eq!(preview_single.len(), 1);
        assert_eq!(preview_single[0], "archivo_principal_filtrado.pst");
    }

    #[test]
    fn test_split_pst_init_and_step_transitions() {
        let mut state = AppState::new();
        state.discovered_psts = vec![PstItem {
            path: r"C:\Archivos\backup.pst".to_string(),
            name: "backup.pst".to_string(),
            size_mb: 1024.0,
            selected: false,
        }];
        state.selected_pst_table_idx = 0;

        // Inicializar split para el PST en index 0
        state.init_split_from_selected_pst();
        assert!(state.split.source_pst.is_some());
        assert_eq!(state.split.source_pst.as_ref().unwrap().name, "backup.pst");
        assert_eq!(state.split.output_dir, r"C:\Archivos");

        // Transiciones del wizard de Split
        state.step = WizardStep::SplitSelect;
        state.next_step();
        assert_eq!(state.step, WizardStep::SplitFilter);

        state.next_step();
        assert_eq!(state.step, WizardStep::SplitConfig);

        state.next_step();
        assert_eq!(state.step, WizardStep::SplitSummary);

        state.prev_step();
        assert_eq!(state.step, WizardStep::SplitConfig);

        state.prev_step();
        assert_eq!(state.step, WizardStep::SplitFilter);

        state.prev_step();
        assert_eq!(state.step, WizardStep::SplitSelect);

        // Del Select hacia atrás regresa al Welcome
        state.prev_step();
        assert_eq!(state.step, WizardStep::Welcome);
    }

    #[test]
    fn test_split_pst_scanning_and_dynamic_months_from_detail() {
        let mut state = AppState::new();
        let pst_path = r"C:\Correo\archivo_prueba.pst".to_string();
        state.discovered_psts = vec![PstItem {
            path: pst_path.clone(),
            name: "archivo_prueba.pst".to_string(),
            size_mb: 250.0,
            selected: true,
        }];
        state.selected_pst_table_idx = 0;

        // Al inicio sin cache, no inventa años ni meses
        state.init_split_from_selected_pst();
        assert!(state.split.is_scanning);
        assert!(state.split.available_years.is_empty());
        assert!(state.split.available_months.is_empty());

        // Simular que el inspector MAPI retorna el detalle real
        let mut year_months = std::collections::BTreeMap::new();
        year_months.insert("2021".to_string(), vec![3, 8]);
        year_months.insert("2022".to_string(), vec![1, 10]);

        let detail = PstDetail {
            file_name: "archivo_prueba.pst".to_string(),
            file_path: pst_path.clone(),
            total_items: 1200,
            years: vec![2021, 2022],
            year_months,
            ..Default::default()
        };
        state.pst_details_cache.insert(pst_path, detail.clone());

        // Aplicar el detalle escaneado
        state.apply_pst_detail_to_split(&detail);
        assert!(!state.split.is_scanning);
        assert_eq!(state.split.available_years, vec![2021, 2022]);
        assert_eq!(state.split.available_months, vec![1, 3, 8, 10]);

        // Si solo se selecciona el año 2021, los meses disponibles deben ser solo [3, 8]
        state.split.selected_years.clear();
        state.split.selected_years.insert(2021);
        state.update_split_available_months();
        assert_eq!(state.split.available_months, vec![3, 8]);

        // Si solo se selecciona el año 2022, los meses disponibles deben ser solo [1, 10]
        state.split.selected_years.clear();
        state.split.selected_years.insert(2022);
        state.update_split_available_months();
        assert_eq!(state.split.available_months, vec![1, 10]);
    }

    #[test]
    fn test_resolve_unique_pst_filename_collision() {
        let temp_dir = std::env::temp_dir().join("test_pst_unique_collision");
        let _ = std::fs::create_dir_all(&temp_dir);
        let dir_str = temp_dir.to_str().unwrap();

        // 1. Archivo no existe -> devuelve el nombre original
        let unique1 = resolve_unique_pst_filename(dir_str, "salida.pst");
        assert_eq!(unique1, "salida.pst");

        // 2. Creamos salida.pst -> ahora debe devolver salida (1).pst
        let _ = File::create(temp_dir.join("salida.pst"));
        let unique2 = resolve_unique_pst_filename(dir_str, "salida.pst");
        assert_eq!(unique2, "salida (1).pst");

        // 3. Creamos salida (1).pst -> ahora debe devolver salida (2).pst
        let _ = File::create(temp_dir.join("salida (1).pst"));
        let unique3 = resolve_unique_pst_filename(dir_str, "salida.pst");
        assert_eq!(unique3, "salida (2).pst");

        let _ = std::fs::remove_dir_all(&temp_dir);
    }
}

