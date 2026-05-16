use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostConfig {
    pub projects: HashMap<String, ProjectConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub workdir: Option<String>,
    /// Optional glyph rendered next to the project name in the sidebar.
    /// Pick a single nerd-font codepoint (e.g. `""` for a folder,
    /// `""` for a phone) or any short unicode string. TUIs cannot
    /// render raster/SVG; this is the per-project icon hook.
    #[serde(default)]
    pub icon: Option<String>,
    pub apps: HashMap<String, AppConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub workdir: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub command: Option<String>,
    #[serde(default)]
    pub commands: Option<HashMap<String, SubCommand>>,
    /// Per-app icon glyph rendered next to the app name in the sidebar.
    /// Same shape as [`ProjectConfig::icon`]; renderer prefers the app
    /// icon when set, otherwise leaves the row plain.
    #[serde(default)]
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubCommand {
    pub command: String,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}
