import { useEffect, useState } from "react";
import { useNavigate, useLocation, Link } from "react-router-dom";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Comments } from "@/components/comments";
import PluginSlot from "@/components/plugin-slot";
import { Card, CardContent } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { Calendar, Folder, ArrowLeft, ArrowRight, Tag, Eye, Heart, MessageSquare, BookOpen, Clock } from "lucide-react";
import { useTranslation, useI18nStore } from "@/lib/i18n";
import { toast } from "sonner";
import { getNoteva, getArticleUrl } from "@/hooks/useNoteva";
import { TocSidebar } from "@/components/toc-sidebar";

interface Article {
  id: number;
  slug: string;
  title: string;
  content: string;
  content_html?: string;
  html?: string;
  word_count?: number;
  reading_time?: number;
  prev?: { id: number; slug: string; title: string } | null;
  next?: { id: number; slug: string; title: string } | null;
  related?: { id: number; slug: string; title: string; thumbnail?: string }[];
  category?: { id: number; name: string; slug: string };
  tags?: { id: number; name: string; slug: string }[];
  toc?: { level: number; text: string; id: string }[];
  thumbnail?: string | null;
  [key: string]: any;
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
    const Noteva = getNoteva();
    if (!Noteva) return;
    Noteva.seo.setArticleMeta({
      title: article.title,
      excerpt: article.content?.slice(0, 160)?.replace(/[#*`>\n]/g, '').trim(),
      thumbnail: article.thumbnail ?? undefined,
      slug: article.slug || String(article.id),
      published_at: (article.published_at ?? article.publishedAt) ?? undefined,
    }, siteInfo.name, window.location.origin);
  }, [article, siteInfo.name]);

  useEffect(() => {
    if (!slug) { setNotFound(true); setLoading(false); return; }

    const loadSiteInfo = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadSiteInfo, 50); return; }
      try {
        const info = await Noteva.site.getInfo();
        setSiteInfo({ name: info.name || "Noteva" });
      } catch { }
    };
    loadSiteInfo();
  }, []);

  useEffect(() => {
    if (article) {
      document.title = `${article.title} - ${siteInfo.name}`;
    }
  }, [article, siteInfo.name]);

  useEffect(() => {
    if (!slug) return;

    const loadArticle = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadArticle, 50); return; }

      try {
        const data = await Noteva.articles.get(slug);
        setArticle(data);
        setLikeCount(Noteva.articles.getStats(data).likes);

        try {
          const likeResult = await Noteva.interactions.checkLike('article', data.id);
          setIsLiked(likeResult.liked);
        } catch { setIsLiked(false); }

        await Noteva.articles.incrementView(data.id);
      } catch (err) {
        console.error(err);
        setNotFound(true);
      } finally {
        setLoading(false);
      }
    };
    loadArticle();
  }, [slug]);

  const handleLike = async () => {
    if (!article) return;
    const Noteva = getNoteva();
    if (!Noteva) return;
    try {
      const result = await Noteva.interactions.like('article', article.id);
      setIsLiked(result.liked);
      setLikeCount(result.like_count);
      toast.success(result.liked ? t("comment.liked") : t("comment.unliked"));
    } catch { toast.error(t("comment.likeFailed")); }
  };

  const getDateLocale = () => {
    switch (locale) { case "zh-TW": return "zh-TW"; case "en": return "en-US"; default: return "zh-CN"; }
  };

  const Noteva = getNoteva();

  if (loading) {
    return (
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1"><div className="container py-8 max-w-4xl mx-auto">
          <Skeleton className="h-10 w-3/4 mb-4" /><Skeleton className="h-6 w-1/2 mb-8" /><Skeleton className="h-64 w-full" />
        </div></main>
        <SiteFooter />
      </div>
    );
  }

  if (notFound || !article) {
    return (
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1"><div className="container py-16 text-center max-w-4xl mx-auto">
          <h1 className="text-4xl font-bold mb-4">{t("error.notFound")}</h1>
          <p className="text-muted-foreground mb-8">{t("error.notFoundDesc")}</p>
          <Button onClick={() => navigate("/")}>{t("error.backHome")}</Button>
        </div></main>
        <SiteFooter />
      </div>
    );
  }

  return (
    <div className="relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container py-8 max-w-6xl mx-auto flex gap-8">
          <article className="flex-1 min-w-0 max-w-4xl">
            <Button variant="ghost" size="sm" className="mb-6" onClick={() => navigate(-1)}>
              <ArrowLeft className="h-4 w-4 mr-2" />{t("common.back")}
            </Button>

            <header className="mb-8">
              <h1 className="text-4xl font-bold mb-4">{article.title}</h1>
              <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
                <span className="flex items-center gap-1">
                  <Calendar className="h-4 w-4" />
                  {new Date(Noteva?.articles.getDate(article) || '').toLocaleDateString(getDateLocale(), { year: "numeric", month: "long", day: "numeric" })}
                </span>
                {article.category && (
                  <Link to={`/categories?c=${article.category.slug}`} className="flex items-center gap-1 hover:text-foreground transition-colors">
                    <Folder className="h-4 w-4" />{article.category.name}
                  </Link>
                )}
                {(article.word_count ?? 0) > 0 && (
                  <span className="flex items-center gap-1"><BookOpen className="h-4 w-4" />{article.word_count}{t("article.words")}</span>
                )}
                {(article.reading_time ?? 0) > 0 && (
                  <span className="flex items-center gap-1"><Clock className="h-4 w-4" />{article.reading_time}{t("article.minRead")}</span>
                )}
                <span className="flex items-center gap-1"><Eye className="h-4 w-4" />{(Noteva?.articles.getStats(article).views ?? 0) + 1}</span>
                <span className="flex items-center gap-1"><MessageSquare className="h-4 w-4" />{Noteva?.articles.getStats(article).comments ?? 0}</span>
              </div>
            </header>

            <Card>
              <CardContent className="prose dark:prose-invert max-w-none p-6 md:p-8 [&_img.twemoji]:!w-[1.2em] [&_img.twemoji]:!h-[1.2em] [&_img.twemoji]:!inline-block [&_img.twemoji]:!m-0 [&_img.twemoji]:!align-[-0.1em] [&_img.emoji]:!w-[1.2em] [&_img.emoji]:!h-[1.2em] [&_img.emoji]:!inline-block [&_img.emoji]:!m-0 [&_img.emoji]:!align-[-0.1em]">
                <PluginSlot name="article_content_top" />
                <div dangerouslySetInnerHTML={{ __html: Noteva?.articles.getHtml(article) || '' }} />
                <PluginSlot name="article_content_bottom" />
              </CardContent>
            </Card>

            <PluginSlot name="article_after_content" className="my-4" />

            <div className="mt-6 flex flex-wrap items-center justify-between gap-4">
              <div className="flex flex-wrap items-center gap-2">
                {article.category && (
                  <Link to={`/categories?c=${article.category.slug}`}>
                    <Badge variant="outline" className="hover:bg-secondary"><Folder className="h-3 w-3 mr-1" />{article.category.name}</Badge>
                  </Link>
                )}
                {article.tags && article.tags.map((tag) => (
                  <Link key={tag.id} to={`/tags?t=${tag.slug}`}>
                    <Badge variant="secondary" className="hover:bg-secondary/80"><Tag className="h-3 w-3 mr-1" />{tag.name}</Badge>
                  </Link>
                ))}
              </div>
              <Button variant={isLiked ? "default" : "outline"} size="sm" onClick={handleLike} className="gap-2">
                <Heart className={`h-4 w-4 ${isLiked ? "fill-current" : ""}`} />{likeCount}
              </Button>
            </div>

            {/* Prev / Next navigation */}
            {(article.prev || article.next) && (
              <div className="mt-8 grid grid-cols-1 sm:grid-cols-2 gap-4">
                {article.prev ? (
                  <Link to={getArticleUrl(article.prev)} className="group flex items-center gap-2 p-4 rounded-lg border hover:bg-muted/50 transition-colors">
                    <ArrowLeft className="h-4 w-4 shrink-0 text-muted-foreground group-hover:text-foreground transition-colors" />
                    <div className="min-w-0">
                      <div className="text-xs text-muted-foreground">{t("article.prev")}</div>
                      <div className="text-sm font-medium truncate">{article.prev.title}</div>
                    </div>
                  </Link>
                ) : <div />}
                {article.next && (
                  <Link to={getArticleUrl(article.next)} className="group flex items-center justify-end gap-2 p-4 rounded-lg border hover:bg-muted/50 transition-colors text-right">
                    <div className="min-w-0">
                      <div className="text-xs text-muted-foreground">{t("article.next")}</div>
                      <div className="text-sm font-medium truncate">{article.next.title}</div>
                    </div>
                    <ArrowRight className="h-4 w-4 shrink-0 text-muted-foreground group-hover:text-foreground transition-colors" />
                  </Link>
                )}
              </div>
            )}

            {/* Related articles */}
            {article.related && article.related.length > 0 && (
              <div className="mt-8">
                <h3 className="text-lg font-semibold mb-3">{t("article.related")}</h3>
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
                  {article.related.map((item) => (
                    <Link key={item.id} to={getArticleUrl(item)} className="block p-3 rounded-lg border hover:bg-muted/50 transition-colors">
                      <span className="text-sm font-medium line-clamp-2">{item.title}</span>
                    </Link>
                  ))}
                </div>
              </div>
            )}

            <Comments articleId={article.id} />
          </article>
          {article.toc && article.toc.length > 0 && (
            <TocSidebar toc={article.toc} />
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
