"use client";

import { useEffect, useState, Suspense } from "react";
import { useSearchParams } from "next/navigation";
import Link from "next/link";
import Image from "next/image";
import { useTranslation, useI18nStore } from "@/lib/i18n";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { Tags as TagsIcon, Calendar, Eye, Heart, MessageSquare, Folder, Pin, ArrowLeft } from "lucide-react";
import { getNoteva, getArticleUrl } from "@/hooks/useNoteva";

interface Tag {
  id: number;
  name: string;
  slug: string;
}

interface Article {
  id: number;
  slug: string;
  title: string;
  content: string;
  thumbnail?: string;
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
  category?: { id: number; name: string; slug: string };
  tags?: { id: number; name: string; slug: string }[];
}

function extractFirstImage(content: string): string | null {
  const imgRegex = /!\[.*?\]\((.*?)\)/;
  const match = content.match(imgRegex);
  return match ? match[1] : null;
}

function TagsContent() {
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);
  const searchParams = useSearchParams();
  const selectedSlug = searchParams.get("t") || "";
  
  const [tags, setTags] = useState<Tag[]>([]);
  const [articles, setArticles] = useState<Article[]>([]);
  const [selectedTag, setSelectedTag] = useState<Tag | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        setTimeout(fetchData, 50);
        return;
      }

      try {
        // 获取标签列表
        const tagList = await Noteva.tags.list();
        setTags(tagList);
        
        if (selectedSlug) {
          const tag = tagList.find((t: Tag) => t.slug === selectedSlug);
          setSelectedTag(tag || null);
          
          if (tag) {
            // 获取该标签下的文章
            const result = await Noteva.articles.list({ pageSize: 100, tag: selectedSlug });
            setArticles(result.articles || []);
          }
        }
      } catch (err) {
        console.error(err);
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, [selectedSlug]);

  const getDateLocale = () => {
    switch (locale) {
      case "zh-TW": return "zh-TW";
      case "en": return "en-US";
      default: return "zh-CN";
    }
  };

  const getThumbnail = (article: Article): string | null => {
    if (article.thumbnail) return article.thumbnail;
    return extractFirstImage(article.content);
  };

  // 兼容字段名
  const getPublishedDate = (a: Article) => 
    a.published_at || a.publishedAt || a.created_at || a.createdAt || "";
  const getViewCount = (a: Article) => a.view_count ?? a.viewCount ?? 0;
  const getLikeCount = (a: Article) => a.like_count ?? a.likeCount ?? 0;
  const getCommentCount = (a: Article) => a.comment_count ?? a.commentCount ?? 0;
  const isPinned = (a: Article) => a.is_pinned || a.isPinned;

  // 显示标签下的文章列表
  if (selectedSlug && selectedTag) {
    return (
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container py-8 max-w-4xl mx-auto">
            <div className="mb-8">
              <Link href="/tags">
                <Button variant="ghost" size="sm" className="mb-4">
                  <ArrowLeft className="h-4 w-4 mr-2" />{t("common.back")}
                </Button>
              </Link>
              <div className="flex items-center gap-3 mb-2">
                <TagsIcon className="h-8 w-8 text-primary" />
                <h1 className="text-3xl font-bold">#{selectedTag.name}</h1>
              </div>
              <p className="text-muted-foreground">{t("article.totalArticles")}: {articles.length}</p>
            </div>

            <div className="grid gap-6">
              {loading ? (
                Array.from({ length: 3 }).map((_, i) => (
                  <Card key={i}><CardHeader><Skeleton className="h-6 w-3/4" /></CardHeader><CardContent><Skeleton className="h-4 w-full mb-2" /></CardContent></Card>
                ))
              ) : articles.length === 0 ? (
                <Card><CardContent className="py-12 text-center text-muted-foreground">{t("article.noArticles")}</CardContent></Card>
              ) : (
                articles.map((article) => {
                  const thumbnail = getThumbnail(article);
                  return (
                    <Card key={article.id} className="hover:shadow-md transition-shadow overflow-hidden">
                      <div className="flex">
                        <div className="flex-1">
                          <CardHeader>
                            <div className="flex items-center gap-2">
                              {isPinned(article) && (<Badge variant="destructive" className="gap-1"><Pin className="h-3 w-3" />{t("article.pinned")}</Badge>)}
                              <CardTitle className="flex-1"><Link href={getArticleUrl(article)} className="hover:text-primary transition-colors">{article.title}</Link></CardTitle>
                            </div>
                            <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
                              <span className="flex items-center gap-1"><Calendar className="h-4 w-4" />{new Date(getPublishedDate(article)).toLocaleDateString(getDateLocale())}</span>
                              <span className="flex items-center gap-1"><Eye className="h-4 w-4" />{getViewCount(article)}</span>
                              <span className="flex items-center gap-1"><Heart className="h-4 w-4" />{getLikeCount(article)}</span>
                              <span className="flex items-center gap-1"><MessageSquare className="h-4 w-4" />{getCommentCount(article)}</span>
                            </div>
                          </CardHeader>
                          <CardContent>
                            <p className="text-muted-foreground line-clamp-2 mb-4">{article.content.slice(0, 200)}...</p>
                            {article.category && (
                              <Link href={`/categories?c=${article.category.slug}`}><Badge variant="outline" className="hover:bg-secondary"><Folder className="h-3 w-3 mr-1" />{article.category.name}</Badge></Link>
                            )}
                          </CardContent>
                        </div>
                        {thumbnail && (
                          <div className="hidden sm:block w-48 flex-shrink-0">
                            <Link href={getArticleUrl(article)} className="block h-full">
                              <div className="relative h-full min-h-[160px]"><Image src={thumbnail} alt={article.title} fill className="object-cover" sizes="192px" /></div>
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

  // 显示标签列表
  return (
    <div className="relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container py-8 max-w-4xl mx-auto">
          <div className="mb-8">
            <h1 className="text-3xl font-bold mb-2">{t("nav.tags")}</h1>
            <p className="text-muted-foreground">{t("tag.totalTags")}: {tags.length}</p>
          </div>

          {loading ? (
            <div className="flex flex-wrap gap-3">
              {[1, 2, 3, 4, 5, 6, 7, 8].map((i) => (<Skeleton key={i} className="h-10 w-24" />))}
            </div>
          ) : tags.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center text-muted-foreground">
                <TagsIcon className="h-12 w-12 mx-auto mb-4 opacity-50" />
                {t("tag.noTags")}
              </CardContent>
            </Card>
          ) : (
            <div className="flex flex-wrap gap-3">
              {tags.map((tag) => (
                <Link key={tag.id} href={`/tags?t=${tag.slug}`} className="inline-flex items-center gap-2 px-4 py-2 bg-card border rounded-full hover:border-primary hover:text-primary transition-colors">
                  <span>#</span>
                  <span>{tag.name}</span>
                </Link>
              ))}
            </div>
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}

export default function TagsPage() {
  return (
    <Suspense fallback={<div className="relative flex min-h-screen flex-col"><SiteHeader /><main className="flex-1"><div className="container py-8 max-w-4xl mx-auto"><Skeleton className="h-10 w-48 mb-8" /></div></main><SiteFooter /></div>}>
      <TagsContent />
    </Suspense>
  );
}
