# Changelog

English | [简体中文](CHANGELOG.md)

All notable changes to Noteva will be documented in this file.

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
