use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};

pub const CURRENT_VERSION: u32 = 1;
pub const STATE_DIR: &str = ".frost";
pub const STATE_FILE: &str = ".frost/state.json";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppState {
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostState {
    pub version: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub active_theme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub theme_mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_project: Option<String>,
    #[serde(default)]
    pub expanded_apps: Vec<String>,
    #[serde(default)]
    pub apps: HashMap<String, AppState>,
}

impl Default for FrostState {
    fn default() -> Self {
        Self {
            version: CURRENT_VERSION,
            active_theme: None,
            theme_mode: None,
            last_project: None,
            expanded_apps: Vec::new(),
            apps: HashMap::new(),
        }
    }
}

pub struct StateStore {
    state: FrostState,
    save_timer: Option<tokio::time::Instant>,
    persist_path: PathBuf,
}

impl StateStore {
    pub fn new() -> Self {
        let persist_path = Self::default_path();
        Self {
            state: FrostState::default(),
            save_timer: None,
            persist_path,
        }
    }

    pub fn with_path(mut self, path: impl AsRef<Path>) -> Self {
        self.persist_path = path.as_ref().to_path_buf();
        self
    }

    pub fn load(&mut self) {
        if let Ok(text) = std::fs::read_to_string(&self.persist_path) {
            if let Ok(parsed) = serde_json::from_str::<FrostState>(&text) {
                self.state = parsed;
            }
        }
    }

    pub fn save_now(&self) -> std::io::Result<()> {
        let dir = self
            .persist_path
            .parent()
            .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad path"))?;
        std::fs::create_dir_all(dir)?;
        let json = serde_json::to_string_pretty(&self.state)?;
        std::fs::write(&self.persist_path, json)
    }

    pub fn request_save(&mut self) {
        self.save_timer = Some(tokio::time::Instant::now());
        // In a real async context, a task would poll this.
        // For simplicity in core, we save immediately on request.
        let _ = self.save_now();
    }

    pub fn state(&self) -> &FrostState {
        &self.state
    }

    pub fn state_mut(&mut self) -> &mut FrostState {
        self.save_timer = Some(tokio::time::Instant::now());
        &mut self.state
    }

    pub fn set_last_project(&mut self, project: impl Into<String>) {
        self.state.last_project = Some(project.into());
        self.request_save();
    }

    pub fn set_expanded_apps(&mut self, apps: Vec<String>) {
        self.state.expanded_apps = apps;
        self.request_save();
    }

    pub fn update_app(&mut self, app_id: impl Into<String>, update: AppState) {
        self.state.apps.insert(app_id.into(), update);
        self.request_save();
    }

    fn default_path() -> PathBuf {
        dirs::home_dir()
            .map(|h| h.join(STATE_FILE))
            .unwrap_or_else(|| PathBuf::from(STATE_FILE))
    }
}

impl Default for StateStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_state_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("state.json");

        let mut store = StateStore::with_path(StateStore::new(), &path);
        store.state.active_theme = Some("dracula".to_string());
        store.state.last_project = Some("portfolio".to_string());
        store.state.expanded_apps = vec!["portfolio/frontend".to_string()];
        store.save_now().unwrap();

        let mut store2 = StateStore::with_path(StateStore::new(), &path);
        store2.load();
        assert_eq!(store2.state.active_theme, Some("dracula".to_string()));
        assert_eq!(store2.state.last_project, Some("portfolio".to_string()));
        assert_eq!(
            store2.state.expanded_apps,
            vec!["portfolio/frontend".to_string()]
        );
    }
}
