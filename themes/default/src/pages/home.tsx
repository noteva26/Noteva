import {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useRef,
  useState,
  useTransition,
} from "react";
import { useSearchParams } from "react-router-dom";
import { motion } from "motion/react";
import { ArticleSummaryCard } from "@/components/article-summary-card";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { ChevronLeft, ChevronRight, FileText } from "lucide-react";
import {
  getInjectedSiteConfig,
  waitForNoteva,
  type NotevaArticle,
} from "@/hooks/useNoteva";
import { useI18nStore, useTranslation } from "@/lib/i18n";
import {
  getThemeListItemMotion,
  themeHoverLift,
  themePageHeaderMotion,
  themeSpring,
} from "@/lib/motion";

const PAGE_SIZE = 10;
const ARTICLE_SKELETON_KEYS = ["article-a", "article-b", "article-c"];

interface SiteInfo {
  name: string;
  subtitle: string;
  description: string;
}

function getInitialSiteInfo(): SiteInfo | null {
  const config = getInjectedSiteConfig();
  if (!config) return null;

  return {
    name: config.site_name || "Noteva",
    subtitle: config.site_subtitle || "",
    description: config.site_description || "",
  };
}

function getDateLocale(locale: string) {
  const candidate = locale === "en" ? "en-US" : locale;
  try {
    new Intl.DateTimeFormat(candidate);
    return candidate;
  } catch {
    return "zh-CN";
  }
}

function getPageParam(value: string | null) {
  const page = Number.parseInt(value || "", 10);
  return Number.isFinite(page) && page > 0 ? page : 1;
}

function preloadPostPage() {
  void import("@/pages/post");
}

export default function HomePage() {
  const mountedRef = useRef(false);
  const articlesRequestIdRef = useRef(0);
  const [articles, setArticles] = useState<NotevaArticle[]>([]);
  const [loading, setLoading] = useState(true);
  const [siteInfo, setSiteInfo] = useState<SiteInfo | null>(() =>
    getInitialSiteInfo()
  );
  const [totalPages, setTotalPages] = useState(1);
  const [isPaging, startPageTransition] = useTransition();
  const { t } = useTranslation();
  const locale = useI18nStore((state) => state.locale);
  const [searchParams, setSearchParams] = useSearchParams();
  const searchQuery = searchParams.get("q") || "";
  const deferredSearchQuery = useDeferredValue(searchQuery);
  const queryKeyword = useMemo(
    () => deferredSearchQuery.trim(),
    [deferredSearchQuery]
  );
  const currentPage = getPageParam(searchParams.get("page"));
  const dateLocale = getDateLocale(locale);
  const pageIsLoading = loading || isPaging;

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    if (!siteInfo?.name) return;

    document.title = siteInfo.name;

    let active = true;
    const updateSeo = async () => {
      const noteva = await waitForNoteva({ timeout: 3_000 });
      if (!active || !noteva) return;

      noteva.seo.setSiteMeta(
        siteInfo.name,
        siteInfo.description || siteInfo.subtitle || "",
        window.location.origin
      );
    };

    void updateSeo();

    return () => {
      active = false;
    };
  }, [siteInfo?.description, siteInfo?.name, siteInfo?.subtitle]);

  useEffect(() => {
    if (siteInfo) return undefined;

    let active = true;
    const loadSiteInfo = async () => {
      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setSiteInfo({ name: "Noteva", subtitle: "", description: "" });
        return;
      }

      try {
        const info = await noteva.site.getInfo();
        if (!active) return;

        setSiteInfo({
          name: info.name || "Noteva",
          subtitle: info.subtitle || "",
          description: info.description || "",
        });
      } catch {
        if (active) {
          setSiteInfo({ name: "Noteva", subtitle: "", description: "" });
        }
      }
    };

    void loadSiteInfo();

    return () => {
      active = false;
    };
  }, [siteInfo]);

  const loadArticles = useCallback(async () => {
    const requestId = ++articlesRequestIdRef.current;

    const noteva = await waitForNoteva();
    if (!noteva) {
      if (mountedRef.current && requestId === articlesRequestIdRef.current) {
        setArticles([]);
        setTotalPages(1);
        setLoading(false);
      }
      return;
    }

    try {
      const result = await noteva.articles.list({
        page: currentPage,
        pageSize: PAGE_SIZE,
        keyword: queryKeyword || undefined,
      });

      if (mountedRef.current && requestId === articlesRequestIdRef.current) {
        setArticles(result.articles || []);
        setTotalPages(result.totalPages || Math.max(1, Math.ceil((result.total || 0) / PAGE_SIZE)));
      }
    } catch {
      if (mountedRef.current && requestId === articlesRequestIdRef.current) {
        setArticles([]);
        setTotalPages(1);
      }
    } finally {
      if (mountedRef.current && requestId === articlesRequestIdRef.current) {
        setLoading(false);
      }
    }
  }, [currentPage, queryKeyword]);

  useEffect(() => {
    if (mountedRef.current) {
      setLoading(true);
    }

    void loadArticles();
  }, [loadArticles]);

  useEffect(() => {
    if (typeof window === "undefined") return undefined;

    const browser = window as unknown as {
      requestIdleCallback?: (
        callback: IdleRequestCallback,
        options?: IdleRequestOptions
      ) => number;
      cancelIdleCallback?: (handle: number) => void;
      setTimeout: typeof window.setTimeout;
      clearTimeout: typeof window.clearTimeout;
    };

    if (browser.requestIdleCallback && browser.cancelIdleCallback) {
      const id = browser.requestIdleCallback(preloadPostPage, {
        timeout: 2500,
      });

      return () => browser.cancelIdleCallback?.(id);
    }

    const id = browser.setTimeout(preloadPostPage, 1200);
    return () => browser.clearTimeout(id);
  }, []);

  const goToPage = useCallback(
    (page: number) => {
      const nextPage = Math.min(totalPages, Math.max(1, page));
      const params = new URLSearchParams(searchParams);

      if (nextPage <= 1) {
        params.delete("page");
      } else {
        params.set("page", String(nextPage));
      }

      startPageTransition(() => {
        setSearchParams(params);
      });
      window.scrollTo({ top: 0, behavior: "smooth" });
    },
    [searchParams, setSearchParams, totalPages]
  );

  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-4xl py-8 md:py-10">
          <motion.div
            {...themePageHeaderMotion}
            className="mb-8 text-center"
          >
            {siteInfo ? (
              <>
                <h1 className="mx-auto mb-3 max-w-3xl text-3xl font-semibold leading-tight md:text-4xl">
                  {t("home.welcome")} {siteInfo.name}
                </h1>
                <p className="mx-auto max-w-3xl text-base leading-7 text-muted-foreground">
                  {siteInfo.subtitle ||
                    siteInfo.description ||
                    t("home.subtitle")}
                </p>
              </>
            ) : (
              <>
                <div className="mx-auto mb-4 h-10 w-64 rounded skeleton-shimmer" />
                <div className="mx-auto h-6 w-full max-w-md rounded skeleton-shimmer" />
              </>
            )}
          </motion.div>

          {searchQuery ? (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="mb-6 rounded-lg border bg-card/80 px-4 py-3 text-center text-sm text-muted-foreground"
            >
              {t("common.search")}:{" "}
              <span className="font-medium text-foreground">{searchQuery}</span>
            </motion.div>
          ) : null}

          <div className="grid gap-6 article-list">
            {pageIsLoading ? (
              ARTICLE_SKELETON_KEYS.map((key) => (
                <Card key={key} className="overflow-hidden">
                  <CardContent className="p-6">
                    <div className="mb-4 h-6 w-3/4 rounded skeleton-shimmer" />
                    <div className="mb-2 h-4 w-full rounded skeleton-shimmer" />
                    <div className="h-4 w-2/3 rounded skeleton-shimmer" />
                  </CardContent>
                </Card>
              ))
            ) : articles.length === 0 ? (
              <motion.div
                initial={{ opacity: 0, y: 10 }}
                animate={{ opacity: 1, y: 0 }}
                transition={themeSpring}
              >
                <Card className="border-dashed">
                  <CardContent className="flex flex-col items-center justify-center py-14 text-center">
                    <div className="mb-4 flex size-12 items-center justify-center rounded-full bg-muted">
                      <FileText className="size-5 text-muted-foreground" />
                    </div>
                    <p className="text-muted-foreground">
                      {searchQuery ? t("common.noData") : t("home.noPostsYet")}
                    </p>
                  </CardContent>
                </Card>
              </motion.div>
            ) : (
              articles.map((article, index) => (
                <motion.div
                  key={article.id}
                  {...getThemeListItemMotion(index)}
                  whileHover={themeHoverLift}
                >
                  <ArticleSummaryCard
                    article={article}
                    dateLocale={dateLocale}
                    highlightQuery={deferredSearchQuery}
                    priorityImage={index < 2}
                    onWarmRoute={preloadPostPage}
                  />
                </motion.div>
              ))
            )}
          </div>

          {!pageIsLoading && totalPages > 1 ? (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.2 }}
              className="mt-8 flex items-center justify-center gap-2"
            >
              <Button
                variant="outline"
                size="sm"
                onClick={() => goToPage(currentPage - 1)}
                disabled={currentPage <= 1 || isPaging}
              >
                <ChevronLeft className="mr-1 h-4 w-4" />
                {t("pagination.prev")}
              </Button>
              <span className="px-3 text-sm text-muted-foreground">
                {t("pagination.page")
                  .replace("{current}", String(currentPage))
                  .replace("{total}", String(totalPages))}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => goToPage(currentPage + 1)}
                disabled={currentPage >= totalPages || isPaging}
              >
                {t("pagination.next")}
                <ChevronRight className="ml-1 h-4 w-4" />
              </Button>
            </motion.div>
          ) : null}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
