use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::actions::Action;

/// Translate a crossterm [`Event`] into an [`Action`], if applicable.
pub fn handle_event(event: Event) -> Option<Action> {
    match event {
        Event::Key(key) => handle_key(key),
        Event::Resize(width, height) => Some(Action::Resize { width, height }),
        _ => None,
    }
}

fn handle_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        // Ctrl+C or Ctrl+Q always quit.
        KeyCode::Char('c') | KeyCode::Char('q')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(Action::Quit)
        }
        // Plain 'q' also quits (no modifier).
        KeyCode::Char('q') if key.modifiers.is_empty() => Some(Action::Quit),
        _ => None,
    }
}
