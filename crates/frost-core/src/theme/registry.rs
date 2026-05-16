use crate::theme::builtin::builtin_themes;
use crate::theme::types::ThemeJson;
use std::collections::HashMap;

pub struct ThemeRegistry {
    themes: HashMap<String, ThemeJson>,
    system_theme: Option<ThemeJson>,
}

impl ThemeRegistry {
    pub fn new() -> Self {
        Self {
            themes: builtin_themes(),
            system_theme: None,
        }
    }

    pub fn with_builtin_themes() -> Self {
        Self {
            themes: builtin_themes(),
            system_theme: None,
        }
    }

    pub fn get(&self, id: &str) -> Option<&ThemeJson> {
        if id == "system" {
            self.system_theme.as_ref()
        } else {
            self.themes.get(id)
        }
    }

    pub fn has(&self, id: &str) -> bool {
        if id == "system" {
            self.system_theme.is_some()
        } else {
            self.themes.contains_key(id)
        }
    }

    pub fn get_all(&self) -> &HashMap<String, ThemeJson> {
        &self.themes
    }

    pub fn get_ids(&self) -> Vec<String> {
        let mut ids: Vec<_> = self.themes.keys().cloned().collect();
        if self.system_theme.is_some() {
            ids.push("system".to_string());
        }
        ids.sort();
        ids
    }

    pub fn merge(&mut self, themes: HashMap<String, ThemeJson>) {
        self.themes.extend(themes);
    }

    pub fn upsert(&mut self, id: String, theme: ThemeJson) {
        self.themes.insert(id, theme);
    }

    pub fn remove(&mut self, id: &str) -> bool {
        self.themes.remove(id).is_some()
    }

    pub fn set_system_theme(&mut self, theme: Option<ThemeJson>) {
        self.system_theme = theme;
    }

    pub fn get_system_theme(&self) -> Option<&ThemeJson> {
        self.system_theme.as_ref()
    }
}

impl Default for ThemeRegistry {
    fn default() -> Self {
        Self::with_builtin_themes()
    }
}
