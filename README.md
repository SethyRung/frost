# Frost (App Manager)

A terminal UI for managing local dev services across multiple projects. Start and stop entire stacks with one command, switch projects without leaving the TUI, and stream logs from all your apps in real time.

## Quickstart

```bash
bun install
bun dev
```

## What it is

- **Project-grouped context switching** — organize apps by project and manage entire stacks together
- **Real-time dashboard** — live status indicators, process monitoring, and color-coded log streaming
- **Built for local development** — not a generic process manager; optimized for the dev services you boot every day

## Tech stack

- [Bun](https://bun.sh) — runtime and package manager
- [TypeScript](https://www.typescriptlang.org)
- [OpenTUI](https://opentui.com) — TUI framework (React bindings via `@opentui/react`)

## Project structure

```
src/
  index.tsx          # Entrypoint — renders the TUI root component
package.json         # Scripts and dependencies
tsconfig.json        # TypeScript config with @opentui/react JSX transform
```

## Commands

| Command       | Description               |
| ------------- | ------------------------- |
| `bun install` | Install dependencies      |
| `bun dev`     | Run the TUI in watch mode |

## Configuration

Create a `frost.json` file in your project (or any parent directory):

```json
{
  "$schema": "./schemas/config.json",
  "projects": {
    "my-web-app": {
      "root": "./apps/web",
      "apps": {
        "frontend": { "command": "bun run dev", "cwd": "./frontend" },
        "api": { "command": "bun run start", "cwd": "./api" }
      }
    }
  }
}
```

## Contributing

See [`AGENTS.md`](./AGENTS.md) for architecture notes, conventions, and developer guidance.

## License

[MIT](./LICENSE) — Copyright (c) 2026-present Sethy Rung.

---

Created with [`bun create tui`](https://git.new/create-tui).
