import { useState, useEffect, useRef, useMemo, useCallback } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  articlesApi,
  categoriesApi,
  tagsApi,
  uploadApi,
  Category,
  Tag,
  UpdateArticleInput,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ArrowLeft, Save, Eye, X, Trash2, Bold, Italic, Link, Image as ImageIcon, List, ListOrdered, Quote, Code, Heading1, Heading2, Pin, Loader2, Check } from "lucide-react";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@/components/ui/alert-dialog";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { EmojiPicker } from "@/components/ui/emoji-picker";

interface PluginEditorButton {
  id: string;
  label: string;
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

function extractImages(content: string): string[] {
  const imgRegex = /!\[.*?\]\((.*?)\)/g;
  const images: string[] = [];
  let match;
  while ((match = imgRegex.exec(content)) !== null) {
    images.push(match[1]);
  }
  return images;
}

export default function EditArticlePage() {
  const navigate = useNavigate();
  const { id } = useParams();
  const articleId = parseInt(id || "0");
  const { t } = useTranslation();
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [preview, setPreview] = useState(false);
  const [previewHtml, setPreviewHtml] = useState("");
  const [categories, setCategories] = useState<Category[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const [pluginButtons, setPluginButtons] = useState<PluginEditorButton[]>([]);
  
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);
  const [lastSavedContent, setLastSavedContent] = useState("");
  const [autoSaveEnabled, setAutoSaveEnabled] = useState(true);
  const autoSaveTimerRef = useRef<NodeJS.Timeout | null>(null);

  const [form, setForm] = useState({
    title: "",
    slug: "",
    content: "",
    status: "draft" as "draft" | "published" | "archived",
    category_id: 0,
    thumbnail: null as string | null,
    is_pinned: false,
    pin_order: 0,
  });

  const contentImages = useMemo(() => extractImages(form.content), [form.content]);

  useEffect(() => {
    if (!loading && lastSavedContent) {
      const currentState = JSON.stringify({ ...form, selectedTags });
      setHasUnsavedChanges(currentState !== lastSavedContent);
    }
  }, [form, selectedTags, lastSavedContent, loading]);

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
    if (!autoSaveEnabled || !hasUnsavedChanges || !articleId || form.status === "published") return;
    
    autoSaveTimerRef.current = setTimeout(async () => {
      if (hasUnsavedChanges && form.title.trim()) {
        try {
          const data: UpdateArticleInput = {
            title: form.title, slug: form.slug, content: form.content,
            status: form.status, category_id: form.category_id,
            tag_ids: selectedTags, thumbnail: form.thumbnail,
            is_pinned: form.is_pinned, pin_order: form.pin_order,
          };
          await articlesApi.update(articleId, data);
          setLastSavedContent(JSON.stringify({ ...form, selectedTags }));
          setHasUnsavedChanges(false);
          toast.success(t("article.autoSaved"), { duration: 2000 });
        } catch {}
      }
    }, 30000);
    
    return () => { if (autoSaveTimerRef.current) clearTimeout(autoSaveTimerRef.current); };
  }, [hasUnsavedChanges, autoSaveEnabled, articleId, form, selectedTags, t]);

  useEffect(() => {
    if (!articleId) return;
    const fetchData = async () => {
      try {
        const [catRes, tagRes, articleRes, pluginsRes] = await Promise.all([
          categoriesApi.list(), tagsApi.list(), articlesApi.getById(articleId),
          fetch("/api/v1/plugins/enabled").then(r => r.json()).catch(() => ({ data: [] })),
        ]);
        setCategories(catRes.data.categories);
        setTags(tagRes.data.tags);
        const buttons: PluginEditorButton[] = [];
        if (Array.isArray(pluginsRes)) {
          pluginsRes.forEach((plugin: EnabledPluginInfo) => {
            if (plugin.editor_config?.toolbar) buttons.push(...plugin.editor_config.toolbar);
          });
        }
        setPluginButtons(buttons);
        const article = articleRes.data;
        const formData = {
          title: article.title, slug: article.slug, content: article.content,
          status: article.status, category_id: article.category_id,
          thumbnail: article.thumbnail || null,
          is_pinned: article.is_pinned || false, pin_order: article.pin_order || 0,
        };
        const tagIds = article.tags?.map((t: any) => t.id) || [];
        setForm(formData);
        setSelectedTags(tagIds);
        setLastSavedContent(JSON.stringify({ ...formData, selectedTags: tagIds }));
      } catch (error) {
        toast.error("加载失败");
        navigate("/manage/articles");
      } finally {
        setLoading(false);
      }
    };
    fetchData();
  }, [articleId, navigate]);

  const toggleTag = (tagId: number) => {
    setSelectedTags((prev) => prev.includes(tagId) ? prev.filter((id) => id !== tagId) : [...prev, tagId]);
  };

  const insertMarkdown = (before: string, after: string = "") => {
    const textarea = textareaRef.current;
    if (!textarea) return;
    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const text = form.content;
    const selected = text.substring(start, end);
    const newText = text.substring(0, start) + before + selected + after + text.substring(end);
    setForm(f => ({ ...f, content: newText }));
    setTimeout(() => {
      textarea.focus();
      textarea.setSelectionRange(start + before.length, start + before.length + selected.length);
    }, 0);
  };

  const handleImageUpload = async () => {
    const input = document.createElement("input");
    input.type = "file";
    input.accept = "image/*";
    input.onchange = async (e) => {
      const file = (e.target as HTMLInputElement).files?.[0];
      if (!file) return;
      try {
        const { data } = await uploadApi.image(file);
        insertMarkdown(`![${file.name}](${data.url})`, "");
        toast.success(t("article.saveSuccess"));
      } catch { toast.error(t("article.saveFailed")); }
    };
    input.click();
  };

  const handlePaste = async (e: React.ClipboardEvent) => {
    const items = e.clipboardData.items;
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.type.startsWith("image/")) {
        e.preventDefault();
        const file = item.getAsFile();
        if (!file) continue;
        try {
          const { data } = await uploadApi.image(file);
          insertMarkdown(`![image](${data.url})`, "");
          toast.success(t("article.saveSuccess"));
        } catch { toast.error(t("article.saveFailed")); }
        break;
      }
    }
  };

  const handleSubmit = async (status?: "draft" | "published" | "archived") => {
    if (!form.title.trim()) { toast.error(t("article.title") + " " + t("common.error")); return; }
    setSaving(true);
    setSaveSuccess(false);
    try {
      const data: UpdateArticleInput = {
        title: form.title, slug: form.slug, content: form.content,
        status: status || form.status, category_id: form.category_id,
        tag_ids: selectedTags, thumbnail: form.thumbnail,
        is_pinned: form.is_pinned, pin_order: form.pin_order,
      };
      await articlesApi.update(articleId, data);
      const newForm = { ...form, status: status || form.status };
      setForm(newForm);
      setLastSavedContent(JSON.stringify({ ...newForm, selectedTags }));
      setHasUnsavedChanges(false);
      setSaveSuccess(true);
      setTimeout(() => setSaveSuccess(false), 2000);
      toast.success(status === "published" ? t("article.publishSuccess") : t("article.saveSuccess"));
    } catch (error) { toast.error(t("article.saveFailed")); }
    finally { setSaving(false); }
  };

  const handleDelete = async () => {
    try {
      await articlesApi.delete(articleId);
      toast.success(t("article.deleteSuccess"));
      navigate("/manage/articles");
    } catch (error) { toast.error(t("article.deleteFailed")); }
  };

  const fetchPreview = async () => {
    if (!form.content.trim()) { setPreviewHtml("<p class='text-muted-foreground'>暂无内容</p>"); return; }
    try {
      const response = await fetch("/api/v1/site/render", {
        method: "POST", headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content: form.content }),
      });
      if (response.ok) { const data = await response.json(); setPreviewHtml(data.html); }
      else { setPreviewHtml(`<p>${form.content.replace(/\n/g, "<br>")}</p>`); }
    } catch { setPreviewHtml(`<p>${form.content.replace(/\n/g, "<br>")}</p>`); }
  };

  const togglePreview = () => { if (!preview) fetchPreview(); setPreview(!preview); };

  if (loading) {
    return (
      <div className="space-y-6">
        <Skeleton className="h-10 w-[200px]" />
        <div className="grid gap-6 lg:grid-cols-3">
          <div className="lg:col-span-2 space-y-4">
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-10 w-full" />
            <Skeleton className="h-[400px] w-full" />
          </div>
          <div className="space-y-4">
            <Skeleton className="h-[120px] w-full" />
            <Skeleton className="h-[120px] w-full" />
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-4">
          <Button variant="ghost" size="icon" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-4 w-4" />
          </Button>
          <div>
            <h1 className="text-3xl font-bold">{t("article.editArticle")}</h1>
            <p className="text-muted-foreground">{t("article.useMarkdown")}</p>
          </div>
        </div>
        <div className="flex items-center gap-2">
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button variant="outline" size="icon">
                <Trash2 className="h-4 w-4 text-destructive" />
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
                <AlertDialogDescription>
                  {t("article.confirmDelete", { title: form.title })}
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
                <AlertDialogAction onClick={handleDelete}>{t("common.delete")}</AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
          <Button variant="outline" onClick={togglePreview}>
            <Eye className="h-4 w-4 mr-2" />
            {preview ? t("common.edit") : t("article.preview")}
          </Button>
          <Button variant="outline" onClick={() => handleSubmit("draft")} disabled={saving}>
            {saving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSuccess ? <Check className="h-4 w-4 mr-2 text-green-500" /> : <Save className="h-4 w-4 mr-2" />}
            {t("article.saveDraft")}
            {hasUnsavedChanges && <span className="ml-1 text-amber-500">•</span>}
          </Button>
          <Button onClick={() => handleSubmit("published")} disabled={saving}>
            {saving && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            {t("article.publish")}
          </Button>
        </div>
      </div>

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2 space-y-4">
          <div className="space-y-2">
            <Label htmlFor="title">{t("article.title")}</Label>
            <Input id="title" placeholder={t("article.title")} value={form.title} onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="slug">{t("article.slug")}</Label>
            <Input id="slug" placeholder="url-friendly-slug" value={form.slug} onChange={(e) => setForm((f) => ({ ...f, slug: e.target.value }))} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="content">{t("article.content")}</Label>
            {preview ? (
              <Card>
                <CardContent className="prose prose-sm dark:prose-invert max-w-none p-4 min-h-[400px]">
                  <div dangerouslySetInnerHTML={{ __html: previewHtml }} />
                </CardContent>
              </Card>
            ) : (
              <div className="space-y-2">
                <div className="flex flex-wrap gap-1 p-2 border rounded-t-md bg-muted/50">
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("**", "**")} title="Bold"><Bold className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("*", "*")} title="Italic"><Italic className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("# ")} title="H1"><Heading1 className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("## ")} title="H2"><Heading2 className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("[", "](url)")} title="Link"><Link className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={handleImageUpload} title="Image"><ImageIcon className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("- ")} title="List"><List className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("1. ")} title="Ordered"><ListOrdered className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("> ")} title="Quote"><Quote className="h-4 w-4" /></Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("`", "`")} title="Code"><Code className="h-4 w-4" /></Button>
                  <EmojiPicker onSelect={(emoji) => insertMarkdown(emoji)} />
                  {pluginButtons.length > 0 && (
                    <>
                      <div className="w-px h-6 bg-border mx-1" />
                      {pluginButtons.map((button) => (
                        <Button key={button.id} type="button" variant="ghost" size="sm" onClick={() => insertMarkdown(button.insertBefore, button.insertAfter)} title={button.label}>
                          {button.icon ? <span className="text-base">{button.icon}</span> : <span className="text-xs font-medium">{button.label.slice(0, 2)}</span>}
                        </Button>
                      ))}
                    </>
                  )}
                </div>
                <Textarea ref={textareaRef} id="content" placeholder={t("article.useMarkdown")} value={form.content} onChange={(e) => setForm((f) => ({ ...f, content: e.target.value }))} onPaste={handlePaste} className="min-h-[400px] font-mono rounded-t-none" />
                <p className="text-xs text-muted-foreground">{t("article.pasteImage")}</p>
              </div>
            )}
          </div>
        </div>

        <div className="space-y-4">
          <Card>
            <CardHeader><CardTitle className="text-base">{t("article.category")}</CardTitle></CardHeader>
            <CardContent>
              <Select value={form.category_id.toString()} onValueChange={(v) => setForm((f) => ({ ...f, category_id: parseInt(v) }))}>
                <SelectTrigger><SelectValue placeholder={t("article.category")} /></SelectTrigger>
                <SelectContent>{categories.map((cat) => (<SelectItem key={cat.id} value={cat.id.toString()}>{cat.name}</SelectItem>))}</SelectContent>
              </Select>
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">{t("article.tags")}</CardTitle></CardHeader>
            <CardContent className="space-y-3">
              {selectedTags.length > 0 && (
                <div className="flex flex-wrap gap-2 pb-2 border-b">
                  {selectedTags.map((tagId) => { const tag = tags.find((t) => t.id === tagId); return tag ? (<Badge key={tag.id} variant="default" className="cursor-pointer" onClick={() => toggleTag(tag.id)}>{tag.name}<X className="h-3 w-3 ml-1" /></Badge>) : null; })}
                </div>
              )}
              <div className="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
                {tags.filter((tag) => !selectedTags.includes(tag.id)).map((tag) => (<Badge key={tag.id} variant="outline" className="cursor-pointer hover:bg-muted" onClick={() => toggleTag(tag.id)}>{tag.name}</Badge>))}
                {tags.length === 0 && <p className="text-sm text-muted-foreground">{t("tag.noTags")}</p>}
              </div>
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">{t("article.status")}</CardTitle></CardHeader>
            <CardContent>
              <Select value={form.status} onValueChange={(v) => setForm((f) => ({ ...f, status: v as typeof form.status }))}>
                <SelectTrigger><SelectValue /></SelectTrigger>
                <SelectContent>
                  <SelectItem value="draft">{t("article.draft")}</SelectItem>
                  <SelectItem value="published">{t("article.published")}</SelectItem>
                  <SelectItem value="archived">{t("article.archived")}</SelectItem>
                </SelectContent>
              </Select>
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base flex items-center gap-2"><Pin className="h-4 w-4" />{t("article.pinned")}</CardTitle></CardHeader>
            <CardContent className="space-y-4">
              <div className="flex items-center gap-2">
                <input type="checkbox" id="is_pinned" checked={form.is_pinned} onChange={(e) => setForm((f) => ({ ...f, is_pinned: e.target.checked }))} className="h-4 w-4 rounded border-gray-300" />
                <Label htmlFor="is_pinned">{t("article.enablePin")}</Label>
              </div>
              {form.is_pinned && (
                <div className="space-y-2">
                  <Label htmlFor="pin_order">{t("article.pinOrder")}</Label>
                  <Input id="pin_order" type="number" min={0} value={form.pin_order} onChange={(e) => setForm((f) => ({ ...f, pin_order: parseInt(e.target.value) || 0 }))} placeholder="0" />
                  <p className="text-xs text-muted-foreground">{t("article.pinOrderHint")}</p>
                </div>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader><CardTitle className="text-base">{t("article.thumbnail")}</CardTitle></CardHeader>
            <CardContent className="space-y-4">
              {form.thumbnail && (
                <div className="relative">
                  <img src={form.thumbnail} alt="Thumbnail" className="rounded-md object-cover w-full" />
                  <Button variant="destructive" size="icon" className="absolute top-2 right-2 h-6 w-6" onClick={() => setForm((f) => ({ ...f, thumbnail: null }))}><X className="h-3 w-3" /></Button>
                </div>
              )}
              {contentImages.length > 0 ? (
                <div className="space-y-2">
                  <Label>{t("article.selectThumbnail")}</Label>
                  <div className="grid grid-cols-3 gap-2">
                    {contentImages.map((img, idx) => (
                      <button key={idx} type="button" onClick={() => setForm((f) => ({ ...f, thumbnail: img }))} className={`relative aspect-video rounded-md overflow-hidden border-2 transition-colors ${form.thumbnail === img ? "border-primary" : "border-transparent hover:border-muted-foreground/50"}`}>
                        <img src={img} alt={`Image ${idx + 1}`} className="object-cover w-full h-full" />
                      </button>
                    ))}
                  </div>
                </div>
              ) : (
                <p className="text-sm text-muted-foreground">{t("article.noImages")}</p>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
