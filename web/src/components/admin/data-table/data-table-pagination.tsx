import { ChevronLeft, ChevronRight } from "lucide-react";
import { Button } from "@/components/ui/button";

interface DataTablePaginationProps {
  page: number;
  totalPages: number;
  selectedCount?: number;
  rowCount?: number;
  loading?: boolean;
  onPrevious: () => void;
  onNext: () => void;
  pageLabel?: string;
  selectedLabel?: (selected: number, rows: number) => string;
}

export function DataTablePagination({
  page,
  totalPages,
  selectedCount = 0,
  rowCount = 0,
  loading = false,
  onPrevious,
  onNext,
  pageLabel,
  selectedLabel,
}: DataTablePaginationProps) {
  "use memo";

  const normalizedTotalPages = Math.max(1, totalPages);

  return (
    <div className="flex flex-col gap-3 border-t px-3 py-3 sm:flex-row sm:items-center sm:justify-between">
      <p className="text-sm text-muted-foreground">
        {selectedLabel
          ? selectedLabel(selectedCount, rowCount)
          : `${selectedCount} of ${rowCount} row(s) selected`}
      </p>
      <div className="flex items-center justify-between gap-6 sm:justify-end">
        <p className="text-sm font-medium">
          {pageLabel || `Page ${page} of ${normalizedTotalPages}`}
        </p>
        <div className="flex items-center gap-2">
          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8"
            onClick={onPrevious}
            disabled={page <= 1 || loading}
          >
            <ChevronLeft className="h-4 w-4" />
          </Button>
          <Button
            variant="outline"
            size="icon"
            className="h-8 w-8"
            onClick={onNext}
            disabled={page >= normalizedTotalPages || loading}
          >
            <ChevronRight className="h-4 w-4" />
          </Button>
        </div>
      </div>
    </div>
  );
}
