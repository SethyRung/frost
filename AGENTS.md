# AGENTS.md — App Manager CLI/TUI

## Project Identity

A **project orchestrator TUI** for managing local dev services across multiple projects. Not a generic process manager — the core differentiator is **project-grouped context switching** (start/stop entire stacks with one command, switch projects without leaving the TUI).

## Tech Stack (Non-Negotiable)

- **Runtime**: Bun (native TS support, fast startup). Do not add Node-specific tooling unless absolutely required.
- **TUI Framework**: OpenTUI (Zig core + TS bindings). Do NOT use Ink, Blessed, or React-based TUI libraries. OpenTUI is production-proven (powers OpenCode) and chosen for native performance with high-frequency log streaming.
- **Process Management**: Native child process spawning via Bun APIs. Docker support is explicitly post-MVP.
- **Config**: `.appmanager.config.ts` — executable TypeScript config, not JSON/YAML/static files.
- **State**: JSON file at `~/.appmanager/state.json`.

## Architecture Notes

- **Groups apps by project**. A project is a named collection of apps/services.
- **App Registry**: Catalog of runnable app types (Vite, Next.js, NestJS, PostgreSQL, Docker, custom scripts). Apps are declared in config, not hardcoded.
- **Dashboard Layout**:
  - Left pane: Project/app tree with status indicators (✓/✗/⏳).
  - Right pane: Live log streaming (color-coded by app).
  - Bottom: Keyboard shortcuts / control bar.
- **Real-time updates**: Process monitoring, crash detection, log streaming. Performance matters — avoid unnecessary re-renders or allocations in the hot path.

## Commands & Execution

- Entrypoint will be `src/index.ts` (or `bin/appmanager` wrapper). Confirm before changing.
- MVP commands: `start`, `stop`, `status`, `restart` (targeting apps or whole projects).
- No daemon mode for MVP — the TUI *is* the long-running process.

## Scope Guardrails

- **MVP** (~2–3 weeks): Basic layout, process spawning, log streaming, keyboard controls, project/app definitions.
- **v2** (do not build yet): Health checks, auto-restart, config hot-reload, search/filter, themes, Docker support.
- Do not introduce pm2, docker-compose, or Kubernetes concepts into the core. This is a local developer tool.

## State & Config

- Config file name: `.appmanager.config.ts` (looks for this in cwd or walks up).
- State file: `~/.appmanager/state.json` — persists app statuses, last project, etc.
- Both are agent-managed; document schema changes if you evolve them.

## Repo Status

This is a **seed repo** — `AGENTS.md` will need updating once build scripts, tests, and package layout land.
