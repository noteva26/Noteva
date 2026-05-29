import { useRef, useTransition } from "react";
import { uploadApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Upload, X, Loader2, ImageIcon } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useTranslation } from "@/lib/i18n";

interface ImageUploadProps {
  value?: string;
  onChange?: (url: string) => void;
  className?: string;
  /** Max file size in MB (default 2) */
  maxSizeMB?: number;
}

/**
 * Generic image uploader with a square preview, suited for logos, covers, etc.
 * Accepts a direct URL input or a file upload (via /upload/image).
 */
export function ImageUpload({ value, onChange, className, maxSizeMB = 2 }: ImageUploadProps) {
  const { t } = useTranslation();
  const [isUploading, startUploadTransition] = useTransition();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleUpload = (file: File) => {
    if (!file.type.startsWith("image/")) {
      toast.error(t("settings.avatarInvalidType"));
      return;
    }

    if (file.size > maxSizeMB * 1024 * 1024) {
      toast.error(t("settings.avatarTooLarge"));
      return;
    }

    startUploadTransition(async () => {
      try {
        const { data } = await uploadApi.image(file);
        onChange?.(data.url);
        toast.success(t("settings.avatarUploadSuccess"));
      } catch {
        toast.error(t("settings.avatarUploadFailed"));
      } finally {
        if (fileInputRef.current) {
          fileInputRef.current.value = "";
        }
      }
    });
  };

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) handleUpload(file);
  };

  const clearImage = () => {
    onChange?.("");
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  return (
    <div className={cn("flex items-center gap-4", className)}>
      <input
        ref={fileInputRef}
        type="file"
        accept="image/*"
        className="hidden"
        onChange={handleFileChange}
      />

      <div
        className="relative h-16 w-16 shrink-0 cursor-pointer overflow-hidden rounded-lg border-2 border-dashed border-muted-foreground/25 bg-muted flex items-center justify-center transition-colors hover:border-primary/50"
        onClick={() => fileInputRef.current?.click()}
      >
        {isUploading ? (
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        ) : value ? (
          <img src={value} alt="" className="h-full w-full object-contain" />
        ) : (
          <ImageIcon className="h-6 w-6 text-muted-foreground" />
        )}
      </div>

      <div className="flex flex-1 gap-2">
        <Input
          placeholder="https://..."
          value={value || ""}
          onChange={(event) => onChange?.(event.target.value)}
          className="flex-1"
        />
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={() => fileInputRef.current?.click()}
          disabled={isUploading}
        >
          <Upload className="h-4 w-4" />
        </Button>
        {value && (
          <Button
            type="button"
            variant="outline"
            size="icon"
            onClick={clearImage}
            disabled={isUploading}
          >
            <X className="h-4 w-4" />
          </Button>
        )}
      </div>
    </div>
  );
}
