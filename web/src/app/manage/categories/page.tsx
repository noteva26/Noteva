"use client";

import { useEffect, useState } from "react";
import { categoriesApi, Category, CreateCategoryInput } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Skeleton } from "@/components/ui/skeleton";
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
import { Plus, Edit, Trash2, FolderTree, ChevronRight } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useTranslation } from "@/lib/i18n";

interface CategoryWithChildren extends Category {
  children?: CategoryWithChildren[];
}

export default function CategoriesPage() {
  const { t } = useTranslation();
  const [categories, setCategories] = useState<Category[]>([]);
  const [loading, setLoading] = useState(true);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [editingCategory, setEditingCategory] = useState<Category | null>(null);
  const [deletingCategory, setDeletingCategory] = useState<Category | null>(null);
  const [expandedIds, setExpandedIds] = useState<Set<number>>(new Set());

  const [form, setForm] = useState({
    name: "",
    slug: "",
    description: "",
    parent_id: null as number | null,
  });

  const fetchCategories = async () => {
    try {
      const response = await categoriesApi.list();
      // 后端返回 { categories: [...] }
      const cats = response.data?.categories || [];
      setCategories(Array.isArray(cats) ? cats : []);
    } catch (error) {
      toast.error(t("error.loadFailed"));
      setCategories([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCategories();
  }, []);

  const buildTree = (items: Category[]): CategoryWithChildren[] => {
    if (!items || !Array.isArray(items)) return [];
    
    const map = new Map<number, CategoryWithChildren>();
    const roots: CategoryWithChildren[] = [];

    items.forEach((item) => {
      map.set(item.id, { ...item, children: [] });
    });

    items.forEach((item) => {
      const node = map.get(item.id)!;
      if (item.parent_id && map.has(item.parent_id)) {
        map.get(item.parent_id)!.children!.push(node);
      } else {
        roots.push(node);
      }
    });

    return roots;
  };

  const toggleExpand = (id: number) => {
    setExpandedIds((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const openCreateDialog = (parentId?: number) => {
    setEditingCategory(null);
    setForm({ name: "", slug: "", description: "", parent_id: parentId || null });
    setDialogOpen(true);
  };

  const openEditDialog = (category: Category) => {
    setEditingCategory(category);
    setForm({
      name: category.name,
      slug: category.slug,
      description: category.description || "",
      parent_id: category.parent_id,
    });
    setDialogOpen(true);
  };

  const handleSubmit = async () => {
    if (!form.name.trim()) {
      toast.error(t("category.name"));
      return;
    }

    try {
      if (editingCategory) {
        await categoriesApi.update(editingCategory.id, form);
        toast.success(t("category.updateSuccess"));
      } else {
        await categoriesApi.create(form as CreateCategoryInput);
        toast.success(t("category.createSuccess"));
      }
      setDialogOpen(false);
      fetchCategories();
    } catch (error) {
      toast.error(t("common.error"));
    }
  };

  const handleDelete = async () => {
    if (!deletingCategory) return;
    try {
      await categoriesApi.delete(deletingCategory.id);
      toast.success(t("category.deleteSuccess"));
      setDeleteDialogOpen(false);
      setDeletingCategory(null);
      fetchCategories();
    } catch (error) {
      toast.error(t("category.deleteFailed"));
    }
  };


  const renderCategory = (category: CategoryWithChildren, level = 0) => {
    const hasChildren = category.children && category.children.length > 0;
    const isExpanded = expandedIds.has(category.id);
    const isDefaultCategory = category.slug === "uncategorized";

    return (
      <div key={category.id}>
        <div
          className={cn(
            "flex items-center gap-2 p-3 hover:bg-muted/50 rounded-lg transition-colors",
            level > 0 && "ml-6"
          )}
        >
          <button
            className={cn(
              "p-1 rounded hover:bg-muted transition-colors",
              !hasChildren && "invisible"
            )}
            onClick={() => toggleExpand(category.id)}
          >
            <ChevronRight
              className={cn(
                "h-4 w-4 transition-transform",
                isExpanded && "rotate-90"
              )}
            />
          </button>
          <FolderTree className="h-4 w-4 text-muted-foreground" />
          <span className="flex-1 font-medium">
            {isDefaultCategory ? t("category.uncategorized") : category.name}
          </span>
          <span className="text-sm text-muted-foreground">{category.slug}</span>
          <div className="flex items-center gap-1">
            <Button
              variant="ghost"
              size="icon"
              onClick={() => openEditDialog(category)}
            >
              <Edit className="h-4 w-4" />
            </Button>
            {!isDefaultCategory && (
              <Button
                variant="ghost"
                size="icon"
                onClick={() => {
                  setDeletingCategory(category);
                  setDeleteDialogOpen(true);
                }}
              >
                <Trash2 className="h-4 w-4 text-destructive" />
              </Button>
            )}
            <Button
              variant="ghost"
              size="sm"
              onClick={() => openCreateDialog(category.id)}
            >
              <Plus className="h-4 w-4" />
            </Button>
          </div>
        </div>
        {hasChildren && isExpanded && (
          <div className="border-l ml-5">
            {category.children!.map((child) => renderCategory(child, level + 1))}
          </div>
        )}
      </div>
    );
  };

  const tree = buildTree(categories);

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t("manage.categories")}</h1>
          <p className="text-muted-foreground">{t("category.totalCategories")}</p>
        </div>
        <Button onClick={() => openCreateDialog()}>
          <Plus className="h-4 w-4 mr-2" />
          {t("category.newCategory")}
        </Button>
      </div>

      <Card>
        <CardContent className="p-4">
          {loading ? (
            <div className="space-y-3">
              {Array.from({ length: 5 }).map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : tree.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <FolderTree className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>{t("category.noCategories")}</p>
              <Button variant="link" onClick={() => openCreateDialog()}>
                {t("category.createFirst")}
              </Button>
            </div>
          ) : (
            <div className="space-y-1">
              {tree.map((category) => renderCategory(category))}
            </div>
          )}
        </CardContent>
      </Card>

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
              <Label htmlFor="slug">Slug</Label>
              <Input
                id="slug"
                value={form.slug}
                onChange={(e) => setForm((f) => ({ ...f, slug: e.target.value }))}
                placeholder="url-friendly-slug"
              />
            </div>
            <div className="space-y-2">
              <Label htmlFor="parent">{t("category.parent")}</Label>
              <Select
                value={form.parent_id?.toString() || "none"}
                onValueChange={(v) =>
                  setForm((f) => ({ ...f, parent_id: v === "none" ? null : parseInt(v) }))
                }
              >
                <SelectTrigger>
                  <SelectValue placeholder={t("category.parent")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="none">{t("category.noParent")}</SelectItem>
                  {Array.isArray(categories) && categories
                    .filter((c) => c.id !== editingCategory?.id)
                    .map((cat) => (
                      <SelectItem key={cat.id} value={cat.id.toString()}>
                        {cat.name}
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
            <Button onClick={handleSubmit}>
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
            <AlertDialogAction onClick={handleDelete}>{t("common.delete")}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
