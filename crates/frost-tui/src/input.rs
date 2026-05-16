use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::actions::Action;
use crate::state::{Focus, Overlay};

pub fn handle_event(event: Event, overlay: Option<Overlay>, focus: Focus) -> Option<Action> {
    match event {
        Event::Key(key) => handle_key(key, overlay, focus),
        Event::Resize(width, height) => Some(Action::Resize { width, height }),
        _ => None,
    }
}

fn handle_key(key: KeyEvent, overlay: Option<Overlay>, focus: Focus) -> Option<Action> {
    if let Some(_overlay) = overlay {
        return handle_overlay_key(key);
    }

    if focus == Focus::LogViewer {
        return handle_log_viewer_key(key);
    }

    handle_sidebar_key(key)
}

fn handle_sidebar_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Char('c') | KeyCode::Char('q')
            if key.modifiers.contains(KeyModifiers::CONTROL) =>
        {
            Some(Action::Quit)
        }
        KeyCode::Char('q') if key.modifiers.is_empty() => Some(Action::Quit),
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => Some(Action::Down),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Action::Toggle),
        KeyCode::PageUp => Some(Action::ScrollUp),
        KeyCode::PageDown => Some(Action::ScrollDown),
        KeyCode::End => Some(Action::ScrollBottom),
        KeyCode::Tab => Some(Action::ToggleFocus),
        KeyCode::Char('p') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::OpenPalette)
        }
        KeyCode::Char('/') if key.modifiers.is_empty() => Some(Action::OpenSearch),
        _ => None,
    }
}

fn handle_log_viewer_key(key: KeyEvent) -> Option<Action> {
    // Ctrl+C always quits regardless of focus.
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::Quit);
    }

    // Tab switches back to sidebar.
    if key.code == KeyCode::Tab {
        return Some(Action::ToggleFocus);
    }

    // Escape returns to sidebar.
    if key.code == KeyCode::Esc {
        return Some(Action::ToggleFocus);
    }

    // Ctrl+P opens palette from any focus.
    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::OpenPalette);
    }

    // Convert key to raw bytes and forward to PTY.
    let bytes = key_to_bytes(key)?;
    Some(Action::WriteInput(bytes))
}

fn handle_overlay_key(key: KeyEvent) -> Option<Action> {
    match key.code {
        KeyCode::Esc => Some(Action::CloseOverlay),
        KeyCode::Enter => Some(Action::Confirm),
        KeyCode::Up | KeyCode::Char('k') if key.modifiers.is_empty() => Some(Action::Up),
        KeyCode::Down | KeyCode::Char('j') if key.modifiers.is_empty() => Some(Action::Down),
        KeyCode::PageUp => Some(Action::Up),
        KeyCode::PageDown => Some(Action::Down),
        KeyCode::Backspace => Some(Action::FilterBackspace),
        KeyCode::Char('u') if key.modifiers.contains(KeyModifiers::CONTROL) => {
            Some(Action::FilterClear)
        }
        KeyCode::Char(c) if key.modifiers.is_empty() || key.modifiers == KeyModifiers::SHIFT => {
            Some(Action::FilterChar(c))
        }
        _ => None,
    }
}

fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    let mods = key.modifiers;

    match key.code {
        KeyCode::Char(c) => {
            if mods.contains(KeyModifiers::CONTROL) {
                let code = (c as u8) & 0x1F;
                if code < 0x20 {
                    Some(vec![code])
                } else {
                    None
                }
            } else if mods.contains(KeyModifiers::ALT) {
                Some(vec![0x1B, c as u8])
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                Some(s.as_bytes().to_vec())
            }
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => Some(vec![0x7F]),
        KeyCode::Tab => Some(vec![b'\t']),
        KeyCode::Esc => Some(vec![0x1B]),
        KeyCode::Home => Some(b"\x1B[H".to_vec()),
        KeyCode::End => Some(b"\x1B[F".to_vec()),
        KeyCode::PageUp => Some(b"\x1B[5~".to_vec()),
        KeyCode::PageDown => Some(b"\x1B[6~".to_vec()),
        KeyCode::Up => Some(b"\x1B[A".to_vec()),
        KeyCode::Down => Some(b"\x1B[B".to_vec()),
        KeyCode::Right => Some(b"\x1B[C".to_vec()),
        KeyCode::Left => Some(b"\x1B[D".to_vec()),
        KeyCode::F(1) => Some(b"\x1BOP".to_vec()),
        KeyCode::F(2) => Some(b"\x1BOQ".to_vec()),
        KeyCode::F(3) => Some(b"\x1BOR".to_vec()),
        KeyCode::F(4) => Some(b"\x1BOS".to_vec()),
        KeyCode::F(n) if (5..=12).contains(&n) => {
            Some(format!("\x1B[{}~", n + 7).into_bytes())
        }
        KeyCode::Insert => Some(b"\x1B[2~".to_vec()),
        KeyCode::Delete => Some(b"\x1B[3~".to_vec()),
        _ => None,
    }
}
