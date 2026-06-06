/**
 * Noteva SDK
 * 为主题和插件提供统一的 API 接口
 */
(function (window) {
  'use strict';

  // API 基础路径
  const API_BASE = '/api/v1';
  const SDK_VERSION = '1.0.0';

  class NotevaError extends Error {
    constructor(message, options = {}) {
      super(message);
      this.name = 'NotevaError';
      this.status = options.status || 0;
      this.code = options.code || null;
      this.data = options.data || null;
      this.url = options.url || '';
      this.method = options.method || '';
    }
  }

  const MUTATION_METHODS = new Set(['POST', 'PUT', 'PATCH', 'DELETE']);

  function getCookie(name) {
    if (typeof document === 'undefined' || !document.cookie) return '';
    const escapedName = name.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
    const match = document.cookie.match(new RegExp(`(?:^|; )${escapedName}=([^;]*)`));
    return match ? decodeURIComponent(match[1]) : '';
  }

  function csrfHeaders(method) {
    if (!MUTATION_METHODS.has(String(method || 'GET').toUpperCase())) return {};
    const token = getCookie('csrf_token');
    return token ? { 'X-CSRF-Token': token } : {};
  }

  function extractErrorMessage(result, fallback) {
    if (typeof result === 'string' && result.trim()) return result;
    if (!result || typeof result !== 'object') return fallback;

    if (typeof result.message === 'string' && result.message.trim()) {
      return result.message;
    }

    if (typeof result.error === 'string' && result.error.trim()) {
      return result.error;
    }

    if (result.error && typeof result.error === 'object') {
      const message = result.error.message || result.error.error;
      if (typeof message === 'string' && message.trim()) return message;
    }

    return fallback;
  }

  // ============================================
  // 钩子系统
  // ============================================
  const hooks = {
    _hooks: {},

    /**
     * 注册钩子
     * @param {string} name - 钩子名称
     * @param {Function} callback - 回调函数
     * @param {number} priority - 优先级（数字越小越先执行）
     */
    on(name, callback, priority = 10) {
      if (!this._hooks[name]) {
        this._hooks[name] = [];
      }
      this._hooks[name].push({ callback, priority });
      this._hooks[name].sort((a, b) => a.priority - b.priority);
      return () => this.off(name, callback);
    },

    /**
     * 移除钩子
     */
    off(name, callback) {
      if (!this._hooks[name]) return;
      this._hooks[name] = this._hooks[name].filter(h => h.callback !== callback);
    },

    /**
     * 触发钩子
     * @param {string} name - 钩子名称
     * @param {...any} args - 传递给回调的参数
     * @returns {any} - 最后一个回调的返回值
     */
    trigger(name, ...args) {
      if (!this._hooks[name]) return args[0];
      let result = args[0];
      for (const hook of this._hooks[name]) {
        try {
          const ret = hook.callback(result, ...args.slice(1));
          if (ret !== undefined) result = ret;
        } catch (e) {
          console.error(`[Noteva] Hook handler error for "${name}":`, e);
          events.emit('hook:error', { name, error: e });
        }
      }
      return result;
    },

    /**
     * 异步触发钩子
     */
    async triggerAsync(name, ...args) {
      if (!this._hooks[name]) return args[0];
      let result = args[0];
      for (const hook of this._hooks[name]) {
        try {
          const ret = await hook.callback(result, ...args.slice(1));
          if (ret !== undefined) result = ret;
        } catch (e) {
          console.error(`[Noteva] Async hook handler error for "${name}":`, e);
          events.emit('hook:error', { name, error: e });
        }
      }
      return result;
    },
  };

  // ============================================
  // 事件系统
  // ============================================
  const events = {
    _listeners: {},

    on(event, callback) {
      if (!this._listeners[event]) {
        this._listeners[event] = [];
      }
      this._listeners[event].push(callback);
      return () => this.off(event, callback);
    },

    once(event, callback) {
      const wrapper = (...args) => {
        this.off(event, wrapper);
        callback(...args);
      };
      return this.on(event, wrapper);
    },

    off(event, callback) {
      if (!this._listeners[event]) return;
      this._listeners[event] = this._listeners[event].filter(cb => cb !== callback);
    },

    emit(event, data) {
      if (!this._listeners[event]) return;
      for (const callback of this._listeners[event]) {
        try {
          callback(data);
        } catch (e) {
          console.error(`[Noteva] Event handler error for "${event}":`, e);
        }
      }
    },
  };

  // ============================================
  // HTTP 请求封装
  // ============================================
  async function request(method, url, data = null, options = {}) {
    const config = {
      method,
      headers: {
        'Content-Type': 'application/json',
        ...options.headers,
        ...csrfHeaders(method),
      },
      credentials: 'include',
    };

    if (data && (method === 'POST' || method === 'PUT' || method === 'PATCH')) {
      config.body = JSON.stringify(data);
    }

    // 触发请求前钩子
    hooks.trigger('api_request_before', { method, url, data });

    try {
      const response = await fetch(API_BASE + url, config);
      const result = await response.json().catch(() => ({}));

      // 触发请求后钩子
      hooks.trigger('api_request_after', { method, url, response, result });

      if (!response.ok) {
        throw new NotevaError(extractErrorMessage(result, `HTTP ${response.status}`), {
          status: response.status,
          code: result.code,
          data: result,
          url,
          method,
        });
      }

      return result;
    } catch (error) {
      hooks.trigger('api_error', error);
      throw error;
    }
  }

  const api = {
    get: (url, params) => {
      if (params) {
        // 过滤掉 undefined, null, 空字符串
        const filtered = {};
        for (const [key, value] of Object.entries(params)) {
          if (value !== undefined && value !== null && value !== '') {
            filtered[key] = value;
          }
        }
        const query = Object.keys(filtered).length > 0
          ? '?' + new URLSearchParams(filtered).toString()
          : '';
        return request('GET', url + query);
      }
      return request('GET', url);
    },
    post: (url, data) => request('POST', url, data),
    put: (url, data) => request('PUT', url, data),
    patch: (url, data) => request('PATCH', url, data),
    delete: (url) => request('DELETE', url),
  };

  const firstValue = (...values) => values.find(value => value !== undefined && value !== null);

  const asNumber = (value, fallback = 0) => {
    if (typeof value === 'number' && Number.isFinite(value)) return value;
    if (typeof value === 'string' && value.trim() !== '') {
      const parsed = Number(value);
      if (Number.isFinite(parsed)) return parsed;
    }
    return fallback;
  };

  const asBoolean = (value, fallback = false) => {
    if (typeof value === 'boolean') return value;
    if (typeof value === 'number') return value !== 0;
    if (typeof value === 'string') {
      const normalized = value.trim().toLowerCase();
      if (['true', '1', 'yes', 'on'].includes(normalized)) return true;
      if (['false', '0', 'no', 'off', ''].includes(normalized)) return false;
    }
    return fallback;
  };

  const asArray = (value) => Array.isArray(value) ? value : [];

  const stripMarkup = (value) => String(value || '')
    .replace(/```[\s\S]*?```/g, '')
    .replace(/`[^`]*`/g, '')
    .replace(/!?\[[^\]]*\]\([^)]*\)/g, '')
    .replace(/<[^>]+>/g, ' ')
    .replace(/#{1,6}\s/g, '')
    .replace(/[*_~`>|\-]/g, '')
    .replace(/\s+/g, ' ')
    .trim();

  const isEmojiImage = (src, attrs = '') => {
    const normalizedSrc = String(src || '').toLowerCase();
    const normalizedAttrs = String(attrs || '').toLowerCase();
    const classMatch = attrs.match(/\bclass=["']([^"']+)["']/i);
    if (classMatch) {
      const classes = classMatch[1].split(/\s+/);
      if (classes.includes('twemoji') || classes.includes('emoji')) return true;
    }
    if (/\bdata-(twemoji|emoji)\b/.test(normalizedAttrs)) return true;
    if (/twemoji|@twemoji|\/twemoji[/-]/.test(normalizedSrc)) return true;

    const altMatch = attrs.match(/\balt=["']([^"']+)["']/i);
    if (altMatch) {
      const alt = altMatch[1].trim();
      if (alt && Array.from(alt).length <= 4 && /\p{Extended_Pictographic}/u.test(alt)) {
        return true;
      }
    }

    return false;
  };

  const normalizeMarkdownImageUrl = (value) => {
    const raw = String(value || '').trim();
    if (!raw) return '';
    if (raw.startsWith('<') && raw.endsWith('>')) return raw.slice(1, -1).trim();
    return raw.split(/\s+(?=(?:"[^"]*"|'[^']*')$)/)[0].trim();
  };

  const extractThumbnail = (value) => {
    const content = String(value || '');
    const htmlImagePattern = /<img\b([^>]*)>/gi;
    let htmlMatch;
    while ((htmlMatch = htmlImagePattern.exec(content))) {
      const attrs = htmlMatch[1] || '';
      const srcMatch = attrs.match(/\bsrc=["']([^"']+)["']/i);
      const src = srcMatch ? srcMatch[1].trim() : '';
      if (src && !isEmojiImage(src, attrs)) return src;
    }

    const markdownImagePattern = /!\[[^\]]*\]\(([^)]+)\)/g;
    let markdownMatch;
    while ((markdownMatch = markdownImagePattern.exec(content))) {
      const src = normalizeMarkdownImageUrl(markdownMatch[1]);
      if (src && !isEmojiImage(src)) return src;
    }

    return null;
  };

  const normalizeSimpleUser = (user) => {
    if (!user) return null;
    return {
      id: asNumber(user.id),
      username: user.username || '',
      email: user.email || '',
      role: user.role || '',
      status: user.status || '',
      displayName: firstValue(user.displayName, user.display_name, user.username, ''),
      avatar: firstValue(user.avatar, user.avatar_url, ''),
      totpEnabled: asBoolean(firstValue(user.totpEnabled, user.totp_enabled), false),
      createdAt: firstValue(user.createdAt, user.created_at, ''),
      updatedAt: firstValue(user.updatedAt, user.updated_at, ''),
    };
  };

  const normalizeCategory = (category) => {
    if (!category) return null;
    return {
      id: asNumber(category.id),
      slug: category.slug || '',
      name: category.name || category.title || '',
      description: category.description || '',
      parentId: firstValue(category.parentId, category.parent_id, null),
      articleCount: asNumber(firstValue(category.articleCount, category.article_count), 0),
      createdAt: firstValue(category.createdAt, category.created_at, ''),
      updatedAt: firstValue(category.updatedAt, category.updated_at, ''),
    };
  };

  const normalizeTag = (tag) => {
    if (!tag) return null;
    return {
      id: asNumber(tag.id),
      slug: tag.slug || '',
      name: tag.name || tag.title || '',
      articleCount: asNumber(firstValue(tag.articleCount, tag.article_count), 0),
      createdAt: firstValue(tag.createdAt, tag.created_at, ''),
      updatedAt: firstValue(tag.updatedAt, tag.updated_at, ''),
    };
  };

  const normalizeArticleLink = (article) => {
    if (!article) return null;
    const content = firstValue(article.content, article.html, article.content_html, '');
    const thumbnail = firstValue(
      article.thumbnail,
      article.coverImage,
      article.cover_image,
      extractThumbnail(content)
    );
    return {
      id: asNumber(article.id),
      slug: article.slug || String(article.id || ''),
      title: article.title || '',
      thumbnail: thumbnail || null,
      url: article.url || '',
    };
  };

  const normalizeArticle = (article) => {
    if (!article) return null;
    const content = firstValue(article.content, '');
    const html = firstValue(article.html, article.content_html, '');
    const thumbnail = firstValue(
      article.thumbnail,
      article.coverImage,
      article.cover_image,
      extractThumbnail(html || content)
    );
    const metaSummary = article.meta && typeof article.meta === 'object' ? article.meta.summary : undefined;
    const excerpt = firstValue(article.summary, article.excerpt, metaSummary, stripMarkup(content || html).slice(0, 200));
    return {
      id: asNumber(article.id),
      slug: article.slug || String(article.id || ''),
      title: article.title || '',
      content,
      html,
      summary: firstValue(article.summary, excerpt),
      excerpt,
      thumbnail: thumbnail || null,
      coverImage: firstValue(article.coverImage, article.cover_image, thumbnail, null),
      authorId: firstValue(article.authorId, article.author_id, null),
      categoryId: firstValue(article.categoryId, article.category_id, null),
      status: article.status || '',
      author: normalizeSimpleUser(article.author),
      category: normalizeCategory(article.category),
      tags: asArray(article.tags).map(normalizeTag).filter(Boolean),
      createdAt: firstValue(article.createdAt, article.created_at, ''),
      updatedAt: firstValue(article.updatedAt, article.updated_at, ''),
      publishedAt: firstValue(article.publishedAt, article.published_at, article.createdAt, article.created_at, ''),
      scheduledAt: firstValue(article.scheduledAt, article.scheduled_at, null),
      viewCount: asNumber(firstValue(article.viewCount, article.view_count), 0),
      likeCount: asNumber(firstValue(article.likeCount, article.like_count), 0),
      commentCount: asNumber(firstValue(article.commentCount, article.comment_count), 0),
      wordCount: asNumber(firstValue(article.wordCount, article.word_count), 0),
      readingTime: asNumber(firstValue(article.readingTime, article.reading_time), 0),
      isPinned: asBoolean(firstValue(article.isPinned, article.is_pinned), false),
      pinOrder: asNumber(firstValue(article.pinOrder, article.pin_order), 0),
      prev: normalizeArticleLink(article.prev),
      next: normalizeArticleLink(article.next),
      related: asArray(article.related).map(normalizeArticleLink).filter(Boolean),
      toc: asArray(article.toc),
      meta: firstValue(article.meta, null),
      canonicalUrl: firstValue(article.canonicalUrl, article.canonical_url, null),
    };
  };

  const normalizeArticleList = (result = {}) => {
    const page = asNumber(result.page, 1);
    const pageSize = asNumber(firstValue(result.pageSize, result.page_size), 10);
    const total = asNumber(result.total, 0);
    const totalPages = asNumber(firstValue(result.totalPages, result.total_pages), pageSize > 0 ? Math.ceil(total / pageSize) : 0);
    return {
      articles: asArray(result.articles).map(normalizeArticle).filter(Boolean),
      total,
      page,
      pageSize,
      totalPages,
      hasMore: page * pageSize < total,
    };
  };

  const normalizePage = (page) => {
    if (!page) return null;
    return {
      id: asNumber(page.id),
      slug: page.slug || String(page.id || ''),
      title: page.title || '',
      content: firstValue(page.content, ''),
      html: firstValue(page.html, page.content_html, ''),
      status: page.status || '',
      source: page.source || '',
      createdAt: firstValue(page.createdAt, page.created_at, ''),
      updatedAt: firstValue(page.updatedAt, page.updated_at, ''),
    };
  };

  const normalizeFriendLink = (link) => {
    if (!link) return null;
    return {
      id: asNumber(link.id),
      name: firstValue(link.name, ''),
      url: firstValue(link.url, ''),
      logo: firstValue(link.logo, null),
      description: firstValue(link.description, null),
      category: firstValue(link.category, null),
      sortOrder: asNumber(firstValue(link.sortOrder, link.sort_order), 0),
      status: firstValue(link.status, ''),
      isRecommended: asBoolean(firstValue(link.isRecommended, link.is_recommended), false),
      createdAt: firstValue(link.createdAt, link.created_at, ''),
      updatedAt: firstValue(link.updatedAt, link.updated_at, ''),
    };
  };

  const normalizeAboutProfile = (payload = {}) => {
    const profile = payload.profile || payload;
    return {
      enabled: asBoolean(firstValue(profile.enabled, false), false),
      navEnabled: asBoolean(firstValue(profile.navEnabled, profile.nav_enabled), false),
      displayName: firstValue(profile.displayName, profile.display_name, ''),
      avatar: firstValue(profile.avatar, ''),
      headline: firstValue(profile.headline, ''),
      bio: firstValue(profile.bio, ''),
      location: firstValue(profile.location, ''),
      website: firstValue(profile.website, ''),
      socialLinks: asArray(firstValue(profile.socialLinks, profile.social_links)).map((link) => ({
        label: firstValue(link?.label, ''),
        url: firstValue(link?.url, ''),
        icon: firstValue(link?.icon, ''),
      })),
      timeline: asArray(profile.timeline).map((item) => ({
        title: firstValue(item?.title, ''),
        date: firstValue(item?.date, ''),
        description: firstValue(item?.description, ''),
      })),
      extraMarkdown: firstValue(profile.extraMarkdown, profile.extra_markdown, ''),
      extraHtml: firstValue(payload.extraHtml, payload.extra_html, ''),
    };
  };

  const normalizeComment = (comment) => {
    if (!comment) return null;
    return {
      id: asNumber(comment.id),
      articleId: firstValue(comment.articleId, comment.article_id, null),
      articleSlug: firstValue(comment.articleSlug, comment.article_slug, null),
      userId: firstValue(comment.userId, comment.user_id, null),
      parentId: firstValue(comment.parentId, comment.parent_id, null),
      nickname: firstValue(comment.nickname, comment.author?.username, null),
      email: firstValue(comment.email, null),
      content: comment.content || '',
      html: firstValue(comment.html, comment.content_html, ''),
      status: comment.status || '',
      createdAt: firstValue(comment.createdAt, comment.created_at, ''),
      updatedAt: firstValue(comment.updatedAt, comment.updated_at, ''),
      avatarUrl: firstValue(comment.avatarUrl, comment.avatar_url, comment.author?.avatar, ''),
      likeCount: asNumber(firstValue(comment.likeCount, comment.like_count), 0),
      isLiked: asBoolean(firstValue(comment.isLiked, comment.is_liked), false),
      isAuthor: asBoolean(firstValue(comment.isAuthor, comment.is_author), false),
      replies: asArray(comment.replies).map(normalizeComment).filter(Boolean),
    };
  };

  const normalizeNavItem = (item) => {
    if (!item) return null;
    const type = firstValue(item.type, item.navType, item.nav_type, 'external');
    const target = firstValue(item.target, item.url, '');
    return {
      id: asNumber(item.id),
      parentId: firstValue(item.parentId, item.parent_id, null),
      title: firstValue(item.title, item.name, ''),
      name: firstValue(item.name, item.title, ''),
      type,
      target,
      url: firstValue(item.url, target, ''),
      openNewTab: asBoolean(firstValue(item.openNewTab, item.open_new_tab, item.target === '_blank'), false),
      order: asNumber(firstValue(item.order, item.sortOrder, item.sort_order), 0),
      visible: asBoolean(firstValue(item.visible, true), true),
      children: asArray(item.children).map(normalizeNavItem).filter(Boolean),
    };
  };

  const getInjectedNavItems = () => {
    const injected = window.__NAV_ITEMS__;
    if (!Array.isArray(injected)) return null;
    return injected.map(normalizeNavItem).filter(Boolean);
  };

  const normalizeLikeResult = (result = {}) => ({
    success: asBoolean(firstValue(result.success, true), true),
    liked: asBoolean(result.liked, false),
    likeCount: asNumber(firstValue(result.likeCount, result.like_count), 0),
  });

  const normalizeSiteInfo = (data = {}) => ({
    version: data.version || '',
    name: firstValue(data.name, data.site_name, 'Noteva'),
    description: firstValue(data.description, data.site_description, ''),
    subtitle: firstValue(data.subtitle, data.site_subtitle, ''),
    logo: firstValue(data.logo, data.site_logo, '/logo.png'),
    footer: firstValue(data.footer, data.site_footer, ''),
    url: firstValue(data.url, data.site_url, ''),
    permalinkStructure: firstValue(data.permalinkStructure, data.permalink_structure, '/posts/{slug}'),
    emailVerificationEnabled: asBoolean(firstValue(data.emailVerificationEnabled, data.email_verification_enabled), false),
    demoMode: asBoolean(firstValue(data.demoMode, data.demo_mode), false),
    customCss: firstValue(data.customCss, data.custom_css, ''),
    customJs: firstValue(data.customJs, data.custom_js, ''),
    showToc: asBoolean(firstValue(data.showToc, data.show_toc), true),
    showPostNav: asBoolean(firstValue(data.showPostNav, data.show_post_nav), true),
    showRelatedPosts: asBoolean(firstValue(data.showRelatedPosts, data.show_related_posts), true),
    showComments: asBoolean(firstValue(data.showComments, data.show_comments), true),
    friendLinksNavEnabled: asBoolean(firstValue(data.friendLinksNavEnabled, data.friend_links_nav_enabled), true),
    aboutNavEnabled: asBoolean(firstValue(data.aboutNavEnabled, data.about_nav_enabled), false),
    stats: {
      totalArticles: asNumber(firstValue(data.stats?.totalArticles, data.stats?.total_articles), 0),
      totalCategories: asNumber(firstValue(data.stats?.totalCategories, data.stats?.total_categories), 0),
      totalTags: asNumber(firstValue(data.stats?.totalTags, data.stats?.total_tags), 0),
      totalComments: asNumber(firstValue(data.stats?.totalComments, data.stats?.total_comments), 0),
    },
  });

  const normalizeThemeInfo = (data = {}) => ({
    name: data.name || '',
    displayName: firstValue(data.displayName, data.display_name, data.name, ''),
    version: data.version || '',
    description: data.description || '',
    author: data.author || '',
    repository: data.repository || '',
    preview: data.preview || '',
    requiresNoteva: firstValue(data.requiresNoteva, data.requires_noteva, ''),
    compatible: firstValue(data.compatible, true),
    compatibilityMessage: firstValue(data.compatibilityMessage, data.compatibility_message, ''),
    config: data.config || {},
    hasSettings: asBoolean(firstValue(data.hasSettings, data.has_settings), false),
  });

  const coerceSettingValue = (value) => {
    if (typeof value !== 'string') return value;
    const trimmed = value.trim();
    if (trimmed === '') return '';
    const lowered = trimmed.toLowerCase();
    if (lowered === 'true') return true;
    if (lowered === 'false') return false;
    if (lowered === 'null') return null;
    if (/^-?(0|[1-9]\d*)(\.\d+)?$/.test(trimmed)) return Number(trimmed);
    if ((trimmed.startsWith('{') && trimmed.endsWith('}')) || (trimmed.startsWith('[') && trimmed.endsWith(']'))) {
      try { return JSON.parse(trimmed); } catch { return value; }
    }
    return value;
  };

  const normalizeSettings = (payload = {}) => {
    const values = payload && typeof payload === 'object' && payload.values ? payload.values : payload;
    const normalized = {};
    for (const [key, value] of Object.entries(values || {})) {
      normalized[key] = coerceSettingValue(value);
    }
    return normalized;
  };

  const normalizeArchiveEntry = (entry = {}) => {
    const month = entry.month || '';
    const [yearText, monthText] = String(month).split('-');
    return {
      month,
      year: asNumber(firstValue(entry.year, yearText), 0),
      monthNumber: asNumber(firstValue(entry.monthNumber, entry.month_number, monthText), 0),
      count: asNumber(entry.count, 0),
    };
  };

  // ============================================
  // 站点 API
  // ============================================
  const site = {
    _info: null,
    _nav: null,
    _permalinkStructure: '/posts/{slug}',

    async getInfo() {
      if (this._info) return this._info;
      this._info = normalizeSiteInfo(await api.get('/site/info'));
      this._permalinkStructure = this._info.permalinkStructure || '/posts/{slug}';
      return this._info;
    },

    async getNav() {
      if (this._nav) return this._nav;
      const injected = getInjectedNavItems();
      if (injected) {
        this._nav = injected;
        return this._nav;
      }
      const result = await api.get('/nav');
      this._nav = asArray(result.items).map(normalizeNavItem).filter(Boolean);
      return this._nav;
    },

    async refresh() {
      this._info = null;
      this._nav = null;
      return this.getInfo();
    },
  };

  const urls = {
    article(article) {
      if (!article) return '/posts/';
      const structure = site._permalinkStructure || '/posts/{slug}';
      return structure
        .replace('{id}', encodeURIComponent(String(article.id || '')))
        .replace('{slug}', encodeURIComponent(String(article.slug || article.id || '')));
    },

    category(category) {
      const slug = typeof category === 'string' ? category : category?.slug;
      return `/categories?c=${encodeURIComponent(slug || '')}`;
    },

    tag(tag) {
      const slug = typeof tag === 'string' ? tag : tag?.slug;
      return `/tags?t=${encodeURIComponent(slug || '')}`;
    },

    page(page) {
      const slug = typeof page === 'string' ? page : page?.slug;
      return `/${String(slug || '').replace(/^\/+/, '')}`;
    },

    asset(path) {
      return `/${String(path || '').replace(/^\/+/, '')}`;
    },

    upload(path) {
      const normalized = String(path || '').replace(/^\/+/, '');
      return normalized.startsWith('uploads/') ? `/${normalized}` : `/uploads/${normalized}`;
    },
  };

  const theme = {
    _info: null,
    _config: null,
    _settings: null,

    async getInfo() {
      if (this._info) return this._info;
      this._info = normalizeThemeInfo(await api.get('/theme/info'));
      return this._info;
    },

    async getConfig(key) {
      if (!this._config) {
        const result = await api.get('/theme/config');
        this._config = result.config || {};
        events.emit('theme:config:loaded', this._config);
      }
      return key ? this._config[key] : this._config;
    },

    async getSettings(key) {
      if (!this._settings) {
        try {
          this._settings = normalizeSettings(await api.get('/theme/settings'));
        } catch (e) {
          console.warn('[Noteva] Failed to load theme settings:', e);
          this._settings = {};
        }
      }
      return key ? this._settings[key] : this._settings;
    },

    async getSetting(key, defaultValue = undefined) {
      const value = await this.getSettings(key);
      return value === undefined ? defaultValue : value;
    },

    async refreshSettings() {
      this._settings = null;
      return this.getSettings();
    },

    _setConfig(config) {
      this._config = config || {};
      events.emit('theme:config:change', this._config);
    },
  };

  // ============================================
  // 文章 API
  // ============================================
  const articles = {
    async list(params = {}) {
      const queryParams = {
        page: params.page || 1,
        page_size: params.pageSize || 10,
        published_only: true,
      };
      // 只添加有值的可选参数
      if (params.category) queryParams.category = params.category;
      if (params.tag) queryParams.tag = params.tag;
      if (params.keyword) queryParams.keyword = params.keyword;
      if (params.sort) queryParams.sort = params.sort;

      return normalizeArticleList(await api.get('/articles', queryParams));
    },

    async popular(params = {}) {
      const limit = Math.min(Math.max(asNumber(firstValue(params.limit, params.pageSize), 5), 1), 20);
      const result = await this.list({
        page: 1,
        pageSize: limit,
        category: params.category,
        tag: params.tag,
        sort: params.sort || 'views',
      });
      return result.articles.slice(0, limit);
    },

    async get(slug) {
      const article = normalizeArticle(await api.get(`/articles/${slug}`));
      // 触发文章查看钩子
      const processed = hooks.trigger('article_view', article);
      page.set({
        type: 'article',
        articleId: processed?.id || null,
        article: processed || null,
        pageId: null,
        customPage: null,
      });
      events.emit('article:view', processed);
      return processed;
    },

    async related(slug, params = {}) {
      const article = normalizeArticle(await api.get(`/articles/${slug}`));
      return asArray(article?.related).slice(0, params.limit || 5);
    },

    async archives() {
      const result = await api.get('/articles/archives');
      return asArray(result).map(normalizeArchiveEntry);
    },

    /**
     * 增加文章浏览计数
     * @param {number} articleId - 文章 ID
     */
    async incrementView(articleId) {
      try {
        await api.post(`/view/${articleId}`);
      } catch (e) {
        // 浏览计数失败不影响用户体验
      }
    },
  };

  // ============================================
  // 交互 API（点赞）
  // ============================================
  const interactions = {
    /**
     * 点赞或取消点赞
     * @param {'article'|'comment'} targetType - 目标类型
     * @param {number} targetId - 目标 ID
     * @returns {Promise<{liked: boolean, likeCount: number}>}
     */
    async like(targetType, targetId) {
      return normalizeLikeResult(await api.post('/like', { target_type: targetType, target_id: targetId }));
    },

    /**
     * 检查当前用户是否已点赞
     * @param {'article'|'comment'} targetType - 目标类型
     * @param {number} targetId - 目标 ID
     * @returns {Promise<{liked: boolean}>}
     */
    async checkLike(targetType, targetId) {
      return normalizeLikeResult(await api.get('/like/check', { target_type: targetType, target_id: targetId }));
    },
  };

  // ============================================
  // 搜索工具
  // ============================================
  const search = {
    /**
     * 高亮文本中的关键词
     * @param {string} text - 原始文本
     * @param {string} keyword - 搜索关键词
     * @returns {string} 带 <mark> 标签的 HTML 字符串
     */
    highlight(text, keyword) {
      if (!text || !keyword) return text || '';
      const escaped = keyword.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
      const regex = new RegExp(`(${escaped})`, 'gi');
      return text.replace(regex, '<mark class="noteva-highlight">$1</mark>');
    },
  };

  // ============================================
  // 页面 API
  // ============================================
  const pages = {
    async list() {
      const result = await api.get('/pages');
      return asArray(result.pages).map(normalizePage).filter(Boolean);
    },

    async get(slug) {
      const result = await api.get(`/page/${slug}`);
      const customPage = normalizePage(result.page || result);
      page.set({
        type: 'page',
        articleId: null,
        article: null,
        pageId: customPage?.id || null,
        customPage,
      });
      return customPage;
    },
  };

  // ============================================
  // 友联 API
  // ============================================
  const friendLinks = {
    async list(params = {}) {
      const result = await api.get('/friend-links', params);
      return asArray(result.links || result).map(normalizeFriendLink).filter(Boolean);
    },
  };

  // ============================================
  // About API
  // ============================================
  const about = {
    async get() {
      return normalizeAboutProfile(await api.get('/about'));
    },
  };

  // ============================================
  // 分类 API
  // ============================================
  const categories = {
    async list() {
      const result = await api.get('/categories');
      return asArray(result.categories).map(normalizeCategory).filter(Boolean);
    },

    async get(slug) {
      const categories = await this.list();
      return categories.find(category => category.slug === slug || String(category.id) === String(slug)) || null;
    },
  };

  // ============================================
  // 标签 API
  // ============================================
  const tags = {
    async list() {
      const result = await api.get('/tags');
      return asArray(result.tags).map(normalizeTag).filter(Boolean);
    },

    async get(slug) {
      const tags = await this.list();
      return tags.find(tag => tag.slug === slug || String(tag.id) === String(slug)) || null;
    },
  };

  // ============================================
  // 评论 API
  // ============================================
  const comments = {
    async list(articleId) {
      const result = await api.get(`/comments/${articleId}`);
      const commentList = asArray(result.comments || result).map(normalizeComment).filter(Boolean);
      // 触发评论显示前钩子
      return hooks.trigger('comment_before_display', commentList);
    },

    async create(data) {
      // 触发评论创建前钩子
      const processedData = hooks.trigger('comment_before_create', data);
      const captchaToken = processedData.captchaToken;

      const comment = await api.post(`/comments`, {
        article_id: processedData.articleId,
        content: processedData.content,
        parent_id: processedData.parentId,
        nickname: processedData.nickname,
        email: processedData.email,
        captcha_token: captchaToken,
      });

      // 触发评论创建后钩子
      const normalized = normalizeComment(comment.comment || comment);
      hooks.trigger('comment_after_create', normalized, {
        articleId: data.articleId,
        parentId: data.parentId,
      });
      events.emit('comment:create', normalized);

      return normalized;
    },

    async recent(limit = 10) {
      const safeLimit = Math.min(Math.max(asNumber(limit, 10), 1), 50);
      const result = await api.get(`/comments/recent`, { limit: safeLimit });
      return asArray(result.comments || result).map(normalizeComment).filter(Boolean);
    },
  };

  const POW_K = [
    0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
    0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
    0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
    0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
    0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
    0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
    0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
    0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
  ];

  function powRotr(value, bits) {
    return (value >>> bits) | (value << (32 - bits));
  }

  function powSha256(input) {
    const bytes = [];
    for (let i = 0; i < input.length; i += 1) bytes.push(input.charCodeAt(i) & 0xff);
    const bitLength = bytes.length * 8;
    bytes.push(0x80);
    while ((bytes.length % 64) !== 56) bytes.push(0);
    bytes.push(0, 0, 0, 0, (bitLength >>> 24) & 0xff, (bitLength >>> 16) & 0xff, (bitLength >>> 8) & 0xff, bitLength & 0xff);

    const h = [
      0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a,
      0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
    ];
    const w = new Array(64);

    for (let offset = 0; offset < bytes.length; offset += 64) {
      for (let i = 0; i < 16; i += 1) {
        const j = offset + i * 4;
        w[i] = ((bytes[j] << 24) | (bytes[j + 1] << 16) | (bytes[j + 2] << 8) | bytes[j + 3]) >>> 0;
      }
      for (let i = 16; i < 64; i += 1) {
        const s0 = powRotr(w[i - 15], 7) ^ powRotr(w[i - 15], 18) ^ (w[i - 15] >>> 3);
        const s1 = powRotr(w[i - 2], 17) ^ powRotr(w[i - 2], 19) ^ (w[i - 2] >>> 10);
        w[i] = (w[i - 16] + s0 + w[i - 7] + s1) >>> 0;
      }

      let [a, b, c, d, e, f, g, hh] = h;
      for (let i = 0; i < 64; i += 1) {
        const s1 = powRotr(e, 6) ^ powRotr(e, 11) ^ powRotr(e, 25);
        const ch = (e & f) ^ (~e & g);
        const temp1 = (hh + s1 + ch + POW_K[i] + w[i]) >>> 0;
        const s0 = powRotr(a, 2) ^ powRotr(a, 13) ^ powRotr(a, 22);
        const maj = (a & b) ^ (a & c) ^ (b & c);
        const temp2 = (s0 + maj) >>> 0;
        hh = g;
        g = f;
        f = e;
        e = (d + temp1) >>> 0;
        d = c;
        c = b;
        b = a;
        a = (temp1 + temp2) >>> 0;
      }

      h[0] = (h[0] + a) >>> 0;
      h[1] = (h[1] + b) >>> 0;
      h[2] = (h[2] + c) >>> 0;
      h[3] = (h[3] + d) >>> 0;
      h[4] = (h[4] + e) >>> 0;
      h[5] = (h[5] + f) >>> 0;
      h[6] = (h[6] + g) >>> 0;
      h[7] = (h[7] + hh) >>> 0;
    }

    const digest = [];
    for (const word of h) {
      digest.push((word >>> 24) & 0xff, (word >>> 16) & 0xff, (word >>> 8) & 0xff, word & 0xff);
    }
    return digest;
  }

  function powHasLeadingZeroBits(digest, bits) {
    let remaining = Number(bits || 0);
    for (const byte of digest) {
      if (remaining <= 0) return true;
      if (remaining >= 8) {
        if (byte !== 0) return false;
        remaining -= 8;
      } else {
        const mask = (0xff << (8 - remaining)) & 0xff;
        return (byte & mask) === 0;
      }
    }
    return remaining <= 0;
  }

  function powMatches(challenge, solution) {
    const action = challenge.action || 'comment';
    const bits = asNumber(firstValue(challenge.leadingZeroBits, challenge.leading_zero_bits), 16);
    const payload = `${challenge.id}:${challenge.nonce}:${action}:${solution}`;
    return powHasLeadingZeroBits(powSha256(payload), bits);
  }

  async function solvePowFallback(challenge, options = {}) {
    const startedAt = Date.now();
    const batchSize = asNumber(options.batchSize, 1024);
    let candidate = asNumber(options.start, 0);

    while (true) {
      for (let i = 0; i < batchSize; i += 1) {
        const solution = String(candidate);
        if (powMatches(challenge, solution)) {
          return {
            solution,
            elapsedMs: Date.now() - startedAt,
            attempts: candidate + 1,
          };
        }
        candidate += 1;
      }
      if (typeof options.onProgress === 'function') {
        options.onProgress({ attempts: candidate, elapsedMs: Date.now() - startedAt });
      }
      await new Promise((resolve) => setTimeout(resolve, 0));
    }
  }

  const POW_WORKER_SOURCE = `
    const POW_K = ${JSON.stringify(POW_K)};
    ${powRotr.toString()}
    ${powSha256.toString()}
    ${powHasLeadingZeroBits.toString()}
    function firstValue(){ for (let i = 0; i < arguments.length; i += 1) if (arguments[i] !== undefined && arguments[i] !== null) return arguments[i]; }
    function asNumber(value, fallback = 0) { const parsed = Number(value); return Number.isFinite(parsed) ? parsed : fallback; }
    ${powMatches.toString()}
    self.onmessage = function(event) {
      const challenge = event.data.challenge;
      const batchSize = Number(event.data.batchSize || 2048);
      let candidate = Number(event.data.start || 0);
      const startedAt = Date.now();
      for (;;) {
        for (let i = 0; i < batchSize; i += 1) {
          const solution = String(candidate);
          if (powMatches(challenge, solution)) {
            self.postMessage({ type: 'done', solution, elapsedMs: Date.now() - startedAt, attempts: candidate + 1 });
            return;
          }
          candidate += 1;
        }
        self.postMessage({ type: 'progress', attempts: candidate, elapsedMs: Date.now() - startedAt });
      }
    };
  `;

  function solvePowInWorker(challenge, options = {}) {
    return new Promise((resolve, reject) => {
      if (typeof Worker === 'undefined' || typeof Blob === 'undefined' || typeof URL === 'undefined') {
        reject(new Error('Web Worker is not available'));
        return;
      }

      const blobUrl = URL.createObjectURL(new Blob([POW_WORKER_SOURCE], { type: 'application/javascript' }));
      const worker = new Worker(blobUrl);
      const timeoutMs = asNumber(options.timeoutMs, 30000);
      let settled = false;
      const cleanup = () => {
        worker.terminate();
        URL.revokeObjectURL(blobUrl);
      };
      const timeout = setTimeout(() => {
        if (settled) return;
        settled = true;
        cleanup();
        reject(new Error('Captcha verification timed out'));
      }, timeoutMs);

      worker.onmessage = (event) => {
        if (event.data?.type === 'progress') {
          if (typeof options.onProgress === 'function') options.onProgress(event.data);
          return;
        }
        if (event.data?.type === 'done') {
          if (settled) return;
          settled = true;
          clearTimeout(timeout);
          cleanup();
          resolve(event.data);
        }
      };
      worker.onerror = (error) => {
        if (settled) return;
        settled = true;
        clearTimeout(timeout);
        cleanup();
        reject(error);
      };
      worker.postMessage({
        challenge,
        start: options.start || 0,
        batchSize: options.batchSize || 2048,
      });
    });
  }

  async function solvePowChallenge(challenge, options = {}) {
    try {
      return await solvePowInWorker(challenge, options);
    } catch {
      return solvePowFallback(challenge, options);
    }
  }

  const POW_CAPTCHA_DEFAULT_LABELS = {
    verifying: 'Verifying...',
    verified: 'Verification complete',
    retry: 'Verification failed, click to retry',
    expired: 'Verification expired, click to retry',
    required: 'Click to verify you are human',
    brand: 'Noteva',
  };

  const POW_CAPTCHA_ZH_LABELS = {
    verifying: '正在验证...',
    verified: '验证完成',
    retry: '验证失败，点击重试',
    expired: '验证已过期，点击重新验证',
    required: '点击验证你是真人',
    brand: 'Noteva',
  };

  function isBrokenPowCaptchaLabel(value) {
    const text = String(value || '').trim();
    return !text || /^\?+$/.test(text) || /^comment\.captcha/.test(text);
  }

  function getPowCaptchaLabels(options = {}) {
    const locale = String(
      options.locale ||
        (typeof document !== 'undefined' ? document.documentElement?.lang : '') ||
        (typeof navigator !== 'undefined' ? navigator.language : '') ||
        ''
    );
    const defaults = locale.toLowerCase().startsWith('zh')
      ? POW_CAPTCHA_ZH_LABELS
      : POW_CAPTCHA_DEFAULT_LABELS;
    const labels = options.labels || {};
    const merged = { ...defaults };

    for (const key of Object.keys(defaults)) {
      merged[key] = isBrokenPowCaptchaLabel(labels[key])
        ? defaults[key]
        : String(labels[key]);
    }

    return merged;
  }

  function applyCapWidgetI18n(capWidget, options = {}) {
    const labels = options.labels || {};
    const mappings = {
      'data-cap-i18n-initial-state': labels.initial || labels.required,
      'data-cap-i18n-verifying-label': labels.verifying,
      'data-cap-i18n-solved-label': labels.verified,
      'data-cap-i18n-error-label': labels.error || labels.retry,
      'data-cap-i18n-troubleshooting-label': labels.troubleshooting,
      'data-cap-i18n-wasm-disabled': labels.wasmDisabled,
      'data-cap-i18n-verify-aria-label': labels.verifyAria || labels.required,
      'data-cap-i18n-verifying-aria-label': labels.verifyingAria || labels.verifying,
      'data-cap-i18n-verified-aria-label': labels.verifiedAria || labels.verified,
      'data-cap-i18n-required-label': labels.required,
      'data-cap-i18n-error-aria-label': labels.errorAria || labels.error || labels.retry,
    };

    if (options.locale) {
      capWidget.setAttribute('lang', String(options.locale));
    }

    Object.entries(mappings).forEach(([attribute, value]) => {
      const text = String(value || '').trim();
      if (text && !isBrokenPowCaptchaLabel(text)) {
        capWidget.setAttribute(attribute, text);
      }
    });
  }

  function injectPowCaptchaStyle() {
    if (typeof document === 'undefined' || document.getElementById('noteva-pow-captcha-style')) return;

    const style = document.createElement('style');
    style.id = 'noteva-pow-captcha-style';
    style.textContent = `
      .noteva-pow-captcha {
        width: min(100%, 28rem);
        min-height: 4rem;
        display: flex;
        align-items: center;
        gap: 1rem;
        border: 1px solid hsl(var(--border, 214.3 31.8% 91.4%));
        border-radius: 1rem;
        background: hsl(var(--card, 0 0% 100%));
        color: hsl(var(--foreground, 222.2 84% 4.9%));
        padding: 0.78rem 1rem;
        font: inherit;
        text-align: left;
        box-shadow: 0 1px 2px hsl(var(--foreground, 222.2 84% 4.9%) / 0.04);
        transition:
          border-color 160ms ease,
          background-color 160ms ease,
          box-shadow 160ms ease,
          transform 160ms ease;
      }

      .noteva-pow-captcha:not(:disabled) {
        cursor: pointer;
      }

      .noteva-pow-captcha:not(:disabled):hover {
        border-color: hsl(var(--primary, 222.2 47.4% 11.2%) / 0.45);
        box-shadow: 0 12px 32px hsl(var(--foreground, 222.2 84% 4.9%) / 0.08);
        transform: translateY(-1px);
      }

      .noteva-pow-captcha:focus-visible {
        outline: 2px solid hsl(var(--ring, 222.2 84% 4.9%) / 0.72);
        outline-offset: 2px;
      }

      .noteva-pow-captcha[data-status="solving"] {
        cursor: wait;
        border-color: hsl(var(--primary, 222.2 47.4% 11.2%) / 0.42);
      }

      .noteva-pow-captcha[data-status="ready"] {
        border-color: hsl(142 68% 42% / 0.42);
        background: hsl(142 68% 42% / 0.06);
      }

      .noteva-pow-captcha[data-status="error"],
      .noteva-pow-captcha[data-status="expired"] {
        border-color: hsl(var(--destructive, 0 84.2% 60.2%) / 0.42);
        background: hsl(var(--destructive, 0 84.2% 60.2%) / 0.06);
      }

      .noteva-pow-captcha-box {
        width: 1.9rem;
        height: 1.9rem;
        flex: none;
        display: grid;
        place-items: center;
        border: 1px solid hsl(var(--border, 214.3 31.8% 91.4%));
        border-radius: 0.48rem;
        background: hsl(var(--background, 0 0% 100%));
        color: hsl(var(--foreground, 222.2 84% 4.9%));
        transition:
          border-color 160ms ease,
          background-color 160ms ease,
          color 160ms ease;
      }

      .noteva-pow-captcha[data-status="ready"] .noteva-pow-captcha-box {
        border-color: hsl(142 68% 42% / 0.58);
        background: hsl(142 68% 42%);
        color: white;
      }

      .noteva-pow-captcha[data-status="error"] .noteva-pow-captcha-box,
      .noteva-pow-captcha[data-status="expired"] .noteva-pow-captcha-box {
        border-color: hsl(var(--destructive, 0 84.2% 60.2%) / 0.56);
        color: hsl(var(--destructive, 0 84.2% 60.2%));
      }

      .noteva-pow-captcha-check {
        width: 0.48rem;
        height: 0.82rem;
        border-right: 2px solid currentColor;
        border-bottom: 2px solid currentColor;
        transform: rotate(42deg) translateY(-0.06rem);
      }

      .noteva-pow-captcha-spinner {
        width: 1rem;
        height: 1rem;
        border: 2px solid currentColor;
        border-right-color: transparent;
        border-radius: 999px;
        animation: notevaPowSpin 720ms linear infinite;
      }

      .noteva-pow-captcha-alert {
        font-size: 1rem;
        font-weight: 700;
        line-height: 1;
      }

      .noteva-pow-captcha-label {
        min-width: 0;
        flex: 1;
        font-size: 0.95rem;
        font-weight: 600;
        line-height: 1.35;
      }

      .noteva-pow-captcha-brand {
        flex: none;
        color: hsl(var(--muted-foreground, 215.4 16.3% 46.9%));
        font-size: 0.78rem;
        text-decoration: underline;
        text-underline-offset: 0.16rem;
      }

      @keyframes notevaPowSpin {
        to {
          transform: rotate(360deg);
        }
      }

      @media (prefers-reduced-motion: reduce) {
        .noteva-pow-captcha,
        .noteva-pow-captcha-box {
          transition-duration: 0.01ms;
        }

        .noteva-pow-captcha-spinner {
          animation-duration: 1.6s;
        }
      }
    `;
    document.head.appendChild(style);
  }

  const captcha = {
    _config: null,
    _scriptPromises: {},
    _widgets: new Map(),

    _clearPowTimer(widget) {
      if (widget?.expireTimer) {
        clearTimeout(widget.expireTimer);
        widget.expireTimer = null;
      }
    },

    _expirePowWidget(element, widget) {
      if (!widget || widget.status !== 'ready') return;

      widget.token = '';
      this._clearPowTimer(widget);
      this._setPowWidgetStatus(element, widget, 'expired');
      if (typeof widget.options?.expiredCallback === 'function') {
        widget.options.expiredCallback();
      }
    },

    _setPowWidgetStatus(element, widget, status) {
      if (!element || !widget?.button) return;

      const labels = widget.labels || POW_CAPTCHA_DEFAULT_LABELS;
      const labelByStatus = {
        idle: labels.required,
        required: labels.required,
        solving: labels.verifying,
        ready: labels.verified,
        expired: labels.expired,
        error: labels.retry,
      };

      widget.status = status;
      widget.button.dataset.status = status;
      widget.button.disabled = status === 'solving';
      widget.button.setAttribute('aria-busy', status === 'solving' ? 'true' : 'false');
      widget.label.textContent = labelByStatus[status] || labels.required;
      widget.icon.textContent = '';

      if (status === 'ready') {
        const check = document.createElement('span');
        check.className = 'noteva-pow-captcha-check';
        check.setAttribute('aria-hidden', 'true');
        widget.icon.appendChild(check);
      } else if (status === 'solving') {
        const spinner = document.createElement('span');
        spinner.className = 'noteva-pow-captcha-spinner';
        spinner.setAttribute('aria-hidden', 'true');
        widget.icon.appendChild(spinner);
      } else if (status === 'error' || status === 'expired') {
        const alert = document.createElement('span');
        alert.className = 'noteva-pow-captcha-alert';
        alert.setAttribute('aria-hidden', 'true');
        alert.textContent = '!';
        widget.icon.appendChild(alert);
      }
    },

    _renderPowWidget(element, widget, options = {}) {
      injectPowCaptchaStyle();
      element.innerHTML = '';

      const labels = getPowCaptchaLabels(options);
      const button = document.createElement('button');
      button.type = 'button';
      button.className = 'noteva-pow-captcha';
      button.dataset.status = 'idle';
      button.setAttribute('aria-live', 'polite');

      const icon = document.createElement('span');
      icon.className = 'noteva-pow-captcha-box';
      icon.setAttribute('aria-hidden', 'true');

      const label = document.createElement('span');
      label.className = 'noteva-pow-captcha-label';

      const brand = document.createElement('span');
      brand.className = 'noteva-pow-captcha-brand';
      brand.textContent = labels.brand || 'Noteva';

      button.append(icon, label, brand);
      element.appendChild(button);

      Object.assign(widget, {
        action: options.action || 'comment',
        button,
        icon,
        label,
        labels,
        options,
        status: 'idle',
        token: '',
      });

      button.addEventListener('click', () => {
        if (widget.status === 'solving' || widget.status === 'ready') return;
        this.solve({
          ...widget.options,
          action: widget.action,
          container: element,
          force: true,
        }).catch(() => {});
      });

      this._setPowWidgetStatus(element, widget, 'idle');
    },

    async getConfig() {
      if (!this._config) {
        const config = await api.get('/captcha/config');
        this._config = {
          ...config,
          provider: config.provider || 'none',
          siteKey: config.siteKey || config.site_key || '',
          site_key: config.site_key || config.siteKey || '',
          capBaseUrl: firstValue(config.capBaseUrl, config.cap_base_url, ''),
          cap_base_url: firstValue(config.cap_base_url, config.capBaseUrl, ''),
          capEndpoint: firstValue(config.capEndpoint, config.cap_endpoint, ''),
          cap_endpoint: firstValue(config.cap_endpoint, config.capEndpoint, ''),
          pow: config.pow ? {
            difficulty: config.pow.difficulty || 'normal',
            leadingZeroBits: asNumber(firstValue(config.pow.leadingZeroBits, config.pow.leading_zero_bits), 16),
            challengeTtlSeconds: asNumber(firstValue(config.pow.challengeTtlSeconds, config.pow.challenge_ttl_seconds), 120),
            tokenTtlSeconds: asNumber(firstValue(config.pow.tokenTtlSeconds, config.pow.token_ttl_seconds), 300),
            autoSolve: asBoolean(firstValue(config.pow.autoSolve, config.pow.auto_solve), true),
          } : null,
        };
      }
      return this._config;
    },

    async loadScript(provider) {
      if (provider !== 'turnstile' && provider !== 'hcaptcha' && provider !== 'cap') return Promise.resolve();
      if (this._scriptPromises[provider]) return this._scriptPromises[provider];
      const scripts = provider === 'turnstile'
        ? [{ src: 'https://challenges.cloudflare.com/turnstile/v0/api.js?render=explicit' }]
        : provider === 'hcaptcha'
          ? [{ src: 'https://js.hcaptcha.com/1/api.js?render=explicit' }]
          : [
              { src: 'https://cdn.jsdelivr.net/npm/@cap.js/widget@3' },
              { src: 'https://cdn.jsdelivr.net/npm/cap-widget', type: 'module' },
            ];
      this._scriptPromises[provider] = new Promise((resolve, reject) => {
        if (
          (provider === 'turnstile' && window.turnstile) ||
          (provider === 'hcaptcha' && window.hcaptcha) ||
          (provider === 'cap' && typeof customElements !== 'undefined' && customElements.get?.('cap-widget'))
        ) {
          resolve();
          return;
        }

        let index = 0;
        let settled = false;
        const resolveOnce = () => {
          if (settled) return;
          settled = true;
          resolve();
        };
        const rejectOnce = (error) => {
          if (settled) return;
          settled = true;
          reject(error);
        };
        const loadNext = () => {
          if (settled) return;
          const current = scripts[index];
          if (!current) {
            rejectOnce(new Error(`Failed to load ${provider} captcha script`));
            return;
          }

          const script = document.createElement('script');
          script.src = current.src;
          if (current.type) script.type = current.type;
          script.async = true;
          script.defer = true;
          script.onload = () => {
            if (provider !== 'cap' || typeof customElements === 'undefined') {
              resolveOnce();
              return;
            }

            if (customElements.get?.('cap-widget')) {
              resolveOnce();
              return;
            }

            const timeout = setTimeout(() => {
              script.remove();
              index += 1;
              loadNext();
            }, 1500);

            customElements.whenDefined('cap-widget').then(() => {
              clearTimeout(timeout);
              resolveOnce();
            }).catch(() => {
              clearTimeout(timeout);
              script.remove();
              index += 1;
              loadNext();
            });
          };
          script.onerror = () => {
            script.remove();
            index += 1;
            loadNext();
          };
          document.head.appendChild(script);
        };

        loadNext();
      });
      return this._scriptPromises[provider];
    },

    _renderCapWidget(element, widget, options = {}) {
      const capEndpoint = firstValue(
        options.capEndpoint,
        options.cap_endpoint,
        widget.capEndpoint,
        ''
      );
      if (!capEndpoint) return null;

      element.innerHTML = '';
      const capWidget = document.createElement('cap-widget');
      capWidget.setAttribute('data-cap-api-endpoint', capEndpoint);
      if (options.theme) capWidget.setAttribute('data-theme', options.theme);
      if (options.size) capWidget.setAttribute('data-size', options.size);
      applyCapWidgetI18n(capWidget, options);

      const readCapToken = (event) => {
        const detail = event?.detail;
        return typeof detail === 'string'
          ? detail
          : firstValue(
              detail?.token,
              detail?.response,
              detail?.value,
              detail?.solution,
              event?.token,
              event?.response,
              capWidget.token,
              capWidget.value,
              capWidget.getAttribute?.('token'),
              capWidget.getAttribute?.('value'),
              ''
            );
      };
      const handleSolve = (event) => {
        const token = readCapToken(event);
        widget.token = token || '';
        if (widget.token && typeof options.callback === 'function') {
          options.callback(widget.token);
        }
      };
      const handleError = (event) => {
        widget.token = '';
        if (typeof options.errorCallback === 'function') {
          options.errorCallback(event?.detail || event);
        }
      };
      const handleExpired = () => {
        widget.token = '';
        if (typeof options.expiredCallback === 'function') {
          options.expiredCallback();
        }
      };
      const handleProgress = (event) => {
        if (typeof options.progressCallback === 'function') {
          options.progressCallback(event?.detail || event);
        }
      };
      const handleReset = () => {
        widget.token = '';
        if (typeof options.resetCallback === 'function') {
          options.resetCallback();
        }
      };

      capWidget.addEventListener('solve', handleSolve);
      capWidget.addEventListener('solved', handleSolve);
      capWidget.addEventListener('success', handleSolve);
      capWidget.addEventListener('verified', handleSolve);
      capWidget.addEventListener('verify', handleSolve);
      capWidget.addEventListener('token', handleSolve);
      capWidget.addEventListener('error', handleError);
      capWidget.addEventListener('expire', handleExpired);
      capWidget.addEventListener('expired', handleExpired);
      capWidget.addEventListener('progress', handleProgress);
      capWidget.addEventListener('challenge', handleProgress);
      capWidget.addEventListener('reset', handleReset);
      element.appendChild(capWidget);

      Object.assign(widget, {
        id: capWidget,
        element: capWidget,
        token: '',
        cleanup() {
          capWidget.removeEventListener('solve', handleSolve);
          capWidget.removeEventListener('solved', handleSolve);
          capWidget.removeEventListener('success', handleSolve);
          capWidget.removeEventListener('verified', handleSolve);
          capWidget.removeEventListener('verify', handleSolve);
          capWidget.removeEventListener('token', handleSolve);
          capWidget.removeEventListener('error', handleError);
          capWidget.removeEventListener('expire', handleExpired);
          capWidget.removeEventListener('expired', handleExpired);
          capWidget.removeEventListener('progress', handleProgress);
          capWidget.removeEventListener('challenge', handleProgress);
          capWidget.removeEventListener('reset', handleReset);
        },
      });

      return capWidget;
    },

    async render(container, options = {}) {
      const element = typeof container === 'string' ? document.querySelector(container) : container;
      if (!element) return null;
      const config = await this.getConfig();
      if (!config.enabled) {
        element.innerHTML = '';
        return null;
      }
      const provider = options.provider || config.provider;
      const siteKey = options.siteKey || config.siteKey || config.site_key;
      if (!provider || provider === 'none') return null;
      if (provider === 'noteva_pow') {
        const previous = this._widgets.get(element);
        if (previous?.provider === 'noteva_pow') this._clearPowTimer(previous);
        const widget = { provider, id: null, token: '' };
        this._widgets.set(element, widget);
        this._renderPowWidget(element, widget, options);
        return { provider, element };
      }
      if (provider === 'cap') {
        const previous = this._widgets.get(element);
        if (previous?.cleanup) previous.cleanup();
        const capEndpoint = firstValue(
          options.capEndpoint,
          options.cap_endpoint,
          config.capEndpoint,
          config.cap_endpoint
        );
        if (!capEndpoint) return null;
        await this.loadScript(provider);
        const widget = { provider, id: null, token: '', capEndpoint };
        this._widgets.set(element, widget);
        return this._renderCapWidget(element, widget, { ...options, capEndpoint });
      }
      if (!siteKey) return null;
      await this.loadScript(provider);
      const widgetOptions = {
        sitekey: siteKey,
        callback: options.callback,
        'expired-callback': options.expiredCallback,
        'error-callback': options.errorCallback,
      };
      const id = provider === 'turnstile'
        ? window.turnstile.render(element, widgetOptions)
        : window.hcaptcha.render(element, widgetOptions);
      this._widgets.set(element, { provider, id });
      return id;
    },

    getToken(container) {
      const element = typeof container === 'string' ? document.querySelector(container) : container;
      const widget = element ? this._widgets.get(element) : null;
      if (!widget) return '';
      if (widget.provider === 'noteva_pow') return widget.token || '';
      if (widget.provider === 'cap') return widget.token || widget.element?.token || widget.element?.value || '';
      if (widget.provider === 'turnstile' && window.turnstile) return window.turnstile.getResponse(widget.id) || '';
      if (widget.provider === 'hcaptcha' && window.hcaptcha) return window.hcaptcha.getResponse(widget.id) || '';
      return '';
    },

    reset(container) {
      const element = typeof container === 'string' ? document.querySelector(container) : container;
      const widget = element ? this._widgets.get(element) : null;
      if (!widget) return;
      if (widget.provider === 'noteva_pow') {
        widget.token = '';
        this._clearPowTimer(widget);
        this._setPowWidgetStatus(element, widget, 'idle');
        return;
      }
      if (widget.provider === 'cap') {
        widget.token = '';
        if (widget.element?.reset) widget.element.reset();
        return;
      }
      if (widget.provider === 'turnstile' && window.turnstile) window.turnstile.reset(widget.id);
      if (widget.provider === 'hcaptcha' && window.hcaptcha) window.hcaptcha.reset(widget.id);
    },

    destroy(container) {
      const element = typeof container === 'string' ? document.querySelector(container) : container;
      const widget = element ? this._widgets.get(element) : null;
      if (!widget) return;
      if (widget.provider === 'noteva_pow') {
        this._clearPowTimer(widget);
        element.innerHTML = '';
        this._widgets.delete(element);
        return;
      }
      if (widget.provider === 'cap') {
        if (widget.cleanup) widget.cleanup();
        element.innerHTML = '';
        this._widgets.delete(element);
        return;
      }
      if (widget.provider === 'turnstile' && window.turnstile?.remove) window.turnstile.remove(widget.id);
      if (widget.provider === 'hcaptcha' && window.hcaptcha?.remove) window.hcaptcha.remove(widget.id);
      this._widgets.delete(element);
    },

    async createChallenge(options = {}) {
      const result = await api.post('/captcha/challenge', {
        action: options.action || 'comment',
      });
      return result.challenge || result;
    },

    async verifyPow(options = {}) {
      const result = await api.post('/captcha/verify', {
        action: options.action || 'comment',
        challenge_id: options.challengeId || options.challenge_id,
        solution: options.solution,
        elapsed_ms: options.elapsedMs || options.elapsed_ms,
      });
      return {
        token: result.token || '',
        action: result.action || options.action || 'comment',
        expiresAt: result.expiresAt || result.expires_at || '',
      };
    },

    async solve(options = {}) {
      const config = await this.getConfig();
      if (!config.enabled || config.provider === 'none') return '';
      if (config.provider !== 'noteva_pow') {
        return this.getToken(options.container);
      }

      const action = options.action || 'comment';
      const element = typeof options.container === 'string'
        ? document.querySelector(options.container)
        : options.container;
      const widget = element ? this._widgets.get(element) : null;

      if (widget?.provider === 'noteva_pow' && widget.token && !options.force) {
        return widget.token;
      }

      if (widget?.provider === 'noteva_pow') {
        widget.options = {
          ...(widget.options || {}),
          ...options,
        };
        widget.action = action;
        widget.token = '';
        this._clearPowTimer(widget);
        this._setPowWidgetStatus(element, widget, 'solving');
      }

      try {
        const challenge = await this.createChallenge({ action });
        const solved = await solvePowChallenge(challenge, options);
        const verified = await this.verifyPow({
          action,
          challengeId: challenge.id,
          solution: solved.solution,
          elapsedMs: solved.elapsedMs,
        });

        if (element) {
          const current = this._widgets.get(element) || { provider: 'noteva_pow', id: null };
          current.token = verified.token;
          current.action = action;
          this._widgets.set(element, current);

          if (current.provider === 'noteva_pow') {
            this._clearPowTimer(current);
            this._setPowWidgetStatus(element, current, 'ready');

            const expiresAt = verified.expiresAt ? new Date(verified.expiresAt).getTime() : 0;
            const delay = expiresAt > Date.now()
              ? Math.max(0, expiresAt - Date.now())
              : asNumber(config.pow?.tokenTtlSeconds, 300) * 1000;
            current.expireTimer = setTimeout(() => {
              this._expirePowWidget(element, current);
            }, Math.max(1000, delay));

            if (typeof current.options?.callback === 'function') {
              current.options.callback(verified.token);
            }
          }
        }

        return verified.token;
      } catch (error) {
        if (widget?.provider === 'noteva_pow') {
          widget.token = '';
          this._clearPowTimer(widget);
          this._setPowWidgetStatus(element, widget, 'error');
          if (typeof widget.options?.errorCallback === 'function') {
            widget.options.errorCallback(error);
          }
        }
        throw error;
      }
    },
  };

  // ============================================
  // 用户 API
  // ============================================
  const user = {
    _current: null,
    _checked: false,

    isLoggedIn() {
      return this._current !== null;
    },

    getCurrent() {
      return this._current;
    },

    // Promise 锁，防止并发调用
    _checkPromise: null,

    async check() {
      // 如果已经检查过，直接返回
      if (this._checked) return this._current;

      // 如果正在检查中，等待现有的 Promise
      if (this._checkPromise) return this._checkPromise;

      // 创建新的检查 Promise
      this._checkPromise = (async () => {
        try {
          this._current = normalizeSimpleUser(await api.get('/auth/me'));
          this._checked = true;
          return this._current;
        } catch (e) {
          this._current = null;
          this._checked = true;
          return null;
        } finally {
          this._checkPromise = null;
        }
      })();

      return this._checkPromise;
    },

    async login(credentials) {
      hooks.trigger('user_login_before', credentials);
      try {
        // 转换字段名：前端用 username，后端期望 username_or_email
        const loginData = {
          username_or_email: credentials.username || credentials.username_or_email,
          password: credentials.password,
        };
        const result = await api.post('/auth/login', loginData);
        this._current = result.user;
        hooks.trigger('user_login_after', this._current);
        events.emit('user:login', this._current);
        return result;
      } catch (error) {
        hooks.trigger('user_login_failed', credentials, error);
        throw error;
      }
    },

    async register(data) {
      hooks.trigger('user_register_before', data);
      const result = await api.post('/auth/register', data);
      this._current = result.user;
      hooks.trigger('user_register_after', this._current);
      events.emit('user:login', this._current);
      return result;
    },

    async logout() {
      const currentUser = this._current;
      hooks.trigger('user_logout', currentUser);
      await api.post('/auth/logout');
      this._current = null;
      this._checked = true;
      events.emit('user:logout');
    },

    async updateProfile(data) {
      const result = await api.put('/auth/profile', data);
      // 更新本地缓存的用户信息
      if (this._current) {
        this._current = { ...this._current, ...result };
      }
      events.emit('user:update', this._current);
      return result;
    },

    async changePassword(currentPassword, newPassword) {
      await api.put('/auth/password', {
        current_password: currentPassword,
        new_password: newPassword,
      });
    },

    hasPermission(permission) {
      if (!this._current) return false;
      if (this._current.role === 'admin') return true;
      // 可以扩展更细粒度的权限检查
      return false;
    },
  };

  const publicUser = {
    isLoggedIn: () => user.isLoggedIn(),
    getCurrent: () => user.getCurrent(),
    check: () => user.check(),
    logout: () => user.logout(),
    hasPermission: (permission) => user.hasPermission(permission),
  };

  const errors = {
    NotevaError,
    isNotFound(error) {
      return error?.status === 404;
    },
    isUnauthorized(error) {
      return error?.status === 401;
    },
    isForbidden(error) {
      return error?.status === 403;
    },
    isValidation(error) {
      return error?.status === 400 || error?.code === 'VALIDATION_ERROR';
    },
  };

  // ============================================
  // 路由辅助
  // ============================================
  const router = {
    getPath() {
      return window.location.pathname;
    },

    getQuery(key) {
      const params = new URLSearchParams(window.location.search);
      return params.get(key);
    },

    getQueryAll() {
      const params = new URLSearchParams(window.location.search);
      const result = {};
      for (const [key, value] of params) {
        result[key] = value;
      }
      return result;
    },

    /**
     * 匹配路由模式
     * @param {string} pattern - 路由模式，如 "/posts/:slug"
     * @returns {{ matched: boolean, params: object }}
     */
    match(pattern) {
      const path = this.getPath();
      const patternParts = pattern.split('/').filter(Boolean);
      const pathParts = path.split('/').filter(Boolean);

      if (patternParts.length !== pathParts.length) {
        return { matched: false, params: {} };
      }

      const params = {};
      for (let i = 0; i < patternParts.length; i++) {
        if (patternParts[i].startsWith(':')) {
          params[patternParts[i].slice(1)] = decodeURIComponent(pathParts[i]);
        } else if (patternParts[i] !== pathParts[i]) {
          return { matched: false, params: {} };
        }
      }

      return { matched: true, params };
    },

    /**
     * 从路径中提取参数
     */
    getParam(name) {
      // 常见路由模式
      const patterns = [
        '/posts/:slug',
        '/categories/:slug',
        '/tags/:slug',
        '/:slug',
      ];

      for (const pattern of patterns) {
        const result = this.match(pattern);
        if (result.matched && result.params[name]) {
          return result.params[name];
        }
      }
      return null;
    },

    push(path) {
      const oldPath = this.getPath();
      events.emit('route:before', { from: oldPath, to: path });
      hooks.trigger('route_before', { from: oldPath, to: path });
      window.history.pushState({}, '', path);
    },

    replace(path) {
      const oldPath = this.getPath();
      events.emit('route:before', { from: oldPath, to: path });
      hooks.trigger('route_before', { from: oldPath, to: path });
      window.history.replaceState({}, '', path);
    },
  };

  const page = {
    type: 'unknown',
    path: '',
    query: {},
    articleId: null,
    article: null,
    pageId: null,
    customPage: null,

    set(context = {}) {
      const has = (key) => Object.prototype.hasOwnProperty.call(context, key);
      this.type = context.type || this.type || 'unknown';
      this.path = context.path || router.getPath();
      this.query = context.query || router.getQueryAll();
      this.articleId = has('articleId') ? context.articleId : firstValue(context.article?.id, this.articleId, null);
      this.article = has('article') ? context.article : firstValue(context.article, this.article, null);
      this.pageId = has('pageId') ? context.pageId : firstValue(context.customPage?.id, this.pageId, null);
      this.customPage = has('customPage') ? context.customPage : firstValue(context.customPage, this.customPage, null);
      const current = this.get();
      events.emit('page:change', current);
      return current;
    },

    get() {
      return {
        type: this.type,
        path: this.path || router.getPath(),
        query: this.query || router.getQueryAll(),
        articleId: this.articleId,
        article: this.article,
        pageId: this.pageId,
        customPage: this.customPage,
      };
    },

    clear() {
      this.type = 'unknown';
      this.path = router.getPath();
      this.query = router.getQueryAll();
      this.articleId = null;
      this.article = null;
      this.pageId = null;
      this.customPage = null;
      return this.get();
    },
  };

  // ============================================
  // 工具函数
  // ============================================
  const utils = {
    /**
     * 格式化日期
     */
    formatDate(date, format = 'YYYY-MM-DD') {
      const d = new Date(date);
      if (isNaN(d.getTime())) return '';

      if (format === 'relative') {
        return this.timeAgo(date);
      }

      const year = d.getFullYear();
      const month = String(d.getMonth() + 1).padStart(2, '0');
      const day = String(d.getDate()).padStart(2, '0');
      const hours = String(d.getHours()).padStart(2, '0');
      const minutes = String(d.getMinutes()).padStart(2, '0');
      const seconds = String(d.getSeconds()).padStart(2, '0');

      return format
        .replace('YYYY', year)
        .replace('MM', month)
        .replace('DD', day)
        .replace('HH', hours)
        .replace('mm', minutes)
        .replace('ss', seconds)
        .replace('年', '年')
        .replace('月', '月')
        .replace('日', '日');
    },

    /**
     * 相对时间
     */
    timeAgo(date) {
      const now = new Date();
      const d = new Date(date);
      const diff = Math.floor((now - d) / 1000);

      if (diff < 60) return '刚刚';
      if (diff < 3600) return `${Math.floor(diff / 60)} 分钟前`;
      if (diff < 86400) return `${Math.floor(diff / 3600)} 小时前`;
      if (diff < 2592000) return `${Math.floor(diff / 86400)} 天前`;
      if (diff < 31536000) return `${Math.floor(diff / 2592000)} 个月前`;
      return `${Math.floor(diff / 31536000)} 年前`;
    },

    /**
     * HTML 转义
     */
    escapeHtml(str) {
      const div = document.createElement('div');
      div.textContent = str;
      return div.innerHTML;
    },

    /**
     * 截断文本
     */
    truncate(text, length, suffix = '...') {
      if (!text || text.length <= length) return text;
      return text.slice(0, length) + suffix;
    },

    /**
     * 从 Markdown 生成摘要
     */
    excerpt(markdown, length = 200) {
      // 移除 Markdown 语法
      const text = markdown
        .replace(/```[\s\S]*?```/g, '')  // 代码块
        .replace(/`[^`]+`/g, '')          // 行内代码
        .replace(/!\[.*?\]\(.*?\)/g, '')  // 图片
        .replace(/\[([^\]]+)\]\(.*?\)/g, '$1')  // 链接
        .replace(/[#*_~>`-]/g, '')        // 其他标记
        .replace(/\n+/g, ' ')             // 换行
        .trim();
      return this.truncate(text, length);
    },

    /**
     * 防抖
     */
    debounce(fn, delay) {
      let timer = null;
      return function (...args) {
        clearTimeout(timer);
        timer = setTimeout(() => fn.apply(this, args), delay);
      };
    },

    /**
     * 节流
     */
    throttle(fn, delay) {
      let last = 0;
      return function (...args) {
        const now = Date.now();
        if (now - last >= delay) {
          last = now;
          fn.apply(this, args);
        }
      };
    },

    /**
     * 复制到剪贴板
     */
    async copyToClipboard(text) {
      try {
        await navigator.clipboard.writeText(text);
        return true;
      } catch {
        // 降级方案
        const textarea = document.createElement('textarea');
        textarea.value = text;
        textarea.style.position = 'fixed';
        textarea.style.opacity = '0';
        document.body.appendChild(textarea);
        textarea.select();
        document.execCommand('copy');
        document.body.removeChild(textarea);
        return true;
      }
    },

    /**
     * 生成唯一 ID
     */
    uniqueId(prefix = 'noteva') {
      return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
    },

    /**
     * 检测深色模式偏好
     */
    prefersDarkMode() {
      return window.matchMedia('(prefers-color-scheme: dark)').matches;
    },

    /**
     * 图片懒加载
     */
    lazyLoadImages(selector = 'img[data-src]') {
      const images = document.querySelectorAll(selector);
      const observer = new IntersectionObserver((entries) => {
        entries.forEach(entry => {
          if (entry.isIntersecting) {
            const img = entry.target;
            img.src = img.dataset.src;
            img.removeAttribute('data-src');
            observer.unobserve(img);
          }
        });
      });
      images.forEach(img => observer.observe(img));
    },
  };

  // ============================================
  // UI 组件
  // ============================================
  const ui = {
    /**
     * Toast 提示
     */
    toast(message, type = 'info', duration = 3000) {
      // 触发钩子，允许插件自定义 toast
      const handled = hooks.trigger('ui_toast', { message, type, duration, handled: false });
      if (handled.handled) return;

      // 默认实现
      const container = this._getToastContainer();
      const toast = document.createElement('div');
      toast.className = `noteva-toast noteva-toast-${type}`;
      toast.textContent = message;
      container.appendChild(toast);

      setTimeout(() => {
        toast.classList.add('noteva-toast-hide');
        setTimeout(() => toast.remove(), 300);
      }, duration);
    },

    _getToastContainer() {
      let container = document.getElementById('noteva-toast-container');
      if (!container) {
        container = document.createElement('div');
        container.id = 'noteva-toast-container';
        document.body.appendChild(container);
      }
      return container;
    },

    /**
     * 确认对话框
     */
    async confirm(options) {
      if (typeof options === 'string') {
        options = { message: options };
      }

      const { title = '确认', message, confirmText = '确定', cancelText = '取消' } = options;

      return new Promise((resolve) => {
        const overlay = document.createElement('div');
        overlay.className = 'noteva-modal-overlay';
        overlay.innerHTML = `
          <div class="noteva-modal">
            <div class="noteva-modal-header">${this._escape(title)}</div>
            <div class="noteva-modal-body">${this._escape(message)}</div>
            <div class="noteva-modal-footer">
              <button class="noteva-btn noteva-btn-cancel">${this._escape(cancelText)}</button>
              <button class="noteva-btn noteva-btn-confirm">${this._escape(confirmText)}</button>
            </div>
          </div>
        `;

        document.body.appendChild(overlay);

        overlay.querySelector('.noteva-btn-cancel').onclick = () => {
          overlay.remove();
          resolve(false);
        };

        overlay.querySelector('.noteva-btn-confirm').onclick = () => {
          overlay.remove();
          resolve(true);
        };

        overlay.onclick = (e) => {
          if (e.target === overlay) {
            overlay.remove();
            resolve(false);
          }
        };
      });
    },

    /**
     * 加载状态
     */
    showLoading() {
      let loader = document.getElementById('noteva-loading');
      if (!loader) {
        loader = document.createElement('div');
        loader.id = 'noteva-loading';
        loader.innerHTML = '<div class="noteva-spinner"></div>';
        document.body.appendChild(loader);
      }
      loader.style.display = 'flex';
    },

    hideLoading() {
      const loader = document.getElementById('noteva-loading');
      if (loader) loader.style.display = 'none';
    },

    /**
     * 模态框
     */
    modal(options) {
      const { title = '', content = '', onClose } = options;

      const overlay = document.createElement('div');
      overlay.className = 'noteva-modal-overlay';
      overlay.innerHTML = `
        <div class="noteva-modal">
          ${title ? `<div class="noteva-modal-header">${this._escape(title)}<button class="noteva-modal-close">&times;</button></div>` : ''}
          <div class="noteva-modal-body">${content}</div>
        </div>
      `;

      document.body.appendChild(overlay);

      const close = () => {
        overlay.remove();
        if (onClose) onClose();
      };

      overlay.querySelector('.noteva-modal-close')?.addEventListener('click', close);
      overlay.addEventListener('click', (e) => {
        if (e.target === overlay) close();
      });

      return { close, element: overlay };
    },

    _escape(str) {
      return utils.escapeHtml(str);
    },
  };

  // ============================================
  // 本地存储
  // ============================================
  const storage = {
    _prefix: 'noteva_',

    get(key, defaultValue = null) {
      try {
        const value = localStorage.getItem(this._prefix + key);
        if (value === null) return defaultValue;
        return JSON.parse(value);
      } catch {
        return defaultValue;
      }
    },

    set(key, value) {
      try {
        localStorage.setItem(this._prefix + key, JSON.stringify(value));
      } catch (e) {
        console.warn('[Noteva] Storage set failed:', e);
      }
    },

    remove(key) {
      localStorage.removeItem(this._prefix + key);
    },

    clear() {
      const keys = [];
      for (let i = 0; i < localStorage.length; i++) {
        const key = localStorage.key(i);
        if (key.startsWith(this._prefix)) {
          keys.push(key);
        }
      }
      keys.forEach(key => localStorage.removeItem(key));
    },
  };

  // ============================================
  // 文件上传 API
  // ============================================
  async function uploadMultipart(url, entries) {
    const formData = new FormData();
    for (const [field, value] of entries) {
      formData.append(field, value);
    }

    const response = await fetch(API_BASE + url, {
      method: 'POST',
      headers: csrfHeaders('POST'),
      body: formData,
      credentials: 'include',
    });

    const result = await response.json().catch(() => ({}));
    if (!response.ok) {
      throw new NotevaError(extractErrorMessage(result, `HTTP ${response.status}`), {
        status: response.status,
        code: result.code,
        data: result,
        url,
        method: 'POST',
      });
    }
    return result;
  }

  const normalizeUploadResult = (result = {}) => ({
    url: result.url || '',
    filename: result.filename || '',
    size: asNumber(result.size, 0),
    contentType: firstValue(result.contentType, result.content_type, ''),
  });

  const normalizeMultiUploadResult = (result = {}) => ({
    files: asArray(result.files).map(normalizeUploadResult),
    failed: asArray(result.failed),
  });

  const upload = {
    /**
     * 上传图片
     * @param {File} file - 文件对象
     * @returns {Promise<{url: string, filename: string, size: number}>}
     */
    async image(file) {
      return normalizeUploadResult(await uploadMultipart('/upload/image', [['file', file]]));
    },

    /**
     * 批量上传图片
     * @param {File[]|FileList} files - 文件列表
     */
    async images(files) {
      return normalizeMultiUploadResult(await uploadMultipart('/upload/images', Array.from(files || []).map(file => ['files', file])));
    },

    /**
     * 上传普通文件
     * @param {File} file - 文件对象
     */
    async file(file) {
      return normalizeUploadResult(await uploadMultipart('/upload/file', [['file', file]]));
    },

    /**
     * 上传插件文件
     * @param {string} pluginId - 插件 ID
     * @param {File} file - 文件对象
     * @returns {Promise<{url: string, filename: string, size: number}>}
     */
    async pluginFile(pluginId, file) {
      return normalizeUploadResult(await uploadMultipart(`/upload/plugin/${encodeURIComponent(pluginId)}/file`, [['file', file]]));
    },
  };

  // ============================================
  // 缓存 API
  // ============================================
  const cache = {
    /**
     * 获取缓存值
     */
    async get(key) {
      try {
        const result = await api.get(`/cache/${key}`);
        return result.value;
      } catch (e) {
        if (e.status === 404) return null;
        throw e;
      }
    },

    /**
     * 设置缓存值
     * @param {string} key - 缓存键
     * @param {string} value - 缓存值
     * @param {number} ttl - 过期时间（秒），默认 3600
     */
    async set(key, value, ttl = 3600) {
      await api.put(`/cache/${key}`, { value, ttl });
    },

    /**
     * 删除缓存值
     */
    async delete(key) {
      await api.delete(`/cache/${key}`);
    },
  };

  // ============================================
  // SEO 辅助
  // ============================================
  const seo = {
    setTitle(title) {
      document.title = title;
    },

    setMeta(meta) {
      Object.entries(meta).forEach(([name, content]) => {
        let el = document.querySelector(`meta[name="${name}"]`);
        if (!el) {
          el = document.createElement('meta');
          el.name = name;
          document.head.appendChild(el);
        }
        el.content = content;
      });
    },

    setOpenGraph(og) {
      Object.entries(og).forEach(([property, content]) => {
        const prop = `og:${property}`;
        let el = document.querySelector(`meta[property="${prop}"]`);
        if (!el) {
          el = document.createElement('meta');
          el.setAttribute('property', prop);
          document.head.appendChild(el);
        }
        el.content = content;
      });
    },

    setTwitterCard(twitter) {
      Object.entries(twitter).forEach(([name, content]) => {
        const prop = `twitter:${name}`;
        let el = document.querySelector(`meta[name="${prop}"]`);
        if (!el) {
          el = document.createElement('meta');
          el.name = prop;
          document.head.appendChild(el);
        }
        el.content = content;
      });
    },

    set(options) {
      if (options.title) this.setTitle(options.title);
      if (options.meta) this.setMeta(options.meta);
      if (options.og) this.setOpenGraph(options.og);
      if (options.twitter) this.setTwitterCard(options.twitter);

      // 触发 SEO meta 标签钩子，允许插件修改或添加 meta 标签
      const modifiedOptions = hooks.trigger('seo_meta_tags', options);
      if (modifiedOptions && modifiedOptions !== options) {
        if (modifiedOptions.title) this.setTitle(modifiedOptions.title);
        if (modifiedOptions.meta) this.setMeta(modifiedOptions.meta);
        if (modifiedOptions.og) this.setOpenGraph(modifiedOptions.og);
        if (modifiedOptions.twitter) this.setTwitterCard(modifiedOptions.twitter);
      }
    },

    /**
     * 一键设置文章页 SEO（title + meta + OG + Twitter）
     * @param {object} article - Article object with title, excerpt, thumbnail, slug, publishedAt, updatedAt
     * @param {string} siteName - 站点名称
     * @param {string} [siteUrl] - 站点 URL
     */
    setArticleMeta(article, siteName, siteUrl) {
      const title = `${article.title} - ${siteName}`;
      const desc = (article.excerpt || '').substring(0, 200);
      const url = siteUrl ? `${siteUrl.replace(/\/$/, '')}${urls.article(article)}` : '';
      const image = article.thumbnail || article.coverImage || '';

      this.set({
        title,
        meta: { description: desc },
        og: {
          title: article.title,
          description: desc,
          type: 'article',
          site_name: siteName,
          ...(url ? { url } : {}),
          ...(image ? { image } : {}),
        },
        twitter: {
          card: image ? 'summary_large_image' : 'summary',
          title: article.title,
          description: desc,
          ...(image ? { image } : {}),
        },
      });
    },

    /**
     * 一键设置站点首页 SEO
     * @param {string} siteName - 站点名称
     * @param {string} description - 站点描述
     * @param {string} [siteUrl] - 站点 URL
     */
    setSiteMeta(siteName, description, siteUrl) {
      const desc = (description || '').substring(0, 200);
      this.set({
        title: siteName,
        meta: { description: desc },
        og: {
          title: siteName,
          description: desc,
          type: 'website',
          site_name: siteName,
          ...(siteUrl ? { url: siteUrl } : {}),
        },
        twitter: {
          card: 'summary',
          title: siteName,
          description: desc,
        },
      });
    },
  };

  // ============================================
  // TOC（文章目录）
  // ============================================
  const toc = {
    /**
     * 从 DOM 中提取 heading 结构生成目录数据
     * @param {string|HTMLElement} [container='article'] - 内容容器选择器或元素
     * @param {string} [levels='h1,h2,h3,h4'] - 要提取的 heading 层级
     * @returns {Array<{id:string, text:string, level:number}>}
     */
    extract(container, levels) {
      const el = typeof container === 'string'
        ? document.querySelector(container)
        : (container || document.querySelector('article') || document.querySelector('.prose') || document.querySelector('main'));
      if (!el) return [];

      const selector = levels || 'h1,h2,h3,h4';
      const headings = el.querySelectorAll(selector);
      return Array.from(headings)
        .filter(h => h.id)
        .map(h => ({
          id: h.id,
          text: h.textContent?.trim() || '',
          level: parseInt(h.tagName.charAt(1), 10),
        }));
    },

    /**
     * 平滑滚动到指定 heading
     * @param {string} id - heading 的 id
     * @param {number} [offset=80] - 顶部偏移量（用于固定导航栏）
     */
    scrollTo(id, offset) {
      const el = document.getElementById(id);
      if (!el) return;
      const top = el.getBoundingClientRect().top + window.scrollY - (offset ?? 80);
      window.scrollTo({ top, behavior: 'smooth' });
    },

    /**
     * 监听滚动并返回当前可见的 heading id（scroll spy）
     * @param {Array<{id:string}>} items - TOC 项列表
     * @param {function} callback - 回调函数，接收当前激活的 id
     * @param {number} [offset=100] - 触发判定的顶部偏移量
     * @returns {function} 取消监听的函数
     */
    observe(items, callback, offset) {
      const off = offset ?? 100;
      const handler = () => {
        let activeId = '';
        for (const item of items) {
          const el = document.getElementById(item.id);
          if (el) {
            const rect = el.getBoundingClientRect();
            if (rect.top <= off) {
              activeId = item.id;
            }
          }
        }
        callback(activeId);
      };
      window.addEventListener('scroll', handler, { passive: true });
      handler(); // 初始调用
      return () => window.removeEventListener('scroll', handler);
    },
  };

  // ============================================
  // 国际化
  // ============================================
  const i18n = {
    _locale: 'zh-CN',
    _messages: {},

    getLocale() {
      return this._locale;
    },

    setLocale(locale) {
      this._locale = locale;
      events.emit('locale:change', locale);
    },

    addMessages(locale, messages) {
      this._messages[locale] = { ...this._messages[locale], ...messages };
    },

    t(key, params = {}) {
      const messages = this._messages[this._locale] || {};
      let text = key.split('.').reduce((obj, k) => obj?.[k], messages) || key;

      // 替换参数 {name}
      Object.entries(params).forEach(([k, v]) => {
        text = text.replace(new RegExp(`\\{${k}\\}`, 'g'), v);
      });

      return text;
    },

    /**
     * 获取所有可用语言列表
     * 主题可用此方法填充语言切换器
     * @param {Array<{code: string, name: string}>} builtinLocales - 主题内置语言列表
     * @returns {Array<{code: string, name: string}>}
     */
    getLocales(builtinLocales = []) {
      return Array.isArray(builtinLocales) ? builtinLocales : [];
    },
  };

  // ============================================
  // 插件系统
  // ============================================
  const plugins = {
    _plugins: {},
    _settings: {},
    _enabled: [],
    _loaded: false,

    /**
     * 注册插件
     */
    register(id, plugin) {
      this._plugins[id] = plugin;
      if (plugin.init) {
        plugin.init();
      }
    },

    /**
     * 获取插件
     */
    get(id) {
      return this._plugins[id];
    },

    /**
     * 获取插件设置
     */
    getSettings(pluginId) {
      return this._settings[pluginId] || {};
    },

    /**
     * 获取已启用插件列表
     */
    list() {
      return this._enabled.slice();
    },

    /**
     * 等待 SDK 与启用插件列表加载完成，并返回插件设置。
     * defaults 会先合并，用户保存的设置优先级更高。
     */
    async ready(pluginId, defaults = {}) {
      await ready();
      await loadEnabledPlugins();
      return {
        ...defaults,
        ...this.getSettings(pluginId),
      };
    },

    /**
     * 触发插件后台动作。需要管理员登录。
     */
    async action(pluginId, action, data = {}) {
      return api.post(
        `/admin/plugins/${encodeURIComponent(pluginId)}/action/${encodeURIComponent(action)}`,
        data
      );
    },

    /**
     * 调用插件公开 API。返回 JSON 响应时自动解析，否则返回文本。
     */
    async request(pluginId, path = '', options = {}) {
      const method = String(options.method || 'GET').toUpperCase();
      const normalizedPath = String(path || '').replace(/^\/+/, '');
      const url = `/plugins/${encodeURIComponent(pluginId)}/api/${normalizedPath}`;
      const headers = { ...(options.headers || {}) };
      let body = options.body !== undefined ? options.body : options.data;

      if (body !== undefined && body !== null && typeof body === 'object' && !(body instanceof FormData) && !(body instanceof Blob)) {
        body = JSON.stringify(body);
        if (!headers['Content-Type'] && !headers['content-type']) {
          headers['Content-Type'] = 'application/json';
        }
      }
      Object.assign(headers, csrfHeaders(method));

      hooks.trigger('api_request_before', { method, url, data: body });

      try {
        const response = await fetch(API_BASE + url, {
          method,
          headers,
          credentials: 'include',
          body: method === 'GET' || method === 'HEAD' ? undefined : body,
        });
        const contentType = response.headers.get('content-type') || '';
        const result = contentType.includes('application/json')
          ? await response.json().catch(() => null)
          : await response.text();

        hooks.trigger('api_request_after', { method, url, response, result });

        if (!response.ok) {
          throw new NotevaError(
            extractErrorMessage(result, `HTTP ${response.status}`),
            {
              status: response.status,
              code: result?.code || result?.error?.code || null,
              data: result,
              url,
              method,
            }
          );
        }

        return result;
      } catch (error) {
        hooks.trigger('api_error', error);
        throw error;
      }
    },

    /**
     * request 的短别名，适合插件代码里写 Noteva.plugins.api(...)
     */
    api(pluginId, path = '', options = {}) {
      return this.request(pluginId, path, options);
    },

    /**
     * 获取插件数据
     */
    async getData(pluginId, key) {
      const result = await api.get(`/plugins/${encodeURIComponent(pluginId)}/data/${encodeURIComponent(key)}`);
      return result.value;
    },

    data: {
      async get(pluginId, key) {
        return plugins.getData(pluginId, key);
      },
    },

    storage: {
      async get(pluginId, key) {
        return plugins.getData(pluginId, key);
      },
    },

    /**
     * 获取编辑器工具栏按钮
     */
    getEditorButtons() {
      const buttons = [];
      hooks.trigger('editor_toolbar_buttons', buttons);
      return buttons;
    },
  };

  async function loadEnabledPlugins() {
    if (plugins._loaded) return plugins._enabled;
    try {
      const enabledPlugins = asArray(await api.get('/plugins/enabled'));
      plugins._enabled = enabledPlugins;
      for (const plugin of enabledPlugins) {
        plugins._settings[plugin.id] = normalizeSettings(plugin.settings || {});

        // 触发编辑器工具栏钩子
        if (plugin.editor_config && plugin.editor_config.toolbar) {
          for (const button of plugin.editor_config.toolbar) {
            hooks.trigger('editor_toolbar_button', {
              pluginId: plugin.id,
              button: button,
            });
          }
        }
      }
      plugins._loaded = true;
    } catch (e) {
      console.warn('[Noteva] Failed to load plugin settings:', e);
      plugins._enabled = [];
    }
    return plugins._enabled;
  }

  // ============================================
  // Shortcode 系统
  // ============================================
  const shortcodes = {
    _handlers: {},

    /**
     * 注册 shortcode
     */
    register(name, handler) {
      this._handlers[name] = handler;
    },

    /**
     * 解析并渲染 shortcode
     */
    async render(content, context = {}) {
      // 匹配 [name attr="value"]content[/name] 或 [name attr="value" /]
      const regex = /\[(\w+)([^\]]*)\]([\s\S]*?)\[\/\1\]|\[(\w+)([^\]]*?)\/\]/g;

      let result = content;
      let match;

      while ((match = regex.exec(content)) !== null) {
        const name = match[1] || match[4];
        const attrsStr = match[2] || match[5] || '';
        const innerContent = match[3] || '';

        const handler = this._handlers[name];
        if (handler) {
          const attrs = this._parseAttrs(attrsStr);
          try {
            const rendered = await handler.render(innerContent, attrs, context);
            result = result.replace(match[0], rendered);
          } catch (e) {
            console.error(`[Noteva] Shortcode "${name}" render error:`, e);
          }
        }
      }

      return result;
    },

    _parseAttrs(str) {
      const attrs = {};
      const regex = /(\w+)=["']([^"']*)["']/g;
      let match;
      while ((match = regex.exec(str)) !== null) {
        attrs[match[1]] = match[2];
      }
      return attrs;
    },
  };

  // ============================================
  // 页面注入钩子
  // ============================================
  const slots = {
    _slots: {},
    _rendered: new Set(),

    /**
     * 注册插槽内容
     * @param {string} name - 插槽名称 (head_end, body_end, etc.)
     * @param {string|Function} content - HTML 内容或返回 HTML 的函数
     * @param {number} priority - 优先级
     */
    register(name, content, priority = 10) {
      if (!this._slots[name]) {
        this._slots[name] = [];
      }
      this._slots[name].push({ content, priority });
      this._slots[name].sort((a, b) => a.priority - b.priority);

      // 如果插槽已经渲染过，立即注入新内容
      if (this._rendered.has(name)) {
        this._injectToSlot(name, content);
      }
    },

    /**
     * 获取插槽内容
     */
    getContent(name) {
      const items = this._slots[name] || [];
      return items.map(item => {
        if (typeof item.content === 'function') {
          return item.content();
        }
        return item.content;
      }).join('\n');
    },

    /**
     * 渲染插槽到 DOM
     */
    render(name, container) {
      const content = this.getContent(name);
      if (content) {
        if (typeof container === 'string') {
          container = document.querySelector(container);
        }
        if (container) {
          const wrapper = document.createElement('div');
          wrapper.className = `noteva-slot noteva-slot-${name}`;
          wrapper.innerHTML = content;
          container.appendChild(wrapper);

          // 执行插入的脚本
          wrapper.querySelectorAll('script').forEach(oldScript => {
            const newScript = document.createElement('script');
            Array.from(oldScript.attributes).forEach(attr => {
              newScript.setAttribute(attr.name, attr.value);
            });
            newScript.textContent = oldScript.textContent;
            oldScript.parentNode.replaceChild(newScript, oldScript);
          });
        }
      }
      this._rendered.add(name);
      hooks.trigger(name, { container });
    },

    /**
     * 注入内容到已渲染的插槽
     */
    _injectToSlot(name, content) {
      const slot = document.querySelector(`.noteva-slot-${name}`);
      if (slot) {
        const html = typeof content === 'function' ? content() : content;
        const temp = document.createElement('div');
        temp.innerHTML = html;
        while (temp.firstChild) {
          slot.appendChild(temp.firstChild);
        }
      }
    },

    /**
     * 自动渲染所有插槽
     */
    autoRender() {
      // 查找所有带 data-noteva-slot 属性的元素
      document.querySelectorAll('[data-noteva-slot]').forEach(el => {
        const name = el.dataset.notevaSlot;
        this.render(name, el);
      });
    },
  };

  // ============================================
  // 调试工具
  // ============================================
  const debug = {
    _enabled: false,
    _logRequests: false,
    _logEvents: false,
    _logHooks: false,

    enable() {
      this._enabled = true;
      console.log('[Noteva] Debug mode enabled');
    },

    disable() {
      this._enabled = false;
    },

    logRequests(enabled) {
      this._logRequests = enabled;
      if (enabled) {
        hooks.on('api_request_before', (data) => {
          console.log('[Noteva API]', data.method, data.url, data.data);
        });
      }
    },

    logEvents(enabled) {
      this._logEvents = enabled;
      // 需要在事件系统中添加日志
    },

    logHooks(enabled) {
      this._logHooks = enabled;
    },

    mockUser(userData) {
      user._current = userData;
      user._checked = true;
      console.log('[Noteva] Mocked user:', userData);
    },

    mockThemeConfig(config) {
      theme._setConfig(config);
      console.log('[Noteva] Mocked theme config:', config);
    },
  };

  // ============================================
  // 数学公式 & Mermaid 图表自动渲染
  // ============================================
  const _loadedScripts = {};

  function _loadScript(src) {
    if (_loadedScripts[src]) return _loadedScripts[src];
    _loadedScripts[src] = new Promise((resolve, reject) => {
      const s = document.createElement('script');
      s.src = src;
      s.onload = resolve;
      s.onerror = reject;
      document.head.appendChild(s);
    });
    return _loadedScripts[src];
  }

  function _loadCSS(href) {
    if (document.querySelector(`link[href="${href}"]`)) return;
    const link = document.createElement('link');
    link.rel = 'stylesheet';
    link.href = href;
    document.head.appendChild(link);
  }

  async function _renderMathAndDiagrams() {
    // KaTeX: render .math-inline and .math-block elements
    const mathEls = document.querySelectorAll('.math-inline, .math-block');
    if (mathEls.length > 0) {
      _loadCSS('https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.css');
      await _loadScript('https://cdn.jsdelivr.net/npm/katex@0.16.11/dist/katex.min.js');
      mathEls.forEach(el => {
        if (el.dataset.katexRendered) return;
        try {
          window.katex.render(el.textContent, el, {
            displayMode: el.classList.contains('math-block'),
            throwOnError: false,
          });
          el.dataset.katexRendered = '1';
        } catch (e) {
          console.warn('[Noteva] KaTeX render error:', e);
        }
      });
    }

    // Mermaid: render .mermaid elements
    const mermaidEls = document.querySelectorAll('.mermaid:not([data-mermaid-rendered])');
    if (mermaidEls.length > 0) {
      await _loadScript('https://cdn.jsdelivr.net/npm/mermaid@11/dist/mermaid.min.js');
      if (!window._notevaMermaidInit) {
        window.mermaid.initialize({
          startOnLoad: false,
          theme: document.documentElement.classList.contains('dark') ? 'dark' : 'default',
        });
        window._notevaMermaidInit = true;
      }
      try {
        await window.mermaid.run({ nodes: mermaidEls });
        mermaidEls.forEach(el => el.dataset.mermaidRendered = '1');
      } catch (e) {
        console.warn('[Noteva] Mermaid render error:', e);
      }
    }
  }

  function _enhanceContentPrimitives() {
    const revealSpoiler = (el) => {
      el.classList.add('is-revealed');
      el.setAttribute('aria-expanded', 'true');
    };

    document.querySelectorAll('.noteva-spoiler:not([data-noteva-bound])').forEach(el => {
      el.dataset.notevaBound = '1';
      el.addEventListener('click', () => revealSpoiler(el));
      el.addEventListener('keydown', event => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault();
          revealSpoiler(el);
        }
      });
    });

    const locale = i18n.getLocale ? i18n.getLocale() : undefined;
    const formatDate = (value, timezone) => {
      if (!value) return '';
      const date = new Date(value.length === 10 ? `${value}T00:00:00` : value);
      if (Number.isNaN(date.getTime())) return value;
      try {
        return new Intl.DateTimeFormat(locale || undefined, {
          year: 'numeric',
          month: 'short',
          day: 'numeric',
          ...(value.includes('T') ? { hour: '2-digit', minute: '2-digit' } : {}),
          ...(timezone ? { timeZone: timezone } : {}),
        }).format(date);
      } catch {
        return value;
      }
    };

    document.querySelectorAll('.noteva-date[data-noteva-date]:not([data-noteva-formatted])').forEach(el => {
      const value = el.getAttribute('datetime') || el.textContent || '';
      el.textContent = formatDate(value, el.dataset.timezone);
      el.dataset.notevaFormatted = '1';
    });

    document.querySelectorAll('.noteva-date-range[data-noteva-date-range]:not([data-noteva-formatted])').forEach(el => {
      const from = el.dataset.from || '';
      const to = el.dataset.to || '';
      const timezone = el.dataset.timezone;
      const formattedFrom = formatDate(from, timezone);
      const formattedTo = formatDate(to, timezone);
      if (formattedFrom && formattedTo) {
        el.textContent = `${formattedFrom} - ${formattedTo}`;
        el.dataset.notevaFormatted = '1';
      }
    });
  }

  // ============================================
  // Emoji / Twemoji
  // ============================================
  const emoji = {
    _twemojiLoaded: false,
    _twemojiLoading: null,

    /** Emoji data grouped by category */
    categories: [
      {
        id: 'smileys', label: { 'zh-CN': '表情', 'zh-TW': '表情', en: 'Smileys' }, icon: '😀', emojis: {
          'grinning': '😀', 'smiley': '😃', 'smile': '😄', 'grin': '😁', 'laughing': '😆',
          'sweat_smile': '😅', 'rofl': '🤣', 'joy': '😂', 'slightly_smiling_face': '🙂',
          'upside_down_face': '🙃', 'melting_face': '🫠', 'wink': '😉', 'blush': '😊',
          'innocent': '😇', 'smiling_face_with_three_hearts': '🥰', 'heart_eyes': '😍',
          'star_struck': '🤩', 'kissing_heart': '😘', 'kissing': '😗',
          'kissing_closed_eyes': '😚', 'kissing_smiling_eyes': '😙', 'smiling_face_with_tear': '🥲',
          'yum': '😋', 'stuck_out_tongue': '😛', 'stuck_out_tongue_winking_eye': '😜',
          'zany_face': '🤪', 'stuck_out_tongue_closed_eyes': '😝', 'money_mouth_face': '🤑',
          'hugs': '🤗', 'hand_over_mouth': '🤭', 'shushing_face': '🤫', 'thinking': '🤔',
          'saluting_face': '🫡', 'zipper_mouth_face': '🤐', 'raised_eyebrow': '🤨',
          'neutral_face': '😐', 'expressionless': '😑', 'no_mouth': '😶',
          'dotted_line_face': '🫥', 'smirk': '😏', 'unamused': '😒', 'roll_eyes': '🙄',
          'grimacing': '😬', 'lying_face': '🤥', 'shaking_face': '🫨', 'relieved': '😌',
          'pensive': '😔', 'sleepy': '😪', 'drooling_face': '🤤', 'sleeping': '😴',
          'mask': '😷', 'face_with_thermometer': '🤒', 'face_with_head_bandage': '🤕',
          'nauseated_face': '🤢', 'vomiting': '🤮', 'sneezing_face': '🤧',
          'hot': '🥵', 'cold': '🥶', 'woozy_face': '🥴', 'dizzy_face': '😵',
          'exploding_head': '🤯', 'cowboy_hat_face': '🤠', 'partying_face': '🥳',
          'disguised_face': '🥸', 'sunglasses': '😎', 'nerd_face': '🤓', 'monocle_face': '🧐',
          'confused': '😕', 'worried': '😟', 'slightly_frowning_face': '🙁',
          'open_mouth': '😮', 'hushed': '😯', 'astonished': '😲', 'flushed': '😳',
          'pleading_face': '🥺', 'face_holding_back_tears': '🥹',
          'fearful': '😨', 'cold_sweat': '😰', 'cry': '😢', 'sob': '😭', 'scream': '😱',
          'disappointed': '😞', 'sweat': '😓', 'weary': '😩', 'tired_face': '😫',
          'yawning_face': '🥱', 'triumph': '😤', 'rage': '😡', 'angry': '😠',
          'cursing_face': '🤬', 'smiling_imp': '😈', 'imp': '👿', 'skull': '💀',
          'poop': '💩', 'clown_face': '🤡', 'ghost': '👻', 'alien': '👽', 'robot': '🤖',
        }
      },
      {
        id: 'gestures', label: { 'zh-CN': '手势', 'zh-TW': '手勢', en: 'Gestures' }, icon: '👋', emojis: {
          'wave': '👋', 'raised_back_of_hand': '🤚', 'hand': '✋', 'vulcan_salute': '🖖',
          'ok_hand': '👌', 'pinched_fingers': '🤌', 'pinching_hand': '🤏',
          'v': '✌️', 'crossed_fingers': '🤞', 'love_you_gesture': '🤟', 'metal': '🤘',
          'call_me_hand': '🤙', 'point_left': '👈', 'point_right': '👉', 'point_up_2': '👆',
          'middle_finger': '🖕', 'point_down': '👇', 'point_up': '☝️',
          '+1': '👍', '-1': '👎', 'fist': '✊', 'facepunch': '👊',
          'clap': '👏', 'raised_hands': '🙌', 'heart_hands': '🫶', 'open_hands': '👐',
          'handshake': '🤝', 'pray': '🙏', 'writing_hand': '✍️', 'nail_care': '💅', 'muscle': '💪',
        }
      },
      {
        id: 'hearts', label: { 'zh-CN': '心形', 'zh-TW': '心形', en: 'Hearts' }, icon: '❤️', emojis: {
          'heart': '❤️', 'orange_heart': '🧡', 'yellow_heart': '💛', 'green_heart': '💚',
          'blue_heart': '💙', 'purple_heart': '💜', 'black_heart': '🖤', 'white_heart': '🤍',
          'brown_heart': '🤎', 'pink_heart': '🩷', 'broken_heart': '💔',
          'two_hearts': '💕', 'revolving_hearts': '💞', 'heartbeat': '💓', 'heartpulse': '💗',
          'growing_heart': '💖', 'cupid': '💘', 'gift_heart': '💝',
          'love_letter': '💌', 'kiss': '💋', '100': '💯', 'anger': '💢', 'boom': '💥',
          'dizzy': '💫', 'sweat_drops': '💦', 'dash': '💨', 'speech_balloon': '💬', 'zzz': '💤',
        }
      },
      {
        id: 'animals', label: { 'zh-CN': '动物', 'zh-TW': '動物', en: 'Animals' }, icon: '🐱', emojis: {
          'monkey_face': '🐵', 'dog': '🐶', 'cat': '🐱', 'lion': '🦁', 'tiger': '🐯',
          'horse': '🐴', 'unicorn': '🦄', 'cow': '🐮', 'pig': '🐷', 'frog': '🐸',
          'rabbit': '🐰', 'bear': '🐻', 'panda_face': '🐼', 'koala': '🐨',
          'chicken': '🐔', 'penguin': '🐧', 'bird': '🐦', 'eagle': '🦅', 'owl': '🦉',
          'fox_face': '🦊', 'wolf': '🐺', 'turtle': '🐢', 'snake': '🐍', 'dragon_face': '🐲',
          'whale': '🐳', 'dolphin': '🐬', 'fish': '🐟', 'octopus': '🐙', 'shark': '🦈',
          'butterfly': '🦋', 'bug': '🐛', 'bee': '🐝', 'ladybug': '🐞', 'snail': '🐌',
        }
      },
      {
        id: 'food', label: { 'zh-CN': '食物', 'zh-TW': '食物', en: 'Food' }, icon: '🍔', emojis: {
          'apple': '🍎', 'grapes': '🍇', 'watermelon': '🍉', 'tangerine': '🍊', 'banana': '🍌',
          'strawberry': '🍓', 'peach': '🍑', 'cherries': '🍒', 'mango': '🥭', 'pineapple': '🍍',
          'avocado': '🥑', 'eggplant': '🍆', 'carrot': '🥕', 'corn': '🌽', 'hot_pepper': '🌶️',
          'hamburger': '🍔', 'fries': '🍟', 'pizza': '🍕', 'hotdog': '🌭', 'taco': '🌮',
          'sushi': '🍣', 'ramen': '🍜', 'rice': '🍚', 'curry': '🍛',
          'ice_cream': '🍨', 'doughnut': '🍩', 'cookie': '🍪', 'birthday': '🎂', 'cake': '🍰',
          'chocolate_bar': '🍫', 'candy': '🍬', 'coffee': '☕', 'tea': '🍵', 'beer': '🍺',
          'wine_glass': '🍷', 'cocktail': '🍸', 'champagne': '🍾',
        }
      },
      {
        id: 'travel', label: { 'zh-CN': '旅行', 'zh-TW': '旅行', en: 'Travel' }, icon: '🚗', emojis: {
          'car': '🚗', 'taxi': '🚕', 'bus': '🚌', 'ambulance': '🚑', 'fire_engine': '🚒',
          'motorcycle': '🏍️', 'bicycle': '🚲', 'airplane': '✈️', 'rocket': '🚀',
          'ship': '🚢', 'sailboat': '⛵', 'train': '🚋', 'helicopter': '🚁',
          'house': '🏠', 'office': '🏢', 'hospital': '🏥', 'school': '🏫',
          'sunrise': '🌅', 'sunset': '🌇', 'camping': '🏕️', 'beach_umbrella': '🏖️',
          'mountain': '⛰️', 'volcano': '🌋', 'world_map': '🗺️', 'compass': '🧭',
        }
      },
      {
        id: 'objects', label: { 'zh-CN': '物品', 'zh-TW': '物品', en: 'Objects' }, icon: '💻', emojis: {
          'watch': '⌚', 'iphone': '📱', 'computer': '💻', 'keyboard': '⌨️',
          'camera': '📷', 'tv': '📺', 'bulb': '💡', 'fire': '🔥', 'bomb': '💣',
          'gem': '💎', 'money_with_wings': '💸', 'credit_card': '💳',
          'envelope': '✉️', 'package': '📦', 'pencil2': '✏️', 'memo': '📝',
          'briefcase': '💼', 'clipboard': '📋', 'calendar': '📅', 'pushpin': '📌',
          'scissors': '✂️', 'lock': '🔒', 'key': '🔑', 'hammer': '🔨', 'gear': '⚙️',
          'link': '🔗', 'mag': '🔍',
        }
      },
      {
        id: 'symbols', label: { 'zh-CN': '符号', 'zh-TW': '符號', en: 'Symbols' }, icon: '⭐', emojis: {
          'warning': '⚠️', 'no_entry': '⛔', 'x': '❌', 'o': '⭕', 'question': '❓', 'exclamation': '❗',
          'white_check_mark': '✅', 'star': '⭐', 'star2': '🌟', 'sparkles': '✨', 'zap': '⚡',
          'sunny': '☀️', 'cloud': '☁️', 'umbrella': '☂️', 'snowflake': '❄️', 'rainbow': '🌈', 'ocean': '🌊',
          'recycle': '♻️', 'arrow_up': '⬆️', 'arrow_down': '⬇️', 'arrow_left': '⬅️', 'arrow_right': '➡️',
          'new': '🆕', 'free': '🆓', 'cool': '🆒', 'ok': '🆗', 'sos': '🆘',
        }
      },
      {
        id: 'activities', label: { 'zh-CN': '活动', 'zh-TW': '活動', en: 'Activities' }, icon: '⚽', emojis: {
          'soccer': '⚽', 'basketball': '🏀', 'football': '🏈', 'baseball': '⚾', 'tennis': '🎾',
          'trophy': '🏆', '1st_place_medal': '🥇', '2nd_place_medal': '🥈', '3rd_place_medal': '🥉',
          'dart': '🎯', 'video_game': '🎮', 'jigsaw': '🧩', 'teddy_bear': '🧸',
          'art': '🎨', 'musical_note': '🎵', 'microphone': '🎤', 'headphones': '🎧',
          'guitar': '🎸', 'piano': '🎹', 'drum': '🥁',
          'tada': '🎉', 'confetti_ball': '🎊', 'balloon': '🎈', 'gift': '🎁', 'ribbon': '🎀',
          'christmas_tree': '🎄', 'jack_o_lantern': '🎃', 'firecracker': '🧨',
        }
      },
    ],

    /**
     * Get category labels resolved for a locale
     * @param {string} [locale] - e.g. 'zh-CN', 'en'. Defaults to SDK i18n locale.
     * @returns {Array<{id,label,icon,emojis}>}
     */
    getCategories(locale) {
      const loc = locale || i18n.getLocale() || 'zh-CN';
      return this.categories.map(cat => ({
        id: cat.id,
        label: cat.label[loc] || cat.label['en'] || cat.label['zh-CN'],
        icon: cat.icon,
        emojis: cat.emojis,
      }));
    },

    /**
     * Get flat emoji map (shortcode → unicode)
     * @returns {Record<string,string>}
     */
    getMap() {
      const map = {};
      for (const cat of this.categories) {
        Object.assign(map, cat.emojis);
      }
      return map;
    },

    /**
     * Load Twemoji from CDN (lazy, cached)
     * @returns {Promise<object>} twemoji API
     */
    async loadTwemoji() {
      if (window.twemoji) {
        this._twemojiLoaded = true;
        return window.twemoji;
      }
      if (this._twemojiLoading) return this._twemojiLoading;
      this._twemojiLoading = (async () => {
        await _loadScript('https://cdn.jsdelivr.net/npm/@twemoji/api@latest/dist/twemoji.min.js');
        this._twemojiLoaded = true;
        return window.twemoji;
      })();
      return this._twemojiLoading;
    },

    /**
     * Parse an element's emoji to Twemoji images
     * Loads Twemoji from CDN if not yet loaded.
     * @param {HTMLElement} element
     * @param {object} [options] - twemoji.parse options override
     */
    async parse(element, options) {
      const tw = await this.loadTwemoji();
      if (!tw || !element) return;
      tw.parse(element, {
        folder: 'svg',
        ext: '.svg',
        ...options,
      });
    },

    /**
     * Synchronous parse — only works if Twemoji is already loaded.
     * Falls back to no-op if not loaded yet.
     * @param {HTMLElement} element
     * @param {object} [options]
     */
    parseSync(element, options) {
      if (!window.twemoji || !element) return;
      window.twemoji.parse(element, {
        folder: 'svg',
        ext: '.svg',
        ...options,
      });
    },

    /**
     * Check if Twemoji is loaded
     * @returns {boolean}
     */
    isLoaded() {
      return this._twemojiLoaded;
    },
  };

  // ============================================
  // 初始化
  // ============================================
  let _ready = false;
  let _readyCallbacks = [];

  function reportReadyError(error) {
    console.error('[Noteva] Ready callback error:', error);
    setTimeout(() => { throw error; }, 0);
  }

  function runReadyCallback(callback) {
    try {
      const result = callback();
      if (result && typeof result.catch === 'function') {
        result.catch(reportReadyError);
      }
    } catch (e) {
      reportReadyError(e);
    }
  }

  async function init() {
    if (_ready) return;

    try {
      // 触发 system_init 钩子
      hooks.trigger('system_init');

      // 检查用户登录状态
      await user.check();

      // 加载站点信息
      const siteInfo = await site.getInfo();

      // 从 site info 同步版本号
      if (siteInfo && siteInfo.version) {
        window.Noteva.version = siteInfo.version;
      }

      // 注入自定义 CSS/JS（如果后端未注入）
      if (siteInfo) {
        if (siteInfo.customCss && !document.getElementById('noteva-custom-css')) {
          const style = document.createElement('style');
          style.id = 'noteva-custom-css';
          style.textContent = siteInfo.customCss;
          document.head.appendChild(style);
        }
        if (siteInfo.customJs && !document.getElementById('noteva-custom-js')) {
          const script = document.createElement('script');
          script.id = 'noteva-custom-js';
          script.textContent = siteInfo.customJs;
          document.body.appendChild(script);
        }
      }

      // 加载主题配置
      await theme.getConfig();

      // 加载启用的插件设置
      await loadEnabledPlugins();

      // 自动渲染插槽
      slots.autoRender();

      // 触发 body_end 钩子（页面加载完成）
      hooks.trigger('body_end');

      // 自动增强平台内容组件，并渲染数学公式和 Mermaid 图表
      hooks.on('content_render', _enhanceContentPrimitives, 10);
      hooks.on('content_render', _renderMathAndDiagrams, 20);

      page.clear();

      // 触发内容渲染钩子
      hooks.trigger('content_render', {
        path: router.getPath(),
        query: router.getQueryAll(),
      });

      // SPA 路由变化监听：拦截 pushState/replaceState 和 popstate
      // 自动触发 route_change 和 content_render，主题无需手动处理
      let _lastPath = router.getPath();
      let _contentRenderTimer = null;

    const _triggerContentRender = (path) => {
      hooks.trigger('content_render', {
        path: path || router.getPath(),
        query: router.getQueryAll(),
      });
    };

    const _onRouteChange = () => {
      const newPath = router.getPath();
      if (newPath !== _lastPath) {
        const oldPath = _lastPath;
        _lastPath = newPath;
        page.clear();

        // 触发路由变化钩子
        hooks.trigger('route_change', {
          from: oldPath,
          to: newPath,
          query: router.getQueryAll(),
        });
        events.emit('route:change', { from: oldPath, to: newPath });

        // 清除之前的定时器，避免重复触发
        if (_contentRenderTimer) clearTimeout(_contentRenderTimer);

        // 兜底：最多等 800ms 后强制触发一次
        _contentRenderTimer = setTimeout(() => {
          _triggerContentRender(newPath);
        }, 800);
      }
    };

    // MutationObserver：监听 DOM 变化，自动检测内容渲染完成
    // 这样主题开发者完全不需要手动触发 content_render
    const _contentSelectors = [
      'article', '.post-content', '.article-content', '.page-content',
      '.entry-content', '#content', '#post-content', 'main',
      '[data-content]', '.prose', '.markdown-body',
    ];

    let _mutationDebounce = null;
    const _observer = new MutationObserver((mutations) => {
      // 检查是否有实质性的内容变化（不只是属性变化）
      const hasContentChange = mutations.some(m =>
        m.type === 'childList' && m.addedNodes.length > 0
      );
      if (!hasContentChange) return;

      // 检查变化是否发生在内容区域
      const isContentArea = mutations.some(m => {
        const target = m.target;
        if (!target || !target.matches) return false;
        // 直接匹配或者是内容区域的子元素
        return _contentSelectors.some(sel => {
          try { return target.matches(sel) || target.closest(sel); } catch (e) { return false; }
        });
      });

      if (isContentArea) {
        // 防抖：DOM 可能连续变化，等稳定后再触发
        if (_mutationDebounce) clearTimeout(_mutationDebounce);
        _mutationDebounce = setTimeout(() => {
          // 清除兜底定时器
          if (_contentRenderTimer) {
            clearTimeout(_contentRenderTimer);
            _contentRenderTimer = null;
          }
          _triggerContentRender();
        }, 150);
      }
    });

    _observer.observe(document.body, {
      childList: true,
      subtree: true,
    });

    // 拦截 history.pushState 和 replaceState
    const _origPushState = history.pushState.bind(history);
    const _origReplaceState = history.replaceState.bind(history);
    history.pushState = function (...args) {
      _origPushState(...args);
      _onRouteChange();
    };
    history.replaceState = function (...args) {
      _origReplaceState(...args);
      _onRouteChange();
    };
    window.addEventListener('popstate', _onRouteChange);
    } catch (e) {
      console.error('[Noteva] SDK initialization error:', e);
      events.emit('sdk:error', e);
    }

    // 触发初始化完成
    _ready = true;
    events.emit('theme:ready');

    // 执行等待的回调
    const callbacks = _readyCallbacks;
    _readyCallbacks = [];
    callbacks.forEach(runReadyCallback);
  }

  function ready(callback) {
    if (_ready) {
      if (callback) runReadyCallback(callback);
    } else if (callback) {
      _readyCallbacks.push(callback);
    }
    return new Promise(resolve => {
      if (_ready) resolve();
      else _readyCallbacks.push(resolve);
    });
  }

  // ============================================
  // 导出全局对象
  // ============================================
  window.Noteva = {
    // 版本
    version: '0.3.2',
    sdkVersion: SDK_VERSION,

    // 核心系统
    hooks,
    events,
    api,

    // 数据 API
    site,
    theme,
    articles,
    pages,
    friendLinks,
    about,
    categories,
    tags,
    comments,
    captcha,
    user: publicUser,
    interactions,
    search,

    // 辅助工具
    urls,
    router,
    page,
    utils,
    errors,
    ui,
    upload,
    storage,
    cache,
    seo,
    toc,
    i18n,

    // 插件系统
    plugins,
    shortcodes,
    slots,

    // Emoji / Twemoji
    emoji,

    // 调试
    debug,

    // 初始化
    ready,
  };

  // 自动初始化
  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }

})(window);
