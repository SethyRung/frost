use crate::theme::types::RGBA;
use serde::{Deserialize, Serialize};
use std::fmt;

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
}

/// A line of the terminal display grid (a row of cells).
pub type DisplayLine = Vec<TerminalCell>;

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
