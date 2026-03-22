# Noteva

> 轻量、极简、现代化的博客系统

[English](README.en.md) | 简体中文

一个使用 Rust 构建的高性能博客系统，支持多主题和插件扩展。单文件部署，开箱即用。

![preview](themes/default/preview.png)

## ✨ 特性

- 🪶 **轻量高效** — 单文件部署，内存 < 50MB，冷启动 < 1 秒
- 🎨 **主题系统** — 支持任意前端框架开发主题，热重载切换
- 🔌 **插件系统** — 前端 JS/CSS 注入、Shortcode、WASM 后端钩子
- 📝 **Markdown** — 代码高亮、数学公式、Shortcode 扩展
- 💬 **评论系统** — 嵌套回复、表情、审核、Markdown
- 🌍 **国际化** — 简体中文 / 繁體中文 / English
- 💾 **备份恢复** — 一键备份、Markdown 导出、WordPress 导入
- 🌐 **SEO** — Sitemap、RSS Feed、robots.txt 自动生成
- 🔐 **安全** — 登录限速、CSRF 保护、安全日志

## 🌐 在线演示

| | |
|---|---|
| **演示站** | [demo.noteva.org](https://demo.noteva.org/) |
| **管理后台** | [demo.noteva.org/manage](https://demo.noteva.org/manage) |
| **账号 / 密码** | `demo` / `demo123456` |

## 🚀 部署

### 一键脚本（推荐）

适用于 Linux / macOS：

```bash
curl -fsSL https://raw.githubusercontent.com/noteva26/Noteva/main/install.sh | bash
```

自动检测架构、下载二进制、交互式配置、注册系统服务。再次运行进入升级模式。

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

### 手动下载

```bash
# 下载最新版
wget https://github.com/noteva26/Noteva/releases/latest/download/noteva-linux-x86_64.tar.gz

# 解压运行
tar -xzf noteva-linux-x86_64.tar.gz
chmod +x noteva && ./noteva
```

### 从源码编译

```bash
git clone https://github.com/noteva26/noteva.git
cd noteva
cargo run --release
```

> **首次访问**：打开 `http://localhost:8080/manage/setup` 设置管理员账号。

## ⚙️ 配置

编辑 `config.yml`（首次运行自动生成）：

```yaml
server:
  host: "0.0.0.0"
  port: 8080

database:
  url: "data/noteva.db"    # SQLite（默认）
  # url: "mysql://user:pass@localhost/noteva"  # MySQL

cache:
  enabled: true
  type: "memory"           # memory 或 redis

upload:
  dir: "uploads"
  max_size: 10485760       # 10MB
```

## 📚 文档

| 文档 | 说明 |
|------|------|
| [API 参考](docs/api.md) | 完整 API 端点文档 |
| [插件开发](docs/plugin-development.md) | 前端/WASM 插件开发指南 |
| [主题开发](docs/theme-development.md) | 主题开发指南，SDK API 说明 |
| [更新日志](CHANGELOG.md) | 版本更新记录 |

## 💝 赞助

如果 Noteva 对你有帮助，欢迎赞助支持！

- [🥉 Bronze ($1)](https://www.creem.io/payment/prod_NLloGph4FdG0QH5BN2DXr)
- [🥈 Silver ($5)](https://www.creem.io/payment/prod_1FqirOkv4JY21wExvWN3PW)
- [🥇 Gold ($10)](https://www.creem.io/payment/prod_2wV2YqQHJHsqrpWAipx40s)

## 📄 许可证

[GPL-3.0](LICENSE) with Plugin/Theme Exception

核心程序采用 GPL-3.0 许可证。通过 SDK/API 开发的主题和插件**不受 GPL 约束**，可自由选择任何许可证。详见 [LICENSE](LICENSE) 和 [COPYING](COPYING)。

---

<p align="center">Made with ❤️ by Noteva Team</p>
