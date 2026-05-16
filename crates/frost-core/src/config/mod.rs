pub mod schema;

use crate::config::schema::{FrostConfig, SubCommand};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("TOML parse error: {0}")]
    Toml(#[from] toml::de::Error),
    #[error("Config validation error: {0}")]
    Validation(String),
    #[error("No config found")]
    NotFound,
}

/// Walk up from cwd looking for `frost.toml` (then `frost.json` fallback).
pub fn find_config(cwd: impl AsRef<Path>) -> Option<PathBuf> {
    let filenames = ["frost.toml", "frost.json"];
    let mut dir = cwd.as_ref().to_path_buf();
    loop {
        for filename in &filenames {
            let path = dir.join(filename);
            if path.exists() {
                return Some(path);
            }
        }
        let parent = dir.parent()?;
        if parent == dir {
            break;
        }
        dir = parent.to_path_buf();
    }
    None
}

/// Load and parse a config file from the given path.
pub fn load_config(path: impl AsRef<Path>) -> Result<FrostConfig, ConfigError> {
    let text = std::fs::read_to_string(path.as_ref())?;
    let ext = path
        .as_ref()
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("");

    let config: FrostConfig = if ext == "json" {
        serde_json::from_str(&text).map_err(|e| ConfigError::Validation(e.to_string()))?
    } else {
        toml::from_str(&text)?
    };

    validate_config(&config)?;
    Ok(config)
}

fn validate_config(config: &FrostConfig) -> Result<(), ConfigError> {
    if config.projects.is_empty() {
        return Err(ConfigError::Validation(
            "Config must have at least one project".to_string(),
        ));
    }

    for (project_name, project) in &config.projects {
        if project.apps.is_empty() {
            return Err(ConfigError::Validation(format!(
                "Project '{}' must have at least one app",
                project_name
            )));
        }

        for (app_name, app) in &project.apps {
            let has_command = app.command.is_some();
            let has_commands = app
                .commands
                .as_ref()
                .map(|c| !c.is_empty())
                .unwrap_or(false);

            if !has_command && !has_commands {
                return Err(ConfigError::Validation(format!(
                    "App '{}' in project '{}' must have either 'command' or 'commands'",
                    app_name, project_name
                )));
            }

            if let Some(commands) = &app.commands {
                for (sub_name, sub) in commands {
                    if sub.command.trim().is_empty() {
                        return Err(ConfigError::Validation(format!(
                            "Sub-command '{}' in app '{}' project '{}' must have a non-empty command",
                            sub_name, app_name, project_name
                        )));
                    }
                }
            }

            if let Some(cmd) = &app.command {
                if cmd.trim().is_empty() {
                    return Err(ConfigError::Validation(format!(
                        "App '{}' in project '{}' must have a non-empty command",
                        app_name, project_name
                    )));
                }
            }
        }
    }

    Ok(())
}

/// A flattened, runtime-ready command with all paths resolved.
#[derive(Debug, Clone)]
pub struct RuntimeCommand {
    pub project_name: String,
    pub app_name: String,
    pub subcommand_name: String,
    pub command: String,
    pub workdir: PathBuf,
    pub is_default: bool,
    pub env: HashMap<String, String>,
}

/// Flatten config into a list of spawnable runtime commands.
///
/// `workdir` resolution chain: sub-command level → app level → project level → config dir.
pub fn flatten_config(config: &FrostConfig, config_path: impl AsRef<Path>) -> Vec<RuntimeCommand> {
    let config_dir = config_path
        .as_ref()
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));

    let mut commands = Vec::new();

    for (project_name, project) in &config.projects {
        let project_dir = resolve_workdir(&config_dir, project.workdir.as_deref());

        for (app_name, app) in &project.apps {
            let app_dir = resolve_workdir(&project_dir, app.workdir.as_deref());

            let subcommands: Vec<(String, SubCommand)> = if let Some(cmds) = &app.commands {
                cmds.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
            } else if let Some(cmd) = &app.command {
                vec![(
                    "default".to_string(),
                    SubCommand {
                        command: cmd.clone(),
                        workdir: None,
                        env: None,
                    },
                )]
            } else {
                continue;
            };

            let default_sub = app
                .default
                .clone()
                .or_else(|| {
                    let mut keys: Vec<_> = subcommands.iter().map(|(k, _)| k.clone()).collect();
                    keys.sort();
                    keys.into_iter().next()
                })
                .unwrap_or_else(|| "default".to_string());

            for (sub_name, sub) in subcommands {
                let workdir = resolve_workdir(&app_dir, sub.workdir.as_deref());
                let is_default = sub_name == default_sub;

                commands.push(RuntimeCommand {
                    project_name: project_name.clone(),
                    app_name: app_name.clone(),
                    subcommand_name: sub_name,
                    command: sub.command.clone(),
                    workdir,
                    is_default,
                    env: sub.env.clone().unwrap_or_default(),
                });
            }
        }
    }

    commands
}

fn resolve_workdir(base: &Path, relative: Option<&str>) -> PathBuf {
    match relative {
        Some(rel) => {
            let path = Path::new(rel);
            if path.is_absolute() {
                path.to_path_buf()
            } else {
                base.join(path)
                    .canonicalize()
                    .unwrap_or_else(|_| base.join(path))
            }
        }
        None => base.to_path_buf(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn make_temp_toml(content: &str) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("frost.toml");
        let mut file = std::fs::File::create(&path).unwrap();
        file.write_all(content.as_bytes()).unwrap();
        (dir, path)
    }

    #[test]
    fn test_load_basic_toml() {
        let (_dir, path) = make_temp_toml(
            r#"
[projects.portfolio]
workdir = "../portfolio"

[projects.portfolio.apps.frontend]
workdir = "./frontend"
default = "dev"

[projects.portfolio.apps.frontend.commands.dev]
command = "bun dev"

[projects.portfolio.apps.frontend.commands.build]
command = "bun run build"
"#,
        );

        let config = load_config(&path).unwrap();
        assert_eq!(config.projects.len(), 1);
        assert!(config.projects.contains_key("portfolio"));
    }

    #[test]
    fn test_flatten_config() {
        let (_dir, path) = make_temp_toml(
            r#"
[projects.portfolio]
workdir = "../portfolio"

[projects.portfolio.apps.frontend]
workdir = "./frontend"
default = "dev"

[projects.portfolio.apps.frontend.commands.dev]
command = "bun dev"

[projects.portfolio.apps.frontend.commands.build]
command = "bun run build"

[projects.portfolio.apps.api]
command = "bun serve"
"#,
        );

        let config = load_config(&path).unwrap();
        let commands = flatten_config(&config, &path);

        assert_eq!(commands.len(), 3);

        let dev = commands
            .iter()
            .find(|c| c.subcommand_name == "dev")
            .unwrap();
        assert_eq!(dev.project_name, "portfolio");
        assert_eq!(dev.app_name, "frontend");
        assert_eq!(dev.command, "bun dev");
        assert!(dev.is_default);
        assert!(dev.workdir.to_string_lossy().contains("frontend"));

        let api = commands.iter().find(|c| c.app_name == "api").unwrap();
        assert_eq!(api.subcommand_name, "default");
        assert_eq!(api.command, "bun serve");
        assert!(api.is_default);
    }

    #[test]
    fn test_find_config_walks_up() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("frost.toml");
        std::fs::write(
            &config_path,
            "[projects.p]\n\n[projects.p.apps.a]\ncommand = 'x'\n",
        )
        .unwrap();

        let subdir = dir.path().join("a").join("b").join("c");
        std::fs::create_dir_all(&subdir).unwrap();

        let found = find_config(&subdir).unwrap();
        assert_eq!(found, config_path);
    }

    #[test]
    fn test_validation_missing_commands() {
        let (_dir, path) = make_temp_toml(
            r#"
[projects.portfolio]

[projects.portfolio.apps.frontend]
workdir = "./frontend"
"#,
        );

        let result = load_config(&path);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_real_frost_toml() {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let repo_root = manifest.parent().unwrap().parent().unwrap();
        let config_path = repo_root.join("frost.toml");

        assert!(config_path.exists(), "frost.toml should exist at repo root");

        let config = load_config(&config_path).expect("should parse real frost.toml");
        assert!(
            !config.projects.is_empty(),
            "should have at least one project"
        );

        let commands = flatten_config(&config, &config_path);
        assert!(
            !commands.is_empty(),
            "should flatten to at least one command"
        );
    }
}
