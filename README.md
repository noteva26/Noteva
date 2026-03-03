# Noteva

> 轻量、极简、现代化的博客系统

[English](README.en.md) | 简体中文

一个使用 Rust 构建的高性能博客系统，支持多主题和插件扩展。专注于简洁、快速和可扩展性。

![preview](themes/default/preview.png)

## ✨ 核心优势

### 🪶 轻量化
- **单文件部署** - 编译后仅一个可执行文件，无需复杂依赖
- **低资源占用** - 内存占用 < 50MB，CPU 使用率极低
- **SQLite 数据库** - 无需额外数据库服务，开箱即用
- **快速启动** - 冷启动 < 1 秒

### 🎯 极简设计
- **简洁界面** - 专注内容，去除冗余功能
- **直观管理** - 清晰的管理后台，易于上手
- **零配置启动** - 首次运行自动初始化，无需复杂配置
- **Markdown 优先** - 纯粹的写作体验

### 🚀 现代化
- **Rust 构建** - 内存安全、高性能、并发友好
- **现代前端** - Vite + React 18 + Tailwind CSS
- **热重载** - 插件和主题支持热重载，无需重启
- **响应式设计** - 完美适配桌面和移动设备

## 🎨 特性

- 🔌 **插件系统** - 灵活的插件机制，支持前端 JS/CSS 注入、Shortcode、钩子
- 🎨 **多主题** - 支持任意前端框架开发主题（React、Vue、原生 JS）
- 📝 **Markdown** - 完整的 Markdown 支持，含代码高亮和 Shortcode
- 💬 **评论系统** - 内置评论功能，支持嵌套回复、表情、Markdown
- 🔐 **用户系统** - 注册、登录、权限管理、安全日志
- 🌍 **国际化** - 支持中文、英文、繁体中文
- 📊 **缓存优化** - 智能缓存策略，提升性能
- 🔄 **热重载** - 插件和主题支持动态加载

## 🚀 快速开始

### 直接运行（推荐）

#### 环境要求
- Rust 1.70+

```bash
# 克隆项目
git clone https://github.com/noteva26/noteva.git
cd noteva

# 编译运行
cargo run --release

# 访问
# 前台: http://localhost:8080
# 后台: http://localhost:8080/manage
```

### 一键脚本部署（推荐）

适用于 Linux / macOS，自动下载最新版、交互式配置、注册系统服务：

```bash
curl -fsSL https://raw.githubusercontent.com/noteva26/Noteva/main/install.sh | bash
```

脚本会自动完成：
1. 🔍 检测系统架构（x86_64 / arm64）
2. 📦 下载最新 Release 二进制文件
3. ⚙️ 交互式配置（数据库、缓存、端口等）
4. 🔧 注册 systemd / launchd 系统服务（可选）
5. 🚀 启动 Noteva

已安装过？再次运行脚本会进入**升级模式**，仅更新二进制文件，保留配置和数据。

### 手动下载二进制

```bash
# 下载最新版本
wget https://github.com/noteva26/Noteva/releases/latest/download/noteva-linux-x86_64.tar.gz

# 解压
tar -xzf noteva-linux-x86_64.tar.gz

# 运行
chmod +x noteva && ./noteva
```

### Docker 部署

如果你更喜欢容器化部署：

```bash
# 拉取镜像
docker pull ghcr.io/noteva26/noteva:latest

# 运行
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

或使用 Docker Compose：

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

### 首次使用

首次运行会自动：
1. 创建 SQLite 数据库
2. 执行数据库迁移
3. 创建必要的目录结构

**首次访问管理后台时，需要设置管理员账号。**

访问 `http://localhost:8080/manage/setup` 完成初始化设置。

## 📦 项目结构

```
noteva/
├── src/                    # Rust 后端源码
│   ├── api/               # API 路由
│   ├── db/                # 数据库层
│   ├── models/            # 数据模型
│   ├── services/          # 业务逻辑
│   ├── cache/             # 缓存系统
│   ├── plugin/            # 插件系统
│   └── theme/             # 主题系统
├── themes/                 # 主题目录
│   ├── default/           # 默认主题 (Vite + React)
│   └── prose/             # Prose 主题 (Vite + React)
├── plugins/                # 插件目录
│   ├── hide-until-reply/  # 回复可见插件
│   ├── music-player/      # 音乐播放器插件
│   ├── video-embed/       # 视频嵌入插件
│   ├── friendlinks/       # 友情链接插件
│   └── profile/           # 个人主页插件
├── web/                    # 管理后台 (Vite + React)
├── data/                   # 数据目录
│   └── noteva.db          # SQLite 数据库
├── uploads/                # 上传文件目录
├── docs/                   # 文档
└── config.yml              # 配置文件
```

## ⚙️ 配置

编辑 `config.yml`：

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
  type: "memory"  # memory 或 redis
```

## 🎨 主题开发

主题放在 `themes/` 目录，基本结构：

```
themes/my-theme/
├── theme.json      # 主题配置
├── dist/           # 构建输出
│   └── index.html  # 入口文件
└── preview.png     # 预览图
```

Noteva 会自动注入 SDK，提供 `window.Noteva` 全局对象：

```javascript
Noteva.ready(async () => {
  const site = await Noteva.site.getInfo();
  const { articles } = await Noteva.articles.list();
  // ...
});
```

详见 [主题开发文档](docs/theme-development.md)

## 🔌 插件开发

插件放在 `plugins/` 目录，基本结构：

```
plugins/my-plugin/
├── plugin.json     # 插件配置
├── frontend.js     # 前端脚本
├── frontend.css    # 前端样式
└── settings.json   # 设置项定义
```

### 官方插件

- **hide-until-reply** - 回复可见插件
- **music-player** - 音乐播放器插件
- **video-embed** - 视频嵌入插件（YouTube、Bilibili、Twitter/X）
- **friendlinks** - 友情链接插件
- **profile** - 个人主页插件

详见 [插件开发文档](docs/plugin-development.md)

## 🚢 部署

### 直接运行

```bash
# 编译 Release 版本
cargo build --release

# 运行
./target/release/noteva
```

### Systemd 服务

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

### Docker 部署

```bash
# 使用官方镜像
docker pull ghcr.io/noteva26/noteva:latest

# 运行
docker run -d \
  -p 8080:8080 \
  -v ./data:/app/data \
  -v ./uploads:/app/uploads \
  --name noteva \
  ghcr.io/noteva26/noteva:latest
```

### 反向代理 (Nginx)

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

## 📡 API 端点

### 公开 API

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/v1/site/info` | 站点信息 |
| GET | `/api/v1/articles` | 文章列表 |
| GET | `/api/v1/articles/:slug` | 文章详情 |
| GET | `/api/v1/page/:slug` | 页面详情 |
| GET | `/api/v1/categories` | 分类列表 |
| GET | `/api/v1/tags` | 标签列表 |
| GET | `/api/v1/comments/:article_id` | 评论列表 |
| POST | `/api/v1/comments` | 发表评论 |
| GET | `/api/v1/plugins/enabled` | 已启用插件列表 |

### 认证 API

| 方法 | 路径 | 说明 |
|-----|------|------|
| POST | `/api/v1/auth/login` | 登录 |
| POST | `/api/v1/auth/register` | 注册 |
| POST | `/api/v1/auth/logout` | 退出登录 |
| GET | `/api/v1/auth/me` | 当前用户信息 |

### 管理 API（需要认证）

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/v1/admin/articles` | 管理文章列表 |
| POST | `/api/v1/admin/articles` | 创建文章 |
| PUT | `/api/v1/admin/articles/:id` | 更新文章 |
| DELETE | `/api/v1/admin/articles/:id` | 删除文章 |
| GET | `/api/v1/admin/plugins` | 插件列表 |
| POST | `/api/v1/admin/plugins/:id/toggle` | 启用/禁用插件 |
| GET | `/api/v1/admin/themes` | 主题列表 |
| POST | `/api/v1/admin/themes/switch` | 切换主题 |

## 🛠️ 技术栈

**后端**
- Rust 1.75+
- Axum (Web 框架)
- SQLite / MySQL (数据库)
- SQLx (数据库驱动)
- Tokio (异步运行时)

**前端（主题 & 管理后台）**
- Vite 5
- React 18
- TypeScript
- Tailwind CSS
- shadcn/ui

## 💻 开发

```bash
# 后端开发（热重载）
cargo watch -x run

# 默认主题开发
cd themes/default
pnpm install
pnpm dev

# Prose 主题开发
cd themes/prose
pnpm install
pnpm dev

# 管理后台开发
cd web
pnpm install
pnpm dev
```

## 🗺️ 路线图

### v0.1.0 ✅
- [x] 基础博客功能
- [x] 主题系统
- [x] 插件系统（实验性）
- [x] 评论系统
- [x] 用户系统
- [x] 缓存优化
- [x] 国际化支持
- [x] 热重载机制

### v0.1.1-beta ✅
- [x] 插件数据存储能力
- [x] 友链 & 个人主页插件
- [x] 视频嵌入插件
- [x] SEO 优化（Rust 后端注入 meta 标签）
- [x] 前端从 Next.js 迁移至 Vite

### v0.1.4-beta（当前版本）
- [x] 独立商城平台（插件/主题商城）
- [x] S3/COS 图床插件（WASM）
- [x] 在线安装统一策略（Release 优先 + 仓库验证回退）
- [x] 插件设置敏感字段脱敏
- [x] 前后端协同性优化
- [x] 性能优化（WASM 预编译缓存、子进程池化、前端 code splitting）
- [x] 自定义 CSS/JS 注入
- [x] 页脚增强

### v0.1.2-beta
- [x] Prose 主题（三栏布局，参考安知鱼视觉风格）
- [x] 主题列表修复 & 元数据读取优化
- [x] 插件兼容性修复（视频嵌入、友链、个人主页）
- [x] TLS 依赖切换至 rustls（支持交叉编译）
- [ ] 代码清理 & 文档更新
- [ ] 稳定性优化

### v0.1.3-beta
- [x] 管理后台插件集成
- [x] 编辑器扩展 API
- [ ] 更多编辑器功能

### v0.1.4-beta（计划中）
- [ ] 插件 API 路由
- [ ] 生命周期钩子
- [ ] 前台路由注册

### v0.1.5（正式版）
- [ ] 插件生态完善
- [ ] 官方插件库
- [ ] 完整文档

### 未来版本
- [ ] 全文搜索
- [ ] RSS 订阅
- [ ] 站点地图
- [ ] 邮件通知
- [ ] 更多主题

## 💝 赞助支持

如果 Noteva 对你有帮助，欢迎赞助支持项目持续发展！

### 赞助档位

| 档位 | 金额 | 权益 |
|------|------|------|
| 🥉 Bronze | $1 | Supporter badge + 我们的感谢 |
| 🥈 Silver | $5 | Silver badge + 优先支持 + 早期功能预告 |
| 🥇 Gold | $10 | Gold badge + 优先支持 + Beta 测试 + README 署名 |

**赞助链接：**
- [🥉 Bronze Supporter ($1)](https://www.creem.io/payment/prod_NLloGph4FdG0QH5BN2DXr)
- [🥈 Silver Supporter ($5)](https://www.creem.io/payment/prod_1FqirOkv4JY21wExvWN3PW)
- [🥇 Gold Supporter ($10)](https://www.creem.io/payment/prod_2wV2YqQHJHsqrpWAipx40s)

你的支持将用于：
- 🚀 新功能开发
- 🐛 Bug 修复和维护
- 📚 文档完善
- 🎨 更多主题和插件

## 📄 许可证

MIT License

## 🤝 贡献

欢迎提交 Issue 和 Pull Request！

### 贡献指南

1. Fork 本仓库
2. 创建特性分支 (`git checkout -b feature/AmazingFeature`)
3. 提交改动 (`git commit -m 'Add some AmazingFeature'`)
4. 推送到分支 (`git push origin feature/AmazingFeature`)
5. 提交 Pull Request

## 📞 联系方式

- GitHub Issues: [https://github.com/noteva26/noteva/issues](https://github.com/noteva26/noteva/issues)
- 讨论区: [https://github.com/noteva26/noteva/discussions](https://github.com/noteva26/noteva/discussions)

## ⭐ Star History

如果觉得 Noteva 不错，请给个 Star 支持一下！

[![Star History Chart](https://api.star-history.com/svg?repos=noteva26/noteva&type=Date)](https://star-history.com/#noteva26/noteva&Date)

---

<p align="center">Made with ❤️ by Noteva Team</p>
