# Noteva API 参考文档

> API 基准路径: `/api/v1`

## 认证方式

Noteva 使用基于 Cookie 的 Session 认证。登录后浏览器会自动管理 `session` Cookie。

管理 API 请求需要附带 CSRF Token（通过 `X-CSRF-Token` 请求头）。

---

## 公开 API

### 站点信息

#### `GET /api/v1/site/info`

返回站点基本信息。

**响应示例：**
```json
{
  "name": "My Blog",
  "description": "A personal blog",
  "language": "zh-CN",
  "theme": "default"
}
```

---

### 文章

#### `GET /api/v1/articles`

获取文章列表（分页）。

**查询参数：**
| 参数 | 类型 | 说明 |
|------|------|------|
| `page` | number | 页码（默认 1） |
| `per_page` | number | 每页数量（默认 10） |
| `category` | string | 分类筛选 |
| `tag` | string | 标签筛选 |
| `status` | string | 状态筛选（admin） |
| `sort` | string | 排序字段 |

**响应示例：**
```json
{
  "articles": [...],
  "total": 42,
  "page": 1,
  "per_page": 10
}
```

#### `GET /api/v1/articles/:slug`

通过 slug 获取文章详情。

#### `GET /api/v1/articles/resolve`

通过查询条件解析文章（支持多种查找方式）。

---

### 分类 & 标签

#### `GET /api/v1/categories`

获取所有分类。

#### `GET /api/v1/tags`

获取所有标签。

---

### 评论

#### `GET /api/v1/comments/:article_id`

获取文章的评论列表（树形结构）。

**响应示例：**
```json
{
  "comments": [
    {
      "id": 1,
      "article_id": 5,
      "user_id": null,
      "parent_id": null,
      "nickname": "Guest",
      "content": "Great article!",
      "status": "approved",
      "created_at": "2024-01-01T00:00:00Z",
      "avatar_url": "https://...",
      "like_count": 0,
      "is_liked": false,
      "replies": [...]
    }
  ]
}
```

#### `GET /api/v1/comments/recent`

获取全站最近评论。

**查询参数：**
| 参数 | 类型 | 说明 |
|------|------|------|
| `limit` | number | 返回数量（默认 10，最大 50） |

#### `POST /api/v1/comments`

发表评论。

**请求体：**
```json
{
  "article_id": 5,
  "parent_id": null,
  "nickname": "Guest",
  "email": "guest@example.com",
  "content": "Hello world!"
}
```

> 若站点开启了登录评论限制，未登录用户无法评论。

---

### 点赞

#### `POST /api/v1/like`

点赞或取消点赞。

**请求体：**
```json
{
  "target_type": "article",
  "target_id": 5
}
```

> `target_type` 可选值: `article`, `comment`

#### `GET /api/v1/like/check`

检查当前用户是否已点赞。

**查询参数：** `target_type`, `target_id`

---

### 浏览计数

#### `POST /api/v1/view/:article_id`

增加文章浏览计数。返回 `200 OK`。

---

### 页面

#### `GET /api/v1/pages`

获取页面列表。

#### `GET /api/v1/page/:slug`

通过 slug 获取页面详情。

---

### 导航

#### `GET /api/v1/nav`

获取导航菜单项。

---

### 主题

#### `GET /api/v1/theme/config`

获取当前主题配置（theme.json 内容）。

#### `GET /api/v1/theme/info`

获取当前主题基本信息。

#### `GET /api/v1/theme/settings`

获取当前主题的公开设置。

---

### 插件

#### `GET /api/v1/plugins/enabled`

获取已启用插件列表。

#### `GET /api/v1/plugins/assets/plugins.js`

获取所有已启用插件的合并 JavaScript。

#### `GET /api/v1/plugins/assets/plugins.css`

获取所有已启用插件的合并 CSS。

#### `GET /api/v1/plugins/:id/data/:key`

获取插件数据（只读）。

#### `ANY /api/v1/plugins/:id/api/*path`

代理请求到 WASM 插件的自定义 API 路由。

---

### SEO 端点

| 路径 | 说明 |
|------|------|
| `/sitemap.xml` | 站点地图（XML） |
| `/feed.xml` / `/rss.xml` / `/feed` | RSS 订阅 |
| `/robots.txt` | 爬虫规则 |

---

## 认证 API

#### `POST /api/v1/auth/login`

用户登录。

**请求体：**
```json
{
  "username": "admin",
  "password": "password123"
}
```

#### `POST /api/v1/auth/register`

用户注册。

#### `POST /api/v1/auth/logout`

退出登录。

#### `GET /api/v1/auth/me`

获取当前登录用户信息（需要认证）。

#### `PUT /api/v1/auth/profile`

更新用户资料（需要认证）。

#### `PUT /api/v1/auth/password`

修改密码（需要认证）。

---

## 管理 API

> 所有管理 API 需要管理员权限（admin 角色）。

### 仪表盘

#### `GET /api/v1/admin/dashboard`

获取仪表盘统计数据。

#### `GET /api/v1/admin/stats`

获取系统运行统计信息。

---

### 文章管理

#### `GET /api/v1/admin/articles/:id`

通过 ID 获取文章详情。

#### `POST /api/v1/articles`

创建文章。

**请求体：**
```json
{
  "title": "Hello World",
  "slug": "hello-world",
  "content": "# Hello\n\nThis is my first post.",
  "category_id": 1,
  "status": "published",
  "tag_ids": [1, 2]
}
```

#### `PUT /api/v1/admin/articles/:id`

更新文章。

#### `DELETE /api/v1/admin/articles/:id`

删除文章。

---

### 分类管理

#### `POST /api/v1/admin/categories`

创建分类。

#### `PUT /api/v1/admin/categories/:id`

更新分类。

#### `DELETE /api/v1/admin/categories/:id`

删除分类。

---

### 标签管理

#### `POST /api/v1/admin/tags`

创建标签。

#### `DELETE /api/v1/admin/tags/:id`

删除标签。

---

### 评论管理

#### `GET /api/v1/admin/comments`

获取评论列表（支持状态筛选和分页）。

**查询参数：** `status`, `page`, `per_page`

#### `GET /api/v1/admin/comments/pending`

获取待审核评论。

#### `POST /api/v1/admin/comments/:id/approve`

审核通过评论。

#### `POST /api/v1/admin/comments/:id/reject`

拒绝评论（标记为垃圾评论）。

#### `DELETE /api/v1/admin/comments/:id`

删除评论。

---

### 主题管理

#### `GET /api/v1/admin/themes`

获取已安装主题列表。

#### `POST /api/v1/admin/themes/switch`

切换当前主题。

#### `POST /api/v1/admin/themes/upload`

上传主题（ZIP 文件）。

#### `POST /api/v1/admin/themes/github/install`

从 GitHub 安装主题。

#### `DELETE /api/v1/admin/themes/:name`

删除主题。

---

### 插件管理

#### `GET /api/v1/admin/plugins` (nested router)

通过管理路由获取插件列表及管理操作。

#### `POST /api/v1/admin/plugins/upload`

上传插件（ZIP 文件）。

#### `POST /api/v1/admin/plugins/github/install`

从 GitHub 安装插件。

#### `DELETE /api/v1/admin/plugins/:id/uninstall`

卸载插件。

#### `POST /api/v1/admin/plugins/reload`

重新加载所有插件。

---

### 站点设置

#### `GET /api/v1/admin/settings`

获取所有站点设置。

#### `PUT /api/v1/admin/settings`

批量更新设置。

**请求体：**
```json
{
  "site_name": "My Blog",
  "site_description": "A personal blog",
  "comment_moderation": "true",
  "require_login_to_comment": "false"
}
```

---

### 备份与恢复

#### `GET /api/v1/admin/backup`

下载完整备份（ZIP 格式，包含数据库数据和上传文件）。

#### `POST /api/v1/admin/backup/restore`

从备份 ZIP 恢复数据。

**请求方式：** `multipart/form-data`，字段名 `file`

#### `GET /api/v1/admin/backup/export-markdown`

导出所有文章为 Markdown ZIP（含 YAML frontmatter）。

#### `POST /api/v1/admin/backup/import`

导入文章。自动检测格式：

- **ZIP 文件** → Markdown 导入（解析 YAML frontmatter）
- **XML 文件** → WordPress WXR 导入

**请求方式：** `multipart/form-data`，字段名 `file`

**响应示例：**
```json
{
  "status": "ok",
  "imported": 15,
  "skipped": 2,
  "errors": ["Skipped 'hello': slug 'hello' already exists"]
}
```

---

### 安全

#### `GET /api/v1/admin/login-logs`

获取登录安全日志。

---

### 系统更新

#### `GET /api/v1/admin/update-check`

检查新版本。

#### `POST /api/v1/admin/update-perform`

执行自动更新。

---

## 错误响应格式

所有 API 错误返回统一格式：

```json
{
  "error": {
    "message": "Error description",
    "code": "ERROR_CODE"
  }
}
```

常见 HTTP 状态码：

| 状态码 | 说明 |
|--------|------|
| 200 | 成功 |
| 201 | 创建成功 |
| 204 | 删除成功 |
| 400 | 请求参数错误 |
| 401 | 未认证 |
| 403 | 无权限 |
| 404 | 资源未找到 |
| 500 | 服务器内部错误 |
