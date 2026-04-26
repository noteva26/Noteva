# Changelog

English | [简体中文](CHANGELOG.md)

All notable changes to Noteva will be documented in this file.

## [v0.2.8] - 2026-04-27

### Plugin System
- **Plugin database API completed** - Added isolated database access for WASM plugins, allowing plugins with declared permissions to run controlled SQL operations while keeping data scoped by plugin ID.
- **Plugin database safeguards tightened** - Strengthened permission checks, SQL validation, and execution boundaries so plugins cannot access core data or other plugins' data unexpectedly.

### Admin Dashboard
- **Sidebar version entry improved** - Added a version and update-check entry to the admin sidebar, kept release-note visibility, and removed the duplicated system update entry from settings.
- **Dashboard recent articles capped** - The `/manage` dashboard now consistently shows only the latest 5 articles, with matching loading and rendered states.
- **Article editor scrolling improved** - The Markdown editor and preview panes on article create/edit pages now keep a stable height and scroll internally for long content instead of stretching the whole page.
- **Article status filtering fixed** - `/manage/articles` now queries and counts published, draft, and archived articles by status on the backend, so draft and archived filters no longer show the full article list.
- **Scheduled publishing semantics completed** - Articles with a scheduled publish time remain drafts until the background task publishes them, scheduled times can be cleared, and the scheduled marker is removed after publication.

### Bug Fixes
- **Broken article emoji rendering fixed** - Markdown rendering no longer forces native Unicode emoji into external Twemoji images, fixing stars and other emoji appearing as broken images in default-theme article tables.
- **Article thumbnail extraction fixed** - SDK thumbnail extraction now skips Emoji/Twemoji images, preventing the first emoji in article content from being treated as the article thumbnail.

### Documentation
- **Plugin development docs updated** - Documented the plugin database API, permission declarations, execution constraints, and usage guidance for plugin-private data.

### Build & Version
- **Version unified to 0.2.8** - Updated the Rust crate, frontend packages, default theme, SDK built-in version, and development metadata.
- **CI test stability fixed** - Fixed a possible SQLite schema lock in the migration rollback test, reducing flaky CI failures.

---

## [v0.2.7] - 2026-04-26

### Theme System
- **Theme Runtime SDK v1 stabilized** - Removed legacy compatibility aliases and consolidated theme calls around the stable modular `Noteva.*` APIs. The default theme has been adapted to the new SDK.
- **Theme package validation strengthened** - Added `theme.json` schema v1 validation, requiring key fields such as `schema`, `short`, `description`, `repository`, and `requires.noteva`, plus checks for `dist/index.html`, preview paths, and repository URL format.
- **Theme settings validation tightened** - `settings.json` now supports schema v1 validation. Saving settings rejects unknown fields and validates/coerces values by declared type, reducing runtime issues caused by invalid theme settings.

### Bug Fixes
- **Default theme header layout fixed** - Fixed excessive left-side whitespace around the top logo area so the header aligns normally again.
- **Default theme dropdown layout shift fixed** - Fixed horizontal page movement when opening language and theme dropdown menus.
- **Default theme visual regression fixed** - Restored the clean plain background and tuned home/article-card typography back down so list and reading pages no longer feel oversized or overly sparse.

### Documentation
- **Theme development docs expanded** - Merged theme package structure, manifest fields, repository rules, version compatibility, and settings declaration requirements into `docs/theme-development.md`.
- **Theme JSON Schemas added** - Added JSON Schemas for `theme.json` and `settings.json` to support future theme validation and editor assistance.

### Build & Version
- **Version unified to 0.2.7** - Updated the Rust crate, frontend packages, default theme, SDK built-in version, and development metadata.

---

## [v0.2.6] - 2026-04-26

### Highlights
- **Admin dashboard and default theme upgraded to React 19** - Both frontends now use React 19 / React DOM 19, with TypeScript, build, and component compatibility updates.
- **Admin dashboard UX improvements** - Improved loading states and interaction details across articles, categories, tags, pages, navigation, comments, files, plugins, themes, and security logs, reducing skeleton flicker and repeated full-page loading.
- **Default theme reading experience redesigned** - The post page now behaves more like a reading page than a detail view, with the top back button removed and improvements to content width, table of contents placement, line height, paragraph spacing, and information density.

### Admin Dashboard
- **List-page loading states refined** - Multiple management list pages now use first-load skeletons plus lightweight sync indicators for later refreshes, avoiding full-page flicker while filtering or reloading data.
- **Shared admin primitives added** - Added reusable page header, data sync bar, and confirmation dialog components to reduce duplicated UI logic.
- **Confirmation flows improved** - Replaced selected native `confirm` / `alert` usage with consistent in-app confirmation flows for delete and batch operations.
- **Shared helpers extracted** - Added common helpers for API error parsing, formatting, and GitHub-related utilities to reduce page-level duplication.
- **React Compiler experiment gate** - Added a gated React Compiler build path for the admin dashboard via `REACT_COMPILER=1`; it remains disabled by default.

### Default Theme
- **Post reading layout improved** - The article column is wider and more comfortable for reading; posts with a table of contents use a right-side sticky rail, while posts without a TOC fall back to a centered single-column layout.
- **Reading density tuned** - Tightened body line height, paragraph spacing, heading spacing, and content-card padding to reduce unused whitespace and improve long-form reading flow, especially for Chinese content.
- **Article summary cards unified** - Added a shared article summary card used by the home page, category detail page, and tag detail page for consistent title, excerpt, thumbnail, category, tag, and interaction metadata display.
- **Category, tag, and archive pages refined** - Category/tag index and detail pages now work better as content indexes; the archive page now uses a more compact timeline-style article index.
- **SDK readiness logic unified** - Consolidated scattered SDK polling into `waitForNoteva()`, reducing repeated `setTimeout` loops, state updates after unmount, and page flicker risks.
- **Comments and emoji loading refined** - Comments and the emoji picker now use the shared SDK readiness flow while keeping the React 19 `useOptimistic` comment submission experience.
- **Header and footer polished** - The header now has active navigation states, closes the mobile menu on route changes, and reads injected site config more consistently; the default footer copyright no longer depends on HTML injection.

### Build & CI
- **CI build order fixed** - CI now builds `web/dist` and `themes/default/dist` before running Rust `cargo check` / `cargo test`, fixing `rust-embed` failures when dist directories are missing.
- **Frontend chunk splitting improved** - Admin dashboard and default theme Vite builds split React, UI, Motion, and other dependencies into stable vendor chunks, reducing the main entry size and improving browser cache behavior.
- **Version unified to 0.2.6** - Updated `Cargo.toml`, `Cargo.lock`, root `package.json`, `web/package.json`, `themes/default/package.json`, the default theme `theme.json`, and the SDK built-in version.

---

## [v0.2.5] - 2026-04-25

### Security
- **Generic plugin proxy disabled** - `/api/v1/plugins/proxy` no longer forwards arbitrary URLs, preventing frontend plugins from using a shared server proxy for SSRF or secret exposure. Plugins that need external APIs should store user configuration in plugin settings and call those endpoints from WASM backend code with the `network` permission.
- **WASM worker sandbox hardened** - Added memory, instruction, request/response size, log, storage, and database operation limits. HTTP host calls now only allow `http/https`, block local/private/metadata addresses, and disable redirects.
- **Plugin/theme archive extraction hardened** - ZIP/TAR extraction now rejects path traversal, symlinks, special files, unsafe package names, oversized entries, and excessive unpacked size.

### Reliability
- **Core migrations are transactional** - Migration SQL execution and `_migrations` recording now happen in one transaction, with validation for known versions, matching names, and continuous history.
- **Plugin migrations are transactional** - Plugin migration SQL and `plugin_migrations` recording now happen in one transaction; failed migrations no longer leave partial tables or records.
- **Safer plugin install overwrite order** - Plugin installation validates `plugin.json` and the real plugin ID before replacing an existing plugin directory.

### Engineering Quality
- **CI workflow added** - Added Rust `cargo check --all-targets --locked`, `cargo test --locked`, admin frontend build, and default theme build checks.
- **Frontend lockfiles can be committed** - Unignored `web/pnpm-lock.yaml` and `themes/default/pnpm-lock.yaml` so CI `--frozen-lockfile` installs are reproducible.
- **Low-risk frontend cleanup** - Removed unused admin `@dnd-kit/*` dependencies and unused default theme `axios`, plus tightened a few `any` usages without changing routes or core behavior.

### Documentation
- **Plugin development docs updated** - Documented generic proxy deprecation, WASM network restrictions, package safety rules, and transactional plugin migrations.
- **Theme development docs updated** - Documented theme package safety rules.

---

## [v0.2.4] - 2026-03-28

### 🔌 Plugin Integration Standardization
- **Default Theme Data Attributes** — Added `data-article-id`, `data-comment-id`, `data-page-id` attributes and semantic CSS classes (`article-meta`, `article-content`, `comment-meta`, `comment-content`, `comment-actions`, `article-list`, `page-content`) across 7 components, enabling plugins to target DOM elements via standard selectors
- **Custom Page Plugin Slots** — Added `page_content_top` / `page_content_bottom` PluginSlot to `custom-page.tsx`

### 🐛 Bug Fixes
- **Plugin Store ID Mapping** — Fixed blog consumer using raw store `slug` instead of `plugin_id` for `StorePluginInfo.slug`, now prioritizes `plugin_id` with slug fallback

---

## [v0.2.3] - 2026-03-23

### 🔌 Plugin System Enhancements
- **Hook Data Completeness** — 11 hook trigger points (comments, user login, API middleware) now include full context: `ip`, `user_agent`, `email`, `created_at`, etc. Plugins can access complete request info without hacks
- **Plugin Development Docs Updated** — `docs/plugin-development.md` hook reference tables and `hook-registry.json` synced with actual code

### 🔧 Framework Upgrade
- **Default Theme + Admin Panel Tailwind CSS v4** — Upgraded from v3.4 to v4, CSS-first config (`@theme` + `@plugin`), removed `tailwind.config.ts` and `postcss.config`, using `@tailwindcss/vite` plugin

### 📝 UX Improvements
- **Auto Language Detection** — First visit auto-detects browser language and matches against available locale packs; falls back to English if no match found

### 🐛 Bug Fixes
- **Custom Locales Missing on Frontend** — Fixed default theme language switcher only showing built-in locales; now correctly loads admin-added custom language packs

### 🏗️ CI/CD
- **Automated Release Notes** — `release.yml` now extracts changelog content from `CHANGELOG.en.md` instead of using a hardcoded template

---

## [v0.2.2] - 2026-03-22

### 🔧 Framework Upgrade
- **Axum 0.8** — Backend framework upgraded from 0.7.9 to 0.8.8 (route syntax `:param` → `{param}`, tower 0.5, tower-http 0.6)

### 🔒 Security Fixes
- **CSRF Token CSPRNG** — `generate_csrf_token()` changed from `DefaultHasher` (predictable) to `getrandom` (cryptographically secure)
- **SHA256 Update Verification** — Auto-downloads `.sha256` checksum and verifies binary integrity before applying updates
- **License Unification** — `Cargo.toml` license corrected from MIT to GPL-3.0-or-later (matching LICENSE file)
- **Plugin Upload Path Traversal** — `plugin_id` now strictly validated, rejecting `../` and other path traversal attacks
- **Comment Length Limit** — Added 10,000-character cap to prevent malicious oversized comments
- **Password Strength Unified** — Registration now requires ≥8 characters, matching change-password validation
- **Cookie Build Safety** — `auth.rs` `HeaderValue::expect()` replaced with `map_err()`, preventing panics on invalid chars
- **Zip Slip Prevention** — Both theme and plugin ZIP/TAR extraction now validates paths, blocking malicious archives from writing outside target directory
- **Upload Path Traversal Guard** — `/uploads/` static file serving now uses `canonicalize` to block `../` attacks reading config or database files
- **CORS Config Resilience** — Invalid `cors_origin` no longer panics; falls back to `*` with a warning log
- **Theme/Plugin Delete Path Validation** — `delete_theme` and `uninstall_plugin` now validate names, preventing directory traversal

### 🧹 Code Cleanup
- **Frontend ‘use client’ Removal** — Removed invalid Next.js `"use client"` directives from 24 components (unnecessary in Vite + React project)

### ⚡ Performance
- **N+1 Tag Query Eliminated** — Added `get_by_article_ids()` batch method, article listing tag fetches reduced from N+1 to 1 query
- **Database-Level Sorting** — `ArticleSortBy` enum threaded through full stack (API → Service → Repository), removed in-memory sorting
- **Named Sort Support** — Article list now supports `sort_by` parameter for database-level sorting by published_at or created_at

### 🐛 Bug Fixes
- **Env Variable Overrides Broken** — `main.rs` changed from `Config::load()` to `Config::load_with_env()`, `NOTEVA_*` environment variables now work
- **config.example.yml Field Mismatch** — `upload.dir` → `upload.path`, `upload.max_size` → `upload.max_file_size`
- **Docker Missing wasm-worker** — Runtime image now copies `wasm-worker` binary, fixing plugin system in containers
- **Release Missing wasm-worker** — `release.yml` now packages `wasm-worker` binary for all 3 platforms
- **SHA256 Checksum Format Mismatch** — CI now generates per-file `.sha256` files alongside `checksums.txt`, matching `update.rs` download format

### 🏗️ Operations
- **Expired Session Cleanup** — Background task in `main.rs` now cleans up expired sessions every 30 minutes
- **Graceful Shutdown** — Server handles Ctrl+C / SIGTERM gracefully, completing in-flight requests before exit (Docker/K8s compatible)

---

## [v0.2.1] - 2026-03-13

### 🎉 New Features
- **SDK i18n API** — Added `Noteva.i18n.getCustomLocales()` / `getLocales()` / `loadCustomLocales()`, themes no longer need to read `window.__CUSTOM_LOCALES__` directly
- **File-based Custom Locales** — Locale packs migrated from database to `data/locales/*.json` for lighter, file-based management
- **Admin Panel Japanese** — Built-in Japanese translation (`ja.json`), 150+ keys covering the entire admin interface

### 📝 Improvements
- Default + Prose themes `loadCustomLocales()` now use SDK API, removing duplicate code
- `noteva-sdk.d.ts` TypeScript declarations updated with new i18n methods
- Demo mode whitelist expanded: added like, view count, register, 2FA, cache, and plugin proxy endpoints
- `locale.rs` moved from `db/repositories/` to `services/` (file I/O doesn't belong in the database layer)
- Migration 29 changed to `DROP TABLE IF EXISTS custom_locales` (deprecated)
- Removed debug `console.log` from Prose theme
- Docs: Added SDK `Noteva.i18n` API reference table

---

## [v0.2.0] - 2026-03-06

### 🎉 Highlights
- **Stable release** — Removed Beta tag, unified version to 0.2.0

### 🐛 Bug Fixes
- Fixed `setup` page checking admin existence by matching Chinese string instead of `errorCode`
- Fixed `login` page rate-limit retry messages hardcoded in Chinese
- Fixed date formatting not following language switch in `plugins/themes/pages/settings` (`toLocaleDateString` missing locale arg)
- Fixed `plugins` page download count `" downloads"` hardcoded in English
- Fixed `avatar-upload` component with 4 hardcoded Chinese toast messages
- Fixed `loading-state` component hardcoded Chinese loading text
- Fixed `settings-renderer` component hardcoded Chinese "Add Item" button
- Fixed `language-switcher` component hardcoded Chinese title attribute
- Fixed `settings` page "Failed to load settings" hardcoded in English
- Fixed `files` page batch delete missing confirmation dialog

### 🌍 Internationalization
- `files/index.tsx` — Replaced 34 hardcoded Chinese strings with i18n keys
- Added `fileManage` i18n section (34 keys × 3 languages)
- Added `common.addItem`, `common.switchLanguage`, `settings.avatar*` and other i18n keys
- All `toLocaleString` / `toLocaleDateString` calls now pass dynamic locale

### 🔧 Internal
- Backend: Cleared 6 compiler warnings, fixed 3 N+1 queries
- Backend: Fixed `.unwrap()` safety issue in `backup.rs`
- Unified version to 0.2.0 across Cargo.toml and 3 package.json files

---

## [v0.1.9-beta] - 2026-03-05

### 🎉 New Features
- **SDK Field Helpers** — `Noteva.articles.getDate/getStats/isPinned/getThumbnail/getExcerpt/getHtml/incrementView` for field compatibility
- **Interactions Module** — `Noteva.interactions.like()` / `checkLike()` for article and comment likes
- **Search Utils** — `Noteva.search.highlight()` for keyword highlighting in search results
- **Custom Fonts** — 14 Google Fonts + system default, auto-injected via SDK CSS variable `--noteva-font`

### 📝 Improvements
- Default theme fully migrated to SDK calls (~150 lines of duplicate helpers removed)
- `site.getInfo()` now passes through all backend fields via spread operator
- TypeScript type declarations updated for all new SDK methods (`noteva-sdk.d.ts`)

### 🔧 Internal
- Version unified to 0.1.9-beta across Cargo.toml, SDK, theme.json, hook-registry.json
- Zero direct API calls remaining in default theme (all go through SDK)
- Comment interface simplified with `[key: string]: any` fallback

---

## [v0.1.8-beta] - 2026-03-04

### 🎉 New Features
- **38+ Backend Hooks** — Full lifecycle hooks for articles, pages, comments, categories, tags, users, settings, and more
- **Cron System** — `cron_register` / `cron_tick` hooks for scheduled plugin tasks (60s interval)
- **RSS & Sitemap Filters** — `feed_filter` / `sitemap_filter` hooks for plugins to modify SEO output
- **Article Import** — Import from Markdown ZIP (YAML frontmatter) and WordPress WXR XML
- **Recent Comments API** — `GET /api/v1/comments/recent` for site-wide recent comments
- **Word Count & Reading Time** — Displayed on article pages
- **Article Navigation** — Previous/Next article links at the bottom of posts
- **Related Articles** — Recommended related posts below article content
- **Monthly Archives** — Archive page with articles grouped by year and month

### 📝 Improvements
- Comment nesting depth optimized (max 4 levels visual indent)
- Full backup & restore with Markdown export

### 🔧 Internal
- Hook registry updated to v0.1.8-beta (17 new hooks registered)
- SDK: added `comments.recent(limit)` method
- Default theme: Article interface extended with `word_count`, `reading_time`, `prev`, `next`, `related` fields
- i18n: Added translation keys for new article metadata (zh-CN, zh-TW, en)

---

## [v0.1.7-beta]

### Features
- Performance optimizations (route-level lazy loading, API request caching)
- Code quality improvements (unwrap cleanup, error handling unification)
- Route loading indicator (NProgress)
- Database abstraction refactoring (dispatch macro)

---

## [v0.1.6-beta]

### Features
- Theme & Plugin authorization hooks (`theme_activate`, `plugin_activate`)
- File upload filter (presign delegation)
- Plugin upgrade hook for data migration

---

## [v0.1.5]

### Features
- Plugin destroy hook for resource cleanup
- Plugin upgrade hook for version migration

---

## [v0.1.4-beta]

### Features
- Image upload filter hook
- Plugin store & theme store

---

## [v0.1.3-beta]

### Features
- Initial plugin system (WASM, frontend JS/CSS, Shortcode)
- Comment system (nested replies, moderation, emoji)
- User system (registration, login, permissions)
- Cache optimization (memory/Redis, ETag)
- i18n support (zh-CN, zh-TW, en)
- SEO (Sitemap, RSS, robots.txt)
