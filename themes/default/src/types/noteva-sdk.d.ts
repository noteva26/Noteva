type NotevaSettingValue =
  | string
  | number
  | boolean
  | null
  | NotevaSettingValue[]
  | { [key: string]: NotevaSettingValue };

interface NotevaArticleLink {
  id: number;
  slug: string;
  title: string;
  thumbnail?: string | null;
  url?: string;
}

interface NotevaCategory {
  id: number;
  slug: string;
  name: string;
  description?: string;
  parentId?: number | null;
  articleCount: number;
  createdAt?: string;
  updatedAt?: string;
}

interface NotevaTag {
  id: number;
  slug: string;
  name: string;
  articleCount: number;
  createdAt?: string;
  updatedAt?: string;
}

interface NotevaArticle {
  id: number;
  slug: string;
  title: string;
  content: string;
  html: string;
  excerpt: string;
  thumbnail: string | null;
  coverImage: string | null;
  authorId?: number | null;
  categoryId?: number | null;
  status: string;
  author?: NotevaUser | null;
  category?: NotevaCategory | null;
  tags: NotevaTag[];
  createdAt: string;
  updatedAt: string;
  publishedAt: string;
  scheduledAt?: string | null;
  viewCount: number;
  likeCount: number;
  commentCount: number;
  wordCount: number;
  readingTime: number;
  isPinned: boolean;
  pinOrder: number;
  prev?: NotevaArticleLink | null;
  next?: NotevaArticleLink | null;
  related: NotevaArticleLink[];
  toc: Array<{ id: string; text: string; level: number }>;
  meta?: unknown;
  canonicalUrl?: string | null;
}

interface NotevaComment {
  id: number;
  articleId?: number | null;
  userId?: number | null;
  parentId?: number | null;
  nickname?: string | null;
  email?: string | null;
  content: string;
  html?: string;
  status?: string;
  createdAt: string;
  updatedAt?: string;
  avatarUrl: string;
  likeCount: number;
  isLiked: boolean;
  isAuthor: boolean;
  replies: NotevaComment[];
}

interface NotevaUser {
  id: number;
  username: string;
  email: string;
  role: string;
  status?: string;
  displayName?: string;
  avatar?: string;
  totpEnabled?: boolean;
  createdAt?: string;
  updatedAt?: string;
}

interface NotevaSiteInfo {
  version: string;
  name: string;
  description: string;
  subtitle: string;
  logo: string;
  footer: string;
  url: string;
  permalinkStructure: string;
  emailVerificationEnabled: boolean;
  demoMode: boolean;
  customCss: string;
  customJs: string;
  fontFamily: string;
  stats: {
    totalArticles: number;
    totalCategories: number;
    totalTags: number;
  };
}

interface NotevaThemeInfo {
  name: string;
  displayName: string;
  version: string;
  description?: string;
  author?: string;
  repository?: string;
  preview?: string;
  requiresNoteva?: string;
  compatible?: boolean;
  compatibilityMessage?: string;
  config: Record<string, unknown>;
  hasSettings: boolean;
}

interface NotevaInjectedSiteConfig {
  site_name?: string;
  site_description?: string;
  site_subtitle?: string;
  site_logo?: string;
  site_footer?: string;
  [key: string]: unknown;
}

interface NotevaLocaleInfo {
  code: string;
  name: string;
  nativeName?: string;
  builtIn?: boolean;
}

interface NotevaNavItem {
  id: number;
  parentId?: number | null;
  title: string;
  name: string;
  type: "builtin" | "page" | "external" | string;
  target: string;
  url: string;
  openNewTab: boolean;
  order: number;
  visible: boolean;
  children: NotevaNavItem[];
}

interface NotevaArticleListResult {
  articles: NotevaArticle[];
  total: number;
  page: number;
  pageSize: number;
  totalPages: number;
  hasMore: boolean;
}

interface NotevaArchiveEntry {
  month: string;
  year: number;
  monthNumber: number;
  count: number;
}

interface NotevaRuntimeError extends Error {
  status: number;
  code: string | null;
  data: unknown;
  url: string;
  method: string;
}

interface NotevaUploadResult {
  url: string;
  filename: string;
  size: number;
  contentType: string;
}

interface NotevaMultiUploadResult {
  files: NotevaUploadResult[];
  failed: string[];
}

interface NotevaPageContext {
  type: "unknown" | "article" | "page" | string;
  path: string;
  query: Record<string, string>;
  articleId: number | null;
  article: NotevaArticle | null;
  pageId: number | null;
  customPage: NotevaPage | null;
}

interface NotevaSDK {
  version: string;
  sdkVersion: string;

  hooks: {
    on(name: string, callback: (...args: any[]) => any, priority?: number): () => void;
    off(name: string, callback: (...args: any[]) => any): void;
    trigger<T = any>(name: string, ...args: any[]): T;
    triggerAsync<T = any>(name: string, ...args: any[]): Promise<T>;
  };

  events: {
    on(event: string, callback: (data?: any) => void): () => void;
    once(event: string, callback: (data?: any) => void): () => void;
    off(event: string, callback: (data?: any) => void): void;
    emit(event: string, data?: any): void;
  };

  api: {
    get<T = any>(url: string, params?: Record<string, any>): Promise<T>;
    post<T = any>(url: string, data?: any): Promise<T>;
    put<T = any>(url: string, data?: any): Promise<T>;
    patch<T = any>(url: string, data?: any): Promise<T>;
    delete<T = any>(url: string): Promise<T>;
  };

  site: {
    getInfo(): Promise<NotevaSiteInfo>;
    getNav(): Promise<NotevaNavItem[]>;
    refresh(): Promise<NotevaSiteInfo>;
  };

  theme: {
    getInfo(): Promise<NotevaThemeInfo>;
    getConfig<T = any>(key?: string): Promise<T>;
    getSettings(): Promise<Record<string, NotevaSettingValue>>;
    getSettings<T = NotevaSettingValue>(key: string): Promise<T | undefined>;
    getSetting<T = NotevaSettingValue>(key: string, defaultValue?: T): Promise<T>;
    refreshSettings(): Promise<Record<string, NotevaSettingValue>>;
  };

  articles: {
    list(params?: {
      page?: number;
      pageSize?: number;
      category?: string;
      tag?: string;
      keyword?: string;
      sort?: "date" | "views" | "comments" | "latest" | string;
    }): Promise<NotevaArticleListResult>;
    get(slug: string): Promise<NotevaArticle>;
    related(slug: string, params?: { limit?: number }): Promise<NotevaArticleLink[]>;
    archives(): Promise<NotevaArchiveEntry[]>;
    incrementView(articleId: number): Promise<void>;
  };

  pages: {
    list(): Promise<NotevaPage[]>;
    get(slug: string): Promise<NotevaPage>;
  };

  categories: {
    list(): Promise<NotevaCategory[]>;
    get(slug: string): Promise<NotevaCategory | null>;
  };

  tags: {
    list(): Promise<NotevaTag[]>;
    get(slug: string): Promise<NotevaTag | null>;
  };

  comments: {
    list(articleId: number): Promise<NotevaComment[]>;
    create(data: {
      articleId: number;
      content: string;
      parentId?: number;
      nickname?: string;
      email?: string;
    }): Promise<NotevaComment>;
    recent(limit?: number): Promise<NotevaComment[]>;
  };

  interactions: {
    like(targetType: "article" | "comment", targetId: number): Promise<{ success: boolean; liked: boolean; likeCount: number }>;
    checkLike(targetType: "article" | "comment", targetId: number): Promise<{ success: boolean; liked: boolean; likeCount: number }>;
  };

  user: {
    isLoggedIn(): boolean;
    getCurrent(): NotevaUser | null;
    check(): Promise<NotevaUser | null>;
    logout(): Promise<void>;
    hasPermission(permission: string): boolean;
  };

  urls: {
    article(article: { id: number | string; slug?: string }): string;
    category(category: string | { slug?: string }): string;
    tag(tag: string | { slug?: string }): string;
    page(page: string | { slug?: string }): string;
    asset(path: string): string;
    upload(path: string): string;
  };

  router: {
    getPath(): string;
    getQuery(key: string): string | null;
    getQueryAll(): Record<string, string>;
    match(pattern: string): { matched: boolean; params: Record<string, string> };
    getParam(name: string): string | null;
    push(path: string): void;
    replace(path: string): void;
  };

  page: NotevaPageContext & {
    set(context?: Partial<NotevaPageContext>): NotevaPageContext;
    get(): NotevaPageContext;
    clear(): NotevaPageContext;
  };

  utils: {
    formatDate(date: string | Date, format?: string): string;
    timeAgo(date: string | Date): string;
    escapeHtml(str: string): string;
    truncate(text: string, length: number, suffix?: string): string;
    excerpt(markdown: string, length?: number): string;
    debounce<T extends (...args: any[]) => any>(fn: T, delay: number): T;
    throttle<T extends (...args: any[]) => any>(fn: T, delay: number): T;
    copyToClipboard(text: string): Promise<boolean>;
    uniqueId(prefix?: string): string;
    prefersDarkMode(): boolean;
    lazyLoadImages(selector?: string): void;
  };

  errors: {
    NotevaError: new (message: string, options?: Record<string, unknown>) => NotevaRuntimeError;
    isNotFound(error: unknown): boolean;
    isUnauthorized(error: unknown): boolean;
    isForbidden(error: unknown): boolean;
    isValidation(error: unknown): boolean;
  };

  ui: {
    toast(message: string, type?: "info" | "success" | "warning" | "error", duration?: number): void;
    confirm(options: string | { title?: string; message: string; confirmText?: string; cancelText?: string }): Promise<boolean>;
    showLoading(): void;
    hideLoading(): void;
    modal(options: { title?: string; content: string; onClose?: () => void }): { close: () => void; element: HTMLElement };
  };

  storage: {
    get<T = any>(key: string, defaultValue?: T): T;
    set(key: string, value: any): void;
    remove(key: string): void;
    clear(): void;
  };

  search: {
    highlight(text: string, keyword: string): string;
  };

  upload: {
    image(file: File): Promise<NotevaUploadResult>;
    images(files: File[] | FileList): Promise<NotevaMultiUploadResult>;
    file(file: File): Promise<NotevaUploadResult>;
    pluginFile(pluginId: string, file: File): Promise<NotevaUploadResult>;
  };

  cache: {
    get<T = any>(key: string): Promise<T | null>;
    set(key: string, value: any, ttl?: number): Promise<void>;
    delete(key: string): Promise<void>;
  };

  seo: {
    setTitle(title: string): void;
    setMeta(meta: Record<string, string>): void;
    setOpenGraph(og: Record<string, string>): void;
    setTwitterCard(twitter: Record<string, string>): void;
    set(options: { title?: string; meta?: Record<string, string>; og?: Record<string, string>; twitter?: Record<string, string> }): void;
    setArticleMeta(article: { title: string; excerpt?: string; thumbnail?: string | null; coverImage?: string | null; slug?: string; id?: number | string; publishedAt?: string; updatedAt?: string }, siteName: string, siteUrl?: string): void;
    setSiteMeta(siteName: string, description: string, siteUrl?: string): void;
  };

  toc: {
    extract(container?: string | HTMLElement, levels?: string): Array<{ id: string; text: string; level: number }>;
    scrollTo(id: string, offset?: number): void;
    observe(items: Array<{ id: string }>, callback: (activeId: string) => void, offset?: number): () => void;
  };

  i18n: {
    getLocale(): string;
    setLocale(locale: string): void;
    addMessages(locale: string, messages: Record<string, any>): void;
    t(key: string, params?: Record<string, any>): string;
    getLocales(builtinLocales?: NotevaLocaleInfo[]): NotevaLocaleInfo[];
  };

  plugins: {
    register(id: string, plugin: any): void;
    get(id: string): any;
    getSettings(pluginId: string): Record<string, any>;
    list(): any[];
    getData(pluginId: string, key: string): Promise<any>;
  };

  shortcodes: {
    register(name: string, handler: any): void;
    render(content: string, context?: any): Promise<string>;
  };

  slots: {
    register(name: string, content: string | (() => string), priority?: number): void;
    getContent(name: string): string;
    render(name: string, container: string | HTMLElement): void;
    autoRender(): void;
  };

  emoji: {
    categories: Array<{
      id: string;
      label: Record<string, string>;
      icon: string;
      emojis: Record<string, string>;
    }>;
    getCategories(locale?: string): Array<{
      id: string;
      label: string;
      icon: string;
      emojis: Record<string, string>;
    }>;
    getMap(): Record<string, string>;
    loadTwemoji(): Promise<any>;
    parse(element: HTMLElement, options?: Record<string, any>): Promise<void>;
    parseSync(element: HTMLElement, options?: Record<string, any>): void;
    isLoaded(): boolean;
  };

  debug: {
    enable(): void;
    disable(): void;
    logRequests(enabled: boolean): void;
    logEvents(enabled: boolean): void;
    logHooks(enabled: boolean): void;
    mockUser(userData: any): void;
    mockThemeConfig(config: any): void;
  };

  ready(callback?: () => void): Promise<void>;
}

interface NotevaPage {
  id: number;
  slug: string;
  title: string;
  content: string;
  html: string;
  status: string;
  source?: string;
  createdAt: string;
  updatedAt: string;
}

declare global {
  interface Window {
    Noteva: NotevaSDK;
    __SITE_CONFIG__?: NotevaInjectedSiteConfig;
  }
}

export {};
