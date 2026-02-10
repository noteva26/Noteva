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
import { Calendar, Folder, ArrowLeft, Tag, Eye, Heart, MessageSquare } from "lucide-react";
import { useTranslation, useI18nStore } from "@/lib/i18n";
import { toast } from "sonner";
import { getNoteva } from "@/hooks/useNoteva";

interface Article {
  id: number;
  slug: string;
  title: string;
  content: string;
  content_html?: string;
  html?: string;
  published_at?: string;
  publishedAt?: string;
  created_at?: string;
  createdAt?: string;
  view_count?: number;
  viewCount?: number;
  like_count?: number;
  likeCount?: number;
  comment_count?: number;
  commentCount?: number;
  category?: { id: number; name: string; slug: string };
  tags?: { id: number; name: string; slug: string }[];
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

  useEffect(() => {
    if (!slug) { setNotFound(true); setLoading(false); return; }

    const loadSiteInfo = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadSiteInfo, 50); return; }
      try {
        const info = await Noteva.site.getInfo();
        setSiteInfo({ name: info.name || "Noteva" });
      } catch {}
    };
    loadSiteInfo();
  }, []);

  useEffect(() => {
    if (article) {
      document.title = `${article.title} - ${siteInfo.name}`;
    }
  }, [article, siteInfo.name]);

  // 文章内容渲染到 DOM 后，通知插件处理内容（视频嵌入等）
  useEffect(() => {
    if (!article || loading) return;
    // requestAnimationFrame 确保 React 已经把 HTML 渲染到 DOM
    requestAnimationFrame(() => {
      const Noteva = (window as any).Noteva;
      if (Noteva?.hooks) {
        Noteva.hooks.trigger('content_render', {
          path: window.location.pathname,
          articleId: article.id,
        });
      }
    });
  }, [article, loading]);

  useEffect(() => {
    if (!slug) return;
    
    const loadArticle = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadArticle, 50); return; }

      try {
        const data = await Noteva.articles.get(slug);
        setArticle(data);
        setLikeCount((data as any).like_count ?? data.likeCount ?? 0);
        
        try {
          const likeResult = await Noteva.api.get(`/like/check?target_type=article&target_id=${data.id}`);
          setIsLiked(likeResult.liked);
        } catch { setIsLiked(false); }
        
        try { await Noteva.api.post(`/view/${data.id}`); } catch {}
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
      const result = await Noteva.api.post('/like', { target_type: 'article', target_id: article.id });
      setIsLiked(result.liked);
      setLikeCount(result.like_count);
      toast.success(result.liked ? t("comment.liked") : t("comment.unliked"));
    } catch { toast.error(t("comment.likeFailed")); }
  };

  const getDateLocale = () => {
    switch (locale) { case "zh-TW": return "zh-TW"; case "en": return "en-US"; default: return "zh-CN"; }
  };

  const getPublishedDate = (a: Article) => a.published_at || a.publishedAt || a.created_at || a.createdAt || "";
  const getViewCount = (a: Article) => (a.view_count ?? a.viewCount ?? 0) + 1;
  const getCommentCount = (a: Article) => a.comment_count ?? a.commentCount ?? 0;
  const getHtml = (a: Article) => a.content_html || a.html || "";

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
        <article className="container py-8 max-w-4xl mx-auto">
          <Button variant="ghost" size="sm" className="mb-6" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-4 w-4 mr-2" />{t("common.back")}
          </Button>

          <header className="mb-8">
            <h1 className="text-4xl font-bold mb-4">{article.title}</h1>
            <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
              <span className="flex items-center gap-1">
                <Calendar className="h-4 w-4" />
                {new Date(getPublishedDate(article)).toLocaleDateString(getDateLocale(), { year: "numeric", month: "long", day: "numeric" })}
              </span>
              {article.category && (
                <Link to={`/categories?c=${article.category.slug}`} className="flex items-center gap-1 hover:text-foreground transition-colors">
                  <Folder className="h-4 w-4" />{article.category.name}
                </Link>
              )}
              <span className="flex items-center gap-1"><Eye className="h-4 w-4" />{getViewCount(article)}</span>
              <span className="flex items-center gap-1"><MessageSquare className="h-4 w-4" />{getCommentCount(article)}</span>
            </div>
          </header>

          <Card>
            <CardContent className="prose prose-lg dark:prose-invert max-w-none p-6 md:p-8">
              <PluginSlot name="article_content_top" />
              <div dangerouslySetInnerHTML={{ __html: getHtml(article) }} />
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

          <Comments articleId={article.id} />
        </article>
      </main>
      <SiteFooter />
    </div>
  );
}
