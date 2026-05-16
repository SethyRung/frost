/// High-level actions the TUI can dispatch.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Periodic tick fired by the event loop.
    Tick,
    /// Exit the application.
    Quit,
    /// Terminal window was resized.
    Resize { width: u16, height: u16 },
    /// Move selection up in the sidebar tree.
    Up,
    /// Move selection down in the sidebar tree.
    Down,
    /// Toggle expand/collapse (project/app) or start/stop (subcommand).
    Toggle,
    /// Scroll log viewer up (PageUp).
    ScrollUp,
    /// Scroll log viewer down (PageDown).
    ScrollDown,
    /// Jump to bottom of log viewer (End).
    ScrollBottom,
    /// Switch focus between sidebar and log viewer.
    ToggleFocus,
    /// Write raw bytes to the focused process's PTY stdin.
    WriteInput(Vec<u8>),

    // Overlays
    /// Open the command palette.
    OpenPalette,
    /// Open the search dialog.
    OpenSearch,
    /// Close any open overlay.
    CloseOverlay,
    /// Confirm selection in an overlay (Enter when overlay is open).
    Confirm,
    /// Append a character to the overlay filter text.
    FilterChar(char),
    /// Backspace in the overlay filter text.
    FilterBackspace,
    /// Clear the overlay filter text.
    FilterClear,
}
