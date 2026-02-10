import { useState, useEffect, useRef, useCallback } from "react";
import { useNavigate } from "react-router-dom";
import {
  articlesApi,
  categoriesApi,
  tagsApi,
  uploadApi,
  Category,
  Tag,
  CreateArticleInput,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Badge } from "@/components/ui/badge";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { ArrowLeft, Save, Eye, X, Bold, Italic, Link, Image as ImageIcon, List, ListOrdered, Quote, Code, Heading1, Heading2, Smile, Loader2, Check } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { EmojiPicker } from "@/components/ui/emoji-picker";

export default function NewArticlePage() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const [saving, setSaving] = useState(false);
  const [saveSuccess, setSaveSuccess] = useState(false);
  const [preview, setPreview] = useState(false);
  const [previewHtml, setPreviewHtml] = useState("");
  const [categories, setCategories] = useState<Category[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const textareaRef = useRef<HTMLTextAreaElement>(null);
  
  // Track unsaved changes
  const [hasUnsavedChanges, setHasUnsavedChanges] = useState(false);

  const [form, setForm] = useState({
    title: "",
    slug: "",
    content: "",
    status: "draft" as "draft" | "published",
    category_id: null as number | null,
  });

  // Track changes for unsaved warning
  useEffect(() => {
    const hasContent = !!(form.title.trim() || form.content.trim());
    setHasUnsavedChanges(hasContent);
  }, [form.title, form.content]);

  // Warn before leaving with unsaved changes
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
    Promise.all([categoriesApi.list(), tagsApi.list()])
      .then(([catRes, tagRes]) => {
        // 鍚庣杩斿洖 { categories: [...] } 鍜?{ tags: [...] }
        const catData = catRes.data?.categories || [];
        const tagData = tagRes.data?.tags || [];
        setCategories(Array.isArray(catData) ? catData : []);
        setTags(Array.isArray(tagData) ? tagData : []);
        // 鎵惧埌榛樿鍒嗙被 (uncategorized) 鎴栦娇鐢ㄧ涓€涓?
        const defaultCat = catData.find((c: Category) => c.slug === "uncategorized") || catData[0];
        if (defaultCat) {
          setForm((f) => ({ ...f, category_id: defaultCat.id }));
        }
      })
      .catch(() => toast.error("Failed to load data"));
  }, []); // 绉婚櫎 t 渚濊禆

  const generateSlug = (title: string) => {
    return title
      .toLowerCase()
      .replace(/[^a-z0-9\u4e00-\u9fa5\-]+/g, "-")
      .replace(/-+/g, "-")
      .replace(/^-|-$/g, "");
  };

  const handleTitleChange = (title: string) => {
    setForm((f) => ({
      ...f,
      title,
      slug: generateSlug(title),
    }));
  };

  const toggleTag = (tagId: number) => {
    setSelectedTags((prev) =>
      prev.includes(tagId)
        ? prev.filter((id) => id !== tagId)
        : [...prev, tagId]
    );
  };

  const handleSubmit = async (status: "draft" | "published") => {
    if (!form.title.trim()) {
      toast.error(t("article.title"));
      return;
    }
    if (!form.content.trim()) {
      toast.error(t("article.content"));
      return;
    }
    if (!form.category_id) {
      toast.error(t("article.category"));
      return;
    }

    setSaving(true);
    setSaveSuccess(false);
    try {
      const data: CreateArticleInput = {
        title: form.title,
        slug: form.slug || generateSlug(form.title),
        content: form.content,
        status,
        category_id: form.category_id,
        tag_ids: selectedTags,
      };
      const response = await articlesApi.create(data);
      
      // Show success animation
      setSaveSuccess(true);
      setHasUnsavedChanges(false);
      
      toast.success(status === "published" ? t("article.publishSuccess") : t("article.saveSuccess"));
      
      // Redirect to edit page for the new article
      setTimeout(() => {
        navigate(`/manage/articles/${response.data.id}`);
      }, 500);
    } catch (error) {
      toast.error(t("article.saveFailed"));
    } finally {
      setSaving(false);
    }
  };

  // Insert text at cursor position
  const insertMarkdown = useCallback((before: string, after: string = "") => {
    const textarea = textareaRef.current;
    if (!textarea) return;

    const start = textarea.selectionStart;
    const end = textarea.selectionEnd;
    const content = form.content;
    const selected = content.substring(start, end);
    const newContent = content.substring(0, start) + before + selected + after + content.substring(end);
    
    setForm((f) => ({ ...f, content: newContent }));
    
    // Restore cursor position after state update
    setTimeout(() => {
      textarea.focus();
      textarea.setSelectionRange(start + before.length, start + before.length + selected.length);
    }, 0);
  }, [form.content]);

  // Handle image upload
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
      } catch {
        toast.error(t("article.saveFailed"));
      }
    };
    input.click();
  };

  // Handle paste for images
  const handlePaste = useCallback(async (e: React.ClipboardEvent) => {
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
        } catch {
          toast.error(t("article.saveFailed"));
        }
        break;
      }
    }
  }, [insertMarkdown, t]);

  // Fetch preview HTML from backend (with shortcode processing)
  const fetchPreview = async () => {
    if (!form.content.trim()) {
      setPreviewHtml("<p class='text-muted-foreground'>鏆傛棤鍐呭</p>");
      return;
    }
    try {
      // Use the render endpoint to get processed HTML
      const response = await fetch("/api/v1/site/render", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ content: form.content }),
      });
      if (response.ok) {
        const data = await response.json();
        setPreviewHtml(data.html);
      } else {
        // Fallback: just show raw content
        setPreviewHtml(`<p>${form.content.replace(/\n/g, "<br>")}</p>`);
      }
    } catch {
      setPreviewHtml(`<p>${form.content.replace(/\n/g, "<br>")}</p>`);
    }
  };

  // Toggle preview mode
  const togglePreview = () => {
    if (!preview) {
      fetchPreview();
    }
    setPreview(!preview);
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
          <Button variant="outline" onClick={togglePreview}>
            <Eye className="h-4 w-4 mr-2" />
            {preview ? t("common.edit") : t("article.preview")}
          </Button>
          <Button variant="outline" onClick={() => handleSubmit("draft")} disabled={saving}>
            {saving ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : saveSuccess ? (
              <Check className="h-4 w-4 mr-2 text-green-500" />
            ) : (
              <Save className="h-4 w-4 mr-2" />
            )}
            {t("article.saveDraft")}
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
            <Input
              id="title"
              placeholder={t("article.title")}
              value={form.title}
              onChange={(e) => handleTitleChange(e.target.value)}
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="slug">Slug</Label>
            <Input
              id="slug"
              placeholder="url-friendly-slug"
              value={form.slug}
              onChange={(e) => setForm((f) => ({ ...f, slug: e.target.value }))}
            />
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
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("**", "**")} title="Bold">
                    <Bold className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("*", "*")} title="Italic">
                    <Italic className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("# ")} title="Heading 1">
                    <Heading1 className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("## ")} title="Heading 2">
                    <Heading2 className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("[", "](url)")} title="Link">
                    <Link className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={handleImageUpload} title="Image">
                    <ImageIcon className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("- ")} title="List">
                    <List className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("1. ")} title="Ordered List">
                    <ListOrdered className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("> ")} title="Quote">
                    <Quote className="h-4 w-4" />
                  </Button>
                  <Button type="button" variant="ghost" size="sm" onClick={() => insertMarkdown("`", "`")} title="Code">
                    <Code className="h-4 w-4" />
                  </Button>
                  <EmojiPicker onSelect={(emoji) => insertMarkdown(emoji)} />
                </div>
                <Textarea
                  ref={textareaRef}
                  id="content"
                  placeholder={t("article.useMarkdown")}
                  value={form.content}
                  onChange={(e) => setForm((f) => ({ ...f, content: e.target.value }))}
                  onPaste={handlePaste}
                  className="min-h-[400px] font-mono rounded-t-none"
                />
                <p className="text-xs text-muted-foreground">{t("article.pasteImage")}</p>
              </div>
            )}
          </div>
        </div>

        <div className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle className="text-base">{t("article.category")}</CardTitle>
            </CardHeader>
            <CardContent>
              <Select
                value={form.category_id?.toString() || ""}
                onValueChange={(v) => setForm((f) => ({ ...f, category_id: parseInt(v) }))}
              >
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
            <CardHeader>
              <CardTitle className="text-base">{t("article.tags")}</CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              {/* Selected tags */}
              {selectedTags.length > 0 && (
                <div className="flex flex-wrap gap-2 pb-2 border-b">
                  {selectedTags.map((tagId) => {
                    const tag = tags.find((t) => t.id === tagId);
                    return tag ? (
                      <Badge
                        key={tag.id}
                        variant="default"
                        className="cursor-pointer"
                        onClick={() => toggleTag(tag.id)}
                      >
                        {tag.name}
                        <X className="h-3 w-3 ml-1" />
                      </Badge>
                    ) : null;
                  })}
                </div>
              )}
              {/* Available tags */}
              <div className="flex flex-wrap gap-2 max-h-32 overflow-y-auto">
                {tags
                  .filter((tag) => !selectedTags.includes(tag.id))
                  .map((tag) => (
                    <Badge
                      key={tag.id}
                      variant="outline"
                      className="cursor-pointer hover:bg-muted"
                      onClick={() => toggleTag(tag.id)}
                    >
                      {tag.name}
                    </Badge>
                  ))}
                {tags.length === 0 && (
                  <p className="text-sm text-muted-foreground">{t("tag.noTags")}</p>
                )}
              </div>
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}

