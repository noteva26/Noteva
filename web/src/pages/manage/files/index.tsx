import { useEffect, useOptimistic, useState, useTransition } from "react";
import { useTranslation } from "@/lib/i18n";
import { filesApi, type FileInfo, type StorageStatsResponse } from "@/lib/api";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Checkbox } from "@/components/ui/checkbox";
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
    Search,
    Trash2,
    Image,
    FileText,
    HardDrive,
    Copy,
    ExternalLink,
    RefreshCw,
} from "lucide-react";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";
import { ConfirmDialog } from "@/components/admin/confirm-dialog";
import { formatDateTime, formatFileSize } from "@/lib/format";

export default function FilesPage() {
    const { t, locale } = useTranslation();
    const [files, setFiles] = useState<FileInfo[]>([]);
    const [optimisticFiles, removeOptimisticFiles] = useOptimistic(
        files,
        (currentFiles, namesToRemove: Set<string>) =>
            currentFiles.filter((file) => !namesToRemove.has(file.name))
    );
    const [stats, setStats] = useState<StorageStatsResponse | null>(null);
    const [search, setSearch] = useState("");
    const [appliedSearch, setAppliedSearch] = useState("");
    const [typeFilter, setTypeFilter] = useState<string>("");
    const [loading, setLoading] = useState(true);
    const [hasLoaded, setHasLoaded] = useState(false);
    const [refreshKey, setRefreshKey] = useState(0);
    const [deleteTarget, setDeleteTarget] = useState<FileInfo | null>(null);
    const [batchDeleteOpen, setBatchDeleteOpen] = useState(false);
    const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());
    const [isRefreshing, startRefreshTransition] = useTransition();
    const [isDeleting, startDeleteTransition] = useTransition();

    useEffect(() => {
        let active = true;

        const fetchFiles = async () => {
            try {
                setLoading(true);
                const [filesRes, statsRes] = await Promise.all([
                    filesApi.list({ search: appliedSearch || undefined, file_type: typeFilter || undefined }),
                    filesApi.stats(),
                ]);
                if (!active) return;

                setFiles(filesRes.data.files);
                setStats(statsRes.data);
                setSelectedFiles((current) => {
                    const loadedNames = new Set(filesRes.data.files.map((file) => file.name));
                    return new Set(Array.from(current).filter((name) => loadedNames.has(name)));
                });
            } catch {
                if (active) toast.error(t("fileManage.loadFailed"));
            } finally {
                if (active) {
                    setLoading(false);
                    setHasLoaded(true);
                }
            }
        };

        fetchFiles();
        return () => {
            active = false;
        };
    }, [appliedSearch, refreshKey, t, typeFilter]);

    const handleSearch = (e: React.FormEvent) => {
        e.preventDefault();
        startRefreshTransition(() => {
            setAppliedSearch(search.trim());
            setRefreshKey((key) => key + 1);
        });
    };

    const handleDelete = async (file: FileInfo) => {
        startDeleteTransition(async () => {
            removeOptimisticFiles(new Set([file.name]));
            setDeleteTarget(null);
            setSelectedFiles((current) => {
                const next = new Set(current);
                next.delete(file.name);
                return next;
            });

            try {
                await filesApi.delete(file.name);
                toast.success(t("fileManage.deleteSuccess", { name: file.name }));
                setRefreshKey((key) => key + 1);
            } catch {
                setRefreshKey((key) => key + 1);
                toast.error(t("fileManage.deleteFailed", { name: file.name }));
            }
        });
    };

    const handleBatchDelete = async () => {
        if (selectedFiles.size === 0) return;
        setBatchDeleteOpen(true);
    };

    const confirmBatchDelete = async () => {
        const names = new Set(selectedFiles);

        startDeleteTransition(async () => {
            setBatchDeleteOpen(false);
            removeOptimisticFiles(names);
            setSelectedFiles(new Set());

            let success = 0;
            const failed: string[] = [];
            for (const name of names) {
                try {
                    await filesApi.delete(name);
                    success++;
                } catch {
                    failed.push(name);
                }
            }
            if (failed.length === 0) {
                toast.success(t("fileManage.batchDeleteSuccess", { count: success.toString() }));
            } else {
                const failedNames = failed.slice(0, 3).join(", ");
                toast.error(
                    t(success > 0 ? "fileManage.batchDeletePartial" : "fileManage.batchDeleteFailed", {
                        success: success.toString(),
                        failed: failed.length.toString(),
                        names: failedNames,
                    })
                );
            }
            setRefreshKey((key) => key + 1);
        });
    };

    const copyUrl = (url: string) => {
        navigator.clipboard.writeText(window.location.origin + url);
        toast.success(t("fileManage.linkCopied"));
    };

    const formatDate = (dateStr: string) => {
        return formatDateTime(dateStr, locale, { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
    };

    const toggleSelect = (name: string) => {
        setSelectedFiles((current) => {
            const next = new Set(current);
            if (next.has(name)) next.delete(name);
            else next.add(name);
            return next;
        });
    };

    const toggleSelectAll = () => {
        if (selectedFiles.size === optimisticFiles.length) {
            setSelectedFiles(new Set());
        } else {
            setSelectedFiles(new Set(optimisticFiles.map((file) => file.name)));
        }
    };

    const showInitialLoading = loading && !hasLoaded;
    const isSyncing = (loading && hasLoaded) || isRefreshing;
    const isBusy = showInitialLoading || isRefreshing;
    const hasFiles = optimisticFiles.length > 0;
    const allSelected = selectedFiles.size === optimisticFiles.length && optimisticFiles.length > 0;

    return (
        <div className="space-y-6">
            {/* Header */}
            <div className="flex items-center justify-between">
                <h1 className="text-2xl font-bold">{t("fileManage.title")}</h1>
                <Button variant="outline" size="sm" onClick={() => setRefreshKey((key) => key + 1)} disabled={isBusy}>
                    <RefreshCw className={`h-4 w-4 mr-2 ${isSyncing ? "animate-spin" : ""}`} />
                    {t("fileManage.refresh")}
                </Button>
            </div>
            <DataSyncBadge active={isSyncing} label={t("common.loading")} />

            {/* Stats Cards */}
            {stats && (
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                    <div className="rounded-xl border bg-card p-4 flex items-center gap-3">
                        <div className="h-10 w-10 rounded-lg bg-primary/10 flex items-center justify-center">
                            <HardDrive className="h-5 w-5 text-primary" />
                        </div>
                        <div>
                            <p className="text-sm text-muted-foreground">{t("fileManage.storageUsage")}</p>
                            <p className="text-lg font-semibold">{stats.total_size_display}</p>
                        </div>
                    </div>
                    <div className="rounded-xl border bg-card p-4 flex items-center gap-3">
                        <div className="h-10 w-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
                            <Image className="h-5 w-5 text-blue-500" />
                        </div>
                        <div>
                            <p className="text-sm text-muted-foreground">{t("fileManage.imageFiles")}</p>
                            <p className="text-lg font-semibold">{stats.image_count}</p>
                        </div>
                    </div>
                    <div className="rounded-xl border bg-card p-4 flex items-center gap-3">
                        <div className="h-10 w-10 rounded-lg bg-emerald-500/10 flex items-center justify-center">
                            <FileText className="h-5 w-5 text-emerald-500" />
                        </div>
                        <div>
                            <p className="text-sm text-muted-foreground">{t("fileManage.otherFiles")}</p>
                            <p className="text-lg font-semibold">{stats.other_count}</p>
                        </div>
                    </div>
                </div>
            )}

            {/* Search & Filter */}
            <div className="flex flex-col sm:flex-row gap-3">
                <form onSubmit={handleSearch} className="flex-1 flex gap-2">
                    <div className="relative flex-1">
                        <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                        <Input
                            placeholder={t("fileManage.searchPlaceholder")}
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            className="pl-9"
                        />
                    </div>
                    <Button type="submit" variant="secondary">{t("fileManage.search")}</Button>
                </form>
                <div className="flex gap-2">
                    <Button
                        variant={typeFilter === "" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setTypeFilter("")}
                    >
                        {t("fileManage.all")}
                    </Button>
                    <Button
                        variant={typeFilter === "image" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setTypeFilter("image")}
                    >
                        <Image className="h-3.5 w-3.5 mr-1" />
                        {t("fileManage.images")}
                    </Button>
                    <Button
                        variant={typeFilter === "file" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setTypeFilter("file")}
                    >
                        <FileText className="h-3.5 w-3.5 mr-1" />
                        {t("fileManage.files")}
                    </Button>
                </div>
            </div>

            {/* Batch actions */}
            {selectedFiles.size > 0 && (
                <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50 border">
                    <span className="text-sm text-muted-foreground">{t("fileManage.selectedCount", { count: selectedFiles.size.toString() })}</span>
                    <Button variant="destructive" size="sm" onClick={handleBatchDelete}>
                        <Trash2 className="h-3.5 w-3.5 mr-1" />
                        {t("fileManage.batchDelete")}
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => setSelectedFiles(new Set())}>
                        {t("fileManage.cancelSelect")}
                    </Button>
                </div>
            )}

            {/* File List */}
            <DataSyncBar active={isSyncing} label={t("common.loading")} />
            {showInitialLoading ? (
                <div className="flex justify-center py-12">
                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
                </div>
            ) : !hasFiles ? (
                <div className="text-center text-muted-foreground py-12 border rounded-lg bg-muted/20">
                    {t("fileManage.noFiles")}
                </div>
            ) : (
                <div className={`border rounded-lg overflow-hidden transition-opacity ${isSyncing ? "opacity-70" : ""}`}>
                    <table className="w-full">
                        <thead className="bg-muted/50">
                            <tr>
                                <th className="w-10 p-3">
                                    <Checkbox
                                        checked={allSelected}
                                        onCheckedChange={toggleSelectAll}
                                        aria-label={t("fileManage.selectedCount", { count: selectedFiles.size.toString() })}
                                    />
                                </th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground">{t("fileManage.preview")}</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground">{t("fileManage.fileName")}</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground hidden sm:table-cell">{t("fileManage.fileSize")}</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground hidden md:table-cell">{t("fileManage.fileType")}</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground hidden lg:table-cell">{t("fileManage.uploadTime")}</th>
                                <th className="text-right p-3 text-sm font-medium text-muted-foreground">{t("fileManage.actions")}</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y">
                            {optimisticFiles.map((file) => (
                                <tr key={file.name} className="hover:bg-muted/30 transition-colors">
                                    <td className="p-3">
                                        <Checkbox
                                            checked={selectedFiles.has(file.name)}
                                            onCheckedChange={() => toggleSelect(file.name)}
                                            aria-label={file.name}
                                        />
                                    </td>
                                    <td className="p-3 w-16">
                                        {file.is_image ? (
                                            <img
                                                src={file.url}
                                                alt={file.name}
                                                className="w-10 h-10 rounded object-cover border"
                                                loading="lazy"
                                            />
                                        ) : (
                                            <div className="w-10 h-10 rounded bg-muted flex items-center justify-center">
                                                <FileText className="h-5 w-5 text-muted-foreground" />
                                            </div>
                                        )}
                                    </td>
                                    <td className="p-3">
                                        <span className="text-sm font-medium truncate block max-w-[200px]" title={file.name}>
                                            {file.name}
                                        </span>
                                    </td>
                                    <td className="p-3 text-sm text-muted-foreground hidden sm:table-cell">
                                        {formatFileSize(file.size)}
                                    </td>
                                    <td className="p-3 hidden md:table-cell">
                                        <span className="text-xs px-2 py-0.5 rounded-full bg-muted">
                                            {file.file_type}
                                        </span>
                                    </td>
                                    <td className="p-3 text-sm text-muted-foreground hidden lg:table-cell">
                                        {formatDate(file.created_at)}
                                    </td>
                                    <td className="p-3">
                                        <div className="flex items-center justify-end gap-1">
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                className="h-8 w-8"
                                                onClick={() => copyUrl(file.url)}
                                                title={t("fileManage.copyLink")}
                                            >
                                                <Copy className="h-3.5 w-3.5" />
                                            </Button>
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                className="h-8 w-8"
                                                asChild
                                                title={t("fileManage.openInNewTab")}
                                            >
                                                <a href={file.url} target="_blank" rel="noopener noreferrer">
                                                    <ExternalLink className="h-3.5 w-3.5" />
                                                </a>
                                            </Button>
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                className="h-8 w-8 text-destructive hover:text-destructive"
                                                onClick={() => setDeleteTarget(file)}
                                                title={t("fileManage.delete")}
                                            >
                                                <Trash2 className="h-3.5 w-3.5" />
                                            </Button>
                                        </div>
                                    </td>
                                </tr>
                            ))}
                        </tbody>
                    </table>
                </div>
            )}

            {/* Delete confirmation */}
            <AlertDialog open={!!deleteTarget} onOpenChange={(open) => !open && setDeleteTarget(null)}>
                <AlertDialogContent>
                    <AlertDialogHeader>
                        <AlertDialogTitle>{t("fileManage.confirmDelete")}</AlertDialogTitle>
                        <AlertDialogDescription>
                            {t("fileManage.confirmDeleteDesc", { name: deleteTarget?.name || "" })}
                            {deleteTarget?.is_image && t("fileManage.imageWarning")}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
                        <AlertDialogAction
                            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                            disabled={isDeleting}
                            onClick={() => deleteTarget && handleDelete(deleteTarget)}
                        >
                            {t("fileManage.delete")}
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
            <ConfirmDialog
                open={batchDeleteOpen}
                title={t("fileManage.batchDelete")}
                description={t("fileManage.confirmBatchDelete", { count: selectedFiles.size.toString() })}
                confirmLabel={t("common.delete")}
                cancelLabel={t("common.cancel")}
                destructive
                loading={isDeleting}
                onOpenChange={setBatchDeleteOpen}
                onConfirm={confirmBatchDelete}
            />
        </div>
    );
}
