use std::io;
use std::path::PathBuf;

use anyhow::{Context, Result};
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
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
mod sidebar;
mod state;
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

    // Create the process manager and subscribe to screen updates.
    let process_manager = ProcessManager::new();
    let mut screen_rx = process_manager.subscribe_screen();

    // Run the app.
    let result = run_app(&mut terminal, process_manager, config, config_path, &mut screen_rx).await;

    // Restore terminal regardless of result.
    restore_terminal(&mut terminal)?;

    result
}

/// Enter raw mode + alternate screen.
fn setup_terminal() -> Result<Terminal<CrosstermBackend<io::Stdout>>> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    Ok(Terminal::new(backend)?)
}

/// Leave raw mode + alternate screen.
fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

/// Spawn a background thread that forwards crossterm events into a tokio channel.
fn spawn_event_reader(tx: mpsc::UnboundedSender<Event>) {
    use crossterm::event::{KeyCode, KeyModifiers};

    std::thread::spawn(move || {
        loop {
            match crossterm::event::read() {
                Ok(event) => {
                    let is_ctrl_c = matches!(
                        event,
                        Event::Key(k)
                            if k.code == KeyCode::Char('c')
                                && k.modifiers.contains(KeyModifiers::CONTROL)
                    );
                    if tx.send(event).is_err() {
                        break;
                    }
                    if is_ctrl_c {
                        break;
                    }
                }
                Err(_) => break,
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
) -> Result<()> {
    let mut app = App::new(process_manager, config, config_path);

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
