import { useEffect, useRef, useState, useTransition } from "react";
import { uploadApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { X, Image as ImageIcon, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

interface ImageUploadProps {
  onUpload?: (url: string) => void;
  onInsert?: (markdown: string) => void;
  className?: string;
}

export function ImageUpload({ onUpload, onInsert, className }: ImageUploadProps) {
  const [preview, setPreview] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const [isUploading, startUploadTransition] = useTransition();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const previewUrlRef = useRef<string | null>(null);

  useEffect(() => {
    return () => {
      if (previewUrlRef.current) {
        URL.revokeObjectURL(previewUrlRef.current);
      }
    };
  }, []);

  const setPreviewUrl = (url: string | null) => {
    if (previewUrlRef.current) {
      URL.revokeObjectURL(previewUrlRef.current);
      previewUrlRef.current = null;
    }
    previewUrlRef.current = url;
    setPreview(url);
  };

  const handleUpload = (file: File) => {
    if (!file.type.startsWith("image/")) {
      toast.error("Please select an image file");
      return;
    }

    if (file.size > 5 * 1024 * 1024) {
      toast.error("Image size must be under 5MB");
      return;
    }

    setPreviewUrl(URL.createObjectURL(file));
    startUploadTransition(async () => {
      try {
        const { data } = await uploadApi.image(file);
        onUpload?.(data.url);
        onInsert?.(`![${file.name}](${data.url})`);
        toast.success("Image uploaded");
      } catch {
        toast.error("Image upload failed");
        setPreviewUrl(null);
      } finally {
        if (fileInputRef.current) {
          fileInputRef.current.value = "";
        }
      }
    });
  };

  const handleDrop = (event: React.DragEvent) => {
    event.preventDefault();
    setDragOver(false);
    const file = event.dataTransfer.files[0];
    if (file) handleUpload(file);
  };

  const handlePaste = (event: React.ClipboardEvent) => {
    const items = event.clipboardData.items;
    for (let index = 0; index < items.length; index++) {
      const item = items[index];
      if (item.type.startsWith("image/")) {
        const file = item.getAsFile();
        if (file) handleUpload(file);
        break;
      }
    }
  };

  const handleFileChange = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (file) handleUpload(file);
  };

  const clearPreview = () => {
    setPreviewUrl(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  return (
    <Card
      className={cn(
        "relative transition-colors",
        dragOver && "border-primary bg-primary/5",
        className
      )}
      onDragOver={(event) => { event.preventDefault(); setDragOver(true); }}
      onDragLeave={() => setDragOver(false)}
      onDrop={handleDrop}
      onPaste={handlePaste}
    >
      <CardContent className="p-4">
        <input
          ref={fileInputRef}
          type="file"
          accept="image/*"
          className="hidden"
          onChange={handleFileChange}
        />

        {preview ? (
          <div className="relative">
            <img
              src={preview}
              alt="Preview"
              className="max-h-48 mx-auto rounded-lg object-contain"
            />
            {isUploading && (
              <div className="absolute inset-0 flex items-center justify-center bg-background/80 rounded-lg">
                <Loader2 className="h-8 w-8 animate-spin text-primary" />
              </div>
            )}
            {!isUploading && (
              <Button
                variant="ghost"
                size="icon"
                className="absolute top-2 right-2"
                onClick={clearPreview}
              >
                <X className="h-4 w-4" />
              </Button>
            )}
          </div>
        ) : (
          <div
            className="flex flex-col items-center justify-center py-8 cursor-pointer"
            onClick={() => fileInputRef.current?.click()}
          >
            <div className="p-3 rounded-full bg-muted mb-3">
              <ImageIcon className="h-6 w-6 text-muted-foreground" />
            </div>
            <p className="text-sm font-medium">Click or drag to upload an image</p>
            <p className="text-xs text-muted-foreground mt-1">
              Supports JPG, PNG, GIF, max 5MB
            </p>
            <p className="text-xs text-muted-foreground">
              You can also paste an image
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
