# 更新日志

[English](CHANGELOG.en.md) | 简体中文

Noteva 的所有重要变更都会记录在这里。

## [v0.2.8] - 2026-04-27

### 插件系统
- **插件数据库 API 完善** - 为 WASM 插件补齐隔离数据库访问能力，支持插件在声明权限后执行受控 SQL 操作，并通过插件 ID 自动隔离数据边界。
- **插件数据库安全约束增强** - 收紧插件数据库调用的权限校验、语句校验和执行边界，避免插件越权访问核心数据或其他插件数据。

### 管理后台
- **侧边栏版本入口优化** - 在后台侧边栏增加当前版本与更新检查入口，保留更新日志查看能力，并移除设置页中重复的系统更新入口。
- **仪表盘最近文章收敛** - `/manage` 仪表盘最近文章固定展示最新 5 篇，加载态和渲染态保持一致。
- **文章编辑器滚动体验优化** - 新建和编辑文章时 Markdown 编辑区与预览区保持稳定高度，长内容改为内部滚动，避免编辑页面被内容持续撑高。
- **文章状态筛选修复** - `/manage/articles` 的已发布、草稿、已归档筛选改为后端按状态查询和计数，草稿与归档不再错误显示全部文章。
- **定时发布语义完善** - 文章设置定时发布时间后会作为草稿等待后台任务自动发布，发布时间可清空，实际发布后会自动移除定时标记。

### Bug 修复
- **文章 Emoji 破图修复** - Markdown 渲染不再把原生 Unicode Emoji 强制转换为外链 Twemoji 图片，修复星星等表情在默认主题文章表格中显示为破图的问题。
- **文章缩略图提取修复** - SDK 提取文章缩略图时会跳过 Emoji/Twemoji 图片，避免把正文第一个表情错误识别为文章缩略图。

### 文档
- **插件开发文档更新** - 补充插件数据库 API、权限声明、调用约束和使用建议，方便插件作者按统一方式访问插件私有数据。

### 构建与版本
- **版本号统一为 0.2.8** - 同步更新 Rust crate、前端包、默认主题、SDK 内置版本和开发元信息中的项目版本。
- **CI 测试稳定性修复** - 修复 SQLite 迁移失败回滚测试在并发环境下可能出现的 schema lock，降低 CI 偶发失败风险。

---

## [v0.2.7] - 2026-04-26

### 主题系统
- **Theme Runtime SDK v1 定型** - 清理旧式兼容别名，将主题侧调用收敛到更稳定的 `Noteva.*` 模块化 API，默认主题已按新版 SDK 适配。
- **主题包规范校验增强** - 新增 `theme.json` schema v1 校验，要求主题声明 `schema`、`short`、`description`、`repository`、`requires.noteva` 等关键字段，并校验 `dist/index.html`、预览图路径和仓库地址格式。
- **主题设置校验收紧** - `settings.json` 支持 schema v1 校验，保存设置时会拒绝未知字段，并按字段类型进行校验和转换，减少主题配置异常导致的运行时问题。

### Bug 修复
- **默认主题页头布局修复** - 修复顶部 logo 区域左侧留白过大的问题，让页头内容回到正常对齐。
- **默认主题下拉菜单抖动修复** - 修复语言切换、主题切换等下拉菜单打开时触发页面横向位移的问题。
- **默认主题视觉回归修复** - 恢复默认主题纯净背景，并回落首页标题与文章卡片字号，避免阅读和列表页面显得过大、过空。

### 文档
- **主题开发文档补充** - 将主题包结构、字段要求、仓库地址、版本兼容和设置声明规范合并进 `docs/theme-development.md`。
- **主题 JSON Schema 新增** - 新增 `theme.json` 和 `settings.json` 的 JSON Schema，方便后续主题开发、校验和编辑器提示。

### 构建与版本
- **版本号统一为 0.2.7** - 同步更新 Rust crate、前端包、默认主题、SDK 内置版本和开发元信息中的项目版本。

---

## [v0.2.6] - 2026-04-26

### 亮点
- **管理后台和默认主题升级到 React 19** - 后台与默认主题依赖统一升级到 React 19 / React DOM 19，并完成相关 TypeScript、构建和组件兼容调整。
- **管理后台体验优化** - 优化文章、分类、标签、页面、导航、评论、文件、插件、主题、安全日志等页面的加载状态和交互细节，减少骨架屏闪烁和重复 loading。
- **默认主题阅读体验重构** - 文章页从“详情页”调整为更适合长期阅读的版式，移除顶部返回按钮，优化正文宽度、目录位置、行高、段落间距和信息密度。

### 管理后台
- **列表页加载状态收敛** - 多个管理列表页改为首屏 skeleton + 后续同步提示，避免切换筛选、刷新数据时整页反复闪烁。
- **统一后台基础组件** - 新增页面标题、数据同步条、确认弹窗等后台通用组件，减少重复 UI 逻辑。
- **交互确认体验改进** - 替换部分原生 `confirm` / `alert`，后台删除、批量操作等流程更统一。
- **类型和格式化辅助收敛** - 新增 API 错误解析、时间/数字格式化、GitHub 工具等共享辅助方法，减少页面内重复代码。
- **React Compiler 试验入口** - 管理后台新增 gated React Compiler 构建入口，可通过 `REACT_COMPILER=1` 验证，不默认启用。

### 默认主题
- **文章阅读页优化** - 正文主列扩大到更舒适的阅读宽度；有目录时目录改为右侧 sticky 辅助栏，无目录时自动回到单栏居中。
- **阅读密度调整** - 收紧正文行高、段落间距、标题间距和正文卡片内边距，降低无效空白，提升中文长文阅读连续性。
- **文章卡片统一** - 新增共享文章摘要卡片，首页、分类详情、标签详情复用统一的标题、摘要、缩略图、分类、标签和互动数据展示。
- **分类、标签、归档页面优化** - 分类/标签列表和详情页改为更清晰的信息索引；归档页改为更紧凑的时间线式文章索引。
- **SDK 等待逻辑统一** - 将默认主题中分散的 SDK 轮询收敛到 `waitForNoteva()`，减少重复 `setTimeout`、卸载后状态更新和页面闪烁风险。
- **评论与 Emoji 加载优化** - 评论区和 Emoji 选择器改用统一 SDK 等待逻辑，保持 React 19 `useOptimistic` 评论提交体验。
- **页头页脚精修** - 页头新增当前导航态、移动菜单路由切换自动收起和更稳定的站点配置读取；页脚默认版权文案不再依赖 HTML 注入。

### 构建与 CI
- **CI 构建顺序修复** - CI 改为先构建 `web/dist` 和 `themes/default/dist`，再执行 Rust `cargo check` / `cargo test`，修复 `rust-embed` 在 dist 缺失时导致的检查失败。
- **前端 chunk 拆分优化** - 管理后台和默认主题优化 Vite 构建拆包，React、UI、Motion 等依赖拆分为稳定 vendor chunks，减少主入口体积并改善缓存命中。
- **版本号统一为 0.2.6** - 同步更新 `Cargo.toml`、`Cargo.lock`、根 `package.json`、`web/package.json`、`themes/default/package.json`、默认主题 `theme.json` 和 SDK 内置版本。

---

## [v0.2.5] - 2026-04-25

### 安全加固
- **禁用通用插件代理** - `/api/v1/plugins/proxy` 不再转发任意 URL，避免前端插件借公共代理触发 SSRF 或泄露服务端密钥。需要外部 API 的插件应通过插件设置保存用户配置，并由 WASM 后端在 `network` 权限下发起请求。
- **WASM Worker 沙箱收紧** - 为插件子进程补充内存、指令、请求/响应大小、日志/存储/数据库操作数量限制；HTTP 仅允许 `http/https`，禁止本地、内网、元数据地址和跳转。
- **插件/主题安装包校验增强** - ZIP/TAR 解包统一校验路径，拒绝路径穿越、符号链接、特殊文件、异常包名、超大条目和超大解包体积。

### 稳定性
- **核心迁移事务化** - 数据库迁移的 SQL 执行和 `_migrations` 记录写入放在同一事务中，并校验已应用迁移的版本、名称和连续性，避免半成功状态继续启动。
- **插件迁移事务化** - 插件 `migrations/` SQL 与 `plugin_migrations` 记录写入改为同一事务；失败时不再留下半创建表或错误记录。
- **安装覆盖顺序收紧** - 插件安装先定位并校验 `plugin.json` 和真实插件 ID，再覆盖目标目录，减少坏包误删已有插件的风险。

### 工程质量
- **新增 CI 工作流** - 增加 Rust `cargo check --all-targets --locked`、`cargo test --locked`，以及后台和默认主题前端构建检查。
- **前端锁文件纳入提交** - 放开 `web/pnpm-lock.yaml` 和 `themes/default/pnpm-lock.yaml`，保证 CI 的 `--frozen-lockfile` 可复现。
- **前端低风险清理** - 移除后台未使用的 `@dnd-kit/*` 依赖和默认主题未使用的 `axios`，并收紧少量 `any` 类型，不调整路由和核心交互。

### 文档
- **插件开发文档更新** - 补充通用插件代理禁用、WASM 网络限制、安装包安全约束和插件迁移事务化说明。
- **主题开发文档更新** - 补充主题安装包安全约束。

---

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
