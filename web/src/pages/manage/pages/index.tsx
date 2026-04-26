import { useEffect, useOptimistic, useState, useTransition } from "react";
import { useTranslation } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Label } from "@/components/ui/label";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
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
import { Badge } from "@/components/ui/badge";
import { Plus, Pencil, Trash2, RefreshCw } from "lucide-react";
import { toast } from "sonner";
import { api } from "@/lib/api";
import { Card, CardContent } from "@/components/ui/card";
import { AdminPageHeader } from "@/components/admin/page-header";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";
import { getApiErrorMessage } from "@/lib/api-error";
import { formatDateTime } from "@/lib/format";

interface Page {
  id: number;
  slug: string;
  title: string;
  content: string;
  content_html: string;
  status: string;
  created_at: string;
  updated_at: string;
}

export default function PagesManagePage() {
  const { t, locale } = useTranslation();
  const [pages, setPages] = useState<Page[]>([]);
  const [optimisticPages, removeOptimisticPages] = useOptimistic(
    pages,
    (currentPages, pageIds: Set<number>) =>
      currentPages.filter((page) => !pageIds.has(page.id))
  );
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [isRefreshing, startRefreshTransition] = useTransition();
  const [isMutating, startMutationTransition] = useTransition();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [editingPage, setEditingPage] = useState<Page | null>(null);
  const [deletingPage, setDeletingPage] = useState<Page | null>(null);
  const [formData, setFormData] = useState({
    slug: "",
    title: "",
    content: "",
    status: "draft",
  });

  useEffect(() => {
    let active = true;

    const loadPages = async () => {
      try {
        setLoading(true);
        const res = await api.get<{ pages: Page[] }>("/admin/pages");
        if (!active) return;

        setPages(res.data.pages || []);
      } catch {
        if (active) toast.error(t("page.loadFailed"));
      } finally {
        if (active) {
          setLoading(false);
          setHasLoaded(true);
        }
      }
    };

    loadPages();
    return () => {
      active = false;
    };
  }, [refreshKey, t]);

  const openCreateDialog = () => {
    setEditingPage(null);
    setFormData({ slug: "", title: "", content: "", status: "draft" });
    setDialogOpen(true);
  };

  const openEditDialog = (page: Page) => {
    setEditingPage(page);
    setFormData({
      slug: page.slug,
      title: page.title,
      content: page.content,
      status: page.status,
    });
    setDialogOpen(true);
  };

  const handleSubmit = () => {
    if (!formData.slug.trim() || !formData.title.trim()) {
      toast.error(t("page.fillRequired"));
      return;
    }

    startMutationTransition(async () => {
      try {
        if (editingPage) {
          const res = await api.put<{ page: Page }>(`/admin/pages/${editingPage.id}`, formData);
          setPages((current) =>
            current.map((page) => page.id === editingPage.id ? res.data.page : page)
          );
          toast.success(t("page.updateSuccess"));
        } else {
          const res = await api.post<{ page: Page }>("/admin/pages", formData);
          setPages((current) => [...current, res.data.page]);
          toast.success(t("page.createSuccess"));
        }
        setDialogOpen(false);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("page.saveFailed")));
      }
    });
  };

  const handleDelete = () => {
    if (!deletingPage) return;

    const page = deletingPage;
    startMutationTransition(async () => {
      removeOptimisticPages(new Set([page.id]));
      setDeleteDialogOpen(false);
      setDeletingPage(null);

      try {
        await api.delete(`/admin/pages/${page.id}`);
        setPages((current) => current.filter((item) => item.id !== page.id));
        toast.success(t("page.deleteSuccess"));
      } catch {
        toast.error(t("page.deleteFailed"));
        setRefreshKey((key) => key + 1);
      }
    });
  };

  const refreshPages = () => {
    startRefreshTransition(() => setRefreshKey((key) => key + 1));
  };

  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = (loading && hasLoaded) || isRefreshing;

  return (
    <div className="space-y-6">
      <AdminPageHeader
        title={t("page.title")}
        actions={
          <>
            <Button variant="outline" onClick={refreshPages} disabled={isSyncing}>
              <RefreshCw className={`h-4 w-4 mr-2 ${isSyncing ? "animate-spin" : ""}`} />
              {t("common.refresh")}
            </Button>
            <Button onClick={openCreateDialog}>
              <Plus className="h-4 w-4 mr-2" />
              {t("page.newPage")}
            </Button>
          </>
        }
      />
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />

      <Card>
        <CardContent className="p-0">
          <DataSyncBar active={isSyncing} label={t("common.loading")} className="mx-4 mt-4" />
          {showInitialLoading ? (
            <div className="p-6 text-sm text-muted-foreground">{t("common.loading")}</div>
          ) : (
            <Table className={isSyncing ? "opacity-70 transition-opacity" : undefined}>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("page.pageTitle")}</TableHead>
                  <TableHead>{t("page.slug")}</TableHead>
                  <TableHead>{t("page.status")}</TableHead>
                  <TableHead>{t("page.updatedAt")}</TableHead>
                  <TableHead className="text-right">{t("common.edit")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {optimisticPages.map((page) => (
                  <TableRow key={page.id}>
                    <TableCell className="font-medium">{page.title}</TableCell>
                    <TableCell>
                      <code className="text-sm bg-muted px-1 rounded">/{page.slug}</code>
                    </TableCell>
                    <TableCell>
                      <Badge variant={page.status === "published" ? "default" : "secondary"}>
                        {page.status === "published" ? t("page.published") : t("page.draft")}
                      </Badge>
                    </TableCell>
                    <TableCell>{formatDateTime(page.updated_at, locale)}</TableCell>
                    <TableCell className="text-right">
                      <Button variant="ghost" size="icon" onClick={() => openEditDialog(page)}>
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => {
                          setDeletingPage(page);
                          setDeleteDialogOpen(true);
                        }}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
                {optimisticPages.length === 0 && (
                  <TableRow>
                    <TableCell colSpan={5} className="h-32 text-center text-muted-foreground">
                      {t("page.noPages")}
                    </TableCell>
                  </TableRow>
                )}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="max-w-2xl">
          <DialogHeader>
            <DialogTitle>{editingPage ? t("page.editPage") : t("page.newPage")}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label>{t("page.pageTitle")}</Label>
                <Input
                  value={formData.title}
                  onChange={(e) => setFormData({ ...formData, title: e.target.value })}
                  placeholder={t("page.pageTitle")}
                />
              </div>
              <div className="space-y-2">
                <Label>{t("page.slug")}</Label>
                <Input
                  value={formData.slug}
                  onChange={(e) => setFormData({ ...formData, slug: e.target.value })}
                  placeholder="about"
                />
              </div>
            </div>
            <div className="space-y-2">
              <Label>{t("page.content")} (Markdown)</Label>
              <Textarea
                value={formData.content}
                onChange={(e) => setFormData({ ...formData, content: e.target.value })}
                placeholder={t("page.content")}
                rows={10}
              />
            </div>
            <div className="space-y-2">
              <Label>{t("page.status")}</Label>
              <Select
                value={formData.status}
                onValueChange={(v) => setFormData({ ...formData, status: v })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="draft">{t("page.draft")}</SelectItem>
                  <SelectItem value="published">{t("page.published")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button onClick={handleSubmit} disabled={isMutating}>{t("common.save")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("page.confirmDelete", { title: deletingPage?.title || "" })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={handleDelete} disabled={isMutating}>{t("common.delete")}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

