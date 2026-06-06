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

const DEFAULT_CAP_BASE_URL = "https://captcha.noteva.org";
const DEFAULT_CAP_SITE_KEY = "4d0a333fd4";
const DEFAULT_CAP_SECRET_KEY = "sk-P4HovV1v4cRlLpQiKvQC3bZv5i74UTHaIA0dWKWrLc";

export default function CommentSettingsPage() {
  const { t } = useTranslation();
  const [loading, setLoading] = useState(true);
  const [saving, startSavingTransition] = useTransition();
  const [form, setForm] = useState({
    commentModeration: false,
    moderationKeywords: "",
    captchaProvider: "cap",
    captchaSiteKey: DEFAULT_CAP_SITE_KEY,
    captchaSecretKey: DEFAULT_CAP_SECRET_KEY,
    captchaCapBaseUrl: DEFAULT_CAP_BASE_URL,
    captchaPowDifficulty: "normal",
    captchaChallengeTtlSeconds: "120",
    captchaTokenTtlSeconds: "300",
    captchaPowAutoSolve: true,
  });

  useEffect(() => {
    let active = true;

    const loadSettings = async () => {
      try {
        const { data } = await adminApi.getSettings();
        if (!active) return;
        const rawCaptchaProvider = String(data.captcha_provider || "cap");
        const captchaProvider = rawCaptchaProvider === "noteva_pow" ? "cap" : rawCaptchaProvider;
        const captchaSiteKey = String(data.captcha_site_key || "");
        const captchaSecretKey = String(data.captcha_secret_key || "");

        setForm({
          commentModeration: data.comment_moderation === "true",
          moderationKeywords: String(data.moderation_keywords || ""),
          captchaProvider,
          captchaSiteKey:
            captchaProvider === "cap" && !captchaSiteKey.trim()
              ? DEFAULT_CAP_SITE_KEY
              : captchaSiteKey,
          captchaSecretKey:
            captchaProvider === "cap" && !captchaSecretKey.trim()
              ? DEFAULT_CAP_SECRET_KEY
              : captchaSecretKey,
          captchaCapBaseUrl: String(data.captcha_cap_base_url || DEFAULT_CAP_BASE_URL),
          captchaPowDifficulty: String(data.captcha_pow_difficulty || "normal"),
          captchaChallengeTtlSeconds: String(data.captcha_challenge_ttl_seconds || "120"),
          captchaTokenTtlSeconds: String(data.captcha_token_ttl_seconds || "300"),
          captchaPowAutoSolve: data.captcha_pow_auto_solve !== "false",
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
          captcha_cap_base_url: form.captchaCapBaseUrl,
          captcha_pow_difficulty: form.captchaPowDifficulty,
          captcha_challenge_ttl_seconds: form.captchaChallengeTtlSeconds,
          captcha_token_ttl_seconds: form.captchaTokenTtlSeconds,
          captcha_pow_auto_solve: form.captchaPowAutoSolve ? "true" : "false",
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
                    setForm((current) => ({
                      ...current,
                      captchaProvider: value,
                      captchaCapBaseUrl:
                        value === "cap" && !current.captchaCapBaseUrl.trim()
                          ? DEFAULT_CAP_BASE_URL
                          : current.captchaCapBaseUrl,
                      captchaSiteKey:
                        value === "cap" && !current.captchaSiteKey.trim()
                          ? DEFAULT_CAP_SITE_KEY
                          : current.captchaSiteKey,
                      captchaSecretKey:
                        value === "cap" && !current.captchaSecretKey.trim()
                          ? DEFAULT_CAP_SECRET_KEY
                          : current.captchaSecretKey,
                    }))
                  }
                >
                  <SelectTrigger>
                    <SelectValue />
                  </SelectTrigger>
                  <SelectContent>
                    <SelectItem value="none">{t("settings.none")}</SelectItem>
                    <SelectItem value="turnstile">Cloudflare Turnstile</SelectItem>
                    <SelectItem value="hcaptcha">hCaptcha</SelectItem>
                    <SelectItem value="cap">Cap</SelectItem>
                  </SelectContent>
                </Select>
              </div>

              {form.captchaProvider === "cap" ? (
                <div className="space-y-4 rounded-lg border bg-muted/20 p-4">
                  <div className="space-y-1">
                    <Label>{t("settings.captchaProviderCap")}</Label>
                    <p className="text-sm text-muted-foreground">
                      {t("settings.captchaCapDesc")}
                    </p>
                  </div>
                  <div className="grid gap-4 md:grid-cols-2">
                    <div className="space-y-2 md:col-span-2">
                      <Label htmlFor="captchaCapBaseUrl">{t("settings.captchaCapBaseUrl")}</Label>
                      <Input
                        id="captchaCapBaseUrl"
                        value={form.captchaCapBaseUrl}
                        onChange={(event) =>
                          setForm((current) => ({
                            ...current,
                            captchaCapBaseUrl: event.target.value,
                          }))
                        }
                        placeholder={DEFAULT_CAP_BASE_URL}
                      />
                      <p className="text-xs text-muted-foreground">
                        {t("settings.captchaCapBaseUrlDesc")}
                      </p>
                    </div>
                    <div className="space-y-2">
                      <Label htmlFor="captchaSiteKey">{t("settings.captchaSiteKey")}</Label>
                      <Input
                        id="captchaSiteKey"
                        value={form.captchaSiteKey}
                        onChange={(event) =>
                          setForm((current) => ({ ...current, captchaSiteKey: event.target.value }))
                        }
                        placeholder={DEFAULT_CAP_SITE_KEY}
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
                        placeholder="sk-..."
                        type="password"
                      />
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground">
                    {t("settings.captchaCapSecretDesc")}
                  </p>
                </div>
              ) : form.captchaProvider !== "none" ? (
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
              ) : null}
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
