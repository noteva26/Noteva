import axios, { AxiosError, AxiosRequestConfig } from "axios";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "/api/v1";

export const api = axios.create({
  baseURL: API_BASE,
  headers: {
    "Content-Type": "application/json",
  },
  withCredentials: true, // Enable cookie-based authentication
});

// Response interceptor for error handling
api.interceptors.response.use(
  (response) => response,
  (error: AxiosError<ApiErrorResponse>) => {
    if (error.response?.status === 401) {
      // Only redirect to login if we're on a protected route (/manage/*)
      // and it's not an auth-related request
      const isManagePage = window.location.pathname.startsWith("/manage");
      const isAuthRequest = error.config?.url?.includes("/auth/");
      
      if (isManagePage && !isAuthRequest) {
        window.location.href = "/login";
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
  
  switchTheme: (theme: string) =>
    api.post<ThemeResponse>("/admin/themes/switch", { theme }),
  
  // Theme installation
  uploadTheme: (file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    return api.post<ThemeInstallResponse>("/admin/themes/upload", formData, {
      headers: { "Content-Type": "multipart/form-data" },
    });
  },
  
  listGitHubReleases: (repo: string) =>
    api.get<GitHubReleaseInfo[]>("/admin/themes/github/releases", { params: { repo } }),
  
  installGitHubTheme: (downloadUrl: string) =>
    api.post<ThemeInstallResponse>("/admin/themes/github/install", { download_url: downloadUrl }),
  
  deleteTheme: (name: string) =>
    api.delete(`/admin/themes/${name}`),
  
  getSettings: () => api.get<SiteSettings>("/admin/settings"),
  
  updateSettings: (data: SiteSettingsInput) =>
    api.put<SiteSettings>("/admin/settings", data),
  
  checkUpdate: (beta: boolean = false) =>
    api.get<UpdateCheckResponse>("/admin/update-check", { params: { beta } }),
};

// Public site info API (no auth required)
export const siteApi = {
  getInfo: () => api.get<SiteSettings>("/site/info"),
};

// Plugins API
export const pluginsApi = {
  list: () => api.get<PluginListResponse>("/admin/plugins"),
  
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
      headers: { "Content-Type": "multipart/form-data" },
    });
  },
  
  listGitHubReleases: (repo: string) =>
    api.get<GitHubReleaseInfo[]>("/admin/plugins/github/releases", { params: { repo } }),
  
  installGitHubPlugin: (downloadUrl: string) =>
    api.post<PluginInstallResponse>("/admin/plugins/github/install", { download_url: downloadUrl }),
  
  uninstall: (id: string) =>
    api.delete(`/admin/plugins/${id}/uninstall`),
};

// Upload API
export const uploadApi = {
  image: (file: File) => {
    const formData = new FormData();
    formData.append("file", file);
    return api.post<UploadResponse>("/upload/image", formData, {
      headers: { "Content-Type": "multipart/form-data" },
    });
  },
  
  images: (files: File[]) => {
    const formData = new FormData();
    files.forEach((file) => formData.append("files", file));
    return api.post<MultiUploadResponse>("/upload/images", formData, {
      headers: { "Content-Type": "multipart/form-data" },
    });
  },
};

// Type definitions
export interface User {
  id: number;
  username: string;
  email: string;
  role: "admin" | "editor" | "author";
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
}

export interface CreateArticleInput {
  title: string;
  slug?: string;
  content: string;
  status?: "draft" | "published";
  category_id: number;
  tag_ids?: number[];
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
}

export interface ThemeListResponse {
  themes: ThemeResponse[];
  current: string;
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
  require_login_to_comment?: string;
  comment_moderation?: string;
  [key: string]: string | undefined;
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
}

export interface PluginListResponse {
  plugins: Plugin[];
}

export interface PluginSettingsField {
  id: string;
  type: "text" | "textarea" | "number" | "switch" | "select" | "radio" | "checkbox" | "color" | "image" | "array";
  label: string;
  default?: unknown;
  description?: string;
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

// User management types
export interface UserAdmin {
  id: number;
  username: string;
  email: string;
  role: string;
  status: string;
  display_name?: string | null;
  avatar?: string | null;
  created_at: string;
  updated_at: string;
}

export interface UserListResponse {
  users: UserAdmin[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

export interface UpdateUserInput {
  username?: string;
  email?: string;
  role?: string;
  status?: string;
}

// Users API (admin)
export const usersApi = {
  list: (params?: { page?: number; per_page?: number }) =>
    api.get<UserListResponse>("/admin/users", { params }),
  
  get: (id: number) => api.get<UserAdmin>(`/admin/users/${id}`),
  
  update: (id: number, data: UpdateUserInput) =>
    api.put<UserAdmin>(`/admin/users/${id}`, data),
  
  delete: (id: number) => api.delete(`/admin/users/${id}`),
};

// Comment moderation types
export interface PendingComment {
  id: number;
  article_id: number;
  content: string;
  nickname?: string | null;
  email?: string | null;
  avatar_url?: string | null;
  created_at: string;
}

export interface PendingCommentsResponse {
  comments: PendingComment[];
  total: number;
  page: number;
  per_page: number;
  total_pages: number;
}

// Comments API (admin)
export const commentsApi = {
  listPending: (params?: { page?: number; per_page?: number }) =>
    api.get<PendingCommentsResponse>("/admin/comments/pending", { params }),
  
  approve: (id: number) => api.post(`/admin/comments/${id}/approve`),
  
  reject: (id: number) => api.post(`/admin/comments/${id}/reject`),
};

// Email API
export const emailApi = {
  sendCode: (email: string) =>
    api.post<{ message: string }>("/auth/send-code", { email }),
  
  testEmail: (to: string) =>
    api.post<{ success: boolean; message: string }>("/admin/email/test", { email: to }),
};
