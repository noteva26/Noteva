"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
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

// 文章类型（兼容 SDK 和原 API）
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

function getSlugFromPath(): string {
  if (typeof window === 'undefined') return "";
  const path = window.location.pathname;
  const match = path.match(/\/posts\/(.+)/);
  return match ? match[1].replace(/\/$/, '') : "";
}

export default function PostClient() {
  const router = useRouter();
  const [slug, setSlug] = useState("");
  const [article, setArticle] = useState<Article | null>(null);
  const [siteInfo, setSiteInfo] = useState({ name: "Noteva" });
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);
  const [isLiked, setIsLiked] = useState(false);
  const [likeCount, setLikeCount] = useState(0);
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);

  useEffect(() => {
    const s = getSlugFromPath();
    setSlug(s);
    if (!s) {
      setNotFound(true);
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    // 加载站点信息
    const loadSiteInfo = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        setTimeout(loadSiteInfo, 50);
        return;
      }
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

  useEffect(() => {
    if (!slug) return;
    
    const loadArticle = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        setTimeout(loadArticle, 50);
        return;
      }

      try {
        // 使用 SDK 获取文章
        const data = await Noteva.articles.get(slug);
        setArticle(data);
        // 兼容 snake_case 和 camelCase
        setLikeCount((data as any).like_count ?? data.likeCount ?? 0);
        
        // 检查是否已点赞（使用原 API，SDK 暂不支持）
        try {
          const likeResult = await Noteva.api.get(`/like/check?target_type=article&target_id=${data.id}`);
          setIsLiked(likeResult.liked);
        } catch {
          setIsLiked(false);
        }
        
        // 增加浏览量
        try {
          await Noteva.api.post(`/view/${data.id}`);
        } catch {}
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
      const result = await Noteva.api.post('/like', { 
        target_type: 'article', 
        target_id: article.id 
      });
      setIsLiked(result.liked);
      setLikeCount(result.like_count);
      toast.success(result.liked ? t("comment.liked") : t("comment.unliked"));
    } catch (err) {
      toast.error(t("comment.likeFailed"));
    }
  };

  const getDateLocale = () => {
    switch (locale) {
      case "zh-TW": return "zh-TW";
      case "en": return "en-US";
      default: return "zh-CN";
    }
  };

  // 兼容字段名
  const getPublishedDate = (a: Article) => 
    a.published_at || a.publishedAt || a.created_at || a.createdAt || "";
  const getViewCount = (a: Article) => (a.view_count ?? a.viewCount ?? 0) + 1;
  const getCommentCount = (a: Article) => a.comment_count ?? a.commentCount ?? 0;
  const getHtml = (a: Article) => a.content_html || a.html || "";

  if (loading) {
    return (
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container py-8 max-w-4xl mx-auto">
            <Skeleton className="h-10 w-3/4 mb-4" />
            <Skeleton className="h-6 w-1/2 mb-8" />
            <Skeleton className="h-64 w-full" />
          </div>
        </main>
        <SiteFooter />
      </div>
    );
  }

  if (notFound || !article) {
    return (
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container py-16 text-center max-w-4xl mx-auto">
            <h1 className="text-4xl font-bold mb-4">{t("error.notFound")}</h1>
            <p className="text-muted-foreground mb-8">{t("error.notFoundDesc")}</p>
            <Button onClick={() => router.push("/")}>{t("error.backHome")}</Button>
          </div>
        </main>
        <SiteFooter />
      </div>
    );
  }

  return (
    <div className="relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <article className="container py-8 max-w-4xl mx-auto">
          <Button
            variant="ghost"
            size="sm"
            className="mb-6"
            onClick={() => router.back()}
          >
            <ArrowLeft className="h-4 w-4 mr-2" />
            {t("common.back")}
          </Button>

          <header className="mb-8">
            <h1 className="text-4xl font-bold mb-4">{article.title}</h1>
            <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
              <span className="flex items-center gap-1">
                <Calendar className="h-4 w-4" />
                {new Date(getPublishedDate(article)).toLocaleDateString(getDateLocale(), {
                  year: "numeric",
                  month: "long",
                  day: "numeric",
                })}
              </span>
              {article.category && (
                <Link 
                  href={`/categories?c=${article.category.slug}`}
                  className="flex items-center gap-1 hover:text-foreground transition-colors"
                >
                  <Folder className="h-4 w-4" />
                  {article.category.name}
                </Link>
              )}
              <span className="flex items-center gap-1">
                <Eye className="h-4 w-4" />
                {getViewCount(article)}
              </span>
              <span className="flex items-center gap-1">
                <MessageSquare className="h-4 w-4" />
                {getCommentCount(article)}
              </span>
            </div>
          </header>

          <Card>
            <CardContent className="prose prose-lg dark:prose-invert max-w-none p-6 md:p-8">
              {/* article_content_top 插槽 - 警告提示、过期提醒 */}
              <PluginSlot name="article_content_top" />
              
              {/* 使用后端渲染的 HTML（包含 shortcode 处理结果） */}
              <div dangerouslySetInnerHTML={{ __html: getHtml(article) }} />
              
              {/* article_content_bottom 插槽 - 版权声明、相关推荐 */}
              <PluginSlot name="article_content_bottom" />
            </CardContent>
          </Card>

          {/* article_after_content 插槽 - 打赏、分享、点赞 */}
          <PluginSlot name="article_after_content" className="my-4" />

          <div className="mt-6 flex flex-wrap items-center justify-between gap-4">
            <div className="flex flex-wrap items-center gap-2">
              {article.category && (
                <Link href={`/categories?c=${article.category.slug}`}>
                  <Badge variant="outline" className="hover:bg-secondary">
                    <Folder className="h-3 w-3 mr-1" />
                    {article.category.name}
                  </Badge>
                </Link>
              )}
              {article.tags && article.tags.map((tag) => (
                <Link key={tag.id} href={`/tags?t=${tag.slug}`}>
                  <Badge variant="secondary" className="hover:bg-secondary/80">
                    <Tag className="h-3 w-3 mr-1" />
                    {tag.name}
                  </Badge>
                </Link>
              ))}
            </div>
            <Button
              variant={isLiked ? "default" : "outline"}
              size="sm"
              onClick={handleLike}
              className="gap-2"
            >
              <Heart className={`h-4 w-4 ${isLiked ? "fill-current" : ""}`} />
              {likeCount}
            </Button>
          </div>

          <Comments articleId={article.id} />
        </article>
      </main>
      <SiteFooter />
    </div>
  );
}
