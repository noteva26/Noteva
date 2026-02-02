"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useTranslation } from "@/lib/i18n";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { getNoteva, getArticleUrl } from "@/hooks/useNoteva";

interface Article {
  id: number;
  slug: string;
  title: string;
  published_at?: string;
  publishedAt?: string;
  created_at?: string;
  createdAt?: string;
}

interface ArchiveGroup {
  year: number;
  months: {
    month: number;
    articles: Article[];
  }[];
}

export default function ArchivesPage() {
  const { t } = useTranslation();
  const [archives, setArchives] = useState<ArchiveGroup[]>([]);
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    const fetchData = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        setTimeout(fetchData, 50);
        return;
      }

      try {
        // 获取所有文章
        const result = await Noteva.articles.list({ pageSize: 100 });
        const articles = result.articles || [];
        const grouped = groupByYearMonth(articles);
        setArchives(grouped);
      } catch (err) {
        console.error(err);
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, []);

  // 兼容字段名
  const getPublishedDate = (a: Article) => 
    a.published_at || a.publishedAt || a.created_at || a.createdAt || "";

  const groupByYearMonth = (articles: Article[]): ArchiveGroup[] => {
    const map = new Map<number, Map<number, Article[]>>();
    
    articles.forEach((article) => {
      const date = new Date(getPublishedDate(article));
      const year = date.getFullYear();
      const month = date.getMonth() + 1;
      
      if (!map.has(year)) map.set(year, new Map());
      if (!map.get(year)!.has(month)) map.get(year)!.set(month, []);
      map.get(year)!.get(month)!.push(article);
    });
    
    const result: ArchiveGroup[] = [];
    const sortedYears = Array.from(map.keys()).sort((a, b) => b - a);
    
    for (const year of sortedYears) {
      const monthMap = map.get(year)!;
      const sortedMonths = Array.from(monthMap.keys()).sort((a, b) => b - a);
      const months = sortedMonths.map((month) => ({
        month,
        articles: monthMap.get(month)!,
      }));
      result.push({ year, months });
    }
    
    return result;
  };

  const totalArticles = archives.reduce(
    (sum, year) => sum + year.months.reduce((s, m) => s + m.articles.length, 0),
    0
  );

  return (
    <div className="relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container py-8 max-w-4xl mx-auto">
          <div className="mb-8">
            <h1 className="text-3xl font-bold mb-2">{t("nav.archive")}</h1>
            <p className="text-muted-foreground">
              {t("article.totalArticles")}: {totalArticles}
            </p>
          </div>

          {loading ? (
            <div className="space-y-6">
              {[1, 2, 3].map((i) => (
                <Skeleton key={i} className="h-32 w-full" />
              ))}
            </div>
          ) : archives.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center text-muted-foreground">
                {t("article.noArticles")}
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-8">
              {archives.map((yearGroup) => (
                <div key={yearGroup.year}>
                  <h2 className="text-2xl font-bold mb-4 sticky top-20 bg-background py-2">
                    {yearGroup.year}
                  </h2>
                  {yearGroup.months.map((monthGroup) => (
                    <div key={monthGroup.month} className="mb-6">
                      <h3 className="text-lg font-medium text-muted-foreground mb-3">
                        {monthGroup.month}月
                      </h3>
                      <ul className="space-y-3 pl-4 border-l-2 border-muted">
                        {monthGroup.articles.map((article) => (
                          <li key={article.id} className="relative">
                            <span className="absolute -left-[9px] top-2 w-4 h-4 bg-background border-2 border-muted rounded-full" />
                            <Link
                              href={getArticleUrl(article)}
                              className="block pl-6 py-1 hover:text-primary transition-colors"
                            >
                              <span className="text-sm text-muted-foreground mr-2">
                                {new Date(getPublishedDate(article)).getDate()}日
                              </span>
                              <span className="font-medium">{article.title}</span>
                            </Link>
                          </li>
                        ))}
                      </ul>
                    </div>
                  ))}
                </div>
              ))}
            </div>
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
