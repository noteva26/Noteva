# 更新日志

[English](CHANGELOG.en.md) | 简体中文

Noteva 的所有重要变更都会记录在这里。

## [v0.2.4] - 2026-03-28

### 🔌 插件集成规范化
- **Default 主题 data 属性补齐** — 7 个组件统一添加 `data-article-id`、`data-comment-id`、`data-page-id` 属性及语义 CSS class（`article-meta`、`article-content`、`comment-meta`、`comment-content`、`comment-actions`、`article-list`、`page-content`），插件可通过标准选择器定位 DOM
- **自定义页面插件扩展点** — `custom-page.tsx` 新增 `page_content_top` / `page_content_bottom` PluginSlot 插槽

### 🐛 Bug 修复
- **插件商城 ID 映射错误** — 修复主程序消费商城 API 时 `StorePluginInfo.slug` 直接使用原始 slug 而非 `plugin_id` 的问题，现在优先使用 `plugin_id`（为空时回退 slug）

---

## [v0.2.3] - 2026-03-23

### 🔌 插件系统增强
- **Hook 数据完整性补齐** — 评论、用户登录、API 中间件等 11 个 Hook 触发点补齐 `ip`、`user_agent`、`email`、`created_at` 等上下文字段，插件无需 hack 即可获取完整请求信息
- **插件开发文档更新** — `docs/plugin-development.md` Hook 参考表同步更新，所有事件数据字段与代码一致

### 🔧 框架升级
- **Default 主题 + 管理后台 Tailwind CSS v4** — 从 v3.4 升级至 v4，CSS-first 配置（`@theme` + `@plugin`），移除 `tailwind.config.ts`、`postcss.config`，使用 `@tailwindcss/vite` 插件

### 📝 体验优化
- **自动语言检测** — 首次访问时根据浏览器语言环境自动匹配可用语言包，无匹配则回退到英文

### 🐛 Bug 修复
- **前台自定义语言不显示** — 修复 Default 主题语言切换器仅显示内置语言的问题，现在正确加载后台添加的自定义语言包

### 🏗️ CI/CD
- **Release Notes 自动化** — `release.yml` 从 `CHANGELOG.en.md` 自动提取当前版本日志，取代硬编码模板

---

## [v0.2.2] - 2026-03-22

### 🔧 框架升级
- **Axum 0.8** — 后端框架从 0.7.9 升级至 0.8.8（路由语法 `:param` → `{param}`、tower 0.5、tower-http 0.6）

### 🔒 安全修复
- **CSRF Token 改用 CSPRNG** — `generate_csrf_token()` 从 `DefaultHasher`（可预测）改为 `getrandom`（密码学安全随机数）
- **SHA256 更新校验** — 自动下载 `.sha256` 校验文件并验证二进制完整性，不匹配则拒绝更新
- **许可证统一** — `Cargo.toml` 许可证从 MIT 修正为 GPL-3.0-or-later（与 LICENSE 文件一致）
- **插件上传路径注入防护** — `plugin_id` 新增严格校验，拒绝 `../` 等路径穿越攻击
- **评论长度限制** — 新增 10,000 字符上限，防止恶意超大评论写入数据库
- **密码强度统一** — 注册密码校验从「非空」改为「≥8 字符」，与修改密码一致
- **Cookie 构建安全** — `auth.rs` 中 `HeaderValue::expect()` 改为 `map_err()`，避免非法字符触发 panic
- **Zip Slip 防护** — 主题及插件 ZIP/TAR 解压均新增路径校验，阻止恶意压缩包写入系统任意位置
- **上传目录穿越防护** — `/uploads/` 静态文件服务新增 `canonicalize` 校验，阻止 `../` 读取配置文件和数据库
- **CORS 配置容错** — `cors_origin` 解析失败不再 panic，回退至 `*` 并打印警告日志
- **主题/插件删除路径校验** — `delete_theme` 和 `uninstall_plugin` 均新增名称校验，阻止路径穿越

### 🧹 代码清理
- **前端‘use client’清理** — 移除 24 个组件中无效的 Next.js `"use client"` 指令（Vite + React 项目不需要）

### ⚡ 性能优化
- **N+1 标签查询消除** — 新增 `get_by_article_ids()` 批量方法，文章列表标签获取从 N+1 次查询降为 1 次
- **数据库级排序** — `ArticleSortBy` 枚举全链路接入（API → Service → Repository），移除内存排序
- **具名排序支持** — 文章列表新增 `sort_by` 参数，支持按发布时间或创建时间在数据库层排序

### 🐛 Bug 修复
- **环境变量覆盖失效** — `main.rs` 从 `Config::load()` 改为 `Config::load_with_env()`，`NOTEVA_*` 环境变量现在可正常覆盖配置
- **config.example.yml 字段名错误** — `upload.dir` → `upload.path`、`upload.max_size` → `upload.max_file_size`
- **Docker 缺少 wasm-worker** — Dockerfile 运行时镜像新增 `wasm-worker` 二进制复制，修复容器内插件系统无法运行
- **Release 缺少 wasm-worker** — `release.yml` 三平台打包段均新增 `wasm-worker` 二进制
- **SHA256 校验格式不匹配** — CI 从单一 `checksums.txt` 改为同时生成每个文件的 `.sha256`，与 `update.rs` 下载格式一致

### 🏗️ 运维改进
- **过期 Session 定时清理** — `main.rs` 新增每 30 分钟自动清理过期 session 的后台任务
- **Graceful Shutdown** — 服务器支持 Ctrl+C / SIGTERM 优雅关闭，等待进行中请求完成后再退出（Docker/K8s 兼容）

---

## [v0.2.1] - 2026-03-13

### 🎉 新功能
- **SDK i18n API** — 新增 `Noteva.i18n.getCustomLocales()` / `getLocales()` / `loadCustomLocales()`，主题无需手动读取 `window.__CUSTOM_LOCALES__`
- **自定义语言包文件存储** — 语言包从数据库迁移至 `data/locales/*.json`，JSON 文件直接管理，更轻量
- **管理后台日语支持** — 内置日语翻译（`ja.json`），150+ 翻译 key 覆盖全部管理界面

### 📝 改进
- Default + Prose 主题 `loadCustomLocales()` 改用 SDK API，去除重复代码
- `noteva-sdk.d.ts` TypeScript 类型声明同步更新
- Demo 模式白名单扩展：新增点赞、浏览计数、注册、2FA、缓存、插件代理等交互端点
- `locale.rs` 从 `db/repositories/` 移至 `services/`（文件 I/O 不属于数据库层）
- Migration 29 改为 `DROP TABLE IF EXISTS custom_locales`（已废弃）
- Prose 主题移除调试 `console.log`
- 文档新增 SDK `Noteva.i18n` API 参考表

---

## [v0.2.0] - 2026-03-06

### 🎉 亮点
- **正式版发布** — 去掉 Beta 标签，版本统一为 0.2.0

### 🐛 Bug 修复
- 修复 `setup` 页面通过匹配中文字符串判断管理员是否存在（改为 `errorCode`）
- 修复 `login` 页面速率限制提示信息硬编码中文
- 修复 `plugins/themes/pages/settings` 日期格式化不跟随语言切换（`toLocaleDateString` 未传 locale）
- 修复 `plugins` 页面下载次数 `" downloads"` 硬编码英文
- 修复 `avatar-upload` 组件 4 个 toast 消息硬编码中文
- 修复 `loading-state` 组件 `"加载中..."` 硬编码中文
- 修复 `settings-renderer` 组件 `"添加项目"` 按钮硬编码中文
- 修复 `language-switcher` 组件 `title="切换语言"` 硬编码中文
- 修复 `settings` 页面加载失败错误信息硬编码英文
- 修复 `files` 页面批量删除缺少确认对话框

### 🌍 国际化
- `files/index.tsx` — 34 个硬编码中文字符串全部替换为 i18n
- 新增 `fileManage` i18n 分组（34 keys × 3 语言）
- 新增 `common.addItem`、`common.switchLanguage`、`settings.avatar*` 等 i18n keys
- 所有 `toLocaleString` / `toLocaleDateString` 调用统一传入动态 locale

###  内部改进
- 后端：清除 6 个 compiler warning，修复 3 处 N+1 查询
- 后端：修复 `backup.rs` 中 `.unwrap()` 安全隐患
- 版本号统一为 0.2.0（Cargo.toml、package.json × 3）

---

## [v0.1.9-beta] - 2026-03-05

### 🎉 新功能
- **SDK 字段辅助方法** — `Noteva.articles.getDate/getStats/isPinned/getThumbnail/getExcerpt/getHtml/incrementView`
- **互动模块** — `Noteva.interactions.like()` / `checkLike()` 文章和评论点赞
- **搜索工具** — `Noteva.search.highlight()` 搜索结果关键词高亮
- **自定义字体** — 14 款 Google Fonts + 系统默认字体，通过 SDK CSS 变量 `--noteva-font` 自动注入

### 📝 改进
- 默认主题完全迁移至 SDK 调用（删除约 150 行重复辅助代码）
- `site.getInfo()` 现在通过展开运算符透传所有后端字段
- TypeScript 类型声明已更新（`noteva-sdk.d.ts`）

### 🔧 内部改进
- 版本号统一为 0.1.9-beta（Cargo.toml、SDK、theme.json、hook-registry.json）
- 默认主题中零直接 API 调用（全部通过 SDK）
- 评论接口简化，添加 `[key: string]: any` 回退

---

## [v0.1.8-beta] - 2026-03-04

### 🎉 新功能
- **38+ 后端钩子** — 文章、页面、评论、分类、标签、用户、设置等完整生命周期钩子
- **定时任务系统** — `cron_register` / `cron_tick` 钩子支持插件定时任务（60 秒间隔）
- **RSS 和 Sitemap 过滤器** — `feed_filter` / `sitemap_filter` 钩子允许插件修改 SEO 输出
- **文章导入** — 支持 Markdown ZIP（YAML frontmatter）和 WordPress WXR XML 导入
- **最近评论 API** — `GET /api/v1/comments/recent` 获取全站最近评论
- **字数统计和阅读时间** — 文章页面显示
- **文章导航** — 文章底部上一篇/下一篇链接
- **相关文章** — 文章内容下方推荐相关文章
- **月度归档** — 按年月分组的归档页面

### 📝 改进
- 评论嵌套深度优化（最大 4 层视觉缩进）
- 完整备份与恢复，支持 Markdown 导出

### 🔧 内部改进
- Hook 注册表更新至 v0.1.8-beta（新增 17 个钩子）
- SDK：新增 `comments.recent(limit)` 方法
- 默认主题：文章接口扩展 `word_count`、`reading_time`、`prev`、`next`、`related` 字段
- i18n：新增文章元数据翻译（zh-CN、zh-TW、en）

---

## [v0.1.7-beta]

### 新功能
- 性能优化（路由级懒加载、API 请求缓存）
- 代码质量改进（unwrap 清理、错误处理统一）
- 路由加载指示器（NProgress）
- 数据库抽象层重构（dispatch 宏）

---

## [v0.1.6-beta]

### 新功能
- 主题和插件授权钩子（`theme_activate`、`plugin_activate`）
- 文件上传过滤器（预签名代理）
- 插件升级钩子，支持数据迁移

---

## [v0.1.5]

### 新功能
- 插件销毁钩子，支持资源清理
- 插件升级钩子，支持版本迁移

---

## [v0.1.4-beta]

### 新功能
- 图片上传过滤器钩子
- 插件商城和主题商城

---

## [v0.1.3-beta]

### 新功能
- 初始插件系统（WASM、前端 JS/CSS、Shortcode）
- 评论系统（嵌套回复、审核、表情）
- 用户系统（注册、登录、权限管理）
- 缓存优化（内存 / Redis、ETag）
- 国际化支持（zh-CN、zh-TW、en）
- SEO（Sitemap、RSS、robots.txt）
