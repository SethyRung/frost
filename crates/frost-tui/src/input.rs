use crossterm::event::{Event, KeyCode, KeyEvent, KeyModifiers};

use crate::actions::Action;
use crate::state::{Focus, Overlay};

pub fn handle_event(event: Event, overlay: Option<Overlay>, focus: Focus) -> Option<Action> {
    match event {
        Event::Key(key) => handle_key(key, overlay, focus),
        Event::Resize(width, height) => Some(Action::Resize { width, height }),
        Event::Paste(text) => {
            // Paste only forwards to the PTY when the log viewer has focus —
            // sidebar focus would dump arbitrary text into the navigation
            // tree, which is never what the user wants.
            if overlay.is_none() && focus == Focus::LogViewer {
                Some(Action::Paste(text))
            } else {
                None
            }
        }
        Event::Mouse(m) => {
            // Mouse events are always emitted while we hold the host
            // terminal in mouse-capture mode; route them to the app
            // unconditionally and let it decide whether the event is
            // inside the log pane and whether the child wants it.
            if overlay.is_none() {
                Some(Action::Mouse(m))
            } else {
                None
            }
        }
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
    // Ctrl+Shift+C copies the current selection (if any) to the system
    // clipboard. Must be checked BEFORE the generic Ctrl+letter encoder
    // because plain Ctrl+C is used for SIGINT, and crossterm reports
    // this combination with both modifiers set and `c` (or `C`).
    if (key.code == KeyCode::Char('c') || key.code == KeyCode::Char('C'))
        && key
            .modifiers
            .contains(KeyModifiers::CONTROL | KeyModifiers::SHIFT)
    {
        return Some(Action::CopySelection);
    }

    // Ctrl+Q is the quit binding when the log viewer is focused so that
    // Ctrl+C can be forwarded to the child as a real SIGINT via the
    // controlling tty.
    if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::Quit);
    }

    // Tab returns to the sidebar so the user can navigate without quitting.
    if key.code == KeyCode::Tab && key.modifiers.is_empty() {
        return Some(Action::ToggleFocus);
    }

    // Ctrl+P opens the palette from any focus.
    if key.code == KeyCode::Char('p') && key.modifiers.contains(KeyModifiers::CONTROL) {
        return Some(Action::OpenPalette);
    }

    // Scrollback navigation. The Shift+ variants are reserved for the
    // host TUI; the unmodified PageUp/Down/Home/End are forwarded to
    // the PTY so pagers like `less` still work.
    if key.modifiers.contains(KeyModifiers::SHIFT) {
        match key.code {
            KeyCode::PageUp => return Some(Action::ScrollUp),
            KeyCode::PageDown => return Some(Action::ScrollDown),
            KeyCode::Home => return Some(Action::ScrollTop),
            KeyCode::End => return Some(Action::ScrollBottom),
            _ => {}
        }
    }

    // Everything else — including bare Ctrl+C, Esc, arrow keys with
    // modifiers, Shift+Tab, function keys — is encoded and forwarded to
    // the PTY so interactive programs work normally.
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

/// Compute the xterm modifier parameter for CSI sequences such as
/// `\x1B[1;<mod><letter>`. Returns `1` (no modifier) up to `8` (all of
/// shift+alt+ctrl). `Meta` is currently treated as `Alt` because crossterm
/// reports macOS Option as `ALT`.
fn xterm_mod_param(mods: KeyModifiers) -> u8 {
    let mut m = 1u8;
    if mods.contains(KeyModifiers::SHIFT) {
        m += 1;
    }
    if mods.contains(KeyModifiers::ALT) {
        m += 2;
    }
    if mods.contains(KeyModifiers::CONTROL) {
        m += 4;
    }
    m
}

/// Encode an arrow / Home / End / PageUp / PageDown with optional
/// modifiers. Returns the bare 3-byte form when no modifier is set, and
/// the 6-byte CSI parameterised form otherwise.
fn encode_modified_arrow(mods: KeyModifiers, letter: u8) -> Vec<u8> {
    if mods.is_empty() {
        // Bare arrow: ESC [ <letter>.
        vec![0x1B, b'[', letter]
    } else {
        let m = xterm_mod_param(mods);
        format!("\x1B[1;{}{}", m, letter as char).into_bytes()
    }
}

/// Encode a tilde-terminated key (Insert, Delete, PageUp, PageDown, F5+).
/// Bare form: `ESC [ <code> ~`. Modified form: `ESC [ <code> ; <mod> ~`.
fn encode_modified_tilde(mods: KeyModifiers, code: u8) -> Vec<u8> {
    if mods.is_empty() {
        format!("\x1B[{}~", code).into_bytes()
    } else {
        let m = xterm_mod_param(mods);
        format!("\x1B[{};{}~", code, m).into_bytes()
    }
}

pub(crate) fn key_to_bytes(key: KeyEvent) -> Option<Vec<u8>> {
    let mods = key.modifiers;

    match key.code {
        KeyCode::Char(c) => {
            if mods.contains(KeyModifiers::CONTROL) {
                // Standard control-character encoding: A..Z + a few
                // punctuation chars map to 0x01..0x1F.
                let lower = c.to_ascii_lowercase();
                let code = (lower as u8) & 0x1F;
                if code < 0x20 {
                    let mut bytes = Vec::with_capacity(2);
                    if mods.contains(KeyModifiers::ALT) {
                        bytes.push(0x1B);
                    }
                    bytes.push(code);
                    Some(bytes)
                } else {
                    None
                }
            } else if mods.contains(KeyModifiers::ALT) {
                // Alt+<char> sends ESC + <char>.
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                let mut bytes = Vec::with_capacity(1 + s.len());
                bytes.push(0x1B);
                bytes.extend_from_slice(s.as_bytes());
                Some(bytes)
            } else {
                let mut buf = [0u8; 4];
                let s = c.encode_utf8(&mut buf);
                Some(s.as_bytes().to_vec())
            }
        }
        KeyCode::Enter => Some(vec![b'\r']),
        KeyCode::Backspace => {
            // Ctrl+Backspace conventionally sends 0x08; bare Backspace
            // sends DEL (0x7F) so terminals can distinguish the two.
            if mods.contains(KeyModifiers::CONTROL) {
                Some(vec![0x08])
            } else {
                Some(vec![0x7F])
            }
        }
        KeyCode::Tab => {
            if mods.contains(KeyModifiers::SHIFT) {
                Some(b"\x1B[Z".to_vec())
            } else {
                Some(vec![b'\t'])
            }
        }
        KeyCode::BackTab => Some(b"\x1B[Z".to_vec()),
        KeyCode::Esc => Some(vec![0x1B]),

        KeyCode::Up => Some(encode_modified_arrow(mods, b'A')),
        KeyCode::Down => Some(encode_modified_arrow(mods, b'B')),
        KeyCode::Right => Some(encode_modified_arrow(mods, b'C')),
        KeyCode::Left => Some(encode_modified_arrow(mods, b'D')),
        KeyCode::Home => Some(encode_modified_arrow(mods, b'H')),
        KeyCode::End => Some(encode_modified_arrow(mods, b'F')),

        KeyCode::PageUp => Some(encode_modified_tilde(mods, 5)),
        KeyCode::PageDown => Some(encode_modified_tilde(mods, 6)),
        KeyCode::Insert => Some(encode_modified_tilde(mods, 2)),
        KeyCode::Delete => Some(encode_modified_tilde(mods, 3)),

        KeyCode::F(1) => Some(b"\x1BOP".to_vec()),
        KeyCode::F(2) => Some(b"\x1BOQ".to_vec()),
        KeyCode::F(3) => Some(b"\x1BOR".to_vec()),
        KeyCode::F(4) => Some(b"\x1BOS".to_vec()),
        KeyCode::F(n) if (5..=12).contains(&n) => {
            let code = match n {
                5 => 15,
                6 => 17,
                7 => 18,
                8 => 19,
                9 => 20,
                10 => 21,
                11 => 23,
                12 => 24,
                _ => unreachable!(),
            };
            Some(encode_modified_tilde(mods, code))
        }

        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(code: KeyCode, mods: KeyModifiers) -> KeyEvent {
        KeyEvent::new(code, mods)
    }

    #[test]
    fn bare_arrows_use_three_byte_csi() {
        assert_eq!(
            key_to_bytes(k(KeyCode::Up, KeyModifiers::NONE)),
            Some(b"\x1B[A".to_vec())
        );
        assert_eq!(
            key_to_bytes(k(KeyCode::Down, KeyModifiers::NONE)),
            Some(b"\x1B[B".to_vec())
        );
    }

    #[test]
    fn alt_arrows_use_csi_1_3() {
        // ESC [ 1 ; 3 A — Alt+Up
        assert_eq!(
            key_to_bytes(k(KeyCode::Up, KeyModifiers::ALT)),
            Some(b"\x1B[1;3A".to_vec())
        );
        // Alt+Left
        assert_eq!(
            key_to_bytes(k(KeyCode::Left, KeyModifiers::ALT)),
            Some(b"\x1B[1;3D".to_vec())
        );
    }

    #[test]
    fn shift_arrows_use_csi_1_2() {
        assert_eq!(
            key_to_bytes(k(KeyCode::Right, KeyModifiers::SHIFT)),
            Some(b"\x1B[1;2C".to_vec())
        );
    }

    #[test]
    fn ctrl_arrows_use_csi_1_5() {
        assert_eq!(
            key_to_bytes(k(KeyCode::Right, KeyModifiers::CONTROL)),
            Some(b"\x1B[1;5C".to_vec())
        );
    }

    #[test]
    fn shift_tab_emits_csi_z() {
        assert_eq!(
            key_to_bytes(k(KeyCode::Tab, KeyModifiers::SHIFT)),
            Some(b"\x1B[Z".to_vec())
        );
        assert_eq!(
            key_to_bytes(k(KeyCode::BackTab, KeyModifiers::NONE)),
            Some(b"\x1B[Z".to_vec())
        );
    }

    #[test]
    fn ctrl_backspace_distinct_from_bare_backspace() {
        assert_eq!(
            key_to_bytes(k(KeyCode::Backspace, KeyModifiers::NONE)),
            Some(vec![0x7F])
        );
        assert_eq!(
            key_to_bytes(k(KeyCode::Backspace, KeyModifiers::CONTROL)),
            Some(vec![0x08])
        );
    }

    #[test]
    fn ctrl_c_lowercase_or_uppercase_yields_0x03() {
        // Plain Ctrl+C: 0x03 (ETX, drives SIGINT via tty driver).
        assert_eq!(
            key_to_bytes(k(KeyCode::Char('c'), KeyModifiers::CONTROL)),
            Some(vec![0x03])
        );
        // Ctrl+Shift+C (with capital 'C') still becomes 0x03.
        assert_eq!(
            key_to_bytes(k(
                KeyCode::Char('C'),
                KeyModifiers::CONTROL | KeyModifiers::SHIFT
            )),
            Some(vec![0x03])
        );
    }

    #[test]
    fn alt_char_emits_esc_prefix() {
        assert_eq!(
            key_to_bytes(k(KeyCode::Char('b'), KeyModifiers::ALT)),
            Some(vec![0x1B, b'b'])
        );
    }

    #[test]
    fn alt_ctrl_char_emits_esc_then_ctrl_byte() {
        // Alt+Ctrl+H → ESC + 0x08
        assert_eq!(
            key_to_bytes(k(
                KeyCode::Char('h'),
                KeyModifiers::ALT | KeyModifiers::CONTROL
            )),
            Some(vec![0x1B, 0x08])
        );
    }

    #[test]
    fn modified_pageup_uses_tilde_with_param() {
        assert_eq!(
            key_to_bytes(k(KeyCode::PageUp, KeyModifiers::NONE)),
            Some(b"\x1B[5~".to_vec())
        );
        // Shift+PageUp → ESC [ 5 ; 2 ~
        assert_eq!(
            key_to_bytes(k(KeyCode::PageUp, KeyModifiers::SHIFT)),
            Some(b"\x1B[5;2~".to_vec())
        );
    }

    #[test]
    fn function_keys_encode_correctly() {
        // F1–F4 use the SS3 form.
        assert_eq!(
            key_to_bytes(k(KeyCode::F(1), KeyModifiers::NONE)),
            Some(b"\x1BOP".to_vec())
        );
        // F5 uses CSI 15~.
        assert_eq!(
            key_to_bytes(k(KeyCode::F(5), KeyModifiers::NONE)),
            Some(b"\x1B[15~".to_vec())
        );
        // F12 uses CSI 24~.
        assert_eq!(
            key_to_bytes(k(KeyCode::F(12), KeyModifiers::NONE)),
            Some(b"\x1B[24~".to_vec())
        );
    }
}
