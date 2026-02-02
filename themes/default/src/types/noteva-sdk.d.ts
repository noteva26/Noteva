/**
 * Noteva SDK 类型声明
 * 全局 window.Noteva 对象的 TypeScript 类型定义
 */

interface NotevaArticle {
  id: number;
  slug: string;
  title: string;
  content: string;
  html: string;  // SDK 返回 html 而不是 content_html
  excerpt?: string;
  coverImage?: string;
  author?: { id: number; username: string; avatar?: string };
  category?: { id: number; name: string; slug: string };
  tags?: { id: number; name: string; slug: string }[];
  createdAt: string;
  updatedAt: string;
  publishedAt?: string;
  viewCount: number;
  likeCount: number;
  commentCount: number;
  isPinned?: boolean;
  pinOrder?: number;
  thumbnail?: string;
}

interface NotevaCategory {
  id: number;
  name: string;
  slug: string;
  description?: string;
  parentId?: number;
  articleCount?: number;
}

interface NotevaTag {
  id: number;
  name: string;
  slug: string;
  articleCount?: number;
}

interface NotevaComment {
  id: number;
  content: string;
  html?: string;
  author?: { id: number; username: string; avatar?: string };
  parentId?: number;
  createdAt: string;
  replies?: NotevaComment[];
  likeCount: number;
  isLiked: boolean;
  nickname?: string;
  avatarUrl?: string;
}

interface NotevaUser {
  id: number;
  username: string;
  email: string;
  avatar?: string;
  display_name?: string;
  role: string;
}

interface NotevaSiteInfo {
  name: string;
  description: string;
  subtitle: string;
  logo: string;
  footer: string;
  permalinkStructure?: string;
  site_name?: string;
  site_description?: string;
  site_subtitle?: string;
  site_logo?: string;
  site_footer?: string;
  permalink_structure?: string;
  email_verification_enabled?: string;
}

interface NotevaNavItem {
  id: number;
  name: string;
  url: string;
  target?: string;
  order: number;
}

interface NotevaPage {
  id: number;
  title: string;
  slug: string;
  content: string;
  html: string;
  createdAt: string;
  updatedAt: string;
}

interface NotevaArticleListResult {
  articles: NotevaArticle[];
  total: number;
  page: number;
  pageSize: number;
  hasMore: boolean;
}

interface NotevaSDK {
  version: string;
  
  // 核心系统
  hooks: {
    on(name: string, callback: Function, priority?: number): void;
    off(name: string, callback: Function): void;
    trigger(name: string, ...args: any[]): any;
    triggerAsync(name: string, ...args: any[]): Promise<any>;
  };
  
  events: {
    on(event: string, callback: Function): void;
    once(event: string, callback: Function): void;
    off(event: string, callback: Function): void;
    emit(event: string, data?: any): void;
  };
  
  api: {
    get<T = any>(url: string, params?: Record<string, any>): Promise<T>;
    post<T = any>(url: string, data?: any): Promise<T>;
    put<T = any>(url: string, data?: any): Promise<T>;
    patch<T = any>(url: string, data?: any): Promise<T>;
    delete<T = any>(url: string): Promise<T>;
  };
  
  // 站点 API
  site: {
    getInfo(): Promise<NotevaSiteInfo>;
    getNav(): Promise<NotevaNavItem[]>;
    getThemeConfig(key?: string): any;
    getArticleUrl(article: { id: number | string; slug?: string }): string;
  };
  
  // 文章 API
  articles: {
    list(params?: {
      page?: number;
      pageSize?: number;
      category?: string;
      tag?: string;
      keyword?: string;
    }): Promise<NotevaArticleListResult>;
    get(slug: string): Promise<NotevaArticle>;
    getRelated(slug: string, params?: { limit?: number }): Promise<NotevaArticle[]>;
    getArchives(): Promise<{ year: number; month: number; count: number }[]>;
  };
  
  // 页面 API
  pages: {
    list(): Promise<NotevaPage[]>;
    get(slug: string): Promise<NotevaPage>;
  };
  
  // 分类 API
  categories: {
    list(): Promise<NotevaCategory[]>;
    get(slug: string): Promise<NotevaCategory>;
  };
  
  // 标签 API
  tags: {
    list(): Promise<NotevaTag[]>;
    get(slug: string): Promise<NotevaTag>;
  };
  
  // 评论 API
  comments: {
    list(articleId: number): Promise<NotevaComment[]>;
    create(data: {
      articleId: number;
      content: string;
      parentId?: number;
      nickname?: string;
      email?: string;
    }): Promise<NotevaComment>;
    delete(commentId: number): Promise<void>;
  };
  
  // 用户 API
  user: {
    isLoggedIn(): boolean;
    getCurrent(): NotevaUser | null;
    check(): Promise<NotevaUser | null>;
    login(credentials: { username: string; password: string }): Promise<{ user: NotevaUser }>;
    register(data: { username: string; email: string; password: string; verification_code?: string }): Promise<{ user: NotevaUser }>;
    logout(): Promise<void>;
    hasPermission(permission: string): boolean;
    updateProfile(data: { display_name?: string; avatar?: string }): Promise<NotevaUser>;
    changePassword(data: { current_password: string; new_password: string }): Promise<void>;
  };
  
  // 路由辅助
  router: {
    getPath(): string;
    getQuery(key: string): string | null;
    getQueryAll(): Record<string, string>;
    match(pattern: string): { matched: boolean; params: Record<string, string> };
    getParam(name: string): string | null;
    push(path: string): void;
    replace(path: string): void;
  };
  
  // 工具函数
  utils: {
    formatDate(date: string | Date, format?: string): string;
    timeAgo(date: string | Date): string;
    escapeHtml(str: string): string;
    truncate(text: string, length: number, suffix?: string): string;
    excerpt(markdown: string, length?: number): string;
    debounce<T extends Function>(fn: T, delay: number): T;
    throttle<T extends Function>(fn: T, delay: number): T;
    copyToClipboard(text: string): Promise<boolean>;
    uniqueId(prefix?: string): string;
    prefersDarkMode(): boolean;
    lazyLoadImages(selector?: string): void;
  };
  
  // UI 组件
  ui: {
    toast(message: string, type?: 'info' | 'success' | 'warning' | 'error', duration?: number): void;
    confirm(options: string | { title?: string; message: string; confirmText?: string; cancelText?: string }): Promise<boolean>;
    showLoading(): void;
    hideLoading(): void;
    modal(options: { title?: string; content: string; onClose?: () => void }): { close: () => void; element: HTMLElement };
  };
  
  // 本地存储
  storage: {
    get<T = any>(key: string, defaultValue?: T): T;
    set(key: string, value: any): void;
    remove(key: string): void;
    clear(): void;
  };
  
  // SEO
  seo: {
    setTitle(title: string): void;
    setMeta(meta: Record<string, string>): void;
    setOpenGraph(og: Record<string, string>): void;
    setTwitterCard(twitter: Record<string, string>): void;
    set(options: { title?: string; meta?: Record<string, string>; og?: Record<string, string>; twitter?: Record<string, string> }): void;
  };
  
  // 国际化
  i18n: {
    getLocale(): string;
    setLocale(locale: string): void;
    addMessages(locale: string, messages: Record<string, any>): void;
    t(key: string, params?: Record<string, any>): string;
  };
  
  // 插件系统
  plugins: {
    register(id: string, plugin: any): void;
    get(id: string): any;
    getSettings(pluginId: string): Record<string, any>;
    saveSettings(pluginId: string, settings: Record<string, any>): Promise<void>;
    getData(pluginId: string, key: string): Promise<any>;
    setData(pluginId: string, key: string, value: any): Promise<void>;
  };
  
  // Shortcode 系统
  shortcodes: {
    register(name: string, handler: any): void;
    render(content: string, context?: any): Promise<string>;
  };
  
  // 插槽系统
  slots: {
    register(name: string, content: string | (() => string), priority?: number): void;
    getContent(name: string): string;
    render(name: string, container: string | HTMLElement): void;
    autoRender(): void;
  };
  
  // 调试
  debug: {
    enable(): void;
    disable(): void;
    logRequests(enabled: boolean): void;
    logEvents(enabled: boolean): void;
    logHooks(enabled: boolean): void;
    mockUser(userData: any): void;
    mockThemeConfig(config: any): void;
  };
  
  // 初始化
  ready(callback?: () => void): Promise<void>;
}

declare global {
  interface Window {
    Noteva: NotevaSDK;
  }
}

export {};
