import { lazy, Suspense, startTransition, useActionState, useEffect, useMemo, useRef, useState } from "react";
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
import { ArrowLeft, Save, X, Loader2, Check, Clock, RotateCcw } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import type { MarkdownEditorRef } from "@/components/ui/markdown-editor";

const MarkdownEditor = lazy(() => import("@/components/ui/markdown-editor"));

function EditorFallback() {
  return <div className="h-[400px] rounded-md border border-input bg-muted/30 animate-pulse" />;
}

interface PluginEditorButton {
  id: string;
  label: string | Record<string, string>;
  icon?: string;
  insertBefore: string;
  insertAfter: string;
}

interface EnabledPluginInfo {
  id: string;
  settings: Record<string, unknown>;
  editor_config?: {
    toolbar?: PluginEditorButton[];
  };
}

const DRAFT_KEY = "noteva_new_article_draft";

type ArticleSubmitStatus = "draft" | "published";

interface ArticleFormState {
  title: string;
  slug: string;
  content: string;
  status: ArticleSubmitStatus;
  category_id: number | null;
  scheduled_at: string;
}

type SaveState =
  | { type: "idle" }
  | {
    type: "success";
    status: ArticleSubmitStatus;
    articleId: number;
    savedFingerprint: string;
    submittedAt: number;
  }
  | { type: "error"; message: string; submittedAt: number };

const INITIAL_SAVE_STATE: SaveState = { type: "idle" };

function createDraftFingerprint(form: ArticleFormState, selectedTags: number[]) {
  return JSON.stringify({
    ...form,
    selectedTags,
  });
}

export default function NewArticlePage() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const editorRef = useRef<MarkdownEditorRef>(null);
  const [categories, setCategories] = useState<Category[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const [pluginButtons, setPluginButtons] = useState<PluginEditorButton[]>([]);
  const [dataReady, setDataReady] = useState(false);
  const [savedFingerprint, setSavedFingerprint] = useState("");
  const [hasDraftRecovery, setHasDraftRecovery] = useState(false);

  const [form, setForm] = useState<ArticleFormState>({
    title: "",
    slug: "",
    content: "",
    status: "draft",
    category_id: null as number | null,
    scheduled_at: "",
  });

  const draftFingerprint = useMemo(() => createDraftFingerprint(form, selectedTags), [form, selectedTags]);
  const hasDraftContent = !!(form.title.trim() || form.content.trim());
  const hasUnsavedChanges = hasDraftContent && draftFingerprint !== savedFingerprint;
  const canSubmit = !!(form.title.trim() && form.content.trim() && form.category_id);

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

  // Recover draft from localStorage on mount
  useEffect(() => {
    try {
      const saved = localStorage.getItem(DRAFT_KEY);
      if (saved) {
        const draft = JSON.parse(saved) as Partial<ArticleFormState>;
        if (draft.title || draft.content) {
          setHasDraftRecovery(true);
        }
      }
    } catch { }
  }, []);

  const recoverDraft = () => {
    try {
      const saved = localStorage.getItem(DRAFT_KEY);
      if (saved) {
        const draft = JSON.parse(saved) as Partial<ArticleFormState> & { selectedTags?: number[] };
        setForm((f) => ({ ...f, ...draft }));
        if (draft.selectedTags) setSelectedTags(draft.selectedTags);
        setSavedFingerprint("");
        // Push content into CodeMirror editor (it doesn't react to form.content changes)
        if (draft.content && editorRef.current) {
          editorRef.current.setValue(draft.content);
        }
        setHasDraftRecovery(false);
        toast.success(t("article.draftRecovered"));
      }
    } catch { }
  };

  const dismissDraft = () => {
    localStorage.removeItem(DRAFT_KEY);
    setHasDraftRecovery(false);
  };

  // Auto-save to localStorage every 5 seconds
  useEffect(() => {
    if (!hasUnsavedChanges) return;
    const timer = setTimeout(() => {
      try {
        const editorContent = editorRef.current?.getValue() || form.content;
        localStorage.setItem(DRAFT_KEY, JSON.stringify({
          title: form.title, slug: form.slug, content: editorContent,
          category_id: form.category_id, scheduled_at: form.scheduled_at,
          selectedTags,
        }));
      } catch { }
    }, 5000);
    return () => clearTimeout(timer);
  }, [hasUnsavedChanges, form, selectedTags]);

  const [saveState, saveArticle, isSaving] = useActionState<SaveState, ArticleSubmitStatus>(
    async (_prevState, status) => {
      const editorContent = editorRef.current?.getValue() ?? form.content;
      const currentForm: ArticleFormState = { ...form, content: editorContent, status };

      if (!currentForm.title.trim()) {
        return { type: "error", message: t("article.title"), submittedAt: Date.now() };
      }
      if (!currentForm.content.trim()) {
        return { type: "error", message: t("article.content"), submittedAt: Date.now() };
      }
      if (!currentForm.category_id) {
        return { type: "error", message: t("article.category"), submittedAt: Date.now() };
      }

      try {
        const data: CreateArticleInput = {
          title: currentForm.title,
          slug: currentForm.slug || generateSlug(currentForm.title),
          content: currentForm.content,
          status,
          category_id: currentForm.category_id,
          tag_ids: selectedTags,
          scheduled_at: currentForm.scheduled_at ? new Date(currentForm.scheduled_at).toISOString() : undefined,
        };
        const response = await articlesApi.create(data);
        setForm(currentForm);
        return {
          type: "success",
          status,
          articleId: response.data.id,
          savedFingerprint: createDraftFingerprint(currentForm, selectedTags),
          submittedAt: Date.now(),
        };
      } catch {
        return { type: "error", message: t("article.saveFailed"), submittedAt: Date.now() };
      }
    },
    INITIAL_SAVE_STATE
  );

  useEffect(() => {
    if (saveState.type === "error") {
      toast.error(saveState.message);
      return;
    }

    if (saveState.type !== "success") return;

    setSavedFingerprint(saveState.savedFingerprint);
    localStorage.removeItem(DRAFT_KEY);
    toast.success(saveState.status === "published" ? t("article.publishSuccess") : t("article.saveSuccess"));

    const timer = window.setTimeout(() => {
      if (saveState.status === "published") {
        navigate("/manage/articles");
      } else {
        navigate(`/manage/articles/${saveState.articleId}`);
      }
    }, 1000);

    return () => window.clearTimeout(timer);
  }, [navigate, saveState, t]);

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
      .catch(() => { toast.error(t("error.loadFailed")); setDataReady(true); });
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

  const handleSubmit = (status: ArticleSubmitStatus) => {
    if (isSaving) return;
    startTransition(() => {
      saveArticle(status);
    });
  };

  const saveSucceeded = saveState.type === "success" && !hasUnsavedChanges;

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
          <Button variant="outline" onClick={() => handleSubmit("draft")} disabled={isSaving || !canSubmit}>
            {isSaving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSucceeded ? <Check className="h-4 w-4 mr-2 text-green-500" /> : <Save className="h-4 w-4 mr-2" />}
            {t("article.saveDraft")}
          </Button>
          <Button onClick={() => handleSubmit("published")} disabled={isSaving || !canSubmit}>
            {isSaving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSucceeded ? <Check className="h-4 w-4 mr-2" /> : null}
            {t("article.publish")}
          </Button>
        </div>
      </div>

      {hasDraftRecovery && (
        <div className="flex items-center justify-between p-3 rounded-lg bg-amber-500/10 border border-amber-500/30">
          <div className="flex items-center gap-2 text-sm text-amber-700 dark:text-amber-400">
            <RotateCcw className="h-4 w-4" />
            {t("article.draftFound")}
          </div>
          <div className="flex gap-2">
            <Button variant="outline" size="sm" onClick={dismissDraft}>{t("common.dismiss")}</Button>
            <Button size="sm" onClick={recoverDraft}>{t("article.recoverDraft")}</Button>
          </div>
        </div>
      )}

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2 space-y-4">
          <div className="space-y-2">
            <Label htmlFor="title">{t("article.title")}</Label>
            <Input id="title" placeholder={t("article.title")} value={form.title} onChange={(e) => handleTitleChange(e.target.value)} />
          </div>
          <div className="space-y-2">
            <Label htmlFor="slug">{t("common.slug")}</Label>
            <Input id="slug" placeholder="url-friendly-slug" value={form.slug} onChange={(e) => setForm((f) => ({ ...f, slug: e.target.value }))} />
          </div>
          <div className="space-y-2">
            <Label>{t("article.content")}</Label>
            {dataReady ? (
              <Suspense fallback={<EditorFallback />}>
                <MarkdownEditor
                  ref={editorRef}
                  initialValue=""
                  onChange={(value: string) => setForm((f) => ({ ...f, content: value }))}
                  pluginButtons={pluginButtons}
                  placeholder={t("article.useMarkdown")}
                  minHeight={400}
                />
              </Suspense>
            ) : (
              <EditorFallback />
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

          <Card>
            <CardHeader><CardTitle className="text-base flex items-center gap-2"><Clock className="h-4 w-4" />{t("article.scheduledPublish")}</CardTitle></CardHeader>
            <CardContent className="space-y-2">
              <Input
                type="datetime-local"
                value={form.scheduled_at}
                onChange={(e) => setForm((f) => ({ ...f, scheduled_at: e.target.value }))}
              />
              {form.scheduled_at && (
                <div className="flex items-center justify-between">
                  <p className="text-xs text-muted-foreground">{t("article.scheduledHint")}</p>
                  <Button variant="ghost" size="sm" onClick={() => setForm((f) => ({ ...f, scheduled_at: "" }))}>
                    <X className="h-3 w-3" />
                  </Button>
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </div>
    </div>
  );
}
