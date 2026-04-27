import { useEffect, useRef, useState, useTransition } from "react";
import { motion } from "motion/react";
import { adminApi, authApi, localesApi, CustomLocaleItem } from "@/lib/api";
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
import { Settings, User, MessageSquare, Loader2, Download, AlertCircle, Link, Code, Database, Upload, FileText, Type, Shield, ShieldCheck, ShieldOff, Globe, Trash2 } from "lucide-react";
import { toast } from "sonner";
import { useTranslation, registerCustomLocale, unregisterCustomLocale } from "@/lib/i18n";
import { ConfirmDialog } from "@/components/admin/confirm-dialog";
import { getApiErrorMessage } from "@/lib/api-error";

// Permalink format options
const PERMALINK_OPTIONS = [
  { value: "/posts/{slug}", label: "/posts/{slug}", example: "/posts/hello-world" },
  { value: "/posts/{id}", label: "/posts/{id}", example: "/posts/42" },
];

// Curated Google Fonts list
const FONT_OPTIONS = [
  { value: "", label: "System Default" },
  { value: "Inter", label: "Inter" },
  { value: "Noto Sans SC", label: "Noto Sans SC (思源黑体)" },
  { value: "Noto Serif SC", label: "Noto Serif SC (思源宋体)" },
  { value: "LXGW WenKai", label: "LXGW WenKai (霄下文楷)" },
  { value: "Roboto", label: "Roboto" },
  { value: "Open Sans", label: "Open Sans" },
  { value: "Lato", label: "Lato" },
  { value: "Poppins", label: "Poppins" },
  { value: "Montserrat", label: "Montserrat" },
  { value: "Source Sans 3", label: "Source Sans 3" },
  { value: "Merriweather", label: "Merriweather" },
  { value: "Playfair Display", label: "Playfair Display" },
  { value: "JetBrains Mono", label: "JetBrains Mono" },
];

function downloadBlob(blob: Blob, filename: string) {
  const url = URL.createObjectURL(blob);
  const link = document.createElement("a");
  link.href = url;
  link.download = filename;
  link.click();
  URL.revokeObjectURL(url);
}

// Custom Locale Management Card
function CustomLocaleCard() {
  const { t } = useTranslation();
  const [localeList, setLocaleList] = useState<CustomLocaleItem[]>([]);
  const [loading, setLoading] = useState(true);
  const [saving, startSavingTransition] = useTransition();
  const [code, setCode] = useState("");
  const [name, setName] = useState("");
  const [jsonText, setJsonText] = useState("");
  const [urlInput, setUrlInput] = useState("");
  const [loadingUrl, startLoadingUrlTransition] = useTransition();
  const [deleteTarget, setDeleteTarget] = useState<string | null>(null);
  const [deleting, startDeletingTransition] = useTransition();

  const fetchList = async () => {
    try {
      const res = await localesApi.list();
      setLocaleList(res.data.locales);
    } catch {
      // silent
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    let active = true;

    const loadLocales = async () => {
      try {
        const res = await localesApi.list();
        if (active) setLocaleList(res.data.locales);
      } catch {
        // silent
      } finally {
        if (active) setLoading(false);
      }
    };

    void loadLocales();
    return () => {
      active = false;
    };
  }, []);

  const handleLoadFromUrl = () => {
    if (!urlInput.trim()) return;
    startLoadingUrlTransition(async () => {
      try {
        const res = await fetch(urlInput.trim());
        const text = await res.text();
        JSON.parse(text);
        setJsonText(text);
        toast.success("JSON loaded from URL");
      } catch {
        toast.error(t("manage.localeJsonInvalid"));
      }
    });
  };

  const handleSubmit = () => {
    if (!code.trim() || !name.trim() || !jsonText.trim()) return;
    let parsed: Record<string, unknown>;
    try {
      parsed = JSON.parse(jsonText);
      if (typeof parsed !== "object" || Array.isArray(parsed)) throw new Error();
    } catch {
      toast.error(t("manage.localeJsonInvalid"));
      return;
    }
    startSavingTransition(async () => {
      try {
        await localesApi.upsert(code.trim(), name.trim(), parsed);
        registerCustomLocale(code.trim(), name.trim(), parsed);
        toast.success(t("manage.localeUploadSuccess"));
        setCode(""); setName(""); setJsonText(""); setUrlInput("");
        await fetchList();
      } catch {
        toast.error(t("manage.localeUploadError"));
      }
    });
  };

  const handleDelete = (localeCode: string) => {
    setDeleteTarget(localeCode);
  };

  const confirmDelete = () => {
    if (!deleteTarget) return;

    const localeCode = deleteTarget;
    setDeleteTarget(null);
    startDeletingTransition(async () => {
      try {
        await localesApi.delete(localeCode);
        unregisterCustomLocale(localeCode);
        toast.success(t("manage.localeDeleteSuccess"));
        await fetchList();
      } catch {
        toast.error(t("manage.localeDeleteError"));
      }
    });
  };

  const handleFileUpload = (e: React.ChangeEvent<HTMLInputElement>) => {
    const file = e.target.files?.[0];
    if (!file) return;
    const reader = new FileReader();
    reader.onload = (ev) => {
      const text = ev.target?.result as string;
      try {
        JSON.parse(text);
        setJsonText(text);
        // Auto-fill code from filename (e.g. "ja.json" -> "ja")
        const basename = file.name.replace(/\.json$/i, "");
        if (!code) setCode(basename);
      } catch {
        toast.error(t("manage.localeJsonInvalid"));
      }
    };
    reader.readAsText(file);
  };

  return (
    <>
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Globe className="h-5 w-5" />
          {t("manage.customLocales")}
        </CardTitle>
        <CardDescription>{t("manage.customLocalesDesc")}</CardDescription>
      </CardHeader>
      <CardContent className="space-y-6">
        {/* Existing custom locales */}
        {loading ? (
          <Skeleton className="h-10 w-full" />
        ) : localeList.length > 0 ? (
          <div className="space-y-2">
            {localeList.map((loc) => (
              <div key={loc.code} className="flex items-center justify-between p-3 rounded-lg border bg-muted/30">
                <div>
                  <span className="font-medium">{loc.name}</span>
                  <span className="ml-2 text-sm text-muted-foreground">({loc.code})</span>
                </div>
                <Button variant="ghost" size="icon" onClick={() => handleDelete(loc.code)}>
                  <Trash2 className="h-4 w-4 text-destructive" />
                </Button>
              </div>
            ))}
          </div>
        ) : (
          <p className="text-sm text-muted-foreground">{t("manage.localeNoCustom")}</p>
        )}

        {/* Upload form */}
        <div className="space-y-4 pt-4 border-t">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label>{t("manage.localeCode")}</Label>
              <Input
                placeholder={t("manage.localeCodePlaceholder")}
                value={code}
                onChange={(e) => setCode(e.target.value)}
              />
            </div>
            <div className="space-y-2">
              <Label>{t("manage.localeName")}</Label>
              <Input
                placeholder={t("manage.localeNamePlaceholder")}
                value={name}
                onChange={(e) => setName(e.target.value)}
              />
            </div>
          </div>

          {/* Load from URL */}
          <div className="space-y-2">
            <Label>{t("manage.localeLoadFromUrl")}</Label>
            <div className="flex gap-2">
              <Input
                placeholder={t("manage.localeUrlPlaceholder")}
                value={urlInput}
                onChange={(e) => setUrlInput(e.target.value)}
                className="flex-1"
              />
              <Button variant="outline" onClick={handleLoadFromUrl} disabled={loadingUrl || !urlInput.trim()}>
                {loadingUrl ? <Loader2 className="h-4 w-4 animate-spin" /> : <Download className="h-4 w-4" />}
              </Button>
            </div>
          </div>

          {/* JSON content */}
          <div className="space-y-2">
            <div className="flex items-center justify-between">
              <Label>{t("manage.localeJsonContent")}</Label>
              <label className="cursor-pointer text-sm text-primary hover:underline flex items-center gap-1">
                <Upload className="h-3.5 w-3.5" />
                <span>{t("manage.localeUpload")}</span>
                <input type="file" accept=".json" className="hidden" onChange={handleFileUpload} />
              </label>
            </div>
            <textarea
              className="w-full min-h-[200px] p-3 rounded-md border bg-background text-sm font-mono resize-y focus:outline-none focus:ring-2 focus:ring-ring"
              placeholder={t("manage.localeJsonPlaceholder")}
              value={jsonText}
              onChange={(e) => setJsonText(e.target.value)}
            />
          </div>

          <Button onClick={handleSubmit} disabled={saving || !code.trim() || !name.trim() || !jsonText.trim()}>
            {saving && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
            {saving ? t("manage.localeUploading") : t("manage.localeUpload")}
          </Button>
        </div>
      </CardContent>
    </Card>
    <ConfirmDialog
      open={deleteTarget !== null}
      title={t("common.confirm")}
      description={t("manage.localeDeleteConfirm")}
      confirmLabel={t("common.delete")}
      cancelLabel={t("common.cancel")}
      destructive
      loading={deleting}
      onOpenChange={(open) => !open && setDeleteTarget(null)}
      onConfirm={confirmDelete}
    />
    </>
  );
}

// Two-Factor Authentication Card component
function TwoFactorCard() {
  const { t } = useTranslation();
  const [loading2FA, setLoading2FA] = useState(true);
  const [enabled, setEnabled] = useState(false);
  const [setupData, setSetupData] = useState<{ secret: string; qr_code: string } | null>(null);
  const [verifyCode, setVerifyCode] = useState("");
  const [disablePassword, setDisablePassword] = useState("");
  const [disableCode, setDisableCode] = useState("");
  const [processing, startProcessingTransition] = useTransition();
  const [showSetup, setShowSetup] = useState(false);
  const [showDisable, setShowDisable] = useState(false);

  useEffect(() => {
    let active = true;
    authApi.get2FAStatus()
      .then(({ data }) => {
        if (active) setEnabled(data.enabled);
      })
      .catch(() => {})
      .finally(() => {
        if (active) setLoading2FA(false);
      });
    return () => {
      active = false;
    };
  }, []);

  const handleSetup = () => {
    startProcessingTransition(async () => {
      try {
        const { data } = await authApi.setup2FA();
        setSetupData(data);
        setShowSetup(true);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("settings.saveFailed")));
      }
    });
  };

  const handleEnable = () => {
    if (!verifyCode.trim()) {
      toast.error(t("settings.2faCodeRequired") || "Please enter the verification code");
      return;
    }
    startProcessingTransition(async () => {
      try {
        await authApi.enable2FA(verifyCode.trim());
        setEnabled(true);
        setShowSetup(false);
        setSetupData(null);
        setVerifyCode("");
        toast.success(t("settings.2faEnabled") || "Two-factor authentication enabled");
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("settings.saveFailed")));
      }
    });
  };

  const handleDisable = () => {
    if (!disablePassword || !disableCode.trim()) {
      toast.error(t("settings.2faDisableRequired") || "Password and verification code are required");
      return;
    }
    startProcessingTransition(async () => {
      try {
        await authApi.disable2FA(disablePassword, disableCode.trim());
        setEnabled(false);
        setShowDisable(false);
        setDisablePassword("");
        setDisableCode("");
        toast.success(t("settings.2faDisabled") || "Two-factor authentication disabled");
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("settings.saveFailed")));
      }
    });
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Shield className="h-5 w-5" />
          {t("settings.twoFactorAuth") || "Two-Factor Authentication"}
        </CardTitle>
        <CardDescription>
          {t("settings.twoFactorAuthDesc") || "Add an extra layer of security to your account using a TOTP authenticator app."}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        {loading2FA ? (
          <Skeleton className="h-10 w-full" />
        ) : enabled ? (
          /* 2FA is enabled */
          <>
            <div className="flex items-center gap-3 p-3 rounded-lg bg-green-50 dark:bg-green-900/20 border border-green-200 dark:border-green-800">
              <ShieldCheck className="h-5 w-5 text-green-600 dark:text-green-400" />
              <span className="text-sm font-medium text-green-800 dark:text-green-200">
                {t("settings.2faStatusEnabled") || "Two-factor authentication is enabled"}
              </span>
            </div>
            {showDisable ? (
              <div className="space-y-3 p-4 rounded-lg border bg-muted/30">
                <div className="space-y-2">
                  <Label>{t("settings.currentPassword")}</Label>
                  <Input
                    type="password"
                    value={disablePassword}
                    onChange={(e) => setDisablePassword(e.target.value)}
                    placeholder={t("settings.currentPassword")}
                  />
                </div>
                <div className="space-y-2">
                  <Label>{t("settings.2faCode") || "Authenticator Code"}</Label>
                  <Input
                    value={disableCode}
                    onChange={(e) => setDisableCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                    placeholder="000000"
                    maxLength={6}
                    className="font-mono text-center text-lg tracking-widest w-40"
                  />
                </div>
                <div className="flex gap-2">
                  <Button variant="destructive" onClick={handleDisable} disabled={processing}>
                    {processing && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                    {t("settings.2faDisableConfirm") || "Disable 2FA"}
                  </Button>
                  <Button variant="outline" onClick={() => { setShowDisable(false); setDisablePassword(""); setDisableCode(""); }}>
                    {t("common.cancel")}
                  </Button>
                </div>
              </div>
            ) : (
              <Button variant="outline" onClick={() => setShowDisable(true)}>
                <ShieldOff className="h-4 w-4 mr-2" />
                {t("settings.2faDisable") || "Disable Two-Factor Authentication"}
              </Button>
            )}
          </>
        ) : showSetup && setupData ? (
          /* Setup flow: show QR code and verify */
          <div className="space-y-4">
            <div className="flex flex-col items-center gap-3 p-4 rounded-lg border bg-muted/30">
              <p className="text-sm text-muted-foreground text-center">
                {t("settings.2faScanQR") || "Scan this QR code with your authenticator app (Google Authenticator, Authy, etc.)"}
              </p>
              <img src={setupData.qr_code} alt="2FA QR Code" className="w-48 h-48" />
              <div className="space-y-1 text-center">
                <p className="text-xs text-muted-foreground">
                  {t("settings.2faManualEntry") || "Or enter this key manually:"}
                </p>
                <code className="text-xs bg-muted px-2 py-1 rounded select-all break-all">
                  {setupData.secret}
                </code>
              </div>
            </div>
            <div className="space-y-2">
              <Label>{t("settings.2faVerifyCode") || "Enter the 6-digit code from your app"}</Label>
              <div className="flex gap-2">
                <Input
                  value={verifyCode}
                  onChange={(e) => setVerifyCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                  placeholder="000000"
                  maxLength={6}
                  className="font-mono text-center text-lg tracking-widest w-40"
                />
                <Button onClick={handleEnable} disabled={processing || verifyCode.length < 6}>
                  {processing && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
                  {t("settings.2faVerifyAndEnable") || "Verify & Enable"}
                </Button>
              </div>
            </div>
            <Button variant="ghost" size="sm" onClick={() => { setShowSetup(false); setSetupData(null); setVerifyCode(""); }}>
              {t("common.cancel")}
            </Button>
          </div>
        ) : (
          /* 2FA not enabled, show setup button */
          <>
            <div className="flex items-center gap-3 p-3 rounded-lg bg-muted/50 border">
              <ShieldOff className="h-5 w-5 text-muted-foreground" />
              <span className="text-sm text-muted-foreground">
                {t("settings.2faStatusDisabled") || "Two-factor authentication is not enabled"}
              </span>
            </div>
            <Button onClick={handleSetup} disabled={processing}>
              {processing && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              <Shield className="h-4 w-4 mr-2" />
              {t("settings.2faSetup") || "Set Up Two-Factor Authentication"}
            </Button>
          </>
        )}
      </CardContent>
    </Card>
  );
}
export default function SettingsPage() {
  const { user, setUser } = useAuthStore();
  const { updateSettings } = useSiteStore();
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [savingSite, startSavingSiteTransition] = useTransition();
  const [savingComment, startSavingCommentTransition] = useTransition();
  const [savingProfile, startSavingProfileTransition] = useTransition();
  const [savingCustomCode, startSavingCustomCodeTransition] = useTransition();

  const [siteForm, setSiteForm] = useState({
    siteName: "",
    siteDescription: "",
    siteSubtitle: "",
    siteLogo: "",
    siteFooter: "",
    siteUrl: "",
    permalinkStructure: "/posts/{slug}",
    fontFamily: "",
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

  // Backup state
  const [backingUp, startBackupTransition] = useTransition();
  const [restoring, startRestoreTransition] = useTransition();
  const [exportingMd, startExportMarkdownTransition] = useTransition();
  const [restoreDialogOpen, setRestoreDialogOpen] = useState(false);
  const restoreFileRef = useRef<File | null>(null);
  const restoreInputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    let active = true;

    const loadSettings = async () => {
      try {
        const { data } = await adminApi.getSettings();
        if (!active) return;

        setSiteForm({
          siteName: data.site_name || "",
          siteDescription: data.site_description || "",
          siteSubtitle: data.site_subtitle || "",
          siteLogo: data.site_logo || "",
          siteFooter: data.site_footer || "",
          siteUrl: data.site_url || "",
          permalinkStructure: data.permalink_structure || "/posts/{slug}",
          fontFamily: String(data.font_family || ""),
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
      } catch (err) {
        if (!active) return;
        console.error("Failed to load settings:", err);
        setSiteForm({
          siteName: "Noteva",
          siteDescription: "",
          siteSubtitle: "",
          siteLogo: "",
          siteFooter: "",
          siteUrl: "",
          permalinkStructure: "/posts/{slug}",
          fontFamily: "",
        });
        toast.error(t("error.loadFailed"));
      } finally {
        if (active) setLoading(false);
      }
    };

    loadSettings();
    return () => {
      active = false;
    };
  }, [t, user]);

  // Preload Google Font for preview when saved font is loaded
  useEffect(() => {
    if (siteForm.fontFamily && !document.querySelector(`link[data-font="${siteForm.fontFamily}"]`)) {
      const link = document.createElement("link");
      link.rel = "stylesheet";
      link.href = `https://fonts.loli.net/css2?family=${encodeURIComponent(siteForm.fontFamily)}:wght@400;700&display=swap`;
      link.setAttribute("data-font", siteForm.fontFamily);
      document.head.appendChild(link);
    }
  }, [siteForm.fontFamily]);

  const handleSaveSiteSettings = () => {
    if (!siteForm.siteName.trim()) {
      toast.error(t("settings.siteNameRequired"));
      return;
    }
    if (siteForm.siteUrl.trim() && !/^https?:\/\/.+/.test(siteForm.siteUrl.trim())) {
      toast.error(t("settings.siteUrlInvalid"));
      return;
    }
    startSavingSiteTransition(async () => {
      try {
        const newSettings = {
          site_name: siteForm.siteName,
          site_description: siteForm.siteDescription,
          site_subtitle: siteForm.siteSubtitle,
          site_logo: siteForm.siteLogo,
          site_footer: siteForm.siteFooter,
          site_url: siteForm.siteUrl,
          permalink_structure: siteForm.permalinkStructure,
          font_family: siteForm.fontFamily,
        };
        await adminApi.updateSettings(newSettings);
        updateSettings(newSettings);
        toast.success(t("settings.saveSuccess"));
      } catch {
        toast.error(t("settings.saveFailed"));
      }
    });
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
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("settings.saveFailed")));
    }
  };

  const handleSaveCommentSettings = () => {
    startSavingCommentTransition(async () => {
      try {
        await adminApi.updateSettings({
          comment_moderation: commentForm.commentModeration ? "true" : "false",
          moderation_keywords: commentForm.moderationKeywords,
        });
        toast.success(t("settings.saveSuccess"));
      } catch {
        toast.error(t("settings.saveFailed"));
      }
    });
  };

  const handleSaveCustomCode = () => {
    startSavingCustomCodeTransition(async () => {
      try {
        await adminApi.updateSettings({
          custom_css: customCodeForm.customCss,
          custom_js: customCodeForm.customJs,
        });
        toast.success(t("settings.saveSuccess"));
      } catch {
        toast.error(t("settings.saveFailed"));
      }
    });
  };

  const handleSaveProfile = () => {
    startSavingProfileTransition(async () => {
      try {
        const { data } = await authApi.updateProfile({
          display_name: profileForm.displayName || null,
          avatar: profileForm.avatar || null,
        });
        setUser(data);
        toast.success(t("settings.saveSuccess"));
      } catch {
        toast.error(t("settings.saveFailed"));
      }
    });
  };

  const handleDownloadBackup = () => {
    startBackupTransition(async () => {
      try {
        const res = await adminApi.downloadBackup();
        downloadBlob(
          new Blob([res.data], { type: "application/zip" }),
          `noteva-backup-${new Date().toISOString().slice(0, 10)}.zip`
        );
        toast.success(t("settings.backupSuccess"));
      } catch {
        toast.error(t("settings.backupFailed"));
      }
    });
  };

  const handleSelectRestoreFile = (event: React.ChangeEvent<HTMLInputElement>) => {
    const file = event.target.files?.[0];
    event.target.value = "";
    if (!file) return;

    restoreFileRef.current = file;
    setRestoreDialogOpen(true);
  };

  const handleRestoreBackup = () => {
    const file = restoreFileRef.current;
    if (!file) return;

    setRestoreDialogOpen(false);
    startRestoreTransition(async () => {
      try {
        await adminApi.restoreBackup(file);
        toast.success(t("settings.restoreSuccess"));
        window.setTimeout(() => window.location.reload(), 2000);
      } catch {
        toast.error(t("settings.restoreFailed"));
      }
    });
  };

  const handleExportMarkdown = () => {
    startExportMarkdownTransition(async () => {
      try {
        const res = await adminApi.exportMarkdown();
        downloadBlob(
          new Blob([res.data], { type: "application/zip" }),
          `noteva-articles-${new Date().toISOString().slice(0, 10)}.zip`
        );
        toast.success(t("settings.exportSuccess"));
      } catch {
        toast.error(t("settings.exportFailed"));
      }
    });
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
            <TabsTrigger value="locales" className="gap-2">
              <Globe className="h-4 w-4" />
              {t("manage.customLocales")}
            </TabsTrigger>
            <TabsTrigger value="comments" className="gap-2">
              <MessageSquare className="h-4 w-4" />
              {t("settings.comments")}
            </TabsTrigger>
            <TabsTrigger value="account" className="gap-2">
              <User className="h-4 w-4" />
              {t("settings.account")}
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
                    <div className="space-y-2 pt-4 border-t">
                      <Label className="flex items-center gap-2">
                        <Type className="h-4 w-4" />
                        {t("settings.fontFamily")}
                      </Label>
                      <p className="text-sm text-muted-foreground">
                        {t("settings.fontFamilyDesc")}
                      </p>
                      <Select
                        value={siteForm.fontFamily || "__system__"}
                        onValueChange={(v) => {
                          const val = v === "__system__" ? "" : v;
                          setSiteForm((f) => ({ ...f, fontFamily: val }));
                          // Dynamically load Google Font for preview
                          if (val && !document.querySelector(`link[data-font="${val}"]`)) {
                            const link = document.createElement("link");
                            link.rel = "stylesheet";
                            link.href = `https://fonts.loli.net/css2?family=${encodeURIComponent(val)}:wght@400;700&display=swap`;
                            link.setAttribute("data-font", val);
                            document.head.appendChild(link);
                          }
                        }}
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          {FONT_OPTIONS.map((opt) => (
                            <SelectItem key={opt.value || "__system__"} value={opt.value || "__system__"}>
                              {opt.label}
                            </SelectItem>
                          ))}
                        </SelectContent>
                      </Select>
                      <p className="text-xs text-muted-foreground">
                        {t("settings.fontCustomHint")}
                      </p>
                      <Input
                        placeholder={t("settings.fontCustomPlaceholder")}
                        value={FONT_OPTIONS.some(o => o.value === siteForm.fontFamily) ? "" : siteForm.fontFamily}
                        onChange={(e) => {
                          const val = e.target.value;
                          setSiteForm((f) => ({ ...f, fontFamily: val }));
                          if (val && !document.querySelector(`link[data-font="${val}"]`)) {
                            const link = document.createElement("link");
                            link.rel = "stylesheet";
                            link.href = `https://fonts.loli.net/css2?family=${encodeURIComponent(val)}:wght@400;700&display=swap`;
                            link.setAttribute("data-font", val);
                            document.head.appendChild(link);
                          }
                        }}
                      />
                      {siteForm.fontFamily && (
                        <div
                          className="p-4 rounded-md border bg-muted/30 space-y-1"
                          style={{ fontFamily: `"${siteForm.fontFamily}", sans-serif` }}
                        >
                          <p className="text-sm font-medium">{t("settings.fontPreview")}</p>
                          <p className="text-base">The quick brown fox jumps over the lazy dog.</p>
                          <p className="text-base">你好世界！这是一段中文预览文本。1234567890</p>
                          <p className="text-lg font-bold">Bold 粗体文本 Preview</p>
                        </div>
                      )}
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

          <TabsContent value="locales" className="space-y-4">
            <CustomLocaleCard />
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

            {/* Two-Factor Authentication Card */}
            <TwoFactorCard />
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
                <input
                  ref={restoreInputRef}
                  type="file"
                  accept=".zip"
                  className="hidden"
                  onChange={handleSelectRestoreFile}
                />
                <div className="flex flex-col gap-3">
                  <div className="flex items-center justify-between p-4 rounded-lg border">
                    <div>
                      <p className="font-medium">{t("settings.downloadBackup")}</p>
                      <p className="text-sm text-muted-foreground">{t("settings.downloadBackupDesc")}</p>
                    </div>
                    <Button
                      onClick={handleDownloadBackup}
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
                      onClick={() => restoreInputRef.current?.click()}
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
                      onClick={handleExportMarkdown}
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
              onClick={handleRestoreBackup}
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

