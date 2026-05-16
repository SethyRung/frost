# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project

Frost is a Rust TUI for managing local dev services across multiple projects (start/stop/monitor dev servers from a keyboard-driven dashboard). Active work is on branch `rewrite/rust` — a complete rewrite of the legacy TypeScript MVP under `ts/`. Only the Rust workspace at the repo root is the live codebase; `ts/` is archived reference material.

## Workspace Layout

Cargo workspace, edition 2024, resolver 3, two crates under `crates/`:

- **`frost-core`** — Pure library, zero TUI dependencies. Holds config loading, process management, state persistence, and theme resolution. Designed to be reused by a future headless CLI/API.
- **`frost-tui`** — Binary named `frost` (`cargo run` entrypoint). Depends on `frost-core` and brings in `ratatui` + `crossterm`. Owns the event loop, widgets, and input handling.

### Tech stack

| Layer         | Crate                                  |
| ------------- | -------------------------------------- |
| TUI           | ratatui + crossterm                    |
| Process I/O   | portable-pty + alacritty_terminal      |
| Config        | TOML via `toml`                        |
| Process mgmt  | nix                                    |
| Async runtime | tokio                                  |
| Themes        | 33 opencode JSON themes — `serde_json` |

Themes live as JSON files at workspace root in `themes/` (33 opencode-compatible theme files). Embedded into `frost-core` at compile time via `include_str!`.

### TUI source map (`crates/frost-tui/src/`)

| File              | Role                                                                |
| ----------------- | ------------------------------------------------------------------- |
| `main.rs`         | `#[tokio::main]` entrypoint — boots config, state, TUI loop         |
| `app.rs`          | Central `App` — owns `ProcessManager`, runs action dispatch         |
| `state.rs`        | TUI-local UI state (selection, expansion, focus)                    |
| `actions.rs`      | `Action` enum — all state mutations flow through this               |
| `input.rs`        | crossterm event → `Action` translation                              |
| `sidebar.rs`      | Project/app/sub-command tree widget with status icons               |
| `log_viewer.rs`   | Renders `ProcessManager::get_display_lines()` as styled spans       |
| `command_bar.rs`  | Footer key-hint bar                                                 |
| `palette.rs`      | `Ctrl+P` command palette overlay                                    |
| `search.rs`       | `/` search overlay                                                  |
| `theme_dialog.rs` | Theme picker overlay                                                |

### Authoritative docs

- `docs/plan.md` + `docs/project.md` — implementation plan and design rationale; consult before architectural changes. Phase 1–4 fully checked, Phase 5 (Polish) 0/5.
- `AGENTS.md` at repo root mirrors much of this file in table form. When the two diverge, **CLAUDE.md wins** — keep `AGENTS.md` updated as a parallel summary, not a source of truth.

## Common Commands

```bash
# Build & run the TUI (requires a frost.toml in cwd or an ancestor dir)
cargo run

# Build everything
cargo build
cargo build --release

# Fast compile-check (no codegen) — preferred quick verify
cargo check

# Test everything (workspace-wide)
cargo test

# Run a single test by name (substring match)
cargo test test_pty_spawn_terminal_emulator_and_kill

# Run only tests in one crate
cargo test -p frost-core
cargo test -p frost-tui

# Show println! output during tests
cargo test -- --nocapture

# Format Rust + lint
cargo fmt
cargo clippy -- -D warnings

# Format/lint TOML (taplo config in .taplo.toml — install: cargo install taplo-cli --locked)
taplo fmt
taplo lint
taplo check
```

`cargo run` requires a `frost.toml` discoverable by walking up from cwd. The repo ships a sample `frost.toml` at the root pointing at sibling project directories (`../portfolio`, `../Chongkran`, `../WebBridge`) — those won't exist on a fresh checkout, so for development create or point at your own config.

## Architecture (big picture)

### Data flow

1. **Config discovery** — `find_config()` walks up from cwd looking for `frost.toml`. `load_config()` parses via `toml` + `serde`.
2. **Config flattening** — `flatten_config()` expands the nested `[projects.X.apps.Y.commands.Z]` tree into a flat `Vec<RuntimeCommand>` of spawnable units. Legacy single-`command` apps are normalised to a single sub-command named `"default"`. Workdir resolution chain: subcommand → app → project → config dir.
3. **Process spawn** — `process::pty::spawn_pty()` opens a real PTY via `portable-pty`, runs the command as `setsid sh -c <command>` so the child sees `isatty() == true` and gets its own process group. `TERM=xterm-256color` is set (no `FORCE_COLOR` hacks needed — the PTY makes colors native).
4. **Terminal emulation** — `ProcessManager` keeps one `alacritty_terminal::Term<FrostListener>` per process. A blocking reader task reads PTY bytes and feeds them into `Processor::advance(&mut term, bytes)`. The `Term` maintains a styled cell grid that handles ANSI/VT, `\r` overwrites, cursor moves, screen clears, true-color, etc. — there is no custom ANSI parser. `FrostListener` (`crates/frost-core/src/process/listener.rs`) captures out-of-band events: OSC 0/2 window titles and BEL ring into a per-process `TerminalState`. Clipboard / color / PTY-write requests from the child are deliberately ignored for security.
5. **TUI rendering** — The log viewer reads `ProcessManager::get_display_lines()` (extracted from the `Term` grid) and renders each cell as a styled `ratatui::text::Span`. The `Sidebar` reads `pm.list()` / `pm.get_info()` for status icons.
6. **Event broadcast** — `tokio::sync::broadcast` channels publish `ScreenUpdate` (output arrived) and `StateEvent` (process started/stopped/crashed). The TUI subscribes to drive re-renders.
7. **State persistence** — `frost-core/src/state/mod.rs` defines `FrostState` (version, `active_theme`, `theme_mode`, `last_project`, `expanded_apps`, per-app status map). Serialised as JSON to `.frost/state.json` relative to the config dir (`STATE_DIR = ".frost"`, `STATE_FILE = ".frost/state.json"`, `CURRENT_VERSION = 1`). Bump `CURRENT_VERSION` on schema breaks and add migration logic.

### Key invariants & gotchas

- **One sub-command per app at a time.** `App::start_process` first stops any other running subcommand in the same `(project, app)` before spawning. Selecting a different sub-command stops the current one.
- **Process group kill, not `child.kill()`.** Use `kill(Pid::from_raw(-pid), SIGTERM)` (`PtyProcess::kill_process_group`). Plain `child.kill()` only kills the `sh` parent and orphans the real dev server. After 5s grace, the manager sends `SIGKILL`.
- **`generation_id` race guard.** When a process restarts quickly, the old reader's exit event must not overwrite the new process's status. Every spawn gets a monotonic `generation_id`; consumers must verify it matches before applying state transitions.
- **PTY resize must mirror to the emulator.** `ProcessManager::resize` calls both `pty.resize()` and `term.resize()` — drifting between them will corrupt the grid.
- **Process key is the tuple `(project, app, subcommand)`.** This is the canonical identity throughout `ProcessManager`, broadcast events, and the TUI's `selected_process`.

### TUI dispatch

`frost-tui` uses a Redux-style central dispatch: crossterm events → `input::handle_event()` → `Action` enum → `App::handle_action()` → state mutation → next `draw()`. Overlays (`Palette`, `Search`, `ThemeDialog`) capture all input while open and have their own action handling in `App::handle_overlay_action`. Overlay rendering is absolutely-positioned via `Frame::render_widget(.., area)` over the full frame area.

### Theme system

Compatible with opencode's `ThemeJson` format (same JSON files). Pipeline: `ThemeRegistry` (holds all `ThemeJson` defs) → `resolve_theme()` (expands `defs`, picks dark/light mode, parses hex, resolves references like `"primary"`) → `ResolvedTheme` (flat RGBA map). `ThemeStore` caches resolved themes and notifies subscribers via `broadcast`. For ratatui, convert `RGBA` → `ratatui::style::Color` in the TUI crate (Phase 5).

## Coding conventions specific to this repo

- **Edition 2024** and resolver 3 are set workspace-wide — don't downgrade.
- `frost-core` must remain free of any TUI dependency. Anything tied to `ratatui`/`crossterm` belongs in `frost-tui`.
- New embedded themes go in `themes/*.json` plus an `include_str!` entry in `frost-core/src/theme/builtin.rs`.
- Process-manager async work runs on `tokio` (`#[tokio::main]` in `frost-tui::main`, `tokio::spawn` for reader tasks, `tokio::sync::broadcast` for fan-out, `tokio::time::sleep` for the SIGTERM→SIGKILL grace period).
- Process I/O readers are blocking (`std::io::Read` on the PTY master) wrapped in `tokio::spawn` — do not switch them to async readers without first verifying `portable-pty`'s async story on macOS.
- `frost-tui` uses `anyhow` for error context; `frost-core` uses `thiserror` for typed errors (e.g. `ProcessError`, `ConfigError`, `ResolveError`).

## Legacy TypeScript code

`ts/` contains the prior Bun/OpenTUI MVP. Treat it as read-only reference — `docs/plan.md` calls out which behaviours were ported and which were intentionally changed (e.g. pipes → PTY, JSONC → TOML, single command → named sub-commands). Don't add new features there.
