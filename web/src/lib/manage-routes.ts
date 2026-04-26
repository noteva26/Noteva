export const managePageLoaders = {
  dashboard: () => import("@/pages/manage/dashboard"),
  articles: () => import("@/pages/manage/articles"),
  articleNew: () => import("@/pages/manage/articles/new"),
  articleEdit: () => import("@/pages/manage/articles/edit"),
  categories: () => import("@/pages/manage/categories"),
  tags: () => import("@/pages/manage/tags"),
  pages: () => import("@/pages/manage/pages"),
  nav: () => import("@/pages/manage/nav"),
  comments: () => import("@/pages/manage/comments"),
  plugins: () => import("@/pages/manage/plugins"),
  themes: () => import("@/pages/manage/themes"),
  security: () => import("@/pages/manage/security"),
  files: () => import("@/pages/manage/files"),
  settings: () => import("@/pages/manage/settings"),
  login: () => import("@/pages/manage/login"),
  setup: () => import("@/pages/manage/setup"),
};

const manageRoutePreloaders: Record<string, () => Promise<unknown>> = {
  "/manage": managePageLoaders.dashboard,
  "/manage/articles": managePageLoaders.articles,
  "/manage/articles/new": managePageLoaders.articleNew,
  "/manage/categories": managePageLoaders.categories,
  "/manage/tags": managePageLoaders.tags,
  "/manage/pages": managePageLoaders.pages,
  "/manage/nav": managePageLoaders.nav,
  "/manage/comments": managePageLoaders.comments,
  "/manage/files": managePageLoaders.files,
  "/manage/plugins": managePageLoaders.plugins,
  "/manage/themes": managePageLoaders.themes,
  "/manage/security": managePageLoaders.security,
  "/manage/settings": managePageLoaders.settings,
};

function preloadMarkdownEditor() {
  void import("@/components/ui/markdown-editor");
}

export function preloadManageRoute(href: string) {
  const loader =
    manageRoutePreloaders[href] ||
    (href.startsWith("/manage/articles/")
      ? managePageLoaders.articleEdit
      : undefined);

  if (loader) {
    void loader();
  }

  if (href === "/manage/articles/new" || /^\/manage\/articles\/\d+/.test(href)) {
    preloadMarkdownEditor();
  }
}
