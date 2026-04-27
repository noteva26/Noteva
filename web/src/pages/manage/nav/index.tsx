import { useEffect, useOptimistic, useState, useTransition } from "react";
import { useTranslation } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
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
import { Plus, Pencil, Trash2, ChevronUp, ChevronDown, ExternalLink, Eye, EyeOff, FolderOpen } from "lucide-react";
import { toast } from "sonner";
import { api } from "@/lib/api";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";
import { getApiErrorMessage } from "@/lib/api-error";

interface NavItem {
  id: number;
  parent_id: number | null;
  title: string;
  nav_type: string;
  target: string;
  open_new_tab: boolean;
  sort_order: number;
  visible: boolean;
  children?: NavItem[];
}

interface Page {
  id: number;
  slug: string;
  title: string;
  status: string;
}

type NavOptimisticAction =
  | { type: "remove"; id: number }
  | { type: "toggleVisible"; id: number };

function removeNavItem(items: NavItem[], id: number): NavItem[] {
  return items
    .filter((item) => item.id !== id)
    .map((item) => ({
      ...item,
      children: item.children ? removeNavItem(item.children, id) : undefined,
    }));
}

function updateNavItemInTree(items: NavItem[], updatedItem: NavItem): NavItem[] {
  return items.map((item) => {
    if (item.id === updatedItem.id) {
      return { ...updatedItem, children: item.children };
    }

    return {
      ...item,
      children: item.children ? updateNavItemInTree(item.children, updatedItem) : undefined,
    };
  });
}

function insertNavItem(items: NavItem[], newItem: NavItem): NavItem[] {
  if (newItem.parent_id === null) {
    return [...items, newItem];
  }

  return items.map((item) => {
    if (item.id === newItem.parent_id) {
      return { ...item, children: [...(item.children || []), newItem] };
    }

    return {
      ...item,
      children: item.children ? insertNavItem(item.children, newItem) : undefined,
    };
  });
}

function reduceOptimisticNavAction(items: NavItem[], action: NavOptimisticAction): NavItem[] {
  if (action.type === "remove") {
    return removeNavItem(items, action.id);
  }

  return items.map((item) => ({
    ...item,
    visible: item.id === action.id ? !item.visible : item.visible,
    children: item.children ? reduceOptimisticNavAction(item.children, action) : undefined,
  }));
}

function isSafeExternalUrl(value: string): boolean {
  const url = value.trim().toLowerCase();
  return (
    url.startsWith("http://") ||
    url.startsWith("https://") ||
    url.startsWith("mailto:") ||
    url.startsWith("tel:")
  );
}

export default function NavManagePage() {
  const { t } = useTranslation();
  const [navItems, setNavItems] = useState<NavItem[]>([]);
  const [optimisticNavItems, applyOptimisticNavAction] = useOptimistic(
    navItems,
    reduceOptimisticNavAction
  );
  const [pages, setPages] = useState<Page[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [refreshKey, setRefreshKey] = useState(0);
  const [isRefreshing, startRefreshTransition] = useTransition();
  const [isMutating, startMutationTransition] = useTransition();
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [editingItem, setEditingItem] = useState<NavItem | null>(null);
  const [deletingItem, setDeletingItem] = useState<NavItem | null>(null);
  const [parentId, setParentId] = useState<number | null>(null);
  const [formData, setFormData] = useState({
    title: "",
    nav_type: "builtin",
    target: "home",
    open_new_tab: false,
    visible: true,
  });

  const BUILTIN_TARGETS = [
    { value: "home", label: t("navManage.home") },
    { value: "archives", label: t("navManage.archives") },
    { value: "categories", label: t("navManage.categories") },
    { value: "tags", label: t("navManage.tags") },
  ];

  useEffect(() => {
    let active = true;

    const loadData = async () => {
      try {
        setLoading(true);
        const [navRes, pagesRes] = await Promise.all([
          api.get<{ items: NavItem[] }>("/admin/nav/tree"),
          api.get<{ pages: Page[] }>("/admin/pages"),
        ]);
        if (!active) return;

        setNavItems(navRes.data.items || []);
        setPages(pagesRes.data.pages || []);
      } catch {
        if (active) toast.error(t("navManage.loadFailed"));
      } finally {
        if (active) {
          setLoading(false);
          setHasLoaded(true);
        }
      }
    };

    loadData();
    return () => {
      active = false;
    };
  }, [refreshKey, t]);

  const refreshNav = () => {
    startRefreshTransition(() => setRefreshKey((key) => key + 1));
  };

  const getAllItems = (): NavItem[] => {
    const result: NavItem[] = [];
    const collect = (items: NavItem[]) => {
      for (const item of items) {
        result.push(item);
        if (item.children) collect(item.children);
      }
    };
    collect(optimisticNavItems);
    return result;
  };

  const getMaxSortOrder = (pId: number | null): number => {
    const allItems = getAllItems();
    const siblings = allItems.filter(i => i.parent_id === pId);
    if (siblings.length === 0) return 0;
    return Math.max(...siblings.map(s => s.sort_order)) + 1;
  };

  const openCreateDialog = (pId: number | null = null) => {
    setEditingItem(null);
    setParentId(pId);
    setFormData({
      title: "",
      nav_type: pId === null ? "group" : "builtin",
      target: "home",
      open_new_tab: false,
      visible: true,
    });
    setDialogOpen(true);
  };

  const openEditDialog = (item: NavItem) => {
    setEditingItem(item);
    setParentId(item.parent_id);
    const isGroup = item.nav_type === "builtin" && !item.target;
    setFormData({
      title: item.title,
      nav_type: isGroup ? "group" : item.nav_type,
      target: item.target || "home",
      open_new_tab: item.open_new_tab,
      visible: item.visible,
    });
    setDialogOpen(true);
  };

  const handleSubmit = () => {
    if (!formData.title.trim()) {
      toast.error(t("navManage.fillTitle"));
      return;
    }
    if (formData.nav_type === "external" && !formData.target.trim()) {
      toast.error(t("navManage.fillUrl"));
      return;
    }
    if (formData.nav_type === "external" && !isSafeExternalUrl(formData.target)) {
      toast.error(t("navManage.fillUrl"));
      return;
    }

    startMutationTransition(async () => {
      try {
        const payload = {
          parent_id: parentId,
          title: formData.title,
          nav_type: formData.nav_type === "group" ? "builtin" : formData.nav_type,
          target: formData.nav_type === "group" ? "" : formData.target,
          open_new_tab: formData.open_new_tab,
          sort_order: editingItem?.sort_order ?? getMaxSortOrder(parentId),
          visible: formData.visible,
        };

        if (editingItem) {
          const { data } = await api.put<{ item: NavItem }>(`/admin/nav/${editingItem.id}`, payload);
          setNavItems((current) => updateNavItemInTree(current, data.item));
          toast.success(t("navManage.updateSuccess"));
        } else {
          const { data } = await api.post<{ item: NavItem }>("/admin/nav", payload);
          setNavItems((current) => insertNavItem(current, data.item));
          toast.success(t("navManage.createSuccess"));
        }
        setDialogOpen(false);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("navManage.saveFailed")));
      }
    });
  };

  const handleDelete = () => {
    if (!deletingItem) return;

    const item = deletingItem;
    startMutationTransition(async () => {
      applyOptimisticNavAction({ type: "remove", id: item.id });
      setDeleteDialogOpen(false);
      setDeletingItem(null);

      try {
        await api.delete(`/admin/nav/${item.id}`);
        setNavItems((current) => removeNavItem(current, item.id));
        toast.success(t("navManage.deleteSuccess"));
      } catch {
        toast.error(t("navManage.deleteFailed"));
        refreshNav();
      }
    });
  };

  const toggleVisible = (item: NavItem) => {
    startMutationTransition(async () => {
      applyOptimisticNavAction({ type: "toggleVisible", id: item.id });
      try {
        const { data } = await api.put<{ item: NavItem }>(`/admin/nav/${item.id}`, { visible: !item.visible });
        setNavItems((current) => updateNavItemInTree(current, data.item));
      } catch {
        toast.error(t("navManage.updateFailed"));
        refreshNav();
      }
    });
  };

  const moveItem = (item: NavItem, direction: "up" | "down") => {
    const allItems = getAllItems();
    const siblings = allItems
      .filter(i => i.parent_id === item.parent_id)
      .sort((a, b) => a.sort_order - b.sort_order);

    const idx = siblings.findIndex(s => s.id === item.id);
    if (idx === -1) return;
    if (direction === "up" && idx === 0) return;
    if (direction === "down" && idx === siblings.length - 1) return;

    const swapIdx = direction === "up" ? idx - 1 : idx + 1;
    const swapItem = siblings[swapIdx];

    startMutationTransition(async () => {
      try {
        await api.put("/admin/nav/order", {
          items: [
            { id: item.id, parent_id: item.parent_id, sort_order: swapItem.sort_order },
            { id: swapItem.id, parent_id: swapItem.parent_id, sort_order: item.sort_order },
          ],
        });
        refreshNav();
      } catch {
        toast.error(t("navManage.sortFailed"));
      }
    });
  };

  const getNavTypeLabel = (item: NavItem) => {
    if (item.nav_type === "builtin" && !item.target) return t("navManage.group");
    if (item.nav_type === "builtin") return t("navManage.builtin");
    if (item.nav_type === "page") return t("navManage.customPage");
    if (item.nav_type === "external") return t("navManage.external");
    return item.nav_type;
  };

  const getTargetDisplay = (item: NavItem) => {
    if (item.nav_type === "builtin" && !item.target) return null;
    if (item.nav_type === "builtin") {
      return BUILTIN_TARGETS.find(bt => bt.value === item.target)?.label || item.target;
    }
    if (item.nav_type === "page") return `/${item.target}`;
    if (item.nav_type === "external") return item.target;
    return null;
  };

  const renderNavItem = (item: NavItem, level = 0) => {
    const allItems = getAllItems();
    const siblings = allItems
      .filter(i => i.parent_id === item.parent_id)
      .sort((a, b) => a.sort_order - b.sort_order);
    const idx = siblings.findIndex(s => s.id === item.id);
    const isFirst = idx === 0;
    const isLast = idx === siblings.length - 1;
    const isGroup = item.nav_type === "builtin" && !item.target;

    return (
      <div key={item.id} className="mb-2">
        <div
          className={`flex items-center gap-3 p-3 border rounded-lg ${!item.visible ? "opacity-50 bg-muted/50" : "bg-card"
            } ${level === 0 ? "border-primary/30" : ""}`}
          style={{ marginLeft: level * 32 }}
        >
          <div className="w-6 flex justify-center">
            {isGroup ? (
              <FolderOpen className="h-4 w-4 text-primary" />
            ) : item.nav_type === "external" ? (
              <ExternalLink className="h-4 w-4 text-muted-foreground" />
            ) : null}
          </div>

          <div className="flex-1 min-w-0">
            <div className="flex items-center gap-2">
              <span className="font-medium truncate">{item.title}</span>
              <Badge variant={level === 0 ? "default" : "outline"} className="text-xs shrink-0">
                {getNavTypeLabel(item)}
              </Badge>
              {item.open_new_tab && (
                <ExternalLink className="h-3 w-3 text-muted-foreground shrink-0" />
              )}
            </div>
            {getTargetDisplay(item) && (
              <div className="text-sm text-muted-foreground truncate">
                {getTargetDisplay(item)}
              </div>
            )}
          </div>

          <div className="flex items-center gap-1 shrink-0">
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => moveItem(item, "up")} disabled={isFirst || isMutating}>
              <ChevronUp className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => moveItem(item, "down")} disabled={isLast || isMutating}>
              <ChevronDown className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => toggleVisible(item)} disabled={isMutating}>
              {item.visible ? <Eye className="h-4 w-4" /> : <EyeOff className="h-4 w-4" />}
            </Button>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => openEditDialog(item)}>
              <Pencil className="h-4 w-4" />
            </Button>
            <Button variant="ghost" size="icon" className="h-8 w-8" onClick={() => { setDeletingItem(item); setDeleteDialogOpen(true); }}>
              <Trash2 className="h-4 w-4" />
            </Button>
            {level === 0 && (
              <Button variant="outline" size="sm" className="h-8 ml-1" onClick={() => openCreateDialog(item.id)}>
                <Plus className="h-3 w-3 mr-1" />
                {t("navManage.addChild")}
              </Button>
            )}
          </div>
        </div>

        {item.children && item.children.length > 0 && (
          <div className="mt-2">
            {[...item.children].sort((a, b) => a.sort_order - b.sort_order).map(child => renderNavItem(child, level + 1))}
          </div>
        )}
      </div>
    );
  };

  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = (loading && hasLoaded) || isRefreshing;

  return (
    <div className="space-y-6">
      <div className="flex justify-between items-center mb-6">
        <div>
          <h1 className="text-2xl font-bold">{t("navManage.title")}</h1>
          <p className="text-sm text-muted-foreground mt-1">{t("navManage.navHint")}</p>
        </div>
        <Button onClick={() => openCreateDialog(null)}>
          <Plus className="h-4 w-4 mr-2" />
          {t("navManage.addTopNav")}
        </Button>
      </div>
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />

      <div className={`space-y-2 transition-opacity ${isSyncing ? "opacity-70" : ""}`}>
        <DataSyncBar active={isSyncing} label={t("common.loading")} className="mb-3" />
        {showInitialLoading ? (
          <div className="text-center text-muted-foreground py-12 border rounded-lg bg-muted/20">{t("common.loading")}</div>
        ) : (
          <>
            {[...optimisticNavItems].sort((a, b) => a.sort_order - b.sort_order).map(item => renderNavItem(item))}
            {optimisticNavItems.length === 0 && (
              <div className="text-center text-muted-foreground py-12 border rounded-lg bg-muted/20">{t("navManage.noNavItems")}</div>
            )}
          </>
        )}
      </div>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{editingItem ? t("navManage.editNav") : (parentId ? t("navManage.addSubNav") : t("navManage.addTopNav"))}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4">
            <div className="space-y-2">
              <Label>{t("navManage.navTitle")}</Label>
              <Input value={formData.title} onChange={(e) => setFormData({ ...formData, title: e.target.value })} placeholder={t("navManage.navTitle")} />
            </div>

            <div className="space-y-2">
              <Label>{t("navManage.navType")}</Label>
              <Select
                value={formData.nav_type}
                onValueChange={(v) => {
                  setFormData({
                    ...formData,
                    nav_type: v,
                    target: v === "builtin" ? "home" : v === "page" ? (pages[0]?.slug || "") : "",
                    open_new_tab: v === "external",
                  });
                }}
              >
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  {parentId === null && <SelectItem value="group">{t("navManage.groupOnly")}</SelectItem>}
                  <SelectItem value="builtin">{t("navManage.builtin")}</SelectItem>
                  <SelectItem value="page">{t("navManage.customPage")}</SelectItem>
                  <SelectItem value="external">{t("navManage.external")}</SelectItem>
                </SelectContent>
              </Select>
            </div>

            {formData.nav_type === "builtin" && (
              <div className="space-y-2">
                <Label>{t("navManage.targetPage")}</Label>
                <Select value={formData.target} onValueChange={(v) => setFormData({ ...formData, target: v })}>
                  <SelectTrigger><SelectValue /></SelectTrigger>
                  <SelectContent>
                    {BUILTIN_TARGETS.map((bt) => <SelectItem key={bt.value} value={bt.value}>{bt.label}</SelectItem>)}
                  </SelectContent>
                </Select>
              </div>
            )}

            {formData.nav_type === "page" && (
              <div className="space-y-2">
                <Label>{t("navManage.selectPage")}</Label>
                <Select value={formData.target} onValueChange={(v) => setFormData({ ...formData, target: v })}>
                  <SelectTrigger><SelectValue placeholder={t("navManage.selectPage")} /></SelectTrigger>
                  <SelectContent>
                    {pages.filter(p => p.status === "published").map((p) => <SelectItem key={p.slug} value={p.slug}>{p.title} (/{p.slug})</SelectItem>)}
                  </SelectContent>
                </Select>
              </div>
            )}

            {formData.nav_type === "external" && (
              <div className="space-y-2">
                <Label>{t("navManage.url")}</Label>
                <Input value={formData.target} onChange={(e) => setFormData({ ...formData, target: e.target.value })} placeholder="https://example.com" />
              </div>
            )}

            {formData.nav_type === "external" && (
              <div className="flex items-center gap-2">
                <Switch checked={formData.open_new_tab} onCheckedChange={(v) => setFormData({ ...formData, open_new_tab: v })} />
                <Label>{t("navManage.openNewTab")}</Label>
              </div>
            )}

            <div className="flex items-center gap-2">
              <Switch checked={formData.visible} onCheckedChange={(v) => setFormData({ ...formData, visible: v })} />
              <Label>{t("navManage.visible")}</Label>
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>{t("common.cancel")}</Button>
            <Button onClick={handleSubmit} disabled={isMutating}>{t("common.save")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
            <AlertDialogDescription>{t("navManage.confirmDelete", { title: deletingItem?.title || "" })}</AlertDialogDescription>
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
