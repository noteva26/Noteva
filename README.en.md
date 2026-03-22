# Noteva

> Lightweight, Minimalist, Modern Blog System

English | [简体中文](README.md)

A high-performance blog system built with Rust, supporting multi-theme and plugin extensions. Single binary deployment, ready out of the box.

![preview](themes/default/preview.png)

## ✨ Features

- 🪶 **Lightweight** — Single binary deployment, memory < 50MB, cold start < 1 second
- 🎨 **Theme System** — Build themes with any frontend framework, hot reload switching
- 🔌 **Plugin System** — Frontend JS/CSS injection, Shortcode, WASM backend hooks
- 📝 **Markdown** — Code highlighting, math formulas, Shortcode extensions
- 💬 **Comment System** — Nested replies, emoji, moderation, Markdown
- 🌍 **Internationalization** — Simplified Chinese / Traditional Chinese / English
- 💾 **Backup & Restore** — One-click backup, Markdown export, WordPress import
- 🌐 **SEO** — Sitemap, RSS Feed, robots.txt auto-generation
- 🔐 **Security** — Login rate limiting, CSRF protection, security logs

## 🌐 Live Demo

| | |
|---|---|
| **Demo Site** | [demo.noteva.org](https://demo.noteva.org/) |
| **Admin Panel** | [demo.noteva.org/manage](https://demo.noteva.org/manage) |
| **Username / Password** | `demo` / `demo123456` |

## 🚀 Deployment

### One-Click Script (Recommended)

For Linux / macOS:

```bash
curl -fsSL https://raw.githubusercontent.com/noteva26/Noteva/main/install.sh | bash
```

Auto-detects architecture, downloads binary, interactive configuration, registers system service. Run again to enter upgrade mode.

### Docker

```bash
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

<details>
<summary>Docker Compose</summary>

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

</details>

### Manual Download

```bash
# Download latest version
wget https://github.com/noteva26/Noteva/releases/latest/download/noteva-linux-x86_64.tar.gz

# Extract and run
tar -xzf noteva-linux-x86_64.tar.gz
chmod +x noteva && ./noteva
```

### Build from Source

```bash
git clone https://github.com/noteva26/noteva.git
cd noteva
cargo run --release
```

> **First visit**: Open `http://localhost:8080/manage/setup` to set up your admin account.

## ⚙️ Configuration

Edit `config.yml` (auto-generated on first run):

```yaml
server:
  host: "0.0.0.0"
  port: 8080

database:
  url: "data/noteva.db"    # SQLite (default)
  # url: "mysql://user:pass@localhost/noteva"  # MySQL

cache:
  enabled: true
  type: "memory"           # memory or redis

upload:
  dir: "uploads"
  max_size: 10485760       # 10MB
```

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [API Reference](docs/api.md) | Complete API endpoint documentation |
| [Plugin Development](docs/plugin-development.md) | Frontend/WASM plugin development guide |
| [Theme Development](docs/theme-development.md) | Theme development guide, SDK API reference |
| [Changelog](CHANGELOG.md) | Version update history |

## 💝 Sponsor

If Noteva helps you, welcome to sponsor the project!

- [🥉 Bronze ($1)](https://www.creem.io/payment/prod_NLloGph4FdG0QH5BN2DXr)
- [🥈 Silver ($5)](https://www.creem.io/payment/prod_1FqirOkv4JY21wExvWN3PW)
- [🥇 Gold ($10)](https://www.creem.io/payment/prod_2wV2YqQHJHsqrpWAipx40s)

## 📄 License

[GPL-3.0](LICENSE) with Plugin/Theme Exception

The core program is licensed under GPL-3.0. Plugins and themes developed through the SDK/API are **not subject to GPL** and may use any license. See [LICENSE](LICENSE) and [COPYING](COPYING) for details.

---

<p align="center">Made with ❤️ by Noteva Team</p>
