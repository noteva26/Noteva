import { useEffect, useState } from "react";
import { motion } from "motion/react";
import { tagsApi, Tag, TagWithCount } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
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
import { Plus, Search, Trash2, Tags } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useTranslation } from "@/lib/i18n";
import { EmptyState } from "@/components/ui/empty-state";

export default function TagsPage() {
  const { t } = useTranslation();
  const [tags, setTags] = useState<TagWithCount[]>([]);
  const [loading, setLoading] = useState(true);
  const [search, setSearch] = useState("");
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [deletingTag, setDeletingTag] = useState<Tag | null>(null);
  const [newTagName, setNewTagName] = useState("");
  const [selectedTags, setSelectedTags] = useState<Set<number>>(new Set());

  const fetchTags = async () => {
    try {
      const response = await tagsApi.list();
      // 鍚庣杩斿洖 { tags: [...] }
      const tagsArray = response.data?.tags || [];
      const tagsWithCount = (Array.isArray(tagsArray) ? tagsArray : []).map((tag: Tag & { article_count?: number }) => ({
        ...tag,
        count: tag.article_count || 0,
      }));
      setTags(tagsWithCount);
    } catch (error) {
      toast.error(t("error.loadFailed"));
      setTags([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchTags();
  }, []);

  const handleCreate = async () => {
    if (!newTagName.trim()) {
      toast.error(t("tag.name"));
      return;
    }

    try {
      await tagsApi.create(newTagName);
      toast.success(t("tag.createSuccess"));
      setDialogOpen(false);
      setNewTagName("");
      fetchTags();
    } catch (error) {
      toast.error(t("common.error"));
    }
  };

  const handleDelete = async () => {
    if (!deletingTag) return;
    try {
      await tagsApi.delete(deletingTag.id);
      toast.success(t("tag.deleteSuccess"));
      setDeleteDialogOpen(false);
      setDeletingTag(null);
      fetchTags();
    } catch (error) {
      toast.error(t("tag.deleteFailed"));
    }
  };

  const handleBatchDelete = async () => {
    if (selectedTags.size === 0) return;
    try {
      await Promise.all(Array.from(selectedTags).map((id) => tagsApi.delete(id)));
      toast.success(t("tag.batchDeleteSuccess", { count: selectedTags.size.toString() }));
      setSelectedTags(new Set());
      fetchTags();
    } catch (error) {
      toast.error(t("tag.deleteFailed"));
    }
  };

  const toggleSelect = (id: number) => {
    setSelectedTags((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const filteredTags = tags.filter((tag) =>
    tag.name.toLowerCase().includes(search.toLowerCase())
  );

  const maxCount = Math.max(...tags.map((t) => t.count), 1);

  const getTagSize = (count: number) => {
    const ratio = count / maxCount;
    if (ratio > 0.8) return "text-2xl";
    if (ratio > 0.6) return "text-xl";
    if (ratio > 0.4) return "text-lg";
    if (ratio > 0.2) return "text-base";
    return "text-sm";
  };


  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="flex items-center justify-between"
      >
        <div>
          <h1 className="text-3xl font-bold">{t("manage.tags")}</h1>
          <p className="text-muted-foreground">{t("tag.totalTags")}</p>
        </div>
        <div className="flex items-center gap-2">
          {selectedTags.size > 0 && (
            <motion.div
              initial={{ opacity: 0, scale: 0.9 }}
              animate={{ opacity: 1, scale: 1 }}
            >
              <Button variant="destructive" onClick={handleBatchDelete}>
                <Trash2 className="h-4 w-4 mr-2" />
                {t("tag.batchDelete")} ({selectedTags.size})
              </Button>
            </motion.div>
          )}
          <motion.div whileHover={{ scale: 1.02 }} whileTap={{ scale: 0.98 }}>
            <Button onClick={() => setDialogOpen(true)}>
              <Plus className="h-4 w-4 mr-2" />
              {t("tag.newTag")}
            </Button>
          </motion.div>
        </div>
      </motion.div>

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
          {tags.length} {t("manage.tags")}
        </span>
      </div>

      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.1 }}
        className="grid gap-6 md:grid-cols-2"
      >
        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("tag.tagCloud")}</CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="flex flex-wrap gap-2">
                {Array.from({ length: 12 }).map((_, i) => (
                  <div key={i} className="h-8 w-20 skeleton-shimmer rounded" />
                ))}
              </div>
            ) : filteredTags.length === 0 ? (
              <EmptyState
                size="sm"
                icon={Tags}
                description={t("tag.noTags")}
              />
            ) : (
              <div className="flex flex-wrap gap-3 items-center">
                {filteredTags.map((tag, index) => (
                  <motion.span
                    key={tag.id}
                    initial={{ opacity: 0, scale: 0.8 }}
                    animate={{ opacity: 1, scale: 1 }}
                    transition={{ delay: index * 0.02 }}
                    whileHover={{ scale: 1.1 }}
                    className={cn(
                      "cursor-pointer hover:text-primary transition-colors",
                      getTagSize(tag.count),
                      selectedTags.has(tag.id) && "text-primary"
                    )}
                    onClick={() => toggleSelect(tag.id)}
                  >
                    {tag.name}
                    <span className="text-xs text-muted-foreground ml-1">
                      ({tag.count})
                    </span>
                  </motion.span>
                ))}
              </div>
            )}
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle className="text-base">{t("tag.tagList")}</CardTitle>
          </CardHeader>
          <CardContent>
            {loading ? (
              <div className="space-y-2">
                {Array.from({ length: 8 }).map((_, i) => (
                  <div key={i} className="h-10 w-full skeleton-shimmer rounded" />
                ))}
              </div>
            ) : filteredTags.length === 0 ? (
              <EmptyState size="sm" description={t("tag.noTags")} />
            ) : (
              <div className="space-y-2 max-h-[400px] overflow-auto">
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
                      <input
                        type="checkbox"
                        checked={selectedTags.has(tag.id)}
                        onChange={() => toggleSelect(tag.id)}
                        className="rounded"
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
            <Button onClick={handleCreate}>{t("common.create")}</Button>
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
            <AlertDialogAction onClick={handleDelete}>{t("common.delete")}</AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

