import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";

interface AiContentConfirmDialogProps {
  open: boolean;
  title: string;
  description: string;
  originalLabel: string;
  resultLabel: string;
  cancelLabel: string;
  applyLabel: string;
  original: string;
  result: string;
  onOpenChange: (open: boolean) => void;
  onApply: () => void;
}

function ContentPreview({ label, content }: { label: string; content: string }) {
  return (
    <div className="min-w-0 space-y-2">
      <div className="text-xs font-medium text-muted-foreground">{label}</div>
      <pre className="max-h-72 overflow-auto rounded-md border bg-muted/30 p-3 text-xs leading-relaxed text-foreground whitespace-pre-wrap">
        {content || " "}
      </pre>
    </div>
  );
}

export function AiContentConfirmDialog({
  open,
  title,
  description,
  originalLabel,
  resultLabel,
  cancelLabel,
  applyLabel,
  original,
  result,
  onOpenChange,
  onApply,
}: AiContentConfirmDialogProps) {
  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent className="sm:max-w-5xl">
        <AlertDialogHeader>
          <AlertDialogTitle>{title}</AlertDialogTitle>
          <AlertDialogDescription>{description}</AlertDialogDescription>
        </AlertDialogHeader>
        <div className="grid gap-4 lg:grid-cols-2">
          <ContentPreview label={originalLabel} content={original} />
          <ContentPreview label={resultLabel} content={result} />
        </div>
        <AlertDialogFooter>
          <AlertDialogCancel>{cancelLabel}</AlertDialogCancel>
          <AlertDialogAction onClick={onApply}>{applyLabel}</AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  );
}
