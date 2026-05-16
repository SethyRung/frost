use crate::theme::types::RGBA;
use serde::{Deserialize, Serialize};
use std::fmt;
use std::time::Instant;

/// Status of a managed child process.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProcessStatus {
    /// Not running.
    Stopped,
    /// Spawn requested, waiting for first output or timeout.
    Starting,
    /// Running and producing output.
    Running,
    /// Stop requested, waiting for graceful shutdown.
    Stopping,
    /// Exited with a non-zero code (and not from SIGTERM).
    Crashed,
}

impl fmt::Display for ProcessStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ProcessStatus::Stopped => write!(f, "stopped"),
            ProcessStatus::Starting => write!(f, "starting"),
            ProcessStatus::Running => write!(f, "running"),
            ProcessStatus::Stopping => write!(f, "stopping"),
            ProcessStatus::Crashed => write!(f, "crashed"),
        }
    }
}

/// Static metadata about a process.
#[derive(Debug, Clone)]
pub struct ProcessInfo {
    pub project: String,
    pub app: String,
    pub subcommand: String,
    pub status: ProcessStatus,
    pub pid: Option<u32>,
    pub generation_id: u64,
}

/// A single styled cell inside the terminal emulator grid.
#[derive(Debug, Clone, Copy)]
pub struct TerminalCell {
    pub c: char,
    pub fg: RGBA,
    pub bg: RGBA,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub reverse: bool,
    pub dim: bool,
    pub hidden: bool,
    pub strikethrough: bool,
    /// True when this cell occupies two terminal columns (CJK / emoji /
    /// other East-Asian wide characters). Renderers should ensure the
    /// glyph is laid out across two columns and that no neighbouring cell
    /// overlaps the right half.
    pub wide: bool,
}

/// A line of the terminal display grid (a row of cells).
pub type DisplayLine = Vec<TerminalCell>;

/// Out-of-band state the terminal emulator collects via its event
/// listener: window title (OSC 0/2) and the most recent bell timestamp.
/// Lives behind a `Mutex` shared by `FrostListener` (writer) and
/// `ProcessManager` (reader).
#[derive(Debug, Default)]
pub struct TerminalState {
    pub title: Option<String>,
    pub bell_at: Option<Instant>,
}

/// Snapshot of the mouse-reporting DEC modes a process's terminal
/// emulator currently has enabled. Drives the TUI's decision whether to
/// forward a mouse event to the child as an escape sequence or to
/// consume it locally (e.g. for scrollback or selection).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct MouseModes {
    /// DEC 1000 — report button press/release only.
    pub report_click: bool,
    /// DEC 1002 — also report drag (motion while a button is held).
    pub drag: bool,
    /// DEC 1003 — report all motion regardless of button state.
    pub motion: bool,
    /// DEC 1006 — use SGR (`\x1B[<…M/m`) encoding for the reports above.
    pub sgr: bool,
    /// DEC 1007 — translate scroll-wheel events into up/down arrow keys
    /// while the alternate screen is active.
    pub alternate_scroll: bool,
}

impl MouseModes {
    /// True when *any* of the reporting modes is enabled.
    pub fn reporting(&self) -> bool {
        self.report_click || self.drag || self.motion
    }
}

/// Notification that a terminal screen has changed and the TUI should re-render.
#[derive(Debug, Clone)]
pub struct ScreenUpdate {
    pub project: String,
    pub app: String,
    pub subcommand: String,
    pub generation_id: u64,
}

/// Lifecycle events emitted by the ProcessManager.
#[derive(Debug, Clone)]
pub enum StateEvent {
    Started {
        project: String,
        app: String,
        subcommand: String,
    },
    Stopped {
        project: String,
        app: String,
        subcommand: String,
    },
    Crashed {
        project: String,
        app: String,
        subcommand: String,
    },
    Screen(ScreenUpdate),
}
