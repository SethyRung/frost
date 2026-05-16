# Frost

Terminal UI for managing local dev services across multiple projects.
Start, stop, and monitor your dev servers from one keyboard-driven dashboard.

> **Status**: In development. See [plan](docs/plan.md) for the implementation roadmap.

---

## Overview

- **Project-grouped stacks** — organize apps by project, toggle entire stacks
- **Named sub-commands** — `dev`, `build`, `lint` per app, switch between them
- **Real terminal logs** — PTY-backed log viewer with full ANSI/VT rendering
- **Live theme switching** — 33 built-in themes, compatible with opencode theme files
- **Keyboard-driven** — `↑↓` navigate, `Enter` toggle, `Ctrl+P` palette, `/` search

## Quickstart

```bash
cargo run
```

## Configuration

Create a `frost.toml` in your project root:

```toml
[projects.portfolio]
workdir = "../portfolio"
# Optional glyph rendered next to the project name in the sidebar.
# Use any single nerd-font codepoint or short unicode string. TUIs
# cannot render raster/SVG; this is the per-project icon hook.
icon = ""

[projects.portfolio.apps.frontend]
workdir = "./frontend"
default = "dev"
icon = ""

[projects.portfolio.apps.frontend.commands.dev]
command = "pnpm dev"

[projects.portfolio.apps.frontend.commands.build]
command = "pnpm build"

[projects.portfolio.apps.frontend.commands.lint]
command = "pnpm lint"
```

## Tech Stack

| Layer         | Crate                                  |
| ------------- | -------------------------------------- |
| TUI           | ratatui + crossterm                    |
| Process I/O   | portable-pty + alacritty_terminal      |
| Config        | TOML via `toml`                        |
| Process mgmt  | nix                                    |
| Async runtime | tokio                                  |
| Themes        | 33 opencode JSON themes — `serde_json` |

## Docs

| File                                 | Content                               |
| ------------------------------------ | ------------------------------------- |
| [`docs/project.md`](docs/project.md) | Architecture and design decisions     |
| [`docs/plan.md`](docs/plan.md)       | Implementation plan and crate choices |

## Tooling

| Command | Purpose |
|---|---|
| `cargo fmt` | Format Rust sources |
| `cargo clippy` | Lint Rust sources |
| `taplo fmt` | Format TOML files (`frost.toml`, `Cargo.toml`) |
| `taplo lint` | Validate TOML syntax and style |
| `taplo check` | Check TOML files for errors (no changes) |

Install `taplo`:

```bash
cargo install taplo-cli --locked
```

## License

[MIT](LICENSE)
