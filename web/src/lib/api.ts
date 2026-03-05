import axios, { AxiosError, AxiosRequestConfig } from "axios";
import { t } from "./i18n";

/**
 * Translate API error/success message using i18n apiError namespace.
 * Handles both static messages and dynamic messages with names/versions.
 * Tries: dynamic pattern match → exact message match → error code match → original
 */
function translateApiErrorMessage(code: string, message: string): string {
  // Dynamic pattern matching for messages with names/versions
  const patterns: Array<{ regex: RegExp; key: string; params: (m: RegExpMatchArray) => Record<string, string> }> = [
    {
      regex: /^Theme '(.+)' installed successfully$/,
      key: "apiError.themeInstalled",
      params: (m) => ({ name: m[1] }),
    },
    {
      regex: /^Theme '(.+)' updated from (.+) to (.+)$/,
      key: "apiError.themeUpdated",
      params: (m) => ({ name: m[1], from: m[2], to: m[3] }),
    },
    {
      regex: /^Plugin '(.+)' installed successfully \(from (.+)\)$/,
      key: "apiError.pluginInstalledFrom",
      params: (m) => ({ name: m[1], repo: m[2] }),
    },
    {
      regex: /^Plugin '(.+)' installed successfully$/,
      key: "apiError.pluginInstalled",
      params: (m) => ({ name: m[1] }),
    },
    {
      regex: /^Plugin '(.+)' updated from (.+) to (.+)$/,
      key: "apiError.pluginUpdated",
      params: (m) => ({ name: m[1], from: m[2], to: m[3] }),
    },
  ];

  for (const { regex, key, params } of patterns) {
    const match = message.match(regex);
    if (match) {
      const translated = t(key, params(match));
      if (translated !== key) return translated;
    }
  }

  // Try exact message match (e.g. "Invalid username or password")
  const msgKey = `apiError.${message}`;
  const msgTranslation = t(msgKey);
  if (msgTranslation !== msgKey) return msgTranslation;

  // Try error code match (e.g. "UNAUTHORIZED")
  if (code) {
    const codeKey = `apiError.${code}`;
    const codeTranslation = t(codeKey);
    if (codeTranslation !== codeKey) return codeTranslation;
  }

  // Return original message
  return message;
}
const API_BASE = process.env.NEXT_PUBLIC_API_URL || "/api/v1";

export const api = axios.create({
  baseURL: API_BASE,
  headers: {
    "Content-Type": "application/json",
  },
  withCredentials: true, // Enable cookie-based authentication
});

// Helper: read a cookie value by name
function getCookie(name: string): string | undefined {
  const match = document.cookie.match(new RegExp("(?:^|; )" + name + "=([^;]*)"));
  return match ? decodeURIComponent(match[1]) : undefined;
}

// Request interceptor: attach CSRF token on mutation requests
api.interceptors.request.use((config) => {
  const method = (config.method || "get").toLowerCase();
  if (method !== "get" && method !== "head" && method !== "options") {
    const csrfToken = getCookie("csrf_token");
    if (csrfToken) {
      config.headers["X-CSRF-Token"] = csrfToken;
    }
  }
  return config;
});

// Response interceptor for error handling + message translation
api.interceptors.response.use(
  (response) => {
    // Auto-translate success response messages (e.g. theme/plugin install)
    if (response.data?.message && typeof response.data.message === "string") {
      const msg = response.data.message;
      const translated = translateApiErrorMessage("", msg);
      if (translated !== msg) {
        response.data.message = translated;
      }
    }
    return response;
  },
  (error: AxiosError<ApiErrorResponse>) => {
    // Auto-translate error messages using i18n
    if (error.response?.data?.error?.message) {
      const { code, message } = error.response.data.error;
      const translated = translateApiErrorMessage(code, message);
      if (translated !== message) {
        error.response.data.error.message = translated;
      }
    }

    if (error.response?.status === 401) {
      // Only redirect to login if we're on a protected route (/manage/*)
      // and it's not an auth-related request
      const isManagePage = window.location.pathname.startsWith("/manage");
      const isAuthRequest = error.config?.url?.includes("/auth/");
      const isAuthPage = window.location.pathname === "/manage/login" ||
        window.location.pathname === "/manage/setup";

      if (isManagePage && !isAuthRequest && !isAuthPage) {
        window.location.href = "/manage/login";
      }
    }
    return Promise.reject(error);
  }
);

// Types
export interface ApiErrorResponse {
  error: {
    code: string;
    message: string;
    details?: unknown;
  };
}

export interface PagedResult<T> {
  items: T[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// 文章列表响应（后端返回 articles 而不是 items）
export interface ArticleListResult {
  articles: Article[];
  total: number;
  page: number;
  page_size: number;
  total_pages: number;
}

export interface ListParams {
  page?: number;
  per_page?: number;
  status?: string;
}

// Auth API
export const authApi = {
  login: (usernameOrEmail: string, password: string) =>
    api.post<{ token: string; user: User }>("/auth/login", { username_or_email: usernameOrEmail, password }),

  register: (username: string, email: string, password: string) =>
    api.post<{ user: User }>("/auth/register", { username, email, password }),

  logout: () => api.post("/auth/logout"),

  me: () => api.get<User>("/auth/me"),

  hasAdmin: () => api.get<{ has_admin: boolean }>("/auth/has-admin"),

  updateProfile: (data: { display_name?: string | null; avatar?: string | null }) =>
    api.put<User>("/auth/profile", data),

  changePassword: (currentPassword: string, newPassword: string) =>
    api.put<void>("/auth/password", { current_password: currentPassword, new_password: newPassword }),
};

// Articles API
export const articlesApi = {
  list: (params?: ListParams) =>
    api.get<ArticleListResult>("/articles", { params }),

  get: (slug: string) => api.get<Article>(`/articles/${slug}`),

  // Get article by ID (for admin editing - can access drafts)
  getById: (id: number) => api.get<Article>(`/admin/articles/${id}`),

  create: (data: CreateArticleInput) =>
    api.post<Article>("/articles", data),

  update: (id: number, data: UpdateArticleInput) =>
    api.put<Article>(`/admin/articles/${id}`, data),

  delete: (id: number) => api.delete(`/admin/articles/${id}`),
};

// Categories API
export const categoriesApi = {
  list: () => api.get<{ categories: Category[] }>("/categories"),

  tree: () => api.get<{ categories: CategoryTree[] }>("/categories/tree"),

  create: (data: CreateCategoryInput) =>
    api.post<Category>("/admin/categories", data),

  update: (id: number, data: UpdateCategoryInput) =>
    api.put<Category>(`/admin/categories/${id}`, data),

  delete: (id: number) => api.delete(`/admin/categories/${id}`),
};

// Tags API
export const tagsApi = {
  list: () => api.get<{ tags: Tag[] }>("/tags"),

  cloud: (limit?: number) =>
    api.get<{ tags: TagWithCount[] }>("/tags", { params: { cloud: true, limit } }),

  create: (name: string) =>
    api.post<Tag>("/admin/tags", { name }),

  delete: (id: number) => api.delete(`/admin/tags/${id}`),
};

// Admin API
export const adminApi = {
  dashboard: () => api.get<DashboardStats>("/admin/dashboard"),

  systemStats: () => api.get<SystemStats>("/admin/stats"),

  themes: () => api.get<ThemeListResponse>("/admin/themes"),

  reloadThemes: () => api.post<{ success: boolean; message: string; plugin_count: number }>("/admin/themes/reload"),

  switchTheme: (theme: string) =>
    api.post<ThemeResponse>("/admin/themes/switch", { theme }),

  // Theme installation
  uploadTheme: (file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    return api.post<ThemeInstallResponse>("/admin/themes/upload", formData, {
      headers: { "Content-Type": undefined as any },
    });
  },

  listGitHubReleases: (repo: string) =>
    api.get<GitHubReleaseInfo[]>("/admin/themes/github/releases", { params: { repo } }),

  installGitHubTheme: (downloadUrl: string) =>
    api.post<ThemeInstallResponse>("/admin/themes/github/install", { download_url: downloadUrl }),

  installThemeFromRepo: (repo: string, slug?: string) =>
    api.post<ThemeInstallResponse>("/admin/themes/install-from-repo", { repo, slug }),

  deleteTheme: (name: string) =>
    api.delete(`/admin/themes/${name}`),

  // Theme store
  getThemeStore: () => api.get<ThemeStoreResponse>("/admin/themes/store"),

  // Check for theme updates
  checkThemeUpdates: () => api.get<ThemeUpdatesResponse>("/admin/themes/updates"),

  // Update theme
  updateTheme: (name: string) =>
    api.post<ThemeInstallResponse>(`/admin/themes/${name}/update`),

  // Theme settings
  getThemeSettings: (name: string) =>
    api.get<PluginSettingsResponse>(`/admin/themes/${name}/settings`),

  updateThemeSettings: (name: string, settings: Record<string, unknown>) =>
    api.put<{ success: boolean }>(`/admin/themes/${name}/settings`, settings),

  getSettings: () => api.get<SiteSettings>("/admin/settings"),

  updateSettings: (data: SiteSettingsInput) =>
    api.put<SiteSettings>("/admin/settings", data),

  checkUpdate: (beta: boolean = false) =>
    api.get<UpdateCheckResponse>("/admin/update-check", { params: { beta } }),

  performUpdate: (version: string, beta: boolean = false) =>
    api.post<PerformUpdateResponse>("/admin/update-perform", { version, beta }),

  // Login logs (security)
  getLoginLogs: (params?: { page?: number; per_page?: number; username?: string; ip_address?: string; success?: boolean }) =>
    api.get<LoginLogsResponse>("/admin/login-logs", { params }),

  // Backup & Restore
  downloadBackup: () =>
    api.get("/admin/backup", { responseType: "blob" as any }),
  restoreBackup: (file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    return api.post("/admin/backup/restore", formData, {
      headers: { "Content-Type": undefined as any },
    });
  },
  exportMarkdown: () =>
    api.get("/admin/backup/export-markdown", { responseType: "blob" as any }),
};

// Public site info API (no auth required)
export const siteApi = {
  getInfo: () => api.get<SiteSettings>("/site/info"),
};

// Plugins API
export const pluginsApi = {
  list: () => api.get<PluginListResponse>("/admin/plugins"),

  reload: () => api.post<{ success: boolean; message: string; plugin_count: number }>("/admin/plugins/reload"),

  get: (id: string) => api.get<Plugin>(`/admin/plugins/${id}`),

  toggle: (id: string, enabled: boolean) =>
    api.post<Plugin>(`/admin/plugins/${id}/toggle`, { enabled }),

  getSettings: (id: string) =>
    api.get<PluginSettingsResponse>(`/admin/plugins/${id}/settings`),

  updateSettings: (id: string, settings: Record<string, unknown>) =>
    api.post<{ success: boolean; settings: Record<string, unknown> }>(`/admin/plugins/${id}/settings`, settings),

  // Plugin installation
  uploadPlugin: (file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    return api.post<PluginInstallResponse>("/admin/plugins/upload", formData, {
      headers: { "Content-Type": undefined as any },
    });
  },

  listGitHubReleases: (repo: string) =>
    api.get<GitHubReleaseInfo[]>("/admin/plugins/github/releases", { params: { repo } }),

  installGitHubPlugin: (downloadUrl: string) =>
    api.post<PluginInstallResponse>("/admin/plugins/github/install", { download_url: downloadUrl }),

  uninstall: (id: string) =>
    api.delete(`/admin/plugins/${id}/uninstall`),

  // Plugin store
  getStore: () => api.get<PluginStoreResponse>("/admin/plugins/store"),

  // Check for plugin updates
  checkUpdates: () => api.get<PluginUpdatesResponse>("/admin/plugins/updates"),

  // Install plugin from repo
  installFromRepo: (data: { repo: string; pluginId: string }) =>
    api.post<PluginInstallResponse>("/admin/plugins/install-from-repo", { repo: data.repo, plugin_id: data.pluginId }),

  // Update plugin
  updatePlugin: (id: string) =>
    api.post<PluginInstallResponse>(`/admin/plugins/${id}/update`),
};

// Upload API
export const uploadApi = {
  image: (file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    // Don't set Content-Type manually — let browser set it with correct boundary
    return api.post<UploadResponse>("/upload/image", formData, {
      headers: { "Content-Type": undefined as any },
    });
  },

  file: (file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    return api.post<UploadResponse>("/upload/file", formData, {
      headers: { "Content-Type": undefined as any },
    });
  },

  images: (files: File[]) => {
    const formData = new FormData();
    files.forEach((file) => formData.append("files", file));
    return api.post<MultiUploadResponse>("/upload/images", formData, {
      headers: { "Content-Type": undefined as any },
    });
  },
};

// Type definitions
export interface User {
  id: number;
  username: string;
  email: string;
  role: "admin" | "editor" | "author";
  display_name?: string | null;
  avatar?: string | null;
  created_at: string;
  updated_at: string;
}

export interface Article {
  id: number;
  slug: string;
  title: string;
  content: string;
  content_html: string;
  status: "draft" | "published" | "archived";
  author_id: number;
  category_id: number;
  published_at: string | null;
  created_at: string;
  updated_at: string;
  view_count?: number;
  like_count?: number;
  comment_count?: number;
  thumbnail?: string | null;
  is_pinned?: boolean;
  pin_order?: number;
  tags?: Tag[];
  category?: Category;
  author?: User;
  word_count?: number;
  reading_time?: number;
  scheduled_at?: string | null;
}

export interface CreateArticleInput {
  title: string;
  slug?: string;
  content: string;
  status?: "draft" | "published";
  category_id: number;
  tag_ids?: number[];
  scheduled_at?: string;
}

export interface UpdateArticleInput {
  title?: string;
  slug?: string;
  content?: string;
  status?: "draft" | "published" | "archived";
  category_id?: number;
  tag_ids?: number[];
  thumbnail?: string | null;
  is_pinned?: boolean;
  pin_order?: number;
  scheduled_at?: string | null;
}

export interface Category {
  id: number;
  slug: string;
  name: string;
  description: string | null;
  parent_id: number | null;
  sort_order: number;
  created_at: string;
}

export interface CategoryTree extends Category {
  children: CategoryTree[];
}

export interface CreateCategoryInput {
  name: string;
  slug?: string;
  description?: string;
  parent_id?: number;
}

export interface UpdateCategoryInput {
  name?: string;
  slug?: string;
  description?: string;
  parent_id?: number | null;
}

export interface Tag {
  id: number;
  slug: string;
  name: string;
  created_at: string;
}

export interface TagWithCount extends Tag {
  count: number;
}

export interface DashboardStats {
  total_articles: number;
  published_articles: number;
  total_categories: number;
  total_tags: number;
}

export interface SystemStats {
  version: string;
  memory_bytes: number;
  memory_formatted: string;
  system_total_memory: number;
  system_used_memory: number;
  os_name: string;
  uptime_seconds: number;
  uptime_formatted: string;
  total_requests: number;
  avg_response_time_ms: number;
}

export interface ThemeResponse {
  name: string;
  display_name: string;
  description: string | null;
  version: string;
  author: string | null;
  url: string | null;
  preview: string | null;
  active: boolean;
  requires_noteva: string;
  compatible: boolean;
  compatibility_message: string | null;
  has_settings: boolean;
}

export interface ThemeListResponse {
  themes: ThemeResponse[];
  current: string;
}

export interface StoreThemeInfo {
  slug: string;
  name: string;
  version: string;
  description: string | null;
  author: string | null;
  cover_image: string | null;
  github_url: string | null;
  external_url: string | null;
  license_type: string;
  price_info: string | null;
  download_source: string;
  download_count: number;
  avg_rating: number | null;
  rating_count: number | null;
  tags: string[];
  installed: boolean;
}

export interface ThemeStoreResponse {
  themes: StoreThemeInfo[];
}

export interface ThemeUpdateInfo {
  name: string;
  current_version: string;
  latest_version: string;
  has_update: boolean;
}

export interface ThemeUpdatesResponse {
  updates: ThemeUpdateInfo[];
}

export interface UploadResponse {
  url: string;
  filename: string;
  size: number;
  content_type: string;
}

export interface MultiUploadResponse {
  files: UploadResponse[];
  failed: string[];
}

export interface SiteSettings {
  site_name: string;
  site_description: string;
  site_subtitle: string;
  site_logo: string;
  site_footer: string;
  site_url?: string;
  comment_moderation?: string;
  moderation_keywords?: string;
  permalink_structure?: string;
  custom_css?: string;
  custom_js?: string;
  demo_mode?: boolean;
  [key: string]: string | boolean | undefined;
}

export interface SiteSettingsInput {
  [key: string]: string;
}

// Plugin types
export interface Plugin {
  id: string;
  name: string;
  version: string;
  description: string;
  author: string;
  enabled: boolean;
  has_settings: boolean;
  shortcodes: string[];
  requires_noteva: string;
  compatible: boolean;
  compatibility_message: string | null;
}

export interface PluginListResponse {
  plugins: Plugin[];
}

export interface StorePluginInfo {
  slug: string;
  name: string;
  version: string;
  description: string;
  author: string;
  cover_image: string | null;
  github_url: string | null;
  external_url: string | null;
  license_type: string;
  price_info: string | null;
  download_source: string;
  download_count: number;
  avg_rating: number | null;
  rating_count: number | null;
  tags: string[];
  installed: boolean;
}

export interface PluginStoreResponse {
  plugins: StorePluginInfo[];
}

export interface PluginUpdateInfo {
  id: string;
  current_version: string;
  latest_version: string;
  has_update: boolean;
}

export interface PluginUpdatesResponse {
  updates: PluginUpdateInfo[];
}

export interface PluginSettingsField {
  id: string;
  type: "text" | "textarea" | "number" | "switch" | "select" | "radio" | "checkbox" | "color" | "image" | "array";
  label: string;
  default?: unknown;
  description?: string;
  secret?: boolean;
  options?: { value: string; label: string }[];
  min?: number;
  max?: number;
  // array 类型专用：定义数组项的字段结构
  itemFields?: {
    id: string;
    label: string;
    type: "text" | "number";
    placeholder?: string;
    required?: boolean;
  }[];
}

export interface PluginSettingsSection {
  id: string;
  title: string;
  fields: PluginSettingsField[];
}

export interface PluginSettingsSchema {
  sections: PluginSettingsSection[];
}

export interface PluginSettingsResponse {
  schema: PluginSettingsSchema;
  values: Record<string, unknown>;
}

export interface UpdateCheckResponse {
  current_version: string;
  latest_version: string | null;
  update_available: boolean;
  release_url: string | null;
  release_notes: string | null;
  release_date: string | null;
  is_beta: boolean;
  error: string | null;
}

export interface PerformUpdateResponse {
  success: boolean;
  message: string;
}

// Theme installation types
export interface ThemeInstallResponse {
  success: boolean;
  theme_name: string;
  message: string;
}

export interface GitHubReleaseInfo {
  tag_name: string;
  name: string;
  published_at: string | null;
  assets: GitHubAssetInfo[];
}

export interface GitHubAssetInfo {
  name: string;
  size: number;
  download_url: string;
}

// Plugin installation types
export interface PluginInstallResponse {
  success: boolean;
  plugin_name: string;
  message: string;
}

// Comment management types
export interface AdminComment {
  id: number;
  article_id: number;
  content: string;
  status: string;
  nickname?: string | null;
  email?: string | null;
  avatar_url?: string | null;
  created_at: string;
}

export interface AdminCommentsResponse {
  comments: AdminComment[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// Legacy type aliases
export type PendingComment = AdminComment;
export type PendingCommentsResponse = AdminCommentsResponse;


// Login logs types (security)
export interface LoginLogEntry {
  id: number;
  username: string;
  ip_address: string | null;
  user_agent: string | null;
  success: boolean;
  failure_reason: string | null;
  created_at: string;
}

export interface LoginLogsResponse {
  logs: LoginLogEntry[];
  total: number;
  page: number;
  per_page: number;
  success_count: number;
  failed_count: number;
}

// Comments API (admin)
export const commentsApi = {
  listAll: (params?: { page?: number; per_page?: number; status?: string }) =>
    api.get<AdminCommentsResponse>("/admin/comments", { params }),

  listPending: (params?: { page?: number; per_page?: number }) =>
    api.get<AdminCommentsResponse>("/admin/comments/pending", { params }),

  approve: (id: number) => api.post(`/admin/comments/${id}/approve`),

  reject: (id: number) => api.post(`/admin/comments/${id}/reject`),

  delete: (id: number) => api.delete(`/admin/comments/${id}`),
};

// Files management API (space management)
export interface FileInfo {
  name: string;
  url: string;
  size: number;
  file_type: string;
  is_image: boolean;
  created_at: string;
}

export interface FileListResponse {
  files: FileInfo[];
  total: number;
}

export interface StorageStatsResponse {
  total_files: number;
  total_size: number;
  total_size_display: string;
  image_count: number;
  other_count: number;
}

export const filesApi = {
  list: (params?: { search?: string; file_type?: string }) =>
    api.get<FileListResponse>("/admin/files", { params }),

  stats: () =>
    api.get<StorageStatsResponse>("/admin/files/stats"),

  delete: (filename: string) =>
    api.delete(`/admin/files/${encodeURIComponent(filename)}`),
};
