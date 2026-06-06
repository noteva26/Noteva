import { AlertCircle, CheckCircle2, Loader2, PencilLine } from "lucide-react";
import { cn } from "@/lib/utils";

export type WritingStatus = "idle" | "saving" | "saved" | "unsaved" | "error";

interface WritingStatusBarProps {
  status: WritingStatus;
  label: string;
  detail?: string;
}

const statusClassName: Record<WritingStatus, string> = {
  idle: "border-border bg-muted/35 text-muted-foreground",
  saving: "border-primary/25 bg-primary/5 text-primary",
  saved: "border-emerald-500/25 bg-emerald-500/5 text-emerald-700 dark:text-emerald-300",
  unsaved: "border-amber-500/30 bg-amber-500/10 text-amber-700 dark:text-amber-300",
  error: "border-destructive/30 bg-destructive/10 text-destructive",
};

function StatusIcon({ status }: { status: WritingStatus }) {
  if (status === "saving") {
    return <Loader2 className="h-4 w-4 animate-spin" />;
  }

  if (status === "saved") {
    return <CheckCircle2 className="h-4 w-4" />;
  }

  if (status === "error") {
    return <AlertCircle className="h-4 w-4" />;
  }

  return <PencilLine className="h-4 w-4" />;
}

export function WritingStatusBar({ status, label, detail }: WritingStatusBarProps) {
  return (
    <div
      className={cn(
        "flex items-center justify-between gap-3 rounded-lg border px-3 py-2 text-sm transition-colors",
        statusClassName[status]
      )}
      role={status === "error" ? "alert" : "status"}
      aria-live="polite"
    >
      <div className="flex min-w-0 items-center gap-2">
        <StatusIcon status={status} />
        <span className="font-medium">{label}</span>
      </div>
      {detail ? (
        <span className="min-w-0 truncate text-xs opacity-85">{detail}</span>
      ) : null}
    </div>
  );
}
