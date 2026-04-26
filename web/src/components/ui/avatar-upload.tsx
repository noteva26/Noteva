import { useRef, useTransition } from "react";
import { uploadApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Upload, X, Loader2, User } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

interface AvatarUploadProps {
  value?: string;
  onChange?: (url: string) => void;
  className?: string;
}

export function AvatarUpload({ value, onChange, className }: AvatarUploadProps) {
  const [isUploading, startUploadTransition] = useTransition();
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleUpload = (file: File) => {
    if (!file.type.startsWith("image/")) {
      toast.error("Please select an image file");
      return;
    }

    if (file.size > 2 * 1024 * 1024) {
      toast.error("Avatar size must be under 2MB");
      return;
    }

    startUploadTransition(async () => {
      try {
        const { data } = await uploadApi.image(file);
        onChange?.(data.url);
        toast.success("Avatar uploaded");
      } catch {
        toast.error("Avatar upload failed");
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

  const clearAvatar = () => {
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
        className="relative w-16 h-16 rounded-full bg-muted flex items-center justify-center overflow-hidden cursor-pointer border-2 border-dashed border-muted-foreground/25 hover:border-primary/50 transition-colors"
        onClick={() => fileInputRef.current?.click()}
      >
        {isUploading ? (
          <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
        ) : value ? (
          <img src={value} alt="Avatar" className="w-full h-full object-cover" />
        ) : (
          <User className="h-6 w-6 text-muted-foreground" />
        )}
      </div>

      <div className="flex-1 space-y-2">
        <div className="flex gap-2">
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
              onClick={clearAvatar}
              disabled={isUploading}
            >
              <X className="h-4 w-4" />
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}
