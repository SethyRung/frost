use crate::theme::builtin::DEFAULT_THEME_ID;
use crate::theme::registry::ThemeRegistry;
use crate::theme::resolver::resolve_theme_safe;
use crate::theme::types::{ResolvedTheme, ThemeJson, ThemeMode};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tokio::sync::broadcast;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedThemeState {
    pub active: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<ThemeMode>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub lock: Option<ThemeMode>,
}

pub struct ThemeStore {
    registry: ThemeRegistry,
    active: String,
    mode: ThemeMode,
    lock: Option<ThemeMode>,
    resolved_cache: HashMap<(String, ThemeMode), ResolvedTheme>,
    persist_path: Option<PathBuf>,
    notify_tx: broadcast::Sender<()>,
}

impl ThemeStore {
    pub fn new(registry: ThemeRegistry) -> Self {
        let (notify_tx, _) = broadcast::channel(16);
        Self {
            registry,
            active: DEFAULT_THEME_ID.to_string(),
            mode: ThemeMode::Dark,
            lock: None,
            resolved_cache: HashMap::new(),
            persist_path: None,
            notify_tx,
        }
    }

    pub fn with_persist_path(mut self, path: impl AsRef<Path>) -> Self {
        self.persist_path = Some(path.as_ref().to_path_buf());
        self
    }

    pub fn load_persisted(&mut self) {
        if let Some(path) = &self.persist_path {
            if let Ok(text) = std::fs::read_to_string(path) {
                if let Ok(state) = serde_json::from_str::<PersistedThemeState>(&text) {
                    if self.registry.has(&state.active) {
                        self.active = state.active;
                    }
                    if let Some(mode) = state.lock {
                        self.lock = Some(mode);
                        self.mode = mode;
                    } else if let Some(mode) = state.mode {
                        self.mode = mode;
                    }
                }
            }
        }
    }

    pub fn get_active(&self) -> &str {
        &self.active
    }

    pub fn set(&mut self, id: &str) {
        if !self.registry.has(id) {
            return;
        }
        if self.active == id {
            return;
        }
        self.active = id.to_string();
        self.invalidate_cache();
        self.notify();
        self.persist();
    }

    pub fn get_mode(&self) -> ThemeMode {
        self.mode
    }

    pub fn set_mode(&mut self, mode: ThemeMode) {
        if self.mode == mode {
            return;
        }
        self.mode = mode;
        self.invalidate_cache();
        self.notify();
        if self.lock.is_some() {
            self.persist();
        }
    }

    pub fn get_lock(&self) -> Option<ThemeMode> {
        self.lock
    }

    pub fn lock_mode(&mut self, mode: ThemeMode) {
        self.lock = Some(mode);
        self.mode = mode;
        self.invalidate_cache();
        self.notify();
        self.persist();
    }

    pub fn unlock(&mut self) {
        if self.lock.is_none() {
            return;
        }
        self.lock = None;
        self.notify();
        self.persist();
    }

    pub fn get_all(&self) -> &HashMap<String, ThemeJson> {
        self.registry.get_all()
    }

    pub fn get_theme(&self, id: &str) -> Option<&ThemeJson> {
        self.registry.get(id)
    }

    pub fn get_resolved(&mut self) -> Option<&ResolvedTheme> {
        let cache_key = (self.active.clone(), self.mode);
        if !self.resolved_cache.contains_key(&cache_key) {
            let theme = self.registry.get(&self.active)?;
            let resolved = resolve_theme_safe(theme, self.mode)?;
            self.resolved_cache.insert(cache_key.clone(), resolved);
        }
        self.resolved_cache.get(&cache_key)
    }

    pub fn subscribe(&self) -> broadcast::Receiver<()> {
        self.notify_tx.subscribe()
    }

    fn invalidate_cache(&mut self) {
        self.resolved_cache.clear();
    }

    fn notify(&self) {
        let _ = self.notify_tx.send(());
    }

    fn persist(&self) {
        if let Some(path) = &self.persist_path {
            let state = PersistedThemeState {
                active: self.active.clone(),
                mode: if self.lock.is_some() {
                    Some(self.mode)
                } else {
                    None
                },
                lock: self.lock,
            };
            if let Ok(json) = serde_json::to_string_pretty(&state) {
                let _ = std::fs::create_dir_all(path.parent().unwrap_or(path));
                let _ = std::fs::write(path, json);
            }
        }
    }
}
