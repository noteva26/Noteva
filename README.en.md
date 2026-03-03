# Noteva

> Lightweight, Minimalist, Modern Blog System

English | [简体中文](README.md)

A high-performance blog system built with Rust, supporting multi-theme and plugin extensions. Focus on simplicity, speed, and extensibility.

![preview](themes/default/preview.png)

## ✨ Core Advantages

### 🪶 Lightweight
- **Single Binary** - Compiled into one executable, no complex dependencies
- **Low Resource Usage** - Memory < 50MB, minimal CPU usage
- **SQLite Database** - No additional database service required, ready to use
- **Fast Startup** - Cold start < 1 second

### 🎯 Minimalist Design
- **Clean Interface** - Focus on content, remove redundant features
- **Intuitive Management** - Clear admin panel, easy to get started
- **Zero Configuration** - Auto-initialize on first run, no complex setup
- **Markdown First** - Pure writing experience

### 🚀 Modern
- **Built with Rust** - Memory safe, high performance, concurrency friendly
- **Modern Frontend** - Vite + React 18 + Tailwind CSS
- **Hot Reload** - Plugins and themes support hot reload, no restart needed
- **Responsive Design** - Perfect for desktop and mobile devices

## 🎨 Features

- 🔌 **Plugin System** - Flexible plugin mechanism, supports frontend JS/CSS injection, Shortcode, hooks
- 🎨 **Multi-Theme** - Supports any frontend framework (React, Vue, Vanilla JS)
- 📝 **Markdown** - Full Markdown support with code highlighting and Shortcode
- 💬 **Comment System** - Built-in comments with nested replies, emoji, Markdown
- 🔐 **User System** - Registration, login, permission management, security logs
- 🌍 **Internationalization** - Supports Chinese, English, Traditional Chinese
- 📊 **Cache Optimization** - Smart caching strategy for better performance
- 🔄 **Hot Reload** - Plugins and themes support dynamic loading

## 🚀 Quick Start

### Direct Run (Recommended)

#### Requirements
- Rust 1.70+

```bash
# Clone repository
git clone https://github.com/noteva26/noteva.git
cd noteva

# Build and run
cargo run --release

# Access
# Frontend: http://localhost:8080
# Admin: http://localhost:8080/manage
```

Or download pre-compiled binary:

```bash
# Download latest version
wget https://github.com/noteva26/noteva/releases/latest/download/noteva-linux-x64

# Make executable
chmod +x noteva-linux-x64

# Run
./noteva-linux-x64
```

### Docker Deployment

```bash
# Pull image
docker pull ghcr.io/noteva26/noteva:latest

# Run
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

Or use Docker Compose:

```yaml
version: '3.8'

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

### First Use

On first run, it will automatically:
1. Create SQLite database
2. Run database migrations
3. Create necessary directories

**When accessing admin panel for the first time, you need to set up admin account.**

Visit `http://localhost:8080/manage/setup` to complete initialization.

## 📦 Project Structure

```
noteva/
├── src/                    # Rust backend source
│   ├── api/               # API routes
│   ├── db/                # Database layer
│   ├── models/            # Data models
│   ├── services/          # Business logic
│   ├── cache/             # Cache system
│   ├── plugin/            # Plugin system
│   └── theme/             # Theme system
├── themes/                 # Themes directory
│   ├── default/           # Default theme (Vite + React)
│   └── prose/             # Prose theme (Vite + React)
├── plugins/                # Plugins directory
│   ├── hide-until-reply/  # Reply-to-view plugin
│   ├── music-player/      # Music player plugin
│   ├── video-embed/       # Video embed plugin
│   ├── friendlinks/       # Friend links plugin
│   └── profile/           # Profile page plugin
├── web/                    # Admin panel (Vite + React)
├── data/                   # Data directory
│   └── noteva.db          # SQLite database
├── uploads/                # Upload directory
├── docs/                   # Documentation
└── config.yml              # Configuration file
```

## ⚙️ Configuration

Edit `config.yml`:

```yaml
server:
  host: "0.0.0.0"
  port: 8080

database:
  url: "data/noteva.db"

upload:
  dir: "uploads"
  max_size: 10485760  # 10MB

theme:
  active: "default"

cache:
  enabled: true
  type: "memory"  # memory or redis
```

## 🎨 Theme Development

Themes are placed in `themes/` directory with basic structure:

```
themes/my-theme/
├── theme.json      # Theme configuration
├── dist/           # Build output
│   └── index.html  # Entry file
└── preview.png     # Preview image
```

Noteva automatically injects SDK, providing `window.Noteva` global object:

```javascript
Noteva.ready(async () => {
  const site = await Noteva.site.getInfo();
  const { articles } = await Noteva.articles.list();
  // ...
});
```

See [Theme Development Guide](docs/theme-development.md)

## 🔌 Plugin Development

Plugins are placed in `plugins/` directory with basic structure:

```
plugins/my-plugin/
├── plugin.json     # Plugin configuration
├── frontend.js     # Frontend script
├── frontend.css    # Frontend styles
└── settings.json   # Settings definition
```

### Official Plugins

- **hide-until-reply** - Reply-to-view plugin
- **music-player** - Music player plugin
- **video-embed** - Video embed plugin (YouTube, Bilibili, Twitter/X)
- **friendlinks** - Friend links plugin
- **profile** - Profile page plugin

See [Plugin Development Guide](docs/plugin-development.md)

## 🚢 Deployment

### Direct Run

```bash
# Build release version
cargo build --release

# Run
./target/release/noteva
```

### Systemd Service

```ini
# /etc/systemd/system/noteva.service
[Unit]
Description=Noteva Blog
After=network.target

[Service]
Type=simple
User=www-data
WorkingDirectory=/opt/noteva
ExecStart=/opt/noteva/noteva
Restart=always

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl enable noteva
sudo systemctl start noteva
```

### Docker Deployment

```bash
# Use official image
docker pull ghcr.io/noteva26/noteva:latest

# Run
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

### Reverse Proxy (Nginx)

```nginx
server {
    listen 80;
    server_name your-domain.com;

    location / {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /uploads {
        alias /opt/noteva/uploads;
        expires 30d;
    }
}
```

## 📡 API Endpoints

### Public API

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/site/info` | Site information |
| GET | `/api/v1/articles` | Article list |
| GET | `/api/v1/articles/:slug` | Article details |
| GET | `/api/v1/page/:slug` | Page details |
| GET | `/api/v1/categories` | Category list |
| GET | `/api/v1/tags` | Tag list |
| GET | `/api/v1/comments/:article_id` | Comment list |
| POST | `/api/v1/comments` | Post comment |
| GET | `/api/v1/plugins/enabled` | Enabled plugins |

### Auth API

| Method | Path | Description |
|--------|------|-------------|
| POST | `/api/v1/auth/login` | Login |
| POST | `/api/v1/auth/register` | Register |
| POST | `/api/v1/auth/logout` | Logout |
| GET | `/api/v1/auth/me` | Current user |

### Admin API (Auth Required)

| Method | Path | Description |
|--------|------|-------------|
| GET | `/api/v1/admin/articles` | Manage articles |
| POST | `/api/v1/admin/articles` | Create article |
| PUT | `/api/v1/admin/articles/:id` | Update article |
| DELETE | `/api/v1/admin/articles/:id` | Delete article |
| GET | `/api/v1/admin/plugins` | Plugin list |
| POST | `/api/v1/admin/plugins/:id/toggle` | Enable/disable plugin |
| GET | `/api/v1/admin/themes` | Theme list |
| POST | `/api/v1/admin/themes/switch` | Switch theme |

## 🛠️ Tech Stack

**Backend**
- Rust 1.75+
- Axum (Web framework)
- SQLite / MySQL (Database)
- SQLx (Database driver)
- Tokio (Async runtime)

**Frontend (Themes & Admin Panel)**
- Vite 5
- React 18
- TypeScript
- Tailwind CSS
- shadcn/ui

## 💻 Development

```bash
# Backend development (hot reload)
cargo watch -x run

# Default theme development
cd themes/default
pnpm install
pnpm dev

# Prose theme development
cd themes/prose
pnpm install
pnpm dev

# Admin panel development
cd web
pnpm install
pnpm dev
```

## 🗺️ Roadmap

### v0.1.0 ✅
- [x] Basic blog features
- [x] Theme system
- [x] Plugin system (experimental)
- [x] Comment system
- [x] User system
- [x] Cache optimization
- [x] Internationalization
- [x] Hot reload mechanism

### v0.1.1-beta ✅
- [x] Plugin data storage
- [x] Friend links & profile plugins
- [x] Video embed plugin
- [x] SEO optimization (Rust backend meta tag injection)
- [x] Frontend migrated from Next.js to Vite

### v0.1.4-beta (Current)
- [x] Independent store platform (plugin/theme marketplace)
- [x] S3/COS image upload plugin (WASM)
- [x] Unified online install strategy (Release-first + validated repo fallback)
- [x] Plugin settings secret field masking
- [x] Frontend-backend API alignment
- [x] Performance optimizations (WASM precompile cache, subprocess pooling, code splitting)
- [x] Custom CSS/JS injection
- [x] Footer enhancement

### v0.1.2-beta
- [x] Prose theme (three-column layout, AnZhiYu-inspired visuals)
- [x] Theme list fix & metadata loading optimization
- [x] Plugin compatibility fixes (video embed, friend links, profile)
- [x] TLS dependency switched to rustls (cross-compilation support)
- [ ] Code cleanup & documentation updates
- [ ] Stability improvements

### v0.1.3-beta
- [x] Admin panel plugin integration
- [x] Editor extension API
- [ ] More editor features

### v0.1.4-beta (Planned)
- [ ] Plugin API routes
- [ ] Lifecycle hooks
- [ ] Frontend route registration

### v0.1.5 (Stable)
- [ ] Plugin ecosystem refinement
- [ ] Official plugin library
- [ ] Complete documentation

### Future Versions
- [ ] Full-text search
- [ ] RSS feed
- [ ] Sitemap
- [ ] Email notifications
- [ ] More themes

## 💝 Sponsor

If Noteva helps you, welcome to sponsor the project!

### Sponsor Tiers

| Tier | Amount | Benefits |
|------|--------|----------|
| 🥉 Bronze | $1 | Supporter badge + Our thanks |
| 🥈 Silver | $5 | Silver badge + Priority support + Early feature preview |
| 🥇 Gold | $10 | Gold badge + Priority support + Beta testing + README credit |

**Sponsor Links:**
- [🥉 Bronze Supporter ($1)](https://www.creem.io/payment/prod_NLloGph4FdG0QH5BN2DXr)
- [🥈 Silver Supporter ($5)](https://www.creem.io/payment/prod_1FqirOkv4JY21wExvWN3PW)
- [🥇 Gold Supporter ($10)](https://www.creem.io/payment/prod_2wV2YqQHJHsqrpWAipx40s)

Your support will be used for:
- 🚀 New feature development
- 🐛 Bug fixes and maintenance
- 📚 Documentation improvement
- 🎨 More themes and plugins

## 📄 License

MIT License

## 🤝 Contributing

Issues and Pull Requests are welcome!

### Contribution Guide

1. Fork this repository
2. Create feature branch (`git checkout -b feature/AmazingFeature`)
3. Commit changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to branch (`git push origin feature/AmazingFeature`)
5. Submit Pull Request

## 📞 Contact

- GitHub Issues: [https://github.com/noteva26/noteva/issues](https://github.com/noteva26/noteva/issues)
- Discussions: [https://github.com/noteva26/noteva/discussions](https://github.com/noteva26/noteva/discussions)

## ⭐ Star History

If you like Noteva, please give it a Star!

[![Star History Chart](https://api.star-history.com/svg?repos=noteva26/noteva&type=Date)](https://star-history.com/#noteva26/noteva&Date)

---

<p align="center">Made with ❤️ by Noteva Team</p>
