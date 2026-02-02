# Noteva

一个现代化的博客系统，使用 Rust 构建后端，支持多主题和插件扩展。

![preview](themes/default/preview.png)

## 特性

- 🚀 **高性能** - Rust + Axum 构建，轻量快速
- 🎨 **多主题** - 支持任意前端框架开发主题（React、Vue、原生 JS）
- 🔌 **插件系统** - 灵活的插件机制，无需修改核心代码
- 📝 **Markdown** - 完整的 Markdown 支持，含代码高亮
- 💬 **评论系统** - 内置评论功能，支持嵌套回复
- 🔐 **用户系统** - 注册、登录、权限管理
- 📱 **响应式** - 默认主题适配移动端

## 快速开始

### 环境要求

- Rust 1.70+
- Node.js 18+（主题开发）
- pnpm（推荐）

### 安装运行

```bash
# 克隆项目
git clone https://github.com/your-username/noteva.git
cd noteva

# 运行后端
cargo run

# 访问
# 前台: http://localhost:8080
# 后台: http://localhost:8080/manage
```

首次运行会自动创建数据库和默认管理员账号。

### 默认账号

- 用户名: `admin`
- 密码: `admin123`

**请登录后立即修改密码！**

## 项目结构

```
noteva/
├── src/                    # Rust 后端源码
│   ├── api/               # API 路由
│   ├── db/                # 数据库
│   ├── models/            # 数据模型
│   ├── services/          # 业务逻辑
│   ├── plugin/            # 插件系统
│   └── theme/             # 主题系统
├── themes/                 # 主题目录
│   ├── default/           # 默认主题 (Next.js)
│   └── retro/             # 复古主题 (原生 JS)
├── plugins/                # 插件目录
│   ├── hide-until-reply/  # 回复可见插件
│   └── music-player/      # 音乐播放器插件
├── data/                   # 数据目录
│   ├── noteva.db          # SQLite 数据库
│   └── plugins.json       # 插件状态
├── uploads/                # 上传文件目录
├── docs/                   # 文档
└── config.yml              # 配置文件
```

## 配置

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
```

## 主题开发

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

详见 [主题开发文档](docs/主题开发文档.md)

## 插件开发

插件放在 `plugins/` 目录，基本结构：

```
plugins/my-plugin/
├── plugin.json     # 插件配置
├── frontend.js     # 前端脚本
├── frontend.css    # 前端样式
└── settings.json   # 设置项定义
```

详见 [插件开发文档](docs/插件开发文档.md)

## 部署

### 方式一：直接运行

```bash
# 编译 Release 版本
cargo build --release

# 运行
./target/release/noteva
```

### 方式二：Docker

```dockerfile
FROM rust:1.75 as builder
WORKDIR /app
COPY . .
RUN cargo build --release

FROM debian:bookworm-slim
WORKDIR /app
COPY --from=builder /app/target/release/noteva .
COPY --from=builder /app/themes ./themes
COPY --from=builder /app/plugins ./plugins
COPY --from=builder /app/config.yml .
EXPOSE 8080
CMD ["./noteva"]
```

```bash
docker build -t noteva .
docker run -d -p 8080:8080 -v ./data:/app/data -v ./uploads:/app/uploads noteva
```

### 方式三：Systemd 服务

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

## API 端点

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
| POST | `/api/v1/auth/login` | 登录 |
| POST | `/api/v1/auth/register` | 注册 |
| GET | `/api/v1/auth/me` | 当前用户 |

## 技术栈

**后端**
- Rust
- Axum (Web 框架)
- SQLite (数据库)
- SQLx (数据库驱动)

**默认主题**
- Next.js 14
- React 18
- Tailwind CSS
- shadcn/ui

## 开发

```bash
# 后端开发（热重载）
cargo watch -x run

# 默认主题开发
cd themes/default
pnpm install
pnpm dev
```

## 路线图

- [x] 基础博客功能
- [x] 主题系统
- [x] 插件系统
- [x] 评论系统
- [ ] 搜索功能
- [ ] RSS 订阅
- [ ] 站点地图
- [ ] 多语言支持
- [ ] 更多主题

## 赞助支持

如果 Noteva 对你有帮助，欢迎赞助支持项目持续发展！

### 💝 赞助档位

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

## 许可证

MIT License

## 贡献

欢迎提交 Issue 和 Pull Request！
