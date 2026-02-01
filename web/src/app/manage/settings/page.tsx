"use client";

import { useState, useEffect } from "react";
import { motion } from "motion/react";
import { adminApi, emailApi, UpdateCheckResponse } from "@/lib/api";
import { useAuthStore } from "@/lib/store/auth";
import { useSiteStore } from "@/lib/store/site";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Skeleton } from "@/components/ui/skeleton";
import { Settings, User, MessageSquare, Loader2, RefreshCw, Download, AlertCircle, CheckCircle2, Mail } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";

export default function SettingsPage() {
  const { user } = useAuthStore();
  const { updateSettings } = useSiteStore();
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [savingSite, setSavingSite] = useState(false);
  const [savingComment, setSavingComment] = useState(false);
  const [savingEmail, setSavingEmail] = useState(false);
  const [testingEmail, setTestingEmail] = useState(false);

  const [siteForm, setSiteForm] = useState({
    siteName: "",
    siteDescription: "",
    siteSubtitle: "",
    siteLogo: "",
    siteFooter: "",
  });

  const [commentForm, setCommentForm] = useState({
    requireLoginToComment: false,
    commentModeration: false,
    moderationKeywords: "",
  });

  const [emailForm, setEmailForm] = useState({
    smtpHost: "",
    smtpPort: "587",
    smtpUsername: "",
    smtpPassword: "",
    smtpFrom: "",
    emailVerificationEnabled: false,
  });
  const [testEmailAddress, setTestEmailAddress] = useState("");

  const [passwordForm, setPasswordForm] = useState({
    currentPassword: "",
    newPassword: "",
    confirmPassword: "",
  });

  // Update check state
  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResponse | null>(null);
  const [checkingUpdate, setCheckingUpdate] = useState(false);
  // 默认开启 Beta 检查，因为当前版本是 beta
  const [checkBeta, setCheckBeta] = useState(true);

  useEffect(() => {
    adminApi.getSettings()
      .then(({ data }) => {
        setSiteForm({
          siteName: data.site_name || "",
          siteDescription: data.site_description || "",
          siteSubtitle: data.site_subtitle || "",
          siteLogo: data.site_logo || "",
          siteFooter: data.site_footer || "",
        });
        setCommentForm({
          requireLoginToComment: data.require_login_to_comment === "true",
          commentModeration: data.comment_moderation === "true",
          moderationKeywords: data.moderation_keywords || "",
        });
        setEmailForm({
          smtpHost: data.smtp_host || "",
          smtpPort: data.smtp_port || "587",
          smtpUsername: data.smtp_username || "",
          smtpPassword: data.smtp_password || "",
          smtpFrom: data.smtp_from || "",
          emailVerificationEnabled: data.email_verification_enabled === "true",
        });
      })
      .catch((err) => {
        console.error("Failed to load settings:", err);
        setSiteForm({
          siteName: "Noteva Blog",
          siteDescription: "",
          siteSubtitle: "",
          siteLogo: "",
          siteFooter: "",
        });
        toast.error("Failed to load settings");
      })
      .finally(() => setLoading(false));
  }, []);

  const handleSaveSiteSettings = async () => {
    setSavingSite(true);
    try {
      const newSettings = {
        site_name: siteForm.siteName,
        site_description: siteForm.siteDescription,
        site_subtitle: siteForm.siteSubtitle,
        site_logo: siteForm.siteLogo,
        site_footer: siteForm.siteFooter,
      };
      await adminApi.updateSettings(newSettings);
      // 更新全局 store
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
      toast.error(t("auth.password"));
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
    toast.info(t("settings.passwordUpdated"));
  };

  const handleSaveCommentSettings = async () => {
    setSavingComment(true);
    try {
      await adminApi.updateSettings({
        require_login_to_comment: commentForm.requireLoginToComment ? "true" : "false",
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

  const handleSaveEmailSettings = async () => {
    setSavingEmail(true);
    try {
      await adminApi.updateSettings({
        smtp_host: emailForm.smtpHost,
        smtp_port: emailForm.smtpPort,
        smtp_username: emailForm.smtpUsername,
        smtp_password: emailForm.smtpPassword,
        smtp_from: emailForm.smtpFrom,
        email_verification_enabled: emailForm.emailVerificationEnabled ? "true" : "false",
      });
      toast.success(t("settings.saveSuccess"));
    } catch (error) {
      toast.error(t("settings.saveFailed"));
    } finally {
      setSavingEmail(false);
    }
  };

  const handleTestEmail = async () => {
    if (!testEmailAddress) {
      toast.error(t("settings.enterTestEmail"));
      return;
    }
    setTestingEmail(true);
    try {
      const { data } = await emailApi.testEmail(testEmailAddress);
      toast.success(t("settings.testEmailSuccess"));
    } catch (error: any) {
      const message = error.response?.data?.error?.message 
        || error.response?.data?.error 
        || error.response?.data?.message
        || error.message 
        || t("settings.testEmailFailed");
      toast.error(message);
    } finally {
      setTestingEmail(false);
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
          <TabsTrigger value="email" className="gap-2">
            <Mail className="h-4 w-4" />
            {t("settings.email")}
          </TabsTrigger>
          <TabsTrigger value="account" className="gap-2">
            <User className="h-4 w-4" />
            {t("settings.account")}
          </TabsTrigger>
          <TabsTrigger value="update" className="gap-2">
            <Download className="h-4 w-4" />
            {t("settings.update")}
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
                      placeholder="Noteva Blog"
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
                    <Input
                      id="siteFooter"
                      placeholder=""
                      value={siteForm.siteFooter}
                      onChange={(e) => setSiteForm((f) => ({ ...f, siteFooter: e.target.value }))}
                    />
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
                      <Label>{t("settings.requireLoginToComment")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.requireLoginToCommentDesc")}
                      </p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        checked={commentForm.requireLoginToComment}
                        onChange={(e) => setCommentForm((f) => ({ ...f, requireLoginToComment: e.target.checked }))}
                        className="sr-only peer"
                      />
                      <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                    </label>
                  </div>
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

        <TabsContent value="email" className="space-y-4">
          <Card>
            <CardHeader>
              <CardTitle>{t("settings.emailSettings")}</CardTitle>
              <CardDescription>{t("settings.emailSettingsDesc")}</CardDescription>
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
                  <div className="flex items-center justify-between">
                    <div className="space-y-0.5">
                      <Label>{t("settings.emailVerification")}</Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.emailVerificationDesc")}
                      </p>
                    </div>
                    <label className="relative inline-flex items-center cursor-pointer">
                      <input
                        type="checkbox"
                        checked={emailForm.emailVerificationEnabled}
                        onChange={(e) => setEmailForm((f) => ({ ...f, emailVerificationEnabled: e.target.checked }))}
                        className="sr-only peer"
                      />
                      <div className="w-11 h-6 bg-gray-200 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 dark:peer-focus:ring-blue-800 rounded-full peer dark:bg-gray-700 peer-checked:after:translate-x-full peer-checked:after:border-white after:content-[''] after:absolute after:top-[2px] after:left-[2px] after:bg-white after:border-gray-300 after:border after:rounded-full after:h-5 after:w-5 after:transition-all dark:border-gray-600 peer-checked:bg-blue-600"></div>
                    </label>
                  </div>
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="smtpHost">{t("settings.smtpHost")}</Label>
                      <Input
                        id="smtpHost"
                        placeholder="smtp.example.com"
                        value={emailForm.smtpHost}
                        onChange={(e) => setEmailForm((f) => ({ ...f, smtpHost: e.target.value }))}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="smtpPort">{t("settings.smtpPort")}</Label>
                      <Input
                        id="smtpPort"
                        placeholder="587"
                        value={emailForm.smtpPort}
                        onChange={(e) => setEmailForm((f) => ({ ...f, smtpPort: e.target.value }))}
                      />
                    </div>
                  </div>
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2">
                      <Label htmlFor="smtpUsername">{t("settings.smtpUsername")}</Label>
                      <Input
                        id="smtpUsername"
                        placeholder="your@email.com"
                        value={emailForm.smtpUsername}
                        onChange={(e) => setEmailForm((f) => ({ ...f, smtpUsername: e.target.value }))}
                      />
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="smtpPassword">{t("settings.smtpPassword")}</Label>
                      <Input
                        id="smtpPassword"
                        type="password"
                        placeholder="••••••••"
                        value={emailForm.smtpPassword}
                        onChange={(e) => setEmailForm((f) => ({ ...f, smtpPassword: e.target.value }))}
                      />
                    </div>
                  </div>
                  <div className="space-y-2">
                    <Label htmlFor="smtpFrom">{t("settings.smtpFrom")}</Label>
                    <Input
                      id="smtpFrom"
                      placeholder="noreply@example.com"
                      value={emailForm.smtpFrom}
                      onChange={(e) => setEmailForm((f) => ({ ...f, smtpFrom: e.target.value }))}
                    />
                    <p className="text-sm text-muted-foreground">{t("settings.smtpFromDesc")}</p>
                  </div>
                  <Button onClick={handleSaveEmailSettings} disabled={savingEmail}>
                    {savingEmail && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                    {t("settings.saveSettings")}
                  </Button>
                </>
              )}
            </CardContent>
          </Card>

          <Card>
            <CardHeader>
              <CardTitle>{t("settings.testEmail")}</CardTitle>
              <CardDescription>{t("settings.testEmailDesc")}</CardDescription>
            </CardHeader>
            <CardContent className="space-y-4">
              <div className="flex gap-4">
                <Input
                  placeholder={t("settings.testEmailPlaceholder")}
                  value={testEmailAddress}
                  onChange={(e) => setTestEmailAddress(e.target.value)}
                  className="flex-1"
                />
                <Button onClick={handleTestEmail} disabled={testingEmail}>
                  {testingEmail && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                  {t("settings.sendTestEmail")}
                </Button>
              </div>
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
                        <div className="space-y-2">
                          <p className="font-medium text-green-800 dark:text-green-200">
                            {t("settings.newVersionAvailable")}
                          </p>
                          {updateInfo.release_date && (
                            <p className="text-sm text-green-700 dark:text-green-300">
                              {t("settings.releaseDate")}: {new Date(updateInfo.release_date).toLocaleDateString()}
                            </p>
                          )}
                          {updateInfo.release_notes && (
                            <div className="mt-2 p-3 bg-white dark:bg-gray-800 rounded text-sm max-h-40 overflow-y-auto">
                              <pre className="whitespace-pre-wrap font-sans">{updateInfo.release_notes}</pre>
                            </div>
                          )}
                          {updateInfo.release_url && (
                            <a
                              href={updateInfo.release_url}
                              target="_blank"
                              rel="noopener noreferrer"
                              className="inline-flex items-center gap-2 mt-2 text-sm text-green-700 dark:text-green-300 hover:underline"
                            >
                              <Download className="h-4 w-4" />
                              {t("settings.downloadUpdate")}
                            </a>
                          )}
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
      </Tabs>
      </motion.div>
    </div>
  );
}
