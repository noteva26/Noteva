# Noteva

<p align="center">
  <a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/noteva26/Noteva/releases"><img alt="Version" src="https://img.shields.io/badge/version-0.3.0-111827"></a>
  <a href="LICENSE"><img alt="License" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.75%2B-b7410e">
  <img alt="SQLite" src="https://img.shields.io/badge/SQLite-default-0f766e">
</p>

Noteva is a lightweight, modern blog system built with Rust. It ships as a single binary, uses SQLite by default, and keeps the editing, theme, and plugin experience simple enough for a personal site while still being extensible.

<p align="center">
  <img src="docs/images/noteva-hero.png" alt="Noteva overview" width="900">
</p>

## Why Noteva

- Lightweight deployment: one binary, local SQLite by default, optional MySQL and Redis.
- Clean admin dashboard: articles, pages, taxonomy, comments, files, plugins, themes, security logs, backups, and settings.
- Markdown-first writing: preview, syntax highlighting, media upload, image grid, and shortcode support.
- Sandboxed plugins: WASM backend hooks, frontend JS/CSS assets, permissions, settings, storage, and i18n files.
- Framework-agnostic themes: build with React, Vue, vanilla JavaScript, or any frontend stack through the injected `window.Noteva` SDK.
- Internationalization built in: common admin and default-theme languages are packaged directly.
- SEO basics included: permalink settings, sitemap, RSS feed, robots.txt, and site metadata.

## Screenshots

Screenshots use demo content and are intended to show the overall interface and workflow.

| Frontend Home | Reading Page |
| --- | --- |
| ![Frontend home](docs/images/frontend-home.png) | ![Post reading](docs/images/post-reading.png) |

| Article Management | Markdown Editor |
| --- | --- |
| ![Admin articles](docs/images/admin-articles.png) | ![Admin editor](docs/images/admin-editor.png) |

| Plugin Management | Theme Management |
| --- | --- |
| ![Admin plugins](docs/images/admin-plugins.png) | ![Admin themes](docs/images/admin-themes.png) |

<p align="center">
  <img src="docs/images/mobile-post.png" alt="Mobile reading page" width="360">
</p>

## Quick Start

For Linux or macOS, the install script detects the platform, downloads the latest release asset, creates the working directories, and can register a system service.

```bash
curl -fsSL https://raw.githubusercontent.com/noteva26/Noteva/main/install.sh | bash
```

After the first start, open:

```text
http://localhost:8080/manage/setup
```

Create the administrator account there, then continue in the admin dashboard at `/manage`.

## Docker

```bash
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

Docker Compose:

```yaml
services:
  noteva:
    image: ghcr.io/noteva26/noteva:latest
    ports:
      - "8080:8080"
    volumes:
      - ./data:/app/data
      - ./uploads:/app/uploads
    restart: unless-stopped
```

## Build From Source

Requirements:

- Rust 1.75+
- Node.js 20+
- pnpm

```bash
git clone https://github.com/noteva26/Noteva.git
cd Noteva
pnpm run install:all
pnpm run build:frontend
cargo run --bin noteva
```

Development commands:

```bash
pnpm run dev:web      # admin dashboard
pnpm run dev:theme    # default theme
cargo run --bin noteva
```

Release build:

```bash
pnpm run build:frontend
cargo build --release
```

## Configuration

Noteva reads `config.yml` from the working directory. A minimal configuration looks like this:

```yaml
server:
  host: "0.0.0.0"
  port: 8080
  cors_origin: "*"

database:
  driver: "sqlite"
  url: "data/noteva.db"
  # driver: "mysql"
  # url: "mysql://username:password@localhost:3306/noteva"

cache:
  driver: "memory"
  # driver: "redis"
  # redis_url: "redis://127.0.0.1:6379"

upload:
  path: "uploads"
  max_file_size: 10485760
  max_plugin_file_size: 52428800

theme:
  path: "themes"
  active: "default"
```

See [config.example.yml](config.example.yml) for the full example.

## Plugins

Plugins live in `plugins/<plugin-id>/` and are described by `plugin.json`. A plugin may include browser assets, a WASM backend module, settings schema, editor buttons, and locale files.

```text
plugins/my-plugin/
|-- plugin.json
|-- frontend.js
|-- frontend.css
|-- backend.wasm
|-- settings.json
|-- editor.json
`-- locales/
```

Backend plugins run in a WASM sandbox through `wasmtime`. Permissions, hook declarations, storage, and settings are explicit so a plugin can stay isolated from the core application.

Read the full guide: [Plugin Development](docs/plugin-development.md).

## Themes

Themes live in `themes/<theme-name>/`. A theme only needs a manifest and a built frontend entry, so it can be implemented with the frontend framework of your choice.

```text
themes/my-theme/
|-- theme.json
|-- settings.json
|-- dist/index.html
`-- preview.png
```

The runtime injects the `window.Noteva` SDK automatically. Themes should use that SDK for site data, posts, pages, comments, navigation, settings, and plugin slots.

Read the full guide: [Theme Development](docs/theme-development.md).

## Documentation

| Document | Description |
| --- | --- |
| [Plugin Development](docs/plugin-development.md) | Plugin package structure, hooks, permissions, WASM bridge, settings, and frontend integration. |
| [Theme Development](docs/theme-development.md) | Theme package structure, SDK usage, settings, dark mode, plugin slots, and compatibility rules. |
| [Changelog](CHANGELOG.en.md) | English release notes. |
| [中文更新日志](CHANGELOG.md) | Chinese release notes. |
| [License](LICENSE) | License text and additional terms. |

## Roadmap Direction

Noteva is intentionally focused: a quiet writing workflow, a compact admin surface, safe extension points, and simple deployment. Features that add visible complexity are expected to justify their place in the product.

## Support

If Noteva is useful to you, sponsorship helps keep development moving:

- [Bronze ($1)](https://www.creem.io/payment/prod_NLloGph4FdG0QH5BN2DXr)
- [Silver ($5)](https://www.creem.io/payment/prod_1FqirOkv4JY21wExvWN3PW)
- [Gold ($10)](https://www.creem.io/payment/prod_2wV2YqQHJHsqrpWAipx40s)

## License

Noteva is licensed under [GPL-3.0-or-later](LICENSE) with a plugin and theme exception.

Core modifications remain under the GPL. Plugins and themes that interact with Noteva only through the published SDK/API may use their own license. See [LICENSE](LICENSE) and [COPYING](COPYING) for details.
