import { useEffect, useMemo, useState } from "react";
import { Link } from "react-router-dom";
import { motion } from "motion/react";
import { Archive, CalendarDays } from "lucide-react";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import {
  getArticleUrl,
  waitForNoteva,
  type NotevaArticle,
} from "@/hooks/useNoteva";
import { fetchAllArticles } from "@/lib/articles";
import { useI18nStore, useTranslation } from "@/lib/i18n";

interface ArchiveGroup {
  year: number;
  months: { month: number; articles: NotevaArticle[] }[];
}

const ARCHIVE_SKELETON_KEYS = ["archive-a", "archive-b", "archive-c"];

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

function getArticleDate(article: NotevaArticle) {
  const value = article.publishedAt || article.createdAt;
  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? null : date;
}

function groupByYearMonth(articles: NotevaArticle[]): ArchiveGroup[] {
  const map = new Map<number, Map<number, NotevaArticle[]>>();

  articles.forEach((article) => {
    const date = getArticleDate(article);
    if (!date) return;

    const year = date.getFullYear();
    const month = date.getMonth() + 1;

    if (!map.has(year)) map.set(year, new Map());
    const yearGroup = map.get(year);
    if (!yearGroup?.has(month)) yearGroup?.set(month, []);
    yearGroup?.get(month)?.push(article);
  });

  return Array.from(map.keys())
    .sort((a, b) => b - a)
    .map((year) => {
      const monthMap = map.get(year);
      return {
        year,
        months: Array.from(monthMap?.keys() || [])
          .sort((a, b) => b - a)
          .map((month) => ({
            month,
            articles: monthMap?.get(month) || [],
          })),
      };
    });
}

export default function ArchivesPage() {
  const { t } = useTranslation();
  const locale = useI18nStore((state) => state.locale);
  const dateLocale = getDateLocale(locale);
  const [archives, setArchives] = useState<ArchiveGroup[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    let active = true;

    const fetchData = async () => {
      setLoading(true);

      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setArchives([]);
        setLoading(false);
        return;
      }

      try {
        const articles = await fetchAllArticles(noteva);
        if (!active) return;

        setArchives(groupByYearMonth(articles));
      } catch {
        if (active) setArchives([]);
      } finally {
        if (active) setLoading(false);
      }
    };

    void fetchData();

    return () => {
      active = false;
    };
  }, []);

  const totalArticles = useMemo(
    () =>
      archives.reduce(
        (sum, yearGroup) =>
          sum +
          yearGroup.months.reduce(
            (monthSum, monthGroup) => monthSum + monthGroup.articles.length,
            0
          ),
        0
      ),
    [archives]
  );

  const monthFormatter = useMemo(
    () => new Intl.DateTimeFormat(dateLocale, { month: "long" }),
    [dateLocale]
  );

  const dayFormatter = useMemo(
    () => new Intl.DateTimeFormat(dateLocale, { day: "2-digit" }),
    [dateLocale]
  );

  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-4xl py-10">
          <div className="mb-8">
            <p className="mb-2 flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Archive className="h-4 w-4" />
              {t("article.totalArticles")}: {totalArticles}
            </p>
            <h1 className="text-3xl font-semibold">{t("nav.archive")}</h1>
          </div>

          {loading ? (
            <div className="space-y-6">
              {ARCHIVE_SKELETON_KEYS.map((key) => (
                <Skeleton key={key} className="h-32 w-full" />
              ))}
            </div>
          ) : archives.length === 0 ? (
            <Card className="border-dashed">
              <CardContent className="py-14 text-center text-muted-foreground">
                {t("article.noArticles")}
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-10">
              {archives.map((yearGroup) => (
                <section key={yearGroup.year}>
                  <h2 className="sticky top-20 z-10 mb-5 flex items-center gap-2 border-b bg-background/90 py-3 text-2xl font-semibold backdrop-blur">
                    <CalendarDays className="h-5 w-5 text-muted-foreground" />
                    {yearGroup.year}
                  </h2>

                  <div className="space-y-7">
                    {yearGroup.months.map((monthGroup) => (
                      <div key={monthGroup.month} className="grid gap-4 sm:grid-cols-[8rem_1fr]">
                        <div>
                          <h3 className="font-medium text-muted-foreground">
                            {monthFormatter.format(
                              new Date(yearGroup.year, monthGroup.month - 1, 1)
                            )}
                          </h3>
                          <p className="mt-1 text-xs text-muted-foreground">
                            {monthGroup.articles.length} {t("article.totalArticles")}
                          </p>
                        </div>

                        <ul className="relative space-y-3 border-l border-border pl-5">
                          {monthGroup.articles.map((article, index) => {
                            const date = getArticleDate(article);
                            return (
                              <motion.li
                                key={article.id}
                                data-article-id={article.id}
                                className="relative"
                                initial={{ opacity: 0, x: -8 }}
                                animate={{ opacity: 1, x: 0 }}
                                transition={{ delay: index * 0.025 }}
                              >
                                <span className="absolute -left-[1.82rem] top-3 h-3 w-3 rounded-full border-2 border-background bg-primary" />
                                <Link
                                  to={getArticleUrl(article)}
                                  className="group block rounded-lg border bg-card px-4 py-3 transition-colors hover:border-primary/60 hover:bg-muted/30"
                                >
                                  <span className="mb-1 block text-xs font-medium uppercase tracking-wide text-muted-foreground">
                                    {date ? dayFormatter.format(date) : "--"}
                                  </span>
                                  <span className="font-medium underline-offset-4 group-hover:text-primary group-hover:underline">
                                    {article.title}
                                  </span>
                                </Link>
                              </motion.li>
                            );
                          })}
                        </ul>
                      </div>
                    ))}
                  </div>
                </section>
              ))}
            </div>
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
