"use client";

import { useState, useEffect } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { toast } from "sonner";
import { useAuthStore } from "@/lib/store/auth";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardFooter, CardHeader, CardTitle } from "@/components/ui/card";
import { useTranslation } from "@/lib/i18n";
import { useSiteStore } from "@/lib/store/site";

export default function RegisterPage() {
  const router = useRouter();
  const { register: registerUser, isLoading } = useAuthStore();
  const { settings, fetchSettings } = useSiteStore();
  const { t } = useTranslation();
  const [form, setForm] = useState({ username: "", email: "", password: "", confirmPassword: "", verificationCode: "" });
  const [errors, setErrors] = useState<Record<string, string>>({});
  const [emailVerificationEnabled, setEmailVerificationEnabled] = useState(false);
  const [sendingCode, setSendingCode] = useState(false);
  const [countdown, setCountdown] = useState(0);

  // Fetch site settings
  useEffect(() => {
    fetchSettings();
  }, [fetchSettings]);

  // Check if email verification is enabled - fetch directly from API
  useEffect(() => {
    const checkEmailVerification = async () => {
      try {
        // 直接从 API 获取，避免缓存问题
        const response = await fetch('/api/v1/site/info');
        const data = await response.json();
        if (data.email_verification_enabled === "true") {
          setEmailVerificationEnabled(true);
        }
      } catch (e) {
        // 如果 API 失败，回退到 store 中的值
        if (settings.email_verification_enabled === "true") {
          setEmailVerificationEnabled(true);
        }
      }
    };
    checkEmailVerification();
  }, [settings.email_verification_enabled]);

  // Update page title
  useEffect(() => {
    if (settings.site_name) {
      document.title = `${t("auth.register")} - ${settings.site_name}`;
    }
  }, [settings.site_name, t]);

  // Countdown timer
  useEffect(() => {
    if (countdown > 0) {
      const timer = setTimeout(() => setCountdown(countdown - 1), 1000);
      return () => clearTimeout(timer);
    }
  }, [countdown]);

  const validate = () => {
    const newErrors: Record<string, string> = {};
    if (!form.username || form.username.length < 3 || form.username.length > 20) {
      newErrors.username = t("auth.username");
    }
    if (!form.email || !form.email.includes("@")) {
      newErrors.email = t("auth.email");
    }
    if (!form.password || form.password.length < 6) {
      newErrors.password = t("auth.passwordTooShort");
    }
    if (form.password !== form.confirmPassword) {
      newErrors.confirmPassword = t("auth.passwordMismatch");
    }
    if (emailVerificationEnabled && !form.verificationCode) {
      newErrors.verificationCode = t("auth.verificationCodeRequired");
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const handleSendCode = async () => {
    if (!form.email || !form.email.includes("@")) {
      toast.error(t("auth.enterValidEmail"));
      return;
    }
    setSendingCode(true);
    try {
      await window.Noteva.api.post('/auth/send-code', { email: form.email });
      toast.success(t("auth.codeSent"));
      setCountdown(60);
    } catch (error: any) {
      toast.error(error.data?.error || t("auth.sendCodeFailed"));
    } finally {
      setSendingCode(false);
    }
  };

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;
    
    try {
      const registerData: any = {
        username: form.username,
        email: form.email,
        password: form.password,
      };
      if (emailVerificationEnabled) {
        registerData.verification_code = form.verificationCode;
      }
      await registerUser(form.username, form.email, form.password, form.verificationCode || undefined);
      toast.success(t("auth.registerSuccess"));
      router.push("/login");
    } catch (error: any) {
      // SDK 错误格式: error.data?.error 或 error.message
      toast.error(error.data?.error || error.message || t("auth.registerFailed"));
    }
  };

  return (
    <div className="flex min-h-screen items-center justify-center bg-muted/50 p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="space-y-1">
          <CardTitle className="text-2xl font-bold text-center">{settings.site_name}</CardTitle>
          <CardDescription className="text-center">
            {t("auth.createAccount")}
          </CardDescription>
        </CardHeader>
        <form onSubmit={onSubmit}>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="username">{t("auth.username")}</Label>
              <Input
                id="username"
                placeholder="your_username"
                value={form.username}
                onChange={(e) => setForm({ ...form, username: e.target.value })}
              />
              {errors.username && (
                <p className="text-sm text-destructive">{errors.username}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="email">{t("auth.email")}</Label>
              <Input
                id="email"
                type="email"
                placeholder="your@email.com"
                value={form.email}
                onChange={(e) => setForm({ ...form, email: e.target.value })}
              />
              {errors.email && (
                <p className="text-sm text-destructive">{errors.email}</p>
              )}
            </div>
            {emailVerificationEnabled && (
              <div className="space-y-2">
                <Label htmlFor="verificationCode">{t("auth.verificationCode")}</Label>
                <div className="flex gap-2">
                  <Input
                    id="verificationCode"
                    placeholder="123456"
                    value={form.verificationCode}
                    onChange={(e) => setForm({ ...form, verificationCode: e.target.value })}
                    className="flex-1"
                  />
                  <Button
                    type="button"
                    variant="outline"
                    onClick={handleSendCode}
                    disabled={sendingCode || countdown > 0}
                  >
                    {countdown > 0 ? `${countdown}s` : sendingCode ? t("common.loading") : t("auth.sendCode")}
                  </Button>
                </div>
                {errors.verificationCode && (
                  <p className="text-sm text-destructive">{errors.verificationCode}</p>
                )}
              </div>
            )}
            <div className="space-y-2">
              <Label htmlFor="password">{t("auth.password")}</Label>
              <Input
                id="password"
                type="password"
                placeholder="••••••••"
                value={form.password}
                onChange={(e) => setForm({ ...form, password: e.target.value })}
              />
              {errors.password && (
                <p className="text-sm text-destructive">{errors.password}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="confirmPassword">{t("auth.confirmPassword")}</Label>
              <Input
                id="confirmPassword"
                type="password"
                placeholder="••••••••"
                value={form.confirmPassword}
                onChange={(e) => setForm({ ...form, confirmPassword: e.target.value })}
              />
              {errors.confirmPassword && (
                <p className="text-sm text-destructive">{errors.confirmPassword}</p>
              )}
            </div>
          </CardContent>
          <CardFooter className="flex flex-col space-y-4">
            <Button type="submit" className="w-full" disabled={isLoading}>
              {isLoading ? t("common.loading") : t("auth.register")}
            </Button>
            <p className="text-sm text-muted-foreground text-center">
              {t("auth.hasAccount")}{" "}
              <Link href="/login" className="text-primary hover:underline">
                {t("auth.login")}
              </Link>
            </p>
          </CardFooter>
        </form>
      </Card>
    </div>
  );
}
