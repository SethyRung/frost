use frost_core::ProcessStatus;

/// Which overlay is currently open, if any.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlay {
    Palette,
    Search,
    ThemeDialog,
}

/// Global TUI state machine.
#[derive(Debug)]
pub struct AppState {
    /// Set to `true` when the user requests quit.
    pub should_quit: bool,
    /// Monotonically-increasing tick counter (useful for animations).
    pub tick_count: u64,
    /// Current terminal dimensions.
    pub terminal_size: (u16, u16),
    /// Flattened tree selection index (visible items only).
    pub selected_index: usize,
    /// Which (project, app, subcommand) is currently selected for log viewing.
    pub selected_process: Option<(String, String, String)>,
    /// Scroll offset for the log viewer (0 = bottom / auto-scroll).
    #[allow(dead_code)]
    pub log_scroll: usize,
    /// Number of currently running processes.
    pub running_count: usize,
    /// Currently open overlay, if any.
    pub overlay: Option<Overlay>,
    /// Filter text for the current overlay (palette / search).
    pub filter_text: String,
    /// Selected index within the overlay list.
    pub overlay_selected: usize,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            should_quit: false,
            tick_count: 0,
            terminal_size: (0, 0),
            selected_index: 0,
            selected_process: None,
            log_scroll: 0,
            running_count: 0,
            overlay: None,
            filter_text: String::new(),
            overlay_selected: 0,
        }
    }
}

impl AppState {
    /// Get the status icon for a process.
    pub fn status_icon(status: ProcessStatus) -> &'static str {
        match status {
            ProcessStatus::Stopped => "○",
            ProcessStatus::Starting => "◐",
            ProcessStatus::Running => "●",
            ProcessStatus::Stopping => "◑",
            ProcessStatus::Crashed => "✕",
        }
    }
}
