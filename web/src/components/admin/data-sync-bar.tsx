import { Loader2 } from "lucide-react";
import { cn } from "@/lib/utils";

interface DataSyncBarProps {
  active: boolean;
  label?: string;
  className?: string;
}

export function DataSyncBar({ active, label, className }: DataSyncBarProps) {
  "use memo";

  if (!active) return null;

  return (
    <div
      className={cn(
        "pointer-events-none flex h-1.5 w-full overflow-hidden rounded-full bg-primary/10",
        className
      )}
      aria-live="polite"
      aria-label={label}
    >
      <div className="h-full w-1/3 animate-[sync-slide_1.1s_ease-in-out_infinite] rounded-full bg-primary/70" />
      {label ? <span className="sr-only">{label}</span> : null}
    </div>
  );
}

interface DataSyncBadgeProps {
  active: boolean;
  label: string;
  className?: string;
}

export function DataSyncBadge({ active, label, className }: DataSyncBadgeProps) {
  "use memo";

  if (!active) return null;

  return (
    <span
      className={cn(
        "inline-flex items-center gap-1.5 rounded-full border bg-background px-2.5 py-1 text-xs text-muted-foreground shadow-sm",
        className
      )}
    >
      <Loader2 className="h-3 w-3 animate-spin" />
      {label}
    </span>
  );
}
