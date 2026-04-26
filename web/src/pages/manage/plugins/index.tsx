import { useCallback, useEffect, useOptimistic, useRef, useState, useTransition } from "react";
import { pluginsApi, Plugin, PluginSettingsSchema, GitHubReleaseInfo, GitHubAssetInfo, StorePluginInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { SettingsRenderer, parseSettingsValues } from "@/components/settings-renderer";
import { Settings, Puzzle, Code, Save, Upload, Download, Trash2, Github, Loader2, RefreshCw, Search, Package, Tag, AlertTriangle, Store, CheckCircle2 } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";
import { ConfirmDialog } from "@/components/admin/confirm-dialog";
import { getApiErrorMessage } from "@/lib/api-error";
import { formatFileSize } from "@/lib/format";
import { parseGitHubRepo } from "@/lib/github";

const STORE_CACHE_TTL = 5 * 60 * 1000;

type PluginOptimisticAction =
  | { type: "toggle"; id: string; enabled: boolean }
  | { type: "remove"; id: string };

function reduceOptimisticPlugins(plugins: Plugin[], action: PluginOptimisticAction) {
  switch (action.type) {
    case "toggle":
      return plugins.map((plugin) =>
        plugin.id === action.id ? { ...plugin, enabled: action.enabled } : plugin
      );
    case "remove":
      return plugins.filter((plugin) => plugin.id !== action.id);
  }
}

export default function PluginsPage() {
  const { t, locale } = useTranslation();
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [optimisticPlugins, applyOptimisticPlugin] = useOptimistic<Plugin[], PluginOptimisticAction>(
    plugins,
    reduceOptimisticPlugins
  );
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [refreshDone, setRefreshDone] = useState(false);
  const [isRefreshing, startRefreshTransition] = useTransition();
  const [isUploading, startUploadTransition] = useTransition();
  const [pendingToggleId, setPendingToggleId] = useState<string | null>(null);
  const [isTogglePending, startToggleTransition] = useTransition();
  const [pendingDeleteId, setPendingDeleteId] = useState<string | null>(null);
  const [uninstallTarget, setUninstallTarget] = useState<string | null>(null);
  const [isDeletePending, startDeleteTransition] = useTransition();
  const fileInputRef = useRef<HTMLInputElement>(null);
  const refreshDoneTimerRef = useRef<number | null>(null);

  const [repoUrl, setRepoUrl] = useState("");
  const [releases, setReleases] = useState<GitHubReleaseInfo[]>([]);
  const [isLoadingReleases, startLoadReleasesTransition] = useTransition();
  const [pendingAssetUrl, setPendingAssetUrl] = useState<string | null>(null);
  const [isInstallingAsset, startInstallAssetTransition] = useTransition();

  const [storePlugins, setStorePlugins] = useState<StorePluginInfo[]>([]);
  const [isLoadingStore, startLoadStoreTransition] = useTransition();
  const [pendingStorePluginSlug, setPendingStorePluginSlug] = useState<string | null>(null);
  const [isInstallingFromStore, startInstallFromStoreTransition] = useTransition();
  const storeCacheRef = useRef<{ data: StorePluginInfo[]; ts: number } | null>(null);

  const [updates, setUpdates] = useState<Record<string, { current: string; latest: string }>>({});
  const [isCheckingUpdates, startCheckUpdatesTransition] = useTransition();
  const [pendingUpdatePluginId, setPendingUpdatePluginId] = useState<string | null>(null);
  const [isUpdatingPlugin, startUpdatePluginTransition] = useTransition();

  const [settingsOpen, setSettingsOpen] = useState(false);
  const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);
  const [schema, setSchema] = useState<PluginSettingsSchema | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});
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

  const loadPlugins = useCallback(
    async (options: { isRefresh?: boolean; isActive?: () => boolean } = {}) => {
      const { isRefresh = false, isActive = () => true } = options;
      if (!isRefresh) {
        setLoading(true);
      }

      try {
        await pluginsApi.reload();
        const { data } = await pluginsApi.list();
        if (!isActive()) return;

        setPlugins(data?.plugins || []);
        if (isRefresh) {
          markRefreshDone();
        }
      } catch {
        if (!isActive()) return;
        toast.error(t("error.loadFailed"));
        if (!isRefresh) {
          setPlugins([]);
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
        setStorePlugins(storeCacheRef.current.data);
        return;
      }

      try {
        const { data } = await pluginsApi.getStore();
        const plugins = data?.plugins || [];
        setStorePlugins(plugins);
        storeCacheRef.current = { data: plugins, ts: Date.now() };
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
        setStorePlugins([]);
      }
    },
    [t]
  );

  useEffect(() => {
    let active = true;
    void loadPlugins({ isActive: () => active });
    return () => {
      active = false;
    };
  }, [loadPlugins]);

  useEffect(() => {
    return () => {
      if (refreshDoneTimerRef.current) {
        window.clearTimeout(refreshDoneTimerRef.current);
      }
    };
  }, []);

  const refreshPlugins = () => {
    startRefreshTransition(async () => {
      await loadPlugins({ isRefresh: true });
    });
  };

  const fetchReleases = () => {
    const repo = parseGitHubRepo(repoUrl);
    if (!repo) {
      toast.error(t("plugin.enterRepo") || "Please enter a GitHub repo");
      return;
    }

    startLoadReleasesTransition(async () => {
      try {
        const { data } = await pluginsApi.listGitHubReleases(repo);
        setReleases(data || []);
        if (!data?.length) {
          toast.info(t("plugin.noReleases") || "No releases found");
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
        const { data } = await pluginsApi.checkUpdates();
        const updatesMap: Record<string, { current: string; latest: string }> = {};
        data.updates.forEach((update) => {
          updatesMap[update.id] = {
            current: update.current_version,
            latest: update.latest_version,
          };
        });
        setUpdates(updatesMap);
        if (data.updates.length > 0) {
          toast.success(t("plugin.updatesFound", { count: data.updates.length }));
        } else {
          toast.info(t("plugin.allUpToDate"));
        }
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("plugin.checkUpdateFailed")));
      }
    });
  };

  const handleUpdatePlugin = (pluginId: string) => {
    setPendingUpdatePluginId(pluginId);
    startUpdatePluginTransition(async () => {
      try {
        const { data } = await pluginsApi.updatePlugin(pluginId);
        toast.success(data.message);
        setUpdates((current) => {
          const next = { ...current };
          delete next[pluginId];
          return next;
        });
        await loadPlugins({ isRefresh: true });
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("plugin.updateFailed")));
      } finally {
        setPendingUpdatePluginId(null);
      }
    });
  };

  const handleToggle = (plugin: Plugin) => {
    if (!plugin.enabled && !plugin.compatible) {
      toast.error(plugin.compatibility_message || t("plugin.incompatible") || "Plugin is not compatible with current version");
      return;
    }

    const nextEnabled = !plugin.enabled;
    setPendingToggleId(plugin.id);
    startToggleTransition(async () => {
      applyOptimisticPlugin({ type: "toggle", id: plugin.id, enabled: nextEnabled });
      try {
        const { data } = await pluginsApi.toggle(plugin.id, nextEnabled);
        setPlugins((current) =>
          current.map((item) =>
            item.id === plugin.id ? { ...item, ...data, enabled: data?.enabled ?? nextEnabled } : item
          )
        );
        toast.success(plugin.enabled ? t("plugin.disableSuccess") : t("plugin.enableSuccess"));
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("plugin.toggleFailed")));
        await loadPlugins({ isRefresh: true });
      } finally {
        setPendingToggleId(null);
      }
    });
  };

  const handleUpload = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    if (!file) return;

    startUploadTransition(async () => {
      try {
        const { data } = await pluginsApi.uploadPlugin(file);
        toast.success(data.message);
        await loadPlugins({ isRefresh: true });
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
        const { data } = await pluginsApi.installGitHubPlugin(asset.download_url);
        toast.success(data.message);
        await loadPlugins({ isRefresh: true });
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      } finally {
        setPendingAssetUrl(null);
      }
    });
  };

  const handleInstallFromStore = (plugin: StorePluginInfo) => {
    if (!plugin.github_url) {
      toast.error("No GitHub URL available");
      return;
    }

    const githubUrl = plugin.github_url;
    setPendingStorePluginSlug(plugin.slug);
    startInstallFromStoreTransition(async () => {
      try {
        const { data } = await pluginsApi.installFromRepo({
          repo: githubUrl,
          pluginId: plugin.slug,
        });
        toast.success(data.message);
        await loadPlugins({ isRefresh: true });
        await loadStore(true);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      } finally {
        setPendingStorePluginSlug(null);
      }
    });
  };

  const handleUninstall = (pluginId: string) => {
    setUninstallTarget(pluginId);
  };

  const confirmUninstall = () => {
    if (!uninstallTarget) return;

    const pluginId = uninstallTarget;
    setUninstallTarget(null);

    setPendingDeleteId(pluginId);
    startDeleteTransition(async () => {
      applyOptimisticPlugin({ type: "remove", id: pluginId });
      try {
        await pluginsApi.uninstall(pluginId);
        setPlugins((current) => current.filter((plugin) => plugin.id !== pluginId));
        setUpdates((current) => {
          const next = { ...current };
          delete next[pluginId];
          return next;
        });
        toast.success(t("plugin.uninstallSuccess") || "Plugin uninstalled");
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
        await loadPlugins({ isRefresh: true });
      } finally {
        setPendingDeleteId(null);
      }
    });
  };

  const openSettings = (plugin: Plugin) => {
    const requestId = settingsRequestIdRef.current + 1;
    settingsRequestIdRef.current = requestId;
    setSelectedPlugin(plugin);
    setSettingsOpen(true);
    setSchema(null);
    setValues({});

    startLoadSettingsTransition(async () => {
      try {
        const { data } = await pluginsApi.getSettings(plugin.id);
        if (settingsRequestIdRef.current !== requestId) return;
        setSchema(data?.schema || null);
        setValues(parseSettingsValues(data?.values || {}));
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

  const handleSaveSettings = () => {
    if (!selectedPlugin) return;
    startSaveSettingsTransition(async () => {
      try {
        await pluginsApi.updateSettings(selectedPlugin.id, values);
        toast.success(t("plugin.saveSuccess"));
        setSettingsOpen(false);
      } catch {
        toast.error(t("plugin.saveFailed"));
      }
    });
  };

  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = (loading && hasLoaded) || isRefreshing;

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t("plugin.title")}</h1>
          <p className="text-muted-foreground">{t("plugin.description")}</p>
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
            {t("plugin.upload") || "Upload"}
          </Button>
          <Button variant="outline" onClick={checkUpdates} disabled={isCheckingUpdates}>
            {isCheckingUpdates ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Download className="h-4 w-4 mr-2" />
            )}
            {t("plugin.checkUpdates")}
          </Button>
          <Button variant="outline" onClick={refreshPlugins} disabled={isRefreshing}>
            {refreshDone ? (
              <CheckCircle2 className="h-4 w-4 mr-2 text-green-500 animate-in fade-in duration-300" />
            ) : (
              <RefreshCw className={`h-4 w-4 mr-2 transition-transform duration-500 ${isRefreshing ? "animate-spin" : ""}`} />
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
            <Puzzle className="h-4 w-4" />
            {t("plugin.installed") || "Installed"}
          </TabsTrigger>
          <TabsTrigger value="store" className="gap-2" onClick={() => fetchStore()}>
            <Store className="h-4 w-4" />
            {t("plugin.store") || "Store"}
          </TabsTrigger>
          <TabsTrigger value="github" className="gap-2">
            <Github className="h-4 w-4" />
            {t("plugin.online") || "Online"}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="installed">
          {showInitialLoading ? (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {Array.from({ length: 3 }).map((_, i) => (
                <Card key={i}>
                  <CardHeader>
                    <Skeleton className="h-5 w-32" />
                    <Skeleton className="h-4 w-48" />
                  </CardHeader>
                  <CardContent>
                    <Skeleton className="h-4 w-full" />
                  </CardContent>
                </Card>
              ))}
            </div>
          ) : optimisticPlugins.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <Puzzle className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                <p className="text-muted-foreground">{t("plugin.noPlugins")}</p>
              </CardContent>
            </Card>
          ) : (
            <div className={`grid gap-4 md:grid-cols-2 lg:grid-cols-3 transition-opacity ${isSyncing ? "opacity-70" : ""}`}>
              {optimisticPlugins.map((plugin) => {
                const isTogglingPlugin = isTogglePending && pendingToggleId === plugin.id;
                const isDeletingPlugin = isDeletePending && pendingDeleteId === plugin.id;
                const isUpdatingThisPlugin = isUpdatingPlugin && pendingUpdatePluginId === plugin.id;

                return (
                  <Card key={plugin.id} className={!plugin.enabled ? "opacity-60" : ""}>
                    <CardHeader className="pb-3">
                      <div className="flex items-start justify-between">
                        <div className="space-y-1">
                          <CardTitle className="text-lg flex items-center gap-2">
                            <Puzzle className="h-4 w-4" />
                            {plugin.name}
                            {updates[plugin.id] && (
                              <Badge variant="default" className="text-xs">
                                {updates[plugin.id].current} -&gt; {updates[plugin.id].latest}
                              </Badge>
                            )}
                            {!plugin.compatible && (
                              <span title={plugin.compatibility_message || "Not compatible"}>
                                <AlertTriangle className="h-4 w-4 text-amber-500" />
                              </span>
                            )}
                          </CardTitle>
                          <CardDescription className="text-xs">
                            {t("plugin.version")}: {plugin.version}
                            {plugin.author && ` - ${t("plugin.author")}: ${plugin.author}`}
                            {plugin.requires_noteva && ` - ${t("plugin.requires") || "Requires"}: ${plugin.requires_noteva}`}
                          </CardDescription>
                        </div>
                        <Switch
                          checked={plugin.enabled}
                          onCheckedChange={() => handleToggle(plugin)}
                          disabled={isTogglingPlugin || (!plugin.enabled && !plugin.compatible)}
                        />
                      </div>
                      {!plugin.compatible && (
                        <p className="text-xs text-amber-600 dark:text-amber-400 mt-2">
                          {plugin.compatibility_message}
                        </p>
                      )}
                    </CardHeader>
                    <CardContent className="space-y-4">
                      <p className="text-sm text-muted-foreground">
                        {plugin.description || "No description"}
                      </p>

                      {plugin.shortcodes.length > 0 && (
                        <div className="flex flex-wrap gap-1">
                          <Code className="h-4 w-4 text-muted-foreground mr-1" />
                          {plugin.shortcodes.map((shortcode) => (
                            <Badge key={shortcode} variant="secondary" className="text-xs">[{shortcode}]</Badge>
                          ))}
                        </div>
                      )}

                      <div className="flex items-center justify-between pt-2 border-t">
                        <Badge variant={plugin.enabled ? "default" : "secondary"}>
                          {plugin.enabled ? t("plugin.enabled") : t("plugin.disabled")}
                        </Badge>
                        <div className="flex gap-1">
                          {updates[plugin.id] && (
                            <Button
                              variant="default"
                              size="sm"
                              onClick={() => handleUpdatePlugin(plugin.id)}
                              disabled={isUpdatingThisPlugin}
                              title={t("plugin.updateTo", { version: updates[plugin.id].latest })}
                            >
                              {isUpdatingThisPlugin ? (
                                <Loader2 className="h-4 w-4 animate-spin" />
                              ) : (
                                <Download className="h-4 w-4" />
                              )}
                            </Button>
                          )}
                          {plugin.has_settings && (
                            <Button
                              variant="ghost"
                              size="sm"
                              onClick={() => openSettings(plugin)}
                              disabled={!plugin.enabled}
                            >
                              <Settings className="h-4 w-4" />
                            </Button>
                          )}
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleUninstall(plugin.id)}
                            disabled={isDeletingPlugin}
                          >
                            {isDeletingPlugin ? (
                              <Loader2 className="h-4 w-4 animate-spin" />
                            ) : (
                              <Trash2 className="h-4 w-4 text-destructive" />
                            )}
                          </Button>
                        </div>
                      </div>
                    </CardContent>
                  </Card>
                );
              })}
            </div>
          )}
        </TabsContent>

        <TabsContent value="store">
          <Card>
            <CardHeader>
              <CardTitle className="flex items-center gap-2">
                <Store className="h-5 w-5" />
                {t("plugin.officialStore") || "Official Plugin Store"}
              </CardTitle>
              <CardDescription>
                {t("plugin.storeDesc") || "Browse and install official plugins"}
              </CardDescription>
            </CardHeader>
            <CardContent>
              {isLoadingStore ? (
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {Array.from({ length: 6 }).map((_, i) => (
                    <Card key={i}>
                      <CardHeader>
                        <Skeleton className="h-5 w-32" />
                        <Skeleton className="h-4 w-48" />
                      </CardHeader>
                      <CardContent>
                        <Skeleton className="h-4 w-full" />
                      </CardContent>
                    </Card>
                  ))}
                </div>
              ) : storePlugins.length === 0 ? (
                <div className="text-center py-12 text-muted-foreground">
                  <Store className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>{t("plugin.noStorePlugins") || "No plugins available"}</p>
                </div>
              ) : (
                <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
                  {storePlugins.map((plugin) => {
                    const isInstallingPlugin = isInstallingFromStore && pendingStorePluginSlug === plugin.slug;

                    return (
                      <Card key={plugin.slug}>
                        <StorePluginCover plugin={plugin} />
                        <CardHeader className="pb-3">
                          <div className="flex items-start justify-between">
                            <div className="space-y-1">
                              <CardTitle className="text-lg flex items-center gap-2">
                                <Puzzle className="h-4 w-4" />
                                {plugin.name}
                              </CardTitle>
                              <CardDescription className="text-xs">
                                v{plugin.version} - {plugin.author}
                                {plugin.download_count > 0 && ` - ${plugin.download_count} downloads`}
                              </CardDescription>
                            </div>
                          </div>
                        </CardHeader>
                        <CardContent className="space-y-4">
                          <p className="text-sm text-muted-foreground">
                            {plugin.description || "No description"}
                          </p>

                          {plugin.tags.length > 0 && (
                            <div className="flex gap-1 flex-wrap">
                              {plugin.tags.map((tag) => (
                                <Badge key={tag} variant="outline" className="text-xs">{tag}</Badge>
                              ))}
                            </div>
                          )}

                          <div className="flex items-center justify-between pt-2 border-t">
                            {plugin.installed ? (
                              <Badge variant="secondary">{t("plugin.installed") || "Installed"}</Badge>
                            ) : (
                              <Button
                                size="sm"
                                onClick={() => handleInstallFromStore(plugin)}
                                disabled={isInstallingPlugin}
                              >
                                {isInstallingPlugin ? (
                                  <Loader2 className="h-4 w-4 animate-spin" />
                                ) : (
                                  <Download className="h-4 w-4 mr-2" />
                                )}
                                {t("plugin.install") || "Install"}
                              </Button>
                            )}
                            {plugin.github_url && (
                              <a
                                href={plugin.github_url}
                                target="_blank"
                                rel="noopener noreferrer"
                                className="text-xs text-muted-foreground hover:text-foreground"
                              >
                                <Github className="h-4 w-4" />
                              </a>
                            )}
                          </div>
                        </CardContent>
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
                {t("plugin.onlinePlugins") || "Install from GitHub"}
              </CardTitle>
              <CardDescription>
                {t("plugin.onlineDesc") || "Enter a GitHub repository URL to browse releases"}
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
                          {t("plugin.noAssets") || "No downloadable assets"}
                        </p>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {!isLoadingReleases && releases.length === 0 && (
                <div className="text-center py-8 text-muted-foreground">
                  <Github className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>{t("plugin.searchHint") || "Enter a repo and click search"}</p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      <Sheet open={settingsOpen} onOpenChange={handleSettingsOpenChange}>
        <SheetContent className="overflow-y-auto">
          <SheetHeader>
            <SheetTitle className="flex items-center gap-2">
              <Puzzle className="h-5 w-5" />
              {selectedPlugin?.name}
            </SheetTitle>
            <SheetDescription>{t("plugin.settingsTitle")}</SheetDescription>
          </SheetHeader>
          <div className="mt-4 space-y-4">
            {isLoadingSettings ? (
              <div className="flex justify-center py-8 text-muted-foreground">
                <Loader2 className="h-5 w-5 animate-spin" />
              </div>
            ) : (
              <SettingsRenderer
                schema={schema}
                values={values}
                onChange={setValues}
                emptyMessage={t("plugin.noSettings")}
              />
            )}
            {schema?.sections?.length ? (
              <Button onClick={handleSaveSettings} disabled={isSavingSettings || isLoadingSettings} className="w-full">
                {isSavingSettings ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Save className="h-4 w-4 mr-2" />
                )}
                {t("plugin.saveSettings")}
              </Button>
            ) : null}
          </div>
        </SheetContent>
      </Sheet>
      <ConfirmDialog
        open={uninstallTarget !== null}
        title={t("common.confirm")}
        description={
          t("plugin.confirmUninstall")?.replace("{name}", uninstallTarget || "") ||
          `Uninstall plugin "${uninstallTarget || ""}"?`
        }
        confirmLabel={t("common.delete")}
        cancelLabel={t("common.cancel")}
        destructive
        loading={isDeletePending}
        onOpenChange={(open) => !open && setUninstallTarget(null)}
        onConfirm={confirmUninstall}
      />
    </div>
  );
}

function StorePluginCover({ plugin }: { plugin: StorePluginInfo }) {
  const [failed, setFailed] = useState(false);

  if (!plugin.cover_image || failed) {
    return null;
  }

  return (
    <div className="relative h-36 bg-gradient-to-br from-muted to-muted/50 overflow-hidden">
      <img
        src={plugin.cover_image}
        alt={plugin.name}
        className="w-full h-full object-cover"
        onError={() => setFailed(true)}
      />
    </div>
  );
}
