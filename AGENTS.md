# AGENTS.md — Frost

## Project Identity

- **Frost** (package name `frost`) — a terminal UI for managing local dev services across multiple projects.
- "App Manager" is the tagline only; all code, files, and commands use `frost`.
- Core differentiator: **project-grouped context switching** (start/stop entire stacks with one command).

## Tech Stack (Non-Negotiable)

- **Runtime**: Bun (exclusive). Do not add Node-specific tooling.
- **TUI Framework**: OpenTUI React bindings (`@opentui/react`). Do NOT use Ink, Blessed, or other React TUI libraries.
- **Process Management**: Native `Bun.spawn()`. No pm2, docker-compose, etc. Docker support is post-MVP.
- **Language**: TypeScript + TSX. JSX transform is `react-jsx` via `@opentui/react` (`tsconfig.json`). No `import React` needed.
- **Config**: `frost.config.ts` — executable TypeScript.
- **State**: `~/.frost/state.json`.

## Developer Commands

| Command                     | What it does                                                         |
| --------------------------- | -------------------------------------------------------------------- |
| `bun install`               | Install dependencies (uses `bun.lock`)                               |
| `bun dev`                   | Run TUI in watch mode (`bun run --watch src/index.tsx`)              |
| `bun test`                  | Run all tests with `bun:test`                                        |
| `bun test <path>`           | Run a single test file (e.g. `bun test tests/config/loader.test.ts`) |
| `bun typecheck`             | `tsc --noEmit`                                                       |
| `bun lint` / `bun lint:fix` | oxlint                                                               |
| `bun fmt` / `bun fmt:check` | oxfmt                                                                |

## TypeScript & Style

- `verbatimModuleSyntax: true` — use **`import type`** for type-only imports.
- `noUnusedLocals: false`, `noUnusedParameters: false` — unused vars are not compile errors.
- Linter: **oxlint** (`oxlint.config.ts`). Formatter: **oxfmt** (`oxfmt.config.ts`).
- oxfmt style: 2 spaces, double quotes, trailing commas, semis, printWidth 100.
- `tests/fixtures` is in the oxfmt ignore list.

## Source Layout

- `src/index.tsx` — TUI entrypoint. Not exported as a library.
- `src/config/` — `defineConfig()`, types, and loader (`findConfig` / `loadConfig`).
- `src/process/` — `ProcessManager`, `spawnApp`, log ring buffer (max 1000 lines).
- `src/state/` — `StateStore` (JSON persistence, 500ms debounce).
- `src/tui/` — **Empty right now**. Dashboard React components and hooks go here (see `Plan.md`).
- `tests/` — mirrors `src/`. `tests/fixtures/frost.config.ts` is used by config loader tests.

## Config Loader Quirks

- Searches for **`frost.config.ts`** by walking up from cwd.
- At runtime, the loader **strips all `import` lines** from the config file before evaluating it in a temporary `.mjs` with an injected `defineConfig`. Config files cannot rely on runtime imports; `import { defineConfig } from "frost"` is only for TypeScript autocompletion.

## Process Manager Quirks

- `spawnApp` splits the command string on spaces (`command.split(" ")`) — **no shell interpolation**. Use `sh -c "..."` if you need pipes, `&&`, or variable expansion.
- `ProcessManager` emits events: `log`, `stateChange`, `exit`.
- Exit code `143` (SIGTERM) is treated as `stopped`, not `crashed`.

## State Persistence Quirks

- `StateStore` uses `process.env.HOME` (fallback `/root`) to resolve `~/.frost/state.json`.
- Tests override `HOME` to a temp directory (`/tmp/frost-state-test`) to avoid polluting the real state file.

## Scope Guardrails

- **MVP** (current focus): config loading, process spawning, state persistence, basic TUI layout.
- **Do not build yet**: headless CLI commands, health checks, auto-restart, config hot-reload, search/filter, themes, Docker support.
- No daemon mode — the TUI _is_ the long-running process.

## References

- `Plan.md` — implementation roadmap (Phases 0–5). Not all phases are implemented yet.
