pub mod config;
pub mod process;
pub mod state;
pub mod theme;

pub use config::{
    ConfigError, RuntimeCommand, find_config, flatten_config, load_config,
    schema::{AppConfig, FrostConfig, ProjectConfig, SubCommand},
};
pub use process::{
    manager::ProcessManager,
    types::{
        DisplayLine, MouseModes, ProcessInfo, ProcessStatus, ScreenUpdate, StateEvent, TerminalCell,
    },
};
pub use state::{FrostState, StateStore};
pub use theme::{
    builtin::DEFAULT_THEME_ID,
    registry::ThemeRegistry,
    resolver::{ResolveError, ansi_to_rgba, parse_hex, resolve_theme, resolve_theme_safe},
    store::{PersistedThemeState, ThemeStore},
    system::generate_system_theme,
    types::{RGBA, ResolvedTheme, TerminalColors, ThemeJson, ThemeMode},
};
