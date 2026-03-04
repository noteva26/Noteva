# CLAUDE.md

Guidelines for using Claude Code in the Noteva repository.

## Tech Stack

- Rust 1.75+ / Axum 0.7 / Tokio (backend)
- SQLx 0.7 with SQLite (default) or MySQL
- wasmtime 18 for WASM plugin sandboxing (subprocess isolation)
- Vite 5 + React 18 + TypeScript + Tailwind CSS + shadcn/ui (admin frontend)
- Themes: any framework via `window.Noteva` SDK
- pnpm for frontend, cargo for backend

## Project Structure

```
noteva/
├── src/
│   ├── api/                 # Axum route handlers
│   ├── services/            # Business logic
│   ├── db/repositories/     # Data access
│   ├── db/migrations/       # SQL migrations
│   ├── models/              # Serde data models
│   ├── cache/               # Cache abstraction
│   ├── plugin/              # Plugin system (hooks, WASM bridge, shortcodes)
│   ├── theme/               # Theme system
│   └── bin/wasm_worker.rs   # WASM subprocess executor
├── web/                     # Admin dashboard (Vite + React)
├── themes/                  # Theme directories
├── plugins/                 # Plugin directories
└── config.yml               # Configuration
```

## Development

### Running the Project

```bash
# Backend dev server (port 8080)
cargo run

# Admin frontend dev
cd web && pnpm install && pnpm dev

# Theme dev
cd themes/default && pnpm install && pnpm dev
```

### Testing

```bash
# Run all tests
cargo test

# Run specific test
cargo test test_name

# Run tests in a module
cargo test --lib plugin::tests
```

Important: Never run long-running dev servers in automated contexts. Use `cargo test` for validation.

### Building

```bash
# Release build (single binary, static assets embedded)
cargo build --release

# Frontend build
cd web && pnpm build

# WASM plugin
cargo build --release --target wasm32-wasip1
```

### Type Checking

```bash
# Rust
cargo check

# TypeScript (admin frontend)
cd web && pnpm tsc --noEmit
```

## Code Style

### Rust

- Error handling: `anyhow::Result` + `thiserror` for domain errors
- Always async; blocking calls only inside `spawn_blocking`
- Logging: `tracing` (`tracing::info!`, `tracing::error!`)
- DB schema changes: add SQL files in `src/db/migrations/`
- Cache: always go through `cache/` abstraction
- Layer discipline: `api/` → `services/` → `db/repositories/`

### TypeScript

- Strict mode, functional components + Hooks
- Tailwind CSS + shadcn/ui
- pnpm only

### Git

- Commit: `type: description` (e.g. `feat: add search`, `fix: pagination bug`)
- Branch: `feature/xxx`, `fix/xxx`

## Plugin System

- Plugins in `plugins/<id>/`, directory name = `plugin.json` `id`
- Frontend plugins: IIFE, use global `Noteva` SDK
- WASM plugins: export `allocate`, hooks as `hook_xxx`, no external crates, `extern "C"` host functions
- Hook types: **Filter** (chained, return replaces data) vs **Action** (side-effects only)
- Sandbox: 16MB memory, 100M instructions, 5s/30s/300s timeout
- Mark sensitive settings `"secret": true`

## Theme System

- Themes in `themes/<name>/`, must have `theme.json` + `dist/index.html`
- SDK auto-injected — never import manually
- Always use `Noteva.ready()` before calling SDK
- Dark mode: `.dark` class, not `prefers-color-scheme`
- Reserve `<div data-noteva-slot="...">` for plugin injection

## Common Pitfalls

- WASM plugins compiled with `wasm32-wasip1` need WASI support (built-in since v0.1.3)
- Plugin `getSettings()` returns `{}` if user never saved — check switches with `=== false`, not `!value`
- Theme settings stored as strings in DB — always handle type coercion
- After modifying WASM host functions, rebuild both debug and release `wasm-worker`
- `cargo` incremental builds may skip relinking — delete old binary or `touch` source to force rebuild

## Reference Docs

- Plugin API: `docs/plugin-development.md`
- Theme API: `docs/theme-development.md`
- Hook registry: `hook-registry.json`
- Config example: `config.example.yml`

## Skills

Auto-loaded from `.agents/skills/`:

| Category | Skills |
|----------|--------|
| Backend | `axum`, `rust-engineer` |
| Frontend | `shadcn-ui`, `typescript-advanced-types` |
| Review | `typescript-react-reviewer`, `code-review-excellence` |
| Testing | `webapp-testing` |
| Tooling | `find-skills` |

Search for more: `npx skills find [query]`
