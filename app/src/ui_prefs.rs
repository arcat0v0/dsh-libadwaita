use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveFilter {
    Active,
    Archived,
    All,
}

impl Default for ArchiveFilter {
    fn default() -> Self {
        Self::Active
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TranscriptView {
    Concise,
    Standard,
    Detailed,
    Expanded,
}

impl Default for TranscriptView {
    fn default() -> Self {
        Self::Standard
    }
}

impl TranscriptView {
    pub fn groups_intermediate(self) -> bool {
        self != Self::Expanded
    }

    pub fn expands_groups(self) -> bool {
        self == Self::Detailed
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UiPrefs {
    #[serde(default)]
    pub pinned_sessions: Vec<String>,
    #[serde(default)]
    pub archive_filter: ArchiveFilter,
    #[serde(default = "default_true")]
    pub notifications_enabled: bool,
    #[serde(default)]
    pub theme: Option<String>,
    #[serde(default)]
    pub font_size: Option<u32>,
    #[serde(default = "default_true")]
    pub developer_tools: bool,
    #[serde(default)]
    pub transcript_view: TranscriptView,
    #[serde(default)]
    pub plugin_registry: Option<String>,
    #[serde(default)]
    pub dock_tabs: HashMap<String, usize>,
    #[serde(default)]
    pub dock_visible: HashMap<String, bool>,
}

fn default_true() -> bool {
    true
}

impl Default for UiPrefs {
    fn default() -> Self {
        Self {
            pinned_sessions: Vec::new(),
            archive_filter: ArchiveFilter::Active,
            notifications_enabled: true,
            theme: None,
            font_size: None,
            developer_tools: true,
            transcript_view: TranscriptView::Standard,
            plugin_registry: None,
            dock_tabs: HashMap::new(),
            dock_visible: HashMap::new(),
        }
    }
}

impl UiPrefs {
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|raw| serde_json::from_str(&raw).ok())
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let raw = serde_json::to_string_pretty(self).unwrap_or_else(|_| "{}".to_string());
        std::fs::write(path, raw)
    }

    pub fn toggle_pin(&mut self, session_id: &str) {
        if let Some(index) = self
            .pinned_sessions
            .iter()
            .position(|id| id == session_id)
        {
            self.pinned_sessions.remove(index);
        } else {
            self.pinned_sessions.push(session_id.to_string());
        }
    }

    pub fn is_pinned(&self, session_id: &str) -> bool {
        self.pinned_sessions.iter().any(|id| id == session_id)
    }

    pub fn pinned_set(&self) -> std::collections::HashSet<String> {
        self.pinned_sessions.iter().cloned().collect()
    }

    pub fn dock_tab(&self, session_id: &str) -> Option<usize> {
        self.dock_tabs.get(session_id).copied()
    }

    pub fn set_dock_tab(&mut self, session_id: &str, index: usize) {
        self.dock_tabs.insert(session_id.to_string(), index);
    }

    pub fn dock_visible(&self, session_id: &str) -> bool {
        self.dock_visible.get(session_id).copied().unwrap_or(true)
    }

    pub fn set_dock_visible(&mut self, session_id: &str, visible: bool) {
        self.dock_visible.insert(session_id.to_string(), visible);
    }
}

pub fn default_ui_prefs_path() -> PathBuf {
    let base = match std::env::var("XDG_CONFIG_HOME") {
        Ok(value) if !value.trim().is_empty() => PathBuf::from(value),
        _ => {
            let home = std::env::var("HOME").unwrap_or_else(|_| ".".to_string());
            PathBuf::from(home).join(".config")
        }
    };
    base.join("dsh-libadwaita").join("ui-prefs.json")
}
