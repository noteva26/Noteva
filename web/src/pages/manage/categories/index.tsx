import { useEffect, useOptimistic, useState, useTransition } from "react";
import { motion } from "motion/react";
import { categoriesApi, Category, CreateCategoryInput } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Dialog,
  DialogContent,
  DialogHeader,
  DialogTitle,
  DialogFooter,
} from "@/components/ui/dialog";
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
import { Card, CardContent } from "@/components/ui/card";
import { Plus, Edit, Trash2, FolderTree } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { EmptyState } from "@/components/ui/empty-state";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";

export default function CategoriesPage() {
  const { t } = useTranslation();
  const [categories, setCategories] = useState<Category[]>([]);
  const [optimisticCategories, removeOptimisticCategories] = useOptimistic(
    categories,
    (currentCategories, categoryIds: Set<number>) =>
      currentCategories.filter((category) => !categoryIds.has(category.id))
  );
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [isRefreshing, startRefreshTransition] = useTransition();
  const [isMutating, startMutationTransition] = useTransition();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);
  const [deletingCategory, setDeletingCategory] = useState<Category | null>(null);

  const [form, setForm] = useState({
    name: "",
    slug: "",
    description: "",
  });

  useEffect(() => {
    let active = true;

    const fetchCategories = async () => {
      try {
        setLoading(true);
        const response = await categoriesApi.list();
        if (!active) return;

        const cats = response.data?.categories || [];
        setCategories(Array.isArray(cats) ? cats : []);
      } catch {
        if (!active) return;
        toast.error(t("error.loadFailed"));
        setCategories([]);
      } finally {
        if (active) {
          setLoading(false);
          setHasLoaded(true);
        }
      }
    };

    fetchCategories();
    return () => {
      active = false;
    };
  }, [refreshKey, t]);

  const refreshCategories = () => {
    startRefreshTransition(() => {
      setRefreshKey((key) => key + 1);
    });
  };

  const openCreateDialog = () => {
    setEditingCategory(null);
    setForm({ name: "", slug: "", description: "" });
    setDialogOpen(true);
  };

  const openEditDialog = (category: Category) => {
    setEditingCategory(category);
    setForm({
      name: category.name,
      slug: category.slug,
      description: category.description || "",
    });
    setDialogOpen(true);
  };

  const handleSubmit = () => {
    if (!form.name.trim()) {
      toast.error(t("category.name"));
      return;
    }

    startMutationTransition(async () => {
      try {
        if (editingCategory) {
          const response = await categoriesApi.update(editingCategory.id, form);
          setCategories((current) =>
            current.map((category) =>
              category.id === editingCategory.id ? response.data : category
            )
          );
          toast.success(t("category.updateSuccess"));
        } else {
          const response = await categoriesApi.create(form as CreateCategoryInput);
          setCategories((current) => [...current, response.data]);
          toast.success(t("category.createSuccess"));
        }
        setDialogOpen(false);
      } catch {
        toast.error(t("common.error"));
      }
    });
  };

  const handleDelete = () => {
    if (!deletingCategory) return;

    const category = deletingCategory;
    startMutationTransition(async () => {
      removeOptimisticCategories(new Set([category.id]));
      setDeleteDialogOpen(false);
      setDeletingCategory(null);

      try {
        await categoriesApi.delete(category.id);
        setCategories((current) => current.filter((item) => item.id !== category.id));
        toast.success(t("category.deleteSuccess"));
      } catch {
        toast.error(t("category.deleteFailed"));
        refreshCategories();
      }
    });
  };

  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = (loading && hasLoaded) || isRefreshing;

  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="flex items-center justify-between"
      >
        <div>
          <h1 className="text-3xl font-bold">{t("manage.categories")}</h1>
          <p className="text-muted-foreground">{t("category.totalCategories")}</p>
        </div>
        <motion.div whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}>
          <Button onClick={openCreateDialog}>
            <Plus className="h-4 w-4 mr-2" />
            {t("category.newCategory")}
          </Button>
        </motion.div>
      </motion.div>
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />

      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.1 }}
      >
        <Card>
          <CardContent className="p-4">
            <DataSyncBar active={isSyncing} label={t("common.loading")} className="mb-3" />
            {showInitialLoading ? (
              <div className="space-y-3">
                {Array.from({ length: 5 }).map((_, i) => (
                  <div key={i} className="h-12 w-full skeleton-shimmer rounded" />
                ))}
              </div>
            ) : optimisticCategories.length === 0 ? (
              <EmptyState
                icon={FolderTree}
                title={t("category.noCategories")}
                actionText={t("category.createFirst")}
                onAction={openCreateDialog}
              />
            ) : (
              <div className={`space-y-1 transition-opacity ${isSyncing ? "opacity-70" : ""}`}>
                {optimisticCategories.map((category, i) => {
                  const isDefault = category.slug === "uncategorized";
                  return (
                    <motion.div
                      key={category.id}
                      initial={{ opacity: 0, x: -10 }}
                      animate={{ opacity: 1, x: 0 }}
                      transition={{ delay: i * 0.03 }}
                      className="flex items-center gap-2 p-3 hover:bg-muted/50 rounded-lg transition-colors"
                    >
                      <FolderTree className="h-4 w-4 text-muted-foreground" />
                      <span className="flex-1 font-medium">
                        {isDefault ? t("category.uncategorized") : category.name}
                      </span>
                      <span className="text-sm text-muted-foreground">{category.slug}</span>
                      <div className="flex items-center gap-1">
                        <Button variant="ghost" size="icon" onClick={() => openEditDialog(category)}>
                          <Edit className="h-4 w-4" />
                        </Button>
                        {!isDefault && (
                          <Button
                            variant="ghost"
                            size="icon"
                            onClick={() => { setDeletingCategory(category); setDeleteDialogOpen(true); }}
                          >
                            <Trash2 className="h-4 w-4 text-destructive" />
                          </Button>
                        )}
                      </div>
                    </motion.div>
                  );
                })}
              </div>
            )}
          </CardContent>
        </Card>
      </motion.div>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>
              {editingCategory ? t("category.editCategory") : t("category.newCategory")}
            </DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="name">{t("category.name")}</Label>
              <Input
                id="name"
                value={form.name}
                onChange={(e) => setForm((f) => ({ ...f, name: e.target.value }))}
                placeholder={t("category.name")}
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="slug">{t("common.slug")}</Label>
              <Input
                id="slug"
                value={form.slug}
                onChange={(e) => setForm((f) => ({ ...f, slug: e.target.value }))}
                placeholder="url-friendly-slug"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="description">{t("category.description")}</Label>
              <Textarea
                id="description"
                value={form.description}
                onChange={(e) => setForm((f) => ({ ...f, description: e.target.value }))}
                placeholder={t("category.description")}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button onClick={handleSubmit} disabled={isMutating}>
              {editingCategory ? t("common.save") : t("common.create")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("category.confirmDelete", { name: deletingCategory?.name || "" })}
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
