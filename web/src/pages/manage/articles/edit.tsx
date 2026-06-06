import { lazy, Suspense, startTransition, useActionState, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate, useParams } from "react-router-dom";
import {
  articlesApi,
  categoriesApi,
  tagsApi,
  adminApi,
  Category,
  Tag,
  UpdateArticleInput,
} from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
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
import { ArrowLeft, Save, X, Trash2, Pin, Loader2, Check, Clock, Wand2 } from "lucide-react";
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
import type {
  MarkdownEditorRef,
  PluginEditorButton,
} from "@/components/ui/markdown-editor";
import {
  WritingStatusBar,
  type WritingStatus,
} from "./components/writing-status-bar";
import { ArticleSummaryField } from "./components/article-summary-field";
import { AiContentConfirmDialog } from "./components/ai-content-confirm-dialog";
import { AiRunConfirmDialog } from "./components/ai-run-confirm-dialog";
import { AiAssistantDialog } from "./components/ai-assistant-dialog";
import {
  collectPluginEditorButtons,
  createWritingFingerprint,
  getAiAssistantActions,
  getArticleAiErrorMessage,
  getArticleSummaryLabels,
  getWritingStatusLabel,
  isArticleContentAiTask,
  type ArticleSubmitStatus,
  type PendingAiContentChange,
} from "./components/writing-utils";
import {
  getArticleAiPromptSettings,
  type ArticleAiPromptSettings,
  type ArticleAiTask,
} from "@/lib/ai-prompts";

const MarkdownEditor = lazy(() => import("@/components/ui/markdown-editor"));

function EditorFallback() {
  return <Skeleton className="h-[400px] w-full" />;
}

type ArticleStatus = "draft" | "published" | "archived";
type AiTask = ArticleAiTask;

interface ArticleFormState {
  title: string;
  slug: string;
  summary: string;
  content: string;
  status: ArticleStatus;
  category_id: number;
  thumbnail: string | null;
  is_pinned: boolean;
  pin_order: number;
  scheduled_at: string;
}

type SaveState =
  | { type: "idle" }
  | {
    type: "success";
    status: ArticleSubmitStatus;
    savedFingerprint: string;
    submittedAt: number;
  }
  | { type: "error"; message: string; submittedAt: number };

const INITIAL_SAVE_STATE: SaveState = { type: "idle" };

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
  const articleId = Number.parseInt(id || "", 10);
  const hasValidArticleId = Number.isFinite(articleId) && articleId > 0;
  const { t } = useTranslation();
  const editorRef = useRef<MarkdownEditorRef>(null);

  const [loading, setLoading] = useState(true);
  const [categories, setCategories] = useState<Category[]>([]);
  const [tags, setTags] = useState<Tag[]>([]);
  const [selectedTags, setSelectedTags] = useState<number[]>([]);
  const [pluginButtons, setPluginButtons] = useState<PluginEditorButton[]>([]);

  const [lastSavedFingerprint, setLastSavedFingerprint] = useState("");
  const [autoSaveEnabled] = useState(true);
  const [autoSaveStatus, setAutoSaveStatus] = useState<"idle" | "saving" | "saved" | "error">("idle");
  const [contentRevision, setContentRevision] = useState(0);
  const contentRevisionTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const [aiPendingTask, setAiPendingTask] = useState<AiTask | null>(null);
  const [pendingAiRunTask, setPendingAiRunTask] = useState<AiTask | null>(null);
  const [pendingAiContentChange, setPendingAiContentChange] =
    useState<PendingAiContentChange | null>(null);
  const [aiAssistantOpen, setAiAssistantOpen] = useState(false);
  const [aiPromptSettings, setAiPromptSettings] =
    useState<ArticleAiPromptSettings>(() => getArticleAiPromptSettings());

  const [form, setForm] = useState<ArticleFormState>({
    title: "",
    slug: "",
    summary: "",
    content: "",
    status: "draft",
    category_id: 0,
    thumbnail: null as string | null,
    is_pinned: false,
    pin_order: 0,
    scheduled_at: "",
  });

  const getEditorContent = () => editorRef.current?.getValue() ?? form.content;
  const contentImages = useMemo(
    () => extractImages(getEditorContent()),
    [form.content, contentRevision]
  );
  const currentFingerprint = useMemo(
    () => createWritingFingerprint({ ...form, content: getEditorContent() }, selectedTags),
    [form, selectedTags, contentRevision]
  );
  const hasUnsavedChanges = !loading && !!lastSavedFingerprint && currentFingerprint !== lastSavedFingerprint;
  const summaryLabels = useMemo(
    () => getArticleSummaryLabels(t, form.summary),
    [t, form.summary]
  );
  const aiAssistantActions = useMemo(() => getAiAssistantActions(t), [t]);
  const scheduleContentRevision = () => {
    if (contentRevisionTimerRef.current) {
      clearTimeout(contentRevisionTimerRef.current);
    }
    contentRevisionTimerRef.current = setTimeout(() => {
      setContentRevision((value) => value + 1);
      contentRevisionTimerRef.current = null;
    }, 160);
  };

  useEffect(() => {
    return () => {
      if (contentRevisionTimerRef.current) {
        clearTimeout(contentRevisionTimerRef.current);
      }
    };
  }, []);

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
    if (!autoSaveEnabled || !hasUnsavedChanges || !hasValidArticleId || form.status === "published") return;

    const timer = window.setTimeout(async () => {
      const editorContent = getEditorContent();
      const currentForm = { ...form, content: editorContent };

      if (currentForm.title.trim()) {
        try {
          setAutoSaveStatus("saving");
          const data: UpdateArticleInput = {
            title: currentForm.title, slug: currentForm.slug, content: currentForm.content,
            summary: currentForm.summary,
            status: currentForm.status, category_id: currentForm.category_id,
            tag_ids: selectedTags, thumbnail: currentForm.thumbnail,
            is_pinned: currentForm.is_pinned, pin_order: currentForm.pin_order,
            scheduled_at: currentForm.scheduled_at ? new Date(currentForm.scheduled_at).toISOString() : null,
          };
          await articlesApi.update(articleId, data);
          setForm(currentForm);
          setLastSavedFingerprint(createWritingFingerprint(currentForm, selectedTags));
          setAutoSaveStatus("saved");
          toast.success(t("article.autoSaved"), { duration: 2000 });
        } catch {
          setAutoSaveStatus("error");
        }
      }
    }, 30000);

    return () => window.clearTimeout(timer);
  }, [hasUnsavedChanges, autoSaveEnabled, hasValidArticleId, articleId, form, selectedTags, t, contentRevision]);

  useEffect(() => {
    if (!hasValidArticleId) {
      setLoading(false);
      toast.error(t("error.loadFailed"));
      navigate("/manage/articles");
      return;
    }
    let active = true;

    const fetchData = async () => {
      try {
        const [catRes, tagRes, articleRes, pluginsRes, settingsRes] = await Promise.all([
          categoriesApi.list(), tagsApi.list(), articlesApi.getById(articleId),
          fetch("/api/v1/plugins/enabled").then(r => r.json()).catch(() => []),
          adminApi.getSettings().catch(() => null),
        ]);
        if (!active) return;

        setCategories(catRes.data.categories);
        setTags(tagRes.data.tags);
        setPluginButtons(collectPluginEditorButtons(pluginsRes));
        setAiPromptSettings(getArticleAiPromptSettings(settingsRes?.data));
        const article = articleRes.data;
        const formData: ArticleFormState = {
          title: article.title, slug: article.slug, content: article.content,
          summary: article.summary || article.excerpt || (article.meta?.summary as string | undefined) || "",
          status: article.status, category_id: article.category_id,
          thumbnail: article.thumbnail || null,
          is_pinned: article.is_pinned || false, pin_order: article.pin_order || 0,
          scheduled_at: article.scheduled_at ? new Date(article.scheduled_at).toISOString().slice(0, 16) : "",
        };
        const tagIds = article.tags?.map((tag) => tag.id) || [];
        setForm(formData);
        setSelectedTags(tagIds);
        setLastSavedFingerprint(createWritingFingerprint(formData, tagIds));
      } catch {
        if (!active) return;
        toast.error(t("error.loadFailed"));
        navigate("/manage/articles");
      } finally {
        if (active) setLoading(false);
      }
    };

    fetchData();
    return () => {
      active = false;
    };
  }, [hasValidArticleId, articleId, navigate, t]);

  const [saveState, saveArticle, isSaving] = useActionState<SaveState, ArticleSubmitStatus>(
    async (_prevState, status) => {
      const editorContent = getEditorContent();
      const submitStatus: ArticleSubmitStatus = form.scheduled_at ? "draft" : status;
      const currentForm: ArticleFormState = { ...form, content: editorContent, status: submitStatus };

      if (!currentForm.title.trim()) {
        return { type: "error", message: `${t("article.title")} ${t("common.error")}`, submittedAt: Date.now() };
      }

      try {
        const data: UpdateArticleInput = {
          title: currentForm.title, slug: currentForm.slug, content: currentForm.content,
          summary: currentForm.summary,
          status: submitStatus, category_id: currentForm.category_id,
          tag_ids: selectedTags, thumbnail: currentForm.thumbnail,
          is_pinned: currentForm.is_pinned, pin_order: currentForm.pin_order,
          scheduled_at: currentForm.scheduled_at ? new Date(currentForm.scheduled_at).toISOString() : null,
        };
        await articlesApi.update(articleId, data);
        setForm(currentForm);
        return {
          type: "success",
          status: submitStatus,
          savedFingerprint: createWritingFingerprint(currentForm, selectedTags),
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
      setAutoSaveStatus("error");
      toast.error(saveState.message);
      return;
    }

    if (saveState.type !== "success") return;

    setLastSavedFingerprint(saveState.savedFingerprint);
    setAutoSaveStatus("saved");
    toast.success(saveState.status === "published" ? t("article.publishSuccess") : t("article.saveSuccess"));
  }, [saveState, t]);

  const toggleTag = (tagId: number) => {
    setSelectedTags((prev) => prev.includes(tagId) ? prev.filter((id) => id !== tagId) : [...prev, tagId]);
  };

  const runAi = async (task: AiTask) => {
    const content = getEditorContent();
    setAiPendingTask(task);
    try {
      const { data } = await adminApi.aiAssist({
        task,
        title: form.title,
        slug: form.slug,
        summary: form.summary,
        content,
      });
      const result = data.result.trim();
      if (!result) return;
      setPendingAiContentChange({
        task,
        original:
          task === "title"
            ? form.title
            : task === "slug"
              ? form.slug
              : task === "summary"
                ? form.summary
                : content,
        result,
      });
    } catch (error) {
      toast.error(getArticleAiErrorMessage(error));
    } finally {
      setAiPendingTask(null);
    }
  };

  const requestAiRun = (task: AiTask) => {
    if (aiPendingTask) return;
    setPendingAiRunTask(task);
  };

  const confirmAiRun = () => {
    const task = pendingAiRunTask;
    setPendingAiRunTask(null);
    if (task) void runAi(task);
  };

  const applyAiContentChange = () => {
    if (!pendingAiContentChange) return;

    const { task, result } = pendingAiContentChange;
    if (isArticleContentAiTask(task)) {
      editorRef.current?.setValue(result);
      setForm((f) => ({ ...f, content: result }));
      setContentRevision((value) => value + 1);
    } else if (task === "title") {
      setForm((f) => ({ ...f, title: result }));
    } else if (task === "slug") {
      setForm((f) => ({ ...f, slug: result }));
    } else if (task === "summary") {
      setForm((f) => ({ ...f, summary: result }));
    }
    setPendingAiContentChange(null);
    toast.success(t("article.aiApplied"));
  };

  const handleSubmit = (status: ArticleSubmitStatus) => {
    if (isSaving) return;
    startTransition(() => {
      saveArticle(status);
    });
  };

  const handleDelete = async () => {
    try {
      await articlesApi.delete(articleId);
      toast.success(t("article.deleteSuccess"));
      navigate("/manage/articles");
    } catch (error) { toast.error(t("article.deleteFailed")); }
  };

  const saveSucceeded = saveState.type === "success" && !hasUnsavedChanges;
  const canSubmit = hasUnsavedChanges;
  const writingStatus: WritingStatus = isSaving || autoSaveStatus === "saving"
    ? "saving"
    : saveState.type === "error" || autoSaveStatus === "error"
      ? "error"
      : saveSucceeded || autoSaveStatus === "saved"
        ? "saved"
        : hasUnsavedChanges
          ? "unsaved"
          : "idle";
  const writingStatusLabel = getWritingStatusLabel(t, writingStatus, t("article.saved"));
  const writingStatusDetail =
    writingStatus === "error" && saveState.type === "error"
      ? saveState.message
      : autoSaveStatus === "saved"
        ? t("article.autoSaved")
        : undefined;
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
          <Button variant="outline" onClick={() => handleSubmit("draft")} disabled={isSaving || !canSubmit}>
            {isSaving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSucceeded ? <Check className="h-4 w-4 mr-2 text-green-500" /> : <Save className="h-4 w-4 mr-2" />}
            {t("article.saveDraft")}
          </Button>
          <Button variant="outline" onClick={() => setAiAssistantOpen(true)}>
            <Wand2 className="h-4 w-4 mr-2" />
            {t("article.aiAssistant")}
          </Button>
          <Button onClick={() => handleSubmit("published")} disabled={isSaving || !canSubmit}>
            {isSaving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSucceeded ? <Check className="h-4 w-4 mr-2" /> : null}
            {form.scheduled_at ? t("article.scheduledPublish") : form.status === "published" ? t("article.update") : t("article.publish")}
          </Button>
        </div>
      </div>

      <WritingStatusBar
        status={writingStatus}
        label={writingStatusLabel}
        detail={writingStatusDetail}
      />

      <div className="grid gap-6 lg:grid-cols-3">
        <div className="lg:col-span-2 space-y-4">
          <div className="space-y-2">
            <Label htmlFor="title">{t("article.title")}</Label>
            <div className="flex gap-2">
              <Input id="title" placeholder={t("article.title")} value={form.title} onChange={(e) => setForm((f) => ({ ...f, title: e.target.value }))} />
              <Button type="button" variant="outline" size="icon" onClick={() => requestAiRun("title")} title="AI">
                <Wand2 className="h-4 w-4" />
              </Button>
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="slug">{t("article.slug")}</Label>
            <div className="flex gap-2">
              <Input id="slug" placeholder="url-friendly-slug" value={form.slug} onChange={(e) => setForm((f) => ({ ...f, slug: e.target.value }))} />
              <Button type="button" variant="outline" size="icon" onClick={() => requestAiRun("slug")} title="AI">
                <Wand2 className="h-4 w-4" />
              </Button>
            </div>
          </div>
          <ArticleSummaryField
            id="summary"
            value={form.summary}
            onChange={(summary) => setForm((f) => ({ ...f, summary }))}
            onGenerate={() => requestAiRun("summary")}
            onClear={() => setForm((f) => ({ ...f, summary: "" }))}
            disabled={aiPendingTask === "summary"}
            labels={summaryLabels}
          />
          <div className="space-y-2">
            <div className="flex items-center justify-between gap-2">
              <Label>{t("article.content")}</Label>
              <div className="flex gap-2">
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => requestAiRun("format_markdown")}
                  disabled={aiPendingTask === "format_markdown"}
                >
                  {aiPendingTask === "format_markdown" ? (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  ) : (
                    <Wand2 className="h-4 w-4 mr-2" />
                  )}
                  {t("article.formatMarkdown")}
                </Button>
                <Button
                  type="button"
                  variant="outline"
                  size="sm"
                  onClick={() => requestAiRun("improve_writing")}
                  disabled={aiPendingTask === "improve_writing"}
                >
                  {aiPendingTask === "improve_writing" ? (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  ) : (
                    <Wand2 className="h-4 w-4 mr-2" />
                  )}
                  {t("article.improveWriting")}
                </Button>
              </div>
            </div>
            <Suspense fallback={<EditorFallback />}>
              <MarkdownEditor
                ref={editorRef}
                initialValue={form.content}
                onChange={scheduleContentRevision}
                pluginButtons={pluginButtons}
                placeholder={t("article.useMarkdown")}
                minHeight={400}
                onSetThumbnail={(thumbnail) => setForm((f) => ({ ...f, thumbnail }))}
              />
            </Suspense>
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
              <Select value={form.status} onValueChange={(v) => setForm((f) => ({ ...f, status: v as ArticleStatus }))}>
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

      <AiContentConfirmDialog
        open={!!pendingAiContentChange}
        onOpenChange={(open) => {
          if (!open) setPendingAiContentChange(null);
        }}
        title={t("article.aiContentConfirmTitle")}
        description={t("article.aiContentConfirmDescription")}
        originalLabel={t("article.aiOriginalContent")}
        resultLabel={t("article.aiResultContent")}
        cancelLabel={t("common.cancel")}
        applyLabel={t("article.applyAiResult")}
        original={pendingAiContentChange?.original ?? ""}
        result={pendingAiContentChange?.result ?? ""}
        onApply={applyAiContentChange}
      />
      <AiRunConfirmDialog
        open={!!pendingAiRunTask}
        onOpenChange={(open) => {
          if (!open) setPendingAiRunTask(null);
        }}
        title={t("article.aiRunConfirmTitle")}
        description={t("article.aiRunConfirmDescription")}
        cancelLabel={t("common.cancel")}
        runLabel={t("article.aiRun")}
        onConfirm={confirmAiRun}
      />
      <AiAssistantDialog
        open={aiAssistantOpen}
        onOpenChange={setAiAssistantOpen}
        promptSettings={aiPromptSettings}
        pendingTask={aiPendingTask}
        actions={aiAssistantActions}
        onRun={requestAiRun}
        labels={{
          title: t("article.aiAssistant"),
          description: t("article.aiAssistantDesc"),
          endpoint: t("article.aiEndpoint"),
          model: t("article.aiModel"),
          defaultPrompt: t("article.aiDefaultPrompt"),
          currentPrompt: t("article.aiCurrentPrompt"),
          systemPrompt: t("article.aiSystemPrompt"),
          run: t("article.aiRun"),
        }}
      />
    </div>
  );
}
