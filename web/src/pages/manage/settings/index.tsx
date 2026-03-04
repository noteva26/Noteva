import { useState, useEffect, useRef } from "react";
import { motion } from "motion/react";
import { adminApi, UpdateCheckResponse, authApi } from "@/lib/api";
import { useAuthStore } from "@/lib/store/auth";
import { useSiteStore } from "@/lib/store/site";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Skeleton } from "@/components/ui/skeleton";
import { AvatarUpload } from "@/components/ui/avatar-upload";
import {
  AlertDialog,
  AlertDialogContent,
  AlertDialogHeader,
  AlertDialogFooter,
  AlertDialogTitle,
  AlertDialogDescription,
  AlertDialogAction,
  AlertDialogCancel,
} from "@/components/ui/alert-dialog";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Settings, User, MessageSquare, Loader2, RefreshCw, Download, AlertCircle, CheckCircle2, Link, RotateCw, Code, Database, Upload, FileText } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import ReactMarkdown from "react-markdown";

// Permalink format options
const PERMALINK_OPTIONS = [
  { value: "/posts/{slug}", label: "/posts/{slug}", example: "/posts/hello-world" },
  { value: "/posts/{id}", label: "/posts/{id}", example: "/posts/42" },
];

export default function SettingsPage() {
  const { user } = useAuthStore();
  const { updateSettings } = useSiteStore();
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [savingSite, setSavingSite] = useState(false);
  const [savingComment, setSavingComment] = useState(false);
  const [savingProfile, setSavingProfile] = useState(false);
  const [savingCustomCode, setSavingCustomCode] = useState(false);

  const [siteForm, setSiteForm] = useState({
    siteName: "",
    siteDescription: "",
    siteSubtitle: "",
    siteLogo: "",
    siteFooter: "",
    siteUrl: "",
    permalinkStructure: "/posts/{slug}",
  });



  const [commentForm, setCommentForm] = useState({
    commentModeration: false,
    moderationKeywords: "",
  });

  const [profileForm, setProfileForm] = useState({
    displayName: "",
    avatar: "",
  });

  const [passwordForm, setPasswordForm] = useState({
    currentPassword: "",
    newPassword: "",
    confirmPassword: "",
  });

  const [customCodeForm, setCustomCodeForm] = useState({
    customCss: "",
    customJs: "",
  });

  // Update check state
  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResponse | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  const [performingUpdate, setPerformingUpdate] = useState(false);
  const [updateRestarting, setUpdateRestarting] = useState(false);
  // 榛樿寮€鍚?Beta 妫€鏌ワ紝鍥犱负褰撳墠鐗堟湰鏄?beta
  const [checkBeta, setCheckBeta] = useState(true);
  const [updateDialogOpen, setUpdateDialogOpen] = useState(false);

  // Backup state
  const [backingUp, setBackingUp] = useState(false);
  const [restoring, setRestoring] = useState(false);
  const [exportingMd, setExportingMd] = useState(false);
  const [restoreDialogOpen, setRestoreDialogOpen] = useState(false);
  const restoreFileRef = useRef<File | null>(null);

  useEffect(() => {
    adminApi.getSettings()
      .then(({ data }) => {
        setSiteForm({
          siteName: data.site_name || "",
          siteDescription: data.site_description || "",
          siteSubtitle: data.site_subtitle || "",
          siteLogo: data.site_logo || "",
          siteFooter: data.site_footer || "",
          siteUrl: data.site_url || "",
          permalinkStructure: data.permalink_structure || "/posts/{slug}",
        });

        setCommentForm({
          commentModeration: data.comment_moderation === "true",
          moderationKeywords: data.moderation_keywords || "",
        });
        setCustomCodeForm({
          customCss: String(data.custom_css || ""),
          customJs: String(data.custom_js || ""),
        });
        // Load profile from user
        if (user) {
          setProfileForm({
            displayName: user.display_name || "",
            avatar: user.avatar || "",
          });
        }
      })
      .catch((err) => {
        console.error("Failed to load settings:", err);
        setSiteForm({
          siteName: "Noteva",
          siteDescription: "",
          siteSubtitle: "",
          siteLogo: "",
          siteFooter: "",
          siteUrl: "",
          permalinkStructure: "/posts/{slug}",
        });
        toast.error("Failed to load settings");
      })
      .finally(() => setLoading(false));
  }, [user]);

  const handleSaveSiteSettings = async () => {
    if (!siteForm.siteName.trim()) {
      toast.error(t("settings.siteNameRequired"));
      return;
    }
    if (siteForm.siteUrl.trim() && !/^https?:\/\/.+/.test(siteForm.siteUrl.trim())) {
      toast.error(t("settings.siteUrlInvalid"));
      return;
    }
    setSavingSite(true);
    try {
      const newSettings = {
        site_name: siteForm.siteName,
        site_description: siteForm.siteDescription,
        site_subtitle: siteForm.siteSubtitle,
        site_logo: siteForm.siteLogo,
        site_footer: siteForm.siteFooter,
        site_url: siteForm.siteUrl,
        permalink_structure: siteForm.permalinkStructure,
      };
      await adminApi.updateSettings(newSettings);
      // 鏇存柊鍏ㄥ眬 store
      updateSettings(newSettings);
      toast.success(t("settings.saveSuccess"));
    } catch (error) {
      toast.error(t("settings.saveFailed"));
    } finally {
      setSavingSite(false);
    }
  };

  const handleChangePassword = async () => {
    if (!passwordForm.currentPassword || !passwordForm.newPassword) {
      toast.error(t("settings.passwordRequired"));
      return;
    }
    if (passwordForm.newPassword !== passwordForm.confirmPassword) {
      toast.error(t("auth.passwordMismatch"));
      return;
    }
    if (passwordForm.newPassword.length < 8) {
      toast.error(t("auth.passwordTooShort"));
      return;
    }
    // TODO: Implement password change API
    try {
      await authApi.changePassword(passwordForm.currentPassword, passwordForm.newPassword);
      toast.success(t("settings.passwordUpdated"));
      setPasswordForm({ currentPassword: "", newPassword: "", confirmPassword: "" });
    } catch (error: any) {
      const msg = error?.response?.data?.error?.message || t("settings.saveFailed");
      toast.error(msg);
    }
  };

  const handleSaveCommentSettings = async () => {
    setSavingComment(true);
    try {
      await adminApi.updateSettings({
        comment_moderation: commentForm.commentModeration ? "true" : "false",
        moderation_keywords: commentForm.moderationKeywords,
      });
      toast.success(t("settings.saveSuccess"));
    } catch (error) {
      toast.error(t("settings.saveFailed"));
    } finally {
      setSavingComment(false);
    }
  };

  const handleSaveCustomCode = async () => {
    setSavingCustomCode(true);
    try {
      await adminApi.updateSettings({
        custom_css: customCodeForm.customCss,
        custom_js: customCodeForm.customJs,
      });
      toast.success(t("settings.saveSuccess"));
    } catch (error) {
      toast.error(t("settings.saveFailed"));
    } finally {
      setSavingCustomCode(false);
    }
  };

  const handleSaveProfile = async () => {
    setSavingProfile(true);
    try {
      await authApi.updateProfile({
        display_name: profileForm.displayName || null,
        avatar: profileForm.avatar || null,
      });
      toast.success(t("settings.saveSuccess"));
    } catch (error) {
      toast.error(t("settings.saveFailed"));
    } finally {
      setSavingProfile(false);
    }
  };

  const handleCheckUpdate = async () => {
    setCheckingUpdate(true);
    try {
      const { data } = await adminApi.checkUpdate(checkBeta);
      setUpdateInfo(data);
      if (data.error) {
        toast.error(data.error);
      } else if (data.update_available) {
        toast.success(t("settings.updateAvailable"));
      } else {
        toast.info(t("settings.noUpdate"));
      }
    } catch (error) {
      toast.error(t("settings.checkUpdateFailed"));
    } finally {
      setCheckingUpdate(false);
    }
  };

  const handlePerformUpdate = () => {
    if (!updateInfo?.latest_version) return;
    setUpdateDialogOpen(true);
  };

  const doPerformUpdate = async () => {
    if (!updateInfo?.latest_version) return;
    setUpdateDialogOpen(false);
    setPerformingUpdate(true);
    try {
      await adminApi.performUpdate(updateInfo.latest_version, updateInfo.is_beta);
      toast.success(t("settings.updateSuccess"));
      setUpdateRestarting(true);
    } catch (error: any) {
      const msg = error?.response?.data?.error?.message || t("settings.updateFailed");
      toast.error(msg);
    } finally {
      setPerformingUpdate(false);
    }
  };

  // Auto-refresh: poll server after update restart
  useEffect(() => {
    if (!updateRestarting) return;
    let cancelled = false;
    const poll = async () => {
      // Wait a few seconds for server to actually shut down
      await new Promise(r => setTimeout(r, 5000));
      while (!cancelled) {
        try {
          await fetch("/api/v1/site/info", { cache: "no-store" });
          // Server is back! Reload after a brief pause
          if (!cancelled) {
            toast.success(t("settings.updateRestartDone") || "Update complete! Reloading...");
            await new Promise(r => setTimeout(r, 1500));
            window.location.reload();
          }
          return;
        } catch {
          // Server still down, wait and retry
          await new Promise(r => setTimeout(r, 3000));
        }
      }
    };
    poll();
    return () => { cancelled = true; };
  }, [updateRestarting]);


  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
      >
        <h1 className="text-3xl font-bold">{t("settings.title")}</h1>
        <p className="text-muted-foreground">{t("settings.site")}</p>
      </motion.div>

      <motion.div
        initial={{ opacity: 0 }}
        animate={{ opacity: 1 }}
        transition={{ duration: 0.4, delay: 0.1 }}
      >
        <Tabs defaultValue="general" className="space-y-6">
          <TabsList>
            <TabsTrigger value="general" className="gap-2">
              <Settings className="h-4 w-4" />
              {t("settings.general")}
            </TabsTrigger>
            <TabsTrigger value="comments" className="gap-2">
              <MessageSquare className="h-4 w-4" />
              {t("settings.comments")}
            </TabsTrigger>
            <TabsTrigger value="account" className="gap-2">
              <User className="h-4 w-4" />
              {t("settings.account")}
            </TabsTrigger>
            <TabsTrigger value="update" className="gap-2">
              <Download className="h-4 w-4" />
              {t("settings.update")}
            </TabsTrigger>
            <TabsTrigger value="customCode" className="gap-2">
              <Code className="h-4 w-4" />
              {t("settings.customCode")}
            </TabsTrigger>
            <TabsTrigger value="data" className="gap-2">
              <Database className="h-4 w-4" />
              {t("settings.dataManagement")}
            </TabsTrigger>
          </TabsList>

          <TabsContent value="general" className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>{t("settings.site")}</CardTitle>
                <CardDescription>{t("settings.siteDescription")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {loading ? (
                  <div className="space-y-4">
                    {Array.from({ length: 5 }).map((_, i) => (
                      <Skeleton key={i} className="h-10 w-full" />
                    ))}
                  </div>
                ) : (
                  <>
                    <div className="space-y-2">
                      <Label htmlFor="siteName">{t("settings.siteName")}</Label>
                      <Input
                        id="siteName"
                        placeholder="Noteva"
                        value={siteForm.siteName}
                        onChange={(e) => setSiteForm((f) => ({ ...f, siteName: e.target.value }))}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="siteSubtitle">{t("settings.siteSubtitle")}</Label>
                      <Input
                        id="siteSubtitle"
                        placeholder=""
                        value={siteForm.siteSubtitle}
                        onChange={(e) => setSiteForm((f) => ({ ...f, siteSubtitle: e.target.value }))}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="siteDescription">{t("settings.siteDescription")}</Label>
                      <Input
                        id="siteDescription"
                        placeholder=""
                        value={siteForm.siteDescription}
                        onChange={(e) => setSiteForm((f) => ({ ...f, siteDescription: e.target.value }))}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="siteLogo">{t("settings.siteLogo")}</Label>
                      <Input
                        id="siteLogo"
                        placeholder="https://..."
                        value={siteForm.siteLogo}
                        onChange={(e) => setSiteForm((f) => ({ ...f, siteLogo: e.target.value }))}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="siteFooter">{t("settings.siteFooter")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.siteFooterDesc")}
                      </p>
                      <textarea
                        id="siteFooter"
                        className="w-full min-h-[80px] p-3 rounded-md border bg-background text-sm resize-y focus:outline-none focus:ring-2 focus:ring-ring"
                        placeholder={t("settings.siteFooterPlaceholder")}
                        value={siteForm.siteFooter}
                        onChange={(e) => setSiteForm((f) => ({ ...f, siteFooter: e.target.value }))}
                      />
                    </div>
                    <div className="space-y-2 pt-4 border-t">
                      <Label htmlFor="siteUrl">{t("settings.siteUrl")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.siteUrlDesc")}
                      </p>
                      <Input
                        id="siteUrl"
                        placeholder={t("settings.siteUrlPlaceholder")}
                        value={siteForm.siteUrl}
                        onChange={(e) => setSiteForm((f) => ({ ...f, siteUrl: e.target.value }))}
                      />
                    </div>

                    <div className="space-y-2 pt-4 border-t">
                      <Label className="flex items-center gap-2">
                        <Link className="h-4 w-4" />
                        {t("settings.permalink")}
                      </Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.permalinkDesc")}
                      </p>
                      <Select
                        value={siteForm.permalinkStructure}
                        onValueChange={(v) => setSiteForm((f) => ({ ...f, permalinkStructure: v }))}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {PERMALINK_OPTIONS.map((opt) => (
                            <SelectItem key={opt.value} value={opt.value}>
                              <div className="flex flex-col">
                                <span>{opt.label}</span>
                                <span className="text-xs text-muted-foreground">{opt.example}</span>
                              </div>
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                    </div>
                    <Button onClick={handleSaveSiteSettings} disabled={savingSite}>
                      {savingSite && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                      {t("settings.saveSettings")}
                    </Button>
                  </>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="comments" className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>{t("settings.commentSettings")}</CardTitle>
                <CardDescription>{t("settings.commentSettingsDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                {loading ? (
                  <div className="space-y-4">
                    <Skeleton className="h-10 w-full" />
                    <Skeleton className="h-10 w-full" />
                  </div>
                ) : (
                  <>
                    <div className="flex items-center justify-between">
                      <div className="space-y-0.5">
                        <Label>{t("settings.commentModeration")}</Label>
                        <p className="text-sm text-muted-foreground">
                          {t("settings.commentModerationDesc")}
                        </p>
                      </div>
                      <label className="relative inline-flex items-center cursor-pointer">
                        <input
                          type="checkbox"
                          checked={commentForm.commentModeration}
                          onChange={(e) => setCommentForm((f) => ({ ...f, commentModeration: e.target.checked }))}
                          className="sr-only peer"
                        />
                        <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                      </label>
                    </div>
                    <div className="space-y-2">
                      <Label>{t("settings.moderationKeywords")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.moderationKeywordsDesc")}
                      </p>
                      <Input
                        value={commentForm.moderationKeywords}
                        onChange={(e) => setCommentForm((f) => ({ ...f, moderationKeywords: e.target.value }))}
                        placeholder={t("settings.moderationKeywordsPlaceholder")}
                      />
                    </div>
                    <Button onClick={handleSaveCommentSettings} disabled={savingComment}>
                      {savingComment && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                      {t("settings.saveSettings")}
                    </Button>
                  </>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="account" className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>{t("settings.accountInfo")}</CardTitle>
                <CardDescription>{t("settings.viewAccountInfo")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid gap-4 md:grid-cols-2">
                  <div className="space-y-2">
                    <Label>{t("auth.username")}</Label>
                    <Input value={user?.username || ""} disabled />
                  </div>
                  <div className="space-y-2">
                    <Label>{t("auth.email")}</Label>
                    <Input value={user?.email || ""} disabled />
                  </div>
                </div>
                <div className="space-y-2">
                  <Label>{t("settings.role")}</Label>
                  <Input value={user?.role || ""} disabled className="w-32" />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>{t("settings.profileSettings")}</CardTitle>
                <CardDescription>{t("settings.profileSettingsDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="displayName">{t("user.displayName")}</Label>
                  <Input
                    id="displayName"
                    placeholder={t("user.displayNamePlaceholder")}
                    value={profileForm.displayName}
                    onChange={(e) => setProfileForm((f) => ({ ...f, displayName: e.target.value }))}
                  />
                </div>
                <div className="space-y-2">
                  <Label>{t("user.avatar")}</Label>
                  <AvatarUpload
                    value={profileForm.avatar}
                    onChange={(url) => setProfileForm((f) => ({ ...f, avatar: url }))}
                  />
                </div>
                <Button onClick={handleSaveProfile} disabled={savingProfile}>
                  {savingProfile && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                  {t("settings.saveSettings")}
                </Button>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>{t("settings.changePassword")}</CardTitle>
                <CardDescription>{t("settings.updatePassword")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="space-y-2">
                  <Label htmlFor="currentPassword">{t("settings.currentPassword")}</Label>
                  <Input
                    id="currentPassword"
                    type="password"
                    value={passwordForm.currentPassword}
                    onChange={(e) =>
                      setPasswordForm((f) => ({ ...f, currentPassword: e.target.value }))
                    }
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="newPassword">{t("settings.newPassword")}</Label>
                  <Input
                    id="newPassword"
                    type="password"
                    value={passwordForm.newPassword}
                    onChange={(e) =>
                      setPasswordForm((f) => ({ ...f, newPassword: e.target.value }))
                    }
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="confirmPassword">{t("settings.confirmNewPassword")}</Label>
                  <Input
                    id="confirmPassword"
                    type="password"
                    value={passwordForm.confirmPassword}
                    onChange={(e) =>
                      setPasswordForm((f) => ({ ...f, confirmPassword: e.target.value }))
                    }
                  />
                </div>
                <Button onClick={handleChangePassword}>{t("settings.updatePassword")}</Button>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="update" className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>{t("settings.systemUpdate")}</CardTitle>
                <CardDescription>{t("settings.systemUpdateDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-6">
                {/* Beta toggle */}
                <div className="flex items-center justify-between">
                  <div className="space-y-0.5">
                    <Label>{t("settings.checkBeta")}</Label>
                    <p className="text-sm text-muted-foreground">
                      {t("settings.checkBetaDesc")}
                    </p>
                  </div>
                  <label className="relative inline-flex items-center cursor-pointer">
                    <input
                      type="checkbox"
                      checked={checkBeta}
                      onChange={(e) => setCheckBeta(e.target.checked)}
                      className="sr-only peer"
                    />
                    <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                  </label>
                </div>

                {/* Check update button */}
                <Button onClick={handleCheckUpdate} disabled={checkingUpdate}>
                  {checkingUpdate ? (
                    <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                  ) : (
                    <RefreshCw className="h-4 w-4 mr-2" />
                  )}
                  {t("settings.checkUpdate")}
                </Button>

                {/* Update info */}
                {updateInfo && (
                  <div className="space-y-4 pt-4 border-t">
                    <div className="flex items-center gap-2">
                      <span className="text-sm text-muted-foreground">{t("settings.currentVersion")}:</span>
                      <span className="font-mono">v{updateInfo.current_version}</span>
                    </div>

                    {updateInfo.latest_version && (
                      <div className="flex items-center gap-2">
                        <span className="text-sm text-muted-foreground">{t("settings.latestVersion")}:</span>
                        <span className="font-mono">v{updateInfo.latest_version}</span>
                        {updateInfo.is_beta && (
                          <span className="text-xs px-2 py-0.5 bg-yellow-100 text-yellow-800 dark:bg-yellow-900 dark:text-yellow-200 rounded">
                            Beta
                          </span>
                        )}
                      </div>
                    )}

                    {updateInfo.update_available ? (
                      <div className="p-4 bg-green-50 dark:bg-green-900/20 rounded-lg border border-green-200 dark:border-green-800">
                        <div className="flex items-start gap-3">
                          <Download className="h-5 w-5 text-green-600 dark:text-green-400 mt-0.5" />
                          <div className="space-y-2 flex-1">
                            <p className="font-medium text-green-800 dark:text-green-200">
                              {t("settings.newVersionAvailable")}
                            </p>
                            {updateInfo.release_date && (
                              <p className="text-sm text-green-700 dark:text-green-300">
                                {t("settings.releaseDate")}: {new Date(updateInfo.release_date).toLocaleDateString()}
                              </p>
                            )}
                            {updateInfo.release_notes && (
                              <div className="mt-2 p-3 bg-white dark:bg-gray-800 rounded text-sm max-h-40 overflow-y-auto prose prose-sm dark:prose-invert prose-headings:text-base prose-headings:font-semibold prose-p:my-1 prose-ul:my-1 prose-li:my-0">
                                <ReactMarkdown>{updateInfo.release_notes}</ReactMarkdown>
                              </div>
                            )}
                            <div className="flex items-center gap-3 mt-3">
                              <Button
                                onClick={handlePerformUpdate}
                                disabled={performingUpdate || updateRestarting}
                                size="sm"
                              >
                                {performingUpdate ? (
                                  <Loader2 className="h-4 w-4 mr-2 animate-spin" />
                                ) : (
                                  <RotateCw className="h-4 w-4 mr-2" />
                                )}
                                {performingUpdate ? t("settings.updating") : t("settings.performUpdate")}
                              </Button>
                              {updateInfo.release_url && (
                                <a
                                  href={updateInfo.release_url}
                                  target="_blank"
                                  rel="noopener noreferrer"
                                  className="inline-flex items-center gap-2 text-sm text-green-700 dark:text-green-300 hover:underline"
                                >
                                  <Download className="h-4 w-4" />
                                  {t("settings.downloadUpdate")}
                                </a>
                              )}
                            </div>
                          </div>
                        </div>
                      </div>
                    ) : updateInfo.error ? (
                      <div className="p-4 bg-red-50 dark:bg-red-900/20 rounded-lg border border-red-200 dark:border-red-800">
                        <div className="flex items-center gap-3">
                          <AlertCircle className="h-5 w-5 text-red-600 dark:text-red-400" />
                          <p className="text-red-800 dark:text-red-200">{updateInfo.error}</p>
                        </div>
                      </div>
                    ) : (
                      <div className="p-4 bg-blue-50 dark:bg-blue-900/20 rounded-lg border border-blue-200 dark:border-blue-800">
                        <div className="flex items-center gap-3">
                          <CheckCircle2 className="h-5 w-5 text-blue-600 dark:text-blue-400" />
                          <p className="text-blue-800 dark:text-blue-200">{t("settings.upToDate")}</p>
                        </div>
                      </div>
                    )}
                  </div>
                )}

                {/* Restarting notice */}
                {updateRestarting && (
                  <div className="p-4 bg-amber-50 dark:bg-amber-900/20 rounded-lg border border-amber-200 dark:border-amber-800">
                    <div className="flex items-center gap-3">
                      <Loader2 className="h-5 w-5 text-amber-600 dark:text-amber-400 animate-spin" />
                      <p className="text-amber-800 dark:text-amber-200">{t("settings.updateRestarting")}</p>
                    </div>
                  </div>
                )}

                {/* Update instructions */}
                <div className="pt-4 border-t">
                  <h4 className="font-medium mb-2">{t("settings.howToUpdate")}</h4>
                  <ol className="list-decimal list-inside space-y-1 text-sm text-muted-foreground">
                    <li>{t("settings.updateStep1")}</li>
                    <li>{t("settings.updateStep2")}</li>
                    <li>{t("settings.updateStep3")}</li>
                  </ol>
                </div>
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="customCode" className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>{t("settings.customCodeSettings")}</CardTitle>
                <CardDescription>{t("settings.customCodeDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-6">
                {loading ? (
                  <div className="space-y-4">
                    <Skeleton className="h-40 w-full" />
                    <Skeleton className="h-40 w-full" />
                  </div>
                ) : (
                  <>
                    <div className="space-y-2">
                      <Label htmlFor="customCss">{t("settings.customCss")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.customCssDesc")}
                      </p>
                      <textarea
                        id="customCss"
                        className="w-full min-h-[200px] p-3 rounded-md border bg-muted/50 font-mono text-sm resize-y focus:outline-none focus:ring-2 focus:ring-ring"
                        placeholder={t("settings.customCssPlaceholder")}
                        value={customCodeForm.customCss}
                        onChange={(e) => setCustomCodeForm((f) => ({ ...f, customCss: e.target.value }))}
                        spellCheck={false}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="customJs">{t("settings.customJs")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.customJsDesc")}
                      </p>
                      <textarea
                        id="customJs"
                        className="w-full min-h-[200px] p-3 rounded-md border bg-muted/50 font-mono text-sm resize-y focus:outline-none focus:ring-2 focus:ring-ring"
                        placeholder={t("settings.customJsPlaceholder")}
                        value={customCodeForm.customJs}
                        onChange={(e) => setCustomCodeForm((f) => ({ ...f, customJs: e.target.value }))}
                        spellCheck={false}
                      />
                    </div>
                    <Button onClick={handleSaveCustomCode} disabled={savingCustomCode}>
                      {savingCustomCode && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                      {t("settings.saveSettings")}
                    </Button>
                  </>
                )}
              </CardContent>
            </Card>
          </TabsContent>

          <TabsContent value="data" className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>{t("settings.backupRestore")}</CardTitle>
                <CardDescription>{t("settings.backupDescription")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex flex-col gap-3">
                  <div className="flex items-center justify-between p-4 rounded-lg border">
                    <div>
                      <p className="font-medium">{t("settings.downloadBackup")}</p>
                      <p className="text-sm text-muted-foreground">{t("settings.downloadBackupDesc")}</p>
                    </div>
                    <Button
                      onClick={async () => {
                        setBackingUp(true);
                        try {
                          const res = await adminApi.downloadBackup();
                          const blob = new Blob([res.data as any], { type: "application/zip" });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement("a");
                          a.href = url;
                          a.download = `noteva-backup-${new Date().toISOString().slice(0, 10)}.zip`;
                          a.click();
                          URL.revokeObjectURL(url);
                          toast.success(t("settings.backupSuccess"));
                        } catch { toast.error(t("settings.backupFailed")); }
                        finally { setBackingUp(false); }
                      }}
                      disabled={backingUp}
                    >
                      {backingUp ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Download className="h-4 w-4 mr-2" />}
                      {t("settings.downloadBackup")}
                    </Button>
                  </div>

                  <div className="flex items-center justify-between p-4 rounded-lg border">
                    <div>
                      <p className="font-medium">{t("settings.restoreBackup")}</p>
                      <p className="text-sm text-muted-foreground">{t("settings.restoreBackupDesc")}</p>
                    </div>
                    <Button
                      variant="destructive"
                      onClick={() => {
                        const input = document.createElement("input");
                        input.type = "file";
                        input.accept = ".zip";
                        input.onchange = (e) => {
                          const file = (e.target as HTMLInputElement).files?.[0];
                          if (!file) return;
                          restoreFileRef.current = file;
                          setRestoreDialogOpen(true);
                        };
                        input.click();
                      }}
                      disabled={restoring}
                    >
                      {restoring ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <Upload className="h-4 w-4 mr-2" />}
                      {t("settings.restoreBackup")}
                    </Button>
                  </div>

                  <div className="flex items-center justify-between p-4 rounded-lg border">
                    <div>
                      <p className="font-medium">{t("settings.exportMarkdown")}</p>
                      <p className="text-sm text-muted-foreground">{t("settings.exportMarkdownDesc")}</p>
                    </div>
                    <Button
                      variant="outline"
                      onClick={async () => {
                        setExportingMd(true);
                        try {
                          const res = await adminApi.exportMarkdown();
                          const blob = new Blob([res.data as any], { type: "application/zip" });
                          const url = URL.createObjectURL(blob);
                          const a = document.createElement("a");
                          a.href = url;
                          a.download = `noteva-articles-${new Date().toISOString().slice(0, 10)}.zip`;
                          a.click();
                          URL.revokeObjectURL(url);
                          toast.success(t("settings.exportSuccess"));
                        } catch { toast.error(t("settings.exportFailed")); }
                        finally { setExportingMd(false); }
                      }}
                      disabled={exportingMd}
                    >
                      {exportingMd ? <Loader2 className="h-4 w-4 mr-2 animate-spin" /> : <FileText className="h-4 w-4 mr-2" />}
                      {t("settings.exportMarkdown")}
                    </Button>
                  </div>
                </div>
              </CardContent>
            </Card>
          </TabsContent>
        </Tabs>
      </motion.div>

      {/* Update Confirmation Dialog */}
      <AlertDialog open={updateDialogOpen} onOpenChange={setUpdateDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2">
              <RotateCw className="h-5 w-5 text-primary" />
              {t("settings.systemUpdate")}
            </AlertDialogTitle>
            <AlertDialogDescription asChild>
              <div className="space-y-2">
                <p>{t("settings.confirmUpdate").replace("{version}", updateInfo?.latest_version || "")}</p>
                {updateInfo?.is_beta && (
                  <p className="text-amber-600 dark:text-amber-400 text-xs">
                    Beta
                  </p>
                )}
              </div>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={doPerformUpdate}>
              <RotateCw className="h-4 w-4 mr-2" />
              {t("settings.performUpdate")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>

      {/* Restore Backup Confirmation Dialog */}
      <AlertDialog open={restoreDialogOpen} onOpenChange={setRestoreDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2">
              <AlertCircle className="h-5 w-5 text-destructive" />
              {t("settings.restoreBackup")}
            </AlertDialogTitle>
            <AlertDialogDescription asChild>
              <div className="space-y-3">
                <p>{t("settings.restoreConfirm")}</p>
                {restoreFileRef.current && (
                  <div className="flex items-center gap-2 p-2 rounded bg-muted text-sm">
                    <Upload className="h-4 w-4 text-muted-foreground" />
                    <span className="font-mono">{restoreFileRef.current.name}</span>
                    <span className="text-muted-foreground">
                      ({(restoreFileRef.current.size / 1024 / 1024).toFixed(1)} MB)
                    </span>
                  </div>
                )}
              </div>
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction
              className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
              onClick={async () => {
                const file = restoreFileRef.current;
                if (!file) return;
                setRestoring(true);
                setRestoreDialogOpen(false);
                try {
                  await adminApi.restoreBackup(file);
                  toast.success(t("settings.restoreSuccess"));
                  setTimeout(() => window.location.reload(), 2000);
                } catch { toast.error(t("settings.restoreFailed")); }
                finally { setRestoring(false); }
              }}
            >
              <Upload className="h-4 w-4 mr-2" />
              {t("settings.restoreBackup")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}

