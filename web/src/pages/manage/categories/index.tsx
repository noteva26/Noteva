import { useDeferredValue, useEffect, useMemo, useOptimistic, useState, useTransition } from "react";
import { motion } from "motion/react";
import { categoriesApi, Category, CreateCategoryInput } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
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
import { Plus, Edit, Trash2, FolderTree, Search } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { EmptyState } from "@/components/ui/empty-state";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";

interface CategoriesPageProps {
  embedded?: boolean;
}

export default function CategoriesPage({ embedded = false }: CategoriesPageProps) {
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
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);
  const [deletingCategory, setDeletingCategory] = useState<Category | null>(null);

  const [form, setForm] = useState({
    name: "",
    slug: "",
    description: "",
    parent_id: null as number | null,
  });

  const categoryById = useMemo(
    () => new Map(categories.map((category) => [category.id, category])),
    [categories]
  );

  const availableParentCategories = useMemo(() => {
    if (!editingCategory) return categories;

    const descendantIds = new Set<number>();
    const collectDescendants = (parentId: number) => {
      categories
        .filter((category) => category.parent_id === parentId)
        .forEach((category) => {
          descendantIds.add(category.id);
          collectDescendants(category.id);
        });
    };
    collectDescendants(editingCategory.id);

    return categories.filter(
      (category) => category.id !== editingCategory.id && !descendantIds.has(category.id)
    );
  }, [categories, editingCategory]);

  const filteredCategories = useMemo(() => {
    const keyword = deferredSearch.trim().toLowerCase();
    if (!keyword) return optimisticCategories;

    return optimisticCategories.filter((category) =>
      [category.name, category.slug, category.description || ""]
        .some((value) => value.toLowerCase().includes(keyword))
    );
  }, [deferredSearch, optimisticCategories]);

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
    setForm({ name: "", slug: "", description: "", parent_id: null });
    setDialogOpen(true);
  };

  const openEditDialog = (category: Category) => {
    setEditingCategory(category);
    setForm({
      name: category.name,
      slug: category.slug,
      description: category.description || "",
      parent_id: category.parent_id ?? null,
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
        const payload = {
          name: form.name,
          slug: form.slug,
          description: form.description,
          parent_id: form.parent_id,
        };

        if (editingCategory) {
          const response = await categoriesApi.update(editingCategory.id, payload);
          setCategories((current) =>
            current.map((category) =>
              category.id === editingCategory.id ? response.data : category
            )
          );
          toast.success(t("category.updateSuccess"));
        } else {
          const response = await categoriesApi.create(payload as CreateCategoryInput);
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
    <div className={embedded ? "flex h-full min-h-0 flex-col gap-4" : "space-y-6"}>
      <div className="flex min-h-[44px] items-center justify-between gap-3">
        <div className="min-w-0">
          {embedded ? (
            <h2 className="text-xl font-semibold">{t("manage.categories")}</h2>
          ) : (
            <h1 className="text-3xl font-bold">{t("manage.categories")}</h1>
          )}
          <p className={embedded ? "text-sm text-muted-foreground" : "text-muted-foreground"}>
            {t("category.totalCategories")}
          </p>
        </div>
        <Button size={embedded ? "sm" : "default"} onClick={openCreateDialog}>
          <Plus className="h-4 w-4 mr-2" />
          {t("category.newCategory")}
        </Button>
      </div>
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />

      <div className={embedded ? "min-h-0 flex-1" : undefined}>
        <Card className={embedded ? "flex h-full min-h-[460px] flex-col" : undefined}>
          <CardContent className="flex min-h-0 flex-1 flex-col p-4">
            <div className="mb-3 flex items-center gap-3">
              <div className="relative min-w-0 flex-1">
                <Search className="absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  placeholder={t("common.search")}
                  value={search}
                  onChange={(event) => setSearch(event.target.value)}
                  className="h-9 pl-9"
                />
              </div>
              <span className="shrink-0 text-sm text-muted-foreground">
                {filteredCategories.length} / {optimisticCategories.length}
              </span>
            </div>
            <DataSyncBar active={isSyncing} label={t("common.loading")} className="mb-3" />
            {showInitialLoading ? (
              <div className="space-y-3">
                {Array.from({ length: 6 }).map((_, i) => (
                  <div key={i} className="h-11 w-full skeleton-shimmer rounded" />
                ))}
              </div>
            ) : filteredCategories.length === 0 ? (
              <EmptyState
                icon={FolderTree}
                title={t("category.noCategories")}
                actionText={optimisticCategories.length === 0 ? t("category.createFirst") : undefined}
                onAction={optimisticCategories.length === 0 ? openCreateDialog : undefined}
              />
            ) : (
              <div
                className={`min-h-0 flex-1 space-y-2 overflow-auto transition-opacity ${embedded ? "max-h-[420px]" : ""} ${isSyncing ? "opacity-70" : ""}`}
              >
                {filteredCategories.map((category, i) => {
                  const isDefault = category.slug === "uncategorized";
                  const parentName = category.parent_id
                    ? categoryById.get(category.parent_id)?.name
                    : null;
                  return (
                    <motion.div
                      key={category.id}
                      initial={{ opacity: 0, y: 8 }}
                      animate={{ opacity: 1, y: 0 }}
                      transition={{
                        type: "spring",
                        stiffness: 400,
                        damping: 30,
                        delay: i * 0.025,
                      }}
                      className="flex min-h-11 items-center gap-2 rounded-lg p-3 transition-colors hover:bg-muted/50"
                    >
                      <FolderTree className="h-4 w-4 text-muted-foreground" />
                      <div className="min-w-0 flex-1">
                        <div className="truncate font-medium">
                          {isDefault ? t("category.uncategorized") : category.name}
                        </div>
                        <div className="truncate text-xs text-muted-foreground">{category.slug}</div>
                      </div>
                      {parentName ? (
                        <span className="rounded-md bg-muted px-2 py-1 text-xs text-muted-foreground">
                          {parentName}
                        </span>
                      ) : null}
                      <div className="flex items-center gap-1">
                        <Button
                          variant="ghost"
                          size="icon"
                          className="h-8 w-8"
                          onClick={() => openEditDialog(category)}
                        >
                          <Edit className="h-4 w-4" />
                        </Button>
                        {!isDefault && (
                          <Button
                            variant="ghost"
                            size="icon"
                            className="h-8 w-8"
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
      </div>

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
              <Label htmlFor="parent_id">{t("category.parent")}</Label>
              <Select
                value={form.parent_id ? String(form.parent_id) : "none"}
                onValueChange={(value) =>
                  setForm((f) => ({
                    ...f,
                    parent_id: value === "none" ? null : Number(value),
                  }))
                }
              >
                <SelectTrigger id="parent_id">
                  <SelectValue placeholder={t("category.noParent")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">{t("category.noParent")}</SelectItem>
                  {availableParentCategories.map((category) => (
                    <SelectItem key={category.id} value={String(category.id)}>
                      {category.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
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
