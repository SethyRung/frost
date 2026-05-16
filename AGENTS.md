# AGENTS.md — Frost

## Project

Terminal UI for managing local dev services across multiple projects. Keyboard-driven dashboard to start/stop/monitor dev servers, grouped by project.

The **Rust workspace** (root `Cargo.toml`) is the active codebase. `ts/` is a legacy TypeScript implementation (has its own `AGENTS.md`).

## Workspace Layout

| Path | Role |
|---|---|
| `crates/frost-core` | Library crate — config loading, process management (PTY), state persistence, theme system |
| `crates/frost-tui` | Binary crate (`frost`) — ratatui TUI, event loop, UI components |
| `themes/*.json` | 33 built-in opencode-compatible themes, embedded at compile time |
| `frost.toml` | Example/user config (project-local) |

Binary entrypoint: `crates/frost-tui/src/main.rs`.

## Commands

| Command | Purpose |
|---|---|
| `cargo run` | Build and run the TUI |
| `cargo test` | Run all tests (inline `#[cfg(test)]` modules) |
| `cargo test -p frost-core` | Test only the core crate |
| `cargo fmt` | Format Rust sources |
| `cargo clippy` | Lint Rust sources |
| `taplo fmt` | Format TOML files (`.taplo.toml` config: 2-space indent, 100 col width) |
| `taplo check` | Validate TOML without changes |

Install taplo: `cargo install taplo-cli --locked`

No CI workflows exist yet.

## Rust Specifics

- **Edition 2024** — requires Rust 1.85+.
- **Workspace resolver 3** (`resolver = "3"` in root `Cargo.toml`).
- Tests use `tempfile` for isolated filesystem fixtures. No external test runner.

## Config Format

Rust version reads **TOML** (`frost.toml`) first, falls back to `frost.json`. Walks up from cwd to find config.

Key schema: `FrostConfig { projects: HashMap<String, ProjectConfig> }` where each project has `apps` with `command` or `commands` (named sub-commands). See `crates/frost-core/src/config/schema.rs`.

Workdir resolution chain: sub-command → app → project → config file directory.

## Architecture Notes

- **Process I/O**: Uses `portable-pty` + `alacritty_terminal` for real PTY-backed terminal emulation — not simple pipe capture. This gives full ANSI/VT rendering.
- **Screen updates**: `ProcessManager` broadcasts `ScreenUpdate` via `tokio::broadcast`. The TUI subscribes and redraws on new output or a 250ms tick.
- **Race condition guard**: `generation_id` on processes prevents stale exit callbacks from overwriting newer spawn state.
- **Theme system**: `Registry → Resolver → Store`. Resolver expands `defs`, selects `dark`/`light` mode, resolves accent aliases, and parses hex to RGBA. Compatible with opencode `ThemeJson` format.
- **State persistence**: `~/.frost/state.json` via `StateStore` with 500ms debounced writes (same format as TS version).

## Style

- Rust: `cargo fmt` defaults. No custom rustfmt config.
- TOML: `.taplo.toml` — 2-space indent, 100 column width, `reorder_keys = false` (except dependency tables which are sorted).
