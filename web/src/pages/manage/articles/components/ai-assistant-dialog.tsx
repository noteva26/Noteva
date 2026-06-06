import { Loader2, Sparkles } from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@/components/ui/accordion";
import {
  ARTICLE_AI_TASK_PROMPT_KEYS,
  DEFAULT_ARTICLE_AI_PROMPTS,
  type ArticleAiPromptSettings,
  type ArticleAiTask,
} from "@/lib/ai-prompts";
import { cn } from "@/lib/utils";

interface AiAssistantAction {
  task: ArticleAiTask;
  label: string;
  description: string;
  boundary: string;
  tone: "safe" | "review";
}

interface AiAssistantDialogProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  promptSettings: ArticleAiPromptSettings;
  pendingTask: ArticleAiTask | null;
  onRun: (task: ArticleAiTask) => void;
  labels: {
    title: string;
    description: string;
    endpoint: string;
    model: string;
    defaultPrompt: string;
    currentPrompt: string;
    systemPrompt: string;
    run: string;
  };
  actions: AiAssistantAction[];
}

function PromptPreview({
  label,
  value,
  muted,
}: {
  label: string;
  value: string;
  muted?: boolean;
}) {
  return (
    <div className="space-y-1">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <pre
        className={cn(
          "max-h-48 overflow-auto rounded-md border p-3 text-xs leading-relaxed whitespace-pre-wrap",
          muted ? "bg-muted/25 text-muted-foreground" : "bg-muted/40 text-foreground"
        )}
      >
        {value || " "}
      </pre>
    </div>
  );
}

export function AiAssistantDialog({
  open,
  onOpenChange,
  promptSettings,
  pendingTask,
  onRun,
  labels,
  actions,
}: AiAssistantDialogProps) {
  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-h-[88vh] overflow-hidden sm:max-w-5xl">
        <DialogHeader>
          <DialogTitle className="flex items-center gap-2">
            <Sparkles className="h-5 w-5" />
            {labels.title}
          </DialogTitle>
          <DialogDescription>{labels.description}</DialogDescription>
        </DialogHeader>

        <div className="grid min-h-0 gap-4 overflow-hidden lg:grid-cols-[minmax(0,1fr)_minmax(320px,0.78fr)]">
          <div className="min-h-0 overflow-auto pr-1">
            <div className="space-y-3">
              {actions.map((action) => {
                const running = pendingTask === action.task;
                return (
                  <div
                    key={action.task}
                    className="rounded-lg border bg-card p-4 text-card-foreground"
                  >
                    <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
                      <div className="min-w-0 space-y-2">
                        <div className="flex flex-wrap items-center gap-2">
                          <h3 className="font-medium">{action.label}</h3>
                          <Badge
                            variant={action.tone === "safe" ? "secondary" : "warning"}
                            className={action.tone === "safe" ? "" : "text-xs"}
                          >
                            {action.boundary}
                          </Badge>
                        </div>
                        <p className="text-sm text-muted-foreground">
                          {action.description}
                        </p>
                      </div>
                      <Button
                        type="button"
                        size="sm"
                        onClick={() => onRun(action.task)}
                        disabled={!!pendingTask}
                      >
                        {running ? (
                          <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                        ) : (
                          <Sparkles className="mr-2 h-4 w-4" />
                        )}
                        {labels.run}
                      </Button>
                    </div>
                  </div>
                );
              })}
            </div>
          </div>

          <div className="min-h-0 overflow-auto rounded-lg border bg-muted/15 p-4">
            <div className="mb-3 flex flex-wrap gap-2 text-xs">
              <Badge variant="outline">
                {labels.endpoint}: {promptSettings.provider}
              </Badge>
              {promptSettings.model ? (
                <Badge variant="outline">
                  {labels.model}: {promptSettings.model}
                </Badge>
              ) : null}
            </div>

            <Accordion type="single" collapsible defaultValue="system">
              <AccordionItem value="system">
                <AccordionTrigger className="py-3 text-sm">
                  {labels.systemPrompt}
                </AccordionTrigger>
                <AccordionContent className="space-y-3">
                  <PromptPreview
                    label={labels.currentPrompt}
                    value={promptSettings.system}
                  />
                  <PromptPreview
                    label={labels.defaultPrompt}
                    value={DEFAULT_ARTICLE_AI_PROMPTS.system}
                    muted
                  />
                </AccordionContent>
              </AccordionItem>
              {actions.map((action) => {
                const promptKey = ARTICLE_AI_TASK_PROMPT_KEYS[action.task];
                return (
                  <AccordionItem key={action.task} value={action.task}>
                    <AccordionTrigger className="py-3 text-sm">
                      {action.label}
                    </AccordionTrigger>
                    <AccordionContent className="space-y-3">
                      <PromptPreview
                        label={labels.currentPrompt}
                        value={promptSettings[promptKey]}
                      />
                      <PromptPreview
                        label={labels.defaultPrompt}
                        value={DEFAULT_ARTICLE_AI_PROMPTS[promptKey]}
                        muted
                      />
                    </AccordionContent>
                  </AccordionItem>
                );
              })}
            </Accordion>
          </div>
        </div>
      </DialogContent>
    </Dialog>
  );
}

export type { AiAssistantAction };
