# Noteva Development Guidelines

This document serves as a comprehensive guide for AI coding assistants working on the Noteva repository.

## Project Description

Noteva is a lightweight, modern blog system built with Rust. It features a WASM-sandboxed plugin system, multi-theme support with any frontend framework, and a React-based admin dashboard. Single-binary deployment, SQLite by default.

- Repository: https://github.com/noteva26/Noteva
- License: MIT
- Version: v0.1.6-beta

## Tech Stack

- Backend: Rust 1.75+, Axum 0.7, Tokio
- Database: SQLx 0.7 (SQLite default / MySQL), auto-migration
- Cache: moka (in-memory) / Redis (optional, feature-gated)
- WASM Runtime: wasmtime 18 (plugin sandbox, subprocess isolation)
- Markdown: pulldown-cmark + syntect (syntax highlighting)
- Serialization: serde, serde_json, serde_yaml, toml
- Auth: argon2 hashing, session-based
- Admin Frontend (`web/`): Vite 5 + React 18 + TypeScript + Tailwind CSS + shadcn/ui
- Themes (`themes/`): Any framework (React, Vue, vanilla JS) via `window.Noteva` SDK
- Package Manager: pnpm (frontend), cargo (backend)

## Directory Structure

```
noteva/
├── src/
│   ├── main.rs              # Entry point
│   ├── api/                 # Axum HTTP handlers (route layer)
│   ├── services/            # Business logic layer
│   ├── db/
│   │   ├── migrations/      # SQL migration files
│   │   └── repositories/    # Data access layer
│   ├── models/              # Data models (serde Serialize/Deserialize)
│   ├── cache/               # Cache abstraction (memory / redis)
│   ├── plugin/              # Plugin core (loader, hooks, WASM bridge, shortcodes)
│   ├── theme/               # Theme loading & switching
│   └── bin/
│       ├── wasm_worker.rs   # WASM plugin subprocess executor
│       └── generate_hook_docs.rs
├── web/                     # Admin dashboard (Vite + React + TS)
├── themes/                  # Theme directories (default, prose, fusion, pixel)
├── plugins/                 # Plugin directories
├── data/                    # SQLite database (auto-created on first run)
├── uploads/                 # User uploads
├── docs/                    # Documentation (plugin & theme dev guides)
└── config.yml               # Main configuration
```

## Development

### Commands

```bash
# Backend
cargo run                    # Dev server (port 8080)
cargo build --release        # Release build (single binary)
cargo test                   # Run tests
cargo watch -x run           # Hot-reload dev

# Admin frontend
cd web && pnpm install && pnpm dev
cd web && pnpm build

# Theme dev
cd themes/default && pnpm install && pnpm dev

# WASM plugin
rustup target add wasm32-wasip1
cargo build --release --target wasm32-wasip1
cp target/wasm32-wasip1/release/my_plugin.wasm plugins/my-plugin/backend.wasm
```

### Architecture

Request flow: `api/` (handlers) → `services/` (business logic) → `db/repositories/` (data access)

- Public API: `/api/v1/*`
- Admin API: `/api/v1/admin/*` (protected by `require_admin` + `require_auth` middleware)
- First visit setup: `http://localhost:8080/manage/setup`

## Code Style

### Rust

- Error handling: `anyhow::Result` for business errors, `thiserror` for domain error types
- Always async; no blocking calls outside `spawn_blocking`
- Logging: `tracing` crate (`tracing::info!`, `tracing::error!`)
- DB changes: add SQL migration files in `src/db/migrations/`
- Cache: use `cache/` module abstraction — never call moka/redis directly
- Models: derive `Serialize` / `Deserialize` via serde
- Release profile: `opt-level = "z"`, LTO, `panic = "abort"`, strip symbols

### TypeScript (web/ & themes/)

- Strict TypeScript
- Functional components + Hooks
- Tailwind CSS utilities; shadcn/ui for admin UI
- pnpm only — no npm/yarn

### Git

- Commit: `type: description` (e.g. `feat: add search`, `fix: comment pagination`)
- Branch: `feature/xxx`, `fix/xxx`

## Plugin Development

Plugins live in `plugins/<plugin-id>/`. Directory name must match `id` in `plugin.json`.

### Structure

```
plugins/my-plugin/
├── plugin.json        # Metadata & hook declarations (required)
├── frontend.js        # Browser script (optional)
├── frontend.css       # Browser styles (optional)
├── backend.wasm       # WASM module, target wasm32-wasip1 (optional)
├── settings.json      # Settings UI definition (optional)
├── editor.json        # Editor toolbar buttons (optional)
└── locales/*.json     # i18n translations (optional)
```

### Key Rules

- Frontend: IIFE wrapper, access SDK via global `Noteva` object
- WASM: must export `allocate`; hooks named `hook_xxx`; no external crate deps; host functions via `extern "C"`
- Sensitive settings: `"secret": true` — auto-filtered from public API
- Storage: auto-isolated by plugin ID
- Sandbox limits: 16MB memory, 100M instructions, 5s/30s/300s timeout
- Hook types: **Filter** (return replaces data, chained) vs **Action** (return ignored, side-effects only)
- Permissions: `network`, `storage`, `database`, `read_articles`, `read_comments`, `write_articles`, `write_comments`, `read_config`, `write_config`, `fs_read`, `fs_write`

## Theme Development

Themes live in `themes/<theme-name>/`.

### Structure

```
themes/my-theme/
├── theme.json         # name, short, version, author (required)
├── settings.json      # Theme settings definition (optional)
├── dist/index.html    # Built entry (required)
└── preview.png        # Preview image (recommended)
```

### Key Rules

- SDK auto-injected — never import manually
- Wrap init in `Noteva.ready(() => { ... })`
- Use `Noteva.*` SDK for all API calls — never hardcode paths
- Dark mode: `.dark` class selector, not `prefers-color-scheme`
- Reserve plugin slots: `<div data-noteva-slot="...">`
- Settings stored as strings — handle type coercion on read

## Important Notes

- Single-binary deployment: static assets embedded via `rust-embed`
- Config: `config.yml` (env vars override); see `config.example.yml`
- Upload limits: 10MB images, 50MB plugin packages
- Store: `https://store.noteva.org`
- Detailed API reference: `docs/plugin-development.md`, `docs/theme-development.md`

## Skills

Installed agent skills in `.agents/skills/` (auto-loaded by compatible AI tools):

| Skill | Purpose |
|-------|---------|
| `axum` | Axum web framework patterns |
| `rust-engineer` | Rust engineering best practices |
| `shadcn-ui` | shadcn/ui component development |
| `typescript-advanced-types` | TypeScript advanced type patterns |
| `typescript-react-reviewer` | TypeScript + React code review |
| `webapp-testing` | Playwright web app testing (Anthropic official) |
| `code-review-excellence` | Structured code review workflow |
| `find-skills` | Discover and install more skills via `npx skills find` |
