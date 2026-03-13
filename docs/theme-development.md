# Noteva 主题开发文档

> 版本：v0.2.1

## 快速开始

Noteva 支持任意前端框架开发主题（React、Vue、原生 JS 等），只需遵循简单约定。

### 主题目录结构

```
themes/
└── my-theme/
    ├── theme.json          # 主题配置（必需）
    ├── settings.json       # 主题设置定义（可选）
    ├── dist/               # 构建输出（必需）
    │   └── index.html      # 入口文件
    └── preview.png         # 预览图（推荐）
```

### theme.json 示例

```json
{
  "name": "我的主题",
  "short": "my-theme",
  "description": "一个简洁的博客主题",
  "version": "1.0.0",
  "author": "Your Name",
  "pages": [
    { "slug": "about", "title": "关于" }
  ]
}
```

`pages` 字段声明主题需要的页面。切换到该主题时，后端自动创建缺失的 Page 记录（status=published, content 为空）。已存在的同 slug 页面不会被覆盖。用户可在管理后台编辑这些页面的内容和 SEO 信息。

## Noteva SDK

Noteva 会自动在主题页面注入 SDK，提供 `window.Noteva` 全局对象。

**重要：不要手动引入 SDK，后端会自动注入！**

### 初始化

```javascript
// 等待 SDK 就绪
Noteva.ready(() => {
  // SDK 已加载完成，可以使用所有 API
  init();
});

// 或使用 Promise
await Noteva.ready();
```

## 站点 API

### 获取站点信息

```javascript
const site = await Noteva.site.getInfo();
// 返回:
// {
//   version: "0.1.9-beta",
//   name: "站点名称",
//   description: "站点描述",
//   subtitle: "副标题",
//   logo: "/uploads/logo.png",
//   footer: "页脚内容",
//   email_verification_enabled: "false",
//   permalink_structure: "/posts/{slug}",
//   demo_mode: false,
//   stats: {
//     total_articles: 42,
//     total_categories: 5,
//     total_tags: 18
//   },
//   font_family: "Poppins",          // v0.1.9 新增
//   custom_css: "...",               // v0.1.9 新增（全字段透传）
//   custom_js: "...",                // v0.1.9 新增
// }
```

### 生成文章 URL（重要）

**务必使用此方法生成文章链接，不要硬编码 `/posts/${slug}`。**

文章 URL 格式取决于后台设置的 `permalink_structure`（`/posts/{slug}` 或 `/posts/{id}`），SDK 会自动处理：

```javascript
// ✅ 正确做法：始终使用 getArticleUrl
const url = Noteva.site.getArticleUrl(article);
// slug 模式 → "/posts/hello-world"
// id 模式   → "/posts/42"

// ❌ 错误做法：硬编码 slug
const url = `/posts/${article.slug}`;  // id 模式下会生成错误的 URL！
```

> **React/Vue 主题**：SDK 类型定义为 `getArticleUrl(article: { id: number | string; slug?: string }): string`。
> 如果 SDK 尚未加载（fallback 场景），内置主题会自动从 `window.__SITE_CONFIG__` 读取 permalink 设置。

### 获取导航菜单

```javascript
const nav = await Noteva.site.getNav();
// 返回:
// [
//   { id: 1, name: "首页", url: "/", target: "_self", children: [] },
//   { id: 2, name: "关于", url: "/about", target: "_self", children: [
//     { id: 3, name: "子菜单", url: "/sub", target: "_self" }
//   ]}
// ]
```

### 获取主题设置

主题可以通过 `settings.json` 定义自定义设置项，管理员在后台配置，主题通过 SDK 读取。

```javascript
// 获取所有设置
const settings = await Noteva.site.getThemeSettings();
// 返回: { "primary_color": "#3b82f6", "show_sidebar": "true", ... }

// 获取单个设置
const color = await Noteva.site.getThemeSettings('primary_color');
```

#### settings.json 格式

在主题根目录创建 `settings.json`，定义可配置项。后台会自动按 `sections` 分组显示，每个分组可折叠。

> v0.1.6-beta 起，主题和插件的设置界面统一使用侧边抽屉（Sheet）+ 手风琴折叠（Accordion）展示，每个 section 独立折叠，默认展开第一个。设置渲染逻辑由共享组件 `SettingsRenderer` 统一处理，主题和插件体验一致。

```json
{
  "sections": [
    {
      "id": "appearance",
      "title": "外观设置",
      "fields": [
        {
          "id": "primary_color",
          "label": "主题色",
          "type": "text",
          "default": "#3b82f6",
          "description": "主题的主要颜色"
        },
        {
          "id": "show_sidebar",
          "label": "显示侧边栏",
          "type": "switch",
          "default": true
        }
      ]
    }
  ]
}
```

#### 支持的字段类型

##### text — 单行文本

```json
{
  "id": "site_title",
  "type": "text",
  "label": "站点标题",
  "default": "",
  "description": "显示在页面顶部的标题",
  "placeholder": "输入标题"
}
```

标记 `"secret": true` 的字段会以密码框显示，且不会通过公开 API 返回（仅管理后台可见），适合存储 API Key 等敏感信息。

##### textarea — 多行文本

```json
{
  "id": "custom_css",
  "type": "textarea",
  "label": "自定义 CSS",
  "default": "",
  "description": "注入自定义样式"
}
```

##### switch — 开关

值为布尔类型。注意：数据库存储为字符串 `"true"` / `"false"`，读取时需要解析。

```json
{
  "id": "show_toc",
  "type": "switch",
  "label": "显示目录",
  "default": true,
  "description": "文章页是否显示侧边目录"
}
```

读取示例：
```javascript
const settings = await Noteva.site.getThemeSettings();
// settings.show_toc 可能是 true / false / "true" / "false"
const showToc = settings.show_toc === true || settings.show_toc === "true";
```

##### select — 下拉选择

`options` 必须是 `{ value, label }` 对象数组，不能是纯字符串。

```json
{
  "id": "layout_mode",
  "type": "select",
  "label": "布局模式",
  "default": "grid",
  "options": [
    { "value": "grid", "label": "网格布局" },
    { "value": "list", "label": "列表布局" },
    { "value": "timeline", "label": "时间线" }
  ]
}
```

##### number — 数字

支持 `min` / `max` 限制范围。

```json
{
  "id": "page_size",
  "type": "number",
  "label": "每页文章数",
  "default": 10,
  "min": 1,
  "max": 50
}
```

##### array — 可视化数组编辑器

用于管理结构化列表数据（如歌单、友链等）。后台会渲染可视化编辑器，支持添加/删除/排序。

`itemFields` 定义每条数据的字段结构，支持 `text` 和 `number` 类型。

```json
{
  "id": "friends_list",
  "type": "array",
  "label": "友链列表",
  "default": [],
  "description": "添加友情链接",
  "itemFields": [
    { "id": "name", "type": "text", "label": "名称" },
    { "id": "url", "type": "text", "label": "链接" },
    { "id": "avatar", "type": "text", "label": "头像" },
    { "id": "desc", "type": "text", "label": "描述" }
  ]
}
```

数据库中存储为 JSON 字符串，读取时需要解析：

```javascript
const settings = await Noteva.site.getThemeSettings();
let list = settings.friends_list;
// 可能是数组（已解析）或 JSON 字符串
if (typeof list === "string") {
  try { list = JSON.parse(list); } catch { list = []; }
}
// list: [{ name: "...", url: "...", avatar: "...", desc: "..." }, ...]
```

#### 完整 settings.json 示例

```json
{
  "sections": [
    {
      "id": "general",
      "title": "基本设置",
      "fields": [
        { "id": "primary_color", "type": "text", "label": "主题色", "default": "#3b82f6" },
        { "id": "layout_mode", "type": "select", "label": "布局", "default": "grid",
          "options": [
            { "value": "grid", "label": "网格" },
            { "value": "list", "label": "列表" }
          ]
        },
        { "id": "show_sidebar", "type": "switch", "label": "显示侧边栏", "default": true },
        { "id": "page_size", "type": "number", "label": "每页文章数", "default": 10, "min": 1, "max": 50 }
      ]
    },
    {
      "id": "music",
      "title": "音乐播放器",
      "fields": [
        { "id": "music_enabled", "type": "switch", "label": "启用播放器", "default": false },
        {
          "id": "music_playlist", "type": "array", "label": "歌单", "default": [],
          "itemFields": [
            { "id": "name", "type": "text", "label": "歌名" },
            { "id": "artist", "type": "text", "label": "歌手" },
            { "id": "url", "type": "text", "label": "音频地址" },
            { "id": "cover", "type": "text", "label": "封面地址" }
          ]
        }
      ]
    }
  ]
}
```

#### 数据类型注意事项

所有设置值在数据库中均以字符串形式存储。读取时：
- `switch` 类型：值可能是 `true`（布尔）或 `"true"`（字符串），建议用 `val === true || val === "true"` 判断
- `array` 类型：值可能是数组（已解析）或 JSON 字符串，建议先检查 `Array.isArray()`，否则 `JSON.parse()`
- `number` 类型：值可能是数字或字符串，建议用 `Number()` 转换

## 文章 API

### 获取文章列表

```javascript
const { articles, total, page, pageSize, hasMore } = await Noteva.articles.list({
  page: 1,
  pageSize: 10,
  category: 'tech',     // 可选：按分类
  tag: 'javascript',    // 可选：按标签
  keyword: '搜索词',     // 可选：搜索
  sort: 'latest',       // 可选：排序方式 - latest(默认), views, comments
});
```

### 获取文章归档

```javascript
const archives = await Noteva.articles.archives();
// 返回: [{ month: "2026-03", count: 5 }, { month: "2026-02", count: 8 }, ...]
```

### 获取单篇文章

```javascript
const article = await Noteva.articles.get('hello-world');  // 通过 slug
// 返回:
// {
//   id: 1,
//   title: "文章标题",
//   slug: "hello-world",
//   content: "Markdown 原文",
//   html: "渲染后的 HTML",
//   excerpt: "摘要",
//   cover_image: "封面图",
//   author: { id, username, avatar },
//   category: { id, name, slug },
//   tags: [{ id, name }],
//   created_at: "2026-01-01T00:00:00Z",
//   view_count: 100,
//   like_count: 10,
//   comment_count: 5,
//   is_pinned: false,
//   word_count: 1200,                     // v0.1.8 新增
//   reading_time: 4,                      // v0.1.8 新增（分钟）
//   scheduled_at: "2026-03-10T08:00:00Z", // v0.1.8 新增，仅草稿
//   prev: { id, title, slug },            // v0.1.8 新增
//   next: { id, title, slug },            // v0.1.8 新增
//   related: [{ id, title, slug }]        // v0.1.8 新增（同分类）
//   toc: [{ id, text, level }],           // v0.2.0 新增（目录结构）
// }
```

### 文章字段兼容工具（v0.1.9 新增）

主题开发者不用关心 `snake_case` vs `camelCase` 字段名差异：

```javascript
// 获取发布日期（兼容 published_at / publishedAt / created_at / createdAt）
const date = Noteva.articles.getDate(article);  // "2026-01-01T00:00:00Z"

// 获取统计数据
const stats = Noteva.articles.getStats(article);
// { views: 100, likes: 10, comments: 5 }

// 判断是否置顶
const pinned = Noteva.articles.isPinned(article);  // true/false

// 获取缩略图（优先 thumbnail > cover_image > 正文第一张图）
const thumb = Noteva.articles.getThumbnail(article);  // URL 或 null

// 生成纯文本摘要（优先后端 excerpt > 正文截断）
const excerpt = Noteva.articles.getExcerpt(article, 200);

// 获取渲染后的 HTML
const html = Noteva.articles.getHtml(article);

// 增加浏览计数
await Noteva.articles.incrementView(article.id);
```

## 页面 API

### 获取自定义页面

```javascript
const page = await Noteva.pages.get('about');  // 通过 slug
// 返回:
// {
//   id: 1,
//   title: "关于",
//   slug: "about",
//   content: "Markdown",
//   html: "HTML"
// }
```

## 分类和标签 API

```javascript
// 获取所有分类
const categories = await Noteva.categories.list();

// 获取所有标签
const tags = await Noteva.tags.list();
```

## 评论 API

### 获取文章评论

```javascript
const comments = await Noteva.comments.list(articleId);
// 返回评论数组，包含嵌套的 replies
// 每个评论包含 is_author 字段标识是否为作者/管理员评论
```

### 发表评论

```javascript
// 游客评论
const comment = await Noteva.comments.create({
  articleId: 1,
  content: '评论内容',
  nickname: '昵称',      // 游客必填
  email: 'email@example.com',  // 可选
  parentId: null,        // 回复时填父评论 ID
});

// 管理员评论（已登录状态下自动使用账户信息）
const comment = await Noteva.comments.create({
  articleId: 1,
  content: '评论内容',
  parentId: null,
});
```

## 交互 API（v0.1.9 新增）

```javascript
// 点赞或取消点赞
const result = await Noteva.interactions.like('article', articleId);
// { liked: true, like_count: 11 }

// 检查是否已点赞
const { liked } = await Noteva.interactions.checkLike('article', articleId);
```

## 搜索工具（v0.1.9 新增）

```javascript
// 高亮搜索关键词
const highlighted = Noteva.search.highlight('Hello World 你好世界', '世界');
// 'Hello World 你好<mark class="noteva-highlight">世界</mark>'
```

## 用户 API

> 注意：v0.0.7 起移除了前台用户注册/登录系统，仅保留管理员登录功能。普通访客以游客身份评论。

```javascript
// 检查登录状态（异步，会请求后端）
const currentUser = await Noteva.user.check();

// 同步检查（需先调用 check）
const isLoggedIn = Noteva.user.isLoggedIn();
const user = Noteva.user.getCurrent();
// user: { id, username, email, avatar, role, display_name }

// 登出（仅管理员）
await Noteva.user.logout();
```

## 路由辅助

```javascript
// 获取当前路径
const path = Noteva.router.getPath();  // "/posts/hello"

// 获取查询参数
const page = Noteva.router.getQuery('page');  // "2"

// 路由匹配
const match = Noteva.router.match('/posts/:slug');
// { matched: true, params: { slug: "hello" } }

// 导航
Noteva.router.push('/posts/new');
```

## 工具函数

```javascript
// 日期格式化
Noteva.utils.formatDate(date, 'YYYY-MM-DD');
Noteva.utils.formatDate(date, 'YYYY年MM月DD日');

// 相对时间
Noteva.utils.timeAgo(date);  // "3 天前"

// HTML 转义
Noteva.utils.escapeHtml('<script>');

// 截断文本
Noteva.utils.truncate(text, 100);

// 防抖/节流
const debouncedFn = Noteva.utils.debounce(fn, 300);
const throttledFn = Noteva.utils.throttle(fn, 100);

// 复制到剪贴板
await Noteva.utils.copyToClipboard(text);
```

## UI 组件

```javascript
// Toast 提示
Noteva.ui.toast('保存成功');
Noteva.ui.toast('操作失败', 'error');

// 确认对话框
const confirmed = await Noteva.ui.confirm('确定删除？');

// 加载状态
Noteva.ui.showLoading();
Noteva.ui.hideLoading();
```

## 事件系统

```javascript
// 监听事件
Noteva.events.on('user:login', (user) => {
  console.log('管理员登录:', user);
});

// 内置事件
// - theme:ready     主题加载完成
// - user:login      管理员登录
// - user:logout     管理员登出
// - route:change    路由变化
// - comment:create  评论发表
```

## 钩子系统

```javascript
// 注册钩子
Noteva.hooks.on('content_render', (context) => {
  // 内容渲染时触发
});

// 触发钩子
Noteva.hooks.trigger('custom_hook', data);
```

## 本地存储

```javascript
// 自动 JSON 序列化
Noteva.storage.set('key', { foo: 'bar' });
const data = Noteva.storage.get('key');
Noteva.storage.remove('key');
```

## SEO

```javascript
Noteva.seo.setTitle('页面标题');
Noteva.seo.setMeta({ description: '描述', keywords: '关键词' });
Noteva.seo.setOpenGraph({ title: '标题', image: '图片' });
```

## 插件兼容

主题需要预留插件注入点：

```html
<div id="noteva-slot-body-start"></div>
<div id="noteva-slot-header-before"></div>
<!-- 内容 -->
<div id="noteva-slot-article-top"></div>
<div id="noteva-slot-article-bottom"></div>
<div id="noteva-slot-footer-after"></div>
<div id="noteva-slot-body-end"></div>
```

## 完整示例：原生 JS 主题

```html
<!DOCTYPE html>
<html>
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>My Theme</title>
  <link rel="stylesheet" href="./style.css">
</head>
<body>
  <div id="app">Loading...</div>
  <script src="./app.js"></script>
</body>
</html>
```

```javascript
// app.js
Noteva.ready(async () => {
  const app = document.getElementById('app');
  const site = await Noteva.site.getInfo();
  const nav = await Noteva.site.getNav();
  const { articles } = await Noteva.articles.list();
  
  app.innerHTML = `
    <header>
      <h1>${site.name}</h1>
      <nav>
        ${nav.map(item => `<a href="${item.url}">${item.name}</a>`).join('')}
      </nav>
    </header>
    <main>
      ${articles.map(a => `
        <article>
          <h2><a href="${Noteva.site.getArticleUrl(a)}">${a.title}</a></h2>
          <p>${Noteva.articles.getExcerpt(a, 150)}</p>
        </article>
      `).join('')}
    </main>
    <footer>${site.footer || ''}</footer>
  `;
});
```

## 开发调试

1. 启动后端：`cargo run`（端口 8080）
2. 主题放在 `themes/` 目录
3. 后台切换主题测试
4. 使用 `Noteva.debug.enable()` 开启调试模式

## 注意事项

1. **不要手动引入 SDK** — 后端自动注入
2. **使用 `Noteva.ready()`** — 确保 SDK 加载完成
3. **使用 `getArticleUrl()`** — 不要硬编码文章链接，permalink 模式可能是 id 或 slug
4. **API 路径已封装** — 直接使用 SDK 方法，不要手动拼接 `/api/v1/*`
5. **导航支持二级菜单** — 检查 `children` 字段
6. **暗色模式** — 使用 `.dark` CSS class 选择器，不要用 `prefers-color-scheme`

## 暗色模式

后台切换暗色模式时会在 `<html>` 标签添加 `class="dark"`。主题应使用此 class 实现暗色样式：

```css
/* ✅ 正确做法 */
.dark body { background: #1a1a1a; color: #e5e5e5; }
.dark .card { background: #2a2a2a; }

/* ❌ 错误做法 */
@media (prefers-color-scheme: dark) { ... }
```

## 开发与构建

### 开发流程

```bash
# 1. 启动后端（端口 8080）
cargo run

# 2. 进入主题目录开发
cd themes/my-theme
pnpm install
pnpm dev          # 主题自身的 dev server（可选）

# 3. 构建主题
pnpm build        # 输出到 dist/

# 4. 后台切换到该主题测试
# 访问 http://localhost:8080/manage/settings → 主题 → 切换
```

### 打包发布

发布主题时只需包含以下文件：

```
my-theme/
├── theme.json          # 必需
├── settings.json       # 可选
├── preview.png         # 推荐
└── dist/               # 必需（编译后的前端资源）
    ├── index.html
    ├── assets/
    └── ...
```

**不要包含** `node_modules`、`src`、`package.json` 等源码文件。用户安装的是编译后的产物。

### 兼容性声明

在 `theme.json` 中通过 `requires` 字段声明最低兼容版本：

```json
{
  "name": "My Theme",
  "version": "1.0.0",
  "requires": ">=0.2.0"
}
```

## 商业主题授权

商业主题可以通过伴生插件实现授权验证。主题本身没有 WASM，授权逻辑由伴生插件在 `theme_activate` 钩子中完成。

### 工作原理

1. 主题作者发布一个伴生插件（如 `my-theme-license`），包含 `backend.wasm`
2. 伴生插件监听 `theme_activate` 钩子
3. 用户切换到该主题时，伴生插件验证授权码
4. 验证失败返回 `{allow: false, message: "..."}` → 主题切换被回滚

### 伴生插件示例

```json
{
  "id": "my-theme-license",
  "name": "My Theme 授权",
  "version": "1.0.0",
  "hooks": { "backend": ["theme_activate"] },
  "permissions": ["network", "storage"],
  "settings": true
}
```

`settings.json` 中定义授权码输入框：

```json
{
  "sections": [{
    "id": "license",
    "title": "授权设置",
    "fields": [{
      "id": "license_key",
      "type": "text",
      "label": "授权码",
      "secret": true,
      "placeholder": "输入授权码"
    }]
  }]
}
```

详细的 WASM 授权验证实现参见 `docs/plugin-development.md` 中的 `plugin_activate` / `theme_activate` 章节。

## 自定义国际化（i18n）

Noteva 支持用户上传自定义语言包 JSON，实现任意语言的界面翻译。自定义语言包数据由后端在页面加载时自动注入，**所有主题无需单独适配即可使用**。

### 工作原理

后端在注入 SDK 时，同时注入 `window.__CUSTOM_LOCALES__` 全局变量：

```html
<!-- 后端自动注入（无需手动引入） -->
<script>
  window.__SITE_CONFIG__ = { ... };
  window.__CUSTOM_LOCALES__ = [
    {
      "code": "ja",
      "name": "日本語",
      "translations": { "common": { "save": "保存", ... }, ... }
    },
    ...
  ];
</script>
```

### 在主题中使用

主题通过 SDK 的 `Noteva.i18n` API 获取自定义语言包数据：

```typescript
// 方式一：使用 SDK API 获取自定义语言包列表
const customLocales = Noteva.i18n.getCustomLocales();
// 返回: [{ code: "ja", name: "日本語", translations: { ... } }, ...]

// 方式二：获取所有可用语言（内置 + 自定义合并）
const allLocales = Noteva.i18n.getLocales(builtinLocales);
// 返回: [{ code: "zh-CN", ... }, { code: "ja", name: "日本語", isCustom: true }, ...]

// 方式三：直接注册到主题的 i18n 系统
for (const item of Noteva.i18n.getCustomLocales()) {
  customMessages[item.code] = item.translations;
  localeList.push({ code: item.code, name: item.name, isCustom: true });
}
```

### SDK `Noteva.i18n` API 参考

| 方法 | 返回值 | 说明 |
|------|--------|------|
| `getLocale()` | `string` | 获取当前语言代码，如 `"zh-CN"` |
| `setLocale(code)` | `void` | 切换语言，触发 `locale:change` 事件 |
| `addMessages(locale, messages)` | `void` | 注册翻译消息（合并到已有消息） |
| `t(key, params?)` | `string` | 按 key 翻译，支持 `{name}` 参数占位符 |
| `loadCustomLocales()` | `Array` | 从 `window.__CUSTOM_LOCALES__` 加载自定义语言包 |
| `getCustomLocales()` | `Array<{code, name, translations}>` | 获取所有自定义语言包（首次调用自动加载） |
| `getLocales(builtinLocales?)` | `Array<{code, name, nativeName, isCustom?}>` | 合并内置 + 自定义语言列表 |

> `getCustomLocales()` 和 `getLocales()` 是懒加载的，首次调用时自动读取服务端注入的数据，无需手动调用 `loadCustomLocales()`。

### JSON 格式

自定义语言包 JSON 结构与内置语言包完全相同，采用嵌套对象 + 点分路径：

```json
{
  "common": {
    "save": "保存",
    "cancel": "キャンセル",
    "delete": "削除"
  },
  "manage": {
    "dashboard": "ダッシュボード",
    "articles": "記事"
  },
  "article": {
    "title": "タイトル",
    "publish": "公開"
  }
}
```

> 缺失的翻译 key 会自动 fallback 到简体中文（`zh-CN`），不会导致页面报错。

### 管理方式

管理员在管理后台 **设置 → 自定义语言包** 中管理：

- **粘贴 JSON**：直接粘贴翻译 JSON
- **上传文件**：上传 `.json` 文件（自动读取文件名作为语言代码）
- **从 URL 加载**：输入 JSON 文件的远程 URL

### API 端点

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/v1/locales` | 公开 | 获取自定义语言列表（code + name） |
| GET | `/api/v1/locales/:code` | 公开 | 获取指定语言的完整翻译 |
| POST | `/api/v1/admin/locales` | 管理员 | 创建/更新自定义语言包 |
| DELETE | `/api/v1/admin/locales/:code` | 管理员 | 删除自定义语言包 |

### 存储

自定义语言包以 JSON 文件形式存储在 `data/locales/` 目录下，每个语言对应一个文件（如 `data/locales/ja.json`）。文件格式为：

```json
{
  "name": "日本語",
  "translations": {
    "common": { "save": "保存", ... },
    ...
  }
}
```

用户也可以直接将 JSON 文件放入 `data/locales/` 目录，服务重启后自动识别。
