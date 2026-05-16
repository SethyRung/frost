use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::actions::Action;
use crate::state::Overlay;

/// Translate a crossterm [`Event`] into an [`Action`], if applicable.
pub fn handle_event(event: Event, overlay: Option<Overlay>) -> Option<Action> {
    match event {
        Event::Key(key) => handle_key(key, overlay),
        Event::Resize(width, height) => Some(Action::Resize { width, height }),
        _ => None,
    }
}

fn handle_key(key: KeyEvent, overlay: Option<Overlay>) -> Option<Action> {
    // When an overlay is open, handle overlay-specific keys first.
    if let Some(_overlay) = overlay {
        return handle_overlay_key(key);
    }

    // No overlay open — handle normal navigation.
    match key.code {
        // Ctrl+C or Ctrl+Q always quit.
        KeyCode::Char('c') | KeyCode::Char('q')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(Action::Quit)
        }
        // Plain 'q' also quits (no modifier).
        KeyCode::Char('q') if key.modifiers.is_empty() => Some(Action::Quit),
        // Navigation.
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
            Some(Action::Up)
        }
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
            Some(Action::Down)
        }
        // Toggle expand / start-stop.
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Toggle),
        // Open palette.
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::OpenPalette)
        }
        // Open search.
        KeyCode::Char('/') if key.modifiers.is_empty() => {
            Some(Action::OpenSearch)
        }
        _ => None,
    }
}

fn handle_overlay_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        // Escape closes overlay.
        KeyCode::Esc => Some(Action::CloseOverlay),
        // Enter confirms selection.
        KeyCode::Enter => Some(Action::Confirm),
        // Navigation within overlay.
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => {
            Some(Action::Up)
        }
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => {
            Some(Action::Down)
        }
        // Backspace removes last filter char.
        KeyCode::Backspace => Some(Action::FilterBackspace),
        // Ctrl+U clears filter.
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::FilterClear)
        }
        // Typing characters appends to filter.
        KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            Some(Action::FilterChar(c))
        }
        _ => None,
    }
}
