"use client";

import { useEffect, useState } from "react";
import { adminApi, articlesApi, DashboardStats, Article, SystemStats } from "@/lib/api";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { FileText, FolderTree, Tags, Eye, TrendingUp, Activity, HardDrive, Clock, Zap } from "lucide-react";
import {
  ResponsiveContainer,
  PieChart,
  Pie,
  Cell,
  Tooltip,
} from "recharts";
import { useTranslation, useI18nStore } from "@/lib/i18n";

export default function DashboardPage() {
  const [stats, setStats] = useState<DashboardStats | null>(null);
  const [systemStats, setSystemStats] = useState<SystemStats | null>(null);
  const [recentArticles, setRecentArticles] = useState<Article[]>([]);
  const [loading, setLoading] = useState(true);
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);

  useEffect(() => {
    Promise.all([
      adminApi.dashboard(),
      adminApi.systemStats(),
      articlesApi.list({ per_page: 5 }),
    ])
      .then(([statsRes, sysStatsRes, articlesRes]) => {
        setStats(statsRes.data);
        setSystemStats(sysStatsRes.data);
        setRecentArticles(articlesRes.data?.articles || []);
      })
      .catch(console.error)
      .finally(() => setLoading(false));
  }, []);

  // Refresh system stats every 5 seconds
  useEffect(() => {
    const interval = setInterval(() => {
      adminApi.systemStats()
        .then((res) => setSystemStats(res.data))
        .catch(() => {});
    }, 5000);
    return () => clearInterval(interval);
  }, []);

  const getDateLocale = () => {
    switch (locale) {
      case "zh-TW": return "zh-TW";
      case "en": return "en-US";
      default: return "zh-CN";
    }
  };

  const statCards = [
    {
      title: t("article.totalArticles"),
      value: stats?.total_articles ?? 0,
      icon: FileText,
      color: "text-blue-500",
    },
    {
      title: t("article.publishedArticles"),
      value: stats?.published_articles ?? 0,
      icon: Eye,
      color: "text-green-500",
    },
    {
      title: t("category.totalCategories"),
      value: stats?.total_categories ?? 0,
      icon: FolderTree,
      color: "text-purple-500",
    },
    {
      title: t("tag.totalTags"),
      value: stats?.total_tags ?? 0,
      icon: Tags,
      color: "text-orange-500",
    },
  ];

  // 只有在有真实数据时才显示趋势图
  const hasRealData = stats && stats.total_articles > 0;

  const statusData = [
    { name: t("article.published"), value: stats?.published_articles ?? 0, color: "#22c55e" },
    { name: t("article.draft"), value: (stats?.total_articles ?? 0) - (stats?.published_articles ?? 0), color: "#94a3b8" },
  ];

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">{t("manage.dashboard")}</h1>
        <p className="text-muted-foreground">{t("manage.welcome")}</p>
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-4">
        {statCards.map((card) => (
          <Card key={card.title}>
            <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
              <CardTitle className="text-sm font-medium">
                {card.title}
              </CardTitle>
              <card.icon className={`h-4 w-4 ${card.color}`} />
            </CardHeader>
            <CardContent>
              {loading ? (
                <div className="h-8 w-16 bg-muted animate-pulse rounded" />
              ) : (
                <div className="text-2xl font-bold">{card.value}</div>
              )}
            </CardContent>
          </Card>
        ))}
      </div>

      <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-7">
        {/* Article Status Chart */}
        <Card className="lg:col-span-4">
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <TrendingUp className="h-4 w-4" />
              {t("article.status")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {!hasRealData ? (
              <div className="h-[200px] flex items-center justify-center text-muted-foreground">
                {t("article.noArticles")}
              </div>
            ) : (
              <div className="h-[200px]">
                <ResponsiveContainer width="100%" height="100%">
                  <PieChart>
                    <Pie
                      data={statusData}
                      cx="50%"
                      cy="50%"
                      innerRadius={50}
                      outerRadius={70}
                      paddingAngle={5}
                      dataKey="value"
                    >
                      {statusData.map((entry, index) => (
                        <Cell key={`cell-${index}`} fill={entry.color} />
                      ))}
                    </Pie>
                    <Tooltip
                      contentStyle={{
                        backgroundColor: "hsl(var(--card))",
                        border: "1px solid hsl(var(--border))",
                        borderRadius: "8px",
                      }}
                    />
                  </PieChart>
                </ResponsiveContainer>
              </div>
            )}
            {hasRealData && (
              <div className="flex justify-center gap-4 mt-2">
                {statusData.map((item) => (
                  <div key={item.name} className="flex items-center gap-2">
                    <div
                      className="w-3 h-3 rounded-full"
                      style={{ backgroundColor: item.color }}
                    />
                    <span className="text-sm text-muted-foreground">
                      {item.name}: {item.value}
                    </span>
                  </div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        {/* Status Pie Chart */}
        <Card className="lg:col-span-3">
          <CardHeader>
            <CardTitle>{t("manage.quickActions")}</CardTitle>
          </CardHeader>
          <CardContent className="space-y-2">
            <a
              href="/manage/articles/new"
              className="flex items-center gap-2 p-3 rounded-lg hover:bg-muted transition-colors"
            >
              <FileText className="h-4 w-4" />
              <span>{t("article.newArticle")}</span>
            </a>
            <a
              href="/manage/categories"
              className="flex items-center gap-2 p-3 rounded-lg hover:bg-muted transition-colors"
            >
              <FolderTree className="h-4 w-4" />
              <span>{t("manage.categories")}</span>
            </a>
            <a
              href="/manage/tags"
              className="flex items-center gap-2 p-3 rounded-lg hover:bg-muted transition-colors"
            >
              <Tags className="h-4 w-4" />
              <span>{t("manage.tags")}</span>
            </a>
          </CardContent>
        </Card>
      </div>

      <div className="grid gap-4 md:grid-cols-2">
        {/* Recent Articles */}
        <Card>
          <CardHeader>
            <CardTitle>{t("manage.recentArticles")}</CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <div key={i} className="h-12 bg-muted animate-pulse rounded" />
                ))}
              </div>
            ) : recentArticles.length > 0 ? (
              <div className="space-y-3">
                {recentArticles.map((article) => (
                  <a
                    key={article.id}
                    href={`/manage/articles/${article.id}`}
                    className="flex items-center justify-between p-3 rounded-lg hover:bg-muted transition-colors"
                  >
                    <div className="flex-1 min-w-0">
                      <p className="font-medium truncate">{article.title}</p>
                      <p className="text-xs text-muted-foreground">
                        {new Date(article.created_at).toLocaleDateString(getDateLocale())}
                      </p>
                    </div>
                    <span
                      className={`text-xs px-2 py-1 rounded ${
                        article.status === "published"
                          ? "bg-green-100 text-green-700 dark:bg-green-900 dark:text-green-300"
                          : "bg-gray-100 text-gray-700 dark:bg-gray-800 dark:text-gray-300"
                      }`}
                    >
                      {article.status === "published" ? t("article.published") : t("article.draft")}
                    </span>
                  </a>
                ))}
              </div>
            ) : (
              <p className="text-sm text-muted-foreground text-center py-4">
                {t("article.noArticles")}
              </p>
            )}
          </CardContent>
        </Card>

        {/* System Performance */}
        <Card>
          <CardHeader>
            <CardTitle className="flex items-center gap-2">
              <Activity className="h-4 w-4" />
              {t("manage.performance")}
            </CardTitle>
          </CardHeader>
          <CardContent>
            {loading || !systemStats ? (
              <div className="space-y-3">
                {[1, 2, 3].map((i) => (
                  <div key={i} className="h-8 bg-muted animate-pulse rounded" />
                ))}
              </div>
            ) : (
              <div className="space-y-4">
                {/* Memory Usage */}
                <div>
                  <div className="flex justify-between text-sm mb-1">
                    <span className="text-muted-foreground flex items-center gap-1">
                      <HardDrive className="h-3 w-3" />
                      {t("manage.memory")}
                    </span>
                    <span className="font-medium">{systemStats.memory_formatted}</span>
                  </div>
                  <div className="h-2 bg-muted rounded-full overflow-hidden">
                    <div 
                      className="h-full bg-green-500 transition-all duration-500"
                      style={{ width: `${Math.min((systemStats.memory_bytes / systemStats.system_total_memory) * 100, 100)}%` }}
                    />
                  </div>
                </div>
                
                {/* Response Time */}
                <div className="flex justify-between text-sm pt-2 border-t">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Zap className="h-3 w-3" />
                    {t("manage.avgResponseTime")}
                  </span>
                  <span className="font-medium">{systemStats.avg_response_time_ms.toFixed(2)} ms</span>
                </div>
                
                {/* Total Requests */}
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Activity className="h-3 w-3" />
                    {t("manage.totalRequests")}
                  </span>
                  <span className="font-medium">{systemStats.total_requests.toLocaleString()}</span>
                </div>
                
                {/* Uptime */}
                <div className="flex justify-between text-sm pt-2 border-t">
                  <span className="text-muted-foreground flex items-center gap-1">
                    <Clock className="h-3 w-3" />
                    {t("manage.uptime")}
                  </span>
                  <span className="font-medium">{systemStats.uptime_formatted}</span>
                </div>
                
                {/* OS */}
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">{t("manage.os")}</span>
                  <span className="font-medium">{systemStats.os_name}</span>
                </div>
                
                {/* Version */}
                <div className="flex justify-between text-sm">
                  <span className="text-muted-foreground">{t("manage.version")}</span>
                  <span className="font-medium">v{systemStats.version}</span>
                </div>
              </div>
            )}
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
