import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
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
import { Plus, Pencil, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { api } from "@/lib/api";

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
  const { t } = useTranslation();
  const [pages, setPages] = useState<Page[]>([]);
  const [loading, setLoading] = useState(true);
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
    loadPages();
  }, []);

  const loadPages = async () => {
    try {
      const res = await api.get("/admin/pages");
      setPages(res.data.pages || []);
    } catch (err) {
      toast.error(t("page.loadFailed"));
    } finally {
      setLoading(false);
    }
  };

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

  const handleSubmit = async () => {
    if (!formData.slug.trim() || !formData.title.trim()) {
      toast.error(t("page.fillRequired"));
      return;
    }

    try {
      if (editingPage) {
        await api.put(`/admin/pages/${editingPage.id}`, formData);
        toast.success(t("page.updateSuccess"));
      } else {
        await api.post("/admin/pages", formData);
        toast.success(t("page.createSuccess"));
      }
      setDialogOpen(false);
      loadPages();
    } catch (err: any) {
      toast.error(err.response?.data?.error?.message || t("page.saveFailed"));
    }
  };

  const handleDelete = async () => {
    if (!deletingPage) return;
    try {
      await api.delete(`/admin/pages/${deletingPage.id}`);
      toast.success(t("page.deleteSuccess"));
      setDeleteDialogOpen(false);
      loadPages();
    } catch (err) {
      toast.error(t("page.deleteFailed"));
    }
  };

  if (loading) {
    return <div className="p-6">{t("common.loading")}</div>;
  }

  return (
    <div className="p-6">
      <div className="flex justify-between items-center mb-6">
        <h1 className="text-2xl font-bold">{t("page.title")}</h1>
        <Button onClick={openCreateDialog}>
          <Plus className="h-4 w-4 mr-2" />
          {t("page.newPage")}
        </Button>
      </div>

      <Table>
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
          {pages.map((page) => (
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
              <TableCell>{new Date(page.updated_at).toLocaleString()}</TableCell>
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
          {pages.length === 0 && (
            <TableRow>
              <TableCell colSpan={5} className="text-center text-muted-foreground">
                {t("page.noPages")}
              </TableCell>
            </TableRow>
          )}
        </TableBody>
      </Table>

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
            <Button onClick={handleSubmit}>{t("common.save")}</Button>
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
            <AlertDialogAction onClick={handleDelete}>{t("common.delete")}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

