import { useEffect, useState } from "react";
import { useTranslation } from "@/lib/i18n";
import { filesApi, type FileInfo, type StorageStatsResponse } from "@/lib/api";
import { toast } from "sonner";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
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
    Filter,
    RefreshCw,
} from "lucide-react";

export default function FilesPage() {
    const { t } = useTranslation();
    const [files, setFiles] = useState<FileInfo[]>([]);
    const [stats, setStats] = useState<StorageStatsResponse | null>(null);
    const [search, setSearch] = useState("");
    const [typeFilter, setTypeFilter] = useState<string>("");
    const [loading, setLoading] = useState(true);
    const [deleteTarget, setDeleteTarget] = useState<FileInfo | null>(null);
    const [selectedFiles, setSelectedFiles] = useState<Set<string>>(new Set());

    const fetchFiles = async () => {
        try {
            setLoading(true);
            const [filesRes, statsRes] = await Promise.all([
                filesApi.list({ search: search || undefined, file_type: typeFilter || undefined }),
                filesApi.stats(),
            ]);
            setFiles(filesRes.data.files);
            setStats(statsRes.data);
        } catch {
            toast.error("加载文件列表失败");
        } finally {
            setLoading(false);
        }
    };

    useEffect(() => {
        fetchFiles();
    }, [typeFilter]); // eslint-disable-line react-hooks/exhaustive-deps

    const handleSearch = (e: React.FormEvent) => {
        e.preventDefault();
        fetchFiles();
    };

    const handleDelete = async (file: FileInfo) => {
        try {
            await filesApi.delete(file.name);
            toast.success(`已删除 ${file.name}`);
            fetchFiles();
            setDeleteTarget(null);
            selectedFiles.delete(file.name);
            setSelectedFiles(new Set(selectedFiles));
        } catch {
            toast.error(`删除失败: ${file.name}`);
        }
    };

    const handleBatchDelete = async () => {
        let success = 0;
        for (const name of selectedFiles) {
            try {
                await filesApi.delete(name);
                success++;
            } catch { /* continue */ }
        }
        toast.success(`已删除 ${success} 个文件`);
        setSelectedFiles(new Set());
        fetchFiles();
    };

    const copyUrl = (url: string) => {
        navigator.clipboard.writeText(window.location.origin + url);
        toast.success("链接已复制");
    };

    const formatSize = (size: number) => {
        if (size < 1024) return `${size} B`;
        if (size < 1024 * 1024) return `${(size / 1024).toFixed(1)} KB`;
        return `${(size / 1024 / 1024).toFixed(1)} MB`;
    };

    const formatDate = (dateStr: string) => {
        if (!dateStr) return "-";
        const d = new Date(dateStr);
        return d.toLocaleDateString("zh-CN", { year: "numeric", month: "2-digit", day: "2-digit", hour: "2-digit", minute: "2-digit" });
    };

    const toggleSelect = (name: string) => {
        const next = new Set(selectedFiles);
        if (next.has(name)) next.delete(name);
        else next.add(name);
        setSelectedFiles(next);
    };

    const toggleSelectAll = () => {
        if (selectedFiles.size === files.length) {
            setSelectedFiles(new Set());
        } else {
            setSelectedFiles(new Set(files.map(f => f.name)));
        }
    };

    return (
        <div className="space-y-6">
            {/* Header */}
            <div className="flex items-center justify-between">
                <h1 className="text-2xl font-bold">空间管理</h1>
                <Button variant="outline" size="sm" onClick={fetchFiles} disabled={loading}>
                    <RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
                    刷新
                </Button>
            </div>

            {/* Stats Cards */}
            {stats && (
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
                    <div className="rounded-xl border bg-card p-4 flex items-center gap-3">
                        <div className="h-10 w-10 rounded-lg bg-primary/10 flex items-center justify-center">
                            <HardDrive className="h-5 w-5 text-primary" />
                        </div>
                        <div>
                            <p className="text-sm text-muted-foreground">存储用量</p>
                            <p className="text-lg font-semibold">{stats.total_size_display}</p>
                        </div>
                    </div>
                    <div className="rounded-xl border bg-card p-4 flex items-center gap-3">
                        <div className="h-10 w-10 rounded-lg bg-blue-500/10 flex items-center justify-center">
                            <Image className="h-5 w-5 text-blue-500" />
                        </div>
                        <div>
                            <p className="text-sm text-muted-foreground">图片文件</p>
                            <p className="text-lg font-semibold">{stats.image_count}</p>
                        </div>
                    </div>
                    <div className="rounded-xl border bg-card p-4 flex items-center gap-3">
                        <div className="h-10 w-10 rounded-lg bg-emerald-500/10 flex items-center justify-center">
                            <FileText className="h-5 w-5 text-emerald-500" />
                        </div>
                        <div>
                            <p className="text-sm text-muted-foreground">其他文件</p>
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
                            placeholder="搜索文件名..."
                            value={search}
                            onChange={(e) => setSearch(e.target.value)}
                            className="pl-9"
                        />
                    </div>
                    <Button type="submit" variant="secondary">搜索</Button>
                </form>
                <div className="flex gap-2">
                    <Button
                        variant={typeFilter === "" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setTypeFilter("")}
                    >
                        全部
                    </Button>
                    <Button
                        variant={typeFilter === "image" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setTypeFilter("image")}
                    >
                        <Image className="h-3.5 w-3.5 mr-1" />
                        图片
                    </Button>
                    <Button
                        variant={typeFilter === "file" ? "default" : "outline"}
                        size="sm"
                        onClick={() => setTypeFilter("file")}
                    >
                        <FileText className="h-3.5 w-3.5 mr-1" />
                        文件
                    </Button>
                </div>
            </div>

            {/* Batch actions */}
            {selectedFiles.size > 0 && (
                <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50 border">
                    <span className="text-sm text-muted-foreground">已选 {selectedFiles.size} 项</span>
                    <Button variant="destructive" size="sm" onClick={handleBatchDelete}>
                        <Trash2 className="h-3.5 w-3.5 mr-1" />
                        批量删除
                    </Button>
                    <Button variant="ghost" size="sm" onClick={() => setSelectedFiles(new Set())}>
                        取消选择
                    </Button>
                </div>
            )}

            {/* File List */}
            {loading ? (
                <div className="flex justify-center py-12">
                    <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
                </div>
            ) : files.length === 0 ? (
                <div className="text-center text-muted-foreground py-12 border rounded-lg bg-muted/20">
                    暂无文件
                </div>
            ) : (
                <div className="border rounded-lg overflow-hidden">
                    <table className="w-full">
                        <thead className="bg-muted/50">
                            <tr>
                                <th className="w-10 p-3">
                                    <input
                                        type="checkbox"
                                        checked={selectedFiles.size === files.length && files.length > 0}
                                        onChange={toggleSelectAll}
                                        className="rounded"
                                    />
                                </th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground">预览</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground">文件名</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground hidden sm:table-cell">大小</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground hidden md:table-cell">类型</th>
                                <th className="text-left p-3 text-sm font-medium text-muted-foreground hidden lg:table-cell">上传时间</th>
                                <th className="text-right p-3 text-sm font-medium text-muted-foreground">操作</th>
                            </tr>
                        </thead>
                        <tbody className="divide-y">
                            {files.map((file) => (
                                <tr key={file.name} className="hover:bg-muted/30 transition-colors">
                                    <td className="p-3">
                                        <input
                                            type="checkbox"
                                            checked={selectedFiles.has(file.name)}
                                            onChange={() => toggleSelect(file.name)}
                                            className="rounded"
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
                                        {formatSize(file.size)}
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
                                                title="复制链接"
                                            >
                                                <Copy className="h-3.5 w-3.5" />
                                            </Button>
                                            <Button
                                                variant="ghost"
                                                size="icon"
                                                className="h-8 w-8"
                                                asChild
                                                title="新窗口打开"
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
                                                title="删除"
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
                        <AlertDialogTitle>确认删除</AlertDialogTitle>
                        <AlertDialogDescription>
                            确定要删除文件 <strong>{deleteTarget?.name}</strong> 吗？此操作不可撤销。
                            {deleteTarget?.is_image && "引用此图片的文章将无法显示该图片。"}
                        </AlertDialogDescription>
                    </AlertDialogHeader>
                    <AlertDialogFooter>
                        <AlertDialogCancel>取消</AlertDialogCancel>
                        <AlertDialogAction
                            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
                            onClick={() => deleteTarget && handleDelete(deleteTarget)}
                        >
                            删除
                        </AlertDialogAction>
                    </AlertDialogFooter>
                </AlertDialogContent>
            </AlertDialog>
        </div>
    );
}
