# Changelog

English | [简体中文](CHANGELOG.md)

All notable changes to Noteva will be documented in this file.

## [v0.3.0] - 2026-04-28

### Backend Quality Review
- **Backend API follow-up fixes** - Tightened pagination, input validation, error responses, resource boundaries, and state refresh behavior across admin and public APIs to reduce post-refactor edge cases.
- **Plugin, theme, and update flows refined** - Continued stabilizing GitHub install/update logic, store state, online-update confirmation, and local cache refresh so version and package status match the actual installed state more closely.
- **Data access and runtime boundaries strengthened** - Improved boundary handling and failure feedback around articles, categories, tags, comments, files, settings, plugin data, and the WASM worker to reduce hidden failures and stale state.
- **Login cookie and 2FA security boundaries hardened** - Registration, login, and 2FA verification now share the same HTTPS-site and trusted-proxy logic for `Secure` cookies instead of trusting spoofable forwarded protocol headers directly. 2FA challenges now limit failed verification attempts and prevent already-enabled accounts from overwriting their active secret.
- **Article, filter, and comment validation tightened** - Article slugs are normalized and reject path-like dangerous values on create/update; missing category or tag slugs now return an empty paginated result instead of falling back to all articles; comment content is validated before and after plugin hooks to reject empty or oversized content.
- **Plugin operation and ZIP import path safety completed** - Plugin uninstall and update paths now validate plugin directory names while preserving Chinese/Unicode ID compatibility. Backup restore and Markdown ZIP import now enforce entry-count, per-file, total-unpacked-size, symlink, and path-traversal limits.
- **Windows online-update replacement fixed** - Windows updates no longer try to replace the running executable directly. The updater now writes a `.new.exe` and helper script, then swaps binaries and restarts after the old process exits, reducing failed updates and stale executable leftovers.

### Admin Dashboard
- **Management pages reviewed and fixed** - Addressed interaction details, loading states, batch-operation feedback, and error display issues found across articles, comments, files, navigation, settings, security logs, plugins, and themes.
- **Article-list public links fixed** - Preview and copy-link actions on `/manage/articles` are now available only for published articles and generate real public URLs from `site_url` and `permalink_structure`, avoiding broken draft/archive links and incorrect URLs in ID permalink mode.
- **Article-list batch actions completed** - Selecting articles now reveals a compact batch toolbar for publishing, moving to draft, archiving, deleting, and clearing selection. Batch delete includes confirmation and partial-failure feedback so multi-select has a concrete purpose.
- **Category/tag combined layout aligned** - Unified information density, panel structure, list height, and action areas on `/manage/taxonomy` so the merged category and tag page feels visually balanced.
- **Refresh button feedback unified** - Security logs and custom pages now match the plugin-page refresh pattern: the button icon spins while requesting and briefly switches to a green done state on success. Manual refresh no longer triggers top sync badges, progress bars, or table-opacity flashes.
- **Comment-list refresh flicker reduced** - Manual refresh on the comments page now keeps the current list visible and updates data quietly instead of mounting sync bars, table opacity changes, and extra loading states during fast refreshes.
- **Custom page management polished** - The custom pages list now uses the shared admin page header, card container, first-load state, follow-up sync state, and optimistic delete feedback for steadier create, edit, and delete flows.
- **Security log filtering and refresh flow tidied** - Security log filters are applied explicitly, refresh requests reuse the current pagination and filter parameters, and login timestamps are formatted with the active admin locale.
- **Article editor emoji picker improved** - Fixed emoji picker positioning caused by measuring a hidden element; desktop placement now anchors to the real button and clamps to the viewport, while mobile uses a bottom sheet and avoids forcing the keyboard open.
- **Markdown editor stability polished** - Refined preview requests, media-library search, upload entry points, and edit/preview scrolling to reduce flicker and stale-response overwrites during fast input, switching, and uploads.
- **Image-grid editor command added** - The Markdown editor toolbar now includes an image-grid command that wraps selected content in `[grid]...[/grid]` or inserts an empty grid block when nothing is selected, leaving multi-image layout as an explicit author choice.
- **Article thumbnail clearing fixed** - Clicking the thumbnail `X` while editing an article now persists correctly. The update API distinguishes omitted thumbnail fields, `null` clearing, and string updates, so old thumbnails no longer reappear after refresh.

### Default Theme
- **Default theme experience reviewed and fixed** - Continued tightening state synchronization, SDK readiness, and edge-case rendering across home, archive, category, tag, post, comments, header, footer, and theme interactions.
- **Default theme built-in locales expanded** - The default theme now includes Japanese, Korean, French, German, Spanish, Brazilian Portuguese, Russian, and Italian locale packs, with a unified locale list, browser-language matching, and `html lang` synchronization.
- **Category, tag, and archive consistency refined** - Unified data loading, empty states, pagination, and card density for content index pages so larger sites behave more consistently.
- **`[grid]` image-grid rendering supported** - Backend Markdown rendering now supports standalone `[grid]...[/grid]` image blocks, with responsive styling in the default theme and admin preview. Multiple images render as two columns on mobile, three on desktop, and four-image grids stay in a balanced 2x2 layout.

### Frontend Quality Review
- **Built-in locale coverage expanded** - The admin dashboard now includes Japanese, Korean, French, German, Spanish, Brazilian Portuguese, Russian, and Italian built-in locale packs, reducing the need for users to upload common custom locale packs manually.
- **Locale detection and fallback unified** - The admin dashboard and default theme prefer the browser language, support exact and language-prefix matching, fall back to English when no match exists, persist the selected locale, and keep `html lang` in sync.
- **Locale loading performance improved** - The admin dashboard keeps Chinese, Traditional Chinese, and English in the initial message bundle while lazy-loading the additional locale packs on demand to avoid inflating the first admin payload.
- **Custom locale flow fixed** - The admin language switcher now reads runtime locales so custom locale packs appear after loading. The active locale is synced to `html lang`, and plugin/theme settings render localized schema labels from the current app locale.
- **Article editor and list request stability improved** - New-article draft recovery now reaches the lazy Markdown editor reliably, initial data loading is guarded on unmount, and default-theme article lists use request ordering so older responses cannot overwrite newer search or pagination results.
- **Upload and clipboard feedback fixed** - FormData requests now let the browser generate the multipart boundary instead of passing `Content-Type: undefined`, and file-link copying now reports real success or failure.
- **Frontend HTML rendering fallback sanitized** - Articles, custom pages, footers, admin Markdown preview, and search highlights now pass through a lightweight frontend sanitizer that strips script-like tags, event attributes, and unsafe URLs as an extra defense layer.
- **Frontend i18n polish completed** - Language switcher, table of contents, image upload, empty state, and loading text now use localized strings, reducing hardcoded UI copy across the admin dashboard and default theme.

### Build & Version
- **Version unified to 0.3.0** - Updated the Rust crate, Cargo.lock, frontend packages, default theme, SDK built-in version, and development metadata.

---

## [v0.2.9] - 2026-04-27

### Plugin and Theme Store
- **Store responsibility narrowed to listings** - Plugin and theme store data now focuses on basic listing metadata such as names, descriptions, and repository URLs, while install and update flows are handled by Noteva's own GitHub-based logic.
- **Plugin/theme online install entry redesigned** - Admin "Upload Plugin" and "Upload Theme" flows were reshaped into "Install Plugin" and "Install Theme" dialogs that support both ZIP upload and GitHub repository input.
- **Installed-item update checks improved** - Plugin and theme updates no longer depend on a store update endpoint; installed manifests provide repository information for GitHub version checks and show update actions when available.
- **Chinese/Unicode plugin IDs supported** - Frontend URL handling and backend validation now preserve non-ASCII plugin IDs during install, update, and route-parameter based operations.

### Admin Dashboard
- **Category and tag management merged** - The sidebar now exposes one combined category/tag entry, with a new `/manage/taxonomy` page managing both categories and tags. Legacy `/manage/categories` and `/manage/tags` routes redirect to the new page.
- **Tag management simplified** - Removed the duplicate admin tag-cloud view and kept the management-focused tag list with search, article counts, single delete, and batch delete.
- **Plugin/theme refresh experience refined** - Plugin and theme pages keep existing content during refresh and remove the visually noisy full progress-bar flash.
- **Online update status refresh fixed** - After an application update, stale version-check cache is cleared and a fresh check is forced, preventing the sidebar from showing the old version and another available update after reload.
- **Install and update interactions tidied** - Plugin and theme install, update-check, and refresh actions now use more consistent button states and dialog flows.
- **Article list search moved to backend search** - `/manage/articles` now passes search text as the backend `keyword` query instead of filtering only the current page. Search and filter changes reset the list to page one.
- **Category hierarchy management completed** - Category create/edit dialogs now support selecting a parent category, show parent information in the list, and prevent selecting the current category or one of its descendants as parent.
- **Invalid article edit ID handling fixed** - Visiting an invalid article edit URL no longer leaves the page in an infinite loading state; the admin UI now shows a load failure and returns to the article list.
- **Plugin/theme store state refresh fixed** - Uploading, repository-installing, updating, or deleting plugins/themes now refreshes cached store data so installed state and update actions do not lag behind.
- **Markdown editor media-library search stabilized** - Media-library search in the Markdown editor now uses debounce and request ordering so older responses cannot overwrite newer search results during fast typing.
- **Profile save and file batch-delete feedback fixed** - Saving profile settings now updates the auth store immediately, and file batch delete now distinguishes full success, partial failure, and full failure with useful failure counts and names.

### Default Theme
- **Post category display adjusted** - Removed the category label above the post title in the default theme article page to reduce repeated title-area metadata.
- **Home search moved to full backend search** - Default theme home search now uses the SDK `keyword` query against the article API and supports pagination for search results instead of filtering only the current page.
- **Archive/category/tag article loading completed** - Archive, category detail, and tag detail pages now fetch all matching article pages instead of relying on a fixed `pageSize: 100` limit that could truncate larger sites.
- **Category hierarchy display added** - The default theme category page now groups categories by parent/child relationships and exposes child-category links from parent category cards.
- **Comment error messages and external nav links hardened** - Comment submission now surfaces detailed server errors, and external navigation links are validated before rendering so unsafe URLs are rejected.

### Backend API Security & Stability
- **Public static resource paths hardened** - Theme assets, user theme resources, uploads, backup restore paths, and custom locale files now use safer relative paths, directory-boundary checks, and locale-code whitelisting to reduce traversal risk.
- **2FA login flow fixed** - Accounts with two-factor authentication now receive only a short-lived challenge after password verification; the real session and cookies are created only after the 2FA code succeeds.
- **Public article APIs constrained to published content** - Public article list, search, category, and tag endpoints now consistently return only published articles. Admin article lists use protected admin endpoints to avoid draft/archive leakage.
- **Public write endpoints tightened** - Public `/api/v1/cache/{key}` now keeps only read access, with writes/deletes moved behind authentication; Markdown render preview is also restricted to authenticated use.
- **Upload and download content safety strengthened** - Active content uploads such as HTML, JS, SVG, and XML are rejected, and existing active uploads are forced to download with `nosniff` to reduce same-origin script execution risk.
- **GitHub install and update flows hardened** - Plugin and theme GitHub installs are limited to GitHub archive/release sources, add download-size limits, and verify updated packages still match the target plugin ID or theme short name.
- **Application update integrity checks strengthened** - Auto-update downloads now enforce package and binary size limits and require a valid `.sha256` checksum file instead of continuing without verification.
- **Public settings redaction fixed** - Public plugin/theme settings now expose only fields declared in the settings schema and not marked secret. Missing schemas no longer leak all stored settings.
- **Request origin detection tightened** - Login logs, comment fingerprints, and like fingerprints now use the real connection source IP instead of trusting spoofable `X-Forwarded-For` / `X-Real-IP` headers.
- **Theme SDK mutation CSRF support added** - `noteva-sdk.js` now sends CSRF tokens for JSON requests, uploads, and plugin public API mutation requests, fixing logged-in theme writes being rejected by CSRF middleware.
- **Markdown HTML output sanitized** - Markdown rendering now strips dangerous tags, event attributes, unsafe URLs, and high-risk style values across articles, pages, and admin preview output to reduce XSS risk.

### Backend API Behavior Fixes
- **Admin pagination parameters bounded** - Admin comments and login-log endpoints now bound `page` and `per_page`, preventing negative, zero, or oversized pagination values from causing abnormal queries.
- **CORS and CSRF boundaries fixed** - CORS no longer falls back to `*` when credentials are enabled, and logout once again requires CSRF validation to reduce cross-site write risks.
- **Category hierarchy semantics completed** - Category create/update supports `parent_id`, category errors map to more precise duplicate/not-found/validation responses, and category article queries include child categories.
- **Article write validation fixed** - Invalid article statuses now return validation errors on create/update, and missing category IDs resolve through the default category instead of relying on a hard-coded ID.
- **Reload and file-delete responses fixed** - Theme reload now returns `theme_count`, plugin reload returns `plugin_count`, and file deletion rejects path-like filenames instead of silently taking the basename.
- **Version confirmation after online update improved** - After an online update, the admin UI waits for the new process to report the target version before refreshing, avoiding stale old-process version display and repeated update prompts.
- **Admin article create route added** - New article creation now uses `/api/v1/admin/articles`, and the backend exposes the matching admin POST route instead of relying on the legacy protected `/articles` write endpoint.
- **Navigation target validation added** - Navigation targets are validated by type on the server; external links are limited to `http://`, `https://`, `mailto:`, and `tel:`.

### Build & Version
- **Version unified to 0.2.9** - Updated the Rust crate, frontend packages, default theme, SDK built-in version, and development metadata.

---

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
