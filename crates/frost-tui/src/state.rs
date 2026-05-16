/// Global TUI state machine.
#[derive(Debug, Default)]
pub struct AppState {
    /// Set to `true` when the user requests quit.
    pub should_quit: bool,
    /// Monotonically-increasing tick counter (useful for animations).
    pub tick_count: u64,
    /// Current terminal dimensions.
    pub terminal_size: (u16, u16),
}
