import { useCallback, useEffect, useRef, useState } from "react";

export type NotevaSDKRef = NonNullable<typeof window.Noteva>;
export type NotevaArticle = Awaited<ReturnType<NotevaSDKRef["articles"]["get"]>>;
export type NotevaCategory = Awaited<
  ReturnType<NotevaSDKRef["categories"]["list"]>
>[number];
export type NotevaTag = Awaited<ReturnType<NotevaSDKRef["tags"]["list"]>>[number];
export type NotevaUser = NonNullable<
  Awaited<ReturnType<NotevaSDKRef["user"]["check"]>>
>;

export interface InjectedSiteConfig {
  site_name?: string;
  site_description?: string;
  site_subtitle?: string;
  site_logo?: string;
  site_footer?: string;
  [key: string]: unknown;
}

interface WaitForNotevaOptions {
  interval?: number;
  timeout?: number;
}

export function getNoteva(): NotevaSDKRef | null {
  if (typeof window !== "undefined" && window.Noteva) {
    return window.Noteva;
  }
  return null;
}

export function getInjectedSiteConfig(): InjectedSiteConfig | null {
  if (typeof window === "undefined") return null;
  return window.__SITE_CONFIG__ ?? null;
}

export async function waitForNoteva(
  options: WaitForNotevaOptions = {}
): Promise<NotevaSDKRef | null> {
  if (typeof window === "undefined") return null;

  const existing = getNoteva();
  if (existing) {
    try {
      await existing.ready();
    } catch {
      // The SDK object is present; callers can still attempt graceful fallback.
    }
    return existing;
  }

  const interval = options.interval ?? 50;
  const timeout = options.timeout ?? 10_000;

  return new Promise((resolve) => {
    let settled = false;
    let intervalId: number | undefined;
    let timeoutId: number | undefined;

    const finish = async (sdk: NotevaSDKRef | null) => {
      if (settled) return;
      settled = true;

      if (intervalId !== undefined) window.clearInterval(intervalId);
      if (timeoutId !== undefined) window.clearTimeout(timeoutId);

      if (sdk) {
        try {
          await sdk.ready();
        } catch {
          // Keep the SDK reference so callers can decide whether to fallback.
        }
      }

      resolve(sdk);
    };

    const check = () => {
      const sdk = getNoteva();
      if (sdk) {
        void finish(sdk);
      }
    };

    intervalId = window.setInterval(check, interval);
    timeoutId = window.setTimeout(() => {
      void finish(getNoteva());
    }, timeout);
    check();
  });
}

export function getArticleUrl(article: { id: number | string; slug?: string }): string {
  const noteva = getNoteva();
  if (noteva?.urls?.article) {
    return noteva.urls.article(article);
  }
  return `/posts/${article.slug || article.id}`;
}

export function getCategoryUrl(category: string | { slug?: string }): string {
  const noteva = getNoteva();
  if (noteva?.urls?.category) {
    return noteva.urls.category(category);
  }

  const slug = typeof category === "string" ? category : category.slug || "";
  return `/categories?c=${encodeURIComponent(slug)}`;
}

export function getTagUrl(tag: string | { slug?: string }): string {
  const noteva = getNoteva();
  if (noteva?.urls?.tag) {
    return noteva.urls.tag(tag);
  }

  const slug = typeof tag === "string" ? tag : tag.slug || "";
  return `/tags?t=${encodeURIComponent(slug)}`;
}

function escapeHtml(value: string): string {
  return value
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;")
    .replace(/'/g, "&#39;");
}

export function highlightSearchText(text: string, query?: string): string {
  const keyword = query?.trim();
  const safeText = getNoteva()?.utils?.escapeHtml?.(text) ?? escapeHtml(text);
  if (!keyword) return safeText;

  const noteva = getNoteva();
  if (noteva?.search?.highlight) {
    return noteva.search.highlight(safeText, keyword);
  }

  const escaped = keyword.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escaped})`, "gi");
  return safeText.replace(
    regex,
    '<mark class="noteva-highlight">$1</mark>'
  );
}

export function useNoteva() {
  const [ready, setReady] = useState(false);
  const [sdk, setSdk] = useState<NotevaSDKRef | null>(null);

  useEffect(() => {
    let active = true;

    void waitForNoteva().then((noteva) => {
      if (!active) return;
      setSdk(noteva);
      setReady(Boolean(noteva));
    });

    return () => {
      active = false;
    };
  }, []);

  return { ready, Noteva: sdk };
}

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
    let active = true;

    const load = async () => {
      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setLoading(false);
        return;
      }

      try {
        const siteInfo = await noteva.site.getInfo();
        if (active) setInfo(siteInfo);
      } catch {
        if (active) setInfo(null);
      } finally {
        if (active) setLoading(false);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, []);

  return { info, loading };
}

export function useArticles(params?: {
  page?: number;
  pageSize?: number;
  category?: string;
  tag?: string;
  keyword?: string;
}) {
  const mountedRef = useRef(false);
  const requestIdRef = useRef(0);
  const [articles, setArticles] = useState<NotevaArticle[]>([]);
  const [total, setTotal] = useState(0);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const fetchArticles = useCallback(async () => {
    const requestId = ++requestIdRef.current;
    const canUpdate = () => mountedRef.current && requestId === requestIdRef.current;

    setLoading(true);
    setError(null);

    const noteva = await waitForNoteva();
    if (!noteva) {
      if (canUpdate()) {
        setArticles([]);
        setTotal(0);
        setLoading(false);
      }
      return;
    }

    try {
      const result = await noteva.articles.list(params);
      if (canUpdate()) {
        setArticles(result.articles);
        setTotal(result.total);
      }
    } catch (err) {
      if (canUpdate()) {
        setError(err instanceof Error ? err : new Error("Failed to load articles"));
        setArticles([]);
        setTotal(0);
      }
    } finally {
      if (canUpdate()) setLoading(false);
    }
  }, [
    params?.page,
    params?.pageSize,
    params?.category,
    params?.tag,
    params?.keyword,
  ]);

  useEffect(() => {
    void fetchArticles();
  }, [fetchArticles]);

  return { articles, total, loading, error, refetch: fetchArticles };
}

export function useArticle(slug: string) {
  const [article, setArticle] = useState<NotevaArticle | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    let active = true;

    const load = async () => {
      if (!slug) {
        setLoading(false);
        return;
      }

      setLoading(true);
      setError(null);

      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setArticle(null);
        setLoading(false);
        return;
      }

      try {
        const data = await noteva.articles.get(slug);
        if (active) setArticle(data);
      } catch (err) {
        if (active) {
          setError(err instanceof Error ? err : new Error("Failed to load article"));
          setArticle(null);
        }
      } finally {
        if (active) setLoading(false);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, [slug]);

  return { article, loading, error };
}

export function useCategories() {
  const [categories, setCategories] = useState<NotevaCategory[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;

    const load = async () => {
      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setLoading(false);
        return;
      }

      try {
        const data = await noteva.categories.list();
        if (active) setCategories(data);
      } catch {
        if (active) setCategories([]);
      } finally {
        if (active) setLoading(false);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, []);

  return { categories, loading };
}

export function useTags() {
  const [tags, setTags] = useState<NotevaTag[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;

    const load = async () => {
      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setLoading(false);
        return;
      }

      try {
        const data = await noteva.tags.list();
        if (active) setTags(data);
      } catch {
        if (active) setTags([]);
      } finally {
        if (active) setLoading(false);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, []);

  return { tags, loading };
}

export function useAuth() {
  const [user, setUser] = useState<NotevaUser | null>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;

    const checkAuth = async () => {
      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setLoading(false);
        return;
      }

      try {
        const currentUser = await noteva.user.check();
        if (!active) return;
        setUser(currentUser);
        setIsAuthenticated(Boolean(currentUser));
      } catch {
        if (!active) return;
        setUser(null);
        setIsAuthenticated(false);
      } finally {
        if (active) setLoading(false);
      }
    };

    void checkAuth();

    return () => {
      active = false;
    };
  }, []);

  const logout = useCallback(async () => {
    const noteva = await waitForNoteva();
    if (!noteva) return;

    await noteva.user.logout();
    setUser(null);
    setIsAuthenticated(false);
  }, []);

  return { user, isAuthenticated, loading, logout };
}

export default useNoteva;
