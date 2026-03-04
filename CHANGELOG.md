# Changelog

All notable changes to Noteva will be documented in this file.

## [v0.1.8-beta] - 2025-03-04

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
- Comprehensive plugin development docs with 38+ hook reference table
- Complete API reference documentation (50+ endpoints)

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

## [v0.1.5-beta]

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
