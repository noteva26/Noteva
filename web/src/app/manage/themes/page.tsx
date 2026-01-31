"use client";

import { useEffect, useState } from "react";
import { adminApi, ThemeResponse } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Palette, Check, RefreshCw, ExternalLink, User, Tag } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useTranslation } from "@/lib/i18n";
import Image from "next/image";

export default function ThemesPage() {
  const { t } = useTranslation();
  const [themes, setThemes] = useState<ThemeResponse[]>([]);
  const [currentTheme, setCurrentTheme] = useState("");
  const [loading, setLoading] = useState(true);
  const [switching, setSwitching] = useState(false);

  const fetchThemes = async () => {
    setLoading(true);
    try {
      const { data } = await adminApi.themes();
      setThemes(data?.themes || []);
      setCurrentTheme(data?.current || "default");
    } catch (error) {
      toast.error(t("error.loadFailed"));
      setThemes([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchThemes();
  }, []);

  const handleSwitchTheme = async (themeName: string) => {
    if (themeName === currentTheme) return;
    setSwitching(true);
    try {
      await adminApi.switchTheme(themeName);
      setCurrentTheme(themeName);
      toast.success(t("settings.switchSuccess"));
    } catch (error) {
      toast.error(t("settings.switchFailed"));
    } finally {
      setSwitching(false);
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t("manage.themes")}</h1>
          <p className="text-muted-foreground">{t("settings.selectTheme")}</p>
        </div>
        <Button variant="outline" onClick={fetchThemes} disabled={loading}>
          <RefreshCw className={cn("h-4 w-4 mr-2", loading && "animate-spin")} />
          {t("common.loading").replace("...", "")}
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Palette className="h-5 w-5" />
            {t("settings.themeSettings")}
          </CardTitle>
          <CardDescription>
            {t("settings.currentTheme")}: <span className="font-medium">{currentTheme}</span>
          </CardDescription>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {Array.from({ length: 3 }).map((_, i) => (
                <Skeleton key={i} className="h-64" />
              ))}
            </div>
          ) : themes.length === 0 ? (
            <div className="text-center py-12 text-muted-foreground">
              <Palette className="h-12 w-12 mx-auto mb-4 opacity-50" />
              <p>{t("common.noData")}</p>
            </div>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {themes.map((theme) => (
                <div
                  key={theme.name}
                  className={cn(
                    "relative rounded-lg border-2 overflow-hidden transition-all hover:border-primary hover:shadow-md",
                    currentTheme === theme.name
                      ? "border-primary bg-primary/5 shadow-sm"
                      : "border-muted"
                  )}
                >
                  {/* Preview Image */}
                  <div className="relative h-36 bg-gradient-to-br from-muted to-muted/50 flex items-center justify-center">
                    {theme.preview ? (
                      <Image
                        src={`/themes/${theme.name}/${theme.preview}`}
                        alt={theme.display_name}
                        fill
                        className="object-cover"
                        onError={(e) => {
                          (e.target as HTMLImageElement).style.display = 'none';
                        }}
                      />
                    ) : (
                      <Palette className="h-12 w-12 text-muted-foreground/30" />
                    )}
                    {currentTheme === theme.name && (
                      <div className="absolute top-2 right-2 flex items-center gap-1 bg-primary text-primary-foreground px-2 py-1 rounded text-xs">
                        <Check className="h-3 w-3" />
                        {t("settings.currentTheme")}
                      </div>
                    )}
                  </div>

                  {/* Theme Info */}
                  <div className="p-4">
                    <div className="flex items-start justify-between mb-2">
                      <div>
                        <h3 className="font-semibold text-lg">{theme.display_name}</h3>
                        <div className="flex items-center gap-2 text-xs text-muted-foreground">
                          <Tag className="h-3 w-3" />
                          <span>v{theme.version}</span>
                          {theme.author && (
                            <>
                              <span>•</span>
                              <User className="h-3 w-3" />
                              <span>{theme.author}</span>
                            </>
                          )}
                        </div>
                      </div>
                      {theme.url && (
                        <a
                          href={theme.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="text-muted-foreground hover:text-foreground"
                          onClick={(e) => e.stopPropagation()}
                        >
                          <ExternalLink className="h-4 w-4" />
                        </a>
                      )}
                    </div>
                    
                    {theme.description && (
                      <p className="text-sm text-muted-foreground line-clamp-2 mb-3">
                        {theme.description}
                      </p>
                    )}
                    
                    <Button
                      onClick={() => handleSwitchTheme(theme.name)}
                      disabled={switching || currentTheme === theme.name}
                      variant={currentTheme === theme.name ? "secondary" : "default"}
                      size="sm"
                      className="w-full"
                    >
                      {currentTheme === theme.name ? t("settings.currentTheme") : t("settings.switchTheme")}
                    </Button>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
