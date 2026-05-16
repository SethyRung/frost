# Frost — Rust Rewrite Plan

Complete Rust rewrite from scratch. New TUI design, idiomatic Rust project
structure, `frost.toml` config with named sub-commands, and opencode-compatible
theme system (same JSON files, same resolver logic).

---

## Project Identity (unchanged)

- **Frost** — a terminal UI for managing local dev services across projects.
- Start/stop/monitor multiple project stacks with keyboard-driven navigation.
- No daemon — the TUI is the long-running process.

---

## Project Structure

```
frost/
├── Cargo.toml
├── rust-toolchain.toml
├── frost.toml                     # user config (not in repo)
│
├── crates/
│   ├── frost-core/                # library — zero TUI dependency
│   │   ├── Cargo.toml
│   │   └── src/
│   │       ├── lib.rs
│   │       ├── config/
│   │       │   ├── mod.rs         # types + loader
│   │       │   └── schema.rs      # serde structs
│   │       ├── process/
│   │       │   ├── mod.rs
│   │       │   ├── manager.rs     # ProcessManager
│   │       │   ├── spawner.rs     # spawn + setsid + kill
│   │       │   └── types.rs       # ProcessStatus, LogLine, ProcessInfo
│   │       ├── state/
│   │       │   ├── mod.rs         # StateStore
│   │       │   └── types.rs       # FrostState, AppState
│   │       ├── theme/
│   │       │   ├── mod.rs
│   │       │   ├── types.rs       # ThemeJson, ResolvedTheme, RGBA
│   │       │   ├── resolver.rs    # resolve_theme, resolve_color
│   │       │   ├── registry.rs    # ThemeRegistry
│   │       │   ├── store.rs       # ThemeStore
│   │       │   ├── builtin.rs     # !!include_str! 33 themes
│   │       │   └── system.rs      # generate_system_theme
│   │       └── pty.rs             # PTY spawner (portable-pty)
│   │
│   └── frost-tui/                 # TUI binary — depends on frost-core
│       ├── Cargo.toml
│       └── src/
│           ├── main.rs            # entrypoint, CLI args
│           ├── app.rs             # ratatui App struct (≈ App.tsx)
│           ├── state.rs           # TUI state machine
│           ├── input.rs           # keyboard handler → Actions enum
│           ├── actions.rs         # Action enum (Tick, Quit, Navigate, Toggle, ...)
│           ├── widgets/
│           │   ├── mod.rs
│           │   ├── sidebar.rs     # project/app tree with sub-commands
│           │   ├── log_viewer.rs  # terminal emulator widget (alacritty_terminal)
│           │   ├── command_bar.rs # bottom status bar
│           │   ├── palette.rs     # Ctrl+P command palette
│           │   ├── search.rs      # / fuzzy search
│           │   └── theme_dialog.rs# theme switcher with live preview
│           ├── theme/
│           │   └── mod.rs         # theme → ratatui Color conversion
│           └── util.rs            # misc helpers
│
├── themes/                        # 33 JSON theme files (copied from opencode)
│   ├── opencode.json
│   ├── dracula.json
│   └── ... (31 more)
│
├── docs/
│   ├── project.md
│   └── rust-plan.md              # this file
│
└── ts/                            # legacy TypeScript codebase (archived)
```

**Why a Cargo workspace with two crates:**

- `frost-core` has zero TUI dependencies — it's pure data + process management.
  Could be reused by a future headless CLI, CI integration, or API.
- `frost-tui` depends on `frost-core` and brings in ratatui/crossterm.
- Shared theme JSON files live at workspace root, embedded via `include_str!`.

---

## Config System

### Format: TOML

```toml
# frost.toml

[projects.portfolio]
workdir = "../portfolio"

[projects.portfolio.apps.frontend]
workdir = "./frontend"
default = "dev"

[projects.portfolio.apps.frontend.commands.dev]
command = "bun dev"

[projects.portfolio.apps.frontend.commands.build]
command = "bun run build"

[projects.portfolio.apps.frontend.commands.lint]
command = "bun lint"

# Sub-command with its own workdir (overrides app-level)
[projects.portfolio.apps.frontend.commands.analyze]
command = "bunx analyze"
workdir = "./scripts"

# Legacy shorthand: single-command app — treated as one sub-command "default"
[projects.portfolio.apps.api]
command = "bun serve"
```

### Config Resolution

1. Walk up from cwd looking for `frost.toml` (then `frost.json` fallback).
2. Parse with `toml` crate + `serde`.
3. For each app, resolve sub-commands:
   - If `commands` map present → named sub-commands
   - If only `command` string → single sub-command named `"default"`
   - `default` field → which sub-command runs on toggle. Falls back to
     first alphabetical key if absent.
4. `workdir` resolution chain: sub-command level → app level → project level → config dir.

### Rust Types

```rust
// frost-core/src/config/schema.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FrostConfig {
    pub projects: HashMap<String, ProjectConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProjectConfig {
    pub workdir: Option<String>,
    pub apps: HashMap<String, AppConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub workdir: Option<String>,
    #[serde(default)]
    pub default: Option<String>,
    #[serde(default)]
    pub command: Option<String>,   // legacy shorthand
    #[serde(default)]
    pub commands: Option<HashMap<String, SubCommand>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubCommand {
    pub command: String,
    #[serde(default)]
    pub workdir: Option<String>,
    #[serde(default)]
    pub env: Option<HashMap<String, String>>,  // per-command env vars
}
```

### Flattened Runtime Form

Config is flattened at startup into a list of spawnable units:

```rust
pub struct RuntimeCommand {
    pub project_name: String,      // "portfolio"
    pub app_name: String,          // "frontend"
    pub subcommand_name: String,   // "dev" | "build" | "lint"
    pub command: String,           // "bun dev"
    pub workdir: PathBuf,          // resolved absolute path
    pub is_default: bool,          // true if this is the `default` sub-command
}
```

---

## Process Management

### Architecture: PTY + Terminal Emulator

Instead of piping stdout/stderr line-by-line and parsing ANSI codes
separately, each child process gets a real **pseudo-terminal (PTY)**.
The child sees `isatty() == true` and emits full ANSI sequences
(colors, cursor movements, progress bars, screen clearing) natively.
Frost reads from the PTY master and feeds output into a **terminal
emulator** (`alacritty_terminal`) that maintains a screen grid of
styled cells.

```
┌─ PTY ────────────────────────────────────────────────────┐
│                                                          │
│  Master FD  ←── Frost reads bytes from here              │
│     │                                                    │
│     │  (kernel PTY: bytes pass through transparently)    │
│     │                                                    │
│  Slave FD   →── child process stdin/stdout/stderr        │
│     │                                                    │
│     ▼                                                    │
│  $ bun dev          isatty() → true                      │
│  $ pnpm dev         ANSI colors enabled automatically    │
│  $ cargo watch      \r progress bars work                │
│                     cursor movements, screen clears, etc.│
└──────────────────────────────────────────────────────────┘
                              │
                              ▼  (stream of bytes + ANSI escapes)
┌─ Terminal Emulator (alacritty_terminal) ──────────────┐
│                                                       │
│  Parses: all VT/ANSI sequences                        │
│    - SGR (16/256/true-color fg/bg, bold, italic, ...) │
│    - \r carriage return (progress bar overwrite)      │
│    - CUD/CUU cursor movement                          │
│    - \x1b[2J screen clearing                          │
│    - line wrapping at terminal width                  │
│    - scroll regions, alternate screen                 │
│                                                       │
│  Maintains:  cols × rows grid of cells                │
│    cell { char: char, fg: Rgb, bg: Rgb,               │
│            bold: bool, italic: bool, ... }            │
│  Scrollback: configurable history buffer              │
└───────────────────────────────────────────────────────┘
                              │
                              ▼
┌─ LogViewer Widget ───────────────────────────────────┐
│                                                      │
│  grid rows → ratatui::text::Line                     │
│  styled cells → ratatui::text::Span (with Style)     │
│  renders in Paragraph widget with auto-scroll        │
│  manual scroll up/down through scrollback history    │
└──────────────────────────────────────────────────────┘
```

### Benefits over pipe-based approach

| Feature              | Pipes                                          | PTY + Emulator                          |
| -------------------- | ---------------------------------------------- | --------------------------------------- |
| ANSI colors          | Only if tool respects `FORCE_COLOR`            | Always — isatty() → colors enabled      |
| Progress bars (`\r`) | Each frame rendered as new line                | Same-line overwrite works               |
| Cursor movement      | Not supported                                  | Full CUD/CUU/CUP support                |
| Screen clearing      | Not supported                                  | `\x1b[2J` clears screen grid            |
| True-color (38;2)    | Partial (custom parser)                        | Full — alacritty term                   |
| stderr coloring      | Frost applies theme error color                | Tool's own ANSI colors (more authentic) |
| Env vars needed      | `FORCE_COLOR=3`, `COLORTERM=truecolor`, `TERM` | Only `TERM=xterm-256color`              |
| Custom ANSI parser   | Required (252 lines in TS)                     | Not needed — alacritty handles it       |

### Spawning

Uses `portable-pty` crate for cross-platform PTY creation and `nix`
for process group control:

```rust
// frost-core/src/process/pty.rs

use portable_pty::{PtySize, MasterPty, native_pty_system};
use nix::sys::signal::{kill, Signal};
use nix::unistd::Pid;
use std::process::Command;
use std::os::unix::process::CommandExt;

pub struct PtyProcess {
    pub pid: Pid,
    pub master: Box<dyn MasterPty + Send>,  // Frost reads output from this
    pub reader: Box<dyn std::io::Read + Send>,
}

impl PtyProcess {
    pub fn kill_process_group(&self) -> Result<()> {
        kill(Pid::from_raw(-(self.pid.as_raw())), Signal::SIGTERM)?;
        Ok(())
    }

    /// Resize the PTY — call when terminal window resizes
    pub fn resize(&self, cols: u16, rows: u16) -> Result<()> {
        self.master.resize(PtySize { rows, cols, ..Default::default() })?;
        Ok(())
    }
}

pub fn spawn_pty(command: &str, workdir: &Path, cols: u16, rows: u16) -> Result<PtyProcess> {
    let pty_system = native_pty_system();
    let pty_pair = pty_system.openpty(PtySize {
        rows,
        cols,
        pixel_width: 0,
        pixel_height: 0,
    })?;

    let mut cmd = Command::new("setsid");
    cmd.args(["sh", "-c", command])
        .current_dir(workdir)
        .stdin(pty_pair.slave.try_clone()?)
        .stdout(pty_pair.slave.try_clone()?)
        .stderr(pty_pair.slave.try_clone()?)
        .env("TERM", "xterm-256color")
        // No FORCE_COLOR needed — PTY makes isatty() true
        .process_group(0);

    let child = unsafe {
        cmd.pre_exec(|| {
            // setsid is already called via "setsid" command above.
            // pre_exec runs in the child after fork.
            Ok(())
        })
        .spawn()?
    };

    Ok(PtyProcess {
        pid: Pid::from_raw(child.id() as i32),
        master: pty_pair.master,
        reader: pty_pair.master.try_clone_reader()?,
    })
}
```

### Manager

`ProcessManager` now holds a terminal emulator per process instead of a
ring buffer of log lines. The emulator is `alacritty_terminal::Term<T>`
which handles all ANSI/VT parsing internally.

```rust
// frost-core/src/process/manager.rs

use alacritty_terminal::Term;
use alacritty_terminal::term::Config as TermConfig;

pub struct ProcessManager {
    processes: HashMap<(String, String, String), ProcessState>,
    screen_tx: tokio::sync::broadcast::Sender<ScreenUpdate>,
    state_tx: tokio::sync::broadcast::Sender<StateEvent>,
}

pub struct ProcessState {
    pub pid: Pid,
    pub status: ProcessStatus,
    pub pty: PtyProcess,                       // PTY master for I/O + resize
    pub terminal: Term<EventProxy>,            // terminal emulator screen grid
    pub scrollback: usize,                     // scroll position (0 = bottom)
    pub generation_id: u64,
}
```

**Key behaviors:**

- On spawn: create PTY, initialize `Term` with `col`/`row` from layout, spawn process.
- On output: read bytes from `pty.reader`, feed into `Term.advance_bytes()`.
  Broadcast `ScreenUpdate` to TUI so the widget re-renders the updated grid.
- On stop: SIGTERM to process group, 5-second timeout, mark `Stopped`.
- On restart: stop → create new PTY/Term → start.
- On terminal resize: `pty.resize(cols, rows)` + `Term.resize(cols, rows)`.
- Exit codes: 0 / 143 (SIGTERM) / null → `Stopped`. Other → `Crashed`.
- Race condition guard: `generation_id` check in exit callback.

**Memory**: 80×24 terminal grid ≈ 38 KB. 1000-line scrollback ≈ 1.6 MB.
Ten running apps ≈ 16 MB — negligible.

---

## Theme System

### Compatibility with OpenCode

Frost's theme system reads and resolves the same `ThemeJson` format as opencode.
Theme files are **identical** — the 33 built-in JSON files are a direct copy.

**What's shared:**

- `ThemeJson` schema (`$schema`, `defs`, `theme`)
- Color value format: hex, ANSI integer, `"none"`, dark/light variants,
  reference strings to defs or other theme keys
- ~50 resolved color slots (primary, secondary, text, background*, border*, diff*, markdown*, syntax\*)
- Resolution pipeline: defs expansion → mode selection → hex parsing →
  reference resolution → RGBA output

**What Frost adds (Rust-specific):**

- `ResolvedTheme → ratatui::style::Color` conversion
- Theme store with persistence to `~/.frost/state.json`
- System theme generation from terminal palette (same algorithm, ported from `system.ts`)

### Rust Types

```rust
// frost-core/src/theme/types.rs

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThemeJson {
    #[serde(rename = "$schema", default, skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub defs: Option<HashMap<String, ThemeDefValue>>,

    pub theme: HashMap<String, ThemeValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeValue {
    String(String),
    Number(f64),
    Variant { dark: String, light: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ThemeDefValue {
    String(String),
    Number(f64),
}

#[derive(Debug, Clone, Copy)]
pub struct RGBA {
    pub r: f32,  // 0.0–1.0
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Debug, Clone)]
pub struct ResolvedTheme {
    pub primary: RGBA,
    pub secondary: RGBA,
    pub accent: RGBA,
    pub error: RGBA,
    pub warning: RGBA,
    pub success: RGBA,
    pub info: RGBA,
    pub text: RGBA,
    pub text_muted: RGBA,
    pub background: RGBA,
    pub background_panel: RGBA,
    pub background_element: RGBA,
    pub background_menu: RGBA,
    pub border: RGBA,
    pub border_active: RGBA,
    pub border_subtle: RGBA,
    pub selected_list_item_text: RGBA,
    // ... diff*, markdown*, syntax* fields
    pub thinking_opacity: f64,
}
```

### Theme Store

```rust
// frost-core/src/theme/store.rs

pub struct ThemeStore {
    registry: ThemeRegistry,              // all loaded themes
    active: String,                       // active theme ID
    mode: ThemeMode,                      // Dark | Light
    lock: Option<ThemeMode>,             // force a mode
    resolved_cache: HashMap<(String, ThemeMode), ResolvedTheme>,
    persist_path: Option<PathBuf>,       // ~/.frost/state.json
}

impl ThemeStore {
    pub fn get_active(&self) -> &str;
    pub fn set(&mut self, id: &str);
    pub fn switch_mode(&mut self, mode: ThemeMode);
    pub fn get_all(&self) -> &HashMap<String, ThemeJson>;
    pub fn get_resolved(&mut self) -> &ResolvedTheme;
    pub fn subscribe(&self) -> tokio::sync::broadcast::Receiver<()>;
}
```

### Built-in Themes

33 themes embedded at compile time via `include_str!`:

```rust
// frost-core/src/theme/builtin.rs

pub fn builtin_themes() -> HashMap<String, ThemeJson> {
    let mut themes = HashMap::new();

    macro_rules! load_theme {
        ($name:expr, $path:expr) => {
            let json: ThemeJson = serde_json::from_str(include_str!($path))
                .expect(concat!("Failed to parse built-in theme: ", $name));
            themes.insert($name.to_string(), json);
        };
    }

    load_theme!("opencode",    "../../themes/opencode.json");
    load_theme!("dracula",     "../../themes/dracula.json");
    // ... 31 more
    themes
}
```

---

## TUI Architecture

### Stack

- **ratatui** — immediate-mode TUI framework (widgets, layout, styling)
- **crossterm** — terminal backend (raw mode, input, cursor)
- **alacritty_terminal** — ANSI/VT parser + screen grid (for log viewer)
- **portable-pty** — pseudo-terminal for child process I/O

### Event Loop

ratatui uses an event loop, not React's declarative rendering:

```
┌─ main.rs ──────────────────────────────────────────────┐
│ 1. Parse CLI args (config path, theme override)        │
│ 2. Load config                                         │
│ 3. Initialize ProcessManager                           │
│ 4. Initialize ThemeStore                               │
│ 5. Restore StateStore                                  │
│ 6. Build App state                                     │
│ 7. Run event loop:                                     │
│    loop {                                              │
│        handle_events(input, process_events) → Action   │
│        update(app, action)                             │
│        draw(frame, app)                                │
│    }                                                   │
│ 8. Cleanup (stop processes, save state, restore term)  │
└────────────────────────────────────────────────────────┘
```

### Actions (Redux-style central dispatch)

```rust
// frost-tui/src/actions.rs

pub enum Action {
    // Navigation
    MoveUp,
    MoveDown,
    FocusSidebar,
    FocusLogs,
    ToggleFocus,

    // Process control
    ToggleApp,              // toggle default sub-command (start/stop)
    ToggleProject,          // toggle all apps in project (default sub-command)
    RestartApp,             // restart currently running sub-command
    SelectSubCommand(usize), // stop current, start selected sub-command

    // Overlays
    OpenPalette,
    OpenSearch,
    OpenThemes,
    CloseOverlay,

    // Palette actions
    SwitchTheme(String),
    ReloadConfig,

    // Search
    SearchSelect(usize),

    // System
    Quit,
    Tick,                   // periodic render
}
```

### Layout (Flexbox via ratatui `Layout`)

```
┌────────────────────────────────────────────────────────────┐
│  Sidebar (30 cols)         │  Log Viewer (flex)            │
│                            │                               │
│  Projects & Apps           │  ┌─Logs: p/frontend─────┐     │
│                            │  │ $ bun dev            │     │
│  portfolio                 │  │ → Local: ...         │     │
│    ▸ frontend   ● dev      │  │ ✓ ready in 65ms      │     │
│      dev      running ●    │  │                      │     │
│      build    stopped ○    │  │ ... (scrolls)        │     │
│      lint     stopped ○    │  └──────────────────────┘     │
│    ▸ api         ○         │                               │
│      default  stopped ○    │                               │
│                            │                               │
├────────────────────────────┴───────────────────────────────┤
│ ↑↓ navigate  s toggle  r restart  Ctrl+P palette  / search │
│ 1 running                                                  │
└────────────────────────────────────────────────────────────┘
```

### Sidebar Design (sub-commands expansion)

```
portfolio                ← project group (bold)
  ▸ frontend    ● dev    ← app (indented), shows which sub-command is active
    dev           ●      ← sub-command (more indented), ●current ○other
    build         ○
    lint          ○
  ▸ api         ○        ← app is stopped (no sub-command running)
    default       ○

blog                     ← another project (collapsed)
```

- **Only one sub-command runs per app at a time.** Selecting a different
  sub-command stops the current one, then starts the newly selected one.
- **App entries are expandable** — Enter on an app toggles expansion to show
  sub-commands. The app line shows which sub-command is currently active.
- **Enter on a sub-command** — stops current (if any) and starts this one.
- **Enter on an app** — toggles the `default` sub-command.
- **Project-level toggle (`s`)** — toggles all apps in the project (each
  app's `default` sub-command).
- **Status dots**: `●` (green) = running, `◐` (yellow) = starting/stopping,
  `○` (gray) = stopped, `✕` (red) = crashed.
- The log viewer shows the PTY output of whichever sub-command is currently
  selected/running.

### Overlay System

Same concept as current TypeScript TUI — absolutely-positioned centered boxes:

| Overlay        | Trigger                      | When active                              |
| -------------- | ---------------------------- | ---------------------------------------- |
| CommandPalette | `Ctrl+P`                     | Action list: Switch Theme, Reload Config |
| SearchDialog   | `/`                          | Fuzzy filter projects/apps               |
| ThemeDialog    | Via palette → "Switch Theme" | 33 themes, live preview, viewport scroll |

Overlays capture all input. Escape dismisses. Theme selection is confirmed on
Enter and reverted on Escape.

---

## State Persistence

```rust
// ~/.frost/state.json

{
  "version": 1,
  "active_theme": "opencode",
  "theme_mode": "dark",
  "last_project": "portfolio",
  "expanded_apps": ["portfolio/frontend", "blog"]
}
```

- Write debounce: 500ms.
- Read at startup, write on changes.
- If file missing or corrupted, start with defaults.

---

## CLI (future)

```bash
# TUI mode (default)
frost

# Headless mode (post-MVP)
frost start portfolio         # start all default sub-commands
frost stop portfolio/frontend # stop an app
frost status                  # JSON status output
frost theme set dracula       # switch theme via CLI
```

The `frost-core` crate enables this — the CLI is just another consumer.

---

## Dependency Map

### frost-core

| Crate                  | Purpose                                                 |
| ---------------------- | ------------------------------------------------------- |
| `serde` + `serde_json` | Theme JSON parsing, state persistence                   |
| `toml`                 | Config loading                                          |
| `portable-pty`         | PTY creation for child process I/O                      |
| `alacritty_terminal`   | ANSI/VT terminal emulator — maintains screen grid       |
| `nix`                  | Process group signals (`kill(-pid)`)                    |
| `tokio`                | Async runtime for process I/O + screen update broadcast |
| `dirs`                 | Cross-platform `$HOME` / `$XDG_CONFIG_HOME`             |
| `thiserror`            | Error types                                             |
| `tracing`              | Structured logging                                      |

### frost-tui

| Crate        | Purpose                   |
| ------------ | ------------------------- |
| `ratatui`    | TUI framework             |
| `crossterm`  | Terminal backend          |
| `arboard`    | Clipboard access (future) |
| `frost-core` | All business logic        |

---

## Implementation Phases

### Phase 1 — Foundation (core only, no TUI)

- [ ] `frost-core/src/config/` — types + TOML loader + flattening
- [ ] `frost-core/src/theme/types.rs` — ThemeJson, ResolvedTheme, RGBA
- [ ] `frost-core/src/theme/builtin.rs` — embed 33 themes
- [ ] `frost-core/src/theme/resolver.rs` — color resolution pipeline
- [ ] `frost-core/src/theme/system.rs` — terminal palette → theme generation
- [ ] `frost-core/src/theme/registry.rs` + `store.rs` — theme management
- [ ] `frost-core/src/state/` — StateStore with persistence
- [ ] Unit tests for all core modules

### Phase 2 — Process Management

- [ ] `frost-core/src/process/pty.rs` — PTY spawning (`portable-pty` + `setsid`), process group kill
- [ ] `frost-core/src/process/manager.rs` — start/stop/restart with `alacritty_terminal::Term` per process
- [ ] Screen update broadcast channel (`tokio::sync::broadcast`) — TUI re-renders on terminal output
- [ ] PTY resize on terminal window size change
- [ ] Integration test: spawn real process via PTY, feed output to terminal emulator, kill process group

### Phase 3 — TUI Shell

- [ ] `frost-tui/src/main.rs` — event loop, terminal setup/teardown
- [ ] `frost-tui/src/actions.rs` — Action enum
- [ ] `frost-tui/src/input.rs` — crossterm key → Action mapping
- [ ] `frost-tui/src/state.rs` — TUI state machine
- [ ] `frost-tui/src/app.rs` — top-level App struct
- [ ] Layout skeleton (sidebar + log + command bar, no widgets)

### Phase 4 — Widgets

- [ ] `sidebar.rs` — project/app/sub-command tree with expand/collapse + status icons; only one sub-command runs per app at a time; selecting a different sub-command stops current + starts new
- [ ] `log_viewer.rs` — terminal emulator widget: reads `Term` grid, renders as ratatui `Line`/`Span` with styles, auto-scroll to bottom, scrollback history navigation, PTY resize on layout change
- [ ] `command_bar.rs` — shortcuts + running count
- [ ] `palette.rs` — CommandPalette with filter + action dispatch
- [ ] `search.rs` — SearchDialog with fuzzy filter
- [ ] `theme_dialog.rs` — Theme switcher with viewport scroll + live preview

### Phase 5 — Polish

- [ ] Theme → ratatui Color conversion for all widgets
- [ ] Theme background sync (set terminal background color)
- [ ] Clean shutdown (stop all processes, save state, restore terminal)
- [ ] Integration testing with real `frost.toml` configs
- [ ] Archive `ts/` directory (remove from default branch)

---

## Keys Preserved from TypeScript Implementation

| Concept                 | TS Approach                                    | Rust Approach                                       |
| ----------------------- | ---------------------------------------------- | --------------------------------------------------- |
| `setsid sh -c` spawning | `Bun.spawn(["setsid", "sh", "-c", cmd])`       | `Command::new("setsid").args(["sh", "-c", cmd])`    |
| Process group kill      | `process.kill(-pid, SIGTERM)`                  | `kill(Pid::from_raw(-pid), SIGTERM)`                |
| 5-second stop timeout   | `setTimeout` loop 100×50ms                     | `tokio::time::timeout(Duration::from_secs(5), ...)` |
| Race condition guard    | `app.pid !== exitPid`                          | `generation_id` check                               |
| Theme viewport scroll   | Manual `visibleIds = slice(scrollOffset, +10)` | Same — slice the vec                                |
| Theme live preview      | `store.set(id)` on cursor move                 | Same — `store.set(id)` on cursor move               |
| Theme revert on cancel  | `store.set(initialTheme)`                      | Same                                                |

## Keys Changed / Improved

| Item                  | TS Implementation                                    | Rust Improvement                                                                      |
| --------------------- | ---------------------------------------------------- | ------------------------------------------------------------------------------------- |
| **Process I/O**       | Piped stdout/stderr — child sees `isatty() == false` | PTY — child sees real TTY, colors + progress bars + cursor work natively              |
| **ANSI handling**     | Custom 252-line SGR parser (`ansi.ts`)               | `alacritty_terminal` — full VT/ANSI state machine, true-color, cursor, scroll regions |
| **Log storage**       | `Vec<LogLine>` ring buffer (text lines)              | `Term<T>` screen grid (cells with char + style) + scrollback                          |
| **FORCE_COLOR hacks** | `FORCE_COLOR=3`, `COLORTERM=truecolor` env vars      | **Removed** — PTY makes isatty() true, tools enable color natively                    |
| **Progress bars**     | Each `\r` frame rendered as separate line            | Same-line overwrite — `\r` redraws the current row's cells                            |
| **Config format**     | JSONC (`jsonc-parser`)                               | TOML (`toml` crate) — idiomatic, first-class support                                  |
| **App commands**      | Single `command` string per app                      | Named `[commands]` map with `default`                                                 |
| **Event system**      | Custom EventEmitter (Map of Sets)                    | `tokio::sync::broadcast` — well-tested, Send+Sync                                     |
| **TUI paradigm**      | React declarative (OpenTUI JSX)                      | ratatui immediate mode — simplicity, no virtual DOM                                   |
| **State management**  | React hooks + useState/useRef                        | Central `AppState` struct, pure function `update(state, action) → state`              |
| **Sidebar**           | Flat list, no sub-commands                           | Recursive expand/collapse tree with sub-commands                                      |
