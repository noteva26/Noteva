import { useEffect, useState } from "react";
import { useSearchParams, Link } from "react-router-dom";
import { motion } from "motion/react";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Calendar, Folder, Eye, Heart, MessageSquare, Tag, Pin, FileText, ChevronLeft, ChevronRight } from "lucide-react";
import { useTranslation, useI18nStore } from "@/lib/i18n";
import { getNoteva, getArticleUrl } from "@/hooks/useNoteva";

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

function extractFirstImage(content: string): string | null {
  const imgRegex = /!\[.*?\]\((.*?)\)/;
  const match = content.match(imgRegex);
  return match ? match[1] : null;
}

function getExcerpt(content: string, maxLength: number = 200): string {
  let text = content.replace(/\[([a-zA-Z0-9_-]+)(?:\s+[^\]]*)?]([\s\S]*?)\[\/\1]/g, '');
  text = text.replace(/\[[a-zA-Z0-9_-]+(?:\s+[^\]]*)?\/]/g, '');
  text = text.replace(/\[\/?\w+[^\]]*]/g, '');
  text = text.replace(/!\[[^\]]*\]\([^)]+\)/g, '');
  text = text.replace(/!\([^)]+\)/g, '');
  text = text.replace(/<img[^>]*>/gi, '');
  text = text.replace(/\[([^\]]+)\]\([^)]+\)/g, '$1');
  text = text.replace(/[*_~`#]+/g, '');
  text = text.replace(/<[^>]+>/g, '');
  text = text.replace(/\s+/g, ' ').trim();
  if (text.length <= maxLength) return text;
  return text.slice(0, maxLength) + '...';
}

const PAGE_SIZE = 10;

export default function HomePage() {
  const [articles, setArticles] = useState<Article[]>([]);
  const [loading, setLoading] = useState(true);
  const [siteInfo, setSiteInfo] = useState<{ name: string; subtitle: string; description: string } | null>(null);
  const [totalPages, setTotalPages] = useState(1);
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);
  const [searchParams, setSearchParams] = useSearchParams();
  const searchQuery = searchParams.get("q") || "";
  const currentPage = Math.max(1, parseInt(searchParams.get("page") || "1", 10));

  useEffect(() => {
    const config = (window as any).__SITE_CONFIG__;
    if (config) {
      setSiteInfo({
        name: config.site_name || "Noteva",
        subtitle: config.site_subtitle || "",
        description: config.site_description || "",
      });
      document.title = config.site_name || "Noteva";
    }
  }, []);

  useEffect(() => {
    setLoading(true);
    const loadData = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadData, 50); return; }

      try {
        if (!siteInfo) {
          const info = await Noteva.site.getInfo();
          setSiteInfo({
            name: info.name || "Noteva",
            subtitle: info.subtitle || "",
            description: info.description || "",
          });
          document.title = info.name || "Noteva";
        }

        const result = await Noteva.articles.list({ page: currentPage, pageSize: PAGE_SIZE });
        setArticles(result.articles || []);
        const total = result.total || 0;
        setTotalPages(Math.max(1, Math.ceil(total / PAGE_SIZE)));
      } catch (err) {
        console.error("Failed to load data:", err);
        setArticles([]);
      } finally {
        setLoading(false);
      }
    };
    loadData();
  }, [currentPage]);

  const goToPage = (page: number) => {
    const params = new URLSearchParams(searchParams);
    if (page <= 1) {
      params.delete("page");
    } else {
      params.set("page", String(page));
    }
    setSearchParams(params);
    window.scrollTo({ top: 0, behavior: "smooth" });
  };

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

  const getThumbnail = (article: Article): string | null => {
    if (article.thumbnail) return article.thumbnail;
    return extractFirstImage(article.content);
  };

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
          <motion.div
            initial={{ opacity: 0, y: -10 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.5, ease: "easeOut" }}
            className="mb-8 text-center"
          >
            {siteInfo ? (
              <>
                <h1 className="text-4xl font-bold mb-2">{t("home.welcome")} {siteInfo.name}</h1>
                <p className="text-muted-foreground text-lg">
                  {siteInfo.subtitle || siteInfo.description || t("home.subtitle")}
                </p>
              </>
            ) : (
              <>
                <div className="h-10 w-64 mx-auto mb-4 skeleton-shimmer rounded" />
                <div className="h-6 w-96 mx-auto skeleton-shimmer rounded" />
              </>
            )}
          </motion.div>

          {searchQuery && (
            <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }} className="mb-6 text-center">
              <p className="text-muted-foreground">
                {t("common.search")}: <span className="font-medium text-foreground">{searchQuery}</span>
              </p>
            </motion.div>
          )}

          <div className="grid gap-6">
            {loading ? (
              Array.from({ length: 3 }).map((_, i) => (
                <Card key={i}>
                  <CardHeader><div className="h-6 w-3/4 skeleton-shimmer rounded" /></CardHeader>
                  <CardContent><div className="h-4 w-full mb-2 skeleton-shimmer rounded" /><div className="h-4 w-2/3 skeleton-shimmer rounded" /></CardContent>
                </Card>
              ))
            ) : filteredArticles.length === 0 ? (
              <motion.div initial={{ opacity: 0, y: 10 }} animate={{ opacity: 1, y: 0 }} transition={{ type: "spring", stiffness: 400, damping: 30 }}>
                <Card>
                  <CardContent className="py-12 flex flex-col items-center justify-center text-center">
                    <div className="rounded-full bg-muted flex items-center justify-center mb-4 size-12">
                      <FileText className="size-5 text-muted-foreground" />
                    </div>
                    <p className="text-muted-foreground">{searchQuery ? t("common.noData") : t("home.noPostsYet")}</p>
                  </CardContent>
                </Card>
              </motion.div>
            ) : (
              filteredArticles.map((article, index) => {
                const thumbnail = getThumbnail(article);
                return (
                  <motion.div
                    key={article.id}
                    initial={{ opacity: 0, y: 20 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ type: "spring", stiffness: 400, damping: 30, delay: index * 0.05 }}
                    whileHover={{ y: -2 }}
                  >
                    <Card className="hover:shadow-md transition-shadow overflow-hidden">
                      <div className="flex">
                        <div className="flex-1">
                          <CardHeader>
                            <div className="flex items-center gap-2">
                              {isPinned(article) && (
                                <Badge variant="destructive" className="gap-1"><Pin className="h-3 w-3" />{t("article.pinned")}</Badge>
                              )}
                              <CardTitle className="flex-1">
                                <Link to={getArticleUrl(article)} className="hover:text-primary transition-colors">{article.title}</Link>
                              </CardTitle>
                            </div>
                            <div className="flex flex-wrap items-center gap-4 text-sm text-muted-foreground">
                              <span className="flex items-center gap-1"><Calendar className="h-4 w-4" />{new Date(getPublishedDate(article)).toLocaleDateString(getDateLocale())}</span>
                              <span className="flex items-center gap-1"><Eye className="h-4 w-4" />{getViewCount(article)}</span>
                              <span className="flex items-center gap-1"><Heart className="h-4 w-4" />{getLikeCount(article)}</span>
                              <span className="flex items-center gap-1"><MessageSquare className="h-4 w-4" />{getCommentCount(article)}</span>
                            </div>
                          </CardHeader>
                          <CardContent>
                            <p className="text-muted-foreground line-clamp-2 mb-4">{getExcerpt(article.content)}</p>
                            <div className="flex flex-wrap items-center gap-2">
                              {article.category && (
                                <Link to={`/categories?c=${article.category.slug}`}>
                                  <Badge variant="outline" className="hover:bg-secondary"><Folder className="h-3 w-3 mr-1" />{article.category.name}</Badge>
                                </Link>
                              )}
                              {article.tags && article.tags.slice(0, 3).map((tag) => (
                                <Link key={tag.id} to={`/tags?t=${tag.slug}`}>
                                  <Badge variant="secondary" className="hover:bg-secondary/80"><Tag className="h-3 w-3 mr-1" />{tag.name}</Badge>
                                </Link>
                              ))}
                              {article.tags && article.tags.length > 3 && <Badge variant="secondary">+{article.tags.length - 3}</Badge>}
                            </div>
                          </CardContent>
                        </div>
                        {thumbnail && (
                          <div className="hidden sm:block w-48 flex-shrink-0">
                            <Link to={getArticleUrl(article)} className="block h-full">
                              <div className="relative h-full min-h-[160px]">
                                <img src={thumbnail} alt={article.title} className="absolute inset-0 w-full h-full object-cover" />
                              </div>
                            </Link>
                          </div>
                        )}
                      </div>
                    </Card>
                  </motion.div>
                );
              })
            )}
          </div>

          {/* Pagination */}
          {!loading && !searchQuery && totalPages > 1 && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ delay: 0.2 }}
              className="flex items-center justify-center gap-2 mt-8"
            >
              <Button
                variant="outline"
                size="sm"
                onClick={() => goToPage(currentPage - 1)}
                disabled={currentPage <= 1}
              >
                <ChevronLeft className="h-4 w-4 mr-1" />
                {t("pagination.prev")}
              </Button>
              <span className="text-sm text-muted-foreground px-3">
                {t("pagination.page").replace("{current}", String(currentPage)).replace("{total}", String(totalPages))}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => goToPage(currentPage + 1)}
                disabled={currentPage >= totalPages}
              >
                {t("pagination.next")}
                <ChevronRight className="h-4 w-4 ml-1" />
              </Button>
            </motion.div>
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
