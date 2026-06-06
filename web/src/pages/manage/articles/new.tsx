import { lazy, Suspense, startTransition, useActionState, useEffect, useMemo, useRef, useState } from "react";
import { useNavigate } from "react-router-dom";
import {
  articlesApi,
  categoriesApi,
  tagsApi,
  adminApi,
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
import { ArrowLeft, Save, X, Loader2, Check, Clock, RotateCcw, Wand2 } from "lucide-react";
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
  generateArticleSlug,
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
  return <div className="h-[400px] rounded-md border border-input bg-muted/30 animate-pulse" />;
}

const DRAFT_KEY = "noteva_new_article_draft";

type AiTask = ArticleAiTask;

interface ArticleFormState {
  title: string;
  slug: string;
  summary: string;
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
  const [localDraftSavedAt, setLocalDraftSavedAt] = useState<number | null>(null);
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
    category_id: null as number | null,
    scheduled_at: "",
  });

  const getEditorContent = () => editorRef.current?.getValue() ?? form.content;
  const draftFingerprint = useMemo(
    () => createWritingFingerprint({ ...form, content: getEditorContent() }, selectedTags),
    [form, selectedTags, contentRevision]
  );
  const hasDraftContent = !!(form.title.trim() || getEditorContent().trim());
  const hasUnsavedChanges = hasDraftContent && draftFingerprint !== savedFingerprint;
  const canSubmit = !!(form.title.trim() && getEditorContent().trim() && form.category_id);
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
        if (draft.content !== undefined) setContentRevision((value) => value + 1);
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
        const editorContent = getEditorContent();
        localStorage.setItem(DRAFT_KEY, JSON.stringify({
          title: form.title, slug: form.slug, summary: form.summary, content: editorContent,
          category_id: form.category_id, scheduled_at: form.scheduled_at,
          selectedTags,
        }));
        setLocalDraftSavedAt(Date.now());
      } catch { }
    }, 5000);
    return () => clearTimeout(timer);
  }, [hasUnsavedChanges, form, selectedTags, contentRevision]);

  const [saveState, saveArticle, isSaving] = useActionState<SaveState, ArticleSubmitStatus>(
    async (_prevState, status) => {
      const editorContent = getEditorContent();
      const submitStatus: ArticleSubmitStatus = form.scheduled_at ? "draft" : status;
      const currentForm: ArticleFormState = { ...form, content: editorContent, status: submitStatus };

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
          slug: currentForm.slug || generateArticleSlug(currentForm.title),
          content: currentForm.content,
          summary: currentForm.summary,
          status: submitStatus,
          category_id: currentForm.category_id,
          tag_ids: selectedTags,
          scheduled_at: currentForm.scheduled_at ? new Date(currentForm.scheduled_at).toISOString() : undefined,
        };
        const response = await articlesApi.create(data);
        setForm(currentForm);
        return {
          type: "success",
          status: submitStatus,
          articleId: response.data.id,
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
    let active = true;

    const loadData = async () => {
      try {
        const [catRes, tagRes, pluginsRes, settingsRes] = await Promise.all([
          categoriesApi.list(),
          tagsApi.list(),
          fetch("/api/v1/plugins/enabled").then(r => r.json()).catch(() => []),
          adminApi.getSettings().catch(() => null),
        ]);
        if (!active) return;

        const catData = catRes.data?.categories || [];
        const tagData = tagRes.data?.tags || [];
        setCategories(Array.isArray(catData) ? catData : []);
        setTags(Array.isArray(tagData) ? tagData : []);
        const defaultCat = catData.find((c: Category) => c.slug === "uncategorized") || catData[0];
        if (defaultCat) {
          setForm((f) => ({ ...f, category_id: f.category_id || defaultCat.id }));
        }
        setPluginButtons(collectPluginEditorButtons(pluginsRes));
        setAiPromptSettings(getArticleAiPromptSettings(settingsRes?.data));
        setDataReady(true);
      } catch {
        if (!active) return;
        toast.error(t("error.loadFailed"));
        setDataReady(true);
      }
    };

    void loadData();

    return () => {
      active = false;
    };
  }, [t]);

  const handleTitleChange = (title: string) => {
    setForm((f) => ({ ...f, title, slug: generateArticleSlug(title) }));
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
      setForm((f) => ({ ...f, title: result, slug: f.slug || generateArticleSlug(result) }));
    } else if (task === "slug") {
      setForm((f) => ({ ...f, slug: result }));
    } else if (task === "summary") {
      setForm((f) => ({ ...f, summary: result }));
    }
    setPendingAiContentChange(null);
    toast.success(t("article.aiApplied"));
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
  const writingStatus: WritingStatus = isSaving
    ? "saving"
    : saveState.type === "error"
      ? "error"
      : saveSucceeded
        ? "saved"
        : hasUnsavedChanges
          ? "unsaved"
          : "idle";
  const writingStatusLabel = getWritingStatusLabel(t, writingStatus, t("article.draft"));
  const writingStatusDetail =
    writingStatus === "error" && saveState.type === "error"
      ? saveState.message
      : localDraftSavedAt && hasUnsavedChanges
        ? t("article.localDraftSaved")
        : undefined;
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
          <Button variant="outline" onClick={() => setAiAssistantOpen(true)}>
            <Wand2 className="h-4 w-4 mr-2" />
            {t("article.aiAssistant")}
          </Button>
          <Button onClick={() => handleSubmit("published")} disabled={isSaving || !canSubmit}>
            {isSaving ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : saveSucceeded ? <Check className="h-4 w-4 mr-2" /> : null}
            {form.scheduled_at ? t("article.scheduledPublish") : t("article.publish")}
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
              <Input id="title" placeholder={t("article.title")} value={form.title} onChange={(e) => handleTitleChange(e.target.value)} />
              <Button type="button" variant="outline" size="icon" onClick={() => requestAiRun("title")} title="AI">
                <Wand2 className="h-4 w-4" />
              </Button>
            </div>
          </div>
          <div className="space-y-2">
            <Label htmlFor="slug">{t("common.slug")}</Label>
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
            {dataReady ? (
              <Suspense fallback={<EditorFallback />}>
                <MarkdownEditor
                  ref={editorRef}
                  initialValue={form.content}
                  onChange={scheduleContentRevision}
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
