# Noteva 插件开发文档

> 版本：v0.2.8

## 快速开始

Noteva 插件系统允许你扩展博客功能，无需修改核心代码。

支持两种插件类型：
- **前端插件**：通过 JavaScript/CSS 在浏览器端运行，使用 Noteva SDK
- **WASM 后端插件**：通过 WebAssembly 在服务端沙盒中运行，可参与后端钩子处理

### 插件目录结构

```
plugins/
  my-plugin/
    plugin.json        # 插件配置（必需）
    frontend.js        # 前端脚本（可选）
    frontend.css       # 前端样式（可选）
    backend.wasm       # WASM 后端模块（可选）
    settings.json      # 设置项定义（可选）
    editor.json        # 编辑器工具栏按钮（可选）
    migrations/        # 插件私有数据库迁移 SQL（database=true 时必需）
      001_init.sql
    locales/           # 国际化翻译文件（可选）
      zh-CN.json
      en.json
    README.md          # 插件说明（可选）
```

### v0.2.8 规范速览

- `plugin.json` 必须声明 `"schema": 1`，插件目录名必须与 `id` 一致。
- `settings: true` 时必须提供 `settings.json`；没有设置项时不要保留空的 `settings.json`。
- `settings.json` 必须声明 `"schema": 1`，结构与主题设置一致：`sections -> fields`。
- `homepage` 已替换为 `repository`，用于后续更新检查，支持 GitHub URL 或 `owner/repo`。
- `hooks.backend`、`hooks.frontend`、`hooks.editor`、`permissions` 都会在安装/加载时校验，未知声明会直接拒绝。
- `database: true` 必须同时声明 `"database"` 权限、提供 `backend.wasm` 和 `migrations/*.sql`；运行时 SQL 只能访问插件自己的 `plugin_{id}_*` 表。

JSON Schema 文件位于：
- `docs/schemas/plugin.schema.json`
- `docs/schemas/plugin-settings.schema.json`

### 安全约束速览

- `/api/v1/plugins/proxy` 通用代理已禁用，不再允许前端插件提交任意 URL 让主程序代转发请求。
- 用户自定义中转站仍然支持：插件可以把 `api_url`、`api_key` 等放在 `settings.json` 中，由用户在插件设置里填写；WASM 后端在声明 `network` 权限后，通过 `host_http_request` 请求这些用户配置的公开 `http/https` 地址。
- WASM 网络请求会阻止本机、内网、链路本地、组播、广播和云厂商元数据地址；URL 不能包含用户名密码；HTTP 跳转默认关闭。
- 插件安装包不允许路径穿越、绝对路径、Windows 盘符路径、符号链接、特殊文件、异常包名、超大单文件或超大解包体积。
- 插件数据库迁移会按文件名顺序执行，并和 `plugin_migrations` 记录写入放在同一事务里；失败时不会记录为已执行。

---

## plugin.json 配置

```json
{
  "schema": 1,
  "id": "my-plugin",
  "name": "我的插件",
  "version": "1.0.0",
  "description": "插件描述",
  "author": "Your Name",
  "repository": "https://github.com/your-name/my-plugin",
  "license": "MIT",
  "requires": {
    "noteva": ">=0.2.8"
  },
  "hooks": {
    "frontend": ["content_render"],
    "backend": ["article_after_create", "plugin_activate", "plugin_action"]
  },
  "permissions": ["network", "storage", "read_articles"],
  "shortcodes": [],
  "settings": true
}
```

### 字段说明

| 字段 | 必需 | 说明 |
|-----|------|------|
| `schema` | 是 | 插件清单格式版本，目前固定为 `1` |
| `id` | 是 | 插件唯一标识，小写字母和连字符，必须与目录名一致 |
| `name` | 是 | 插件显示名称 |
| `version` | 是 | 版本号（语义化版本） |
| `description` | 是 | 插件描述 |
| `author` | 是 | 作者 |
| `repository` | 是 | GitHub 仓库地址或 `owner/repo`，用于更新 |
| `license` | 是 | 开源协议 |
| `requires.noteva` | 是 | 最低版本要求，如 `>=0.2.8` |
| `hooks.frontend` | | 前端钩子列表 |
| `hooks.backend` | | 后端钩子列表（需要 backend.wasm） |
| `hooks.editor` | | 编辑器扩展，如 `["toolbar"]`（需要 editor.json） |
| `permissions` | | WASM 插件所需权限列表 |
| `shortcodes` | | 注册的短代码列表 |
| `settings` | | 是否有设置项（true/false） |
| `api` | | 是否暴露公开插件 API（需要 backend.wasm） |
| `database` | | 是否启用插件私有数据库表；为 `true` 时必须声明 `"database"` 权限并提供 `migrations/*.sql` |
| `pages` | | 自动创建的页面列表，每项含 `slug` 和 `title`。启用插件时自动创建缺失的 Page 记录（不覆盖已有页面） |

### 可用权限

| 权限 | 说明 |
|-----|------|
| `network` | 发起 HTTP 请求（调用外部 API） |
| `storage` | 插件数据存储（key-value，按插件 ID 隔离） |
| `database` | 插件私有 SQL 表访问；仅允许访问 `plugin_{id}_*` 表，`id` 中的 `-` 会转为 `_` |
| `read_articles` | 查询文章列表（用于批量处理等场景） |
| `read_comments` | 读取评论数据 |
| `write_articles` | 修改文章数据 |

> `storage` 权限会自动授予所有 WASM 插件，但建议显式声明以保持清晰。

---

## settings.json 配置

定义插件的可配置项，后台会自动生成设置界面。

> v0.1.6-beta 起，插件和主题的设置界面统一使用侧边抽屉（Sheet）+ 手风琴折叠（Accordion）展示，每个 section 独立折叠，默认展开第一个。设置渲染逻辑由共享组件 `SettingsRenderer` 统一处理，插件和主题体验一致。

```json
{
  "schema": 1,
  "sections": [
    {
      "id": "api",
      "title": "API 配置",
      "fields": [
        {
          "id": "api_url",
          "type": "text",
          "label": "API 地址",
          "default": "https://api.example.com",
          "placeholder": "https://..."
        },
        {
          "id": "api_key",
          "type": "text",
          "label": "API Key",
          "default": "",
          "placeholder": "sk-...",
          "secret": true
        }
      ]
    },
    {
      "id": "display",
      "title": "显示设置",
      "fields": [
        {
          "id": "enabled",
          "type": "switch",
          "label": "启用功能",
          "default": true
        },
        {
          "id": "max_count",
          "type": "number",
          "label": "最大数量",
          "default": 10,
          "min": 1,
          "max": 100
        }
      ]
    }
  ]
}
```

### 支持的字段类型

| 类型 | 说明 | 额外属性 |
|-----|------|---------|
| `text` | 单行文本 | `placeholder`, `maxLength`, `secret` |
| `textarea` | 多行文本 | `rows` |
| `number` | 数字 | `min`, `max`, `step` |
| `switch` | 开关 | - |
| `select` | 下拉选择 | `options` |
| `color` | 颜色选择器 | - |
| `array` | 数组/列表 | `itemFields` |

### secret 字段

在字段定义中添加 `"secret": true`，该字段的值不会出现在公开 API（`GET /plugins/enabled`）的响应中。适用于 API Key、密码等敏感信息。

- 公开 API：secret 字段被过滤，前端无法获取
- WASM 钩子：secret 字段正常注入（后端需要使用）
- Action API 响应：所有插件设置字段自动剥离

### array 类型

用于创建可视化的列表编辑器，支持添加、删除、排序。

```json
{
  "id": "links",
  "type": "array",
  "label": "友情链接",
  "default": [],
  "itemFields": [
    { "id": "name", "label": "名称", "type": "text", "required": true },
    { "id": "url", "label": "链接", "type": "text", "required": true },
    { "id": "avatar", "label": "头像", "type": "text" }
  ]
}
```

前端获取：`(await Noteva.plugins.ready('my-plugin')).links` 直接是数组。

---

## editor.json 编辑器工具栏配置

插件可以在管理后台的 Markdown 编辑器工具栏中注册按钮，统一收纳在 ⊕ 下拉菜单中。

### 配置方式

在插件目录下创建 `editor.json`，并在 `plugin.json` 中声明：

```json
{
  "hooks": {
    "editor": ["toolbar"]
  }
}
```

### editor.json 格式

```json
{
  "toolbar": [
    {
      "id": "my-button",
      "label": "按钮名称",
      "icon": "🔒",
      "insertBefore": "[shortcode]",
      "insertAfter": "[/shortcode]"
    }
  ]
}
```

### 字段说明

| 字段 | 必需 | 说明 |
|-----|------|------|
| `id` | 是 | 按钮唯一标识 |
| `label` | 是 | 按钮显示文字（下拉菜单中显示） |
| `icon` | 否 | Emoji 图标，显示在 label 前面。无 icon 时显示默认图标 |
| `insertBefore` | 是 | 点击后在光标前（或选中文本前）插入的文本 |
| `insertAfter` | 是 | 点击后在光标后（或选中文本后）插入的文本 |

### 工作原理

1. 后端 `Plugin::get_editor_config()` 读取插件目录下的 `editor.json`
2. 通过 `GET /api/v1/plugins/enabled` 接口返回各插件的 `editor_config`
3. 前端编辑器收集所有插件的 toolbar 按钮，渲染到 ⊕ 下拉菜单中
4. 没有任何插件注册 toolbar 按钮时，⊕ 按钮不显示

### 示例

回复可见插件（hide-until-reply）：

```json
{
  "toolbar": [
    {
      "id": "hide-until-reply",
      "label": "回复可见",
      "icon": "🔒",
      "insertBefore": "[hide-until-reply]\n",
      "insertAfter": "\n[/hide-until-reply]"
    }
  ]
}
```

媒体播放器插件（media-player）：

```json
{
  "toolbar": [
    {
      "id": "insert-video",
      "label": "插入视频",
      "icon": "🎬",
      "insertBefore": "[video src=\"",
      "insertAfter": "\" /]"
    },
    {
      "id": "insert-audio",
      "label": "插入音频",
      "icon": "🎵",
      "insertBefore": "[audio src=\"",
      "insertAfter": "\" /]"
    }
  ]
}
```

> 一个插件可以注册多个工具栏按钮。新建文章和编辑文章页面都支持插件工具栏。

---

## 前端插件开发（frontend.js）

### 基本结构

```javascript
(function() {
  const PLUGIN_ID = 'my-plugin';

  Noteva.ready(async function() {
    var settings = await Noteva.plugins.ready(PLUGIN_ID, {
      enabled: true
    });

    // 注意：ready() 返回已合并默认值和用户保存值的设置
    // 检查开关类设置时，应使用 === false 而非 !value
    if (settings.enabled === false) return;

    // 插件逻辑...

    console.log('[Plugin] my-plugin loaded');
  });
})();
```

### 前端钩子

| 钩子名 | 类型 | 触发时机 | 参数 |
|-------|------|---------|------|
| `article_view` | Action | 主题加载文章时 | `article` 完整文章对象 |
| `content_render` | Action | SPA 路由变化/DOM 变化时 | `{ path, query }` |
| `comment_after_create` | Action | 评论提交后 | `comment, { articleId }` |
| `body_end` | Action | 页面加载完成 | - |
| `route_change` | Action | SPA 路由变化时 | `{ from, to, query }` |
| `seo_meta_tags` | Filter | SEO 元标签生成时 | `{ title, description, keywords }` |
| `api_request_before` | Action | SDK API 请求前 | `{ method, url, data }` |
| `api_request_after` | Action | SDK API 请求后 | `{ method, url, response, result }` |
| `api_error` | Action | SDK API 请求失败时 | `error` |

```javascript
// 文章查看（最常用，可获取文章 ID）
Noteva.hooks.on('article_view', function(article) {
  // article: { id, slug, title, content, content_html, ... }
  console.log('Viewing article:', article.id, article.title);
});

// 内容渲染（SPA 导航时自动触发）
Noteva.hooks.on('content_render', function(context) {
  // context: { path, query }
});

// 页面加载完成
Noteva.ready(function() {
  // 初始化
});
```

### 路由匹配

```javascript
var match = Noteva.router.match('/posts/:slug');
if (match.matched) {
  var slug = match.params.slug;
  // 当前在文章页
}
```

### 插件数据 API（公开）

读取后端 WASM 插件存储的数据，无需登录：

```javascript
// SDK 方式
var value = await Noteva.plugins.storage.get('my-plugin', 'some-key');
```

### 插件 Action API（需管理员登录）

触发插件的自定义后端操作：

```javascript
var result = await Noteva.plugins.action('my-plugin', 'regenerate', {
  article_id: 123
});
// result: { success: true, data: { ... } }
```

> Action API 在 `/admin/` 路径下，受 `require_admin` + `require_auth` 中间件保护。

### 插件公开 API（WASM）

插件在 `plugin.json` 中声明 `"api": true` 后，可以通过 WASM `handle_request` 暴露公开 API：

```javascript
var result = await Noteva.plugins.request('my-plugin', 'verify', {
  method: 'POST',
  data: { token: 'xxx' }
});
```

`Noteva.plugins.api(...)` 是 `request(...)` 的短别名。JSON 响应会自动解析，非 JSON 响应返回文本。

### 插件注入点（PluginSlot）

主题中预留了多个注入点：

| 注入点 | 位置 |
|-------|------|
| `body_start` | 页面顶部 |
| `body_end` | 页面底部 |
| `article_content_top` | 文章内容顶部 |
| `article_content_bottom` | 文章内容底部 |
| `article_after_content` | 文章卡片下方 |
| `comment_form_before` | 评论表单上方 |
| `comment_form_after` | 评论表单下方 |

```javascript
// 方式一：SDK slots（静态 HTML）
Noteva.slots.register('article_content_top', '<div>内容</div>', 10);

// 方式二：DOM 操作（动态内容）
var slot = document.querySelector('[data-noteva-slot="article_content_top"]');
if (slot) slot.appendChild(myElement);
```

### 用户状态

```javascript
var user = await Noteva.user.check();
if (Noteva.user.isLoggedIn()) {
  var current = Noteva.user.getCurrent();
  // { id, username, email, avatar, role }
  if (current.role === 'admin') {
    // 管理员专属功能
  }
}
```

### UI 工具

```javascript
Noteva.ui.toast('操作成功', 'success');
Noteva.ui.toast('操作失败', 'error');
var confirmed = await Noteva.ui.confirm('确定要执行吗？');
```

### 暗色模式适配

主题使用 class-based 暗色模式（`<html class="dark">`），CSS 应使用 `.dark` 选择器：

```css
.my-plugin-box {
  background: #f0f9ff;
  color: #334155;
}

.dark .my-plugin-box {
  background: #0f172a;
  color: #cbd5e1;
}
```

> 不要使用 `@media (prefers-color-scheme: dark)`，它不会跟随主题切换。

---

## WASM 后端插件开发

WASM 后端插件在服务端沙盒中运行，通过子进程隔离（wasm-worker），插件崩溃不会影响主程序。

### 架构

```
主程序 (noteva)
  |
  +-- 钩子触发 -> 生成 JSON 输入（自动注入插件设置）
  |
  +-- 启动子进程 (wasm-worker)
  |     +-- 加载 backend.wasm
  |     +-- 注册宿主函数（网络、存储、日志、文章查询）
  |     +-- 调用 hook_xxx(ptr, len) -> result_ptr
  |     +-- 返回 JSON 结果 + 存储操作列表
  |
  +-- 执行存储操作 -> 写入数据库
```

关键特性：
- 插件设置自动注入到钩子输入数据中，无需手动读取配置
- 存储操作在子进程结束后由主程序批量执行，保证数据一致性
- 每次钩子调用都是独立的子进程，无状态
- 插件崩溃（panic、内存越界等）只影响子进程，主程序不受影响

### 编译

推荐使用 `wasm32-wasip1` 目标（支持标准库）：

```bash
# 安装编译目标（一次性）
rustup target add wasm32-wasip1

# 编译
cargo build --release --target wasm32-wasip1

# 复制到插件目录
cp target/wasm32-wasip1/release/my_plugin.wasm plugins/my-plugin/backend.wasm
```

Cargo.toml：

```toml
[package]
name = "my-plugin"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[profile.release]
opt-level = "s"
lto = true
strip = true
```

> 不需要任何外部依赖。宿主函数通过 `extern "C"` 声明，JSON 手动解析即可。

### 宿主函数

所有宿主函数在 `"env"` 模块下注册，WASM 插件通过 `extern "C"` 声明使用。

**所有宿主函数都必须声明**，即使不使用。WASM 模块在编译时声明所有导入，实例化时如果缺少任何一个都会失败。权限检查在函数体内部进行——无权限时返回 0 或空结果，不会导致 trap。

```rust
extern "C" {
    // 日志输出（无需权限）
    fn host_log(level_ptr: i32, level_len: i32, msg_ptr: i32, msg_len: i32);

    // HTTP 请求（需要 network 权限）
    fn host_http_request(
        method_ptr: i32, method_len: i32,
        url_ptr: i32, url_len: i32,
        headers_ptr: i32, headers_len: i32,
        body_ptr: i32, body_len: i32,
    ) -> i32;

    // 存储操作（需要 storage 权限，自动授予）
    fn host_storage_get(key_ptr: i32, key_len: i32) -> i32;
    fn host_storage_set(key_ptr: i32, key_len: i32, value_ptr: i32, value_len: i32) -> i32;
    fn host_storage_delete(key_ptr: i32, key_len: i32) -> i32;

    // 文章查询（需要 read_articles 权限）
    fn host_query_articles(filter_ptr: i32, filter_len: i32) -> i32;
    fn host_get_article(id_ptr: i32, id_len: i32) -> i32;

    // 评论查询（需要 read_comments 权限）
    fn host_get_comments(article_id_ptr: i32, article_id_len: i32) -> i32;

    // 文章元数据更新（需要 write_articles 权限）
    fn host_update_article_meta(article_id: i32, data_ptr: i32, data_len: i32) -> i32;

    // 数据库操作（需要 database 权限）
    fn host_db_query(sql_ptr: i32, sql_len: i32, params_ptr: i32, params_len: i32) -> i32;
    fn host_db_execute(sql_ptr: i32, sql_len: i32, params_ptr: i32, params_len: i32) -> i32;
}
```

#### host_log

输出日志到主服务器。level 支持 `"info"`, `"warn"`, `"error"`。

```rust
fn log(level: &str, msg: &str) {
    unsafe {
        host_log(
            level.as_ptr() as i32, level.len() as i32,
            msg.as_ptr() as i32, msg.len() as i32,
        );
    }
}

// 使用
log("info", "Processing article...");
```

日志输出格式：`[wasm:plugin-id][level] message`

#### host_http_request

发起 HTTP 请求。支持 GET/POST/PUT/DELETE/PATCH。

- 超时：10 秒
- 响应体限制：1 MB
- 返回值：指向响应 JSON 的内存指针（4 字节长度前缀 + JSON 字节）
- 返回 0 表示失败

响应 JSON 格式：`{"status": 200, "body": "响应内容"}`

```rust
fn http_post(url: &str, headers: &str, body: &[u8]) -> Option<String> {
    let method = "POST";
    let result_ptr = unsafe {
        host_http_request(
            method.as_ptr() as i32, method.len() as i32,
            url.as_ptr() as i32, url.len() as i32,
            headers.as_ptr() as i32, headers.len() as i32,
            body.as_ptr() as i32, body.len() as i32,
        )
    };
    if result_ptr <= 0 { return None; }
    read_result(result_ptr) // 读取 4 字节长度前缀 + JSON
}
```

#### host_storage_get / set / delete

插件数据存储，按插件 ID 自动隔离。

- `get`：返回 JSON `{"found": true, "value": "..."}` 或 `{"found": false, "value": ""}`
- `set`：返回 > 0 表示成功
- `delete`：返回 > 0 表示成功

```rust
fn storage_get(key: &str) -> Option<String> {
    let result_ptr = unsafe {
        host_storage_get(key.as_ptr() as i32, key.len() as i32)
    };
    if result_ptr <= 0 { return None; }
    let json = read_result(result_ptr)?;
    if !json.contains("\"found\":true") { return None; }
    extract_json_string(&json, "value")
}

fn storage_set(key: &str, value: &str) -> bool {
    unsafe {
        host_storage_set(
            key.as_ptr() as i32, key.len() as i32,
            value.as_ptr() as i32, value.len() as i32,
        ) > 0
    }
}

fn storage_delete(key: &str) -> bool {
    unsafe {
        host_storage_delete(key.as_ptr() as i32, key.len() as i32) > 0
    }
}
```

存储的数据可通过公开 API 读取：`GET /api/v1/plugins/{plugin_id}/data/{key}`

#### host_query_articles

查询所有已发布文章。返回 JSON 数组字符串。

```rust
fn query_articles() -> Option<String> {
    let filter = "{}";
    let result_ptr = unsafe {
        host_query_articles(filter.as_ptr() as i32, filter.len() as i32)
    };
    if result_ptr <= 0 { return None; }
    read_result(result_ptr)
}
// 返回: [{"id":1,"title":"...","slug":"...","content":"..."}, ...]
```

#### host_get_article

根据文章 ID 查询单篇文章。需要 `read_articles` 权限。返回单篇文章的 JSON 字符串，找不到时返回 `null`。

```rust
extern "C" {
    fn host_get_article(id_ptr: i32, id_len: i32) -> i32;
}

fn get_article(id: i64) -> Option<String> {
    let id_str = id.to_string();
    let result_ptr = unsafe {
        host_get_article(id_str.as_ptr() as i32, id_str.len() as i32)
    };
    if result_ptr <= 0 { return None; }
    read_result(result_ptr)
}
// 返回: {"id":1,"title":"...","slug":"...","content":"...","content_html":"..."}
// 找不到时返回: "null"
```

#### host_get_comments

查询指定文章的评论列表。需要 `read_comments` 权限。返回评论 JSON 数组字符串。

```rust
extern "C" {
    fn host_get_comments(article_id_ptr: i32, article_id_len: i32) -> i32;
}

fn get_comments(article_id: i64) -> Option<String> {
    let id_str = article_id.to_string();
    let result_ptr = unsafe {
        host_get_comments(id_str.as_ptr() as i32, id_str.len() as i32)
    };
    if result_ptr <= 0 { return None; }
    read_result(result_ptr)
}
// 返回: [{"id":1,"content":"...","author_name":"...","created_at":"..."}, ...]
```

> **权限说明**：`host_get_article` 需要在 `plugin.json` 中声明 `read_articles` 权限，`host_get_comments` 需要声明 `read_comments` 权限。

#### host_update_article_meta

更新文章的插件元数据。需要 `write_articles` 权限。数据会存储在文章的 `meta` JSON 字段中，按插件 ID 自动隔离命名空间。

```rust
extern "C" {
    fn host_update_article_meta(article_id: i32, data_ptr: i32, data_len: i32) -> i32;
}

fn update_article_meta(article_id: i64, data: &str) -> bool {
    unsafe {
        host_update_article_meta(
            article_id as i32,
            data.as_ptr() as i32,
            data.len() as i32,
        ) > 0
    }
}

// 示例：AI 摘要插件写入摘要
let summary_data = r#"{"summary":"这篇文章讲了...","generated_at":"2026-02-12"}"#;
update_article_meta(article_id, summary_data);
// 文章 meta 字段结果: {"ai-summary": {"summary":"这篇文章讲了...","generated_at":"2026-02-12"}}
```

> **命名空间隔离**：每个插件只能写入自己 `plugin_id` 对应的命名空间。例如 `ai-summary` 插件写入的数据会存储在 `meta["ai-summary"]` 下，不会影响其他插件的数据。文章 API 返回时会包含完整的 `meta` 字段。

#### host_db_query / host_db_execute

插件可以声明 `database: true` 来启用私有 SQL 表。该能力适合关系型数据、小型索引表、插件自己的业务记录；简单配置或少量键值仍优先用 `host_storage_get/set/delete`。

启用条件：

- `plugin.json` 必须同时声明 `"database": true` 和 `"permissions": ["database"]`。
- 必须提供 `backend.wasm`。
- 必须提供 `migrations/` 目录，且至少包含一个 `NNN_name.sql` 文件，例如 `001_init.sql`。
- 插件 ID 中的连字符会在 SQL 表名前缀里转成下划线：`ai-summary` 的表必须命名为 `plugin_ai_summary_*`。

迁移规则：

- 迁移文件按文件名排序执行，执行成功后记录到 `plugin_migrations`。
- 迁移和迁移记录在同一事务里执行；失败时不会记录为已执行。
- 迁移 SQL 只允许访问当前插件前缀的表或索引，不能创建、修改、引用核心表。
- 推荐只在 migrations 中写 `CREATE TABLE`、`CREATE INDEX` 和必要的初始化 `INSERT/UPDATE/DELETE`。

运行时规则：

- `host_db_query` 只接受单条 `SELECT`，返回 JSON 数组字符串指针；失败或无权限返回 `0`。
- `host_db_execute` 只接受单条 `INSERT`、`UPDATE`、`DELETE`，返回影响行数；执行错误返回 `-1`，无权限返回 `0`。
- 运行时 SQL 不允许 `CREATE/ALTER/DROP/TRUNCATE/PRAGMA/ATTACH/DETACH/VACUUM`，结构变更必须走 migrations。
- 所有 SQL 表引用都必须使用当前插件前缀，不能访问 `articles`、`comments`、`users` 等核心表。
- 参数使用 JSON 数组传入，例如 `[123, "title"]`；宿主会按 JSON 类型绑定到 SQL 参数。

示例：

```json
{
  "schema": 1,
  "id": "friendlinks",
  "permissions": ["storage", "database"],
  "database": true
}
```

```sql
-- plugins/friendlinks/migrations/001_init.sql
CREATE TABLE plugin_friendlinks_links (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  url TEXT NOT NULL,
  created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);
CREATE INDEX plugin_friendlinks_links_created_at_idx
  ON plugin_friendlinks_links(created_at);
```

```rust
extern "C" {
    fn host_db_query(sql_ptr: i32, sql_len: i32, params_ptr: i32, params_len: i32) -> i32;
    fn host_db_execute(sql_ptr: i32, sql_len: i32, params_ptr: i32, params_len: i32) -> i32;
}

let sql = "SELECT id, name, url FROM plugin_friendlinks_links ORDER BY id DESC";
let params = "[]";
let ptr = unsafe {
    host_db_query(
        sql.as_ptr() as i32,
        sql.len() as i32,
        params.as_ptr() as i32,
        params.len() as i32,
    )
};
```

### 自定义 API 路由

插件可以注册自定义的 HTTP API 端点。在 `plugin.json` 中声明 `"api": true`，并在 WASM 中导出 `handle_request` 函数。

#### plugin.json 配置

```json
{
  "id": "my-plugin",
  "api": true,
  ...
}
```

#### 路由格式

```
ANY /api/v1/plugins/{plugin_id}/api/{path}
```

例如：`GET /api/v1/plugins/ai-summary/api/status` 或 `POST /api/v1/plugins/my-plugin/api/webhook`

#### WASM 端实现

导出 `handle_request` 函数，接收请求 JSON，返回响应 JSON：

```rust
#[no_mangle]
pub extern "C" fn handle_request(ptr: i32, len: i32) -> i32 {
    // 输入 JSON: {"method":"GET","path":"status","body":""}
    let method = extract_json_string(input, "method").unwrap_or_default();
    let path = extract_json_string(input, "path").unwrap_or_default();
    let body = extract_json_string(input, "body").unwrap_or_default();

    // 处理请求...
    let response = match (method.as_str(), path.as_str()) {
        ("GET", "status") => r#"{"status":200,"body":"{\"ok\":true}"}"#,
        _ => r#"{"status":404,"body":"{\"error\":\"not found\"}"}"#,
    };

    write_response(response)
}
```

#### 响应格式

```json
{
  "status": 200,
  "content_type": "application/json",
  "body": "{\"ok\":true}"
}
```

- `status`：HTTP 状态码，默认 200
- `content_type`：响应类型，默认 `application/json`
- `body`：响应体字符串

> **注意**：自定义 API 路由是公开的，不需要认证。插件内部可通过 `host_storage` 实现简单的鉴权逻辑。每个请求会启动一个 wasm-worker 子进程执行，适合低频场景。

### 数据传递协议

主程序与 WASM 模块之间通过线性内存传递数据：

1. 主程序调用 `allocate(size)` 在 WASM 内存中分配缓冲区
2. 主程序将 JSON 输入写入缓冲区
3. 主程序调用 `hook_xxx(ptr, len)` 执行钩子
4. 钩子函数返回 result_ptr：
   - `0`：无输出（成功但无数据返回）
   - `> 0`：指向结果数据（前 4 字节为小端长度，后跟 JSON 字节）

必须导出的函数：

```rust
#[no_mangle]
pub extern "C" fn allocate(size: i32) -> i32 {
    if size <= 0 || size > 4 * 1024 * 1024 { return 0; }
    let layout = match Layout::from_size_align(size as usize, 1) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() { 0 } else { ptr as i32 }
}
```

读取宿主函数返回的结果：

```rust
fn read_result(ptr: i32) -> Option<String> {
    if ptr <= 0 { return None; }
    unsafe {
        let rp = ptr as usize;
        let len_bytes = slice::from_raw_parts(rp as *const u8, 4);
        let len = u32::from_le_bytes([len_bytes[0], len_bytes[1], len_bytes[2], len_bytes[3]]) as usize;
        if len == 0 || len > 2 * 1024 * 1024 { return None; }
        let data = slice::from_raw_parts((rp + 4) as *const u8, len);
        String::from_utf8(data.to_vec()).ok()
    }
}
```

写出钩子返回值：

```rust
fn write_output(json: &str) -> i32 {
    let bytes = json.as_bytes();
    let total = 4 + bytes.len();
    let layout = match Layout::from_size_align(total, 1) {
        Ok(l) => l,
        Err(_) => return 0,
    };
    let ptr = unsafe { alloc(layout) };
    if ptr.is_null() { return 0; }
    unsafe {
        let len_bytes = (bytes.len() as u32).to_le_bytes();
        std::ptr::copy_nonoverlapping(len_bytes.as_ptr(), ptr, 4);
        std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr.add(4), bytes.len());
    }
    ptr as i32
}
```

### 钩子输入数据

钩子函数接收的 JSON 输入包含两部分：
1. **钩子事件数据**：由触发点提供（如文章 ID、标题、内容等）
2. **插件设置**：自动注入，与后台设置界面中的值一致

例如 `article_after_create` 钩子收到的数据：

```json
{
  "id": 123,
  "title": "文章标题",
  "slug": "article-slug",
  "content": "文章内容...",
  "api_url": "https://api.example.com",
  "api_key": "sk-xxx",
  "model": "gpt-4o-mini",
  "max_count": 10
}
```

其中 `id`, `title`, `slug`, `content` 来自钩子事件，`api_url`, `api_key`, `model`, `max_count` 来自插件设置。

### 可用的后端钩子

> 钩子分为两种类型：**Filter**（过滤钩子）返回值会替换原始数据，链式传递给下一个处理函数；**Action**（动作钩子）返回值被忽略，仅用于副作用操作。

#### Article 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `article_before_create` | Filter | 文章创建前 | `{ title, slug, content, author_id, category_id, status }` | 5s |
| `article_after_create` | Action | 文章创建后 | `{ id, title, slug, content, author_id, category_id, status }` | 30s* |
| `article_before_update` | Filter | 文章更新前 | `{ id, title, content, slug, category_id, status }` | 5s |
| `article_after_update` | Action | 文章更新后 | `{ id, title, slug, content, author_id, category_id, status }` | 30s* |
| `article_before_delete` | Filter | 文章删除前 | `{ id, title }` | 5s |
| `article_after_delete` | Action | 文章删除后 | `{ id, success }` | 5s |
| `article_before_display` | Filter | 文章显示前 | `{ article, format }` | 5s |
| `article_view` | Action | 文章被查看时 | `{ id, slug, timestamp }` | 5s |
| `article_content_filter` | Filter | 文章内容过滤 | `{ content, article_id }` | 5s |
| `article_excerpt_filter` | Filter | 文章摘要过滤 | `{ excerpt, article_id }` | 5s |

#### Comment 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `comment_before_create` | Filter | 评论创建前 | `{ article_id, parent_id, content, nickname, email, user_id, ip, user_agent }` | 5s |
| `comment_after_create` | Action | 评论创建后 | `{ id, article_id, parent_id, content, nickname, email, user_id, status, ip, user_agent, created_at }` | 5s |
| `comment_before_delete` | Filter | 评论删除前 | `{ id }` | 5s |
| `comment_after_delete` | Action | 评论删除后 | `{ id, success }` | 5s |
| `comment_before_display` | Filter | 评论显示前 | `{ comments, count }` | 5s |
| `comment_content_filter` | Filter | 评论内容过滤 | `{ content, article_id, user_id, ip, user_agent, nickname, email }` | 5s |
| `comment_approve` | Action | 评论审核通过 | `{ id, approved }` | 5s |
| `comment_reject` | Action | 评论标记为垃圾 | `{ id, rejected }` | 5s |

#### Page 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `page_before_create` | Filter | 页面创建前 | `{ title, slug, content }` | 5s |
| `page_after_create` | Action | 页面创建后 | `{ id, title, slug }` | 5s |
| `page_before_update` | Filter | 页面更新前 | `{ id, title, slug, content }` | 5s |
| `page_after_update` | Action | 页面更新后 | `{ id, title, slug }` | 5s |
| `page_before_delete` | Filter | 页面删除前 | `{ id }` | 5s |
| `page_after_delete` | Action | 页面删除后 | `{ id, success }` | 5s |

#### Taxonomy 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `category_after_create` | Action | 分类创建后 | `{ id, name, slug }` | 5s |
| `category_after_delete` | Action | 分类删除后 | `{ id }` | 5s |
| `tag_after_create` | Action | 标签创建后 | `{ id, name, slug }` | 5s |
| `tag_after_delete` | Action | 标签删除后 | `{ id }` | 5s |

#### User 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `user_login_before` | Filter | 用户登录前 | `{ username_or_email, ip }` | 5s |
| `user_login_after` | Action | 用户登录后 | `{ user_id, username, session_id, ip, user_agent }` | 5s |
| `user_login_failed` | Action | 登录失败时 | `{ username_or_email, reason, user_id?, ip }` | 5s |
| `user_logout` | Action | 用户登出时 | `{ session_id, user_id }` | 5s |
| `user_register_before` | Filter | 用户注册前 | `{ username, email }` | 5s |
| `user_register_after` | Action | 用户注册后 | `{ id, username, email, role }` | 5s |
| `user_profile_update` | Action | 修改资料后 | `{ id, username, email, display_name, avatar, role }` | 5s |
| `user_password_change` | Action | 修改密码后 | `{ user_id }` | 5s |

#### Settings 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `settings_before_save` | Action | 设置保存前 | `{ keys }` | 5s |
| `settings_after_save` | Action | 设置保存后 | `{ site_name }` | 5s |

#### Content Processing 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `markdown_before_parse` | Filter | Markdown 解析前 | `{ content }` | 5s |
| `markdown_after_parse` | Action | Markdown 解析后 | `{ html }` | 5s |
| `excerpt_generate` | Filter | 摘要生成时 | `{ content, excerpt }` | 5s |

#### System 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `system_init` | Action | 系统初始化完成 | `{ version, timestamp }` | 5s |
| `cache_clear` | Action | 缓存清除时 | `{ scope, timestamp }` | 5s |
| `theme_switch` | Action | 主题切换时 | `{ old_theme, new_theme, timestamp }` | 5s |
| `api_request_before` | Filter | API 请求处理前 | `{ method, uri, path, ip, user_agent, timestamp }` | 5s |
| `api_request_after` | Action | API 请求处理后 | `{ method, uri, path, status, ip, user_agent, timestamp }` | 5s |

#### Plugin 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `plugin_activate` | Filter | 插件启用时 | `{ plugin_id, plugin_version, site_url, timestamp }` + 设置 | 300s |
| `plugin_deactivate` | Action | 插件禁用时 | `{ plugin_id }` | 5s |
| `plugin_destroy` | Action | 插件被禁用时（WASM 卸载前） | `{ plugin_id }` | 5s |
| `plugin_upgrade` | Action | 插件版本变更时 | `{ plugin_id, old_version, new_version }` | 5s |
| `plugin_action` | Action | 自定义操作 | `{ plugin_id, action, data }` + 设置 | 30s* |
| `theme_activate` | Filter | 主题切换时 | `{ theme_name, theme_version, site_url, timestamp }` + 设置 | 300s |

> *带 `network` 权限的插件自动获得 30s 超时；`plugin_activate` 固定 300s（用于批量处理）。

#### Upload 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `image_upload_filter` | Filter | 图片上传时 | `{ filename, original_filename, content_type, size, data_base64, timestamp }` | 30s* |
| `file_upload_filter` | Filter | 通用文件上传时 | `{ filename, original_filename, content_type, size, timestamp }` | 30s* |

> **Presign 委托上传（v0.1.6-beta 新增）**
>
> 上传钩子支持两种响应模式：
>
> 1. **Legacy 直传**：插件在 WASM 内完成上传，返回 `{"handled": true, "url": "https://..."}`。适合小文件（图片），但受 WASM 16MB 内存和 base64 膨胀限制。
> 2. **Presign 委托**：插件只生成签名 URL，返回 `{"handled": true, "presign": {"url": "https://s3.../put", "method": "PUT", "headers": {...}, "public_url": "https://cdn.../file"}}`。主进程用原生 Rust HTTP 客户端流式上传，不受 WASM 内存限制，支持任意大小文件。
>
> `file_upload_filter` 不传 `data_base64`（文件数据不进入 WASM），只能使用 presign 模式。`image_upload_filter` 两种模式都支持，向后兼容。
>
> Presign 响应字段：
> - `url`（必需）：上传目标 URL
> - `method`（可选，默认 `PUT`）：HTTP 方法
> - `headers`（可选）：请求头（Authorization、x-amz-date 等）
> - `public_url`（可选）：文件的公开访问 URL，不同于上传 URL 时使用（如 CDN 域名）
>
> 参考实现：`plugins/s3-image-upload/`（同时支持 legacy 和 presign 两种模式）。

#### Cron 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `cron_register` | Action | 系统启动时 | `{ interval_seconds, timestamp }` | 5s |
| `cron_tick` | Action | 每 60 秒 | `{ timestamp }` | 5s |

> 插件通过 `cron_register` 了解系统的 tick 间隔（当前固定 60 秒）。在 `cron_tick` 中执行定期操作（如备份、清理、同步）。插件可用 `host_storage` 记录上次执行时间来实现更长间隔。

#### SEO / Article List 钩子

| 钩子名 | 类型 | 触发时机 | 事件数据 | 超时 |
|-------|------|---------|---------|------|
| `feed_filter` | Filter | RSS XML 输出前 | `{ xml }` | 5s |
| `sitemap_filter` | Filter | Sitemap XML 输出前 | `{ xml }` | 5s |
| `article_list_filter` | Action | 文章列表 API 返回前 | `{ count, page, per_page }` | 5s |

#### 钩子校验

启用插件时，系统会自动校验 `plugin.json` 中声明的钩子：
- 检查钩子名称是否存在于注册表中
- 检查钩子的 scope 是否匹配（backend 钩子不能声明在 `hooks.frontend` 中，反之亦然）
- 校验不通过时记录警告日志，但不阻止插件加载（向前兼容）

### 系统钩子详解

#### plugin_activate

插件启用时触发（Filter 类型）。适合两种场景：

1. **批量处理存量数据**：如 AI 摘要插件启用时为所有旧文章生成摘要
2. **授权验证**：商业插件在 WASM 中验证授权码，返回 `{allow: false}` 阻止启用

输入数据包含站点信息，方便授权验证：

```json
{
  "plugin_id": "my-plugin",
  "plugin_version": "1.0.0",
  "site_url": "https://example.com",
  "timestamp": "2026-03-01T00:00:00Z",
  "license_key": "xxx"
}
```

> `site_url` 由平台自动注入，`license_key` 等来自插件设置（自动注入）。

**批量处理示例**：

```rust
#[no_mangle]
pub extern "C" fn hook_plugin_activate(ptr: i32, len: i32) -> i32 {
    let plugin_id = extract_json_string(input, "plugin_id").unwrap_or_default();
    if plugin_id != "my-plugin" { return 0; }

    let articles = query_articles().unwrap_or_default();
    // 批量处理...
    0
}
```

**授权验证示例**：

```rust
#[no_mangle]
pub extern "C" fn hook_plugin_activate(ptr: i32, len: i32) -> i32 {
    // ... parse input ...
    let plugin_id = extract_json_string(input, "plugin_id").unwrap_or_default();
    if plugin_id != "my-plugin" { return 0; }

    let license_key = extract_json_string(input, "license_key").unwrap_or_default();
    let site_url = extract_json_string(input, "site_url").unwrap_or_default();

    if license_key.is_empty() {
        return write_output(r#"{"allow":false,"message":"请输入授权码"}"#);
    }

    // Call remote verification API
    let verify_body = format!(
        r#"{{"license":"{}","site_url":"{}"}}"#,
        escape_json_string(&license_key),
        escape_json_string(&site_url),
    );
    let resp = http_post(
        "https://your-api.com/verify",
        "Content-Type: application/json",
        verify_body.as_bytes(),
    );

    match resp {
        Some(body) if body.contains("\"valid\":true") => {
            // Store activation info for later use
            storage_set("activated", "true");
            storage_set("activated_at", &extract_json_string(input, "timestamp").unwrap_or_default());
            0 // allow (default)
        }
        _ => {
            write_output(r#"{"allow":false,"message":"授权验证失败"}"#)
        }
    }
}
```

**定时重验**：在 `plugin.json` 中声明 `activate.interval_hours`，平台会定期重新触发 `plugin_activate`，验证失败自动禁用：

```json
{
  "id": "my-plugin",
  "activate": {
    "on_start": true,
    "interval_hours": 24
  },
  "hooks": {
    "backend": ["plugin_activate"]
  }
}
```

#### theme_activate

主题切换时触发（Filter 类型）。主题本身没有 WASM，授权验证由伴生插件完成。

伴生插件监听 `theme_activate` 钩子，验证主题授权：

```rust
#[no_mangle]
pub extern "C" fn hook_theme_activate(ptr: i32, len: i32) -> i32 {
    // ... parse input ...
    let theme_name = extract_json_string(input, "theme_name").unwrap_or_default();
    if theme_name != "my-premium-theme" { return 0; } // Only handle our theme

    let license_key = extract_json_string(input, "license_key").unwrap_or_default();
    if license_key.is_empty() {
        return write_output(r#"{"allow":false,"message":"请安装授权插件并输入授权码"}"#);
    }

    // Verify license...
    0 // allow
}
```

伴生插件的 `plugin.json`：

```json
{
  "id": "my-premium-theme-license",
  "name": "My Premium Theme License",
  "hooks": {
    "backend": ["theme_activate"]
  },
  "permissions": ["network", "storage"],
  "settings": true
}
```

#### plugin_action

通用的自定义操作入口。前端通过 `POST /admin/plugins/:id/action/:action` 触发。

```rust
#[no_mangle]
pub extern "C" fn hook_plugin_action(ptr: i32, len: i32) -> i32 {
    // 解析输入...
    let plugin_id = extract_json_string(input, "plugin_id").unwrap_or_default();
    if plugin_id != "my-plugin" { return 0; }

    let action = extract_json_string(input, "action").unwrap_or_default();
    match action.as_str() {
        "regenerate" => handle_regenerate(input),
        "cleanup" => handle_cleanup(input),
        _ => 0,
    }
}
```

#### article_after_delete

文章删除后触发，适合清理关联数据：

```rust
#[no_mangle]
pub extern "C" fn hook_article_after_delete(ptr: i32, len: i32) -> i32 {
    // 解析输入获取 article id...
    let article_id = extract_json_number(input, "id").unwrap_or(0);
    // 清理存储的数据
    storage_delete(&format!("data:{}", article_id));
    0
}
```

#### plugin_destroy

插件被禁用时触发（在 WASM 模块卸载之前），适合清理资源。即使钩子执行失败，插件禁用流程也会继续完成。

```rust
#[no_mangle]
pub extern "C" fn hook_plugin_destroy(ptr: i32, len: i32) -> i32 {
    // 解析输入...
    let plugin_id = extract_json_string(input, "plugin_id").unwrap_or_default();
    if plugin_id != "my-plugin" { return 0; }

    // 清理临时数据、关闭连接等
    storage_delete("temp_cache");
    log("info", "Plugin resources cleaned up");
    0
}
```

#### plugin_upgrade

插件版本变更时触发（启用插件时检测到 plugin.json 版本与上次记录不同），适合数据迁移。即使钩子执行失败，插件启用流程也会继续完成。

```rust
#[no_mangle]
pub extern "C" fn hook_plugin_upgrade(ptr: i32, len: i32) -> i32 {
    // 解析输入...
    let plugin_id = extract_json_string(input, "plugin_id").unwrap_or_default();
    if plugin_id != "my-plugin" { return 0; }

    let old_version = extract_json_string(input, "old_version").unwrap_or_default();
    let new_version = extract_json_string(input, "new_version").unwrap_or_default();
    log("info", &format!("Upgrading from {} to {}", old_version, new_version));

    // 执行数据迁移逻辑...
    0
}
```

### 沙盒安全机制

| 限制项 | 默认值 | 说明 |
|-------|-------|------|
| 内存限制 | 16 MB | 防止内存泄漏 |
| 指令限制 | 1 亿条（fuel） | 防止死循环 |
| 执行超时 | 5s / 30s / 300s | 根据钩子类型自动调整 |
| 权限控制 | 白名单 | 只能使用 plugin.json 中声明的权限 |
| 进程隔离 | 子进程 | 每次调用独立子进程，崩溃不影响主程序 |
| HTTP 超时 | 10s | 单次 HTTP 请求超时 |
| HTTP 响应限制 | 1 MB | 响应体大小限制 |

### JSON 解析工具

由于 WASM 插件不引入外部依赖，需要手动解析 JSON。以下是常用的工具函数：

```rust
/// 从 JSON 中提取字符串值（支持 UTF-8 和 \uXXXX 转义）
fn extract_json_string(json: &str, key: &str) -> Option<String> {
    let pattern = format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    if !rest.starts_with('"') { return None; }
    let rest = &rest[1..];

    let mut result = Vec::new();
    let bytes = rest.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'"' { break; }
        if bytes[i] == b'\\' && i + 1 < bytes.len() {
            i += 1;
            match bytes[i] {
                b'n' => result.push(b'\n'),
                b't' => result.push(b'\t'),
                b'r' => result.push(b'\r'),
                b'"' => result.push(b'"'),
                b'\\' => result.push(b'\\'),
                b'/' => result.push(b'/'),
                b'u' if i + 4 < bytes.len() => {
                    // \uXXXX unicode 转义处理
                    // ... 完整实现见 ai-summary 插件源码
                    i += 4;
                    continue;
                }
                other => { result.push(b'\\'); result.push(other); }
            }
        } else {
            result.push(bytes[i]);
        }
        i += 1;
    }
    String::from_utf8(result).ok()
}

/// 从 JSON 中提取数字值
fn extract_json_number(json: &str, key: &str) -> Option<i64> {
    let pattern = format!("\"{}\":", key);
    let start = json.find(&pattern)? + pattern.len();
    let rest = json[start..].trim_start();
    let end = rest.find(|c: char| !c.is_ascii_digit() && c != '-')?;
    rest[..end].parse().ok()
}

/// JSON 字符串转义
fn escape_json_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out
}
```

> 完整的 JSON 工具函数实现可参考 `plugins/ai-summary/wasm-src/src/lib.rs`。

### UTF-8 注意事项

处理中文等多字节字符时需要注意：
- 字符串截断必须在字符边界：使用 `is_char_boundary()` 检查
- 不要用 `b as char` 转换字节为字符，应收集字节后用 `String::from_utf8()`
- JSON 中的 `\uXXXX` 转义需要处理 surrogate pair（`\uD800-\uDFFF`）

```rust
// 安全截断 UTF-8 字符串
let truncated = if content.len() > max_len {
    let mut end = max_len;
    while end > 0 && !content.is_char_boundary(end) {
        end -= 1;
    }
    &content[..end]
} else {
    content
};
```

---

## 国际化（i18n）

插件支持通过 `locales/` 目录提供多语言翻译，框架会自动加载并注入到 SDK 的 i18n 系统中。

### 目录结构

```
plugins/my-plugin/
  locales/
    zh-CN.json    # 简体中文
    en.json       # 英文
    ja.json       # 日文（按需添加）
```

### 翻译文件格式

纯 JSON，键值对结构：

```json
{
  "greeting": "你好",
  "submitBtn": "提交",
  "error": "操作失败"
}
```

### 使用方式

框架会自动将翻译注册到 `Noteva.i18n`，命名空间为插件 ID。在 `frontend.js` 中直接调用：

```javascript
// 插件 ID 为 "my-plugin"
Noteva.i18n.t('my-plugin.greeting')   // → "你好"（当前语言为 zh-CN 时）
Noteva.i18n.t('my-plugin.submitBtn')  // → "Submit"（当前语言为 en 时）
```

无需手动调用 `addMessages`，无需编译，放好 JSON 文件即可。

---

## 后端钩子参考表

> WASM 插件可通过 `plugin.json` 的 `hooks.backend` 字段声明要监听的后端钩子。

### 文章生命周期

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `article_before_create` | Filter | 文章创建前，可修改数据 | 0.1.3 |
| `article_after_create` | Action | 文章创建成功后 | 0.1.3 |
| `article_before_update` | Filter | 文章更新前，可修改数据 | 0.1.3 |
| `article_after_update` | Action | 文章更新成功后 | 0.1.3 |
| `article_before_delete` | Filter | 文章删除前 | 0.1.3 |
| `article_after_delete` | Action | 文章删除成功后 | 0.1.3 |
| `article_before_display` | Filter | 文章显示前，可修改显示数据 | 0.1.3 |
| `article_view` | Action | 文章被查看时 | 0.1.3 |
| `article_content_filter` | Filter | 文章内容过滤 | 0.1.3 |
| `article_excerpt_filter` | Filter | 文章摘要过滤 | 0.1.3 |
| `article_status_change` | Action | 文章状态变化（草稿→发布等） | 0.1.8 |
| `article_list_filter` | Filter | 文章列表返回前，可修改结果 | 0.1.8 |

### 页面生命周期

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `page_before_create` | Filter | 页面创建前 | 0.1.8 |
| `page_after_create` | Action | 页面创建成功后 | 0.1.8 |
| `page_before_update` | Filter | 页面更新前 | 0.1.8 |
| `page_after_update` | Action | 页面更新成功后 | 0.1.8 |
| `page_before_delete` | Filter | 页面删除前 | 0.1.8 |
| `page_after_delete` | Action | 页面删除成功后 | 0.1.8 |

### 评论

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `comment_before_create` | Filter | 评论创建前，可修改数据 | 0.1.3 |
| `comment_after_create` | Action | 评论创建成功后 | 0.1.3 |
| `comment_before_delete` | Filter | 评论删除前 | 0.1.3 |
| `comment_after_delete` | Action | 评论删除成功后 | 0.1.3 |
| `comment_before_display` | Filter | 评论显示前 | 0.1.3 |
| `comment_content_filter` | Filter | 评论内容过滤 | 0.1.3 |
| `comment_approve` | Action | 评论审核通过时 | 0.1.8 |
| `comment_reject` | Action | 评论审核拒绝时 | 0.1.8 |

### 分类 & 标签

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `category_after_create` | Action | 分类创建成功后 | 0.1.8 |
| `category_after_delete` | Action | 分类删除成功后 | 0.1.8 |
| `tag_after_create` | Action | 标签创建成功后 | 0.1.8 |
| `tag_after_delete` | Action | 标签删除成功后 | 0.1.8 |

### 用户

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `user_login_before` | Filter | 用户登录前 | 0.1.3 |
| `user_login_after` | Action | 用户登录成功后 | 0.1.3 |
| `user_login_failed` | Action | 用户登录失败时 | 0.1.3 |
| `user_logout` | Action | 用户登出时 | 0.1.3 |
| `user_register_before` | Filter | 用户注册前 | 0.1.3 |
| `user_register_after` | Action | 用户注册成功后 | 0.1.3 |
| `user_profile_update` | Action | 修改个人资料时 | 0.1.8 |
| `user_password_change` | Action | 修改密码时 | 0.1.8 |

### 内容处理

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `markdown_before_parse` | Filter | Markdown 解析前 | 0.1.3 |
| `markdown_after_parse` | Action | Markdown 解析后 | 0.1.3 |
| `excerpt_generate` | Filter | 摘要生成时 | 0.1.3 |

### 系统 & 插件

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `system_init` | Action | 系统初始化完成后 | 0.1.3 |
| `cache_clear` | Action | 缓存清除时 | 0.1.3 |
| `theme_switch` | Action | 主题切换时 | 0.1.3 |
| `theme_activate` | Filter | 主题启用时（可阻止） | 0.1.6 |
| `plugin_activate` | Filter | 插件启用时（可阻止） | 0.1.6 |
| `plugin_deactivate` | Action | 插件禁用时 | 0.1.3 |
| `plugin_destroy` | Action | 插件被卸载时 | 0.1.5 |
| `plugin_upgrade` | Action | 插件版本变更时 | 0.1.5 |
| `plugin_action` | Action | 插件自定义动作 | 0.1.3 |

### SEO & 数据

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `feed_filter` | Filter | RSS Feed XML 输出前 | 0.1.8 |
| `sitemap_filter` | Filter | Sitemap XML 输出前 | 0.1.8 |
| `nav_items_filter` | Filter | 导航菜单渲染时 | 0.1.3 |

### 设置

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `settings_before_save` | Filter | 设置保存前，可修改或拦截 | 0.1.8 |
| `settings_after_save` | Action | 设置保存成功后 | 0.1.8 |

### 定时任务

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `cron_register` | Action | 系统启动时，通知任务间隔（60s） | 0.1.8 |
| `cron_tick` | Action | 每 60 秒触发一次 | 0.1.8 |

### 上传

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `image_upload_filter` | Filter | 图片上传时，可拦截并返回远程 URL | 0.1.4 |
| `file_upload_filter` | Filter | 文件上传时，可通过 presign 委托上传 | 0.1.6 |

### API 请求

| 钩子名 | 类型 | 触发时机 | 版本 |
|-------|------|---------|------|
| `api_request_before` | Filter | API 请求处理前 | 0.1.3 |
| `api_request_after` | Action | API 请求处理后 | 0.1.3 |

---

## API 参考

### 公开 API（无需登录）

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/v1/plugins/assets/plugins.js` | 所有启用插件的合并 JS |
| GET | `/api/v1/plugins/assets/plugins.css` | 所有启用插件的合并 CSS |
| GET | `/api/v1/plugins/enabled` | 启用的插件列表及设置（secret 字段已过滤） |
| GET | `/api/v1/plugins/:id/data/:key` | 读取插件存储数据 |

### 管理 API（需要管理员登录）

| 方法 | 路径 | 说明 |
|-----|------|------|
| GET | `/api/v1/admin/plugins` | 插件列表 |
| GET | `/api/v1/admin/plugins/:id` | 插件详情 |
| POST | `/api/v1/admin/plugins/:id/toggle` | 启用/禁用插件 |
| GET | `/api/v1/admin/plugins/:id/settings` | 获取插件设置 |
| POST | `/api/v1/admin/plugins/:id/settings` | 更新插件设置 |
| GET | `/api/v1/admin/plugins/:id/data/:key` | 读取插件数据 |
| PUT | `/api/v1/admin/plugins/:id/data/:key` | 写入插件数据 |
| DELETE | `/api/v1/admin/plugins/:id/data/:key` | 删除插件数据 |
| POST | `/api/v1/admin/plugins/:id/action/:action` | 触发插件自定义操作 |
| GET | `/api/v1/admin/plugins/wasm/status` | WASM 运行时状态 |

---

## 调试技巧

1. 浏览器控制台查看 `[Plugin] xxx loaded` 确认前端加载
2. 服务端日志查看 `[wasm:plugin-id]` 前缀的 WASM 插件日志
3. 使用 `host_log("info", "debug message")` 在 WASM 中输出调试信息
4. 检查 `/api/v1/plugins/enabled` 确认插件设置是否正确加载
5. 检查 `/api/v1/plugins/:id/data/:key` 确认存储数据是否正确写入
6. 编译后确保 `backend.wasm` 已复制到插件目录
7. debug 和 release 模式的 `wasm-worker` 是独立的，修改后两个都需要重新编译

### 常见问题

**Q: WASM 插件加载失败，报 `unknown import: wasi_snapshot_preview1::fd_write`**
A: 使用 `wasm32-wasip1` 编译的模块需要 WASI 支持。确保 Noteva 版本 >= 0.1.3-beta，该版本已内置 WASI stubs。

**Q: 插件设置在前端获取为空对象**
A: 用户未在后台保存过设置时，`getSettings()` 返回 `{}`。检查开关类设置应使用 `=== false` 而非 `!value`。

**Q: 存储数据 API 返回 HTML 而非 JSON**
A: 确认请求路径正确（`/api/v1/plugins/:id/data/:key`），检查 Content-Type 响应头。

**Q: cargo build 后 wasm-worker 没更新**
A: cargo 增量编译可能跳过链接。删除旧的 exe 文件后重新编译，或 touch 源文件强制重编译。
