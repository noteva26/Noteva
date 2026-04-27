import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { motion } from "motion/react";
import { ArrowLeft, Tags as TagsIcon } from "lucide-react";
import { ArticleSummaryCard } from "@/components/article-summary-card";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  getTagUrl,
  waitForNoteva,
  type NotevaArticle,
  type NotevaTag,
} from "@/hooks/useNoteva";
import { fetchAllArticles } from "@/lib/articles";
import { useI18nStore, useTranslation } from "@/lib/i18n";

const TAG_SKELETON_KEYS = [
  "tag-a",
  "tag-b",
  "tag-c",
  "tag-d",
  "tag-e",
  "tag-f",
  "tag-g",
  "tag-h",
];

function getDateLocale(locale: string) {
  switch (locale) {
    case "zh-TW":
      return "zh-TW";
    case "en":
      return "en-US";
    default:
      return "zh-CN";
  }
}

export default function TagsPage() {
  const { t } = useTranslation();
  const locale = useI18nStore((state) => state.locale);
  const [searchParams] = useSearchParams();
  const selectedSlug = searchParams.get("t") || "";
  const isDetailView = selectedSlug.length > 0;
  const [tags, setTags] = useState<NotevaTag[]>([]);
  const [articles, setArticles] = useState<NotevaArticle[]>([]);
  const [selectedTag, setSelectedTag] = useState<NotevaTag | null>(null);
  const [loading, setLoading] = useState(true);
  const dateLocale = getDateLocale(locale);

  useEffect(() => {
    let active = true;

    const fetchData = async () => {
      setLoading(true);
      setArticles([]);
      setSelectedTag(null);

      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setTags([]);
        setLoading(false);
        return;
      }

      try {
        const tagList = await noteva.tags.list();
        if (!active) return;

        setTags(tagList);

        if (!selectedSlug) {
          setLoading(false);
          return;
        }

        const tag = tagList.find((item) => item.slug === selectedSlug) || null;
        setSelectedTag(tag);

        if (tag) {
          const articles = await fetchAllArticles(noteva, { tag: selectedSlug });
          if (active) setArticles(articles);
        }
      } catch {
        if (active) {
          setTags([]);
          setArticles([]);
          setSelectedTag(null);
        }
      } finally {
        if (active) setLoading(false);
      }
    };

    void fetchData();

    return () => {
      active = false;
    };
  }, [selectedSlug]);

  if (isDetailView) {
    return (
      <div className="theme-page-shell relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container mx-auto max-w-4xl py-10">
            <div className="mb-8">
              <Button variant="ghost" size="sm" className="mb-5" asChild>
                <Link to="/tags">
                  <ArrowLeft className="mr-2 h-4 w-4" />
                  {t("common.back")}
                </Link>
              </Button>

              <div className="flex items-center gap-3">
                <span className="flex size-11 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <TagsIcon className="h-5 w-5" />
                </span>
                <h1 className="min-w-0 truncate text-3xl font-semibold">
                  #{selectedTag?.name || selectedSlug}
                </h1>
              </div>

              <p className="mt-4 text-sm text-muted-foreground">
                {t("article.totalArticles")}: {articles.length}
              </p>
            </div>

            <div className="grid gap-6 article-list">
              {loading ? (
                TAG_SKELETON_KEYS.slice(0, 3).map((key) => (
                  <Card key={key}>
                    <CardContent className="p-6">
                      <Skeleton className="mb-4 h-6 w-3/4" />
                      <Skeleton className="h-4 w-full" />
                    </CardContent>
                  </Card>
                ))
              ) : !selectedTag || articles.length === 0 ? (
                <Card className="border-dashed">
                  <CardContent className="py-14 text-center text-muted-foreground">
                    {t("article.noArticles")}
                  </CardContent>
                </Card>
              ) : (
                articles.map((article, index) => (
                  <motion.div
                    key={article.id}
                    initial={{ opacity: 0, y: 14 }}
                    animate={{ opacity: 1, y: 0 }}
                    transition={{ delay: index * 0.035 }}
                  >
                    <ArticleSummaryCard
                      article={article}
                      dateLocale={dateLocale}
                      showTags={false}
                      priorityImage={index < 2}
                    />
                  </motion.div>
                ))
              )}
            </div>
          </div>
        </main>
        <SiteFooter />
      </div>
    );
  }

  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-4xl py-10">
          <div className="mb-8">
            <p className="mb-2 text-sm font-medium text-muted-foreground">
              {t("tag.totalTags")}: {tags.length}
            </p>
            <h1 className="text-3xl font-semibold">{t("nav.tags")}</h1>
          </div>

          {loading ? (
            <div className="flex flex-wrap gap-3">
              {TAG_SKELETON_KEYS.map((key) => (
                <Skeleton key={key} className="h-10 w-24 rounded-full" />
              ))}
            </div>
          ) : tags.length === 0 ? (
            <Card className="border-dashed">
              <CardContent className="py-14 text-center text-muted-foreground">
                <TagsIcon className="mx-auto mb-4 h-12 w-12 opacity-50" />
                {t("tag.noTags")}
              </CardContent>
            </Card>
          ) : (
            <div className="flex flex-wrap gap-3">
              {tags.map((tag, index) => (
                <motion.div
                  key={tag.id}
                  initial={{ opacity: 0, scale: 0.96 }}
                  animate={{ opacity: 1, scale: 1 }}
                  transition={{ delay: index * 0.025 }}
                >
                  <Link
                    to={getTagUrl(tag)}
                    className="inline-flex items-center gap-2 rounded-full border bg-card px-4 py-2 text-sm shadow-sm transition-colors hover:border-primary/60 hover:bg-muted/40 hover:text-primary"
                  >
                    <span className="text-muted-foreground">#</span>
                    <span>{tag.name}</span>
                    {typeof tag.articleCount === "number" ? (
                      <span className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                        {tag.articleCount}
                      </span>
                    ) : null}
                  </Link>
                </motion.div>
              ))}
            </div>
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
