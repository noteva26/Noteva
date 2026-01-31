"use client";

import { useState, useRef, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Upload, X, Image as ImageIcon, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";

// 图片上传 API（SDK 暂不支持文件上传，直接使用 fetch）
async function uploadImage(file: File): Promise<{ url: string }> {
  const formData = new FormData();
  formData.append("file", file);
  
  const response = await fetch("/api/v1/upload/image", {
    method: "POST",
    body: formData,
    credentials: "include",
  });
  
  if (!response.ok) {
    throw new Error("Upload failed");
  }
  
  return response.json();
}

interface ImageUploadProps {
  onUpload?: (url: string) => void;
  onInsert?: (markdown: string) => void;
  className?: string;
}

export function ImageUpload({ onUpload, onInsert, className }: ImageUploadProps) {
  const [uploading, setUploading] = useState(false);
  const [preview, setPreview] = useState<string | null>(null);
  const [dragOver, setDragOver] = useState(false);
  const fileInputRef = useRef<HTMLInputElement>(null);

  const handleUpload = useCallback(async (file: File) => {
    if (!file.type.startsWith("image/")) {
      toast.error("请选择图片文件");
      return;
    }

    if (file.size > 5 * 1024 * 1024) {
      toast.error("图片大小不能超过 5MB");
      return;
    }

    setUploading(true);
    setPreview(URL.createObjectURL(file));

    try {
      const data = await uploadImage(file);
      onUpload?.(data.url);
      onInsert?.(`![${file.name}](${data.url})`);
      toast.success("图片上传成功");
    } catch (error) {
      toast.error("图片上传失败");
      setPreview(null);
    } finally {
      setUploading(false);
    }
  }, [onUpload, onInsert]);

  const handleDrop = useCallback((e: React.DragEvent) => {
    e.preventDefault();
    setDragOver(false);
    const file = e.dataTransfer.files[0];
    if (file) handleUpload(file);
  }, [handleUpload]);

  const handlePaste = useCallback((e: React.ClipboardEvent) => {
    const items = e.clipboardData.items;
    for (let i = 0; i < items.length; i++) {
      const item = items[i];
      if (item.type.startsWith("image/")) {
        const file = item.getAsFile();
        if (file) handleUpload(file);
        break;
      }
    }
  }, [handleUpload]);

  const handleFileChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (file) handleUpload(file);
  };

  const clearPreview = () => {
    setPreview(null);
    if (fileInputRef.current) fileInputRef.current.value = "";
  };

  return (
    <Card
      className={cn(
        "relative transition-colors",
        dragOver && "border-primary bg-primary/5",
        className
      )}
      onDragOver={(e) => { e.preventDefault(); setDragOver(true); }}
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
            {uploading && (
              <div className="absolute inset-0 flex items-center justify-center bg-background/80 rounded-lg">
                <Loader2 className="h-8 w-8 animate-spin text-primary" />
              </div>
            )}
            {!uploading && (
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
            <p className="text-sm font-medium">点击或拖拽上传图片</p>
            <p className="text-xs text-muted-foreground mt-1">
              支持 JPG、PNG、GIF，最大 5MB
            </p>
            <p className="text-xs text-muted-foreground">
              也可以直接粘贴图片
            </p>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
