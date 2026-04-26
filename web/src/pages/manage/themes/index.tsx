import { useCallback, useEffect, useOptimistic, useRef, useState, useTransition } from "react";
import { adminApi, ThemeResponse, GitHubReleaseInfo, GitHubAssetInfo, StoreThemeInfo, PluginSettingsSchema } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Input } from "@/components/ui/input";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { SettingsRenderer, parseSettingsValues } from "@/components/settings-renderer";
import { Palette, Check, RefreshCw, ExternalLink, User, Tag, Upload, Download, Trash2, Github, Loader2, Search, Package, AlertTriangle, Store, Settings, Save, CheckCircle2 } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useTranslation } from "@/lib/i18n";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";
import { ConfirmDialog } from "@/components/admin/confirm-dialog";
import { getApiErrorMessage } from "@/lib/api-error";
import { formatFileSize } from "@/lib/format";
import { parseGitHubRepo } from "@/lib/github";

const STORE_CACHE_TTL = 5 * 60 * 1000;

type ThemeOptimisticAction = { type: "remove"; name: string };

function reduceOptimisticThemes(themes: ThemeResponse[], action: ThemeOptimisticAction) {
  switch (action.type) {
    case "remove":
      return themes.filter((theme) => theme.name !== action.name);
  }
}

export default function ThemesPage() {
  const { t, locale } = useTranslation();
  const [themes, setThemes] = useState<ThemeResponse[]>([]);
  const [optimisticThemes, applyOptimisticTheme] = useOptimistic<ThemeResponse[], ThemeOptimisticAction>(
    themes,
    reduceOptimisticThemes
  );
  const [currentTheme, setCurrentTheme] = useState("");
  const [optimisticCurrentTheme, setOptimisticCurrentTheme] = useOptimistic(
    currentTheme,
    (_current, nextTheme: string) => nextTheme
  );
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [refreshDone, setRefreshDone] = useState(false);
  const [isRefreshing, startRefreshTransition] = useTransition();
  const [isUploading, startUploadTransition] = useTransition();
  const [pendingSwitchTheme, setPendingSwitchTheme] = useState<string | null>(null);
  const [isSwitchingTheme, startSwitchThemeTransition] = useTransition();
  const [pendingDeleteTheme, setPendingDeleteTheme] = useState<string | null>(null);
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [activeUpdateTarget, setActiveUpdateTarget] = useState<string | null>(null);
  const [isDeletingTheme, startDeleteThemeTransition] = useTransition();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const refreshDoneTimerRef = useRef<number | null>(null);

  const [repoUrl, setRepoUrl] = useState("");
  const [releases, setReleases] = useState<GitHubReleaseInfo[]>([]);
  const [isLoadingReleases, startLoadReleasesTransition] = useTransition();
  const [pendingAssetUrl, setPendingAssetUrl] = useState<string | null>(null);
  const [isInstallingAsset, startInstallAssetTransition] = useTransition();

  const [storeThemes, setStoreThemes] = useState<StoreThemeInfo[]>([]);
  const [isLoadingStore, startLoadStoreTransition] = useTransition();
  const [pendingStoreThemeSlug, setPendingStoreThemeSlug] = useState<string | null>(null);
  const [isInstallingFromStore, startInstallFromStoreTransition] = useTransition();
  const storeCacheRef = useRef<{ data: StoreThemeInfo[]; ts: number } | null>(null);

  const [updates, setUpdates] = useState<Record<string, { current: string; latest: string }>>({});
  const [isCheckingUpdates, startCheckUpdatesTransition] = useTransition();
  const [pendingUpdateTheme, setPendingUpdateTheme] = useState<string | null>(null);
  const [isUpdatingTheme, startUpdateThemeTransition] = useTransition();

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [selectedTheme, setSelectedTheme] = useState<ThemeResponse | null>(null);
  const [settingsSchema, setSettingsSchema] = useState<PluginSettingsSchema | null>(null);
  const [settingsValues, setSettingsValues] = useState<Record<string, unknown>>({});
  const [isLoadingSettings, startLoadSettingsTransition] = useTransition();
  const [isSavingSettings, startSaveSettingsTransition] = useTransition();
  const settingsRequestIdRef = useRef(0);

  const markRefreshDone = useCallback(() => {
    if (refreshDoneTimerRef.current) {
      window.clearTimeout(refreshDoneTimerRef.current);
    }
    setRefreshDone(true);
    refreshDoneTimerRef.current = window.setTimeout(() => {
      setRefreshDone(false);
      refreshDoneTimerRef.current = null;
    }, 1500);
  }, []);

  const loadThemes = useCallback(
    async (options: { isRefresh?: boolean; isActive?: () => boolean } = {}) => {
      const { isRefresh = false, isActive = () => true } = options;
      if (!isRefresh) {
        setLoading(true);
      }

      try {
        await adminApi.reloadThemes();
        const { data } = await adminApi.themes();
        if (!isActive()) return;

        setThemes(data?.themes || []);
        setCurrentTheme(data?.current || "default");
        if (isRefresh) {
          markRefreshDone();
        }
      } catch {
        if (!isActive()) return;
        toast.error(t("error.loadFailed"));
        if (!isRefresh) {
          setThemes([]);
        }
      } finally {
        if (isActive() && !isRefresh) {
          setLoading(false);
          setHasLoaded(true);
        }
      }
    },
    [markRefreshDone, t]
  );

  const loadStore = useCallback(
    async (force = false) => {
      if (!force && storeCacheRef.current && Date.now() - storeCacheRef.current.ts < STORE_CACHE_TTL) {
        setStoreThemes(storeCacheRef.current.data);
        return;
      }

      try {
        const { data } = await adminApi.getThemeStore();
        const themes = data?.themes || [];
        setStoreThemes(themes);
        storeCacheRef.current = { data: themes, ts: Date.now() };
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
        setStoreThemes([]);
      }
    },
    [t]
  );

  useEffect(() => {
    let active = true;
    void loadThemes({ isActive: () => active });
    return () => {
      active = false;
    };
  }, [loadThemes]);

  useEffect(() => {
    return () => {
      if (refreshDoneTimerRef.current) {
        window.clearTimeout(refreshDoneTimerRef.current);
      }
    };
  }, []);

  const refreshThemes = () => {
    startRefreshTransition(async () => {
      await loadThemes({ isRefresh: true });
    });
  };

  const fetchReleases = () => {
    const repo = parseGitHubRepo(repoUrl);
    if (!repo) {
      toast.error(t("theme.enterRepo") || "Please enter a GitHub repo");
      return;
    }

    startLoadReleasesTransition(async () => {
      try {
        const { data } = await adminApi.listGitHubReleases(repo);
        setReleases(data || []);
        if (!data?.length) {
          toast.info(t("theme.noReleases") || "No releases found");
        }
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
        setReleases([]);
      }
    });
  };

  const fetchStore = (force = false) => {
    startLoadStoreTransition(async () => {
      await loadStore(force);
    });
  };

  const checkUpdates = () => {
    startCheckUpdatesTransition(async () => {
      try {
        const { data } = await adminApi.checkThemeUpdates();
        const updatesMap: Record<string, { current: string; latest: string }> = {};
        data.updates.forEach((update) => {
          updatesMap[update.name] = {
            current: update.current_version,
            latest: update.latest_version,
          };
        });
        setUpdates(updatesMap);
        if (data.updates.length > 0) {
          toast.success(t("theme.updatesFound", { count: data.updates.length }));
        } else {
          toast.info(t("theme.allUpToDate"));
        }
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("theme.checkUpdateFailed")));
      }
    });
  };

  const updateTheme = (themeName: string) => {
    setPendingUpdateTheme(themeName);
    startUpdateThemeTransition(async () => {
      try {
        const { data } = await adminApi.updateTheme(themeName);
        toast.success(data.message);
        setUpdates((current) => {
          const next = { ...current };
          delete next[themeName];
          return next;
        });
        await loadThemes({ isRefresh: true });
        if (themeName === currentTheme) {
          toast.info(t("theme.updateRefreshHint"));
        }
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("theme.updateFailed")));
      } finally {
        setPendingUpdateTheme(null);
      }
    });
  };

  const handleUpdateTheme = (themeName: string) => {
    if (themeName === currentTheme) {
      setActiveUpdateTarget(themeName);
      return;
    }

    updateTheme(themeName);
  };

  const confirmUpdateActiveTheme = () => {
    if (!activeUpdateTarget) return;

    const themeName = activeUpdateTarget;
    setActiveUpdateTarget(null);
    updateTheme(themeName);
  };

  const handleSwitchTheme = (themeName: string) => {
    if (themeName === optimisticCurrentTheme) return;

    const theme = themes.find((item) => item.name === themeName);
    if (theme && !theme.compatible) {
      toast.error(theme.compatibility_message || t("theme.incompatible") || "Theme is not compatible with current version");
      return;
    }

    setPendingSwitchTheme(themeName);
    startSwitchThemeTransition(async () => {
      setOptimisticCurrentTheme(themeName);
      try {
        const { data } = await adminApi.switchTheme(themeName);
        if (data?.name && data.name !== themeName) {
          toast.error(`${t("settings.switchFailed")}: ${data.name}`);
          await loadThemes({ isRefresh: true });
          return;
        }

        setCurrentTheme(themeName);
        setThemes((current) =>
          current.map((item) => ({
            ...item,
            active: item.name === themeName,
          }))
        );
        toast.success(t("settings.switchSuccess"));
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("settings.switchFailed")));
        await loadThemes({ isRefresh: true });
      } finally {
        setPendingSwitchTheme(null);
      }
    });
  };

  const handleUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    startUploadTransition(async () => {
      try {
        const { data } = await adminApi.uploadTheme(file);
        toast.success(data.message);
        await loadThemes({ isRefresh: true });
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      } finally {
        if (fileInputRef.current) {
          fileInputRef.current.value = "";
        }
      }
    });
  };

  const handleInstallAsset = (asset: GitHubAssetInfo) => {
    setPendingAssetUrl(asset.download_url);
    startInstallAssetTransition(async () => {
      try {
        const { data } = await adminApi.installGitHubTheme(asset.download_url);
        toast.success(data.message);
        await loadThemes({ isRefresh: true });
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      } finally {
        setPendingAssetUrl(null);
      }
    });
  };

  const handleInstallFromStore = (theme: StoreThemeInfo) => {
    if (!theme.github_url) {
      toast.error(t("theme.noGitHubUrl") || "No GitHub URL available");
      return;
    }

    const githubUrl = theme.github_url;
    setPendingStoreThemeSlug(theme.slug);
    startInstallFromStoreTransition(async () => {
      try {
        const { data } = await adminApi.installThemeFromRepo(githubUrl, theme.slug);
        toast.success(data.message);
        await loadThemes({ isRefresh: true });
        await loadStore(true);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      } finally {
        setPendingStoreThemeSlug(null);
      }
    });
  };

  const handleDelete = (themeName: string) => {
    setDeleteTarget(themeName);
  };

  const confirmDeleteTheme = () => {
    if (!deleteTarget) return;

    const themeName = deleteTarget;
    setDeleteTarget(null);
    setPendingDeleteTheme(themeName);
    startDeleteThemeTransition(async () => {
      applyOptimisticTheme({ type: "remove", name: themeName });
      try {
        await adminApi.deleteTheme(themeName);
        setThemes((current) => current.filter((theme) => theme.name !== themeName));
        setUpdates((current) => {
          const next = { ...current };
          delete next[themeName];
          return next;
        });
        toast.success(t("theme.deleteSuccess") || "Theme deleted");
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
        await loadThemes({ isRefresh: true });
      } finally {
        setPendingDeleteTheme(null);
      }
    });
  };

  const openThemeSettings = (theme: ThemeResponse) => {
    const requestId = settingsRequestIdRef.current + 1;
    settingsRequestIdRef.current = requestId;
    setSelectedTheme(theme);
    setSettingsOpen(true);
    setSettingsSchema(null);
    setSettingsValues({});

    startLoadSettingsTransition(async () => {
      try {
        const { data } = await adminApi.getThemeSettings(theme.name);
        if (settingsRequestIdRef.current !== requestId) return;
        setSettingsSchema(data?.schema || null);
        setSettingsValues(parseSettingsValues(data?.values || {}));
      } catch {
        if (settingsRequestIdRef.current === requestId) {
          toast.error(t("error.loadFailed"));
        }
      }
    });
  };

  const handleSettingsOpenChange = (open: boolean) => {
    setSettingsOpen(open);
    if (!open) {
      settingsRequestIdRef.current += 1;
    }
  };

  const handleSaveThemeSettings = () => {
    if (!selectedTheme) return;

    startSaveSettingsTransition(async () => {
      try {
        await adminApi.updateThemeSettings(selectedTheme.name, settingsValues);
        toast.success(t("plugin.saveSuccess") || "Settings saved");
        setSettingsOpen(false);
      } catch {
        toast.error(t("plugin.saveFailed") || "Save failed");
      }
    });
  };

  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = (loading && hasLoaded) || isRefreshing;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t("manage.themes")}</h1>
          <p className="text-muted-foreground">{t("settings.selectTheme")}</p>
        </div>
        <div className="flex gap-2">
          <input
            ref={fileInputRef}
            type="file"
            accept=".zip,.tar,.tar.gz,.tgz"
            onChange={handleUpload}
            className="hidden"
          />
          <Button
            variant="outline"
            onClick={() => fileInputRef.current?.click()}
            disabled={isUploading}
          >
            {isUploading ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Upload className="h-4 w-4 mr-2" />
            )}
            {t("theme.upload") || "Upload"}
          </Button>
          <Button variant="outline" onClick={checkUpdates} disabled={isCheckingUpdates}>
            {isCheckingUpdates ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Download className="h-4 w-4 mr-2" />
            )}
            {t("theme.checkUpdates")}
          </Button>
          <Button variant="outline" onClick={refreshThemes} disabled={isRefreshing}>
            {refreshDone ? (
              <CheckCircle2 className="h-4 w-4 mr-2 text-green-500 animate-in fade-in duration-300" />
            ) : (
              <RefreshCw className={cn("h-4 w-4 mr-2 transition-transform duration-500", isRefreshing && "animate-spin")} />
            )}
            {refreshDone ? (t("common.done") || "Done") : (t("common.refresh") || "Refresh")}
          </Button>
        </div>
      </div>
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />
      <DataSyncBar active={isSyncing} label={t("common.loading")} />

      <Tabs defaultValue="installed" className="space-y-4">
        <TabsList>
          <TabsTrigger value="installed" className="gap-2">
            <Palette className="h-4 w-4" />
            {t("theme.installed") || "Installed"}
          </TabsTrigger>
          <TabsTrigger value="store" className="gap-2" onClick={() => fetchStore()}>
            <Store className="h-4 w-4" />
            {t("theme.store") || "Store"}
          </TabsTrigger>
          <TabsTrigger value="github" className="gap-2">
            <Github className="h-4 w-4" />
            {t("theme.online") || "Online"}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="installed">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Palette className="h-5 w-5" />
                {t("settings.themeSettings")}
              </CardTitle>
              <CardDescription>
                {t("settings.currentTheme")}: <span className="font-medium">{optimisticCurrentTheme}</span>
              </CardDescription>
            </CardHeader>
            <CardContent>
              {showInitialLoading ? (
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {Array.from({ length: 3 }).map((_, i) => (
                    <Skeleton key={i} className="h-64" />
                  ))}
                </div>
              ) : optimisticThemes.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Palette className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>{t("common.noData")}</p>
                </div>
              ) : (
                <div className={cn("grid gap-4 md:grid-cols-2 lg:grid-cols-3 transition-opacity", isSyncing && "opacity-70")}>
                  {optimisticThemes.map((theme) => {
                    const isActive = optimisticCurrentTheme === theme.name;
                    const isSwitchingThisTheme = isSwitchingTheme && pendingSwitchTheme === theme.name;
                    const isDeletingThisTheme = isDeletingTheme && pendingDeleteTheme === theme.name;
                    const isUpdatingThisTheme = isUpdatingTheme && pendingUpdateTheme === theme.name;

                    return (
                      <ThemeCard
                        key={theme.name}
                        theme={theme}
                        isActive={isActive}
                        isDefault={theme.name === "default"}
                        isSwitching={isSwitchingThisTheme}
                        isUpdating={isUpdatingThisTheme}
                        deleting={isDeletingThisTheme}
                        onSwitch={() => handleSwitchTheme(theme.name)}
                        onDelete={() => handleDelete(theme.name)}
                        onSettings={theme.has_settings ? () => openThemeSettings(theme) : undefined}
                        onUpdate={updates[theme.name] ? () => handleUpdateTheme(theme.name) : undefined}
                        updateInfo={updates[theme.name]}
                        t={t}
                      />
                    );
                  })}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="store">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Store className="h-5 w-5" />
                {t("theme.officialStore") || "Official Theme Store"}
              </CardTitle>
              <CardDescription>
                {t("theme.storeDesc") || "Browse and install official themes"}
              </CardDescription>
            </CardHeader>
            <CardContent>
              {isLoadingStore ? (
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {Array.from({ length: 3 }).map((_, i) => (
                    <Skeleton key={i} className="h-64" />
                  ))}
                </div>
              ) : storeThemes.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Store className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>{t("theme.noStoreThemes") || "No themes available"}</p>
                </div>
              ) : (
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {storeThemes.map((theme) => {
                    const isInstallingTheme = isInstallingFromStore && pendingStoreThemeSlug === theme.slug;

                    return (
                      <Card key={theme.slug}>
                        <StoreThemePreview theme={theme} installedLabel={t("theme.installed") || "Installed"} />
                        <div className="p-4">
                          <div className="flex items-start justify-between mb-2">
                            <div>
                              <h3 className="font-semibold text-lg">{theme.name}</h3>
                              <div className="flex items-center gap-2 text-xs text-muted-foreground">
                                <Tag className="h-3 w-3" />
                                <span>v{theme.version}</span>
                                {theme.author && (
                                  <>
                                    <span>-</span>
                                    <User className="h-3 w-3" />
                                    <span>{theme.author}</span>
                                  </>
                                )}
                                {theme.download_count > 0 && (
                                  <>
                                    <span>-</span>
                                    <Download className="h-3 w-3" />
                                    <span>{theme.download_count}</span>
                                  </>
                                )}
                              </div>
                            </div>
                            {theme.github_url && (
                              <a
                                href={theme.github_url}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-muted-foreground hover:text-foreground"
                              >
                                <Github className="h-4 w-4" />
                              </a>
                            )}
                          </div>

                          {theme.description && (
                            <p className="text-sm text-muted-foreground line-clamp-2 mb-2">
                              {theme.description}
                            </p>
                          )}

                          {theme.tags.length > 0 && (
                            <div className="flex gap-1 flex-wrap mb-2">
                              {theme.tags.map((tag) => (
                                <Badge key={tag} variant="outline" className="text-xs">{tag}</Badge>
                              ))}
                            </div>
                          )}

                          <Button
                            onClick={() => handleInstallFromStore(theme)}
                            disabled={isInstallingTheme || theme.installed}
                            size="sm"
                            className="w-full"
                          >
                            {isInstallingTheme ? (
                              <Loader2 className="h-4 w-4 animate-spin" />
                            ) : theme.installed ? (
                              t("theme.installed") || "Installed"
                            ) : (
                              <>
                                <Download className="h-4 w-4 mr-2" />
                                {t("theme.install") || "Install"}
                              </>
                            )}
                          </Button>
                        </div>
                      </Card>
                    );
                  })}
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>

        <TabsContent value="github">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Github className="h-5 w-5" />
                {t("theme.onlineThemes") || "Install from GitHub"}
              </CardTitle>
              <CardDescription>
                {t("theme.onlineDesc") || "Enter a GitHub repository URL to browse releases"}
              </CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex gap-2">
                <Input
                  placeholder="owner/repo or https://github.com/owner/repo"
                  value={repoUrl}
                  onChange={(event) => setRepoUrl(event.target.value)}
                  onKeyDown={(event) => event.key === "Enter" && fetchReleases()}
                />
                <Button onClick={fetchReleases} disabled={isLoadingReleases}>
                  {isLoadingReleases ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Search className="h-4 w-4" />
                  )}
                </Button>
              </div>

              {releases.length > 0 && (
                <div className="space-y-4">
                  {releases.map((release) => (
                    <div key={release.tag_name} className="border rounded-lg p-4 space-y-3">
                      <div className="flex items-center justify-between">
                        <div>
                          <h3 className="font-semibold flex items-center gap-2">
                            <Tag className="h-4 w-4" />
                            {release.name || release.tag_name}
                          </h3>
                          <p className="text-xs text-muted-foreground">
                            {release.tag_name}
                            {release.published_at && ` - ${new Date(release.published_at).toLocaleDateString(locale)}`}
                          </p>
                        </div>
                      </div>

                      {release.assets.length > 0 ? (
                        <div className="grid gap-2">
                          {release.assets.map((asset) => {
                            const isInstallingThisAsset = isInstallingAsset && pendingAssetUrl === asset.download_url;

                            return (
                              <div
                                key={asset.download_url}
                                className="flex items-center justify-between p-2 bg-muted/50 rounded"
                              >
                                <div className="flex items-center gap-2">
                                  <Package className="h-4 w-4 text-muted-foreground" />
                                  <span className="text-sm">{asset.name}</span>
                                  <span className="text-xs text-muted-foreground">
                                    ({formatFileSize(asset.size)})
                                  </span>
                                </div>
                                <Button
                                  size="sm"
                                  onClick={() => handleInstallAsset(asset)}
                                  disabled={isInstallingThisAsset}
                                >
                                  {isInstallingThisAsset ? (
                                    <Loader2 className="h-4 w-4 animate-spin" />
                                  ) : (
                                    <Download className="h-4 w-4" />
                                  )}
                                </Button>
                              </div>
                            );
                          })}
                        </div>
                      ) : (
                        <p className="text-sm text-muted-foreground">
                          {t("theme.noAssets") || "No downloadable assets"}
                        </p>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {!isLoadingReleases && releases.length === 0 && repoUrl && (
                <div className="text-center py-8 text-muted-foreground">
                  <Github className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>{t("theme.searchHint") || "Enter a repo and click search"}</p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <Sheet open={settingsOpen} onOpenChange={handleSettingsOpenChange}>
        <SheetContent className="overflow-y-auto">
          <SheetHeader>
            <SheetTitle>{selectedTheme?.display_name}</SheetTitle>
            <SheetDescription>{t("plugin.settingsTitle") || "Settings"}</SheetDescription>
          </SheetHeader>
          <div className="mt-4 space-y-4">
            {isLoadingSettings ? (
              <div className="flex justify-center py-8 text-muted-foreground">
                <Loader2 className="h-5 w-5 animate-spin" />
              </div>
            ) : (
              <SettingsRenderer
                schema={settingsSchema}
                values={settingsValues}
                onChange={setSettingsValues}
                emptyMessage={t("plugin.noSettings") || "No settings available"}
              />
            )}
            {settingsSchema?.sections?.length ? (
              <Button onClick={handleSaveThemeSettings} disabled={isSavingSettings || isLoadingSettings} className="w-full">
                {isSavingSettings ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Save className="h-4 w-4 mr-2" />
                )}
                {t("plugin.saveSettings") || "Save"}
              </Button>
            ) : null}
          </div>
        </SheetContent>
      </Sheet>
      <ConfirmDialog
        open={activeUpdateTarget !== null}
        title={t("common.confirm")}
        description={t("theme.confirmUpdateActive")}
        confirmLabel={t("theme.update")}
        cancelLabel={t("common.cancel")}
        loading={isUpdatingTheme}
        onOpenChange={(open) => !open && setActiveUpdateTarget(null)}
        onConfirm={confirmUpdateActiveTheme}
      />
      <ConfirmDialog
        open={deleteTarget !== null}
        title={t("common.confirm")}
        description={
          t("theme.confirmDelete")?.replace("{name}", deleteTarget || "") ||
          `Delete theme "${deleteTarget || ""}"?`
        }
        confirmLabel={t("common.delete")}
        cancelLabel={t("common.cancel")}
        destructive
        loading={isDeletingTheme}
        onOpenChange={(open) => !open && setDeleteTarget(null)}
        onConfirm={confirmDeleteTheme}
      />
    </div>
  );
}

interface ThemeCardProps {
  theme: ThemeResponse;
  isActive: boolean;
  isDefault: boolean;
  isSwitching: boolean;
  isUpdating: boolean;
  deleting: boolean;
  onSwitch: () => void;
  onDelete: () => void;
  onSettings?: () => void;
  onUpdate?: () => void;
  updateInfo?: { current: string; latest: string };
  t: (key: string, params?: Record<string, string | number>) => string;
}

function ThemeCard({ theme, isActive, isDefault, isSwitching, isUpdating, deleting, onSwitch, onDelete, onSettings, onUpdate, updateInfo, t }: ThemeCardProps) {
  const repositoryHref = theme.repository
    ? theme.repository.startsWith("http")
      ? theme.repository
      : `https://github.com/${theme.repository}`
    : "";

  return (
    <div
      className={cn(
        "relative rounded-lg border-2 overflow-hidden transition-all hover:border-primary hover:shadow-md",
        isActive ? "border-primary bg-primary/5 shadow-sm" : "border-muted",
        !theme.compatible && "opacity-70"
      )}
    >
      <ThemePreview theme={theme} isActive={isActive} currentLabel={t("settings.currentTheme")} incompatibleLabel={t("theme.incompatible") || "Incompatible"} />

      <div className="p-4">
        <div className="flex items-start justify-between mb-2">
          <div>
            <h3 className="font-semibold text-lg flex items-center gap-2">
              {theme.display_name}
              {updateInfo && (
                <Badge variant="default" className="text-xs">
                  {updateInfo.current} -&gt; {updateInfo.latest}
                </Badge>
              )}
              {!theme.compatible && (
                <span title={theme.compatibility_message || t("theme.incompatible") || "Not compatible"}>
                  <AlertTriangle className="h-4 w-4 text-amber-500" />
                </span>
              )}
            </h3>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Tag className="h-3 w-3" />
              <span>v{theme.version}</span>
              {theme.author && (
                <>
                  <span>-</span>
                  <User className="h-3 w-3" />
                  <span>{theme.author}</span>
                </>
              )}
              {theme.requires_noteva && (
                <>
                  <span>-</span>
                  <span>{t("theme.requires") || "Requires"}: {theme.requires_noteva}</span>
                </>
              )}
            </div>
          </div>
          {theme.repository && (
            <a
              href={repositoryHref}
              target="_blank"
              rel="noopener noreferrer"
              className="text-muted-foreground hover:text-foreground"
              onClick={(event) => event.stopPropagation()}
            >
              <ExternalLink className="h-4 w-4" />
            </a>
          )}
        </div>

        {theme.description && (
          <p className="text-sm text-muted-foreground line-clamp-2 mb-2">
            {theme.description}
          </p>
        )}

        {!theme.compatible && theme.compatibility_message && (
          <p className="text-xs text-amber-600 dark:text-amber-400 mb-2">
            {theme.compatibility_message}
          </p>
        )}

        <div className="flex gap-2">
          {updateInfo && onUpdate && (
            <Button
              onClick={onUpdate}
              disabled={isUpdating}
              variant="default"
              size="sm"
              title={t("theme.updateTo", { version: updateInfo.latest })}
            >
              {isUpdating ? (
                <Loader2 className="h-4 w-4 mr-1 animate-spin" />
              ) : (
                <Download className="h-4 w-4 mr-1" />
              )}
              {t("theme.update")}
            </Button>
          )}
          {theme.has_settings && onSettings && (
            <Button
              onClick={onSettings}
              variant="ghost"
              size="sm"
              title={t("plugin.settingsTitle") || "Settings"}
            >
              <Settings className="h-4 w-4" />
            </Button>
          )}
          <Button
            onClick={onSwitch}
            disabled={isSwitching || isActive || !theme.compatible}
            variant={isActive ? "secondary" : "default"}
            size="sm"
            className="flex-1"
          >
            {isSwitching ? (
              <Loader2 className="h-4 w-4 mr-1 animate-spin" />
            ) : null}
            {isActive ? t("settings.currentTheme") : t("settings.switchTheme")}
          </Button>
          {!isDefault && !isActive && (
            <Button
              onClick={onDelete}
              disabled={deleting}
              variant="destructive"
              size="sm"
            >
              {deleting ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Trash2 className="h-4 w-4" />
              )}
            </Button>
          )}
        </div>
      </div>
    </div>
  );
}

function StoreThemePreview({ theme, installedLabel }: { theme: StoreThemeInfo; installedLabel: string }) {
  const [failed, setFailed] = useState(false);
  const showImage = Boolean(theme.cover_image) && !failed;

  return (
    <div className="relative h-36 bg-gradient-to-br from-muted to-muted/50 flex items-center justify-center overflow-hidden">
      {showImage ? (
        <img
          src={theme.cover_image || ""}
          alt={theme.name}
          className="w-full h-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : (
        <Palette className="h-12 w-12 text-muted-foreground/30 absolute" />
      )}
      {theme.installed && (
        <div className="absolute top-2 right-2 flex items-center gap-1 bg-primary text-primary-foreground px-2 py-1 rounded text-xs">
          <Check className="h-3 w-3" />
          {installedLabel}
        </div>
      )}
    </div>
  );
}

function ThemePreview({ theme, isActive, currentLabel, incompatibleLabel }: { theme: ThemeResponse; isActive: boolean; currentLabel: string; incompatibleLabel: string }) {
  const [failed, setFailed] = useState(false);
  const showImage = Boolean(theme.preview) && !failed;

  return (
    <div className="relative h-36 bg-gradient-to-br from-muted to-muted/50 flex items-center justify-center">
      {showImage ? (
        <img
          src={`/themes/${theme.name}/${theme.preview}`}
          alt={theme.display_name}
          className="w-full h-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : (
        <Palette className="h-12 w-12 text-muted-foreground/30" />
      )}
      {isActive && (
        <div className="absolute top-2 right-2 flex items-center gap-1 bg-primary text-primary-foreground px-2 py-1 rounded text-xs">
          <Check className="h-3 w-3" />
          {currentLabel}
        </div>
      )}
      {!theme.compatible && (
        <div className="absolute top-2 left-2 flex items-center gap-1 bg-amber-500 text-white px-2 py-1 rounded text-xs">
          <AlertTriangle className="h-3 w-3" />
          {incompatibleLabel}
        </div>
      )}
    </div>
  );
}
