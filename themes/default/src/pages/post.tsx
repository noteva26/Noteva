import { lazy, Suspense, useEffect, useState } from "react";
import { useNavigate, useLocation, Link } from "react-router-dom";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import PluginSlot from "@/components/plugin-slot";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import {
  ArrowLeft,
  ArrowRight,
  BookOpen,
  Calendar,
  Clock,
  Eye,
  Folder,
  Heart,
  MessageSquare,
  Tag,
} from "lucide-react";
import { useTranslation, useI18nStore } from "@/lib/i18n";
import { toast } from "sonner";
import {
  getArticleUrl,
  getCategoryUrl,
  getTagUrl,
  waitForNoteva,
  type NotevaArticle,
} from "@/hooks/useNoteva";
import { TocSidebar } from "@/components/toc-sidebar";
import { cn } from "@/lib/utils";
import { sanitizeHtml } from "@/lib/sanitize-html";

const Comments = lazy(() =>
  import("@/components/comments").then((module) => ({ default: module.Comments }))
);

type Article = NotevaArticle;

function CommentsFallback() {
  return (
    <div className="mt-8 rounded-lg border bg-card p-6">
      <div className="mb-5 h-6 w-32 rounded skeleton-shimmer" />
      <div className="space-y-3">
        <div className="h-20 rounded-md skeleton-shimmer" />
        <div className="h-9 w-28 rounded-md skeleton-shimmer" />
      </div>
    </div>
  );
}

export default function PostPage() {
  const navigate = useNavigate();
  const location = useLocation();
  // Extract slug from /posts/xxx (supports nested like /posts/2025/01/hello)
  const slug = location.pathname.replace(/^\/posts\//, '').replace(/\/$/, '');

  const [article, setArticle] = useState<Article | null>(null);
  const [siteInfo, setSiteInfo] = useState({ name: "Noteva" });
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);
  const [isLiked, setIsLiked] = useState(false);
  const [likeCount, setLikeCount] = useState(0);
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);

  // OG/SEO meta tags via SDK
  useEffect(() => {
    if (!article || !siteInfo.name) return;

    let active = true;
    const updateSeo = async () => {
      const Noteva = await waitForNoteva({ timeout: 3_000 });
      if (!active || !Noteva) return;

      Noteva.seo.setArticleMeta({
        title: article.title,
        excerpt: article.content?.slice(0, 160)?.replace(/[#*`>\n]/g, "").trim(),
        thumbnail: article.thumbnail ?? undefined,
        slug: article.slug || String(article.id),
        publishedAt: article.publishedAt ?? undefined,
      }, siteInfo.name, window.location.origin);
    };

    void updateSeo();

    return () => {
      active = false;
    };
  }, [article, siteInfo.name]);

  useEffect(() => {
    if (!slug) { setNotFound(true); setLoading(false); return; }

    let active = true;
    const loadSiteInfo = async () => {
      const Noteva = await waitForNoteva();
      if (!active || !Noteva) return;

      try {
        const info = await Noteva.site.getInfo();
        if (active) setSiteInfo({ name: info.name || "Noteva" });
      } catch { }
    };
    void loadSiteInfo();

    return () => {
      active = false;
    };
  }, [slug]);

  useEffect(() => {
    if (article) {
      document.title = `${article.title} - ${siteInfo.name}`;
    }
  }, [article, siteInfo.name]);

  useEffect(() => {
    if (!slug) return;

    let active = true;
    const loadArticle = async () => {
      setLoading(true);
      setNotFound(false);

      const Noteva = await waitForNoteva();
      if (!active) return;

      if (!Noteva) {
        setNotFound(true);
        setLoading(false);
        return;
      }

      try {
        const data = await Noteva.articles.get(slug);
        if (!active) return;

        setArticle(data);
        setLikeCount(data.likeCount);

        try {
          const likeResult = await Noteva.interactions.checkLike("article", data.id);
          if (active) setIsLiked(likeResult.liked);
        } catch {
          if (active) setIsLiked(false);
        }

        await Noteva.articles.incrementView(data.id);
      } catch {
        if (active) setNotFound(true);
      } finally {
        if (active) setLoading(false);
      }
    };

    void loadArticle();

    return () => {
      active = false;
    };
  }, [slug]);

  const handleLike = async () => {
    if (!article) return;
    const Noteva = await waitForNoteva();
    if (!Noteva) return;
    try {
      const result = await Noteva.interactions.like("article", article.id);
      setIsLiked(result.liked);
      setLikeCount(result.likeCount);
      toast.success(result.liked ? t("comment.liked") : t("comment.unliked"));
    } catch { toast.error(t("comment.likeFailed")); }
  };

  const getDateLocale = () => {
    const candidate = locale === "en" ? "en-US" : locale;
    try {
      new Intl.DateTimeFormat(candidate);
      return candidate;
    } catch {
      return "zh-CN";
    }
  };

  if (loading) {
    return (
      <div className="theme-page-shell relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container mx-auto max-w-[900px] py-8">
            <Skeleton className="mb-4 h-10 w-3/4" />
            <Skeleton className="mb-6 h-6 w-1/2" />
            <Skeleton className="h-72 w-full" />
          </div>
        </main>
        <SiteFooter />
      </div>
    );
  }

  if (notFound || !article) {
    return (
      <div className="theme-page-shell relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container mx-auto max-w-4xl py-16 text-center">
            <h1 className="mb-4 text-4xl font-semibold">{t("error.notFound")}</h1>
            <p className="mb-8 text-muted-foreground">{t("error.notFoundDesc")}</p>
            <Button onClick={() => navigate("/")}>{t("error.backHome")}</Button>
          </div>
        </main>
        <SiteFooter />
      </div>
    );
  }

  const stats = {
    views: article.viewCount || 0,
    likes: likeCount,
    comments: article.commentCount || 0,
  };
  const articleHtml = sanitizeHtml(article.html);
  const thumbnail = article.thumbnail;
  const publishedAt = article.publishedAt || "";
  const hasReadableToc =
    (article.toc?.filter((item) => item.level >= 2 && item.level <= 3).length || 0) >= 2;

  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div
          className={cn(
            "container mx-auto grid gap-10 py-8",
            hasReadableToc
              ? "max-w-7xl xl:grid-cols-[minmax(0,900px)_17rem] xl:gap-14"
              : "max-w-[900px]"
          )}
        >
          <article
            className={cn("min-w-0", hasReadableToc && "xl:justify-self-end")}
            data-article-id={article.id}
          >
            <header className="mb-6">
              <h1 className="mb-4 text-4xl font-semibold leading-tight md:text-[2.75rem]">
                {article.title}
              </h1>
              <div className="article-meta flex flex-wrap items-center gap-x-3.5 gap-y-1.5 text-sm text-muted-foreground">
                {publishedAt ? (
                  <span className="flex items-center gap-1">
                    <Calendar className="h-4 w-4" />
                    {new Date(publishedAt).toLocaleDateString(getDateLocale(), { year: "numeric", month: "long", day: "numeric" })}
                  </span>
                ) : null}
                {article.category && (
                  <Link to={getCategoryUrl(article.category)} className="flex items-center gap-1 hover:text-foreground transition-colors">
                    <Folder className="h-4 w-4" />{article.category.name}
                  </Link>
                )}
                {(article.wordCount ?? 0) > 0 && (
                  <span className="flex items-center gap-1"><BookOpen className="h-4 w-4" />{article.wordCount}{t("article.words")}</span>
                )}
                {(article.readingTime ?? 0) > 0 && (
                  <span className="flex items-center gap-1"><Clock className="h-4 w-4" />{article.readingTime}{t("article.minRead")}</span>
                )}
                <span className="flex items-center gap-1"><Eye className="h-4 w-4" />{stats.views + 1}</span>
                <span className="flex items-center gap-1"><MessageSquare className="h-4 w-4" />{stats.comments}</span>
              </div>
            </header>

            {thumbnail ? (
              <div className="mb-6 overflow-hidden rounded-lg border bg-muted">
                <img
                  src={thumbnail}
                  alt={article.title}
                  className="max-h-[460px] w-full object-cover"
                />
              </div>
            ) : null}

            <Card className="article-card overflow-hidden">
              <CardContent className="prose dark:prose-invert max-w-none p-5 md:p-7 [&_img.twemoji]:!w-[1.2em] [&_img.twemoji]:!h-[1.2em] [&_img.twemoji]:!inline-block [&_img.twemoji]:!m-0 [&_img.twemoji]:!align-[-0.1em] [&_img.emoji]:!w-[1.2em] [&_img.emoji]:!h-[1.2em] [&_img.emoji]:!inline-block [&_img.emoji]:!m-0 [&_img.emoji]:!align-[-0.1em]">
                <PluginSlot name="article_content_top" />
                <div className="article-content" dangerouslySetInnerHTML={{ __html: articleHtml }} />
                <PluginSlot name="article_content_bottom" />
              </CardContent>
            </Card>

            <PluginSlot name="article_after_content" className="my-4" />

            <div className="mt-5 flex flex-wrap items-center justify-between gap-4">
              <div className="flex flex-wrap items-center gap-2">
                {article.category && (
                  <Link to={getCategoryUrl(article.category)}>
                    <Badge variant="outline" className="hover:bg-secondary"><Folder className="h-3 w-3 mr-1" />{article.category.name}</Badge>
                  </Link>
                )}
                {article.tags && article.tags.map((tag) => (
                  <Link key={tag.id} to={getTagUrl(tag)}>
                    <Badge variant="secondary" className="hover:bg-secondary/80"><Tag className="h-3 w-3 mr-1" />{tag.name}</Badge>
                  </Link>
                ))}
              </div>
              <Button variant={isLiked ? "default" : "outline"} size="sm" onClick={handleLike} className="gap-2">
                <Heart className={`h-4 w-4 ${isLiked ? "fill-current" : ""}`} />{likeCount}
              </Button>
            </div>

            {(article.prev || article.next) && (
              <div className="mt-7 grid grid-cols-1 gap-3 sm:grid-cols-2">
                {article.prev ? (
                  <Link to={getArticleUrl(article.prev)} className="group flex items-center gap-2 rounded-lg border bg-card p-4 transition-colors hover:border-primary/60 hover:bg-muted/40">
                    <ArrowLeft className="h-4 w-4 shrink-0 text-muted-foreground group-hover:text-foreground transition-colors" />
                    <div className="min-w-0">
                      <div className="text-xs text-muted-foreground">{t("article.prev")}</div>
                      <div className="text-sm font-medium truncate">{article.prev.title}</div>
                    </div>
                  </Link>
                ) : <div />}
                {article.next && (
                  <Link to={getArticleUrl(article.next)} className="group flex items-center justify-end gap-2 rounded-lg border bg-card p-4 text-right transition-colors hover:border-primary/60 hover:bg-muted/40">
                    <div className="min-w-0">
                      <div className="text-xs text-muted-foreground">{t("article.next")}</div>
                      <div className="text-sm font-medium truncate">{article.next.title}</div>
                    </div>
                    <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground group-hover:text-foreground transition-colors" />
                  </Link>
                )}
              </div>
            )}

            {article.related && article.related.length > 0 && (
              <div className="mt-7">
                <h3 className="text-lg font-semibold mb-3">{t("article.related")}</h3>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                  {article.related.map((item) => (
                    <Link key={item.id} to={getArticleUrl(item)} className="block rounded-lg border bg-card p-3 transition-colors hover:border-primary/60 hover:bg-muted/40">
                      <span className="text-sm font-medium line-clamp-2">{item.title}</span>
                    </Link>
                  ))}
                </div>
              </div>
            )}

            <Suspense fallback={<CommentsFallback />}>
              <Comments articleId={article.id} />
            </Suspense>
          </article>
          {hasReadableToc && article.toc && (
            <TocSidebar toc={article.toc} />
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
