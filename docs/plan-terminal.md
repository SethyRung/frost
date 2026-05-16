# Plan — Real Terminal in the Log Tab

Upgrade the **Log (interactive)** tab from a styled-text grid to a full terminal that behaves like macOS Terminal.app / iTerm2: real foreground+background colors, cursor, mouse, selection, copy/paste, accurate resize, scrollback, bell, OSC titles/hyperlinks, wide-char correctness, and full xterm key encodings.

---

## Baseline (what already works)

- PTY spawning via `portable-pty` with `setsid sh -c <cmd>` and `TERM=xterm-256color` (`crates/frost-core/src/process/pty.rs`).
- `alacritty_terminal::Term<VoidListener>` handles ANSI/VT, true-color, cursor, scrollback (`crates/frost-core/src/process/manager.rs`).
- Stdin forwarding plumbed: `ProcessManager::write_stdin` → `PtyProcess::write_stdin` (`manager.rs:282`, `pty.rs:38`).
- Key-to-bytes encoder covers arrows, Home/End/PgUp/PgDn, F1–F12, Ctrl+letter, Alt+letter, Backspace (`crates/frost-tui/src/input.rs:94`).
- Crossterm raw mode + `EnableMouseCapture` + alternate screen already entered in `main.rs:54`.
- `TerminalCell` already carries `bg: RGBA` (`crates/frost-core/src/process/types.rs:48`) — just unused by the renderer.
- `ProcessManager::get_cursor_position` exists but log viewer ignores it.

## Gaps (what's missing for a full terminal)

| # | Area | Symptom |
|---|---|---|
| 1 | Background color | bg fields read from grid never rendered |
| 2 | Cursor | no block/bar/underline drawn, no blink, no hide/show |
| 3 | Cell attributes | reverse, dim, blink, strikethrough, hidden ignored |
| 4 | Wide chars | CJK / emoji clipped because right-half cells are double-drawn |
| 5 | Mouse | wheel scroll, click-to-position, drag selection not handled |
| 6 | Selection / copy | no visual selection; no Cmd+C / Ctrl+Shift+C to system clipboard |
| 7 | Paste | Cmd+V / Ctrl+Shift+V not handled; no bracketed-paste host enable |
| 8 | Scrollback | focused mode forwards all keys to PTY — no way to scroll history |
| 9 | Resize accuracy | `resize_all_processes` guesses `width-30, height-3` — wrong if sidebar resizes |
| 10 | Bell | `\x07` swallowed — no visual flash or audible signal |
| 11 | OSC titles / hyperlinks | window-title, OSC 8 hyperlinks, OSC 52 clipboard ignored |
| 12 | Mouse-app protocol | child apps requesting mouse (vim, htop, less) get nothing |
| 13 | Key encoding | Shift+Enter, Ctrl+Enter, modifyOtherKeys / kitty proto, Alt+arrow ignored |
| 14 | Signal vs quit | Ctrl+C always quits TUI — should send SIGINT to child when log focused |
| 15 | Focus reporting | `\x1B[?1004h` requests not answered |
| 16 | URL detection | no Cmd+click open |

---

## Design decisions (resolve before coding)

1. **Render approach** — Stay with ratatui `buf.set_string` per cell. Reason: existing infra, ratatui already does Unicode width. *Alternative*: ratatui-image or a custom dump-pixel widget — rejected as YAGNI.
2. **Selection model** — Keep selection state in `frost-tui` (UI concern), not `frost-core`. Selection ranges over the visible grid + scrollback indices.
3. **Clipboard crate** — `arboard` (cross-platform, macOS pasteboard via AppKit, no extra perms).
4. **Mouse forwarding modes** — Use `alacritty_terminal`'s mode flags (`Mode::MOUSE_REPORT_CLICK`, `MOUSE_DRAG`, `MOUSE_MOTION`, `SGR_MOUSE`, etc.) the emulator already tracks. Forward when set; otherwise consume locally for scroll/selection.
5. **Ctrl+C semantics** — When log focused: send `\x03` to child (real SIGINT-via-tty). Quit moves to `Ctrl+Q` only when log focused. Sidebar focus keeps `Ctrl+C` = quit.
6. **Scrollback access in focus mode** — Shift+PgUp / Shift+PgDn / Shift+Home / Shift+End reserved for host scrollback; everything else forwards to PTY. Mouse wheel scrolls history when child has no mouse mode active.
7. **Cursor blink** — 500ms blink driven by a `tokio::time::interval` that pushes a `Tick` action.
8. **Performance ceiling** — Re-render on `ScreenUpdate` debounced at 60fps (16ms). Selection drag and cursor blink reuse the same debounce.

---

## Phase A — Rendering fidelity (`frost-tui`)

Goal: pixel-equivalent to alacritty for static frames.

### A1. Background color + reverse video
- Edit `log_viewer.rs:92-107`: extend `Span::styled` style with `.bg(rgba_to_ratatui(cell.bg))`.
- Add `reverse: bool` to `TerminalCell`; populate in `extract_lines` from alacritty `Flags::INVERSE`.
- Tests: snapshot via `insta` on a fixture grid with mixed fg/bg and a reverse run.

### A2. Cursor rendering
- Read `pm.get_cursor_position(...)` in `LogViewer::from_manager`; pass `cursor: Option<(usize, usize)>`.
- In `render`, after laying out cells, overlay cursor cell with style swap (fg↔bg) when blink_on.
- Add `Cursor` shape enum (`Block`, `Bar`, `Underline`) sourced from alacritty `CursorShape`.
- Tick action: spawn one-shot `tokio::spawn` loop pushing `Action::CursorBlink` every 500ms; toggle a bool in `state.rs`.

### A3. Full cell attributes
- Extend `TerminalCell` with `dim`, `blink`, `strikethrough`, `hidden`.
- Map in `extract_lines` from alacritty `Flags`.
- Apply via ratatui `Modifier` bits. `hidden` → replace char with space, keep bg.

### A4. Wide-char correctness
- Use `unicode-width::UnicodeWidthChar` (already a transitive dep of ratatui).
- In `extract_lines`, when a cell has width 2, emit one `TerminalCell` and a placeholder marker; renderer skips the placeholder column.

### A5. Empty / starting / crashed states
- Replace the "No process selected" string with three explicit states using existing `ProcessStatus`. No styling regressions.

---

## Phase B — Accurate resize (`frost-tui` + `frost-core`)

Goal: PTY cols/rows match the inner area of the log viewer exactly, recomputed on every layout pass.

### B1. Pipe inner area back to ProcessManager
- `app.rs` already computes pane chunks during `draw`. Capture the log pane's inner `Rect` into `App.state.log_pane_size`.
- Replace `resize_all_processes(width, height)` with `resize_active_process(cols, rows)` called only when the selected process's pane size changes (debounced to avoid resize storms during continuous drag).
- Inactive processes resize lazily on selection.

### B2. SIGWINCH delivery
- `portable-pty` resize already triggers SIGWINCH on the child. Verify by spawning `bash` + running `tput cols` after resize in an integration test (`tests/resize_test.rs`).

---

## Phase C — Input completeness (`frost-tui`)

Goal: every key combo macOS Terminal sends, frost sends.

### C1. Extend `key_to_bytes`
- Add Shift+Enter (`\x1B\r` or kitty if enabled), Ctrl+Enter, Alt+arrows (`\x1B[1;3A` etc.), Shift+Tab (`\x1B[Z`).
- Add Ctrl+Shift+letter via modifyOtherKeys CSI-u encoding when DEC mode 1036/1039 active.
- Table-driven tests: `(KeyCode, KeyModifiers) -> &[u8]` fixture matched against xterm reference.

### C2. Ctrl+C semantics swap
- In `handle_log_viewer_key`: `Ctrl+C` → `Action::WriteInput(vec![0x03])`.
- Quit binding when log focused: `Ctrl+Q`.
- Update `command_bar.rs` to reflect bindings depending on focus.

### C3. Bracketed paste
- Enable on host: `execute!(stdout, EnableBracketedPaste)` in `main.rs`.
- Handle `Event::Paste(text)`: wrap in `\x1B[200~ ... \x1B[201~` only when child has DEC mode 2004 enabled (read from alacritty `Mode::BRACKETED_PASTE`); otherwise send raw.

### C4. Cmd+V / Ctrl+Shift+V → paste
- On macOS, Cmd does not arrive via crossterm. Use `Ctrl+Shift+V` as the documented paste shortcut; document Cmd+V as Terminal.app's responsibility (no fix possible from inside a TUI).

---

## Phase D — Mouse (`frost-tui`)

### D1. Wire mouse events through dispatch
- Extend `Action` with `Mouse(MouseEvent)`.
- `handle_event` for `Event::Mouse(_)` produces it.
- `App::handle_action` routes based on focus and active emulator mouse mode.

### D2. Mouse-app forwarding
- Expose `Term::mode()` from `ProcessManager::get_mouse_modes(...)`.
- If `SGR_MOUSE` + (`MOUSE_REPORT_CLICK` | `MOUSE_DRAG` | `MOUSE_MOTION`) active → encode SGR `\x1B[<b;x;yM/m` and forward to PTY.
- Else handle locally (selection + wheel scroll).

### D3. Wheel scroll → scrollback
- When child has no mouse mode active, scroll wheel adjusts `log_scroll` in 3-line steps.

---

## Phase E — Selection, copy, paste (`frost-tui`)

### E1. Selection state
- New `selection.rs`: `Selection { anchor: GridPoint, head: GridPoint, mode: Char | Word | Line }`.
- Mouse drag updates `head`. Shift+click extends. Double-click = word, triple = line.

### E2. Selection rendering
- During render, swap fg/bg for cells inside the selection range. Reuse the reverse-video path from A1.

### E3. Copy to clipboard
- Add `arboard` to `frost-tui` deps.
- `Ctrl+Shift+C` → flatten selection to string (skip trailing spaces, join soft-wrapped rows), push to `arboard::Clipboard::set_text`.

### E4. OSC 52 clipboard requests
- When child emits OSC 52, alacritty surfaces it via the listener. Currently `VoidListener` discards. Switch to a small `FrostListener` that pushes events into a channel; TUI honors clipboard requests behind a `allow_osc52_clipboard` toggle (security-sensitive; default off).

---

## Phase F — Scrollback (`frost-tui` + `frost-core`)

### F1. Expose scrollback lines from manager
- Add `ProcessManager::get_scrollback_lines(key, range: Range<usize>) -> Vec<DisplayLine>`.
- Reuse `Term::grid().display_iter()` walking the history buffer.

### F2. Scrollback navigation
- Shift+PgUp/PgDn step by half-page; Shift+Home/End jump.
- `log_scroll` becomes an offset from the bottom of scrollback (currently it's an offset from the live grid only).
- "(scrolled)" title already exists — keep.

---

## Phase G — Bell, title, hyperlinks (`frost-core` + `frost-tui`)

### G1. Bell
- Replace `VoidListener` with `FrostListener`. On `Listener::bell()`, broadcast `StateEvent::Bell { key }`.
- TUI flashes border for 150ms; optional `print!("\x07")` to host terminal (config flag).

### G2. OSC 0/2 — window title
- `FrostListener::set_title(s)` updates `ProcessInfo.title`. Sidebar shows current title next to app name (truncated).

### G3. OSC 8 — hyperlinks
- alacritty exposes `cell.hyperlink()`. Store in `TerminalCell::hyperlink: Option<Arc<str>>`. Render with underline. Cmd-click handler in Phase D forwards URL to `open` (macOS) — feature-gated.

---

## Phase H — Misc polish

- **H1. Focus reporting** — emit `\x1B[I` / `\x1B[O` on TUI focus gain/loss when child enables DEC 1004.
- **H2. Cursor style requests** — honor DECSCUSR (`\x1B[Ps q`) updates from emulator state.
- **H3. Reflow on resize** — alacritty handles in-grid reflow; verify scrollback line wrap survives a resize-down-then-up sequence (integration test).
- **H4. Performance** — coalesce `ScreenUpdate`s arriving within 8ms; profile with `cargo flamegraph` on a `yes | head -1000000` run.
- **H5. Docs** — update `docs/project.md` "Terminal emulator" section + `CLAUDE.md` Architecture data flow step 5.

---

## Test strategy

| Phase | Test type | Fixture |
|---|---|---|
| A1–A4 | Unit + insta snapshot | Hand-built `Term` driven with known byte sequence (`\x1B[31;42mX\x1B[0m`, wide-char, reverse-video) |
| B | Integration | Spawn `bash -c "stty size && sleep 0.1 && stty size"` after resize, assert reported dims |
| C1 | Table-driven unit | xterm reference table |
| C2–C3 | Integration | Spawn `cat`, send Ctrl+C, assert child exited via SIGINT; send paste, assert bracketed wrappers when mode 2004 set |
| D | Unit | Synth `MouseEvent`, assert encoded bytes match `\x1B[<0;5;3M` |
| E | Unit | Selection across wrapped lines flattens correctly |
| F | Integration | Spawn `seq 1 1000`, scroll up, assert line 1 visible |
| G | Integration | Spawn `printf '\x07'` → assert bell event fires |

Coverage target: 80% on `frost-tui::log_viewer`, `frost-tui::input`, `frost-tui::selection`, `frost-core::process::manager`.

---

## Rollout order (suggested PR sequence)

1. **PR1 — A1 + A2** (bg color + cursor) — biggest visible win, smallest risk. ✅
2. **PR2 — A3 + A4** (full attributes + wide chars). ✅
3. **PR3 — B** (accurate resize) — unlocks correct rendering for child apps that read `$COLUMNS`. ✅
4. **PR4 — C** (input completeness + Ctrl+C semantics). ✅
5. **PR5 — D** (mouse + mouse-app forwarding). ✅
6. **PR6 — E1 + E2 + E3** (selection + clipboard via arboard). ✅
7. **PR7 — F** (scrollback nav). ✅
8. **PR8 — G1 + G2** (bell + window title via `FrostListener`). ✅ Hyperlinks (G3) + Phase H polish (focus reporting, cursor style, reflow tests, render coalescing) still open.

Each PR independently usable. Stop after PR3 = "real interactive terminal for typical CLI". Continue through PR8 = "full Mac terminal parity".

### Remaining work (not yet a PR)

- **G3 hyperlinks** — extend `TerminalCell` with `hyperlink: Option<Arc<str>>` from `alacritty` cell hyperlinks; render with underline; Cmd-click → `open <url>` on macOS.
- **E4 OSC 52 clipboard requests** — currently dropped by `FrostListener` for security; add an opt-in `allow_osc52_clipboard` flag that hands `ClipboardStore` events to `arboard`.
- **H1 focus reporting** (DEC 1004), **H2 cursor style** (DECSCUSR), **H3 reflow assertions**, **H4 16 ms render coalescing**.
- **Word / line selection modes** (double / triple click) — timing-based; needs click history.
- **PTY-master TIOCSWINSZ → slave** propagation on macOS — see PR3 test note. `MasterPty::get_size` confirms the master fd updates but `stty size` in the spawned shell still reports stale dims; deeper portable-pty / Darwin investigation needed.

---

## Out of scope

- GPU rendering / sixel / kitty graphics (would require a new render backend).
- True-pixel font ligatures (ratatui is cell-based).
- Tabs inside the log pane (separate feature — multiple panes per app).
- ssh/mosh-style remote PTY (frost is local-only).
