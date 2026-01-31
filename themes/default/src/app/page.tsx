"use client";

import { useEffect, useState, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
import Image from "next/image";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Calendar, Folder, Eye, Heart, MessageSquare, Tag, Pin } from "lucide-react";
import { useTranslation, useI18nStore } from "@/lib/i18n";
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
  is_pinned?: boolean;
  isPinned?: boolean;
  thumbnail?: string;
  category?: { id: number; name: string; slug: string };
  tags?: { id: number; name: string; slug: string }[];
}

// Extract first image URL from markdown content
function extractFirstImage(content: string): string | null {
  const imgRegex = /!\[.*?\]\((.*?)\)/;
  const match = content.match(imgRegex);
  return match ? match[1] : null;
}

// Strip shortcodes and markdown for plain text excerpt
function getExcerpt(content: string, maxLength: number = 200): string {
  // Remove shortcodes like [hide-until-reply]...[/hide-until-reply]
  let text = content.replace(/\[([a-zA-Z0-9_-]+)(?:\s+[^\]]*)?]([\s\S]*?)\[\/\1]/g, '');
  // Remove self-closing shortcodes like [video url="..." /]
  text = text.replace(/\[[a-zA-Z0-9_-]+(?:\s+[^\]]*)?\/]/g, '');
  // Remove remaining shortcode tags
  text = text.replace(/\[\/?\w+[^\]]*]/g, '');
  // Remove markdown images ![alt](url) or ![](url)
  text = text.replace(/!\[[^\]]*\]\([^)]+\)/g, '');
  // Remove non-standard image syntax like !(url)
  text = text.replace(/!\([^)]+\)/g, '');
  // Remove HTML img tags
  text = text.replace(/<img[^>]*>/gi, '');
  // Remove markdown links but keep text
  text = text.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
  // Remove markdown formatting
  text = text.replace(/[*_~`#]+/g, '');
  // Remove HTML tags
  text = text.replace(/<[^>]+>/g, '');
  // Collapse whitespace
  text = text.replace(/\s+/g, ' ').trim();
  
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

function HomeContent() {
  const [articles, setArticles] = useState<Article[]>([]);
  const [loading, setLoading] = useState(true);
  const [siteInfo, setSiteInfo] = useState({ name: "Noteva", subtitle: "", description: "" });
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);
  const searchParams = useSearchParams();
  const searchQuery = searchParams.get("q") || "";

  useEffect(() => {
    // 使用 SDK 加载数据
    const loadData = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        // SDK 还没加载，等待
        setTimeout(loadData, 50);
        return;
      }

      try {
        // 加载站点信息
        const info = await Noteva.site.getInfo();
        setSiteInfo({
          name: info.name || "Noteva",
          subtitle: info.subtitle || "",
          description: info.description || "",
        });
        document.title = info.name || "Noteva";

        // 加载文章列表
        const result = await Noteva.articles.list({ pageSize: 20 });
        setArticles(result.articles || []);
      } catch (err) {
        console.error("Failed to load data:", err);
        setArticles([]);
      } finally {
        setLoading(false);
      }
    };

    loadData();
  }, []);

  const getDateLocale = () => {
    switch (locale) {
      case "zh-TW": return "zh-TW";
      case "en": return "en-US";
      default: return "zh-CN";
    }
  };

  const filteredArticles = articles.filter((article) =>
    article.title.toLowerCase().includes(searchQuery.toLowerCase()) ||
    article.content.toLowerCase().includes(searchQuery.toLowerCase()) ||
    article.tags?.some(tag => tag.name.toLowerCase().includes(searchQuery.toLowerCase())) ||
    article.category?.name.toLowerCase().includes(searchQuery.toLowerCase())
  );

  // Get thumbnail: use article.thumbnail if set, otherwise extract first image from content
  const getThumbnail = (article: Article): string | null => {
    if (article.thumbnail) return article.thumbnail;
    return extractFirstImage(article.content);
  };

  // 兼容 SDK 和原 API 的字段名
  const getPublishedDate = (article: Article) => 
    article.published_at || article.publishedAt || article.created_at || article.createdAt || "";
  const getViewCount = (article: Article) => article.view_count ?? article.viewCount ?? 0;
  const getLikeCount = (article: Article) => article.like_count ?? article.likeCount ?? 0;
  const getCommentCount = (article: Article) => article.comment_count ?? article.commentCount ?? 0;
  const isPinned = (article: Article) => article.is_pinned || article.isPinned;

  return (
    <div className="relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container py-8 max-w-4xl mx-auto">
          <div className="mb-8 text-center">
            <h1 className="text-4xl font-bold mb-2">{t("home.welcome")} {siteInfo.name}</h1>
            <p className="text-muted-foreground text-lg">
              {siteInfo.subtitle || siteInfo.description || t("home.subtitle")}
            </p>
          </div>

          {searchQuery && (
            <div className="mb-6 text-center">
              <p className="text-muted-foreground">
                {t("common.search")}: <span className="font-medium text-foreground">{searchQuery}</span>
              </p>
            </div>
          )}

          <div className="grid gap-6">
            {loading ? (
              Array.from({ length: 3 }).map((_, i) => (
                <Card key={i}>
                  <CardHeader>
                    <Skeleton className="h-6 w-3/4" />
                  </CardHeader>
                  <CardContent>
                    <Skeleton className="h-4 w-full mb-2" />
                    <Skeleton className="h-4 w-2/3" />
                  </CardContent>
                </Card>
              ))
            ) : filteredArticles.length === 0 ? (
              <Card>
                <CardContent className="py-12 text-center text-muted-foreground">
                  {searchQuery ? t("common.noData") : t("home.noPostsYet")}
                </CardContent>
              </Card>
            ) : (
              filteredArticles.map((article) => {
                const thumbnail = getThumbnail(article);
                return (
                  <Card key={article.id} className="hover:shadow-md transition-shadow overflow-hidden">
                    <div className="flex">
                      <div className="flex-1">
                        <CardHeader>
                          <div className="flex items-center gap-2">
                            {isPinned(article) && (
                              <Badge variant="destructive" className="gap-1">
                                <Pin className="h-3 w-3" />
                                {t("article.pinned")}
                              </Badge>
                            )}
                            <CardTitle className="flex-1">
                              <Link
                                href={`/posts/${article.slug}`}
                                className="hover:text-primary transition-colors"
                              >
                                {article.title}
                              </Link>
                            </CardTitle>
                          </div>
                          <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
                            <span className="flex items-center gap-1">
                              <Calendar className="h-4 w-4" />
                              {new Date(getPublishedDate(article)).toLocaleDateString(getDateLocale())}
                            </span>
                            <span className="flex items-center gap-1">
                              <Eye className="h-4 w-4" />
                              {getViewCount(article)}
                            </span>
                            <span className="flex items-center gap-1">
                              <Heart className="h-4 w-4" />
                              {getLikeCount(article)}
                            </span>
                            <span className="flex items-center gap-1">
                              <MessageSquare className="h-4 w-4" />
                              {getCommentCount(article)}
                            </span>
                          </div>
                        </CardHeader>
                        <CardContent>
                          <p className="text-muted-foreground line-clamp-2 mb-4">
                            {getExcerpt(article.content)}
                          </p>
                          <div className="flex flex-wrap items-center justify-between gap-2">
                            <div className="flex flex-wrap items-center gap-2">
                              {article.category && (
                                <Link href={`/categories?c=${article.category.slug}`}>
                                  <Badge variant="outline" className="hover:bg-secondary">
                                    <Folder className="h-3 w-3 mr-1" />
                                    {article.category.name}
                                  </Badge>
                                </Link>
                              )}
                              {article.tags && article.tags.slice(0, 3).map((tag) => (
                                <Link key={tag.id} href={`/tags?t=${tag.slug}`}>
                                  <Badge variant="secondary" className="hover:bg-secondary/80">
                                    <Tag className="h-3 w-3 mr-1" />
                                    {tag.name}
                                  </Badge>
                                </Link>
                              ))}
                              {article.tags && article.tags.length > 3 && (
                                <Badge variant="secondary">+{article.tags.length - 3}</Badge>
                              )}
                            </div>
                          </div>
                        </CardContent>
                      </div>
                      {thumbnail && (
                        <div className="hidden sm:block w-48 flex-shrink-0">
                          <Link href={`/posts/${article.slug}`} className="block h-full">
                            <div className="relative h-full min-h-[160px]">
                              <Image
                                src={thumbnail}
                                alt={article.title}
                                fill
                                className="object-cover"
                                sizes="192px"
                              />
                            </div>
                          </Link>
                        </div>
                      )}
                    </div>
                  </Card>
                );
              })
            )}
          </div>
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}

export default function HomePage() {
  return (
    <Suspense fallback={
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container py-8 max-w-4xl mx-auto">
            <Skeleton className="h-10 w-64 mx-auto mb-4" />
            <Skeleton className="h-6 w-96 mx-auto mb-8" />
            <div className="grid gap-6">
              {Array.from({ length: 3 }).map((_, i) => (
                <Card key={i}>
                  <CardHeader>
                    <Skeleton className="h-6 w-3/4" />
                  </CardHeader>
                  <CardContent>
                    <Skeleton className="h-4 w-full mb-2" />
                    <Skeleton className="h-4 w-2/3" />
                  </CardContent>
                </Card>
              ))}
            </div>
          </div>
        </main>
        <SiteFooter />
      </div>
    }>
      <HomeContent />
    </Suspense>
  );
}
