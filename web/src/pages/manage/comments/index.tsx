import {
  useCallback,
  useEffect,
  useMemo,
  useOptimistic,
  useRef,
  useState,
  useTransition,
} from "react";
import { commentsApi, AdminComment } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  AlertCircle,
  Check,
  ChevronLeft,
  ChevronRight,
  Loader2,
  MessageSquare,
  RefreshCw,
  Trash2,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";

const PER_PAGE = 15;

const STATUS_TABS_KEYS = [
  { key: "", labelKey: "comment.all" },
  { key: "pending", labelKey: "comment.pending" },
  { key: "approved", labelKey: "comment.approved" },
  { key: "spam", labelKey: "spam" },
] as const;

const STATUS_BADGE_KEYS: Record<
  string,
  { labelKey: string; className: string }
> = {
  pending: {
    labelKey: "comment.pending",
    className:
      "bg-amber-100 text-amber-800 dark:bg-amber-900/30 dark:text-amber-400",
  },
  approved: {
    labelKey: "comment.approved",
    className:
      "bg-emerald-100 text-emerald-800 dark:bg-emerald-900/30 dark:text-emerald-400",
  },
  spam: {
    labelKey: "spam",
    className: "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400",
  },
};

type OptimisticCommentAction =
  | {
      type: "status";
      id: number;
      status: string;
      currentFilter: string;
    }
  | {
      type: "delete";
      id: number;
    };

function getStatusLabel(
  status: string,
  t: (key: string, params?: Record<string, string | number>) => string
) {
  if (status === "spam") return "Spam";
  return t(STATUS_BADGE_KEYS[status]?.labelKey || status);
}

function updateOptimisticComments(
  comments: AdminComment[],
  action: OptimisticCommentAction
) {
  if (action.type === "delete") {
    return comments.filter((comment) => comment.id !== action.id);
  }

  return comments.flatMap((comment) => {
    if (comment.id !== action.id) return [comment];

    if (action.currentFilter && action.currentFilter !== action.status) {
      return [];
    }

    return [{ ...comment, status: action.status }];
  });
}

function truncate(value: string, max = 80) {
  return value.length > max ? `${value.slice(0, max)}...` : value;
}

export default function CommentsPage() {
  const { t, locale } = useTranslation();
  const mountedRef = useRef(false);
  const requestIdRef = useRef(0);
  const [comments, setComments] = useState<AdminComment[]>([]);
  const [optimisticComments, applyOptimisticComment] = useOptimistic(
    comments,
    updateOptimisticComments
  );
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);
  const [statusFilter, setStatusFilter] = useState("");
  const [actionLoading, setActionLoading] = useState<number | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);
  const [isRefreshing, startRefreshTransition] = useTransition();
  const [, startActionTransition] = useTransition();

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  const fetchComments = useCallback(
    async (showRefresh = false) => {
      const requestId = ++requestIdRef.current;

      if (!showRefresh) {
        setLoading(true);
      }

      try {
        const { data } = await commentsApi.listAll({
          page,
          per_page: PER_PAGE,
          status: statusFilter || undefined,
        });

        if (!mountedRef.current || requestId !== requestIdRef.current) return;

        setComments(data.comments);
        setTotalPages(data.total_pages);
        setTotal(data.total);
      } catch {
        if (mountedRef.current && requestId === requestIdRef.current) {
          toast.error(t("error.loadFailed"));
        }
      } finally {
        if (mountedRef.current && requestId === requestIdRef.current) {
          if (!showRefresh) {
            setLoading(false);
            setHasLoaded(true);
          }
        }
      }
    },
    [page, statusFilter, t]
  );

  useEffect(() => {
    void fetchComments();
  }, [fetchComments]);

  const runCommentAction = useCallback(
    (
      id: number,
      optimisticAction: OptimisticCommentAction,
      request: () => Promise<unknown>,
      successMessage: string,
      errorMessage: string
    ) => {
      setActionLoading(id);

      startActionTransition(async () => {
        applyOptimisticComment(optimisticAction);

        try {
          await request();
          toast.success(successMessage);
          setDeleteConfirm(null);
          await fetchComments(true);
        } catch {
          setComments((current) => [...current]);
          toast.error(errorMessage);
        } finally {
          if (mountedRef.current) {
            setActionLoading(null);
          }
        }
      });
    },
    [applyOptimisticComment, fetchComments]
  );

  const handleApprove = useCallback(
    (id: number) => {
      runCommentAction(
        id,
        { type: "status", id, status: "approved", currentFilter: statusFilter },
        () => commentsApi.approve(id),
        t("comment.approveSuccess"),
        t("comment.operationFailed")
      );
    },
    [runCommentAction, statusFilter, t]
  );

  const handleReject = useCallback(
    (id: number) => {
      runCommentAction(
        id,
        { type: "status", id, status: "spam", currentFilter: statusFilter },
        () => commentsApi.reject(id),
        t("comment.markedSpam"),
        t("comment.operationFailed")
      );
    },
    [runCommentAction, statusFilter, t]
  );

  const handleDelete = useCallback(
    (id: number) => {
      runCommentAction(
        id,
        { type: "delete", id },
        () => commentsApi.delete(id),
        t("comment.deleteSuccess"),
        t("comment.deleteFailed")
      );
    },
    [runCommentAction, t]
  );

  const formatDate = useCallback(
    (dateStr: string) =>
      new Date(dateStr).toLocaleString(locale, {
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
      }),
    [locale]
  );

  const handleStatusChange = useCallback((status: string) => {
    setStatusFilter(status);
    setPage(1);
    setDeleteConfirm(null);
  }, []);

  const currentStatusTitle = useMemo(() => {
    return statusFilter ? getStatusLabel(statusFilter, t) : t("comment.allComments");
  }, [statusFilter, t]);

  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = (loading && hasLoaded) || isRefreshing;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="flex items-center gap-2 text-2xl font-bold tracking-tight">
            <MessageSquare className="h-6 w-6" />
            {t("manage.comments")}
          </h1>
          <p className="mt-1 text-sm text-muted-foreground">
            {t("comment.totalComments", { count: total.toString() })}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => startRefreshTransition(async () => { await fetchComments(true); })}
          disabled={isRefreshing}
        >
          <RefreshCw
            className={cn("mr-2 h-4 w-4", isRefreshing && "animate-spin")}
          />
          {t("common.refresh")}
        </Button>
      </div>
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />

      <div className="flex w-fit gap-1 rounded-lg bg-muted p-1">
        {STATUS_TABS_KEYS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => handleStatusChange(tab.key)}
            className={cn(
              "rounded-md px-3 py-1.5 text-sm transition-colors",
              statusFilter === tab.key
                ? "bg-background font-medium text-foreground shadow-sm"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            {tab.labelKey === "spam" ? "Spam" : t(tab.labelKey)}
          </button>
        ))}
      </div>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">{currentStatusTitle}</CardTitle>
        </CardHeader>
        <CardContent>
          <DataSyncBar active={isSyncing} label={t("common.loading")} className="mb-3" />
          {showInitialLoading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : optimisticComments.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <MessageSquare className="mb-3 h-10 w-10 opacity-50" />
              <p>{t("comment.noComments")}</p>
            </div>
          ) : (
            <>
              <Table className={cn(isSyncing && "opacity-70 transition-opacity")}>
                <TableHeader>
                  <TableRow>
                    <TableHead>ID</TableHead>
                    <TableHead>{t("comment.commentContent")}</TableHead>
                    <TableHead className="w-32">
                      {t("comment.commenter")}
                    </TableHead>
                    <TableHead className="w-24">{t("comment.status")}</TableHead>
                    <TableHead className="w-36">{t("comment.time")}</TableHead>
                    <TableHead className="w-28 text-right">
                      {t("comment.actions")}
                    </TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {optimisticComments.map((comment) => {
                    const badge = STATUS_BADGE_KEYS[comment.status] || {
                      labelKey: comment.status,
                      className: "bg-muted text-muted-foreground",
                    };
                    const isRowLoading = actionLoading === comment.id;

                    return (
                      <TableRow key={comment.id}>
                        <TableCell className="font-mono text-xs text-muted-foreground">
                          {comment.id}
                        </TableCell>
                        <TableCell>
                          <p className="text-sm leading-relaxed">
                            {truncate(comment.content)}
                          </p>
                        </TableCell>
                        <TableCell>
                          <span className="text-sm">
                            {comment.nickname || t("comment.anonymous")}
                          </span>
                        </TableCell>
                        <TableCell>
                          <span
                            className={cn(
                              "inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium",
                              badge.className
                            )}
                          >
                            {badge.labelKey === "spam"
                              ? "Spam"
                              : t(badge.labelKey)}
                          </span>
                        </TableCell>
                        <TableCell className="text-xs text-muted-foreground">
                          {formatDate(comment.created_at)}
                        </TableCell>
                        <TableCell className="text-right">
                          {deleteConfirm === comment.id ? (
                            <div className="flex items-center justify-end gap-1">
                              <Button
                                size="sm"
                                variant="destructive"
                                className="h-7 px-2 text-xs"
                                onClick={() => handleDelete(comment.id)}
                                disabled={isRowLoading}
                              >
                                {isRowLoading ? (
                                  <Loader2 className="h-3 w-3 animate-spin" />
                                ) : (
                                  t("common.confirm")
                                )}
                              </Button>
                              <Button
                                size="sm"
                                variant="ghost"
                                className="h-7 px-2 text-xs"
                                onClick={() => setDeleteConfirm(null)}
                              >
                                {t("common.cancel")}
                              </Button>
                            </div>
                          ) : (
                            <div className="flex items-center justify-end gap-1">
                              {comment.status !== "approved" && (
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  className="h-7 w-7 p-0 text-emerald-600 hover:bg-emerald-100 hover:text-emerald-700 dark:hover:bg-emerald-900/30"
                                  onClick={() => handleApprove(comment.id)}
                                  disabled={isRowLoading}
                                  title={t("comment.approve")}
                                >
                                  {isRowLoading ? (
                                    <Loader2 className="h-4 w-4 animate-spin" />
                                  ) : (
                                    <Check className="h-4 w-4" />
                                  )}
                                </Button>
                              )}
                              {comment.status !== "spam" && (
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  className="h-7 w-7 p-0 text-amber-600 hover:bg-amber-100 hover:text-amber-700 dark:hover:bg-amber-900/30"
                                  onClick={() => handleReject(comment.id)}
                                  disabled={isRowLoading}
                                  title={t("comment.markSpam")}
                                >
                                  <AlertCircle className="h-4 w-4" />
                                </Button>
                              )}
                              <Button
                                size="sm"
                                variant="ghost"
                                className="h-7 w-7 p-0 text-red-600 hover:bg-red-100 hover:text-red-700 dark:hover:bg-red-900/30"
                                onClick={() => setDeleteConfirm(comment.id)}
                                disabled={isRowLoading}
                                title={t("common.delete")}
                              >
                                <Trash2 className="h-4 w-4" />
                              </Button>
                            </div>
                          )}
                        </TableCell>
                      </TableRow>
                    );
                  })}
                </TableBody>
              </Table>

              {totalPages > 1 && (
                <div className="mt-4 flex items-center justify-between border-t pt-4">
                  <p className="text-sm text-muted-foreground">
                    {t("comment.pageInfo", {
                      current: page.toString(),
                      total: totalPages.toString(),
                    })}
                  </p>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setPage((current) => Math.max(1, current - 1))}
                      disabled={page <= 1 || showInitialLoading}
                    >
                      <ChevronLeft className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        setPage((current) =>
                          Math.min(totalPages, current + 1)
                        )
                      }
                      disabled={page >= totalPages || showInitialLoading}
                    >
                      <ChevronRight className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
