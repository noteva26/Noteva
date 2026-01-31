"use client";

import { useState, useEffect } from "react";
import { adminApi } from "@/lib/api";
import { useAuthStore } from "@/lib/store/auth";
import { useSiteStore } from "@/lib/store/site";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { Settings, User, MessageSquare, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";

export default function SettingsPage() {
  const { user } = useAuthStore();
  const { updateSettings } = useSiteStore();
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [savingSite, setSavingSite] = useState(false);
  const [savingComment, setSavingComment] = useState(false);

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
  });

  const [passwordForm, setPasswordForm] = useState({
    currentPassword: "",
    newPassword: "",
    confirmPassword: "",
  });

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
      });
      toast.success(t("settings.saveSuccess"));
    } catch (error) {
      toast.error(t("settings.saveFailed"));
    } finally {
      setSavingComment(false);
    }
  };


  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">{t("settings.title")}</h1>
        <p className="text-muted-foreground">{t("settings.site")}</p>
      </div>

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
      </Tabs>
    </div>
  );
}
