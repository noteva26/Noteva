import type { PluginEditorButton } from "@/components/ui/markdown-editor";
import { getApiErrorMessage } from "@/lib/api-error";
import type { ArticleAiTask } from "@/lib/ai-prompts";
import type { AiAssistantAction } from "./ai-assistant-dialog";
import { SUMMARY_MAX_LENGTH } from "./article-summary-field";
import type { WritingStatus } from "./writing-status-bar";

type TranslateFn = (key: string, params?: Record<string, string | number>) => string;

export type ArticleSubmitStatus = "draft" | "published";
export type ArticleContentAiTask = Extract<
  ArticleAiTask,
  "format_markdown" | "improve_writing"
>;

export interface PendingAiContentChange {
  task: ArticleAiTask;
  original: string;
  result: string;
}

interface EnabledPluginInfo {
  editor_config?: {
    toolbar?: PluginEditorButton[];
  };
}

export function getArticleAiErrorMessage(error: unknown) {
  return getApiErrorMessage(error, "AI assistant is not configured or request failed");
}

export function createWritingFingerprint<TForm extends object>(
  form: TForm,
  selectedTags: number[]
) {
  return JSON.stringify({
    ...(form as Record<string, unknown>),
    selectedTags,
  });
}

export function generateArticleSlug(title: string) {
  return title
    .toLowerCase()
    .replace(/[^a-z0-9\u4e00-\u9fa5-]+/g, "-")
    .replace(/-+/g, "-")
    .replace(/^-|-$/g, "");
}

export function isArticleContentAiTask(
  task: ArticleAiTask
): task is ArticleContentAiTask {
  return task === "format_markdown" || task === "improve_writing";
}

export function collectPluginEditorButtons(plugins: unknown): PluginEditorButton[] {
  if (!Array.isArray(plugins)) return [];

  return plugins.flatMap((plugin) => {
    const toolbar = (plugin as EnabledPluginInfo).editor_config?.toolbar;
    return Array.isArray(toolbar) ? toolbar : [];
  });
}

export function getArticleSummaryLabels(t: TranslateFn, summary: string) {
  return {
    title: t("article.summary"),
    placeholder: t("article.summaryPlaceholder"),
    hint: t("article.summaryHint"),
    suggestedRange: t("article.summarySuggestedRange"),
    count: t("article.summaryCount", {
      count: summary.trim().length,
      max: SUMMARY_MAX_LENGTH,
    }),
    generate: t("article.generateSummary"),
    clear: t("article.clearSummary"),
  };
}

export function getWritingStatusLabel(
  t: TranslateFn,
  status: WritingStatus,
  idleLabel: string
) {
  if (status === "saving") return t("article.saving");
  if (status === "saved") return t("article.saved");
  if (status === "error") return t("article.saveFailed");
  if (status === "unsaved") return t("article.unsavedChanges");
  return idleLabel;
}

export function getAiAssistantActions(t: TranslateFn): AiAssistantAction[] {
  return [
    {
      task: "title",
      label: t("article.aiActionTitle"),
      description: t("article.aiActionTitleDesc"),
      boundary: t("article.aiBoundaryField"),
      tone: "safe",
    },
    {
      task: "slug",
      label: t("article.aiActionSlug"),
      description: t("article.aiActionSlugDesc"),
      boundary: t("article.aiBoundaryField"),
      tone: "safe",
    },
    {
      task: "summary",
      label: t("article.aiActionSummary"),
      description: t("article.aiActionSummaryDesc"),
      boundary: t("article.aiBoundaryField"),
      tone: "safe",
    },
    {
      task: "format_markdown",
      label: t("article.aiActionFormatMarkdown"),
      description: t("article.aiActionFormatMarkdownDesc"),
      boundary: t("article.aiBoundaryReview"),
      tone: "review",
    },
    {
      task: "improve_writing",
      label: t("article.aiActionImproveWriting"),
      description: t("article.aiActionImproveWritingDesc"),
      boundary: t("article.aiBoundaryReview"),
      tone: "review",
    },
  ];
}
