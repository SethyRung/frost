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
    /// Jump to bottom of log viewer (End / Shift+End).
    ScrollBottom,
    /// Jump to the very top of the scrollback (Shift+Home).
    ScrollTop,
    /// Switch focus between sidebar and log viewer.
    ToggleFocus,
    /// Write raw bytes to the focused process's PTY stdin.
    WriteInput(Vec<u8>),
    /// Forward a pasted string to the focused process. The handler wraps
    /// the bytes in bracketed-paste delimiters when the child has DEC
    /// mode 2004 enabled and otherwise sends them raw.
    Paste(String),
    /// A mouse event in terminal coordinates. The handler decides whether
    /// to forward it to the PTY (when the child has mouse reporting on)
    /// or to consume it locally for scrollback / selection.
    Mouse(crossterm::event::MouseEvent),
    /// Copy the current log-viewer selection to the system clipboard.
    /// Bound to Ctrl+Shift+C in the log viewer; no-op when no selection.
    CopySelection,

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
