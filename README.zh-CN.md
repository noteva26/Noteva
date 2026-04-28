# Noteva

<p align="center">
  <a href="README.md">English</a> | <a href="README.zh-CN.md">简体中文</a>
</p>

<p align="center">
  <a href="https://github.com/noteva26/Noteva/releases"><img alt="版本" src="https://img.shields.io/badge/version-0.3.0-111827"></a>
  <a href="LICENSE"><img alt="许可证" src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue"></a>
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.75%2B-b7410e">
  <img alt="SQLite" src="https://img.shields.io/badge/SQLite-default-0f766e">
</p>

Noteva 是一个使用 Rust 构建的轻量现代博客系统。它默认使用 SQLite，可以单文件部署，同时保留简洁的写作、主题与插件体验，适合个人站点，也能按需扩展。

<p align="center">
  <img src="docs/images/noteva-hero.png" alt="Noteva 概览" width="900">
</p>

## 为什么选择 Noteva

- 轻量部署：单二进制文件，默认本地 SQLite，可选 MySQL 与 Redis。
- 清爽后台：文章、页面、分类标签、评论、文件、插件、主题、安全日志、备份和站点设置。
- Markdown 写作：预览、代码高亮、媒体上传、图片网格与 Shortcode 扩展。
- 沙箱插件：WASM 后端钩子、前端 JS/CSS 注入、权限、设置、存储与多语言文件。
- 前端框架无关主题：通过自动注入的 `window.Noteva` SDK，可用 React、Vue、原生 JavaScript 或任意前端栈开发主题。
- 内置国际化：常用后台与默认主题语言直接随程序提供。
- SEO 基础能力：固定链接、Sitemap、RSS Feed、robots.txt 与站点元信息。

## 截图

截图使用演示内容，主要用于展示整体界面和使用流程。

| 前台首页 | 文章阅读 |
| --- | --- |
| ![前台首页](docs/images/frontend-home.png) | ![文章阅读](docs/images/post-reading.png) |

| 文章管理 | Markdown 编辑器 |
| --- | --- |
| ![文章管理](docs/images/admin-articles.png) | ![Markdown 编辑器](docs/images/admin-editor.png) |

| 插件管理 | 主题管理 |
| --- | --- |
| ![插件管理](docs/images/admin-plugins.png) | ![主题管理](docs/images/admin-themes.png) |

<p align="center">
  <img src="docs/images/mobile-post.png" alt="移动端阅读页" width="360">
</p>

## 快速开始

Linux 或 macOS 可以使用安装脚本。脚本会检测平台，下载最新发布包，创建工作目录，并可注册系统服务。

```bash
curl -fsSL https://raw.githubusercontent.com/noteva26/Noteva/main/install.sh | bash
```

首次启动后访问：

```text
http://localhost:8080/manage/setup
```

在这里创建管理员账号，然后进入 `/manage` 管理后台。

## Docker

```bash
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

Docker Compose：

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

## 从源码构建

环境要求：

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

开发命令：

```bash
pnpm run dev:web      # 管理后台
pnpm run dev:theme    # 默认主题
cargo run --bin noteva
```

发布构建：

```bash
pnpm run build:frontend
cargo build --release
```

## 配置

Noteva 会从工作目录读取 `config.yml`。一个最小配置示例如下：

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

完整示例见 [config.example.yml](config.example.yml)。

## 插件

插件位于 `plugins/<plugin-id>/`，并通过 `plugin.json` 描述。插件可以包含浏览器资源、WASM 后端模块、设置表单、编辑器按钮和多语言文件。

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

后端插件通过 `wasmtime` 在 WASM 沙箱中运行。权限、钩子声明、存储和设置都是显式的，便于插件与核心应用保持隔离。

完整说明见：[插件开发](docs/plugin-development.md)。

## 主题

主题位于 `themes/<theme-name>/`。一个主题只需要清单文件和构建后的前端入口，因此可以使用任意前端框架实现。

```text
themes/my-theme/
|-- theme.json
|-- settings.json
|-- dist/index.html
`-- preview.png
```

运行时会自动注入 `window.Noteva` SDK。主题应通过该 SDK 获取站点数据、文章、页面、评论、导航、设置和插件插槽。

完整说明见：[主题开发](docs/theme-development.md)。

## 文档

| 文档 | 说明 |
| --- | --- |
| [插件开发](docs/plugin-development.md) | 插件包结构、钩子、权限、WASM 桥接、设置和前端集成。 |
| [主题开发](docs/theme-development.md) | 主题包结构、SDK 使用、设置、深色模式、插件插槽和兼容规则。 |
| [English Changelog](CHANGELOG.en.md) | 英文更新日志。 |
| [中文更新日志](CHANGELOG.md) | 中文更新日志。 |
| [许可证](LICENSE) | 许可证正文与附加条款。 |

## 产品方向

Noteva 会保持克制：安静的写作流程、紧凑的管理界面、安全的扩展点和简单部署。任何明显增加复杂度的功能，都应该能证明自己值得留下。

## 赞助

如果 Noteva 对你有帮助，欢迎赞助支持项目继续开发：

- [Bronze ($1)](https://www.creem.io/payment/prod_NLloGph4FdG0QH5BN2DXr)
- [Silver ($5)](https://www.creem.io/payment/prod_1FqirOkv4JY21wExvWN3PW)
- [Gold ($10)](https://www.creem.io/payment/prod_2wV2YqQHJHsqrpWAipx40s)

## 许可证

Noteva 使用 [GPL-3.0-or-later](LICENSE)，并带有插件和主题例外条款。

核心程序修改仍遵循 GPL。仅通过公开 SDK/API 与 Noteva 交互的插件和主题，可以使用自己的许可证。详情见 [LICENSE](LICENSE) 和 [COPYING](COPYING)。
