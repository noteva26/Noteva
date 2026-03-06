import { useEffect, useState, useCallback } from "react";
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
  MessageSquare,
  Check,
  X,
  RefreshCw,
  ChevronLeft,
  ChevronRight,
  Loader2,
  Trash2,
  AlertCircle,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";

const STATUS_TABS_KEYS = [
  { key: "", labelKey: "comment.all" },
  { key: "pending", labelKey: "comment.pending" },
  { key: "approved", labelKey: "comment.approved" },
  { key: "spam", labelKey: "spam" },
] as const;

const STATUS_BADGE_KEYS: Record<string, { labelKey: string; className: string }> = {
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
    className:
      "bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400",
  },
};

export default function CommentsPage() {
  const { t, locale } = useTranslation();
  const [comments, setComments] = useState<AdminComment[]>([]);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);
  const [statusFilter, setStatusFilter] = useState("");
  const [actionLoading, setActionLoading] = useState<number | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState<number | null>(null);
  const perPage = 15;

  const fetchComments = useCallback(
    async (showRefresh = false) => {
      try {
        if (showRefresh) setRefreshing(true);
        else setLoading(true);
        const { data } = await commentsApi.listAll({
          page,
          per_page: perPage,
          status: statusFilter || undefined,
        });
        setComments(data.comments);
        setTotalPages(data.total_pages);
        setTotal(data.total);
      } catch {
        toast.error(t("error.loadFailed"));
      } finally {
        setLoading(false);
        setRefreshing(false);
      }
    },
    [page, statusFilter]
  );

  useEffect(() => {
    fetchComments();
  }, [fetchComments]);

  const handleApprove = async (id: number) => {
    setActionLoading(id);
    try {
      await commentsApi.approve(id);
      toast.success(t("comment.approveSuccess"));
      fetchComments(true);
    } catch {
      toast.error(t("comment.operationFailed"));
    } finally {
      setActionLoading(null);
    }
  };

  const handleReject = async (id: number) => {
    setActionLoading(id);
    try {
      await commentsApi.reject(id);
      toast.success(t("comment.markedSpam"));
      fetchComments(true);
    } catch {
      toast.error(t("comment.operationFailed"));
    } finally {
      setActionLoading(null);
    }
  };

  const handleDelete = async (id: number) => {
    setActionLoading(id);
    try {
      await commentsApi.delete(id);
      toast.success(t("comment.deleteSuccess"));
      setDeleteConfirm(null);
      fetchComments(true);
    } catch {
      toast.error(t("comment.deleteFailed"));
    } finally {
      setActionLoading(null);
    }
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleString(locale, {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    });
  };

  const truncate = (str: string, max = 80) =>
    str.length > max ? str.slice(0, max) + "…" : str;

  const handleStatusChange = (status: string) => {
    setStatusFilter(status);
    setPage(1);
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-2xl font-bold tracking-tight flex items-center gap-2">
            <MessageSquare className="h-6 w-6" />
            {t("manage.comments")}
          </h1>
          <p className="text-sm text-muted-foreground mt-1">
            {t("comment.totalComments", { count: total.toString() })}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => fetchComments(true)}
          disabled={refreshing}
        >
          <RefreshCw
            className={cn("h-4 w-4 mr-2", refreshing && "animate-spin")}
          />
          {t("common.refresh")}
        </Button>
      </div>

      {/* Status filter tabs */}
      <div className="flex gap-1 p-1 bg-muted rounded-lg w-fit">
        {STATUS_TABS_KEYS.map((tab) => (
          <button
            key={tab.key}
            onClick={() => handleStatusChange(tab.key)}
            className={cn(
              "px-3 py-1.5 text-sm rounded-md transition-colors",
              statusFilter === tab.key
                ? "bg-background text-foreground shadow-sm font-medium"
                : "text-muted-foreground hover:text-foreground"
            )}
          >
            {tab.labelKey === "spam" ? "Spam" : t(tab.labelKey)}
          </button>
        ))}
      </div>

      <Card>
        <CardHeader className="pb-3">
          <CardTitle className="text-base">
            {statusFilter
              ? (statusFilter === "spam" ? "Spam" : t(STATUS_BADGE_KEYS[statusFilter]?.labelKey || statusFilter))
              : t("comment.allComments")}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="flex items-center justify-center py-12">
              <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
            </div>
          ) : comments.length === 0 ? (
            <div className="flex flex-col items-center justify-center py-12 text-muted-foreground">
              <MessageSquare className="h-10 w-10 mb-3 opacity-50" />
              <p>{t("comment.noComments")}</p>
            </div>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>ID</TableHead>
                    <TableHead>{t("comment.commentContent")}</TableHead>
                    <TableHead className="w-32">{t("comment.commenter")}</TableHead>
                    <TableHead className="w-24">{t("comment.status")}</TableHead>
                    <TableHead className="w-36">{t("comment.time")}</TableHead>
                    <TableHead className="w-28 text-right">{t("comment.actions")}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {comments.map((comment) => {
                    const badge = STATUS_BADGE_KEYS[comment.status] || {
                      labelKey: comment.status,
                      className: "bg-muted text-muted-foreground",
                    };
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
                              "inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium",
                              badge.className
                            )}
                          >
                            {badge.labelKey === "spam" ? "Spam" : t(badge.labelKey)}
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
                                disabled={actionLoading === comment.id}
                              >
                                {actionLoading === comment.id ? (
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
                                  className="h-7 w-7 p-0 text-emerald-600 hover:text-emerald-700 hover:bg-emerald-100 dark:hover:bg-emerald-900/30"
                                  onClick={() => handleApprove(comment.id)}
                                  disabled={actionLoading === comment.id}
                                  title={t("comment.approve")}
                                >
                                  <Check className="h-4 w-4" />
                                </Button>
                              )}
                              {comment.status !== "spam" && (
                                <Button
                                  size="sm"
                                  variant="ghost"
                                  className="h-7 w-7 p-0 text-amber-600 hover:text-amber-700 hover:bg-amber-100 dark:hover:bg-amber-900/30"
                                  onClick={() => handleReject(comment.id)}
                                  disabled={actionLoading === comment.id}
                                  title={t("comment.markSpam")}
                                >
                                  <AlertCircle className="h-4 w-4" />
                                </Button>
                              )}
                              <Button
                                size="sm"
                                variant="ghost"
                                className="h-7 w-7 p-0 text-red-600 hover:text-red-700 hover:bg-red-100 dark:hover:bg-red-900/30"
                                onClick={() => setDeleteConfirm(comment.id)}
                                disabled={actionLoading === comment.id}
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

              {/* Pagination */}
              {totalPages > 1 && (
                <div className="flex items-center justify-between mt-4 pt-4 border-t">
                  <p className="text-sm text-muted-foreground">
                    {t("comment.pageInfo", { current: page.toString(), total: totalPages.toString() })}
                  </p>
                  <div className="flex items-center gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setPage((p) => Math.max(1, p - 1))}
                      disabled={page <= 1}
                    >
                      <ChevronLeft className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() =>
                        setPage((p) => Math.min(totalPages, p + 1))
                      }
                      disabled={page >= totalPages}
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
