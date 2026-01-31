"use client";

import { useEffect, useState } from "react";
import Link from "next/link";
import { useRouter } from "next/navigation";
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
import { Skeleton } from "@/components/ui/skeleton";
import {
  Plus,
  Search,
  Edit,
  Trash2,
  ChevronLeft,
  ChevronRight,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation, useI18nStore } from "@/lib/i18n";

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
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t("manage.articles")}</h1>
          <p className="text-muted-foreground">{t("article.totalArticles")}</p>
        </div>
        <Button onClick={() => router.push("/manage/articles/new")}>
          <Plus className="h-4 w-4 mr-2" />
          {t("article.newArticle")}
        </Button>
      </div>

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

      <div className="rounded-md border">
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
                  <TableCell><Skeleton className="h-4 w-[200px]" /></TableCell>
                  <TableCell><Skeleton className="h-4 w-[80px]" /></TableCell>
                  <TableCell><Skeleton className="h-4 w-[60px]" /></TableCell>
                  <TableCell><Skeleton className="h-4 w-[100px]" /></TableCell>
                  <TableCell><Skeleton className="h-4 w-[80px] ml-auto" /></TableCell>
                </TableRow>
              ))
            ) : filteredArticles.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="h-24 text-center">
                  {t("article.noArticles")}
                </TableCell>
              </TableRow>
            ) : (
              filteredArticles.map((article) => (
                <TableRow key={article.id}>
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
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

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
