"use client";

import { useEffect, useState, useRef } from "react";
import { pluginsApi, Plugin, PluginSettingsSchema, PluginSettingsField, GitHubReleaseInfo, GitHubAssetInfo, StorePluginInfo } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Settings, Puzzle, Code, Save, Upload, Download, Trash2, Github, Loader2, RefreshCw, Search, Package, Tag, Plus, X, List, AlertTriangle, Store } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";

// 通用数组字段编辑器
interface ArrayFieldEditorProps {
  value: Record<string, unknown>[];
  onChange: (v: Record<string, unknown>[]) => void;
  itemFields: NonNullable<PluginSettingsField["itemFields"]>;
}

function ArrayFieldEditor({ value, onChange, itemFields }: ArrayFieldEditorProps) {
  const items = Array.isArray(value) ? value : [];
  
  const addItem = () => {
    const newItem: Record<string, unknown> = {};
    itemFields.forEach(f => { newItem[f.id] = ""; });
    onChange([...items, newItem]);
  };
  
  const removeItem = (index: number) => {
    onChange(items.filter((_, i) => i !== index));
  };
  
  const updateItem = (index: number, fieldId: string, val: unknown) => {
    const newItems = [...items];
    newItems[index] = { ...newItems[index], [fieldId]: val };
    onChange(newItems);
  };
  
  const moveItem = (from: number, to: number) => {
    if (to < 0 || to >= items.length) return;
    const newItems = [...items];
    const [item] = newItems.splice(from, 1);
    newItems.splice(to, 0, item);
    onChange(newItems);
  };
  
  return (
    <div className="space-y-3">
      {items.map((item, index) => (
        <div key={index} className="border rounded-lg p-3 space-y-2 bg-muted/30">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <List className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-medium">#{index + 1}</span>
            </div>
            <div className="flex items-center gap-1">
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => moveItem(index, index - 1)}
                disabled={index === 0}
              >
                <span className="text-xs">↑</span>
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-7 w-7"
                onClick={() => moveItem(index, index + 1)}
                disabled={index === items.length - 1}
              >
                <span className="text-xs">↓</span>
              </Button>
              <Button
                type="button"
                variant="ghost"
                size="icon"
                className="h-7 w-7 text-destructive hover:text-destructive"
                onClick={() => removeItem(index)}
              >
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>
          <div className="grid gap-2" style={{ gridTemplateColumns: itemFields.length <= 2 ? `repeat(${itemFields.length}, 1fr)` : 'repeat(2, 1fr)' }}>
            {itemFields.map(field => (
              <Input
                key={field.id}
                type={field.type === "number" ? "number" : "text"}
                placeholder={field.placeholder || field.label + (field.required ? " *" : "")}
                value={(item[field.id] as string) || ""}
                onChange={(e) => updateItem(index, field.id, field.type === "number" ? Number(e.target.value) : e.target.value)}
                className={itemFields.length > 2 && itemFields.indexOf(field) >= itemFields.length - (itemFields.length % 2 || 2) ? "col-span-1" : ""}
              />
            ))}
          </div>
        </div>
      ))}
      <Button type="button" variant="outline" className="w-full" onClick={addItem}>
        <Plus className="h-4 w-4 mr-2" />
        添加项目
      </Button>
    </div>
  );
}

export default function PluginsPage() {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [toggling, setToggling] = useState<string | null>(null);
  const [uploading, setUploading] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  
  // GitHub releases
  const [repoUrl, setRepoUrl] = useState("");
  const [releases, setReleases] = useState<GitHubReleaseInfo[]>([]);
  const [loadingReleases, setLoadingReleases] = useState(false);
  const [installingAsset, setInstallingAsset] = useState<string | null>(null);
  
  // Store
  const [storePlugins, setStorePlugins] = useState<StorePluginInfo[]>([]);
  const [loadingStore, setLoadingStore] = useState(false);
  const [installingFromStore, setInstallingFromStore] = useState<string | null>(null);
  
  // Updates
  const [updates, setUpdates] = useState<Record<string, { current: string; latest: string }>>({});
  const [checkingUpdates, setCheckingUpdates] = useState(false);
  const [updatingPlugin, setUpdatingPlugin] = useState<string | null>(null);
  
  // Settings dialog state
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);
  const [schema, setSchema] = useState<PluginSettingsSchema | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [saving, setSaving] = useState(false);

  const fetchPlugins = async () => {
    setLoading(true);
    try {
      // First reload plugins from disk
      await pluginsApi.reload();
      // Then fetch the updated list
      const { data } = await pluginsApi.list();
      setPlugins(data?.plugins || []);
    } catch (error) {
      toast.error(t("error.loadFailed"));
      setPlugins([]);
    } finally {
      setLoading(false);
    }
  };

  const fetchReleases = async () => {
    if (!repoUrl.trim()) {
      toast.error(t("plugin.enterRepo") || "Please enter a GitHub repo");
      return;
    }
    
    // Parse repo from URL or direct input
    let repo = repoUrl.trim();
    const match = repo.match(/github\.com\/([^\/]+\/[^\/]+)/);
    if (match) {
      repo = match[1].replace(/\.git$/, "");
    }
    
    setLoadingReleases(true);
    try {
      const { data } = await pluginsApi.listGitHubReleases(repo);
      setReleases(data || []);
      if (!data?.length) {
        toast.info(t("plugin.noReleases") || "No releases found");
      }
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
      setReleases([]);
    } finally {
      setLoadingReleases(false);
    }
  };

  const fetchStore = async () => {
    setLoadingStore(true);
    try {
      const { data } = await pluginsApi.getStore();
      setStorePlugins(data?.plugins || []);
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
      setStorePlugins([]);
    } finally {
      setLoadingStore(false);
    }
  };

  const checkUpdates = async () => {
    setCheckingUpdates(true);
    try {
      const { data } = await pluginsApi.checkUpdates();
      const updatesMap: Record<string, { current: string; latest: string }> = {};
      data.updates.forEach(u => {
        updatesMap[u.id] = { current: u.current_version, latest: u.latest_version };
      });
      setUpdates(updatesMap);
      if (data.updates.length > 0) {
        toast.success(`发现 ${data.updates.length} 个插件更新`);
      } else {
        toast.info("所有插件都是最新版本");
      }
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || "检查更新失败");
    } finally {
      setCheckingUpdates(false);
    }
  };

  const handleUpdatePlugin = async (pluginId: string) => {
    setUpdatingPlugin(pluginId);
    try {
      const { data } = await pluginsApi.updatePlugin(pluginId);
      toast.success(data.message);
      // Remove from updates list
      setUpdates(prev => {
        const newUpdates = { ...prev };
        delete newUpdates[pluginId];
        return newUpdates;
      });
      fetchPlugins();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || "更新失败");
    } finally {
      setUpdatingPlugin(null);
    }
  };

  useEffect(() => {
    fetchPlugins();
  }, []);

  const handleToggle = async (plugin: Plugin) => {
    // 检查兼容性
    if (!plugin.enabled && !plugin.compatible) {
      toast.error(plugin.compatibility_message || t("plugin.incompatible") || "Plugin is not compatible with current version");
      return;
    }
    
    setToggling(plugin.id);
    try {
      await pluginsApi.toggle(plugin.id, !plugin.enabled);
      toast.success(plugin.enabled ? t("plugin.disableSuccess") : t("plugin.enableSuccess"));
      fetchPlugins();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("plugin.toggleFailed"));
    } finally {
      setToggling(null);
    }
  };

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    
    setUploading(true);
    try {
      const { data } = await pluginsApi.uploadPlugin(file);
      toast.success(data.message);
      fetchPlugins();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setUploading(false);
      if (fileInputRef.current) {
        fileInputRef.current.value = "";
      }
    }
  };

  const handleInstallAsset = async (asset: GitHubAssetInfo) => {
    setInstallingAsset(asset.download_url);
    try {
      const { data } = await pluginsApi.installGitHubPlugin(asset.download_url);
      toast.success(data.message);
      fetchPlugins();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setInstallingAsset(null);
    }
  };

  const handleInstallFromStore = async (plugin: StorePluginInfo) => {
    if (!plugin.compatible) {
      toast.error(plugin.compatibility_message || t("plugin.incompatible"));
      return;
    }
    
    setInstallingFromStore(plugin.id);
    try {
      // Install from repo directly (no need for Release)
      const { data } = await pluginsApi.installFromRepo({
        repo: plugin.homepage,
        pluginId: plugin.id
      });
      toast.success(data.message);
      fetchPlugins();
      fetchStore(); // Refresh store to update installed status
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setInstallingFromStore(null);
    }
  };

  const handleUninstall = async (pluginId: string) => {
    if (!confirm(t("plugin.confirmUninstall")?.replace("{name}", pluginId) || `Uninstall plugin "${pluginId}"?`)) {
      return;
    }
    
    setDeleting(pluginId);
    try {
      await pluginsApi.uninstall(pluginId);
      toast.success(t("plugin.uninstallSuccess") || "Plugin uninstalled");
      fetchPlugins();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setDeleting(null);
    }
  };

  const openSettings = async (plugin: Plugin) => {
    setSelectedPlugin(plugin);
    setSettingsOpen(true);
    try {
      const { data } = await pluginsApi.getSettings(plugin.id);
      setSchema(data?.schema || null);
      setValues(data?.values || {});
    } catch (error) {
      toast.error(t("error.loadFailed"));
    }
  };

  const handleSaveSettings = async () => {
    if (!selectedPlugin) return;
    setSaving(true);
    try {
      await pluginsApi.updateSettings(selectedPlugin.id, values);
      toast.success(t("plugin.saveSuccess"));
      setSettingsOpen(false);
    } catch (error) {
      toast.error(t("plugin.saveFailed"));
    } finally {
      setSaving(false);
    }
  };

  const updateValue = (key: string, value: unknown) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const renderField = (field: PluginSettingsSchema["sections"][0]["fields"][0]) => {
    const value = values[field.id] ?? field.default ?? "";

    switch (field.type) {
      case "text":
        return (
          <Input
            id={field.id}
            value={value as string}
            onChange={(e) => updateValue(field.id, e.target.value)}
          />
        );
      case "textarea":
        return (
          <Textarea
            id={field.id}
            value={value as string}
            onChange={(e) => updateValue(field.id, e.target.value)}
            rows={4}
          />
        );
      case "number":
        return (
          <Input
            id={field.id}
            type="number"
            value={value as number}
            min={field.min}
            max={field.max}
            onChange={(e) => updateValue(field.id, Number(e.target.value))}
          />
        );
      case "switch":
        return (
          <Switch
            id={field.id}
            checked={value as boolean}
            onCheckedChange={(checked) => updateValue(field.id, checked)}
          />
        );
      case "select":
        return (
          <Select value={value as string} onValueChange={(v) => updateValue(field.id, v)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {field.options?.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        );
      case "color":
        return (
          <div className="flex items-center gap-2">
            <Input
              type="color"
              value={value as string}
              onChange={(e) => updateValue(field.id, e.target.value)}
              className="w-12 h-10 p-1"
            />
            <Input
              value={value as string}
              onChange={(e) => updateValue(field.id, e.target.value)}
              className="flex-1"
            />
          </div>
        );
      case "array":
        return field.itemFields ? (
          <ArrayFieldEditor 
            value={value as Record<string, unknown>[]} 
            onChange={(v) => updateValue(field.id, v)} 
            itemFields={field.itemFields}
          />
        ) : null;
      default:
        return (
          <Input
            id={field.id}
            value={value as string}
            onChange={(e) => updateValue(field.id, e.target.value)}
          />
        );
    }
  };

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
            disabled={uploading}
          >
            {uploading ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Upload className="h-4 w-4 mr-2" />
            )}
            {t("plugin.upload") || "Upload"}
          </Button>
          <Button variant="outline" onClick={checkUpdates} disabled={checkingUpdates}>
            {checkingUpdates ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Download className="h-4 w-4 mr-2" />
            )}
            检查更新
          </Button>
          <Button variant="outline" onClick={fetchPlugins} disabled={loading}>
            <RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
            {t("common.refresh") || "Refresh"}
          </Button>
        </div>
      </div>

      <Tabs defaultValue="installed" className="space-y-4">
        <TabsList>
          <TabsTrigger value="installed" className="gap-2">
            <Puzzle className="h-4 w-4" />
            {t("plugin.installed") || "Installed"}
          </TabsTrigger>
          <TabsTrigger value="store" className="gap-2" onClick={fetchStore}>
            <Store className="h-4 w-4" />
            {t("plugin.store") || "Store"}
          </TabsTrigger>
          <TabsTrigger value="github" className="gap-2">
            <Github className="h-4 w-4" />
            {t("plugin.online") || "Online"}
          </TabsTrigger>
        </TabsList>

        <TabsContent value="installed">
          {loading ? (
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
          ) : plugins.length === 0 ? (
            <Card>
              <CardContent className="py-12 text-center">
                <Puzzle className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
                <p className="text-muted-foreground">{t("plugin.noPlugins")}</p>
              </CardContent>
            </Card>
          ) : (
            <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
              {plugins.map((plugin) => (
                <Card key={plugin.id} className={!plugin.enabled ? "opacity-60" : ""}>
                  <CardHeader className="pb-3">
                    <div className="flex items-start justify-between">
                      <div className="space-y-1">
                        <CardTitle className="text-lg flex items-center gap-2">
                          <Puzzle className="h-4 w-4" />
                          {plugin.name}
                          {updates[plugin.id] && (
                            <Badge variant="default" className="text-xs">
                              {updates[plugin.id].current} → {updates[plugin.id].latest}
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
                          {plugin.author && ` · ${t("plugin.author")}: ${plugin.author}`}
                          {plugin.requires_noteva && ` · ${t("plugin.requires") || "Requires"}: ${plugin.requires_noteva}`}
                        </CardDescription>
                      </div>
                      <Switch
                        checked={plugin.enabled}
                        onCheckedChange={() => handleToggle(plugin)}
                        disabled={toggling === plugin.id || (!plugin.enabled && !plugin.compatible)}
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
                        {plugin.shortcodes.map((sc) => (
                          <Badge key={sc} variant="secondary" className="text-xs">[{sc}]</Badge>
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
                            disabled={updatingPlugin === plugin.id}
                            title={`更新到 ${updates[plugin.id].latest}`}
                          >
                            {updatingPlugin === plugin.id ? (
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
                          disabled={deleting === plugin.id}
                        >
                          {deleting === plugin.id ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                          ) : (
                            <Trash2 className="h-4 w-4 text-destructive" />
                          )}
                        </Button>
                      </div>
                    </div>
                  </CardContent>
                </Card>
              ))}
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
              {loadingStore ? (
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
                  {storePlugins.map((plugin) => (
                    <Card key={plugin.id} className={!plugin.compatible ? "opacity-60" : ""}>
                      <CardHeader className="pb-3">
                        <div className="flex items-start justify-between">
                          <div className="space-y-1">
                            <CardTitle className="text-lg flex items-center gap-2">
                              <Puzzle className="h-4 w-4" />
                              {plugin.name}
                              {!plugin.compatible && (
                                <span title={plugin.compatibility_message || "Not compatible"}>
                                  <AlertTriangle className="h-4 w-4 text-amber-500" />
                                </span>
                              )}
                            </CardTitle>
                            <CardDescription className="text-xs">
                              v{plugin.version} · {plugin.author}
                              {plugin.requires_noteva && ` · ${t("plugin.requires") || "Requires"}: ${plugin.requires_noteva}`}
                            </CardDescription>
                          </div>
                        </div>
                        {!plugin.compatible && plugin.compatibility_message && (
                          <p className="text-xs text-amber-600 dark:text-amber-400 mt-2">
                            {plugin.compatibility_message}
                          </p>
                        )}
                      </CardHeader>
                      <CardContent className="space-y-4">
                        <p className="text-sm text-muted-foreground">
                          {plugin.description || "No description"}
                        </p>
                        
                        <div className="flex items-center justify-between pt-2 border-t">
                          {plugin.installed ? (
                            <Badge variant="secondary">{t("plugin.installed") || "Installed"}</Badge>
                          ) : (
                            <Button
                              size="sm"
                              onClick={() => handleInstallFromStore(plugin)}
                              disabled={installingFromStore === plugin.id || !plugin.compatible}
                            >
                              {installingFromStore === plugin.id ? (
                                <Loader2 className="h-4 w-4 animate-spin" />
                              ) : (
                                <Download className="h-4 w-4 mr-2" />
                              )}
                              {t("plugin.install") || "Install"}
                            </Button>
                          )}
                          {plugin.homepage && (
                            <a
                              href={plugin.homepage}
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
                  ))}
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
                  onChange={(e) => setRepoUrl(e.target.value)}
                  onKeyDown={(e) => e.key === "Enter" && fetchReleases()}
                />
                <Button onClick={fetchReleases} disabled={loadingReleases}>
                  {loadingReleases ? (
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
                            {release.published_at && ` · ${new Date(release.published_at).toLocaleDateString()}`}
                          </p>
                        </div>
                      </div>
                      
                      {release.assets.length > 0 ? (
                        <div className="grid gap-2">
                          {release.assets.map((asset) => (
                            <div
                              key={asset.download_url}
                              className="flex items-center justify-between p-2 bg-muted/50 rounded"
                            >
                              <div className="flex items-center gap-2">
                                <Package className="h-4 w-4 text-muted-foreground" />
                                <span className="text-sm">{asset.name}</span>
                                <span className="text-xs text-muted-foreground">
                                  ({formatSize(asset.size)})
                                </span>
                              </div>
                              <Button
                                size="sm"
                                onClick={() => handleInstallAsset(asset)}
                                disabled={installingAsset === asset.download_url}
                              >
                                {installingAsset === asset.download_url ? (
                                  <Loader2 className="h-4 w-4 animate-spin" />
                                ) : (
                                  <Download className="h-4 w-4" />
                                )}
                              </Button>
                            </div>
                          ))}
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

              {!loadingReleases && releases.length === 0 && (
                <div className="text-center py-8 text-muted-foreground">
                  <Github className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>{t("plugin.searchHint") || "Enter a repo and click search"}</p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Settings Dialog */}
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Puzzle className="h-5 w-5" />
              {selectedPlugin?.name}
            </DialogTitle>
            <DialogDescription>{t("plugin.settingsTitle")}</DialogDescription>
          </DialogHeader>
          
          <div className="mt-4 space-y-6">
            {schema?.sections?.length ? (
              <>
                {schema.sections.map((section) => (
                  <div key={section.id} className="space-y-4">
                    <h3 className="font-medium">{section.title}</h3>
                    {section.fields.map((field) => (
                      <div key={field.id} className="space-y-2">
                        <div className="flex items-center justify-between">
                          <Label htmlFor={field.id}>{field.label}</Label>
                          {field.type === "switch" && renderField(field)}
                        </div>
                        {field.type !== "switch" && renderField(field)}
                        {field.description && (
                          <p className="text-xs text-muted-foreground">{field.description}</p>
                        )}
                      </div>
                    ))}
                  </div>
                ))}
                <Button onClick={handleSaveSettings} disabled={saving} className="w-full">
                  <Save className="h-4 w-4 mr-2" />
                  {t("plugin.saveSettings")}
                </Button>
              </>
            ) : (
              <p className="text-center text-muted-foreground py-8">{t("plugin.noSettings")}</p>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
