import { useCallback, useEffect, useRef, useState, useTransition } from "react";
import { adminApi, type LoginLogEntry } from "@/lib/api";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Table, TableBody, TableCell, TableHead, TableHeader, TableRow } from "@/components/ui/table";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { toast } from "sonner";
import { Shield, Search, RefreshCw, CheckCircle2, XCircle, AlertTriangle, Loader2 } from "lucide-react";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";
import { getApiErrorMessage } from "@/lib/api-error";
import { formatDateTime } from "@/lib/format";

interface LoginLogFilters {
  username: string;
  ipAddress: string;
  success: string;
}

const EMPTY_FILTERS: LoginLogFilters = {
  username: "",
  ipAddress: "",
  success: "all",
};

export default function SecurityPage() {
  const { t, locale } = useTranslation();
  const mountedRef = useRef(false);
  const refreshInFlightRef = useRef(false);
  const refreshDoneTimerRef = useRef<number | null>(null);
  const [logs, setLogs] = useState<LoginLogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [total, setTotal] = useState(0);
  const [successCount, setSuccessCount] = useState(0);
  const [failedCount, setFailedCount] = useState(0);
  const [page, setPage] = useState(1);
  const [perPage] = useState(20);
  const [refreshKey, setRefreshKey] = useState(0);
  const [isRefreshPending, setIsRefreshPending] = useState(false);
  const [refreshDone, setRefreshDone] = useState(false);
  const [isFilterPending, startFilterTransition] = useTransition();

  // Filters
  const [filters, setFilters] = useState<LoginLogFilters>(EMPTY_FILTERS);
  const [appliedFilters, setAppliedFilters] = useState<LoginLogFilters>(EMPTY_FILTERS);

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      if (refreshDoneTimerRef.current) {
        window.clearTimeout(refreshDoneTimerRef.current);
      }
      mountedRef.current = false;
    };
  }, []);

  const markRefreshDone = useCallback(() => {
    if (refreshDoneTimerRef.current) {
      window.clearTimeout(refreshDoneTimerRef.current);
    }

    setRefreshDone(true);
    refreshDoneTimerRef.current = window.setTimeout(() => {
      setRefreshDone(false);
      refreshDoneTimerRef.current = null;
    }, 1500);
  }, []);

  const getLoginLogParams = useCallback(() => {
    const params: Parameters<typeof adminApi.getLoginLogs>[0] = { page, per_page: perPage };
    if (appliedFilters.username.trim()) params.username = appliedFilters.username.trim();
    if (appliedFilters.ipAddress.trim()) params.ip_address = appliedFilters.ipAddress.trim();
    if (appliedFilters.success !== "all") {
      params.success = appliedFilters.success === "success";
    }
    return params;
  }, [appliedFilters, page, perPage]);

  useEffect(() => {
    let active = true;

    const fetchLogs = async () => {
      setLoading(true);
      try {
        const { data } = await adminApi.getLoginLogs(getLoginLogParams());
        if (!active) return;

        setLogs(data.logs);
        setTotal(data.total);
        setSuccessCount(data.success_count);
        setFailedCount(data.failed_count);
      } catch (error) {
        if (active) {
          toast.error(getApiErrorMessage(error, t("error.loadFailed")));
        }
      } finally {
        if (active) {
          setLoading(false);
          setHasLoaded(true);
        }
      }
    };

    fetchLogs();
    return () => {
      active = false;
    };
  }, [getLoginLogParams, refreshKey, t]);

  const handleRefresh = useCallback(async () => {
    if (refreshInFlightRef.current) return;

    refreshInFlightRef.current = true;
    setIsRefreshPending(true);
    try {
      const { data } = await adminApi.getLoginLogs(getLoginLogParams());
      if (!mountedRef.current) return;

      setLogs(data.logs);
      setTotal(data.total);
      setSuccessCount(data.success_count);
      setFailedCount(data.failed_count);
      markRefreshDone();
    } catch (error) {
      if (mountedRef.current) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      }
    } finally {
      refreshInFlightRef.current = false;
      if (mountedRef.current) {
        setIsRefreshPending(false);
      }
    }
  }, [getLoginLogParams, markRefreshDone, t]);

  const handleSearch = () => {
    startFilterTransition(() => {
      setPage(1);
      setAppliedFilters({
        username: filters.username.trim(),
        ipAddress: filters.ipAddress.trim(),
        success: filters.success,
      });
      setRefreshKey((key) => key + 1);
    });
  };

  const handleReset = () => {
    startFilterTransition(() => {
      setFilters(EMPTY_FILTERS);
      setAppliedFilters(EMPTY_FILTERS);
      setPage(1);
      setRefreshKey((key) => key + 1);
    });
  };

  const formatDate = (dateStr: string) => {
    return formatDateTime(dateStr, locale, {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
  };

  const totalPages = Math.max(1, Math.ceil(total / perPage));
  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = (loading && hasLoaded) || isFilterPending;
  const isBusy = showInitialLoading || isFilterPending;

  return (
      <div className="space-y-6">
        <div className="flex items-center justify-between">
          <div>
            <h1 className="text-3xl font-bold flex items-center gap-2">
              <Shield className="h-8 w-8" />
              {t("security.title")}
            </h1>
            <p className="text-muted-foreground mt-1">
              {t("security.description")}
            </p>
          </div>
          <Button
            onClick={() => void handleRefresh()}
            variant="outline"
            size="sm"
            className="min-w-28"
            disabled={showInitialLoading || isSyncing || isRefreshPending}
            aria-busy={isRefreshPending}
          >
            {refreshDone ? (
              <CheckCircle2 className="h-4 w-4 mr-2 text-green-500 animate-in fade-in duration-300" />
            ) : (
              <RefreshCw className={cn("h-4 w-4 mr-2 transition-transform duration-500", isRefreshPending && "animate-spin")} />
            )}
            {refreshDone ? t("common.done") : t("common.refresh")}
          </Button>
        </div>
        <DataSyncBadge active={isSyncing} label={t("common.loading")} />

        {/* Filters */}
        <Card className="transition-all hover:shadow-md">
          <CardHeader>
            <CardTitle className="text-lg">{t("security.filters")}</CardTitle>
          </CardHeader>
          <CardContent>
            <div className="grid grid-cols-1 md:grid-cols-4 gap-4">
              <div className="space-y-2">
                <Label htmlFor="username">{t("security.username")}</Label>
                <Input
                  id="username"
                  placeholder={t("security.searchUsername")}
                  value={filters.username}
                  onChange={(e) => setFilters((current) => ({ ...current, username: e.target.value }))}
                  onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="ip">{t("security.ipAddress")}</Label>
                <Input
                  id="ip"
                  placeholder={t("security.searchIp")}
                  value={filters.ipAddress}
                  onChange={(e) => setFilters((current) => ({ ...current, ipAddress: e.target.value }))}
                  onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                />
              </div>

              <div className="space-y-2">
                <Label htmlFor="status">{t("security.status")}</Label>
                <Select value={filters.success} onValueChange={(success) => setFilters((current) => ({ ...current, success }))}>
                  <SelectTrigger id="status">
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="all">{t("common.all")}</SelectItem>
                    <SelectItem value="success">{t("security.success")}</SelectItem>
                    <SelectItem value="failed">{t("security.failed")}</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              <div className="space-y-2 flex items-end gap-2">
                <Button onClick={handleSearch} className="flex-1" disabled={isBusy}>
                  {isBusy ? (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  ) : (
                    <Search className="h-4 w-4 mr-2" />
                  )}
                  {t("common.search")}
                </Button>
                <Button onClick={handleReset} variant="outline" disabled={isBusy}>
                  {t("common.reset")}
                </Button>
              </div>
            </div>
          </CardContent>
        </Card>

        {/* Stats */}
        <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
          <Card className="transition-all hover:shadow-md">
            <CardHeader className="pb-3">
              <CardDescription>{t("security.totalRecords")}</CardDescription>
              <CardTitle className="text-3xl">{total}</CardTitle>
            </CardHeader>
          </Card>
          <Card className="transition-all hover:shadow-md">
            <CardHeader className="pb-3">
              <CardDescription>{t("security.successfulLogins")}</CardDescription>
              <CardTitle className="text-3xl text-green-600">
                {successCount}
              </CardTitle>
            </CardHeader>
          </Card>
          <Card className="transition-all hover:shadow-md">
            <CardHeader className="pb-3">
              <CardDescription>{t("security.failedAttempts")}</CardDescription>
              <CardTitle className="text-3xl text-red-600">
                {failedCount}
              </CardTitle>
            </CardHeader>
          </Card>
        </div>

        {/* Logs Table */}
        <Card className="transition-all hover:shadow-md">
          <CardHeader>
            <CardTitle>{t("security.loginRecords")}</CardTitle>
            <CardDescription>
              {t("security.recordsInfo", { total, page, totalPages })}
            </CardDescription>
          </CardHeader>
          <CardContent>
            <DataSyncBar active={isSyncing} label={t("common.loading")} className="mb-3" />
            {showInitialLoading ? (
              <div className="space-y-2">
                {[...Array(5)].map((_, i) => (
                  <Skeleton key={i} className="h-12 w-full" />
                ))}
              </div>
            ) : logs.length === 0 ? (
              <div className="text-center py-12 text-muted-foreground">
                <AlertTriangle className="h-12 w-12 mx-auto mb-4 opacity-50" />
                <p>{t("security.noRecords")}</p>
              </div>
            ) : (
              <>
                <div className="rounded-md border">
                  <Table className={cn(isSyncing && "opacity-70 transition-opacity")}>
                    <TableHeader>
                      <TableRow>
                        <TableHead>{t("security.status")}</TableHead>
                        <TableHead>{t("security.username")}</TableHead>
                        <TableHead>{t("security.ipAddress")}</TableHead>
                        <TableHead>{t("security.userAgent")}</TableHead>
                        <TableHead>{t("security.failureReason")}</TableHead>
                        <TableHead>{t("security.time")}</TableHead>
                      </TableRow>
                    </TableHeader>
                    <TableBody>
                      {logs.map((log) => (
                        <TableRow
                          key={log.id}
                          className="transition-colors hover:bg-muted/50"
                        >
                          <TableCell>
                            {log.success ? (
                              <Badge variant="default" className="bg-green-600">
                                <CheckCircle2 className="h-3 w-3 mr-1" />
                                {t("security.success")}
                              </Badge>
                            ) : (
                              <Badge variant="destructive">
                                <XCircle className="h-3 w-3 mr-1" />
                                {t("security.failed")}
                              </Badge>
                            )}
                          </TableCell>
                          <TableCell className="font-medium">{log.username}</TableCell>
                          <TableCell className="font-mono text-sm">
                            {log.ip_address || "-"}
                          </TableCell>
                          <TableCell className="max-w-xs truncate text-sm text-muted-foreground">
                            {log.user_agent || "-"}
                          </TableCell>
                          <TableCell className="text-sm">
                            {log.failure_reason || "-"}
                          </TableCell>
                          <TableCell className="text-sm text-muted-foreground">
                            {formatDate(log.created_at)}
                          </TableCell>
                        </TableRow>
                      ))}
                    </TableBody>
                  </Table>
                </div>

                {/* Pagination */}
                {totalPages > 1 && (
                  <div className="flex items-center justify-between mt-4">
                    <div className="text-sm text-muted-foreground">
                      {t("security.showing", {
                        from: (page - 1) * perPage + 1,
                        to: Math.min(page * perPage, total),
                        total
                      })}
                    </div>
                    <div className="flex gap-2">
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setPage(p => Math.max(1, p - 1))}
                      disabled={page === 1 || showInitialLoading}
                      >
                        {t("pagination.prev")}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                        disabled={page >= totalPages || showInitialLoading}
                      >
                        {t("pagination.next")}
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

