use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use portable_pty::{native_pty_system, CommandBuilder, PtySize};
use std::path::Path;

/// Error type for process/PTY operations.
#[derive(Debug, thiserror::Error)]
pub enum ProcessError {
    #[error("PTY error: {0}")]
    Pty(String),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Signal error: {0}")]
    Signal(#[from] nix::errno::Errno),
    #[error("Process not found: {0}/{1}/{2}")]
    NotFound(String, String, String),
}

/// A spawned PTY process.  Holds the master end (for reading output
/// and resizing), a writer for forwarding stdin, and the child handle.
pub struct PtyProcess {
    pub pid: u32,
    pub master: Box<dyn portable_pty::MasterPty + Send>,
    pub reader: Box<dyn std::io::Read + Send>,
    pub writer: Box<dyn std::io::Write + Send>,
    pub child: Box<dyn portable_pty::Child + Send + Sync>,
}

impl PtyProcess {
    /// Send `SIGTERM` to the entire process group (negative PID).
    pub fn kill_process_group(&self) -> Result<(), ProcessError> {
        let pid = Pid::from_raw(self.pid as i32);
        kill(Pid::from_raw(-pid.as_raw()), Signal::SIGTERM)?;
        Ok(())
    }

    /// Write raw bytes to the PTY master (sends stdin to child process).
    pub fn write_stdin(&mut self, data: &[u8]) -> Result<(), ProcessError> {
        use std::io::Write;
        self.writer.write_all(data)?;
        Ok(())
    }

    /// Resize the PTY dimensions.
    pub fn resize(&self, cols: u16, rows: u16) -> Result<(), ProcessError> {
        self.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| ProcessError::Pty(e.to_string()))?;
        Ok(())
    }
}

/// Spawn a command inside a PTY.
///
/// `command` is executed as `sh -c "<command>"` inside the PTY.
/// `portable-pty` handles `setsid()`, controlling terminal setup, and
/// signal cleanup automatically.
pub fn spawn_pty(
    command: &str,
    workdir: &Path,
    cols: u16,
    rows: u16,
) -> Result<PtyProcess, ProcessError> {
    let pty_system = native_pty_system();
    let pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    }).map_err(|e| ProcessError::Pty(e.to_string()))?;

    let mut cmd = CommandBuilder::new("sh");
    cmd.arg("-c");
    cmd.arg(command);
    cmd.cwd(workdir.to_str().unwrap_or("."));
    cmd.env("TERM", "xterm-256color");

    let child = pair.slave
        .spawn_command(cmd)
        .map_err(|e| ProcessError::Pty(e.to_string()))?;

    let pid = child.process_id().ok_or_else(|| {
        ProcessError::Pty("spawned child has no process id".to_string())
    })?;

    let reader = pair.master
        .try_clone_reader()
        .map_err(|e| ProcessError::Pty(e.to_string()))?;

    let writer = pair.master
        .take_writer()
        .map_err(|e| ProcessError::Pty(e.to_string()))?;

    Ok(PtyProcess {
        pid,
        master: pair.master,
        reader,
        writer,
        child,
    })
}
