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

- 🔌 **插件系统** - 灵活的插件机制，支持前端 JS/CSS 注入、Shortcode、钩子（WASM）
- 🎨 **多主题** - 支持任意前端框架开发主题（React、Vue、原生 JS）
- 📝 **Markdown** - 完整的 Markdown 支持，含代码高亮、数学公式、Shortcode
- 💬 **评论系统** - 内置评论功能，支持嵌套回复、表情、Markdown、审核
- 🔐 **用户系统** - 注册、登录、权限管理、安全日志
- 🌍 **国际化** - 支持中文、英文、繁体中文
- 📊 **缓存优化** - 智能缓存策略（内存 / Redis），ETag 支持
- 🔄 **热重载** - 插件和主题支持动态加载
- 💾 **备份恢复** - 一键备份/恢复，支持 Markdown 导出导入、WordPress XML 导入
- 🌐 **SEO** - 自动生成 Sitemap、RSS Feed、robots.txt
- 📄 **自定义页面** - 独立页面管理，支持自定义导航

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
│   └── default/           # 默认主题 (Vite + React)
├── plugins/                # 插件目录（可从商城安装）
├── web/                    # 管理后台 (Vite + React)
├── uploads/                # 上传文件目录（运行时生成）
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

## � 文档

| 文档 | 说明 |
|------|------|
| [API 参考](docs/api.md) | 完整 API 端点文档（50+ 端点） |
| [插件开发](docs/plugin-development.md) | 前端/WASM 插件开发指南，含 38+ 钩子参考表 |
| [主题开发](docs/theme-development.md) | 主题开发指南，SDK API 说明 |
| [v0.1.8 更新日志](docs/v0.1.8-beta.md) | 当前版本开发进度 |

## �🚢 部署

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
| GET | `/api/v1/articles` | 文章列表（分页、分类筛选） |
| GET | `/api/v1/articles/:slug` | 文章详情 |
| GET | `/api/v1/articles/resolve` | 通过查询条件解析文章 |
| GET | `/api/v1/categories` | 分类列表 |
| GET | `/api/v1/tags` | 标签列表 |
| GET | `/api/v1/comments/:article_id` | 文章评论列表 |
| GET | `/api/v1/comments/recent` | 全站最近评论 |
| POST | `/api/v1/comments` | 发表评论 |
| POST | `/api/v1/like` | 点赞/取消点赞 |
| GET | `/api/v1/like/check` | 检查点赞状态 |
| POST | `/api/v1/view/:article_id` | 文章浏览计数 |
| GET | `/api/v1/pages` | 页面列表 |
| GET | `/api/v1/page/:slug` | 页面详情 |
| GET | `/api/v1/nav` | 导航菜单 |
| GET | `/api/v1/theme/config` | 主题配置 |
| GET | `/api/v1/theme/info` | 主题信息 |
| GET | `/api/v1/theme/settings` | 主题设置 |
| GET | `/api/v1/plugins/enabled` | 已启用插件列表 |
| GET | `/sitemap.xml` | 站点地图 |
| GET | `/feed.xml` | RSS 订阅 |
| GET | `/robots.txt` | 爬虫规则 |

### 认证 API

| 方法 | 路径 | 说明 |
|-----|------|------|
| POST | `/api/v1/auth/login` | 登录 |
| POST | `/api/v1/auth/register` | 注册 |
| POST | `/api/v1/auth/logout` | 退出登录 |
| GET | `/api/v1/auth/me` | 当前用户信息 |
| PUT | `/api/v1/auth/profile` | 更新用户资料 |
| PUT | `/api/v1/auth/password` | 修改密码 |

### 管理 API（需要管理员权限）

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/v1/admin/dashboard` | 仪表盘数据 |
| GET | `/api/v1/admin/stats` | 系统统计 |
| GET | `/api/v1/admin/articles/:id` | 获取文章（按ID） |
| PUT | `/api/v1/admin/articles/:id` | 更新文章 |
| DELETE | `/api/v1/admin/articles/:id` | 删除文章 |
| POST | `/api/v1/articles` | 创建文章 |
| GET | `/api/v1/admin/categories` | 分类管理 |
| POST | `/api/v1/admin/categories` | 创建分类 |
| PUT | `/api/v1/admin/categories/:id` | 更新分类 |
| DELETE | `/api/v1/admin/categories/:id` | 删除分类 |
| POST | `/api/v1/admin/tags` | 创建标签 |
| DELETE | `/api/v1/admin/tags/:id` | 删除标签 |
| GET | `/api/v1/admin/comments` | 评论管理列表 |
| GET | `/api/v1/admin/comments/pending` | 待审核评论 |
| POST | `/api/v1/admin/comments/:id/approve` | 审核通过评论 |
| POST | `/api/v1/admin/comments/:id/reject` | 拒绝评论 |
| DELETE | `/api/v1/admin/comments/:id` | 删除评论 |
| GET | `/api/v1/admin/themes` | 主题列表 |
| POST | `/api/v1/admin/themes/switch` | 切换主题 |
| GET | `/api/v1/admin/settings` | 站点设置 |
| PUT | `/api/v1/admin/settings` | 更新设置 |
| GET | `/api/v1/admin/backup` | 下载完整备份 |
| POST | `/api/v1/admin/backup/restore` | 恢复备份 |
| GET | `/api/v1/admin/backup/export-markdown` | 导出 Markdown |
| POST | `/api/v1/admin/backup/import` | 导入文章 |
| GET | `/api/v1/admin/login-logs` | 登录安全日志 |

详细 API 文档请参见 [API 参考文档](docs/api.md)。

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

# 管理后台开发
cd web
pnpm install
pnpm dev
```



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
