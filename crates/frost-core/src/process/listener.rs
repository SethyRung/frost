//! Custom `alacritty_terminal::event::EventListener` implementation that
//! captures out-of-band terminal events (window title, bell) into a
//! shared `TerminalState` the TUI can poll on every frame.
//!
//! The previous `VoidListener` discarded everything; that meant OSC 0/2
//! title updates and BEL (0x07) bell rings never surfaced. We still
//! ignore clipboard / color / PTY-write requests for security reasons —
//! those are routed through explicit, opt-in paths instead.

use alacritty_terminal::event::{Event, EventListener};
use std::sync::{Arc, Mutex};
use std::time::Instant;

use crate::process::types::TerminalState;

/// Listener wired into every `Term` instance the manager spawns. Holds
/// a reference to the per-process `TerminalState` so writes from the
/// emulator thread are visible to the TUI render thread.
#[derive(Debug, Clone)]
pub struct FrostListener {
    state: Arc<Mutex<TerminalState>>,
}

impl FrostListener {
    pub fn new(state: Arc<Mutex<TerminalState>>) -> Self {
        Self { state }
    }

    /// Listener used in unit tests that don't care about title or bell.
    /// The state it points at is owned solely by the listener and
    /// dropped at the end of the test.
    #[cfg(test)]
    pub fn dummy() -> Self {
        Self::new(Arc::new(Mutex::new(TerminalState::default())))
    }

    /// Get a clone of the shared state handle for the manager to read.
    pub fn state(&self) -> Arc<Mutex<TerminalState>> {
        Arc::clone(&self.state)
    }
}

impl EventListener for FrostListener {
    fn send_event(&self, event: Event) {
        match event {
            Event::Title(title) => {
                if let Ok(mut s) = self.state.lock() {
                    s.title = Some(title);
                }
            }
            Event::ResetTitle => {
                if let Ok(mut s) = self.state.lock() {
                    s.title = None;
                }
            }
            Event::Bell => {
                if let Ok(mut s) = self.state.lock() {
                    s.bell_at = Some(Instant::now());
                }
            }
            // Wakeup / MouseCursorDirty / CursorBlinkingChange / Exit /
            // ChildExit don't need handling here — the manager owns
            // those signals via its own reader/state channels. PtyWrite,
            // ClipboardStore, ClipboardLoad, ColorRequest and
            // TextAreaSizeRequest are deliberately dropped: surfacing
            // them would let untrusted child output drive arbitrary
            // host-side actions (clipboard hijack, terminal-spoofing
            // queries). Future opt-in modes can add them behind a flag.
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn title_event_is_captured() {
        let listener = FrostListener::dummy();
        let state = listener.state();
        listener.send_event(Event::Title("hello".into()));
        let s = state.lock().unwrap();
        assert_eq!(s.title.as_deref(), Some("hello"));
    }

    #[test]
    fn reset_title_clears_title() {
        let listener = FrostListener::dummy();
        let state = listener.state();
        listener.send_event(Event::Title("hello".into()));
        listener.send_event(Event::ResetTitle);
        let s = state.lock().unwrap();
        assert!(s.title.is_none());
    }

    #[test]
    fn bell_event_records_timestamp() {
        let listener = FrostListener::dummy();
        let state = listener.state();
        assert!(state.lock().unwrap().bell_at.is_none());
        listener.send_event(Event::Bell);
        assert!(state.lock().unwrap().bell_at.is_some());
    }

    #[test]
    fn ignored_events_do_not_panic() {
        let listener = FrostListener::dummy();
        listener.send_event(Event::Wakeup);
        listener.send_event(Event::MouseCursorDirty);
        listener.send_event(Event::CursorBlinkingChange);
        listener.send_event(Event::Exit);
        listener.send_event(Event::PtyWrite("anything".into()));
    }
}
