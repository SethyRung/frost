use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Borders},
};

use frost_core::ProcessManager;

use crate::{actions::Action, state::AppState};

/// Top-level application container.
pub struct App {
    pub state: AppState,
    #[allow(dead_code)]
    pub process_manager: ProcessManager,
}

impl App {
    pub fn new(process_manager: ProcessManager) -> Self {
        Self {
            state: AppState::default(),
            process_manager,
        }
    }

    /// Apply an incoming action to mutate state.
    pub fn handle_action(&mut self, action: Action) {
        match action {
            Action::Quit => self.state.should_quit = true,
            Action::Tick => self.state.tick_count += 1,
            Action::Resize { width, height } => {
                self.state.terminal_size = (width, height);
            }
        }
    }

    /// Render the layout skeleton (sidebar + log + command bar).
    pub fn draw(&self, frame: &mut Frame) {
        let area = frame.area();

        // Vertical split: main area + command bar at the bottom.
        let vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(1)])
            .split(area);

        // Horizontal split: sidebar (left) + log viewer (right).
        let horiz = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Length(30), Constraint::Min(1)])
            .split(vert[0]);

        // Sidebar placeholder.
        let sidebar = Block::default()
            .title(" Projects ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Cyan));
        frame.render_widget(sidebar, horiz[0]);

        // Log viewer placeholder.
        let log = Block::default()
            .title(" Log ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Green));
        frame.render_widget(log, horiz[1]);

        // Command bar placeholder.
        let cmd_bar = Block::default()
            .title(" Command Bar ")
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow));
        frame.render_widget(cmd_bar, vert[1]);
    }
}
