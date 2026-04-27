import { useDeferredValue, useEffect, useOptimistic, useState, useTransition } from "react";
import { motion } from "motion/react";
import { tagsApi, Tag, TagWithCount } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import { Checkbox } from "@/components/ui/checkbox";
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
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Plus, Search, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useTranslation } from "@/lib/i18n";
import { EmptyState } from "@/components/ui/empty-state";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";

interface TagsPageProps {
  embedded?: boolean;
}

export default function TagsPage({ embedded = false }: TagsPageProps) {
  const { t } = useTranslation();
  const [tags, setTags] = useState<TagWithCount[]>([]);
  const [optimisticTags, removeOptimisticTags] = useOptimistic(
    tags,
    (currentTags, tagIds: Set<number>) =>
      currentTags.filter((tag) => !tagIds.has(tag.id))
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
  const [deletingTag, setDeletingTag] = useState<Tag | null>(null);
  const [newTagName, setNewTagName] = useState("");
  const [selectedTags, setSelectedTags] = useState<Set<number>>(new Set());

  useEffect(() => {
    let active = true;

    const fetchTags = async () => {
      try {
        setLoading(true);
        const response = await tagsApi.list();
        if (!active) return;

        // 鍚庣杩斿洖 { tags: [...] }
        const tagsArray = response.data?.tags || [];
        const tagsWithCount = (Array.isArray(tagsArray) ? tagsArray : []).map((tag: Tag & { article_count?: number }) => ({
          ...tag,
          count: tag.article_count || 0,
        }));
        setTags(tagsWithCount);
        setSelectedTags((current) => {
          const loadedIds = new Set(tagsWithCount.map((tag) => tag.id));
          return new Set(Array.from(current).filter((id) => loadedIds.has(id)));
        });
      } catch {
        if (!active) return;
        toast.error(t("error.loadFailed"));
        setTags([]);
      } finally {
        if (active) {
          setLoading(false);
          setHasLoaded(true);
        }
      }
    };

    fetchTags();
    return () => {
      active = false;
    };
  }, [refreshKey, t]);

  const refreshTags = () => {
    startRefreshTransition(() => {
      setRefreshKey((key) => key + 1);
    });
  };

  const handleCreate = () => {
    if (!newTagName.trim()) {
      toast.error(t("tag.name"));
      return;
    }

    const name = newTagName.trim();
    startMutationTransition(async () => {
      try {
        const response = await tagsApi.create(name);
        setTags((current) => [...current, { ...response.data, count: 0 }]);
        toast.success(t("tag.createSuccess"));
        setDialogOpen(false);
        setNewTagName("");
      } catch {
        toast.error(t("common.error"));
      }
    });
  };

  const handleDelete = () => {
    if (!deletingTag) return;

    const tag = deletingTag;
    startMutationTransition(async () => {
      removeOptimisticTags(new Set([tag.id]));
      setDeleteDialogOpen(false);
      setDeletingTag(null);

      try {
        await tagsApi.delete(tag.id);
        setTags((current) => current.filter((item) => item.id !== tag.id));
        setSelectedTags((current) => {
          const next = new Set(current);
          next.delete(tag.id);
          return next;
        });
        toast.success(t("tag.deleteSuccess"));
      } catch {
        toast.error(t("tag.deleteFailed"));
        refreshTags();
      }
    });
  };

  const handleBatchDelete = () => {
    if (selectedTags.size === 0) return;

    const ids = new Set(selectedTags);
    startMutationTransition(async () => {
      removeOptimisticTags(ids);
      setSelectedTags(new Set());

      try {
        await Promise.all(Array.from(ids).map((id) => tagsApi.delete(id)));
        setTags((current) => current.filter((tag) => !ids.has(tag.id)));
        toast.success(t("tag.batchDeleteSuccess", { count: ids.size.toString() }));
      } catch {
        toast.error(t("tag.deleteFailed"));
        refreshTags();
      }
    });
  };

  const toggleSelect = (id: number) => {
    setSelectedTags((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const filteredTags = optimisticTags.filter((tag) =>
    tag.name.toLowerCase().includes(deferredSearch.toLowerCase())
  );

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
          {embedded ? (
            <h2 className="text-xl font-semibold">{t("manage.tags")}</h2>
          ) : (
            <h1 className="text-3xl font-bold">{t("manage.tags")}</h1>
          )}
          <p className={embedded ? "text-sm text-muted-foreground" : "text-muted-foreground"}>
            {t("tag.totalTags")}
          </p>
        </div>
        <div className="flex items-center gap-2">
          {selectedTags.size > 0 && (
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
            >
              <Button variant="destructive" onClick={handleBatchDelete} disabled={isMutating}>
                <Trash2 className="h-4 w-4 mr-2" />
                {t("tag.batchDelete")} ({selectedTags.size})
              </Button>
            </motion.div>
          )}
          <motion.div whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}>
            <Button onClick={() => setDialogOpen(true)} disabled={isMutating}>
              <Plus className="h-4 w-4 mr-2" />
              {t("tag.newTag")}
            </Button>
          </motion.div>
        </div>
      </motion.div>
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />

      <div className="flex items-center gap-4">
        <div className="relative flex-1 max-w-sm">
          <Search className="absolute left-3 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
          <Input
            placeholder={t("common.search")}
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            className="pl-9"
          />
        </div>
        <span className="text-sm text-muted-foreground">
          {optimisticTags.length} {t("manage.tags")}
        </span>
      </div>

      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.1 }}
      >
        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("tag.tagList")}</CardTitle>
          </CardHeader>
          <CardContent>
            <DataSyncBar active={isSyncing} label={t("common.loading")} className="mb-3" />
            {showInitialLoading ? (
              <div className="space-y-2">
                {Array.from({ length: 8 }).map((_, i) => (
                  <div key={i} className="h-10 w-full skeleton-shimmer rounded" />
                ))}
              </div>
            ) : filteredTags.length === 0 ? (
              <EmptyState size="sm" description={t("tag.noTags")} />
            ) : (
              <div className={`space-y-2 max-h-[400px] overflow-auto transition-opacity ${isSyncing ? "opacity-70" : ""}`}>
                {filteredTags.map((tag, index) => (
                  <motion.div
                    key={tag.id}
                    initial={{ opacity: 0, x: -10 }}
                    animate={{ opacity: 1, x: 0 }}
                    transition={{ delay: index * 0.02 }}
                    whileHover={{ x: 2 }}
                    className={cn(
                      "flex items-center justify-between p-2 rounded-lg hover:bg-muted/50 transition-colors",
                      selectedTags.has(tag.id) && "bg-muted"
                    )}
                  >
                    <div className="flex items-center gap-2">
                      <Checkbox
                        checked={selectedTags.has(tag.id)}
                        onCheckedChange={() => toggleSelect(tag.id)}
                        aria-label={tag.name}
                      />
                      <Badge variant="outline">{tag.name}</Badge>
                      <span className="text-sm text-muted-foreground">
                        {t("tag.articlesCount", { count: tag.count.toString() })}
                      </span>
                    </div>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => {
                        setDeletingTag(tag);
                        setDeleteDialogOpen(true);
                      }}
                    >
                      <Trash2 className="h-4 w-4 text-destructive" />
                    </Button>
                  </motion.div>
                ))}
              </div>
            )}
          </CardContent>
        </Card>
      </motion.div>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("tag.newTag")}</DialogTitle>
          </DialogHeader>
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="tagName">{t("tag.name")}</Label>
              <Input
                id="tagName"
                value={newTagName}
                onChange={(e) => setNewTagName(e.target.value)}
                placeholder={t("tag.name")}
                onKeyDown={(e) => e.key === "Enter" && handleCreate()}
              />
            </div>
          </div>
          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button onClick={handleCreate} disabled={isMutating}>{t("common.create")}</Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("tag.confirmDelete", { name: deletingTag?.name || "" })}
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

