pub mod config;
pub mod process;
pub mod state;
pub mod theme;

// Re-export commonly used items
pub use config::{
    flatten_config, find_config, load_config,
    schema::{AppConfig, FrostConfig, ProjectConfig, SubCommand},
    ConfigError, RuntimeCommand,
};
pub use process::{
    manager::ProcessManager,
    types::{DisplayLine, ProcessInfo, ProcessStatus, ScreenUpdate, StateEvent, TerminalCell},
};
pub use state::{FrostState, StateStore};
pub use theme::{
    builtin::DEFAULT_THEME_ID,
    registry::ThemeRegistry,
    resolver::{resolve_theme, resolve_theme_safe, ansi_to_rgba, parse_hex, ResolveError},
    store::{PersistedThemeState, ThemeStore},
    system::generate_system_theme,
    types::{ResolvedTheme, RGBA, TerminalColors, ThemeJson, ThemeMode},
};
