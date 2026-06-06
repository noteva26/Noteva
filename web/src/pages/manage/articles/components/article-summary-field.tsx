import { Info, Wand2, X } from "lucide-react";
import { Button } from "@/components/ui/button";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { cn } from "@/lib/utils";

interface ArticleSummaryFieldProps {
  id: string;
  value: string;
  onChange: (value: string) => void;
  onGenerate: () => void;
  onClear: () => void;
  disabled?: boolean;
  labels: {
    title: string;
    placeholder: string;
    hint: string;
    suggestedRange: string;
    count: string;
    generate: string;
    clear: string;
  };
}

const SUMMARY_MAX_LENGTH = 240;
const SUMMARY_SUGGESTED_MIN = 80;
const SUMMARY_SUGGESTED_MAX = 160;

export function ArticleSummaryField({
  id,
  value,
  onChange,
  onGenerate,
  onClear,
  disabled,
  labels,
}: ArticleSummaryFieldProps) {
  const length = value.trim().length;
  const hasValue = length > 0;
  const isInSuggestedRange =
    length >= SUMMARY_SUGGESTED_MIN && length <= SUMMARY_SUGGESTED_MAX;

  return (
    <section className="rounded-lg border bg-card p-4 text-card-foreground shadow-sm transition-colors">
      <div className="flex flex-col gap-3 sm:flex-row sm:items-start sm:justify-between">
        <div className="space-y-1">
          <Label htmlFor={id} className="text-sm font-medium">
            {labels.title}
          </Label>
          <div className="flex items-start gap-2 text-xs text-muted-foreground">
            <Info className="mt-0.5 h-3.5 w-3.5 shrink-0" />
            <span>{labels.hint}</span>
          </div>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <Button
            type="button"
            variant="outline"
            size="sm"
            onClick={onGenerate}
            disabled={disabled}
          >
            <Wand2 className="mr-2 h-4 w-4" />
            {labels.generate}
          </Button>
          <Button
            type="button"
            variant="ghost"
            size="icon"
            onClick={onClear}
            disabled={!hasValue || disabled}
            title={labels.clear}
            aria-label={labels.clear}
          >
            <X className="h-4 w-4" />
          </Button>
        </div>
      </div>

      <Textarea
        id={id}
        className="mt-3 min-h-28 resize-y"
        value={value}
        onChange={(event) => onChange(event.target.value)}
        placeholder={labels.placeholder}
        maxLength={SUMMARY_MAX_LENGTH}
      />

      <div className="mt-2 flex flex-col gap-1 text-xs text-muted-foreground sm:flex-row sm:items-center sm:justify-between">
        <span
          className={cn(
            hasValue && isInSuggestedRange && "text-emerald-600 dark:text-emerald-400"
          )}
        >
          {labels.suggestedRange}
        </span>
        <span>{labels.count}</span>
      </div>
    </section>
  );
}

export { SUMMARY_MAX_LENGTH };
