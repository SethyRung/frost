/// High-level actions the TUI can dispatch.
#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    /// Periodic tick fired by the event loop.
    Tick,
    /// Exit the application.
    Quit,
    /// Terminal window was resized.
    Resize { width: u16, height: u16 },
}
