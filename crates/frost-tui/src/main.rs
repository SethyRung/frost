use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::{
    event::{
        DisableBracketedPaste, DisableMouseCapture, EnableBracketedPaste, EnableMouseCapture, Event,
    },
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{Terminal, backend::CrosstermBackend};
use tokio::sync::mpsc;

use frost_core::{ProcessManager, find_config, load_config};

use crate::{actions::Action, app::App};

mod actions;
mod app;
mod command_bar;
mod input;
mod log_viewer;
mod palette;
mod search;
mod selection;
mod sidebar;
mod state;
mod theme_adapter;
mod theme_dialog;

#[tokio::main]
async fn main() -> Result<()> {
    // Find and load config.
    let config_path = find_config(std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
        .context("No frost.toml found — run from a project with a frost.toml")?;
    let config = load_config(&config_path)
        .with_context(|| format!("Failed to load config from {:?}", config_path))?;

    // Set up terminal.
    let mut terminal = setup_terminal()?;

    // Create the process manager and subscribe to screen + state updates.
    let process_manager = ProcessManager::new();
    let mut screen_rx = process_manager.subscribe_screen();
    let mut state_rx = process_manager.subscribe_state();

    // Run the app.
    let result = run_app(
        &mut terminal,
        process_manager,
        config,
        config_path,
        &mut screen_rx,
        &mut state_rx,
    )
    .await;

    // Restore terminal regardless of result.
    restore_terminal(&mut terminal)?;

    result
}

/// Enter raw mode + alternate screen, enable mouse capture, and turn on
/// bracketed paste so the host terminal delivers `Event::Paste(text)`
/// in one shot instead of as a stream of synthetic key events.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(
        stdout,
        EnterAlternateScreen,
        EnableMouseCapture,
        EnableBracketedPaste
    )?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Reverse of [`setup_terminal`]. The host terminal's state must be put
/// back exactly as we found it, otherwise the user's shell session ends
/// up in a broken state (mouse capture stuck, paste mangled).
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        DisableBracketedPaste,
        DisableMouseCapture,
        LeaveAlternateScreen,
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Spawn a background thread that forwards crossterm events into a tokio
/// channel. The loop exits only when the channel is closed (i.e. the main
/// task ended) or when crossterm reports a fatal read error — the old
/// "exit on Ctrl+C" safety break is gone because Ctrl+C is now a real
/// terminal key when the log viewer is focused.
fn spawn_event_reader(tx: mpsc::UnboundedSender<Event>) {
    std::thread::spawn(move || {
        while let Ok(event) = crossterm::event::read() {
            if tx.send(event).is_err() {
                break;
            }
        }
    });
}

/// Main async event loop.
async fn run_app(
    terminal: &mut Terminal<CrosstermBackend<io::Stdout>>,
    process_manager: ProcessManager,
    config: frost_core::FrostConfig,
    config_path: PathBuf,
    screen_rx: &mut tokio::sync::broadcast::Receiver<frost_core::ScreenUpdate>,
    state_rx: &mut tokio::sync::broadcast::Receiver<frost_core::StateEvent>,
) -> Result<()> {
    // Query the actual terminal size so initial PTY dims match the visible
    // log pane, instead of defaulting to a guessed 80x24.
    let initial_size = crossterm::terminal::size().unwrap_or((120, 30));
    let mut app = App::new(process_manager, config, config_path, initial_size);

    // Crossterm events arrive via a dedicated thread → tokio channel.
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();
    spawn_event_reader(event_tx);

    // Tick interval for periodic updates.
    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(250));

    loop {
        // Draw current frame.
        terminal.draw(|frame| app.draw(frame))?;

        // Wait for the next event, tick, or screen update.
        tokio::select! {
            Some(event) = event_rx.recv() => {
                let overlay = app.state.overlay;
                let focus = app.state.focus;
                if let Some(action) = input::handle_event(event, overlay, focus) {
                    app.handle_action(action);
                }
            }
            result = screen_rx.recv() => {
                match result {
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                        // New output arrived — redraw will happen on next loop iteration.
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {
                        // All senders dropped — shouldn't happen.
                    }
                }
            }
            result = state_rx.recv() => {
                match result {
                    Ok(_) | Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {}
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => {}
                }
            }
            _ = tick_interval.tick() => {
                app.handle_action(Action::Tick);
            }
        }

        if app.state.should_quit {
            break;
        }
    }

    // Clean shutdown: stop all running processes.
    app.shutdown();

    Ok(())
}
