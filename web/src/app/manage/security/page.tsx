"use client";

import { useEffect, useState } from "react";
import { adminApi, LoginLogEntry } from "@/lib/api";
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

// Add CSS animation
const style = `
  @keyframes fadeIn {
    from {
      opacity: 0;
      transform: translateY(10px);
    }
    to {
      opacity: 1;
      transform: translateY(0);
    }
  }
`;

export default function SecurityPage() {
  const { t } = useTranslation();
  const [logs, setLogs] = useState<LoginLogEntry[]>([]);
  const [loading, setLoading] = useState(true);
  const [total, setTotal] = useState(0);
  const [successCount, setSuccessCount] = useState(0);
  const [failedCount, setFailedCount] = useState(0);
  const [page, setPage] = useState(1);
  const [perPage] = useState(20);
  
  // Filters
  const [usernameFilter, setUsernameFilter] = useState("");
  const [ipFilter, setIpFilter] = useState("");
  const [successFilter, setSuccessFilter] = useState<string>("all");

  const fetchLogs = async () => {
    setLoading(true);
    try {
      const params: any = { page, per_page: perPage };
      if (usernameFilter.trim()) params.username = usernameFilter.trim();
      if (ipFilter.trim()) params.ip_address = ipFilter.trim();
      if (successFilter !== "all") {
        params.success = successFilter === "success";
      }
      
      const { data } = await adminApi.getLoginLogs(params);
      setLogs(data.logs);
      setTotal(data.total);
      setSuccessCount(data.success_count);
      setFailedCount(data.failed_count);
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchLogs();
  }, [page]);

  const handleSearch = () => {
    setPage(1);
    fetchLogs();
  };

  const handleReset = () => {
    setUsernameFilter("");
    setIpFilter("");
    setSuccessFilter("all");
    setPage(1);
    setTimeout(fetchLogs, 0);
  };

  const formatDate = (dateStr: string) => {
    try {
      const date = new Date(dateStr);
      return date.toLocaleString("zh-CN", {
        year: "numeric",
        month: "2-digit",
        day: "2-digit",
        hour: "2-digit",
        minute: "2-digit",
        second: "2-digit",
      });
    } catch {
      return dateStr;
    }
  };

  const totalPages = Math.ceil(total / perPage);

  return (
    <>
      <style>{style}</style>
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
          <Button onClick={fetchLogs} variant="outline" size="sm" disabled={loading}>
            <RefreshCw className={cn("h-4 w-4 mr-2", loading && "animate-spin")} />
            {t("common.refresh")}
          </Button>
        </div>

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
                  value={usernameFilter}
                  onChange={(e) => setUsernameFilter(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                />
              </div>
              
              <div className="space-y-2">
                <Label htmlFor="ip">{t("security.ipAddress")}</Label>
                <Input
                  id="ip"
                  placeholder={t("security.searchIp")}
                  value={ipFilter}
                  onChange={(e) => setIpFilter(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && handleSearch()}
                />
              </div>
              
              <div className="space-y-2">
                <Label htmlFor="status">{t("security.status")}</Label>
                <Select value={successFilter} onValueChange={setSuccessFilter}>
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
                <Button onClick={handleSearch} className="flex-1" disabled={loading}>
                  {loading ? (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  ) : (
                    <Search className="h-4 w-4 mr-2" />
                  )}
                  {t("common.search")}
                </Button>
                <Button onClick={handleReset} variant="outline" disabled={loading}>
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
            {loading ? (
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
                  <Table>
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
                      {logs.map((log, index) => (
                        <TableRow 
                          key={log.id}
                          className="transition-colors hover:bg-muted/50"
                          style={{
                            animation: `fadeIn 0.3s ease-in-out ${index * 0.05}s both`
                          }}
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
                        disabled={page === 1 || loading}
                      >
                        {t("pagination.prev")}
                      </Button>
                      <Button
                        variant="outline"
                        size="sm"
                        onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                        disabled={page >= totalPages || loading}
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
    </>
  );
}
