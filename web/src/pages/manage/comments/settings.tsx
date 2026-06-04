import { useEffect, useState, useTransition } from "react";
import { adminApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Loader2, MessageSquare, ShieldCheck } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";

export default function CommentSettingsPage() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [saving, startSavingTransition] = useTransition();
  const [form, setForm] = useState({
    commentModeration: false,
    moderationKeywords: "",
    captchaProvider: "none",
    captchaSiteKey: "",
    captchaSecretKey: "",
  });

  useEffect(() => {
    let active = true;

    const loadSettings = async () => {
      try {
        const { data } = await adminApi.getSettings();
        if (!active) return;

        setForm({
          commentModeration: data.comment_moderation === "true",
          moderationKeywords: String(data.moderation_keywords || ""),
          captchaProvider: String(data.captcha_provider || "none"),
          captchaSiteKey: String(data.captcha_site_key || ""),
          captchaSecretKey: String(data.captcha_secret_key || ""),
        });
      } catch {
        if (active) toast.error(t("error.loadFailed"));
      } finally {
        if (active) setLoading(false);
      }
    };

    void loadSettings();

    return () => {
      active = false;
    };
  }, [t]);

  const handleSave = () => {
    startSavingTransition(async () => {
      try {
        await adminApi.updateSettings({
          comment_moderation: form.commentModeration ? "true" : "false",
          moderation_keywords: form.moderationKeywords,
          captcha_provider: form.captchaProvider,
          captcha_site_key: form.captchaSiteKey,
          captcha_secret_key: form.captchaSecretKey,
        });
        toast.success(t("settings.saveSuccess"));
      } catch {
        toast.error(t("settings.saveFailed"));
      }
    });
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="flex items-center gap-2 text-3xl font-bold">
          <MessageSquare className="h-7 w-7" />
          {t("settings.commentSettings")}
        </h1>
        <p className="mt-1 text-muted-foreground">{t("settings.commentSettingsDesc")}</p>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("settings.commentModeration")}</CardTitle>
          <CardDescription>{t("settings.commentModerationDesc")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-5">
          {loading ? (
            <div className="space-y-4">
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
            </div>
          ) : (
            <>
              <div className="flex items-center justify-between gap-4">
                <div className="space-y-0.5">
                  <Label>{t("settings.commentModeration")}</Label>
                  <p className="text-sm text-muted-foreground">
                    {t("settings.commentModerationDesc")}
                  </p>
                </div>
                <label className="relative inline-flex shrink-0 cursor-pointer items-center">
                  <input
                    type="checkbox"
                    checked={form.commentModeration}
                    onChange={(event) =>
                      setForm((current) => ({
                        ...current,
                        commentModeration: event.target.checked,
                      }))
                    }
                    className="peer sr-only"
                  />
                  <div className="h-6 w-11 rounded-full bg-gray-200 peer-checked:bg-blue-600 peer-focus:outline-none peer-focus:ring-4 peer-focus:ring-blue-300 after:absolute after:left-[2px] after:top-[2px] after:h-5 after:w-5 after:rounded-full after:border after:border-gray-300 after:bg-white after:transition-all after:content-[''] peer-checked:after:translate-x-full peer-checked:after:border-white dark:bg-gray-700 dark:border-gray-600 dark:peer-focus:ring-blue-800" />
                </label>
              </div>

              <div className="space-y-2">
                <Label htmlFor="moderationKeywords">{t("settings.moderationKeywords")}</Label>
                <p className="text-sm text-muted-foreground">
                  {t("settings.moderationKeywordsDesc")}
                </p>
                <Input
                  id="moderationKeywords"
                  value={form.moderationKeywords}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      moderationKeywords: event.target.value,
                    }))
                  }
                  placeholder={t("settings.moderationKeywordsPlaceholder")}
                />
              </div>
            </>
          )}
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <ShieldCheck className="h-5 w-5" />
            {t("settings.captcha")}
          </CardTitle>
          <CardDescription>{t("settings.captchaDesc")}</CardDescription>
        </CardHeader>
        <CardContent className="space-y-4">
          {loading ? (
            <div className="space-y-4">
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
              <Skeleton className="h-10 w-full" />
            </div>
          ) : (
            <>
              <div className="space-y-2">
                <Label>{t("settings.captchaProvider")}</Label>
                <Select
                  value={form.captchaProvider}
                  onValueChange={(value) =>
                    setForm((current) => ({ ...current, captchaProvider: value }))
                  }
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">{t("settings.none")}</SelectItem>
                    <SelectItem value="turnstile">Cloudflare Turnstile</SelectItem>
                    <SelectItem value="hcaptcha">hCaptcha</SelectItem>
                  </SelectContent>
                </Select>
              </div>
              <div className="grid gap-4 md:grid-cols-2">
                <div className="space-y-2">
                  <Label htmlFor="captchaSiteKey">{t("settings.captchaSiteKey")}</Label>
                  <Input
                    id="captchaSiteKey"
                    value={form.captchaSiteKey}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, captchaSiteKey: event.target.value }))
                    }
                    placeholder="Site key"
                  />
                </div>
                <div className="space-y-2">
                  <Label htmlFor="captchaSecretKey">{t("settings.captchaSecretKey")}</Label>
                  <Input
                    id="captchaSecretKey"
                    value={form.captchaSecretKey}
                    onChange={(event) =>
                      setForm((current) => ({ ...current, captchaSecretKey: event.target.value }))
                    }
                    placeholder="Secret key"
                    type="password"
                  />
                </div>
              </div>
            </>
          )}

          <Button onClick={handleSave} disabled={saving || loading}>
            {saving && <Loader2 className="mr-2 h-4 w-4 animate-spin" />}
            {t("settings.saveSettings")}
          </Button>
        </CardContent>
      </Card>
    </div>
  );
}
