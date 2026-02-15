import { useState, useEffect, useRef } from "react";
import { useNavigate } from "react-router-dom";
import {
  articlesApi,
  categoriesApi,
  tagsApi,
  Category,
  Tag,
  CreateArticleInput,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ArrowLeft, Save, X, Loader2, Check } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import VditorEditor, { type VditorEditorRef } from "@/components/ui/vditor-editor";

interface PluginEditorButton {
  id: string;
  label: string | Record<string, string>;
  icon?: string;
  insertBefore: string;
  insertAfter: string;
}

interface EnabledPluginInfo {
  id: string;
  settings: Record<string, any>;
  editor_config?: {
    toolbar?: PluginEditorButton[];
  };
}

export default function NewArticlePage() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const editorRef = useRef<VditorEditorRef>(null);
  const [saving, setSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [categories, setCategories] = useState<Category[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const [pluginButtons, setPluginButtons] = useState<PluginEditorButton[]>([]);
  const [dataReady, setDataReady] = useState(false);
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  const [form, setForm] = useState({
    title: "",
    slug: "",
    content: "",
    status: "draft" as "draft" | "published",
    category_id: null as number | null,
  });

  useEffect(() => {
    const hasContent = !!(form.title.trim() || form.content.trim());
    setHasUnsavedChanges(hasContent);
  }, [form.title, form.content]);

  useEffect(() => {
    const handleBeforeUnload = (e: BeforeUnloadEvent) => {
      if (hasUnsavedChanges) {
        e.preventDefault();
        e.returnValue = "";
      }
    };
    window.addEventListener("beforeunload", handleBeforeUnload);
    return () => window.removeEventListener("beforeunload", handleBeforeUnload);
  }, [hasUnsavedChanges]);

  useEffect(() => {
    Promise.all([
      categoriesApi.list(),
      tagsApi.list(),
      fetch("/api/v1/plugins/enabled").then(r => r.json()).catch(() => []),
    ])
      .then(([catRes, tagRes, pluginsRes]) => {
        const catData = catRes.data?.categories || [];
        const tagData = tagRes.data?.tags || [];
        setCategories(Array.isArray(catData) ? catData : []);
        setTags(Array.isArray(tagData) ? tagData : []);
        const defaultCat = catData.find((c: Category) => c.slug === "uncategorized") || catData[0];
        if (defaultCat) {
          setForm((f) => ({ ...f, category_id: defaultCat.id }));
        }
        const buttons: PluginEditorButton[] = [];
        if (Array.isArray(pluginsRes)) {
          pluginsRes.forEach((plugin: EnabledPluginInfo) => {
            if (plugin.editor_config?.toolbar) buttons.push(...plugin.editor_config.toolbar);
          });
        }
        setPluginButtons(buttons);
        setDataReady(true);
      })
      .catch(() => { toast.error("Failed to load data"); setDataReady(true); });
  }, []);

  const generateSlug = (title: string) => {
    return title
      .toLowerCase()
      .replace(/[^a-z0-9\u4e00-\u9fa5\-]+/g, "-")
      .replace(/-+/g, "-")
      .replace(/^-|-$/g, "");
  };

  const handleTitleChange = (title: string) => {
    setForm((f) => ({ ...f, title, slug: generateSlug(title) }));
  };

  const toggleTag = (tagId: number) => {
    setSelectedTags((prev) =>
      prev.includes(tagId) ? prev.filter((id) => id !== tagId) : [...prev, tagId]
    );
  };

  const handleSubmit = async (status: "draft" | "published") => {
    const editorContent = editorRef.current?.getValue() || form.content;
    const currentForm = { ...form, content: editorContent };

    if (!currentForm.title.trim()) { toast.error(t("article.title")); return; }
    if (!currentForm.content.trim()) { toast.error(t("article.content")); return; }
    if (!currentForm.category_id) { toast.error(t("article.category")); return; }

    setSaving(true);
    setSaveSuccess(false);
    try {
      const data: CreateArticleInput = {
        title: currentForm.title,
        slug: currentForm.slug || generateSlug(currentForm.title),
        content: currentForm.content,
        status,
        category_id: currentForm.category_id,
        tag_ids: selectedTags,
      };
      const response = await articlesApi.create(data);
      setSaveSuccess(true);
      setHasUnsavedChanges(false);
      toast.success(status === "published" ? t("article.publishSuccess") : t("article.saveSuccess"));
      setTimeout(() => {
        if (status === "published") {
          navigate("/manage/articles");
        } else {
          navigate(`/manage/articles/${response.data.id}`);
        }
      }, 1000);
    } catch (error) {
      toast.error(t("article.saveFailed"));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <div>
            <h1 className="text-3xl font-bold">{t("article.newArticle")}</h1>
            <p className="text-muted-foreground">{t("article.createNewArticle")}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <Button variant="outline" onClick={() => handleSubmit("draft")} disabled={saving || !form.title.trim() || !form.content.trim()}>
            {saving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSuccess ? <Check className="h-4 w-4 mr-2 text-green-500" /> : <Save className="h-4 w-4 mr-2" />}
            {t("article.saveDraft")}
          </Button>
          <Button onClick={() => handleSubmit("published")} disabled={saving || !form.title.trim() || !form.content.trim()}>
            {saving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSuccess ? <Check className="h-4 w-4 mr-2" /> : null}
            {t("article.publish")}
          </Button>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2 space-y-4">
          <div className="space-y-2">
            <Label htmlFor="title">{t("article.title")}</Label>
            <Input id="title" placeholder={t("article.title")} value={form.title} onChange={(e) => handleTitleChange(e.target.value)} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="slug">Slug</Label>
            <Input id="slug" placeholder="url-friendly-slug" value={form.slug} onChange={(e) => setForm((f) => ({ ...f, slug: e.target.value }))} />
          </div>
          <div className="space-y-2">
            <Label>{t("article.content")}</Label>
            {dataReady ? (
              <VditorEditor
                ref={editorRef}
                initialValue=""
                onChange={(value) => setForm((f) => ({ ...f, content: value }))}
                pluginButtons={pluginButtons}
                placeholder={t("article.useMarkdown")}
                minHeight={400}
              />
            ) : (
              <div className="h-[400px] rounded-md border border-input animate-pulse bg-muted/30" />
            )}
          </div>
        </div>

        <div className="space-y-4">
          <Card>
            <CardHeader><CardTitle className="text-base">{t("article.category")}</CardTitle></CardHeader>
            <CardContent>
              <Select value={form.category_id?.toString() || ""} onValueChange={(v) => setForm((f) => ({ ...f, category_id: parseInt(v) }))}>
                <SelectTrigger>
                  <SelectValue>
                    {form.category_id
                      ? (() => {
                          const cat = categories.find((c) => c.id === form.category_id);
                          if (!cat) return t("article.category");
                          return cat.slug === "uncategorized" ? t("category.uncategorized") : cat.name;
                        })()
                      : t("article.category")}
                  </SelectValue>
                </SelectTrigger>
                <SelectContent>
                  {Array.isArray(categories) && categories.map((cat) => (
                    <SelectItem key={cat.id} value={cat.id.toString()}>
                      {cat.slug === "uncategorized" ? t("category.uncategorized") : cat.name}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">{t("article.tags")}</CardTitle></CardHeader>
            <CardContent className="space-y-3">
              {selectedTags.length > 0 && (
                <div className="flex flex-wrap gap-2 pb-2 border-b">
                  {selectedTags.map((tagId) => {
                    const tag = tags.find((t) => t.id === tagId);
                    return tag ? (
                      <Badge key={tag.id} variant="default" className="cursor-pointer" onClick={() => toggleTag(tag.id)}>
                        {tag.name}<X className="h-3 w-3 ml-1" />
                      </Badge>
                    ) : null;
                  })}
                </div>
              )}
              <div className="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
                {tags.filter((tag) => !selectedTags.includes(tag.id)).map((tag) => (
                  <Badge key={tag.id} variant="outline" className="cursor-pointer hover:bg-muted" onClick={() => toggleTag(tag.id)}>
                    {tag.name}
                  </Badge>
                ))}
                {tags.length === 0 && <p className="text-sm text-muted-foreground">{t("tag.noTags")}</p>}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
