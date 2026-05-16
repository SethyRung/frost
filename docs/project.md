# Frost — Terminal Dev Process Manager

Frost is a terminal UI for managing local development services across
multiple projects. Start, stop, and monitor dev servers from one TUI
with project-grouped commands, ANSI-color log viewing, theme switching,
and keyboard-driven navigation.

**Current status**: TypeScript/Bun MVP complete. Rust rewrite planned.

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  frost.json / frost.jsonc   (config — walks up cwd)     │
├─────────────────────────────────────────────────────────┤
│  src/index.tsx               (entrypoint)                │
│  ┌──────────────────────────────────────────────────┐   │
│  │  App.tsx                                          │   │
│  │  ┌──────────────┐  ┌───────────────────────────┐ │   │
│  │  │  Config      │  │  Dashboard                │ │   │
│  │  │  Loader      │  │  ┌───────────────────────┐ │ │   │
│  │  │              │  │  │ Sidebar    LogViewer   │ │ │   │
│  │  │  findConfig  │  │  │ (app list) (ANSI logs) │ │ │   │
│  │  │  loadConfig  │  │  ├───────────────────────┤ │ │   │
│  │  └──────┬───────┘  │  │ CommandBar             │ │ │   │
│  │         │          │  │ (shortcuts + count)    │ │ │   │
│  └─────────┼──────────│──┤ ┌─────────┬──────────┐ │ │   │
│            │          │  │ │ Palette  │ Search   │ │ │   │
│            │          │  │ │ (Ctrl+P) │ (/)      │ │ │   │
│            │          │  │ │ ┌────────┴───────┐  │ │   │
│            │          │  │ │ │ ThemeDialog    │  │ │ │   │
│            │          │  │ │ │ (33 themes)    │  │ │ │   │
│            │          │  │ │ └────────────────┘  │ │ │   │
│            │          │  │ └────────────────────┘ │ │   │
│            │          │  └───────────────────────┘ │   │
│            │          └────────────────────────────┘   │
│            │                                            │
│  ┌─────────┴──────────────────────────────────────┐   │
│  │  ProcessManager  (event-driven)                 │   │
│  │  ┌───────────┐  ┌───────────┐  ┌────────────┐ │   │
│  │  │ spawnApp  │  │ readLines │  │ appendLog  │ │   │
│  │  │ (setsid)  │  │ (stream)  │  │ (ring 1000)│ │   │
│  │  └─────┬─────┘  └─────┬─────┘  └────────────┘ │   │
│  │        │              │                         │   │
│  │  events: log, stateChange, exit                 │   │
│  └────────┴──────┬────────────────────────────────┘   │
│                  │                                     │
│  ┌───────────────┴─────────────────────────────────────┤
│  │  StateStore  (~/.frost/state.json, 500ms debounce)  │
│  └─────────────────────────────────────────────────────┤
│                                                         │
│  ┌─────────────────────────────────────────────────────┤
│  │  Theme System                                       │
│  │  ┌──────────┐ ┌───────────┐ ┌─────────────────────┐│
│  │  │ Registry │ │ Resolver  │ │ FrostThemeStore     ││
│  │  │ (defs)   │ │ (Theme →  │ │ (switch/mode/notify)││
│  │  └────┬─────┘ │ Resolved) │ └─────────┬───────────┘│
│  │       │       └─────┬─────┘           │            │
│  │       └─────────┬───┘                 │            │
│  │  ┌──────────────┴─────────────────────┴──────────┐ │
│  │  │ 33 built-in themes (from themes/*.json)        │ │
│  │  └────────────────────────────────────────────────┘ │
│  └─────────────────────────────────────────────────────┤
└─────────────────────────────────────────────────────────┘
```

### Data Flow

1. **Config loading** — `findConfig()` walks up from cwd looking for
   `frost.json` or `frost.jsonc`, then `loadConfig()` parses JSONC and
   validates the schema.
2. **State restore** — `StateStore.load()` reads `~/.frost/state.json`
   to restore last-project and app statuses.
3. **Process spawning** — `ProcessManager.start()` calls `spawnApp()`
   which runs `setsid sh -c <command>` for process group isolation.
   Stdout/stderr are piped through `readLines()` and appended to a
   1000-line ring buffer.
4. **TUI bridge** — `useProcessManager` hook subscribes to
   ProcessManager events and mirrors state into React. Keyboard input
   is handled by `useKeyboard` (OpenTUI) and `useNavigation`.
5. **Theme resolution** — `resolvedTheme` is computed from the active
   ThemeJson via the resolver (defs expansion, mode selection) and
   threaded through all TUI components as a `ResolvedTheme` object.

---

## Module Breakdown

### `src/config/` — Configuration

| File        | Purpose                                                       |
| ----------- | ------------------------------------------------------------- |
| `types.ts`  | `AppConfig`, `ProjectConfig`, `FrostConfig`                   |
| `loader.ts` | `findConfig()` walks directories, `loadConfig()` parses JSONC |

**Config format** (`frost.json`):

```jsonc
{
  "$schema": "https://frost.sh/schemas/config.json",
  "projects": {
    "portfolio": {
      "root": "../portfolio",
      "apps": {
        "frontend": { "command": "bun dev", "cwd": "./frontend" },
        "api": { "command": "bun serve", "cwd": "./api" },
      },
    },
    "blog": {
      "apps": {
        "dev": { "command": "npm run dev" },
      },
    },
  },
}
```

- `command`: Shell command to run. Run via `sh -c` so pipes, `&&`, and
  variable expansion work.
- `cwd`: Optional working directory relative to `project.root`.
- `root`: Optional base directory for a project, relative to config
  file location.

### `src/process/` — Process Management

| File         | Purpose                                                        |
| ------------ | -------------------------------------------------------------- |
| `types.ts`   | `ProcessStatus`, `LogLine`, `ProcessInfo`                      |
| `spawner.ts` | `spawnApp()`, `readLines()`, `makeLogLine()`, `appendLog()`    |
| `manager.ts` | `ProcessManager` — event emitter, start/stop/restart lifecycle |

**Process states**: `stopped → starting → running → stopping → stopped`
(`crashed` if exit code ≠ 0, 143, or null)

**Key design decisions:**

- **`setsid sh -c`** — Creates a new process group. Enables killing
  the entire process tree with `process.kill(-pid, SIGTERM)`.
- **Stop timeout** — 5 seconds (100 checks × 50ms). If the process
  hasn't exited, it's force-marked as stopped.
- **Race condition guard** — `exitCode.then()` checks `app.pid !==
exitPid` to prevent stale callbacks from a prior spawn overwriting
  the current process's status.
- **Log ring buffer** — Maximum 1000 lines per app. Oldest lines are
  dropped when the limit is exceeded.
- **ANSI color env vars** — `FORCE_COLOR=3`, `COLORTERM=truecolor`,
  `TERM=xterm-256color` are injected into child environments so that
  tools (nuxt, vite, pnpm) emit ANSI codes despite stdout being a
  pipe rather than a TTY.

### `src/state/` — Persistence

| File       | Purpose                                                   |
| ---------- | --------------------------------------------------------- |
| `types.ts` | `FrostState`, `AppState`, `CURRENT_VERSION`, `STATE_FILE` |
| `store.ts` | `StateStore` — load/save/debounce, app state tracking     |

- **Location**: `~/.frost/state.json`
- **Debounce**: 500ms before writing, to avoid thrash on rapid state
  changes.
- **Schema**: `{ version: 1, lastProject: string | null, apps: { [id]: { status, pid? } } }`
- **Test isolation**: Tests override `HOME` to a temp directory.

### `src/tui/` — Terminal UI

#### Component Tree

```
App
└── Dashboard
    ├── Sidebar          (left panel, 30 cols — project/app tree)
    ├── LogViewer        (right panel, flexGrow — ANSI log display)
    ├── CommandBar       (bottom bar — keyboard shortcuts + running count)
    └── [Overlay]        (absolute positioned centered)
        ├── CommandPalette  (Ctrl+P — actions: switch theme, reload config)
        ├── SearchDialog    (/ — fuzzy filter projects/apps)
        └── ThemeDialog     (via palette — 33 themes, live preview, viewport scroll)
```

#### Keyboard Shortcuts

| Key       | Action                                |
| --------- | ------------------------------------- |
| `↑` / `↓` | Navigate sidebar items                |
| `Tab`     | Switch focus (sidebar ↔ log)          |
| `Enter`   | Toggle selected app (start/stop)      |
| `s`       | Toggle selected app or entire project |
| `r`       | Restart selected app                  |
| `Ctrl+P`  | Open command palette                  |
| `/`       | Open search dialog                    |
| `Escape`  | Close overlay / exit                  |

#### Key Components

- **Sidebar** — Flat list of `project/app` entries with status
  indicators. Uses theme `backgroundPanel`, `backgroundElement` for
  selection highlight, `border` for separators.
- **LogViewer** — Wraps `<scrollbox>` with `stickyStart="bottom"` for
  auto-scroll. Each log line is rendered by `AnsiText`.
- **AnsiText** — Custom ANSI SGR parser (`src/tui/lib/ansi.ts`).
  Handles 16-color, 256-color, and true-color escape sequences plus
  attributes (bold, dim, italic, underline, blink, inverse,
  strikethrough). Falls back to theme `defaultFg`/`defaultBg` when a
  segment has no explicit color.
- **CommandPalette** — Searchable action list. Actions return
  `boolean` — `true` prevents auto-close (used by "Switch Theme" to
  open the ThemeDialog without closing the palette).
- **ThemeDialog** — Viewport-scrolled list (10 visible rows, manual
  offset tracking). Cursor moves the viewport. Live-preview: moving
  the cursor instantly applies the theme. Enter confirms, Esc reverts
  to the original theme.

#### Hooks

| Hook                  | Purpose                                      |
| --------------------- | -------------------------------------------- |
| `useProcessManager`   | Bridges ProcessManager events to React state |
| `useNavigation`       | Flat sidebar selection with wrap-around      |
| `useCommandProcessor` | Command input parsing (legacy)               |

### `src/tui/theme/` — Theme System

**Architecture**: Registry → Resolver → Store → Provider

- **Registry** (`registry.ts`) — Holds all theme defs (built-in +
  custom). Manages additions/removals.
- **Resolver** (`resolver.ts`) — Converts a `ThemeJson` into a flat
  `ResolvedTheme` RGBA map. Handles: defs expansion (`defs.bg` →
  hex), mode selection (`{ dark, light }` → single string), `accent`
  resolution (`"primary"` → resolved primary color), hex parsing,
  color aliases. Also generates syntax highlighting colors.
- **Store** (`store.ts`) — `FrostThemeStore` implements `ThemeStore`.
  Manages active theme, mode (dark/light), mode lock, persistence
  callback, resolved cache, subscriber notification.
- **Provider** (`provider.tsx`) — React context providing
  `useThemeStore()` and `useResolvedTheme()` hooks.

**Theme format** (compatible with opencode's ThemeJson):

```jsonc
{
  "$schema": "https://opencode.ai/theme.json",
  "defs": {
    "bg": "#1a1b26",
    "fg": "#c0caf5",
  },
  "theme": {
    "primary": { "dark": "#7aa2f7", "light": "#2f6feb" },
    "background": { "dark": "bg", "light": "#ffffff" },
    "text": { "dark": "fg", "light": "#1f2328" },
    // ... 80+ color slots
  },
}
```

**33 built-in themes** (in `themes/*.json`): aura, ayu, carbonfox,
catppuccin, catppuccin-frappe, catppuccin-macchiato, cobalt2, cursor,
dracula, everforest, flexoki, github, gruvbox, kanagawa, lucent-orng,
material, matrix, mercury, monokai, nightowl, nord, one-dark, opencode,
orng, osaka-jade, palenight, rosepine, solarized, synthwave84,
tokyonight, vercel, vesper, zenburn.

---

## Design Decisions & Rationale

### `setsid sh -c` instead of `command.split(" ")`

Shell features are essential for real dev commands (`cd && npm run dev`,
`FOO=bar vite`). `setsid` creates a new process group, enabling
`process.kill(-pid)` to kill the entire process tree (shell + all
children).

### Custom ANSI parser instead of a dependency

Only 252 lines. Handles SGR codes specifically. Strips non-SGR escapes
(OSC sequences, device control). XTERM 256-color cube computed
programmatically. Avoids adding a dependency for a focused need.

### Viewport-scrolled theme list instead of `overflow="scroll"`

OpenTUI's `overflow="scroll"` on an auto-sized flex child didn't clip
content properly. The fix: manually slice `visibleIds` based on a
`scrollOffset` that tracks cursor position. Only 10 rows are ever
rendered. No layout clipping dependency.

### Process group kill vs `child.kill()`

`child.kill()` only kills the direct child (the `sh` process). The
shell may have spawned child processes (node, vite, etc.) that
become orphaned. `process.kill(-pid, SIGTERM)` sends the signal to
all processes in the group.

### `exitCode.then()` race condition guard

When an app is restarted quickly, the old `exitCode` promise may
resolve after the new spawn has started. Without the `app.pid !==
exitPid` guard, the old exit would overwrite the new "running" status.

---

## Tooling

| Tool       | Purpose                               | Command               |
| ---------- | ------------------------------------- | --------------------- |
| Bun        | Runtime, package manager, test runner | `bun dev`, `bun test` |
| TypeScript | Type checking                         | `bun typecheck`       |
| oxlint     | Linter (no ESLint)                    | `bun lint`            |
| oxfmt      | Formatter (no Prettier)               | `bun fmt`             |

**Style**: 2-space indent, double quotes, trailing commas, semicolons,
print width 100. `verbatimModuleSyntax: true` (always `import type`
for type-only imports).

---

## Future: Rust Rewrite

**Branch**: `rewrite/rust`
**Status**: Planned, not started

The TypeScript codebase (~4,700 lines) will be rewritten in Rust as a
parallel implementation. Key crate choices:

| Purpose            | Crate                        |
| ------------------ | ---------------------------- |
| TUI framework      | ratatui + crossterm          |
| Config (JSONC)     | serde + json5                |
| Process management | nix (setsid, process groups) |
| Async I/O          | tokio                        |
| ANSI parsing       | ansi-parser                  |
| State persistence  | serde_json                   |
| CLI                | clap                         |

The config format (`frost.json`) and state file (`~/.frost/state.json`)
remain unchanged — only the runtime changes. Estimated Rust line count:
~2,000–2,500 (serde eliminates significant boilerplate).
