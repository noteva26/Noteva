/**
 * Noteva SDK
 * 为主题和插件提供统一的 API 接口
 */
(function(window) {
  'use strict';

  // API 基础路径
  const API_BASE = '/api/v1';

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
        const ret = hook.callback(result, ...args.slice(1));
        if (ret !== undefined) result = ret;
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
        const ret = await hook.callback(result, ...args.slice(1));
        if (ret !== undefined) result = ret;
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
    },
    
    once(event, callback) {
      const wrapper = (...args) => {
        this.off(event, wrapper);
        callback(...args);
      };
      this.on(event, wrapper);
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
        const error = new Error(result.error || `HTTP ${response.status}`);
        error.status = response.status;
        error.data = result;
        throw error;
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

  // ============================================
  // 站点 API
  // ============================================
  const site = {
    _info: null,
    _nav: null,
    _themeConfig: {},
    _permalinkStructure: '/posts/{slug}',
    
    async getInfo() {
      if (this._info) return this._info;
      const data = await api.get('/site/info');
      // 缓存 permalink 结构
      this._permalinkStructure = data.permalink_structure || '/posts/{slug}';
      // 转换字段名，兼容 snake_case 和简短名
      this._info = {
        name: data.site_name || data.name || '',
        description: data.site_description || data.description || '',
        subtitle: data.site_subtitle || data.subtitle || '',
        logo: data.site_logo || data.logo || '',
        footer: data.site_footer || data.footer || '',
        version: data.version || '',
        permalinkStructure: this._permalinkStructure,
        // 保留原始字段
        site_name: data.site_name,
        site_description: data.site_description,
        site_subtitle: data.site_subtitle,
        site_logo: data.site_logo,
        site_footer: data.site_footer,
        permalink_structure: this._permalinkStructure,
      };
      return this._info;
    },
    
    /**
     * 生成文章 URL
     * @param {object} article - 文章对象，需要包含 id 和 slug
     * @returns {string} 文章 URL
     */
    getArticleUrl(article) {
      if (!article) return '/posts/';
      const structure = this._permalinkStructure || '/posts/{slug}';
      return structure
        .replace('{id}', article.id)
        .replace('{slug}', article.slug || article.id);
    },
    
    async getNav() {
      if (this._nav) return this._nav;
      const result = await api.get('/nav');
      this._nav = result.items || [];
      return this._nav;
    },
    
    async loadThemeConfig() {
      try {
        const result = await api.get('/theme/config');
        this._themeConfig = result.config || {};
        events.emit('theme:config:loaded', this._themeConfig);
        return this._themeConfig;
      } catch (e) {
        console.warn('[Noteva] Failed to load theme config:', e);
        return {};
      }
    },
    
    getThemeConfig(key) {
      if (key) return this._themeConfig[key];
      return this._themeConfig;
    },
    
    _setThemeConfig(config) {
      this._themeConfig = config || {};
      events.emit('theme:config:change', this._themeConfig);
    },

    // 主题设置（settings.json 定义，数据库存储）
    _themeSettings: null,

    /**
     * 获取当前主题的设置值
     * 读取 /api/v1/theme/settings（公开接口，无需登录）
     * @param {string} [key] - 可选，指定字段名
     * @returns {Promise<object|string>} 全部设置或单个值
     */
    async getThemeSettings(key) {
      if (!this._themeSettings) {
        try {
          this._themeSettings = await api.get('/theme/settings');
        } catch (e) {
          console.warn('[Noteva] Failed to load theme settings:', e);
          this._themeSettings = {};
        }
      }
      if (key) return this._themeSettings[key];
      return this._themeSettings;
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
        published_only: true,  // 前台只显示已发布文章
      };
      // 只添加有值的可选参数
      if (params.category) queryParams.category = params.category;
      if (params.tag) queryParams.tag = params.tag;
      if (params.keyword) queryParams.keyword = params.keyword;
      
      const result = await api.get('/articles', queryParams);
      return {
        articles: result.articles || [],
        total: result.total || 0,
        page: result.page || 1,
        pageSize: result.page_size || 10,
        hasMore: (result.page || 1) * (result.page_size || 10) < (result.total || 0),
      };
    },
    
    async get(slug) {
      const article = await api.get(`/articles/${slug}`);
      // 触发文章查看钩子
      hooks.trigger('article_view', article);
      events.emit('article:view', article);
      return article;
    },
    
    async getRelated(slug, params = {}) {
      return api.get(`/articles/${slug}/related`, { limit: params.limit || 5 });
    },
    
    async getArchives() {
      return api.get('/articles/archives');
    },
  };

  // ============================================
  // 页面 API
  // ============================================
  const pages = {
    async list() {
      const result = await api.get('/pages');
      return result.pages || [];
    },
    
    async get(slug) {
      const result = await api.get(`/page/${slug}`);
      return result.page || result;
    },
  };

  // ============================================
  // 分类 API
  // ============================================
  const categories = {
    async list() {
      const result = await api.get('/categories');
      return result.categories || [];
    },
    
    async get(slug) {
      return api.get(`/categories/${slug}`);
    },
  };

  // ============================================
  // 标签 API
  // ============================================
  const tags = {
    async list() {
      const result = await api.get('/tags');
      return result.tags || [];
    },
    
    async get(slug) {
      return api.get(`/tags/${slug}`);
    },
  };

  // ============================================
  // 评论 API
  // ============================================
  const comments = {
    async list(articleId) {
      const result = await api.get(`/comments/${articleId}`);
      const commentList = result.comments || result || [];
      // 触发评论显示前钩子
      return hooks.trigger('comment_before_display', commentList);
    },
    
    async create(data) {
      // 触发评论创建前钩子
      const processedData = hooks.trigger('comment_before_create', data);
      
      const comment = await api.post(`/comments`, {
        article_id: processedData.articleId,
        content: processedData.content,
        parent_id: processedData.parentId,
      });
      
      // 触发评论创建后钩子
      hooks.trigger('comment_after_create', comment, { articleId: data.articleId });
      events.emit('comment:create', comment);
      
      return comment;
    },
    
    async delete(commentId) {
      hooks.trigger('comment_before_delete', commentId);
      await api.delete(`/admin/comments/${commentId}`);
      hooks.trigger('comment_after_delete', commentId);
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
          this._current = await api.get('/auth/me');
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
      window.history.pushState({}, '', path);
      events.emit('route:before', { from: oldPath, to: path });
      events.emit('route:change', path);
    },
    
    replace(path) {
      const oldPath = this.getPath();
      window.history.replaceState({}, '', path);
      events.emit('route:before', { from: oldPath, to: path });
      events.emit('route:change', path);
    },
  };

  // 监听浏览器前进后退
  window.addEventListener('popstate', () => {
    events.emit('route:change', router.getPath());
  });

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
      return function(...args) {
        clearTimeout(timer);
        timer = setTimeout(() => fn.apply(this, args), delay);
      };
    },
    
    /**
     * 节流
     */
    throttle(fn, delay) {
      let last = 0;
      return function(...args) {
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
  const upload = {
    /**
     * 上传图片
     * @param {File} file - 文件对象
     * @returns {Promise<{url: string, filename: string, size: number}>}
     */
    async image(file) {
      const formData = new FormData();
      formData.append('file', file);
      
      const response = await fetch('/api/v1/upload/image', {
        method: 'POST',
        body: formData,
        credentials: 'include',
      });
      
      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error?.message || 'Upload failed');
      }
      
      return response.json();
    },
    
    /**
     * 上传插件文件
     * @param {string} pluginId - 插件 ID
     * @param {File} file - 文件对象
     * @returns {Promise<{url: string, filename: string, size: number}>}
     */
    async file(pluginId, file) {
      const formData = new FormData();
      formData.append('file', file);
      
      const response = await fetch(`/api/v1/upload/plugin/${pluginId}/file`, {
        method: 'POST',
        body: formData,
        credentials: 'include',
      });
      
      if (!response.ok) {
        const error = await response.json().catch(() => ({}));
        throw new Error(error.error?.message || 'Upload failed');
      }
      
      return response.json();
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
  };

  // ============================================
  // 插件系统
  // ============================================
  const plugins = {
    _plugins: {},
    _settings: {},
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
     * 保存插件设置
     */
    async saveSettings(pluginId, settings) {
      this._settings[pluginId] = settings;
      await api.put(`/plugins/${pluginId}/settings`, settings);
    },
    
    /**
     * 获取插件数据
     */
    async getData(pluginId, key) {
      const result = await api.get(`/plugins/${pluginId}/data/${key}`);
      return result.value;
    },
    
    /**
     * 设置插件数据
     */
    async setData(pluginId, key, value) {
      await api.put(`/plugins/${pluginId}/data/${key}`, { value });
    },
    
    /**
     * 从后端加载启用的插件设置
     */
    async loadEnabledPlugins() {
      if (this._loaded) return;
      try {
        const enabledPlugins = await api.get('/plugins/enabled');
        for (const plugin of enabledPlugins) {
          this._settings[plugin.id] = plugin.settings || {};
          
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
        this._loaded = true;
      } catch (e) {
        console.warn('[Noteva] Failed to load plugin settings:', e);
      }
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
      site._setThemeConfig(config);
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

  // ============================================
  // Emoji / Twemoji
  // ============================================
  const emoji = {
    _twemojiLoaded: false,
    _twemojiLoading: null,

    /** Emoji data grouped by category */
    categories: [
      { id: 'smileys', label: { 'zh-CN': '表情', 'zh-TW': '表情', en: 'Smileys' }, icon: '😀', emojis: {
        'grinning':'😀','smiley':'😃','smile':'😄','grin':'😁','laughing':'😆',
        'sweat_smile':'😅','rofl':'🤣','joy':'😂','slightly_smiling_face':'🙂',
        'upside_down_face':'🙃','melting_face':'🫠','wink':'😉','blush':'😊',
        'innocent':'😇','smiling_face_with_three_hearts':'🥰','heart_eyes':'😍',
        'star_struck':'🤩','kissing_heart':'😘','kissing':'😗',
        'kissing_closed_eyes':'😚','kissing_smiling_eyes':'😙','smiling_face_with_tear':'🥲',
        'yum':'😋','stuck_out_tongue':'😛','stuck_out_tongue_winking_eye':'😜',
        'zany_face':'🤪','stuck_out_tongue_closed_eyes':'😝','money_mouth_face':'🤑',
        'hugs':'🤗','hand_over_mouth':'🤭','shushing_face':'🤫','thinking':'🤔',
        'saluting_face':'🫡','zipper_mouth_face':'🤐','raised_eyebrow':'🤨',
        'neutral_face':'😐','expressionless':'😑','no_mouth':'😶',
        'dotted_line_face':'🫥','smirk':'😏','unamused':'😒','roll_eyes':'🙄',
        'grimacing':'😬','lying_face':'🤥','shaking_face':'🫨','relieved':'😌',
        'pensive':'😔','sleepy':'😪','drooling_face':'🤤','sleeping':'😴',
        'mask':'😷','face_with_thermometer':'🤒','face_with_head_bandage':'🤕',
        'nauseated_face':'🤢','vomiting':'🤮','sneezing_face':'🤧',
        'hot':'🥵','cold':'🥶','woozy_face':'🥴','dizzy_face':'😵',
        'exploding_head':'🤯','cowboy_hat_face':'🤠','partying_face':'🥳',
        'disguised_face':'🥸','sunglasses':'😎','nerd_face':'🤓','monocle_face':'🧐',
        'confused':'😕','worried':'😟','slightly_frowning_face':'🙁',
        'open_mouth':'😮','hushed':'😯','astonished':'😲','flushed':'😳',
        'pleading_face':'🥺','face_holding_back_tears':'🥹',
        'fearful':'😨','cold_sweat':'😰','cry':'😢','sob':'😭','scream':'😱',
        'disappointed':'😞','sweat':'😓','weary':'😩','tired_face':'😫',
        'yawning_face':'🥱','triumph':'😤','rage':'😡','angry':'😠',
        'cursing_face':'🤬','smiling_imp':'😈','imp':'👿','skull':'💀',
        'poop':'💩','clown_face':'🤡','ghost':'👻','alien':'👽','robot':'🤖',
      }},
      { id: 'gestures', label: { 'zh-CN': '手势', 'zh-TW': '手勢', en: 'Gestures' }, icon: '👋', emojis: {
        'wave':'👋','raised_back_of_hand':'🤚','hand':'✋','vulcan_salute':'🖖',
        'ok_hand':'👌','pinched_fingers':'🤌','pinching_hand':'🤏',
        'v':'✌️','crossed_fingers':'🤞','love_you_gesture':'🤟','metal':'🤘',
        'call_me_hand':'🤙','point_left':'👈','point_right':'👉','point_up_2':'👆',
        'middle_finger':'🖕','point_down':'👇','point_up':'☝️',
        '+1':'👍','-1':'👎','fist':'✊','facepunch':'👊',
        'clap':'👏','raised_hands':'🙌','heart_hands':'🫶','open_hands':'👐',
        'handshake':'🤝','pray':'🙏','writing_hand':'✍️','nail_care':'💅','muscle':'💪',
      }},
      { id: 'hearts', label: { 'zh-CN': '心形', 'zh-TW': '心形', en: 'Hearts' }, icon: '❤️', emojis: {
        'heart':'❤️','orange_heart':'🧡','yellow_heart':'💛','green_heart':'💚',
        'blue_heart':'💙','purple_heart':'💜','black_heart':'🖤','white_heart':'🤍',
        'brown_heart':'🤎','pink_heart':'🩷','broken_heart':'💔',
        'two_hearts':'💕','revolving_hearts':'💞','heartbeat':'💓','heartpulse':'💗',
        'growing_heart':'💖','cupid':'💘','gift_heart':'💝',
        'love_letter':'💌','kiss':'💋','100':'💯','anger':'💢','boom':'💥',
        'dizzy':'💫','sweat_drops':'💦','dash':'💨','speech_balloon':'💬','zzz':'💤',
      }},
      { id: 'animals', label: { 'zh-CN': '动物', 'zh-TW': '動物', en: 'Animals' }, icon: '🐱', emojis: {
        'monkey_face':'🐵','dog':'🐶','cat':'🐱','lion':'🦁','tiger':'🐯',
        'horse':'🐴','unicorn':'🦄','cow':'🐮','pig':'🐷','frog':'🐸',
        'rabbit':'🐰','bear':'🐻','panda_face':'🐼','koala':'🐨',
        'chicken':'🐔','penguin':'🐧','bird':'🐦','eagle':'🦅','owl':'🦉',
        'fox_face':'🦊','wolf':'🐺','turtle':'🐢','snake':'🐍','dragon_face':'🐲',
        'whale':'🐳','dolphin':'🐬','fish':'🐟','octopus':'🐙','shark':'🦈',
        'butterfly':'🦋','bug':'🐛','bee':'🐝','ladybug':'🐞','snail':'🐌',
      }},
      { id: 'food', label: { 'zh-CN': '食物', 'zh-TW': '食物', en: 'Food' }, icon: '🍔', emojis: {
        'apple':'🍎','grapes':'🍇','watermelon':'🍉','tangerine':'🍊','banana':'🍌',
        'strawberry':'🍓','peach':'🍑','cherries':'🍒','mango':'🥭','pineapple':'🍍',
        'avocado':'🥑','eggplant':'🍆','carrot':'🥕','corn':'🌽','hot_pepper':'🌶️',
        'hamburger':'🍔','fries':'🍟','pizza':'🍕','hotdog':'🌭','taco':'🌮',
        'sushi':'🍣','ramen':'🍜','rice':'🍚','curry':'🍛',
        'ice_cream':'🍨','doughnut':'🍩','cookie':'🍪','birthday':'🎂','cake':'🍰',
        'chocolate_bar':'🍫','candy':'🍬','coffee':'☕','tea':'🍵','beer':'🍺',
        'wine_glass':'🍷','cocktail':'🍸','champagne':'🍾',
      }},
      { id: 'travel', label: { 'zh-CN': '旅行', 'zh-TW': '旅行', en: 'Travel' }, icon: '🚗', emojis: {
        'car':'🚗','taxi':'🚕','bus':'🚌','ambulance':'🚑','fire_engine':'🚒',
        'motorcycle':'🏍️','bicycle':'🚲','airplane':'✈️','rocket':'🚀',
        'ship':'🚢','sailboat':'⛵','train':'🚋','helicopter':'🚁',
        'house':'🏠','office':'🏢','hospital':'🏥','school':'🏫',
        'sunrise':'🌅','sunset':'🌇','camping':'🏕️','beach_umbrella':'🏖️',
        'mountain':'⛰️','volcano':'🌋','world_map':'🗺️','compass':'🧭',
      }},
      { id: 'objects', label: { 'zh-CN': '物品', 'zh-TW': '物品', en: 'Objects' }, icon: '💻', emojis: {
        'watch':'⌚','iphone':'📱','computer':'💻','keyboard':'⌨️',
        'camera':'📷','tv':'📺','bulb':'💡','fire':'🔥','bomb':'💣',
        'gem':'💎','money_with_wings':'💸','credit_card':'💳',
        'envelope':'✉️','package':'📦','pencil2':'✏️','memo':'📝',
        'briefcase':'💼','clipboard':'📋','calendar':'📅','pushpin':'📌',
        'scissors':'✂️','lock':'🔒','key':'🔑','hammer':'🔨','gear':'⚙️',
        'link':'🔗','mag':'🔍',
      }},
      { id: 'symbols', label: { 'zh-CN': '符号', 'zh-TW': '符號', en: 'Symbols' }, icon: '⭐', emojis: {
        'warning':'⚠️','no_entry':'⛔','x':'❌','o':'⭕','question':'❓','exclamation':'❗',
        'white_check_mark':'✅','star':'⭐','star2':'🌟','sparkles':'✨','zap':'⚡',
        'sunny':'☀️','cloud':'☁️','umbrella':'☂️','snowflake':'❄️','rainbow':'🌈','ocean':'🌊',
        'recycle':'♻️','arrow_up':'⬆️','arrow_down':'⬇️','arrow_left':'⬅️','arrow_right':'➡️',
        'new':'🆕','free':'🆓','cool':'🆒','ok':'🆗','sos':'🆘',
      }},
      { id: 'activities', label: { 'zh-CN': '活动', 'zh-TW': '活動', en: 'Activities' }, icon: '⚽', emojis: {
        'soccer':'⚽','basketball':'🏀','football':'🏈','baseball':'⚾','tennis':'🎾',
        'trophy':'🏆','1st_place_medal':'🥇','2nd_place_medal':'🥈','3rd_place_medal':'🥉',
        'dart':'🎯','video_game':'🎮','jigsaw':'🧩','teddy_bear':'🧸',
        'art':'🎨','musical_note':'🎵','microphone':'🎤','headphones':'🎧',
        'guitar':'🎸','piano':'🎹','drum':'🥁',
        'tada':'🎉','confetti_ball':'🎊','balloon':'🎈','gift':'🎁','ribbon':'🎀',
        'christmas_tree':'🎄','jack_o_lantern':'🎃','firecracker':'🧨',
      }},
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

  async function init() {
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
      if (siteInfo.custom_css && !document.getElementById('noteva-custom-css')) {
        const style = document.createElement('style');
        style.id = 'noteva-custom-css';
        style.textContent = siteInfo.custom_css;
        document.head.appendChild(style);
      }
      if (siteInfo.custom_js && !document.getElementById('noteva-custom-js')) {
        const script = document.createElement('script');
        script.id = 'noteva-custom-js';
        script.textContent = siteInfo.custom_js;
        document.body.appendChild(script);
      }
    }
    
    // 加载主题配置
    await site.loadThemeConfig();
    
    // 加载启用的插件设置
    await plugins.loadEnabledPlugins();
    
    // 自动渲染插槽
    slots.autoRender();
    
    // 触发 body_end 钩子（页面加载完成）
    hooks.trigger('body_end');
    
    // 触发内容渲染钩子
    hooks.trigger('content_render', {
      path: router.getPath(),
      query: router.getQueryAll(),
    });

    // 自动渲染数学公式和 Mermaid 图表
    hooks.on('content_render', _renderMathAndDiagrams, 20);
    
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
          try { return target.matches(sel) || target.closest(sel); } catch(e) { return false; }
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
    history.pushState = function(...args) {
      _origPushState(...args);
      _onRouteChange();
    };
    history.replaceState = function(...args) {
      _origReplaceState(...args);
      _onRouteChange();
    };
    window.addEventListener('popstate', _onRouteChange);
    
    // 触发初始化完成
    _ready = true;
    events.emit('theme:ready');
    
    // 执行等待的回调
    _readyCallbacks.forEach(cb => cb());
    _readyCallbacks = [];
  }

  function ready(callback) {
    if (_ready) {
      if (callback) callback();
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
    version: '0.1.5',
    
    // 核心系统
    hooks,
    events,
    api,
    
    // 数据 API
    site,
    articles,
    pages,
    categories,
    tags,
    comments,
    user,
    
    // 辅助工具
    router,
    utils,
    ui,
    upload,
    storage,
    cache,
    seo,
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
