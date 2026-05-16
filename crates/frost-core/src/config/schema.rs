use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostConfig {
    pub projects: HashMap<String, ProjectConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub workdir: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubCommand {
    pub command: String,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,
}
