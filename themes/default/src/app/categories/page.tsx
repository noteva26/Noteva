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
import { FolderTree, Calendar, Eye, Heart, MessageSquare, Tag, Pin, ArrowLeft } from "lucide-react";
import { Button } from "@/components/ui/button";
import { getNoteva, getArticleUrl } from "@/hooks/useNoteva";

interface Category {
  id: number;
  name: string;
  slug: string;
  description?: string;
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
  category_id?: number;
  category?: { id: number; name: string; slug: string };
  tags?: { id: number; name: string; slug: string }[];
}

function extractFirstImage(content: string): string | null {
  const imgRegex = /!\[.*?\]\((.*?)\)/;
  const match = content.match(imgRegex);
  return match ? match[1] : null;
}

function CategoriesContent() {
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);
  const searchParams = useSearchParams();
  const selectedSlug = searchParams.get("c") || "";
  
  const [categories, setCategories] = useState<Category[]>([]);
  const [articles, setArticles] = useState<Article[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<Category | null>(null);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        setTimeout(fetchData, 50);
        return;
      }

      try {
        // 获取分类列表
        const cats = await Noteva.categories.list();
        setCategories(cats);
        
        if (selectedSlug) {
          const cat = cats.find((c: Category) => c.slug === selectedSlug);
          setSelectedCategory(cat || null);
          
          if (cat) {
            // 获取该分类下的文章
            const result = await Noteva.articles.list({ pageSize: 100, category: selectedSlug });
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

  // 显示分类下的文章列表
  if (selectedSlug && selectedCategory) {
    return (
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container py-8 max-w-4xl mx-auto">
            <div className="mb-8">
              <Link href="/categories">
                <Button variant="ghost" size="sm" className="mb-4">
                  <ArrowLeft className="h-4 w-4 mr-2" />{t("common.back")}
                </Button>
              </Link>
              <div className="flex items-center gap-3 mb-2">
                <FolderTree className="h-8 w-8 text-primary" />
                <h1 className="text-3xl font-bold">{selectedCategory.name}</h1>
              </div>
              {selectedCategory.description && (
                <p className="text-muted-foreground">{selectedCategory.description}</p>
              )}
              <p className="text-muted-foreground mt-2">{t("article.totalArticles")}: {articles.length}</p>
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
                            {article.tags && article.tags.length > 0 && (
                              <div className="flex flex-wrap items-center gap-2">
                                {article.tags.slice(0, 3).map((tag) => (
                                  <Link key={tag.id} href={`/tags?t=${tag.slug}`}><Badge variant="secondary" className="hover:bg-secondary/80"><Tag className="h-3 w-3 mr-1" />{tag.name}</Badge></Link>
                                ))}
                              </div>
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

  // 显示分类列表
  return (
    <div className="relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container py-8 max-w-4xl mx-auto">
          <div className="mb-8">
            <h1 className="text-3xl font-bold mb-2">{t("nav.categories")}</h1>
            <p className="text-muted-foreground">{t("category.totalCategories")}: {categories.length}</p>
          </div>

          {loading ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {[1, 2, 3, 4, 5, 6].map((i) => (<Skeleton key={i} className="h-24" />))}
            </div>
          ) : categories.length === 0 ? (
            <Card><CardContent className="py-12 text-center text-muted-foreground">{t("category.noCategories")}</CardContent></Card>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {categories.map((category) => (
                <Link key={category.id} href={`/categories?c=${category.slug}`}>
                  <Card className="h-full hover:border-primary transition-colors cursor-pointer">
                    <CardContent className="p-6">
                      <div className="flex items-start gap-4">
                        <div className="p-2 bg-primary/10 rounded-lg"><FolderTree className="h-5 w-5 text-primary" /></div>
                        <div className="flex-1 min-w-0">
                          <h2 className="font-semibold truncate">{category.name}</h2>
                          {category.description && (<p className="text-sm text-muted-foreground mt-1 line-clamp-2">{category.description}</p>)}
                        </div>
                      </div>
                    </CardContent>
                  </Card>
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

export default function CategoriesPage() {
  return (
    <Suspense fallback={<div className="relative flex min-h-screen flex-col"><SiteHeader /><main className="flex-1"><div className="container py-8 max-w-4xl mx-auto"><Skeleton className="h-10 w-48 mb-8" /></div></main><SiteFooter /></div>}>
      <CategoriesContent />
    </Suspense>
  );
}
