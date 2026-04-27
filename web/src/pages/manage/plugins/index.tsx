
import { useCallback, useEffect, useOptimistic, useRef, useState, useTransition } from "react";
import { pluginsApi, Plugin, PluginSettingsSchema, StorePluginInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Sheet, SheetContent, SheetHeader, SheetTitle, SheetDescription } from "@/components/ui/sheet";
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from "@/components/ui/dialog";
import { SettingsRenderer, parseSettingsValues } from "@/components/settings-renderer";
import {
  AlertTriangle,
  Code,
  Download,
  ExternalLink,
  Github,
  Loader2,
  Puzzle,
  RefreshCw,
  Save,
  Settings,
  Store,
  Trash2,
  Upload,
  CheckCircle2,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { ConfirmDialog } from "@/components/admin/confirm-dialog";
import { getApiErrorMessage } from "@/lib/api-error";
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
  const { t } = useTranslation();
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

  const [installOpen, setInstallOpen] = useState(false);
  const [repoUrl, setRepoUrl] = useState("");
  const [isInstallingRepo, startInstallRepoTransition] = useTransition();

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
      } catch (error) {
        if (!isActive()) return;
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
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
        storeCacheRef.current = null;
        if (storePlugins.length > 0) await loadStore(true);
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
        setInstallOpen(false);
        await loadPlugins({ isRefresh: true });
        storeCacheRef.current = null;
        if (storePlugins.length > 0) await loadStore(true);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      } finally {
        if (fileInputRef.current) {
          fileInputRef.current.value = "";
        }
      }
    });
  };

  const handleInstallFromRepo = () => {
    const repo = parseGitHubRepo(repoUrl);
    if (!repo) {
      toast.error(t("plugin.enterRepo"));
      return;
    }

    startInstallRepoTransition(async () => {
      try {
        const { data } = await pluginsApi.installFromRepo({ repo });
        toast.success(data.message);
        setInstallOpen(false);
        setRepoUrl("");
        await loadPlugins({ isRefresh: true });
        storeCacheRef.current = null;
        if (storePlugins.length > 0) await loadStore(true);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("error.loadFailed")));
      }
    });
  };

  const handleInstallFromStore = (plugin: StorePluginInfo) => {
    if (!plugin.github_url) {
      toast.error(t("plugin.noGitHubUrl") || "No GitHub URL available");
      return;
    }

    setPendingStorePluginSlug(plugin.slug);
    startInstallFromStoreTransition(async () => {
      try {
        const { data } = await pluginsApi.installFromRepo({
          repo: plugin.github_url || "",
          pluginId: plugin.plugin_id || undefined,
          storeSlug: plugin.slug,
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
        storeCacheRef.current = null;
        if (storePlugins.length > 0) await loadStore(true);
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
        toast.error(t("plugin.saveFailed") || "Save failed");
      }
    });
  };

  const showInitialLoading = loading && !hasLoaded;
  const showStoreLoading = isLoadingStore && storePlugins.length === 0;

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
            onClick={() => setInstallOpen(true)}
            disabled={isUploading || isInstallingRepo}
          >
            {isUploading ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Download className="h-4 w-4 mr-2" />
            )}
            {t("plugin.installPlugin")}
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

      <div>
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
              <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3 transition-opacity duration-200 ease-out">
                {optimisticPlugins.map((plugin) => {
                  const isTogglingPlugin = isTogglePending && pendingToggleId === plugin.id;
                  const isDeletingPlugin = isDeletePending && pendingDeleteId === plugin.id;
                  const isUpdatingThisPlugin = isUpdatingPlugin && pendingUpdatePluginId === plugin.id;

                  return (
                    <Card key={plugin.id} className={!plugin.enabled ? "opacity-60" : ""}>
                      <CardHeader className="pb-3">
                        <div className="flex items-start justify-between gap-3">
                          <div className="min-w-0 space-y-1">
                            <CardTitle className="flex items-center gap-2 text-lg">
                              <Puzzle className="h-4 w-4 shrink-0" />
                              <span className="truncate">{plugin.name}</span>
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
                          <p className="mt-2 text-xs text-amber-600 dark:text-amber-400">
                            {plugin.compatibility_message}
                          </p>
                        )}
                      </CardHeader>
                      <CardContent className="space-y-4">
                        <p className="line-clamp-3 text-sm text-muted-foreground">
                          {plugin.description || "No description"}
                        </p>

                        {plugin.shortcodes.length > 0 && (
                          <div className="flex flex-wrap gap-1">
                            <Code className="h-4 w-4 text-muted-foreground mr-1" />
                            {plugin.shortcodes.map((shortcode) => (
                              <Badge key={shortcode} variant="secondary" className="text-xs">
                                [{shortcode}]
                              </Badge>
                            ))}
                          </div>
                        )}

                        <div className="flex items-center justify-between border-t pt-2">
                          <Badge variant={plugin.enabled ? "default" : "secondary"}>
                            {plugin.enabled ? t("plugin.enabled") : t("plugin.disabled")}
                          </Badge>
                          <div className="flex gap-1">
                            {plugin.repository && (
                              <Button variant="ghost" size="sm" asChild>
                                <a href={toGitHubHref(plugin.repository)} target="_blank" rel="noopener noreferrer">
                                  <ExternalLink className="h-4 w-4" />
                                </a>
                              </Button>
                            )}
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
                {showStoreLoading ? (
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
                            <div className="flex items-start justify-between gap-3">
                              <div className="min-w-0 space-y-1">
                                <CardTitle className="flex items-center gap-2 text-lg">
                                  <Puzzle className="h-4 w-4 shrink-0" />
                                  <span className="truncate">{plugin.name}</span>
                                </CardTitle>
                                <CardDescription className="text-xs">
                                  v{plugin.version}
                                  {plugin.author && ` - ${plugin.author}`}
                                  {plugin.download_count > 0 && ` - ${plugin.download_count}`}
                                </CardDescription>
                              </div>
                              {plugin.github_url && (
                                <a
                                  href={plugin.github_url}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="text-muted-foreground hover:text-foreground"
                                >
                                  <Github className="h-4 w-4" />
                                </a>
                              )}
                            </div>
                          </CardHeader>
                          <CardContent className="space-y-4">
                            <p className="line-clamp-3 text-sm text-muted-foreground">
                              {plugin.description || "No description"}
                            </p>
                            {plugin.tags.length > 0 && (
                              <div className="flex flex-wrap gap-1">
                                {plugin.tags.map((tag) => (
                                  <Badge key={tag} variant="outline" className="text-xs">
                                    {tag}
                                  </Badge>
                                ))}
                              </div>
                            )}
                            <Button
                              onClick={() => handleInstallFromStore(plugin)}
                              disabled={isInstallingPlugin || plugin.installed}
                              size="sm"
                              className="w-full"
                            >
                              {isInstallingPlugin ? (
                                <Loader2 className="h-4 w-4 animate-spin" />
                              ) : plugin.installed ? (
                                t("plugin.installed") || "Installed"
                              ) : (
                                <>
                                  <Download className="h-4 w-4 mr-2" />
                                  {t("plugin.install") || "Install"}
                                </>
                              )}
                            </Button>
                          </CardContent>
                        </Card>
                      );
                    })}
                  </div>
                )}
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </div>

      <Dialog open={installOpen} onOpenChange={setInstallOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Puzzle className="h-5 w-5" />
              {t("plugin.installPlugin")}
            </DialogTitle>
            <DialogDescription>
              {t("plugin.upload")} / {t("plugin.onlinePlugins")}
            </DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            <div className="rounded-lg border p-4">
              <div className="mb-3 flex items-center gap-2 text-sm font-medium">
                <Upload className="h-4 w-4" />
                {t("plugin.upload")}
              </div>
              <Button
                variant="outline"
                className="w-full"
                onClick={() => fileInputRef.current?.click()}
                disabled={isUploading}
              >
                {isUploading ? (
                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                ) : (
                  <Upload className="h-4 w-4 mr-2" />
                )}
                {t("plugin.upload")}
              </Button>
            </div>

            <div className="rounded-lg border p-4">
              <div className="mb-3 flex items-center gap-2 text-sm font-medium">
                <Github className="h-4 w-4" />
                {t("plugin.onlinePlugins")}
              </div>
              <div className="flex gap-2">
                <Input
                  placeholder="owner/repo or https://github.com/owner/repo"
                  value={repoUrl}
                  onChange={(event) => setRepoUrl(event.target.value)}
                  onKeyDown={(event) => event.key === "Enter" && handleInstallFromRepo()}
                />
                <Button onClick={handleInstallFromRepo} disabled={isInstallingRepo}>
                  {isInstallingRepo ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Download className="h-4 w-4" />
                  )}
                </Button>
              </div>
            </div>
          </div>
        </DialogContent>
      </Dialog>

      <Sheet open={settingsOpen} onOpenChange={handleSettingsOpenChange}>
        <SheetContent className="overflow-y-auto">
          <SheetHeader>
            <SheetTitle>{selectedPlugin?.name}</SheetTitle>
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
                emptyMessage={t("plugin.noSettings") || "No settings available"}
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

function toGitHubHref(repository: string) {
  if (repository.startsWith("http")) return repository;
  return `https://github.com/${repository}`;
}

function StorePluginCover({ plugin }: { plugin: StorePluginInfo }) {
  const [failed, setFailed] = useState(false);
  const showImage = Boolean(plugin.cover_image) && !failed;

  return (
    <div className="relative h-32 overflow-hidden bg-muted/60">
      {showImage ? (
        <img
          src={plugin.cover_image || ""}
          alt={plugin.name}
          className="h-full w-full object-cover"
          onError={() => setFailed(true)}
        />
      ) : (
        <div className="flex h-full items-center justify-center">
          <Puzzle className="h-10 w-10 text-muted-foreground/30" />
        </div>
      )}
      {plugin.installed ? (
        <Badge className="absolute right-2 top-2" variant="secondary">
          Installed
        </Badge>
      ) : null}
    </div>
  );
}
