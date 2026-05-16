use std::io;

use anyhow::Result;
use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture, Event},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{backend::CrosstermBackend, Terminal};
use tokio::sync::mpsc;

use frost_core::ProcessManager;

use crate::{actions::Action, app::App};

mod actions;
mod app;
mod input;
mod state;

#[tokio::main]
async fn main() -> Result<()> {
    // Set up terminal.
    let mut terminal = setup_terminal()?;

    // Create the process manager.
    let process_manager = ProcessManager::new();

    // Run the app.
    let result = run_app(&mut terminal, process_manager).await;

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
                    // Detect Ctrl+C before sending so we can break the thread
                    // after the event has been delivered.
                    let is_ctrl_c = matches!(
                        event,
                        Event::Key(k)
                            if k.code == KeyCode::Char('c')
                                && k.modifiers.contains(KeyModifiers::CONTROL)
                    );
                    if tx.send(event).is_err() {
                        break; // Receiver dropped → exit thread.
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
) -> Result<()> {
    let mut app = App::new(process_manager);

    // Crossterm events arrive via a dedicated thread → tokio channel.
    let (event_tx, mut event_rx) = mpsc::unbounded_channel::<Event>();
    spawn_event_reader(event_tx);

    // Tick interval for periodic updates.
    let mut tick_interval = tokio::time::interval(std::time::Duration::from_millis(250));

    loop {
        // Draw current frame.
        terminal.draw(|frame| app.draw(frame))?;

        // Wait for the next event or tick.
        tokio::select! {
            Some(event) = event_rx.recv() => {
                if let Some(action) = input::handle_event(event) {
                    app.handle_action(action);
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

    Ok(())
}
