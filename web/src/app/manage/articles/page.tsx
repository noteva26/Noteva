"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
import { motion } from "motion/react";
import { articlesApi, Article, PagedResult } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import {
  Plus,
  Search,
  Edit,
  Trash2,
  ChevronLeft,
  ChevronRight,
  FileText,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation, useI18nStore } from "@/lib/i18n";
import { EmptyState } from "@/components/ui/empty-state";

export default function ArticlesPage() {
  const router = useRouter();
  const { t } = useTranslation();
  const locale = useI18nStore((s) => s.locale);
  const [articles, setArticles] = useState<Article[]>([]);
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [status, setStatus] = useState<string>("all");
  const [search, setSearch] = useState("");

  const getDateLocale = () => {
    switch (locale) {
      case "zh-TW": return "zh-TW";
      case "en": return "en-US";
      default: return "zh-CN";
    }
  };

  const fetchArticles = async () => {
    setLoading(true);
    try {
      const params: Record<string, unknown> = { page, per_page: 10 };
      if (status !== "all") params.status = status;
      const { data } = await articlesApi.list(params);
      setArticles(data?.articles || []);
      setTotalPages(data?.total_pages || 1);
    } catch (error) {
      toast.error(t("error.loadFailed"));
      setArticles([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchArticles();
  }, [page, status]);

  const handleDelete = async (id: number) => {
    try {
      await articlesApi.delete(id);
      toast.success(t("article.deleteSuccess"));
      fetchArticles();
    } catch (error) {
      toast.error(t("article.deleteFailed"));
    }
  };

  const filteredArticles = Array.isArray(articles) ? articles.filter((article) =>
    article.title.toLowerCase().includes(search.toLowerCase())
  ) : [];

  const getStatusBadge = (status: string) => {
    switch (status) {
      case "published":
        return <Badge variant="success">{t("article.published")}</Badge>;
      case "draft":
        return <Badge variant="secondary">{t("article.draft")}</Badge>;
      case "archived":
        return <Badge variant="outline">{t("article.archived")}</Badge>;
      default:
        return <Badge>{status}</Badge>;
    }
  };


  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="flex items-center justify-between"
      >
        <div>
          <h1 className="text-3xl font-bold">{t("manage.articles")}</h1>
          <p className="text-muted-foreground">{t("article.totalArticles")}</p>
        </div>
        <motion.div whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}>
          <Button onClick={() => router.push("/manage/articles/new")}>
            <Plus className="h-4 w-4 mr-2" />
            {t("article.newArticle")}
          </Button>
        </motion.div>
      </motion.div>

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={t("article.searchArticles")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
        <Select value={status} onValueChange={setStatus}>
          <SelectTrigger className="w-[140px]">
            <SelectValue placeholder={t("article.status")} />
          </SelectTrigger>
          <SelectContent>
            <SelectItem value="all">{t("common.all")}</SelectItem>
            <SelectItem value="published">{t("article.published")}</SelectItem>
            <SelectItem value="draft">{t("article.draft")}</SelectItem>
            <SelectItem value="archived">{t("article.archived")}</SelectItem>
          </SelectContent>
        </Select>
      </div>

      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.1 }}
        className="rounded-md border"
      >
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead className="w-[40%]">{t("article.title")}</TableHead>
              <TableHead>{t("article.category")}</TableHead>
              <TableHead>{t("article.status")}</TableHead>
              <TableHead>{t("article.updatedAt")}</TableHead>
              <TableHead className="text-right">{t("common.edit")}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {loading ? (
              Array.from({ length: 5 }).map((_, i) => (
                <TableRow key={i}>
                  <TableCell><div className="h-4 w-[200px] skeleton-shimmer rounded" /></TableCell>
                  <TableCell><div className="h-4 w-[80px] skeleton-shimmer rounded" /></TableCell>
                  <TableCell><div className="h-4 w-[60px] skeleton-shimmer rounded" /></TableCell>
                  <TableCell><div className="h-4 w-[100px] skeleton-shimmer rounded" /></TableCell>
                  <TableCell><div className="h-4 w-[80px] ml-auto skeleton-shimmer rounded" /></TableCell>
                </TableRow>
              ))
            ) : filteredArticles.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="h-24">
                  <EmptyState
                    size="sm"
                    icon={FileText}
                    description={t("article.noArticles")}
                    actionText={t("article.newArticle")}
                    onAction={() => router.push("/manage/articles/new")}
                  />
                </TableCell>
              </TableRow>
            ) : (
              filteredArticles.map((article, index) => (
                <motion.tr
                  key={article.id}
                  initial={{ opacity: 0, x: -10 }}
                  animate={{ opacity: 1, x: 0 }}
                  transition={{ delay: index * 0.03 }}
                  className="border-b transition-colors hover:bg-muted/50 data-[state=selected]:bg-muted"
                >
                  <TableCell className="font-medium">
                    <Link
                      href={`/manage/articles/${article.id}`}
                      className="hover:underline"
                    >
                      {article.title}
                    </Link>
                  </TableCell>
                  <TableCell>{article.category?.name || "-"}</TableCell>
                  <TableCell>{getStatusBadge(article.status)}</TableCell>
                  <TableCell className="text-muted-foreground">
                    {new Date(article.updated_at).toLocaleDateString(getDateLocale())}
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex items-center justify-end gap-2">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => router.push(`/manage/articles/${article.id}`)}
                      >
                        <Edit className="h-4 w-4" />
                      </Button>
                      <AlertDialog>
                        <AlertDialogTrigger asChild>
                          <Button variant="ghost" size="icon">
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        </AlertDialogTrigger>
                        <AlertDialogContent>
                          <AlertDialogHeader>
                            <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
                            <AlertDialogDescription>
                              {t("article.confirmDelete", { title: article.title })}
                            </AlertDialogDescription>
                          </AlertDialogHeader>
                          <AlertDialogFooter>
                            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
                            <AlertDialogAction onClick={() => handleDelete(article.id)}>
                              {t("common.delete")}
                            </AlertDialogAction>
                          </AlertDialogFooter>
                        </AlertDialogContent>
                      </AlertDialog>
                    </div>
                  </TableCell>
                </motion.tr>
              ))
            )}
          </TableBody>
        </Table>
      </motion.div>

      {totalPages > 1 && (
        <div className="flex items-center justify-center gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => setPage((p) => Math.max(1, p - 1))}
            disabled={page === 1}
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <span className="text-sm text-muted-foreground">
            {t("pagination.page", { current: page.toString(), total: totalPages.toString() })}
          </span>
          <Button
            variant="outline"
            size="sm"
            onClick={() => setPage((p) => Math.min(totalPages, p + 1))}
            disabled={page === totalPages}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      )}
    </div>
  );
}
