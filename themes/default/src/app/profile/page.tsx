"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import { toast } from "sonner";
import { useAuthStore } from "@/lib/store/auth";
import { useSiteStore } from "@/lib/store/site";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Tabs, TabsContent, TabsList, TabsTrigger } from "@/components/ui/tabs";
import { useTranslation } from "@/lib/i18n";
import { User, Lock, ArrowLeft } from "lucide-react";
import Link from "next/link";

export default function ProfilePage() {
  const router = useRouter();
  const { user, isLoading, updateProfile, changePassword, checkAuth } = useAuthStore();
  const { settings, fetchSettings } = useSiteStore();
  const { t } = useTranslation();
  const [checking, setChecking] = useState(true);

  const [profileForm, setProfileForm] = useState({
    display_name: "",
    avatar: "",
  });

  const [passwordForm, setPasswordForm] = useState({
    currentPassword: "",
    newPassword: "",
    confirmPassword: "",
  });

  // Fetch site settings
  useEffect(() => {
    fetchSettings();
  }, [fetchSettings]);

  // Update page title
  useEffect(() => {
    if (settings.site_name) {
      document.title = `${t("profile.title")} - ${settings.site_name}`;
    }
  }, [settings.site_name, t]);

  // Check auth and redirect if not logged in
  useEffect(() => {
    checkAuth().then(() => {
      const state = useAuthStore.getState();
      if (!state.isAuthenticated) {
        router.replace("/login");
      } else {
        setChecking(false);
        // Initialize form with current user data
        if (state.user) {
          setProfileForm({
            display_name: state.user.display_name || "",
            avatar: state.user.avatar || "",
          });
        }
      }
    });
  }, [router, checkAuth]);

  const handleUpdateProfile = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      await updateProfile(profileForm);
      toast.success(t("profile.updateSuccess"));
    } catch {
      toast.error(t("profile.updateFailed"));
    }
  };

  const handleChangePassword = async (e: React.FormEvent) => {
    e.preventDefault();
    
    if (passwordForm.newPassword.length < 8) {
      toast.error(t("auth.passwordTooShort"));
      return;
    }
    
    if (passwordForm.newPassword !== passwordForm.confirmPassword) {
      toast.error(t("auth.passwordMismatch"));
      return;
    }
    
    try {
      await changePassword(passwordForm.currentPassword, passwordForm.newPassword);
      toast.success(t("profile.passwordChanged"));
      setPasswordForm({ currentPassword: "", newPassword: "", confirmPassword: "" });
    } catch {
      toast.error(t("profile.passwordChangeFailed"));
    }
  };

  if (checking) {
    return (
      <div className="flex min-h-screen items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  return (
    <div className="min-h-screen bg-muted/50 py-8">
      <div className="container max-w-2xl mx-auto px-4">
        <div className="mb-6">
          <Link href="/" className="inline-flex items-center text-sm text-muted-foreground hover:text-foreground">
            <ArrowLeft className="h-4 w-4 mr-1" />
            {t("common.backToHome")}
          </Link>
        </div>

        <Card>
          <CardHeader>
            <CardTitle>{t("profile.title")}</CardTitle>
            <CardDescription>{t("profile.description")}</CardDescription>
          </CardHeader>
          <CardContent>
            <Tabs defaultValue="profile">
              <TabsList className="mb-4">
                <TabsTrigger value="profile" className="gap-2">
                  <User className="h-4 w-4" />
                  {t("profile.info")}
                </TabsTrigger>
                <TabsTrigger value="password" className="gap-2">
                  <Lock className="h-4 w-4" />
                  {t("profile.changePassword")}
                </TabsTrigger>
              </TabsList>

              <TabsContent value="profile">
                <form onSubmit={handleUpdateProfile} className="space-y-4">
                  <div className="space-y-2">
                    <Label>{t("auth.username")}</Label>
                    <Input value={user?.username || ""} disabled />
                    <p className="text-xs text-muted-foreground">{t("profile.usernameCannotChange")}</p>
                  </div>

                  <div className="space-y-2">
                    <Label>{t("auth.email")}</Label>
                    <Input value={user?.email || ""} disabled />
                    <p className="text-xs text-muted-foreground">{t("profile.emailCannotChange")}</p>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="display_name">{t("profile.displayName")}</Label>
                    <Input
                      id="display_name"
                      placeholder={t("profile.displayNamePlaceholder")}
                      value={profileForm.display_name}
                      onChange={(e) => setProfileForm({ ...profileForm, display_name: e.target.value })}
                    />
                    <p className="text-xs text-muted-foreground">{t("profile.displayNameDesc")}</p>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="avatar">{t("profile.avatar")}</Label>
                    <Input
                      id="avatar"
                      placeholder="https://..."
                      value={profileForm.avatar}
                      onChange={(e) => setProfileForm({ ...profileForm, avatar: e.target.value })}
                    />
                    {profileForm.avatar && (
                      <div className="mt-2">
                        <img
                          src={profileForm.avatar}
                          alt="Avatar preview"
                          className="w-16 h-16 rounded-full object-cover"
                          onError={(e) => { (e.target as HTMLImageElement).style.display = 'none'; }}
                        />
                      </div>
                    )}
                  </div>

                  <Button type="submit" disabled={isLoading}>
                    {isLoading ? t("common.loading") : t("common.save")}
                  </Button>
                </form>
              </TabsContent>

              <TabsContent value="password">
                <form onSubmit={handleChangePassword} className="space-y-4">
                  <div className="space-y-2">
                    <Label htmlFor="currentPassword">{t("profile.currentPassword")}</Label>
                    <Input
                      id="currentPassword"
                      type="password"
                      value={passwordForm.currentPassword}
                      onChange={(e) => setPasswordForm({ ...passwordForm, currentPassword: e.target.value })}
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="newPassword">{t("profile.newPassword")}</Label>
                    <Input
                      id="newPassword"
                      type="password"
                      value={passwordForm.newPassword}
                      onChange={(e) => setPasswordForm({ ...passwordForm, newPassword: e.target.value })}
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="confirmPassword">{t("profile.confirmNewPassword")}</Label>
                    <Input
                      id="confirmPassword"
                      type="password"
                      value={passwordForm.confirmPassword}
                      onChange={(e) => setPasswordForm({ ...passwordForm, confirmPassword: e.target.value })}
                    />
                  </div>

                  <Button type="submit" disabled={isLoading}>
                    {isLoading ? t("common.loading") : t("profile.updatePassword")}
                  </Button>
                </form>
              </TabsContent>
            </Tabs>
          </CardContent>
        </Card>
      </div>
    </div>
  );
}
