/**
 * Noteva SDK React Hook
 * 提供类型安全的 SDK 访问
 */

import { useState, useEffect, useCallback } from "react";

/**
 * 获取 Noteva SDK 实例
 * 在客户端环境下返回 window.Noteva，否则返回 null
 */
export function getNoteva() {
  if (typeof window !== "undefined" && window.Noteva) {
    return window.Noteva;
  }
  return null;
}

/**
 * 等待 SDK 就绪的 Hook
 * @returns { ready: boolean, Noteva: SDK | null }
 */
export function useNoteva() {
  const [ready, setReady] = useState(false);
  const [sdk, setSdk] = useState<typeof window.Noteva | null>(null);

  useEffect(() => {
    const checkReady = () => {
      const noteva = getNoteva();
      if (noteva) {
        noteva.ready().then(() => {
          setSdk(noteva);
          setReady(true);
        });
      } else {
        // SDK 还没加载，等待一下
        setTimeout(checkReady, 50);
      }
    };
    checkReady();
  }, []);

  return { ready, Noteva: sdk };
}

/**
 * 获取站点信息的 Hook
 */
export function useSiteInfo() {
  const [info, setInfo] = useState<{
    name: string;
    description: string;
    subtitle: string;
    logo: string;
    footer: string;
    permalinkStructure?: string;
  } | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const noteva = getNoteva();
    if (noteva) {
      noteva.site.getInfo()
        .then(setInfo)
        .catch(console.error)
        .finally(() => setLoading(false));
    } else {
      // 等待 SDK
      const timer = setInterval(() => {
        const n = getNoteva();
        if (n) {
          clearInterval(timer);
          n.site.getInfo()
            .then(setInfo)
            .catch(console.error)
            .finally(() => setLoading(false));
        }
      }, 50);
      return () => clearInterval(timer);
    }
  }, []);

  return { info, loading };
}

/**
 * 生成文章 URL
 * @param article - 文章对象，需要包含 id 和 slug
 * @returns 文章 URL 路径
 */
export function getArticleUrl(article: { id: number | string; slug?: string }): string {
  const noteva = getNoteva();
  if (noteva?.site?.getArticleUrl) {
    return noteva.site.getArticleUrl(article);
  }
  // 默认使用 slug
  return `/posts/${article.slug || article.id}`;
}

/**
 * 获取文章列表的 Hook
 */
export function useArticles(params?: {
  page?: number;
  pageSize?: number;
  category?: string;
  tag?: string;
  keyword?: string;
}) {
  const [articles, setArticles] = useState<any[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  const fetchArticles = useCallback(async () => {
    const noteva = getNoteva();
    if (!noteva) return;
    
    setLoading(true);
    setError(null);
    try {
      const result = await noteva.articles.list(params);
      setArticles(result.articles);
      setTotal(result.total);
    } catch (err) {
      setError(err as Error);
    } finally {
      setLoading(false);
    }
  }, [params?.page, params?.pageSize, params?.category, params?.tag, params?.keyword]);

  useEffect(() => {
    const noteva = getNoteva();
    if (noteva) {
      fetchArticles();
    } else {
      const timer = setInterval(() => {
        if (getNoteva()) {
          clearInterval(timer);
          fetchArticles();
        }
      }, 50);
      return () => clearInterval(timer);
    }
  }, [fetchArticles]);

  return { articles, total, loading, error, refetch: fetchArticles };
}

/**
 * 获取单篇文章的 Hook
 */
export function useArticle(slug: string) {
  const [article, setArticle] = useState<any | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (!slug) {
      setLoading(false);
      return;
    }

    const fetchArticle = async () => {
      const noteva = getNoteva();
      if (!noteva) return;
      
      setLoading(true);
      setError(null);
      try {
        const data = await noteva.articles.get(slug);
        setArticle(data);
      } catch (err) {
        setError(err as Error);
      } finally {
        setLoading(false);
      }
    };

    const noteva = getNoteva();
    if (noteva) {
      fetchArticle();
    } else {
      const timer = setInterval(() => {
        if (getNoteva()) {
          clearInterval(timer);
          fetchArticle();
        }
      }, 50);
      return () => clearInterval(timer);
    }
  }, [slug]);

  return { article, loading, error };
}

/**
 * 获取分类列表的 Hook
 */
export function useCategories() {
  const [categories, setCategories] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetch = async () => {
      const noteva = getNoteva();
      if (!noteva) return;
      
      try {
        const data = await noteva.categories.list();
        setCategories(data);
      } catch (err) {
        console.error(err);
      } finally {
        setLoading(false);
      }
    };

    const noteva = getNoteva();
    if (noteva) {
      fetch();
    } else {
      const timer = setInterval(() => {
        if (getNoteva()) {
          clearInterval(timer);
          fetch();
        }
      }, 50);
      return () => clearInterval(timer);
    }
  }, []);

  return { categories, loading };
}

/**
 * 获取标签列表的 Hook
 */
export function useTags() {
  const [tags, setTags] = useState<any[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetch = async () => {
      const noteva = getNoteva();
      if (!noteva) return;
      
      try {
        const data = await noteva.tags.list();
        setTags(data);
      } catch (err) {
        console.error(err);
      } finally {
        setLoading(false);
      }
    };

    const noteva = getNoteva();
    if (noteva) {
      fetch();
    } else {
      const timer = setInterval(() => {
        if (getNoteva()) {
          clearInterval(timer);
          fetch();
        }
      }, 50);
      return () => clearInterval(timer);
    }
  }, []);

  return { tags, loading };
}

/**
 * 用户认证状态 Hook
 */
export function useAuth() {
  const [user, setUser] = useState<any | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const checkAuth = async () => {
      const noteva = getNoteva();
      if (!noteva) return;
      
      try {
        const currentUser = await noteva.user.check();
        setUser(currentUser);
        setIsAuthenticated(!!currentUser);
      } catch {
        setUser(null);
        setIsAuthenticated(false);
      } finally {
        setLoading(false);
      }
    };

    const noteva = getNoteva();
    if (noteva) {
      checkAuth();
    } else {
      const timer = setInterval(() => {
        if (getNoteva()) {
          clearInterval(timer);
          checkAuth();
        }
      }, 50);
      return () => clearInterval(timer);
    }
  }, []);

  const login = useCallback(async (username: string, password: string) => {
    const noteva = getNoteva();
    if (!noteva) throw new Error("SDK not ready");
    
    const result = await noteva.user.login({ username, password });
    setUser(result.user);
    setIsAuthenticated(true);
    return result;
  }, []);

  const logout = useCallback(async () => {
    const noteva = getNoteva();
    if (!noteva) return;
    
    await noteva.user.logout();
    setUser(null);
    setIsAuthenticated(false);
  }, []);

  return { user, isAuthenticated, loading, login, logout };
}

export default useNoteva;
