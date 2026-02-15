import { useEffect, useState, useRef } from "react";
import { adminApi, ThemeResponse, GitHubReleaseInfo, GitHubAssetInfo, StoreThemeInfo, PluginSettingsSchema, PluginSettingsField } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Palette, Check, RefreshCw, ExternalLink, User, Tag, Upload, Download, Trash2, Github, Loader2, Search, Package, AlertTriangle, Store, Settings, Save } from "lucide-react";
import { toast } from "sonner";
import { cn } from "@/lib/utils";
import { useTranslation } from "@/lib/i18n";

export default function ThemesPage() {
  const { t } = useTranslation();
  const [themes, setThemes] = useState<ThemeResponse[]>([]);
  const [currentTheme, setCurrentTheme] = useState("");
  const [loading, setLoading] = useState(true);
  const [switching, setSwitching] = useState(false);
  const [uploading, setUploading] = useState(false);
  const [deleting, setDeleting] = useState<string | null>(null);
  const fileInputRef = useRef<HTMLInputElement>(null);
  
  // GitHub releases
  const [repoUrl, setRepoUrl] = useState("");
  const [releases, setReleases] = useState<GitHubReleaseInfo[]>([]);
  const [loadingReleases, setLoadingReleases] = useState(false);
  const [installingAsset, setInstallingAsset] = useState<string | null>(null);
  
  // Store
  const [storeThemes, setStoreThemes] = useState<StoreThemeInfo[]>([]);
  const [loadingStore, setLoadingStore] = useState(false);
  const [installingFromStore, setInstallingFromStore] = useState<string | null>(null);
  
  // Updates
  const [updates, setUpdates] = useState<Record<string, { current: string; latest: string }>>({});
  const [checkingUpdates, setCheckingUpdates] = useState(false);

  // Theme settings dialog
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [selectedTheme, setSelectedTheme] = useState<ThemeResponse | null>(null);
  const [settingsSchema, setSettingsSchema] = useState<PluginSettingsSchema | null>(null);
  const [settingsValues, setSettingsValues] = useState<Record<string, unknown>>({});
  const [savingSettings, setSavingSettings] = useState(false);

  const fetchThemes = async () => {
    setLoading(true);
    try {
      // First reload themes from disk
      await adminApi.reloadThemes();
      // Then fetch the updated list
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

  const fetchReleases = async () => {
    if (!repoUrl.trim()) {
      toast.error(t("theme.enterRepo") || "Please enter a GitHub repo");
      return;
    }
    
    // Parse repo from URL or direct input
    let repo = repoUrl.trim();
    // Handle full GitHub URLs
    const match = repo.match(/github\.com\/([^\/]+\/[^\/]+)/);
    if (match) {
      repo = match[1].replace(/\.git$/, "");
    }
    
    setLoadingReleases(true);
    try {
      const { data } = await adminApi.listGitHubReleases(repo);
      setReleases(data || []);
      if (!data?.length) {
        toast.info(t("theme.noReleases") || "No releases found");
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
      const { data } = await adminApi.getThemeStore();
      setStoreThemes(data?.themes || []);
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
      setStoreThemes([]);
    } finally {
      setLoadingStore(false);
    }
  };

  const checkUpdates = async () => {
    setCheckingUpdates(true);
    try {
      const { data } = await adminApi.checkThemeUpdates();
      const updatesMap: Record<string, { current: string; latest: string }> = {};
      data.updates.forEach(u => {
        updatesMap[u.name] = { current: u.current_version, latest: u.latest_version };
      });
      setUpdates(updatesMap);
      if (data.updates.length > 0) {
        toast.success(`发现 ${data.updates.length} 个主题更新`);
      } else {
        toast.info("所有主题都是最新版本");
      }
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || "检查更新失败");
    } finally {
      setCheckingUpdates(false);
    }
  };

  const handleUpdateTheme = async (themeName: string) => {
    if (themeName === currentTheme) {
      if (!confirm("当前主题正在使用中，更新后需要刷新页面。是否继续？")) {
        return;
      }
    }
    
    setSwitching(true);
    try {
      const { data } = await adminApi.updateTheme(themeName);
      toast.success(data.message);
      // Remove from updates list
      setUpdates(prev => {
        const newUpdates = { ...prev };
        delete newUpdates[themeName];
        return newUpdates;
      });
      fetchThemes();
      if (themeName === currentTheme) {
        toast.info("主题已更新，请刷新页面查看效果");
      }
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || "更新失败");
    } finally {
      setSwitching(false);
    }
  };

  useEffect(() => {
    fetchThemes();
  }, []);

  const handleSwitchTheme = async (themeName: string) => {
    if (themeName === currentTheme) return;
    
    // 检查兼容性
    const theme = themes.find(t => t.name === themeName);
    if (theme && !theme.compatible) {
      toast.error(theme.compatibility_message || t("theme.incompatible") || "Theme is not compatible with current version");
      return;
    }
    
    setSwitching(true);
    try {
      const { data } = await adminApi.switchTheme(themeName);
      // Verify the switch actually succeeded (check returned theme name matches)
      if (data?.name && data.name !== themeName) {
        toast.error(t("settings.switchFailed") + `: ${data.name}`);
      } else {
        setCurrentTheme(themeName);
        toast.success(t("settings.switchSuccess"));
      }
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("settings.switchFailed"));
    } finally {
      setSwitching(false);
    }
  };

  const handleUpload = async (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    
    setUploading(true);
    try {
      const { data } = await adminApi.uploadTheme(file);
      toast.success(data.message);
      fetchThemes();
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
      const { data } = await adminApi.installGitHubTheme(asset.download_url);
      toast.success(data.message);
      fetchThemes();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setInstallingAsset(null);
    }
  };

  const handleInstallFromStore = async (theme: StoreThemeInfo) => {
    setInstallingFromStore(theme.slug);
    try {
      if (!theme.github_url) {
        toast.error("No GitHub URL available");
        return;
      }
      
      const { data } = await adminApi.installThemeFromRepo(theme.github_url, theme.slug);
      toast.success(data.message);
      fetchThemes();
      fetchStore(); // Refresh store to update installed status
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setInstallingFromStore(null);
    }
  };

  const handleDelete = async (themeName: string) => {
    if (!confirm(t("theme.confirmDelete")?.replace("{name}", themeName) || `Delete theme "${themeName}"?`)) {
      return;
    }
    
    setDeleting(themeName);
    try {
      await adminApi.deleteTheme(themeName);
      toast.success(t("theme.deleteSuccess") || "Theme deleted");
      fetchThemes();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("error.loadFailed"));
    } finally {
      setDeleting(null);
    }
  };

  const formatSize = (bytes: number) => {
    if (bytes < 1024) return `${bytes} B`;
    if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
    return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
  };

  const openThemeSettings = async (theme: ThemeResponse) => {
    setSelectedTheme(theme);
    setSettingsOpen(true);
    try {
      const { data } = await adminApi.getThemeSettings(theme.name);
      setSettingsSchema(data?.schema || null);
      setSettingsValues(data?.values || {});
    } catch {
      toast.error(t("error.loadFailed"));
    }
  };

  const handleSaveThemeSettings = async () => {
    if (!selectedTheme) return;
    setSavingSettings(true);
    try {
      await adminApi.updateThemeSettings(selectedTheme.name, settingsValues);
      toast.success(t("plugin.saveSuccess") || "Settings saved");
      setSettingsOpen(false);
    } catch {
      toast.error(t("plugin.saveFailed") || "Save failed");
    } finally {
      setSavingSettings(false);
    }
  };

  const renderSettingsField = (field: PluginSettingsField) => {
    const value = settingsValues[field.id] ?? field.default ?? "";
    switch (field.type) {
      case "switch":
        return (
          <Switch
            checked={!!value}
            onCheckedChange={(v) => setSettingsValues({ ...settingsValues, [field.id]: v })}
          />
        );
      case "textarea":
        return (
          <Textarea
            value={String(value)}
            onChange={(e) => setSettingsValues({ ...settingsValues, [field.id]: e.target.value })}
          />
        );
      case "select":
        return (
          <Select
            value={String(value)}
            onValueChange={(v) => setSettingsValues({ ...settingsValues, [field.id]: v })}
          >
            <SelectTrigger><SelectValue /></SelectTrigger>
            <SelectContent>
              {field.options?.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        );
      case "number":
        return (
          <Input
            type="number"
            value={String(value)}
            onChange={(e) => setSettingsValues({ ...settingsValues, [field.id]: Number(e.target.value) })}
            min={field.min}
            max={field.max}
          />
        );
      default:
        return (
          <Input
            type={field.secret ? "password" : "text"}
            value={String(value)}
            onChange={(e) => setSettingsValues({ ...settingsValues, [field.id]: e.target.value })}
          />
        );
    }
  };

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
            disabled={uploading}
          >
            {uploading ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Upload className="h-4 w-4 mr-2" />
            )}
            {t("theme.upload") || "Upload"}
          </Button>
          <Button variant="outline" onClick={checkUpdates} disabled={checkingUpdates}>
            {checkingUpdates ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Download className="h-4 w-4 mr-2" />
            )}
            检查更新
          </Button>
          <Button variant="outline" onClick={fetchThemes} disabled={loading}>
            <RefreshCw className={cn("h-4 w-4 mr-2", loading && "animate-spin")} />
            {t("common.refresh") || "Refresh"}
          </Button>
        </div>
      </div>

      <Tabs defaultValue="installed" className="space-y-4">
        <TabsList>
          <TabsTrigger value="installed" className="gap-2">
            <Palette className="h-4 w-4" />
            {t("theme.installed") || "Installed"}
          </TabsTrigger>
          <TabsTrigger value="store" className="gap-2" onClick={fetchStore}>
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
                    <ThemeCard
                      key={theme.name}
                      theme={theme}
                      isActive={currentTheme === theme.name}
                      isDefault={theme.name === "default"}
                      switching={switching}
                      deleting={deleting === theme.name}
                      onSwitch={() => handleSwitchTheme(theme.name)}
                      onDelete={() => handleDelete(theme.name)}
                      onSettings={theme.has_settings ? () => openThemeSettings(theme) : undefined}
                      onUpdate={updates[theme.name] ? () => handleUpdateTheme(theme.name) : undefined}
                      updateInfo={updates[theme.name]}
                      t={t}
                    />
                  ))}
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
              {loadingStore ? (
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
                  {storeThemes.map((theme) => (
                    <Card key={theme.slug}>
                      <div className="relative h-36 bg-gradient-to-br from-muted to-muted/50 flex items-center justify-center overflow-hidden">
                        {theme.cover_image ? (
                          <img
                            src={theme.cover_image}
                            alt={theme.name}
                            className="w-full h-full object-cover"
                            onError={(e) => {
                              (e.target as HTMLImageElement).style.display = 'none';
                              (e.target as HTMLImageElement).nextElementSibling?.classList.remove('hidden');
                            }}
                          />
                        ) : null}
                        <Palette className={cn("h-12 w-12 text-muted-foreground/30 absolute", theme.cover_image && "hidden")} />
                        {theme.installed && (
                          <div className="absolute top-2 right-2 flex items-center gap-1 bg-primary text-primary-foreground px-2 py-1 rounded text-xs">
                            <Check className="h-3 w-3" />
                            {t("theme.installed") || "Installed"}
                          </div>
                        )}
                      </div>
                      <div className="p-4">
                        <div className="flex items-start justify-between mb-2">
                          <div>
                            <h3 className="font-semibold text-lg">{theme.name}</h3>
                            <div className="flex items-center gap-2 text-xs text-muted-foreground">
                              <Tag className="h-3 w-3" />
                              <span>v{theme.version}</span>
                              {theme.author && (
                                <>
                                  <span>·</span>
                                  <User className="h-3 w-3" />
                                  <span>{theme.author}</span>
                                </>
                              )}
                              {theme.download_count > 0 && (
                                <>
                                  <span>·</span>
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
                          disabled={installingFromStore === theme.slug || theme.installed}
                          size="sm"
                          className="w-full"
                        >
                          {installingFromStore === theme.slug ? (
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
                          {t("theme.noAssets") || "No downloadable assets"}
                        </p>
                      )}
                    </div>
                  ))}
                </div>
              )}

              {!loadingReleases && releases.length === 0 && repoUrl && (
                <div className="text-center py-8 text-muted-foreground">
                  <Github className="h-12 w-12 mx-auto mb-4 opacity-50" />
                  <p>{t("theme.searchHint") || "Enter a repo and click search"}</p>
                </div>
              )}
            </CardContent>
          </Card>
        </TabsContent>
      </Tabs>

      {/* Theme Settings Dialog */}
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle>
              {selectedTheme?.display_name}
            </DialogTitle>
            <DialogDescription>{t("plugin.settingsTitle") || "Settings"}</DialogDescription>
          </DialogHeader>
          <div className="space-y-4">
            {settingsSchema?.sections?.length ? (
              <>
                {settingsSchema.sections.map((section: any) => (
                  <div key={section.title} className="space-y-3">
                    {section.title && (
                      <h4 className="font-medium text-sm text-muted-foreground">{section.title}</h4>
                    )}
                    {section.fields?.map((field: PluginSettingsField) => (
                      <div key={field.id} className="space-y-1.5">
                        <Label>{field.label}</Label>
                        {renderSettingsField(field)}
                        {field.description && (
                          <p className="text-xs text-muted-foreground">{field.description}</p>
                        )}
                      </div>
                    ))}
                  </div>
                ))}
                <Button onClick={handleSaveThemeSettings} disabled={savingSettings} className="w-full">
                  <Save className="h-4 w-4 mr-2" />
                  {t("plugin.saveSettings") || "Save"}
                </Button>
              </>
            ) : (
              <p className="text-center text-muted-foreground py-8">{t("plugin.noSettings") || "No settings available"}</p>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}

interface ThemeCardProps {
  theme: ThemeResponse;
  isActive: boolean;
  isDefault: boolean;
  switching: boolean;
  deleting: boolean;
  onSwitch: () => void;
  onDelete: () => void;
  onSettings?: () => void;
  onUpdate?: () => void;
  updateInfo?: { current: string; latest: string };
  t: (key: string) => string;
}

function ThemeCard({ theme, isActive, isDefault, switching, deleting, onSwitch, onDelete, onSettings, onUpdate, updateInfo, t }: ThemeCardProps) {
  return (
    <div
      className={cn(
        "relative rounded-lg border-2 overflow-hidden transition-all hover:border-primary hover:shadow-md",
        isActive ? "border-primary bg-primary/5 shadow-sm" : "border-muted",
        !theme.compatible && "opacity-70"
      )}
    >
      <div className="relative h-36 bg-gradient-to-br from-muted to-muted/50 flex items-center justify-center">
        {theme.preview ? (
          <img
            src={`/themes/${theme.name}/${theme.preview}`}
            alt={theme.display_name}
            className="w-full h-full object-cover"
            onError={(e) => {
              (e.target as HTMLImageElement).style.display = 'none';
            }}
          />
        ) : (
          <Palette className="h-12 w-12 text-muted-foreground/30" />
        )}
        {isActive && (
          <div className="absolute top-2 right-2 flex items-center gap-1 bg-primary text-primary-foreground px-2 py-1 rounded text-xs">
            <Check className="h-3 w-3" />
            {t("settings.currentTheme")}
          </div>
        )}
        {!theme.compatible && (
          <div className="absolute top-2 left-2 flex items-center gap-1 bg-amber-500 text-white px-2 py-1 rounded text-xs">
            <AlertTriangle className="h-3 w-3" />
            {t("theme.incompatible") || "Incompatible"}
          </div>
        )}
      </div>

      <div className="p-4">
        <div className="flex items-start justify-between mb-2">
          <div>
            <h3 className="font-semibold text-lg flex items-center gap-2">
              {theme.display_name}
              {updateInfo && (
                <Badge variant="default" className="text-xs">
                  {updateInfo.current} → {updateInfo.latest}
                </Badge>
              )}
              {!theme.compatible && (
                <span title={theme.compatibility_message || "Not compatible"}>
                  <AlertTriangle className="h-4 w-4 text-amber-500" />
                </span>
              )}
            </h3>
            <div className="flex items-center gap-2 text-xs text-muted-foreground">
              <Tag className="h-3 w-3" />
              <span>v{theme.version}</span>
              {theme.author && (
                <>
                  <span>·</span>
                  <User className="h-3 w-3" />
                  <span>{theme.author}</span>
                </>
              )}
              {theme.requires_noteva && (
                <>
                  <span>·</span>
                  <span>{t("theme.requires") || "Requires"}: {theme.requires_noteva}</span>
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
              disabled={switching}
              variant="default"
              size="sm"
              title={`更新到 ${updateInfo.latest}`}
            >
              <Download className="h-4 w-4 mr-1" />
              更新
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
            disabled={switching || isActive || !theme.compatible}
            variant={isActive ? "secondary" : "default"}
            size="sm"
            className="flex-1"
          >
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
