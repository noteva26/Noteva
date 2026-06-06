export type ArticleAiTask =
  | "title"
  | "slug"
  | "summary"
  | "format_markdown"
  | "improve_writing";

export interface ArticleAiPromptSettings {
  provider: string;
  apiBase: string;
  model: string;
  system: string;
  title: string;
  slug: string;
  summary: string;
  formatMarkdown: string;
  improveWriting: string;
}

export const DEFAULT_ARTICLE_AI_PROMPTS: ArticleAiPromptSettings = {
  provider: "openai_chat",
  apiBase: "",
  model: "",
  system:
    "You are a concise blog writing assistant. Return only the requested result, without explanations.",
  title:
    "Generate one clear blog post title for this Markdown content:\n\n{{content}}",
  slug:
    "Generate one lowercase URL slug using only letters, numbers and hyphens. Title: {{title}}\nCurrent slug: {{slug}}",
  summary:
    "Write a concise article summary in the same language as the article. Keep it under 120 Chinese characters or 80 English words.\n\nTitle: {{title}}\n\nContent:\n{{content}}",
  formatMarkdown:
    "Clean up only Markdown formatting and structure. Do not change, rewrite, add, remove, or translate any wording. Return the full Markdown only.\n\n{{content}}",
  improveWriting:
    "Improve the expression and readability of this article while preserving meaning and Markdown structure. Return the full Markdown only.\n\nTitle: {{title}}\nSummary: {{summary}}\n\nContent:\n{{content}}",
};

export const ARTICLE_AI_TASK_PROMPT_KEYS: Record<
  ArticleAiTask,
  keyof Pick<
    ArticleAiPromptSettings,
    "title" | "slug" | "summary" | "formatMarkdown" | "improveWriting"
  >
> = {
  title: "title",
  slug: "slug",
  summary: "summary",
  format_markdown: "formatMarkdown",
  improve_writing: "improveWriting",
};

function settingString(settings: Record<string, unknown>, key: string, fallback: string) {
  const value = settings[key];
  return typeof value === "string" && value.trim() ? value : fallback;
}

export function getArticleAiPromptSettings(
  settings?: Record<string, unknown> | null
): ArticleAiPromptSettings {
  if (!settings) return DEFAULT_ARTICLE_AI_PROMPTS;

  return {
    provider: settingString(settings, "ai_provider", DEFAULT_ARTICLE_AI_PROMPTS.provider),
    apiBase: settingString(settings, "ai_api_base", DEFAULT_ARTICLE_AI_PROMPTS.apiBase),
    model: settingString(settings, "ai_model", DEFAULT_ARTICLE_AI_PROMPTS.model),
    system: settingString(settings, "ai_system_prompt", DEFAULT_ARTICLE_AI_PROMPTS.system),
    title: settingString(settings, "ai_prompt_title", DEFAULT_ARTICLE_AI_PROMPTS.title),
    slug: settingString(settings, "ai_prompt_slug", DEFAULT_ARTICLE_AI_PROMPTS.slug),
    summary: settingString(settings, "ai_prompt_summary", DEFAULT_ARTICLE_AI_PROMPTS.summary),
    formatMarkdown: settingString(
      settings,
      "ai_prompt_format_markdown",
      DEFAULT_ARTICLE_AI_PROMPTS.formatMarkdown
    ),
    improveWriting: settingString(
      settings,
      "ai_prompt_improve_writing",
      DEFAULT_ARTICLE_AI_PROMPTS.improveWriting
    ),
  };
}
