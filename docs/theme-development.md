# Noteva 主题开发指南

本文档描述 Noteva Theme Runtime SDK v1 的主题开发方式。Noteva 不绑定 React、Vue 或任何特定前端框架：主题可以是纯 HTML/CSS/JavaScript，也可以是任意框架构建后的静态资源。

## 主题目录

主题放在 `themes/<theme-name>/` 下：

```text
themes/my-theme/
├── theme.json
├── settings.json        # 可选
├── dist/
│   ├── index.html
│   └── assets/
└── preview.png          # 可选
```

规则：

- `theme.json` 必须存在。
- `dist/index.html` 必须存在，是主题运行入口。
- `settings.json` 可选；如果存在，必须符合 Theme Settings Schema v1。
- `preview` 可选；如果在 `theme.json` 中声明，文件必须存在且必须是相对路径。
- 主题源码可以保留在包内，但 Noteva 只保证运行 `dist/` 中的构建产物。

`dist/index.html` 是运行入口。Noteva 会在服务端注入站点配置、SDK、插件资源和 SEO 信息，主题无需手动引入 SDK 文件。

## theme.json

最小合法示例：

```json
{
  "schema": 1,
  "name": "My Theme",
  "short": "my-theme",
  "description": "A clean Noteva theme",
  "version": "1.0.0",
  "author": "Your Name",
  "repository": "https://github.com/user/my-theme",
  "requires": {
    "noteva": ">=0.2.6"
  }
}
```

必填字段：

- `schema`: 当前固定为 `1`。
- `name`: 后台展示名称，可以使用中文。
- `short`: 主题 ID，必须匹配 `^[a-z0-9][a-z0-9-]{0,62}$`，并且安装后的目录名必须与它一致。
- `description`: 简短描述。
- `version`: 主题版本，使用 `x.y.z` 格式。
- `author`: 作者名称。
- `repository`: 更新来源，必须是 GitHub URL 或 `owner/repo`。
- `requires.noteva`: 主题要求的 Noteva 版本。

可选字段：

- `preview`: 预览图相对路径，例如 `preview.png`。
- `pages`: 主题需要自动创建的页面声明。
- `configuration`: 只读静态配置，会通过 `Noteva.theme.getConfig()` 暴露给前台主题。

`pages` 示例：

```json
{
  "pages": [
    {
      "slug": "about",
      "title": "关于"
    }
  ]
}
```

## settings.json

`settings.json` 用来声明后台可编辑的主题设置。没有设置项的主题不需要这个文件。

```json
{
  "schema": 1,
  "sections": [
    {
      "id": "appearance",
      "title": "Appearance",
      "fields": [
        {
          "id": "show_toc",
          "type": "switch",
          "label": "Show TOC",
          "default": true
        }
      ]
    }
  ]
}
```

字段类型 v1 只承诺这些：

- `text`
- `textarea`
- `switch`
- `select`
- `number`
- `color`
- `array`

通用规则：

- section `id` 和 field `id` 必须匹配 `^[a-z][a-z0-9_]{0,63}$`。
- `label` 必填，支持字符串或多语言对象。
- `description`、`placeholder` 可选，支持字符串或多语言对象。
- `default` 必须与字段类型匹配。
- `secret: true` 的字段不会暴露给公开主题接口。
- `select` 必须提供 `options`。
- `array` 必须提供 `itemFields`，v1 的数组子字段只支持 `text` 和 `number`。

## 校验时机

Noteva 会在这些位置校验主题：

- 安装主题包时。
- 扫描已安装主题时。
- 读取 `settings.json` 时。
- 保存主题设置时。

不合规主题不会进入可用主题列表；不合规设置不会被保存。

## JSON Schema

编辑器和主题开发工具可以使用：

- `docs/schemas/theme.schema.json`
- `docs/schemas/theme-settings.schema.json`

## SDK 加载

主题代码应等待 SDK 初始化：

```html
<script>
  Noteva.ready(async () => {
    const site = await Noteva.site.getInfo();
    document.querySelector("#site-title").textContent = site.name;
  });
</script>
```

框架项目中也一样：

```ts
await window.Noteva.ready();
const articles = await window.Noteva.articles.list({ page: 1, pageSize: 10 });
```

## 全局对象

SDK 暴露在 `window.Noteva`：

```ts
Noteva = {
  version,
  sdkVersion,
  ready,

  api,
  site,
  theme,
  articles,
  pages,
  categories,
  tags,
  comments,
  interactions,
  user,

  urls,
  router,
  page,
  utils,
  errors,
  search,
  upload,
  cache,
  ui,
  storage,
  seo,
  toc,
  i18n,

  hooks,
  events,
  plugins,
  shortcodes,
  slots,
  emoji
}
```

SDK v1 的公共数据字段统一使用 `camelCase`。主题代码不需要处理后端原始的 `snake_case` 字段。

`Noteva.api` 是低层请求入口，适合插件或高级主题扩展兜底使用。普通主题优先使用 `site/articles/pages/categories/tags/comments/interactions/theme` 等高层 API，避免直接绑定后端路径。

## SDK 稳定边界

Theme Runtime SDK v1 的目标是给主题和前台插件提供稳定、轻量、框架无关的运行时能力。只要主题通过下列公开模块开发，后续小版本会尽量保持兼容：

```ts
Noteva.ready
Noteva.site
Noteva.theme
Noteva.articles
Noteva.pages
Noteva.categories
Noteva.tags
Noteva.comments
Noteva.interactions
Noteva.user
Noteva.urls
Noteva.router
Noteva.page
Noteva.utils
Noteva.errors
Noteva.search
Noteva.upload
Noteva.cache
Noteva.ui
Noteva.storage
Noteva.seo
Noteva.toc
Noteva.i18n
Noteva.hooks
Noteva.events
Noteva.plugins
Noteva.shortcodes
Noteva.slots
Noteva.emoji
```

稳定含义：

- 字段统一使用 `camelCase`，主题不直接处理后端 `snake_case`。
- 高层 API 的方法名、参数语义和返回结构会保持稳定。
- `Noteva.urls.*` 是链接生成的标准入口，主题不直接拼永久链接。
- `Noteva.page` 是当前页面上下文的标准入口，插件不需要各自猜 URL 或 DOM。
- `Noteva.hooks` 和 `Noteva.events` 的回调异常不会中断后续回调。

高级兜底：

- `Noteva.api` 可以用于少量高级扩展，但直接依赖 `/api/v1/*` 路径会降低主题兼容性。
- `Noteva.upload` 需要登录态，适合前台投稿、资料编辑或插件扩展，不是普通展示主题的必需能力。
- `Noteva.cache` 是前端扩展缓存能力，不应用来保存敏感数据。

不属于 Theme Runtime SDK 的范围：

- 后台管理接口，例如主题安装、插件安装、用户管理、系统设置写入。
- 插件设置写入、插件启停、插件安装卸载。
- v0 阶段的兼容别名，例如 `site.getThemeSettings()`、`site.getArticleUrl()`、`articles.getStats()`、`articles.getExcerpt()`。

如果主题必须使用低层接口，建议把调用集中封装在主题自己的适配层，避免业务组件散落硬编码 API 路径。

## 站点信息

```ts
const site = await Noteva.site.getInfo();
```

返回结构：

```ts
{
  version: "0.2.6",
  name: "Noteva",
  description: "",
  subtitle: "",
  logo: "/logo.png",
  footer: "",
  url: "https://example.com",
  permalinkStructure: "/posts/{slug}",
  emailVerificationEnabled: false,
  demoMode: false,
  customCss: "",
  customJs: "",
  fontFamily: "",
  stats: {
    totalArticles: 12,
    totalCategories: 3,
    totalTags: 8
  }
}
```

导航：

```ts
const nav = await Noteva.site.getNav();
```

导航项：

```ts
{
  id: 1,
  parentId: null,
  title: "Archives",
  name: "Archives",
  type: "builtin",
  target: "archives",
  url: "archives",
  openNewTab: false,
  order: 10,
  visible: true,
  children: []
}
```

刷新站点缓存：

```ts
await Noteva.site.refresh();
```

## 主题信息与设置

主题基础信息：

```ts
const info = await Noteva.theme.getInfo();
```

主题配置：

```ts
const config = await Noteva.theme.getConfig();
const color = await Noteva.theme.getConfig("primaryColor");
```

主题设置：

```ts
const settings = await Noteva.theme.getSettings();
const showToc = await Noteva.theme.getSetting("show_toc", true);
```

SDK 会把公开设置转换成可直接使用的类型：

```ts
{
  show_toc: true,
  posts_per_page: 12,
  hero_links: ["about", "archive"]
}
```

主题作者不需要再写：

```ts
settings.show_toc === true || settings.show_toc === "true"
```

公开设置来自 `settings.json` 的默认值叠加数据库保存值。`secret: true` 的字段不会出现在公开接口中。

## 文章

列表：

```ts
const result = await Noteva.articles.list({
  page: 1,
  pageSize: 10,
  category: "tech",
  tag: "rust",
  keyword: "noteva",
  sort: "date"
});
```

返回：

```ts
{
  articles: [],
  total: 42,
  page: 1,
  pageSize: 10,
  totalPages: 5,
  hasMore: true
}
```

详情：

```ts
const article = await Noteva.articles.get("hello-world");
```

文章对象：

```ts
{
  id: 1,
  slug: "hello-world",
  title: "Hello World",
  content: "# Hello",
  html: "<h1>Hello</h1>",
  excerpt: "Hello",
  thumbnail: "/uploads/cover.jpg",
  coverImage: "/uploads/cover.jpg",
  status: "published",
  category: { id: 1, slug: "tech", name: "Tech", articleCount: 0 },
  tags: [{ id: 1, slug: "rust", name: "Rust", articleCount: 0 }],
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z",
  publishedAt: "2026-01-01T00:00:00Z",
  viewCount: 100,
  likeCount: 10,
  commentCount: 5,
  wordCount: 800,
  readingTime: 3,
  isPinned: false,
  pinOrder: 0,
  prev: null,
  next: null,
  related: [],
  toc: []
}
```

相关文章：

```ts
const related = await Noteva.articles.related("hello-world", { limit: 5 });
```

归档：

```ts
const archives = await Noteva.articles.archives();
```

归档项：

```ts
{ month: "2026-01", year: 2026, monthNumber: 1, count: 8 }
```

浏览量递增：

```ts
await Noteva.articles.incrementView(article.id);
```

## 页面、分类、标签

页面：

```ts
const pages = await Noteva.pages.list();
const page = await Noteva.pages.get("about");
```

页面字段：

```ts
{
  id: 1,
  slug: "about",
  title: "About",
  content: "Markdown",
  html: "<p>Markdown</p>",
  status: "published",
  source: "user",
  createdAt: "2026-01-01T00:00:00Z",
  updatedAt: "2026-01-01T00:00:00Z"
}
```

分类：

```ts
const categories = await Noteva.categories.list();
const category = await Noteva.categories.get("tech");
```

标签：

```ts
const tags = await Noteva.tags.list();
const tag = await Noteva.tags.get("rust");
```

分类和标签统一使用 `articleCount` 表示文章数量。

## 评论

读取评论：

```ts
const comments = await Noteva.comments.list(article.id);
```

创建评论：

```ts
const comment = await Noteva.comments.create({
  articleId: article.id,
  content: "Nice post!",
  nickname: "Alice",
  email: "alice@example.com"
});
```

回复评论：

```ts
await Noteva.comments.create({
  articleId: article.id,
  parentId: comment.id,
  content: "Thanks!",
  nickname: "Bob"
});
```

评论字段：

```ts
{
  id: 1,
  articleId: 1,
  userId: null,
  parentId: null,
  nickname: "Alice",
  email: "alice@example.com",
  content: "Nice post!",
  html: "",
  status: "approved",
  createdAt: "2026-01-01T00:00:00Z",
  avatarUrl: "https://www.gravatar.com/avatar/...",
  likeCount: 0,
  isLiked: false,
  isAuthor: false,
  replies: []
}
```

最近评论：

```ts
const recent = await Noteva.comments.recent(10);
```

## 点赞

```ts
const result = await Noteva.interactions.like("article", article.id);
```

返回：

```ts
{ success: true, liked: true, likeCount: 11 }
```

检查点赞状态：

```ts
const status = await Noteva.interactions.checkLike("comment", comment.id);
```

## 用户状态

前台主题只暴露当前登录状态，不承担完整账户系统：

```ts
const user = await Noteva.user.check();

if (user?.role === "admin") {
  // show manage link
}
```

可用方法：

```ts
Noteva.user.isLoggedIn();
Noteva.user.getCurrent();
await Noteva.user.check();
await Noteva.user.logout();
Noteva.user.hasPermission("admin");
```

## URL 生成

不要手写文章永久链接，使用 `Noteva.urls`：

```ts
const postUrl = Noteva.urls.article(article);
const categoryUrl = Noteva.urls.category(category);
const tagUrl = Noteva.urls.tag(tag);
const pageUrl = Noteva.urls.page(page);
const assetUrl = Noteva.urls.asset("assets/app.css");
const uploadUrl = Noteva.urls.upload("cover.jpg");
```

文章 URL 会遵循后台设置的 `permalinkStructure`。

## 页面上下文

SDK 会维护当前页面上下文，方便主题和插件判断当前是否处于文章页或自定义页面：

```ts
const current = Noteva.page.get();

if (current.type === "article") {
  console.log(current.articleId, current.article?.title);
}
```

读取文章或页面时，SDK 会自动更新上下文：

```ts
const article = await Noteva.articles.get("hello-world");
console.log(Noteva.page.articleId); // article.id

const page = await Noteva.pages.get("about");
console.log(Noteva.page.type); // "page"
```

插件如果只需要当前文章 ID，优先使用：

```ts
const articleId = Noteva.page.articleId;
```

## 工具能力

常用文本和浏览器工具：

```ts
const html = Noteva.utils.escapeHtml(title);
const excerpt = Noteva.utils.excerpt(markdown, 160);
const date = Noteva.utils.formatDate(article.publishedAt, "YYYY-MM-DD");
const highlighted = Noteva.search.highlight(article.title, keyword);
```

本地主题状态建议使用 `Noteva.storage`，会自动加 `noteva_` 前缀：

```ts
Noteva.storage.set("theme-density", "compact");
const density = Noteva.storage.get("theme-density", "comfortable");
```

简单交互组件：

```ts
Noteva.ui.toast("保存成功", "success");
const confirmed = await Noteva.ui.confirm("确定继续吗？");
```

上传和前端缓存属于认证/扩展能力，不是普通文章列表渲染必须项：

```ts
const image = await Noteva.upload.image(file);
const files = await Noteva.upload.images(fileList);
const attachment = await Noteva.upload.file(file);

await Noteva.cache.set("theme:sidebar", data, 3600);
const cached = await Noteva.cache.get("theme:sidebar");
await Noteva.cache.delete("theme:sidebar");
```

`Noteva.upload.*` 需要当前用户已登录。公开主题如果不提供登录态上传入口，不需要使用它。

## 插件槽位

主题应为常用位置预留插件槽位：

```html
<div data-noteva-slot="article_content_top"></div>
<div data-noteva-slot="article_content_bottom"></div>
<div data-noteva-slot="comment_form_before"></div>
```

SDK 会自动渲染：

```ts
Noteva.slots.autoRender();
```

手动渲染：

```ts
Noteva.slots.render("article_content_top", "#slot");
```

## 前端插件 API

插件前端脚本同样通过 `window.Noteva` 工作。公开插件能力只包含前台安全读接口和运行时注册能力：

```ts
const settings = Noteva.plugins.getSettings("music-player");
const data = await Noteva.plugins.getData("ip-location", "article_locs:1");

Noteva.plugins.register("my-plugin", {
  init() {
    // plugin init
  }
});
```

插件设置写入、插件数据写入、插件启停和安装卸载属于后台管理能力，不放进 Theme Runtime SDK。前端插件需要写数据时，应走插件自己的 WASM API 或后端声明的插件接口。

## Hooks 与 Events

Hook 用于可修改数据的流程：

```ts
const off = Noteva.hooks.on("article_view", (article) => {
  return article;
});

off();
```

Event 用于广播通知：

```ts
const unsubscribe = Noteva.events.on("comment:create", (comment) => {
  console.log(comment);
});

unsubscribe();
```

单个 Hook 或 Event 回调异常不会中断后续回调，SDK 会输出错误并继续执行。

## SEO

文章页：

```ts
Noteva.seo.setArticleMeta(article, site.name, window.location.origin);
```

站点页：

```ts
Noteva.seo.setSiteMeta(site.name, site.description, site.url);
```

自定义：

```ts
Noteva.seo.set({
  title: "Custom Title",
  meta: { description: "Custom description" },
  og: { type: "website" },
  twitter: { card: "summary" }
});
```

## TOC

```ts
const toc = Noteva.toc.extract(".article-content", "h2,h3");

const stop = Noteva.toc.observe(toc, (activeId) => {
  console.log(activeId);
});

Noteva.toc.scrollTo("section-1", 80);
stop();
```

## i18n

```ts
Noteva.i18n.setLocale("zh-CN");
Noteva.i18n.addMessages("zh-CN", {
  "theme.readMore": "阅读全文"
});

const text = Noteva.i18n.t("theme.readMore");
```

自定义语言包：

```ts
const locales = Noteva.i18n.getLocales();
const customLocales = Noteva.i18n.getCustomLocales();
```

## 错误处理

SDK 请求错误会抛出 `NotevaError`：

```ts
try {
  await Noteva.articles.get("missing");
} catch (error) {
  if (Noteva.errors.isNotFound(error)) {
    // render 404
  }
}
```

可用判断：

```ts
Noteva.errors.isNotFound(error);
Noteva.errors.isUnauthorized(error);
Noteva.errors.isForbidden(error);
Noteva.errors.isValidation(error);
```

## 纯 HTML 主题示例

```html
<!doctype html>
<html lang="zh-CN">
  <head>
    <meta charset="utf-8" />
    <title>Noteva Theme</title>
  </head>
  <body>
    <header>
      <h1 id="site-title"></h1>
      <nav id="nav"></nav>
    </header>
    <main id="articles"></main>

    <script>
      Noteva.ready(async () => {
        const site = await Noteva.site.getInfo();
        document.querySelector("#site-title").textContent = site.name;

        const nav = await Noteva.site.getNav();
        document.querySelector("#nav").innerHTML = nav
          .filter((item) => item.visible)
          .map((item) => `<a href="${item.url}">${item.title}</a>`)
          .join("");

        const result = await Noteva.articles.list({ page: 1, pageSize: 10 });
        document.querySelector("#articles").innerHTML = result.articles
          .map((article) => `
            <article>
              <h2><a href="${Noteva.urls.article(article)}">${article.title}</a></h2>
              <p>${article.excerpt}</p>
            </article>
          `)
          .join("");
      });
    </script>
  </body>
</html>
```

## 规则

- 使用 `Noteva.ready()` 等待 SDK 初始化。
- 使用 `Noteva.*` 访问公开能力，不硬编码 `/api/v1/*`。
- 主题代码使用 SDK v1 的 `camelCase` 字段。
- 文章链接使用 `Noteva.urls.article()`，不要手写 `/posts/${slug}`。
- 公开主题设置通过 `Noteva.theme.getSettings()` 或 `Noteva.theme.getSetting()` 读取。
- 深色模式使用 `.dark` 类选择器。
- 插件扩展位置使用 `data-noteva-slot` 或 `Noteva.slots.render()`。
- 插件前端只读取公开 settings/data；写入插件设置或状态走后台或插件自有 API。
- 不要依赖 v0 阶段的兼容别名和字段兜底。
