import { useEffect, useState } from "react";
import { Link, useSearchParams } from "react-router-dom";
import { motion } from "motion/react";
import { ArrowLeft, FolderTree } from "lucide-react";
import { ArticleSummaryCard } from "@/components/article-summary-card";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  waitForNoteva,
  type NotevaArticle,
  type NotevaCategory,
} from "@/hooks/useNoteva";
import { useI18nStore, useTranslation } from "@/lib/i18n";

const CATEGORY_SKELETON_KEYS = [
  "category-a",
  "category-b",
  "category-c",
  "category-d",
  "category-e",
  "category-f",
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

export default function CategoriesPage() {
  const { t } = useTranslation();
  const locale = useI18nStore((state) => state.locale);
  const [searchParams] = useSearchParams();
  const selectedSlug = searchParams.get("c") || "";
  const isDetailView = selectedSlug.length > 0;
  const [categories, setCategories] = useState<NotevaCategory[]>([]);
  const [articles, setArticles] = useState<NotevaArticle[]>([]);
  const [selectedCategory, setSelectedCategory] = useState<NotevaCategory | null>(
    null
  );
  const [loading, setLoading] = useState(true);
  const dateLocale = getDateLocale(locale);

  useEffect(() => {
    let active = true;

    const fetchData = async () => {
      setLoading(true);
      setArticles([]);
      setSelectedCategory(null);

      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setCategories([]);
        setLoading(false);
        return;
      }

      try {
        const categoryList = await noteva.categories.list();
        if (!active) return;

        setCategories(categoryList);

        if (!selectedSlug) {
          setLoading(false);
          return;
        }

        const category = categoryList.find((item) => item.slug === selectedSlug) || null;
        setSelectedCategory(category);

        if (category) {
          const result = await noteva.articles.list({
            pageSize: 100,
            category: selectedSlug,
          });
          if (active) setArticles(result.articles || []);
        }
      } catch {
        if (active) {
          setCategories([]);
          setArticles([]);
          setSelectedCategory(null);
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
                <Link to="/categories">
                  <ArrowLeft className="mr-2 h-4 w-4" />
                  {t("common.back")}
                </Link>
              </Button>

              <div className="flex items-center gap-3">
                <span className="flex size-11 items-center justify-center rounded-lg bg-primary/10 text-primary">
                  <FolderTree className="h-5 w-5" />
                </span>
                <div className="min-w-0">
                  <h1 className="truncate text-3xl font-semibold">
                    {selectedCategory?.name || t("nav.categories")}
                  </h1>
                  {selectedCategory?.description ? (
                    <p className="mt-1 text-muted-foreground">
                      {selectedCategory.description}
                    </p>
                  ) : null}
                </div>
              </div>

              <p className="mt-4 text-sm text-muted-foreground">
                {t("article.totalArticles")}: {articles.length}
              </p>
            </div>

            <div className="grid gap-6 article-list">
              {loading ? (
                CATEGORY_SKELETON_KEYS.slice(0, 3).map((key) => (
                  <Card key={key}>
                    <CardContent className="p-6">
                      <Skeleton className="mb-4 h-6 w-3/4" />
                      <Skeleton className="h-4 w-full" />
                    </CardContent>
                  </Card>
                ))
              ) : !selectedCategory || articles.length === 0 ? (
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
                      showCategory={false}
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
              {t("category.totalCategories")}: {categories.length}
            </p>
            <h1 className="text-3xl font-semibold">{t("nav.categories")}</h1>
          </div>

          {loading ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {CATEGORY_SKELETON_KEYS.map((key) => (
                <Skeleton key={key} className="h-28" />
              ))}
            </div>
          ) : categories.length === 0 ? (
            <Card className="border-dashed">
              <CardContent className="py-14 text-center text-muted-foreground">
                {t("category.noCategories")}
              </CardContent>
            </Card>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {categories.map((category, index) => (
                <motion.div
                  key={category.id}
                  initial={{ opacity: 0, y: 12 }}
                  animate={{ opacity: 1, y: 0 }}
                  transition={{ delay: index * 0.03 }}
                >
                  <Link to={`/categories?c=${category.slug}`} className="group block h-full">
                    <Card className="h-full transition-colors hover:border-primary/60 hover:bg-muted/30">
                      <CardContent className="p-5">
                        <div className="flex items-start gap-4">
                          <span className="flex size-10 shrink-0 items-center justify-center rounded-lg bg-primary/10 text-primary transition-colors group-hover:bg-primary group-hover:text-primary-foreground">
                            <FolderTree className="h-5 w-5" />
                          </span>
                          <div className="min-w-0 flex-1">
                            <h2 className="truncate font-semibold">
                              {category.name}
                            </h2>
                            {category.description ? (
                              <p className="mt-1 line-clamp-2 text-sm leading-6 text-muted-foreground">
                                {category.description}
                              </p>
                            ) : null}
                            {typeof category.articleCount === "number" ? (
                              <p className="mt-3 text-xs text-muted-foreground">
                                {category.articleCount} {t("article.totalArticles")}
                              </p>
                            ) : null}
                          </div>
                        </div>
                      </CardContent>
                    </Card>
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
