# 更新日志

[English](CHANGELOG.en.md) | 简体中文

Noteva 的所有重要变更都会记录在这里。

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
