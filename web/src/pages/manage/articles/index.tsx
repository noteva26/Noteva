import {
  useCallback,
  useDeferredValue,
  useEffect,
  useMemo,
  useOptimistic,
  useRef,
  useState,
  useTransition,
} from "react";
import { Link, useNavigate } from "react-router-dom";
import { AnimatePresence, motion } from "motion/react";
import {
  type ColumnDef,
  type RowSelectionState,
  type SortingState,
  type VisibilityState,
  flexRender,
  getCoreRowModel,
  getSortedRowModel,
  useReactTable,
} from "@tanstack/react-table";
import { AdminPageHeader } from "@/components/admin/page-header";
import { DataTableColumnHeader } from "@/components/admin/data-table/data-table-column-header";
import { DataTablePagination } from "@/components/admin/data-table/data-table-pagination";
import { DataTableViewOptions } from "@/components/admin/data-table/data-table-view-options";
import { articlesApi, Article } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
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
} from "@/components/ui/alert-dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import {
  Archive,
  BarChart3,
  CheckCircle2,
  Clock,
  Copy,
  Edit,
  Eye,
  FileClock,
  FileText,
  ListFilter,
  Loader2,
  MessageSquare,
  MoreHorizontal,
  Pin,
  Plus,
  RefreshCw,
  Search,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation, useI18nStore } from "@/lib/i18n";
import { preloadManageRoute } from "@/lib/manage-routes";
import { EmptyState } from "@/components/ui/empty-state";
import { cn } from "@/lib/utils";

const PER_PAGE = 10;
const NEW_ARTICLE_PATH = "/manage/articles/new";

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

function getEditPath(articleId: number) {
  return `/manage/articles/${articleId}`;
}

function removeOptimisticArticle(articles: Article[], id: number) {
  return articles.filter((article) => article.id !== id);
}

function isChineseLocale(locale: string) {
  return locale.startsWith("zh");
}

function localText(locale: string, zh: string, en: string) {
  return isChineseLocale(locale) ? zh : en;
}

function getArticleHref(article: Article) {
  return article.slug ? `/posts/${article.slug}` : `/manage/articles/${article.id}`;
}

function getStatusBadge(
  article: Article,
  t: (key: string, params?: Record<string, string | number>) => string
) {
  if (article.status === "draft" && article.scheduled_at) {
    return (
      <Badge variant="secondary" className="gap-1">
        <Clock className="h-3 w-3" />
        {t("article.scheduled")}
      </Badge>
    );
  }

  switch (article.status) {
    case "published":
      return <Badge variant="success">{t("article.published")}</Badge>;
    case "draft":
      return <Badge variant="secondary">{t("article.draft")}</Badge>;
    case "archived":
      return <Badge variant="outline">{t("article.archived")}</Badge>;
    default:
      return <Badge>{article.status}</Badge>;
  }
}

function formatArticleDate(formatter: Intl.DateTimeFormat, value?: string | null) {
  if (!value) return "-";

  const date = new Date(value);
  return Number.isNaN(date.getTime()) ? "-" : formatter.format(date);
}

export default function ArticlesPage() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const locale = useI18nStore((state) => state.locale);
  const mountedRef = useRef(false);
  const requestIdRef = useRef(0);
  const hasLoadedArticlesRef = useRef(false);
  const [articles, setArticles] = useState<Article[]>([]);
  const [optimisticArticles, deleteOptimisticArticle] = useOptimistic(
    articles,
    removeOptimisticArticle
  );
  const [loading, setLoading] = useState(true);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [totalArticles, setTotalArticles] = useState(0);
  const [status, setStatus] = useState<string>("all");
  const [search, setSearch] = useState("");
  const [articleToDelete, setArticleToDelete] = useState<Article | null>(null);
  const [deletingId, setDeletingId] = useState<number | null>(null);
  const [sorting, setSorting] = useState<SortingState>([]);
  const [columnVisibility, setColumnVisibility] = useState<VisibilityState>({
    tags: false,
    views: false,
    comments: false,
  });
  const [rowSelection, setRowSelection] = useState<RowSelectionState>({});
  const [, startDeleteTransition] = useTransition();
  const [isFilterTransitionPending, startFilterTransition] = useTransition();
  const [isRefreshing, startRefreshTransition] = useTransition();
  const [isFetchingArticles, setIsFetchingArticles] = useState(false);
  const deferredSearch = useDeferredValue(search);
  const normalizedSearch = useMemo(
    () => deferredSearch.trim().toLowerCase(),
    [deferredSearch]
  );
  const dateLocale = getDateLocale(locale);
  const dateFormatter = useMemo(
    () =>
      new Intl.DateTimeFormat(dateLocale, {
        year: "numeric",
        month: "short",
        day: "numeric",
      }),
    [dateLocale]
  );
  const numberFormatter = useMemo(
    () => new Intl.NumberFormat(dateLocale),
    [dateLocale]
  );

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  const fetchArticles = useCallback(
    async (showLoading = true) => {
      const requestId = ++requestIdRef.current;
      const shouldShowSkeleton = showLoading && !hasLoadedArticlesRef.current;

      if (shouldShowSkeleton) {
        setLoading(true);
      }
      setIsFetchingArticles(true);

      try {
        const params: Record<string, unknown> = {
          page,
          per_page: PER_PAGE,
        };

        if (status !== "all") {
          params.status = status;
        }

        const { data } = await articlesApi.list(params);

        if (!mountedRef.current || requestId !== requestIdRef.current) return;

        setArticles(data?.articles || []);
        setTotalPages(data?.total_pages || 1);
        setTotalArticles(data?.total || data?.articles?.length || 0);
        hasLoadedArticlesRef.current = true;
      } catch {
        if (mountedRef.current && requestId === requestIdRef.current) {
          toast.error(t("error.loadFailed"));
          setArticles([]);
          setTotalPages(1);
          setTotalArticles(0);
        }
      } finally {
        if (mountedRef.current && requestId === requestIdRef.current) {
          setLoading(false);
          setIsFetchingArticles(false);
        }
      }
    },
    [page, status, t]
  );

  useEffect(() => {
    void fetchArticles(true);
  }, [fetchArticles]);

  useEffect(() => {
    setRowSelection({});
  }, [normalizedSearch, page, status]);

  const handleDelete = useCallback(
    (article: Article) => {
      setDeletingId(article.id);

      startDeleteTransition(async () => {
        deleteOptimisticArticle(article.id);

        try {
          await articlesApi.delete(article.id);
          toast.success(t("article.deleteSuccess"));
          setArticleToDelete(null);
          await fetchArticles(false);
        } catch {
          setArticles((current) => [...current]);
          toast.error(t("article.deleteFailed"));
        } finally {
          if (mountedRef.current) {
            setDeletingId(null);
          }
        }
      });
    },
    [deleteOptimisticArticle, fetchArticles, t]
  );

  const handleStatusChange = useCallback(
    (nextStatus: string) => {
      if (nextStatus === status) return;

      startFilterTransition(() => {
        setStatus(nextStatus);
        setPage(1);
      });
    },
    [status]
  );

  const filteredArticles = useMemo(() => {
    if (!normalizedSearch) return optimisticArticles;

    return optimisticArticles.filter((article) => {
      const title = article.title.toLowerCase();
      const category = article.category?.name.toLowerCase() || "";
      const tags = article.tags?.map((tag) => tag.name.toLowerCase()).join(" ") || "";
      return title.includes(normalizedSearch) || category.includes(normalizedSearch) || tags.includes(normalizedSearch);
    });
  }, [normalizedSearch, optimisticArticles]);

  const openNewArticle = useCallback(() => {
    navigate(NEW_ARTICLE_PATH);
  }, [navigate]);

  const refreshArticles = useCallback(() => {
    startRefreshTransition(async () => {
      await fetchArticles(false);
    });
  }, [fetchArticles]);

  const goToPreviousPage = useCallback(() => {
    startFilterTransition(() => {
      setPage((current) => Math.max(1, current - 1));
    });
  }, []);

  const goToNextPage = useCallback(() => {
    startFilterTransition(() => {
      setPage((current) => Math.min(totalPages, current + 1));
    });
  }, [totalPages]);

  const copyArticleLink = useCallback(
    async (article: Article) => {
      const url = `${window.location.origin}${getArticleHref(article)}`;
      try {
        await navigator.clipboard.writeText(url);
        toast.success(localText(locale, "链接已复制", "Link copied"));
      } catch {
        toast.error(localText(locale, "复制失败", "Copy failed"));
      }
    },
    [locale]
  );

  const columnHeaderLabels = useMemo(
    () => ({
      sortAscendingLabel: localText(locale, "升序", "Asc"),
      sortDescendingLabel: localText(locale, "降序", "Desc"),
      hideLabel: localText(locale, "隐藏", "Hide"),
    }),
    [locale]
  );

  const columnLabels = useMemo(
    () => ({
      title: t("article.title"),
      category: t("article.category"),
      tags: t("article.tags"),
      status: t("article.status"),
      updated_at: t("article.updatedAt"),
      views: localText(locale, "浏览", "Views"),
      comments: localText(locale, "评论", "Comments"),
    }),
    [locale, t]
  );

  const columns = useMemo<ColumnDef<Article>[]>(
    () => [
      {
        id: "select",
        header: ({ table }) => (
          <Checkbox
            checked={
              table.getIsAllPageRowsSelected() ||
              (table.getIsSomePageRowsSelected() && "indeterminate")
            }
            onCheckedChange={(value) => table.toggleAllPageRowsSelected(Boolean(value))}
            aria-label={localText(locale, "选择全部", "Select all")}
          />
        ),
        cell: ({ row }) => (
          <Checkbox
            checked={row.getIsSelected()}
            onCheckedChange={(value) => row.toggleSelected(Boolean(value))}
            aria-label={localText(locale, "选择行", "Select row")}
          />
        ),
        enableSorting: false,
        enableHiding: false,
      },
      {
        accessorKey: "title",
        header: ({ column }) => (
          <DataTableColumnHeader
            column={column}
            title={t("article.title")}
            {...columnHeaderLabels}
          />
        ),
        cell: ({ row }) => {
          const article = row.original;
          const editPath = getEditPath(article.id);

          return (
            <div className="flex min-w-[320px] flex-col gap-1.5 py-1">
              <div className="flex items-center gap-2">
                {article.is_pinned ? (
                  <span className="inline-flex h-5 w-5 items-center justify-center rounded-md bg-primary/10 text-primary">
                    <Pin className="h-3.5 w-3.5" />
                  </span>
                ) : null}
                <Link
                  to={editPath}
                  className="line-clamp-1 font-medium text-foreground underline-offset-4 hover:underline"
                  onFocus={() => preloadManageRoute(editPath)}
                  onMouseEnter={() => preloadManageRoute(editPath)}
                >
                  {article.title}
                </Link>
              </div>
              <div className="flex flex-wrap items-center gap-2 text-xs text-muted-foreground">
                <span className="max-w-[220px] truncate rounded-md bg-muted px-1.5 py-0.5 font-mono">
                  {article.slug}
                </span>
                {article.scheduled_at ? (
                  <span className="inline-flex items-center gap-1 rounded-md bg-amber-500/10 px-1.5 py-0.5 text-amber-700 dark:text-amber-300">
                    <Clock className="h-3 w-3" />
                    {formatArticleDate(dateFormatter, article.scheduled_at)}
                  </span>
                ) : null}
              </div>
            </div>
          );
        },
      },
      {
        id: "category",
        accessorFn: (article) => article.category?.name || "-",
        header: ({ column }) => (
          <DataTableColumnHeader
            column={column}
            title={t("article.category")}
            {...columnHeaderLabels}
          />
        ),
        cell: ({ row }) => (
          row.original.category?.name ? (
            <Badge variant="outline" className="font-normal">
              {row.original.category.name}
            </Badge>
          ) : (
            <span className="text-sm text-muted-foreground">-</span>
          )
        ),
      },
      {
        id: "tags",
        accessorFn: (article) => article.tags?.map((tag) => tag.name).join(", ") || "",
        header: ({ column }) => (
          <DataTableColumnHeader
            column={column}
            title={t("article.tags")}
            {...columnHeaderLabels}
          />
        ),
        cell: ({ row }) => {
          const tags = row.original.tags || [];
          if (tags.length === 0) return <span className="text-muted-foreground">-</span>;

          return (
            <div className="flex max-w-[220px] flex-wrap gap-1">
              {tags.slice(0, 3).map((tag) => (
                <Badge key={tag.id} variant="outline" className="text-[11px]">
                  {tag.name}
                </Badge>
              ))}
              {tags.length > 3 ? (
                <Badge variant="secondary" className="text-[11px]">+{tags.length - 3}</Badge>
              ) : null}
            </div>
          );
        },
      },
      {
        accessorKey: "status",
        header: ({ column }) => (
          <DataTableColumnHeader
            column={column}
            title={t("article.status")}
            {...columnHeaderLabels}
          />
        ),
        cell: ({ row }) => getStatusBadge(row.original, t),
      },
      {
        accessorKey: "updated_at",
        header: ({ column }) => (
          <DataTableColumnHeader
            column={column}
            title={t("article.updatedAt")}
            {...columnHeaderLabels}
          />
        ),
        cell: ({ row }) => (
          <div className="whitespace-nowrap text-sm">
            <div className="text-foreground">
              {formatArticleDate(dateFormatter, row.original.updated_at)}
            </div>
            {row.original.reading_time || row.original.word_count ? (
              <div className="text-xs text-muted-foreground">
                {row.original.reading_time
                  ? localText(locale, `${row.original.reading_time} 分钟`, `${row.original.reading_time} min`)
                  : numberFormatter.format(row.original.word_count || 0)}
              </div>
            ) : null}
          </div>
        ),
      },
      {
        id: "views",
        accessorFn: (article) => article.view_count || 0,
        header: ({ column }) => (
          <DataTableColumnHeader
            column={column}
            title={localText(locale, "浏览", "Views")}
            {...columnHeaderLabels}
          />
        ),
        cell: ({ row }) => (
          <span className="inline-flex items-center gap-1 text-sm text-muted-foreground">
            <BarChart3 className="h-3.5 w-3.5" />
            {numberFormatter.format(row.original.view_count || 0)}
          </span>
        ),
      },
      {
        id: "comments",
        accessorFn: (article) => article.comment_count || 0,
        header: ({ column }) => (
          <DataTableColumnHeader
            column={column}
            title={localText(locale, "评论", "Comments")}
            {...columnHeaderLabels}
          />
        ),
        cell: ({ row }) => (
          <span className="inline-flex items-center gap-1 text-sm text-muted-foreground">
            <MessageSquare className="h-3.5 w-3.5" />
            {numberFormatter.format(row.original.comment_count || 0)}
          </span>
        ),
      },
      {
        id: "actions",
        enableSorting: false,
        enableHiding: false,
        cell: ({ row }) => {
          const article = row.original;
          const editPath = getEditPath(article.id);
          const publicHref = getArticleHref(article);
          const isDeleting = deletingId === article.id;

          return (
            <div className="flex items-center justify-end gap-1">
              <Button
                variant="ghost"
                size="icon"
                className="h-8 w-8"
                onClick={() => navigate(editPath)}
                onFocus={() => preloadManageRoute(editPath)}
                onMouseEnter={() => preloadManageRoute(editPath)}
                title={t("common.edit")}
              >
                <Edit className="h-4 w-4" />
              </Button>
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="icon" className="h-8 w-8" disabled={isDeleting}>
                    {isDeleting ? (
                      <Loader2 className="h-4 w-4 animate-spin" />
                    ) : (
                      <MoreHorizontal className="h-4 w-4" />
                    )}
                    <span className="sr-only">{localText(locale, "打开菜单", "Open menu")}</span>
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end" className="w-40">
                  <DropdownMenuItem onClick={() => navigate(editPath)}>
                    <Edit className="mr-2 h-4 w-4" />
                    {t("common.edit")}
                  </DropdownMenuItem>
                  <DropdownMenuItem asChild>
                    <a href={publicHref} target="_blank" rel="noopener noreferrer">
                      <Eye className="mr-2 h-4 w-4" />
                      {t("article.preview")}
                    </a>
                  </DropdownMenuItem>
                  <DropdownMenuItem onClick={() => copyArticleLink(article)}>
                    <Copy className="mr-2 h-4 w-4" />
                    {localText(locale, "复制链接", "Copy link")}
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem
                    className="text-destructive focus:text-destructive"
                    onClick={() => setArticleToDelete(article)}
                  >
                    <Trash2 className="mr-2 h-4 w-4" />
                    {t("common.delete")}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            </div>
          );
        },
      },
    ],
    [
      columnHeaderLabels,
      copyArticleLink,
      dateFormatter,
      deletingId,
      locale,
      navigate,
      numberFormatter,
      t,
    ]
  );

  const table = useReactTable({
    data: filteredArticles,
    columns,
    state: {
      sorting,
      columnVisibility,
      rowSelection,
    },
    enableRowSelection: true,
    onSortingChange: setSorting,
    onColumnVisibilityChange: setColumnVisibility,
    onRowSelectionChange: setRowSelection,
    getCoreRowModel: getCoreRowModel(),
    getSortedRowModel: getSortedRowModel(),
  });

  const selectedCount = table.getSelectedRowModel().rows.length;
  const rowCount = table.getRowModel().rows.length;
  const tableSyncing = (isFetchingArticles && !loading) || isFilterTransitionPending;
  const currentPageStats = useMemo(() => {
    const published = optimisticArticles.filter(
      (article) => article.status === "published"
    ).length;
    const drafts = optimisticArticles.filter((article) => article.status === "draft").length;
    const scheduled = optimisticArticles.filter(
      (article) => article.status === "draft" && article.scheduled_at
    ).length;
    const views = optimisticArticles.reduce(
      (sum, article) => sum + (article.view_count || 0),
      0
    );
    const comments = optimisticArticles.reduce(
      (sum, article) => sum + (article.comment_count || 0),
      0
    );

    return { published, drafts, scheduled, views, comments };
  }, [optimisticArticles]);
  const statusOptions = useMemo(
    () => [
      { value: "all", label: t("common.all"), icon: ListFilter },
      { value: "published", label: t("article.published"), icon: CheckCircle2 },
      { value: "draft", label: t("article.draft"), icon: FileClock },
      { value: "archived", label: t("article.archived"), icon: Archive },
    ],
    [t]
  );
  const summaryCards = useMemo(
    () => [
      {
        label: localText(locale, "结果总数", "Results"),
        value: numberFormatter.format(totalArticles),
        detail: localText(
          locale,
          `当前显示 ${numberFormatter.format(rowCount)} 条`,
          `${numberFormatter.format(rowCount)} visible`
        ),
        icon: FileText,
      },
      {
        label: t("article.published"),
        value: numberFormatter.format(currentPageStats.published),
        detail: localText(locale, "当前页已发布", "Published on this page"),
        icon: CheckCircle2,
      },
      {
        label: t("article.draft"),
        value: numberFormatter.format(currentPageStats.drafts),
        detail: currentPageStats.scheduled
          ? localText(
              locale,
              `${numberFormatter.format(currentPageStats.scheduled)} 篇定时`,
              `${numberFormatter.format(currentPageStats.scheduled)} scheduled`
            )
          : localText(locale, "当前页草稿", "Drafts on this page"),
        icon: FileClock,
      },
      {
        label: localText(locale, "互动", "Engagement"),
        value: numberFormatter.format(currentPageStats.views),
        detail: localText(
          locale,
          `${numberFormatter.format(currentPageStats.comments)} 条评论`,
          `${numberFormatter.format(currentPageStats.comments)} comments`
        ),
        icon: BarChart3,
      },
    ],
    [currentPageStats, locale, numberFormatter, rowCount, t, totalArticles]
  );

  return (
    <div className="space-y-6">
      <AdminPageHeader
        title={t("manage.articles")}
        description={localText(
          locale,
          "管理文章、草稿、定时发布和内容状态。",
          "Manage articles, drafts, scheduled publishing, and content status."
        )}
        actions={
          <>
            <Button
              variant="outline"
              onClick={refreshArticles}
              disabled={loading || isRefreshing}
            >
              <RefreshCw className={cn("mr-2 h-4 w-4", isRefreshing && "animate-spin")} />
              {t("common.refresh")}
            </Button>
            <Button
              onClick={openNewArticle}
              onFocus={() => preloadManageRoute(NEW_ARTICLE_PATH)}
              onMouseEnter={() => preloadManageRoute(NEW_ARTICLE_PATH)}
            >
              <Plus className="mr-2 h-4 w-4" />
              {t("article.newArticle")}
            </Button>
          </>
        }
      />

      <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
        {summaryCards.map((card) => {
          const Icon = card.icon;

          return (
            <div
              key={card.label}
              className="rounded-lg border bg-card px-4 py-3 shadow-sm transition-colors hover:bg-muted/20"
            >
              <div className="flex items-center justify-between gap-3">
                <span className="text-sm text-muted-foreground">{card.label}</span>
                <span className="inline-flex h-8 w-8 items-center justify-center rounded-md bg-muted text-muted-foreground">
                  <Icon className="h-4 w-4" />
                </span>
              </div>
              <div className="mt-3 text-2xl font-semibold tracking-tight">{card.value}</div>
              <div className="mt-1 text-xs text-muted-foreground">{card.detail}</div>
            </div>
          );
        })}
      </div>

      <div className="overflow-hidden rounded-lg border bg-card shadow-sm">
        <div className="border-b bg-muted/20 p-4">
          <div className="flex flex-col gap-3 lg:flex-row lg:items-start lg:justify-between">
            <div className="space-y-1">
              <h2 className="text-base font-semibold">
                {localText(locale, "内容列表", "Content list")}
              </h2>
              <p className="text-sm text-muted-foreground">
                {localText(
                  locale,
                  "排序、筛选、选择和列视图现在都集中在这个工作台里。",
                  "Sorting, filtering, selection, and column views now live in one workspace."
                )}
              </p>
            </div>
            <div className="flex flex-wrap items-center gap-2">
              {status !== "all" ? (
                <Badge variant="secondary" className="gap-1">
                  <ListFilter className="h-3 w-3" />
                  {statusOptions.find((option) => option.value === status)?.label}
                </Badge>
              ) : null}
              {selectedCount > 0 ? (
                <Badge variant="outline">
                  {localText(
                    locale,
                    `已选 ${numberFormatter.format(selectedCount)}`,
                    `${numberFormatter.format(selectedCount)} selected`
                  )}
                </Badge>
              ) : null}
              {loading || isRefreshing || tableSyncing ? (
                <Badge variant="outline" className="gap-1">
                  <Loader2 className="h-3 w-3 animate-spin" />
                  {localText(locale, "同步中", "Syncing")}
                </Badge>
              ) : null}
            </div>
          </div>

          <div className="mt-4 flex flex-col gap-3 xl:flex-row xl:items-center">
            <div className="relative min-w-[220px] flex-1 xl:max-w-md">
              <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
              <Input
                placeholder={t("article.searchArticles")}
                value={search}
                onChange={(event) => setSearch(event.target.value)}
                className="h-9 bg-background pl-9"
              />
            </div>
            <div className="hidden items-center rounded-md border bg-background p-1 lg:flex">
              {statusOptions.map((option) => {
                const Icon = option.icon;

                return (
                  <Button
                    key={option.value}
                    type="button"
                    variant="ghost"
                    size="sm"
                    className={cn(
                      "relative h-7 overflow-hidden px-3 text-xs font-medium text-muted-foreground hover:text-foreground",
                      status === option.value && "text-accent-foreground"
                    )}
                    onClick={() => handleStatusChange(option.value)}
                  >
                    {status === option.value ? (
                      <motion.span
                        layoutId="article-status-filter-active"
                        className="absolute inset-0 rounded-sm bg-accent shadow-sm"
                        transition={{ type: "spring", stiffness: 420, damping: 34 }}
                      />
                    ) : null}
                    <span className="relative z-10 inline-flex items-center gap-1.5">
                      <Icon className="h-3.5 w-3.5" />
                      {option.label}
                    </span>
                  </Button>
                );
              })}
            </div>
            <Select value={status} onValueChange={handleStatusChange}>
              <SelectTrigger className="h-9 w-full bg-background lg:hidden">
                <SelectValue placeholder={t("article.status")} />
              </SelectTrigger>
              <SelectContent>
                {statusOptions.map((option) => (
                  <SelectItem key={option.value} value={option.value}>
                    {option.label}
                  </SelectItem>
                ))}
              </SelectContent>
            </Select>
            <DataTableViewOptions
              table={table}
              labels={columnLabels}
              columnsLabel={localText(locale, "列", "Columns")}
              title={localText(locale, "视图", "View")}
            />
          </div>

          <div
            className={cn(
              "mt-3 h-0.5 overflow-hidden rounded-full transition-colors duration-200",
              tableSyncing ? "bg-muted" : "bg-transparent"
            )}
          >
            <AnimatePresence>
              {tableSyncing ? (
                <motion.div
                  className="h-full w-1/3 rounded-full bg-primary shadow-[0_0_10px_hsl(var(--primary)/0.35)]"
                  initial={{ x: "-120%", opacity: 0 }}
                  animate={{ x: ["-120%", "320%"], opacity: 1 }}
                  exit={{ opacity: 0 }}
                  transition={{
                    x: { duration: 1.05, repeat: Infinity, ease: "easeInOut" },
                    opacity: { duration: 0.15 },
                  }}
                />
              ) : null}
            </AnimatePresence>
          </div>
        </div>

        <div
          className={cn(
            "overflow-x-auto transition-opacity duration-200 ease-out",
            tableSyncing && "opacity-60"
          )}
        >
          <Table className="min-w-[940px]">
            <TableHeader className="bg-card">
              {table.getHeaderGroups().map((headerGroup) => (
                <TableRow key={headerGroup.id} className="hover:bg-transparent">
                  {headerGroup.headers.map((header) => (
                    <TableHead
                      key={header.id}
                      className={header.column.id === "actions" ? "w-24 text-right" : undefined}
                    >
                      {header.isPlaceholder
                        ? null
                        : flexRender(header.column.columnDef.header, header.getContext())}
                    </TableHead>
                  ))}
                </TableRow>
              ))}
            </TableHeader>
            <TableBody>
              {loading ? (
                Array.from({ length: 6 }).map((_, index) => (
                  <TableRow key={`article-skeleton-${index}`}>
                    {table.getVisibleLeafColumns().map((column) => (
                      <TableCell key={column.id}>
                        <div
                          className={cn(
                            "h-4 rounded skeleton-shimmer",
                            column.id === "title" ? "w-[240px]" : "w-[88px]",
                            column.id === "actions" && "ml-auto w-[72px]",
                            column.id === "select" && "w-4"
                          )}
                        />
                      </TableCell>
                    ))}
                  </TableRow>
                ))
              ) : table.getRowModel().rows.length === 0 ? (
                <TableRow>
                  <TableCell colSpan={table.getVisibleLeafColumns().length} className="h-40">
                    <EmptyState
                      size="sm"
                      icon={FileText}
                      description={t("article.noArticles")}
                      actionText={t("article.newArticle")}
                      onAction={openNewArticle}
                    />
                  </TableCell>
                </TableRow>
              ) : (
                table.getRowModel().rows.map((row) => (
                  <TableRow key={row.id} data-state={row.getIsSelected() && "selected"}>
                    {row.getVisibleCells().map((cell) => (
                      <TableCell
                        key={cell.id}
                        className={cell.column.id === "actions" ? "text-right" : undefined}
                      >
                        {flexRender(cell.column.columnDef.cell, cell.getContext())}
                      </TableCell>
                    ))}
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </div>

        <DataTablePagination
          page={page}
          totalPages={totalPages}
          loading={loading || tableSyncing}
          selectedCount={selectedCount}
          rowCount={rowCount}
          onPrevious={goToPreviousPage}
          onNext={goToNextPage}
          pageLabel={t("pagination.page", {
            current: page.toString(),
            total: Math.max(1, totalPages).toString(),
          })}
          selectedLabel={(selected, rows) =>
            localText(
              locale,
              `已选择 ${numberFormatter.format(selected)} / ${numberFormatter.format(rows)} 行`,
              `${numberFormatter.format(selected)} of ${numberFormatter.format(rows)} row(s) selected`
            )
          }
        />
      </div>

      <AlertDialog
        open={!!articleToDelete}
        onOpenChange={(open) => {
          if (!open) {
            setArticleToDelete(null);
          }
        }}
      >
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
            <AlertDialogDescription>
              {articleToDelete
                ? t("article.confirmDelete", { title: articleToDelete.title })
                : ""}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              onClick={() => {
                if (articleToDelete) {
                  handleDelete(articleToDelete);
                }
              }}
            >
              {deletingId ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : null}
              {t("common.delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
