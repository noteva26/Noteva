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

export default function LoginPage() {
  const router = useRouter();
  const { login, isLoading } = useAuthStore();
  const [showPassword, setShowPassword] = useState(false);
  const [checking, setChecking] = useState(true);
  const { t } = useTranslation();
  const [form, setForm] = useState({ usernameOrEmail: "", password: "" });
  const [errors, setErrors] = useState<{ usernameOrEmail?: string; password?: string }>({});

  // Check if already logged in via SDK
  useEffect(() => {
    useAuthStore.getState().checkAuth().then(() => {
      if (useAuthStore.getState().isAuthenticated) {
        router.replace("/");
      } else {
        setChecking(false);
      }
    }).catch(() => {
      setChecking(false);
    });
  }, [router]);

  const validate = () => {
    const newErrors: typeof errors = {};
    if (!form.usernameOrEmail) {
      newErrors.usernameOrEmail = t("auth.usernameOrEmail");
    }
    if (!form.password || form.password.length < 6) {
      newErrors.password = t("auth.passwordTooShort");
    }
    setErrors(newErrors);
    return Object.keys(newErrors).length === 0;
  };

  const onSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!validate()) return;
    
    try {
      await login(form.usernameOrEmail, form.password);
      toast.success(t("auth.loginSuccess"));
      router.replace("/");
    } catch (error: any) {
      // SDK 错误格式: error.data?.error 或 error.message
      toast.error(error.data?.error || error.message || t("auth.loginFailed"));
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
    <div className="flex min-h-screen items-center justify-center bg-muted/50 p-4">
      <Card className="w-full max-w-md">
        <CardHeader className="space-y-1">
          <CardTitle className="text-2xl font-bold text-center">Noteva</CardTitle>
          <CardDescription className="text-center">
            {t("auth.loginToAccount")}
          </CardDescription>
        </CardHeader>
        <form onSubmit={onSubmit}>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="usernameOrEmail">{t("auth.usernameOrEmail")}</Label>
              <Input
                id="usernameOrEmail"
                type="text"
                placeholder={t("auth.usernameOrEmail")}
                value={form.usernameOrEmail}
                onChange={(e) => setForm({ ...form, usernameOrEmail: e.target.value })}
              />
              {errors.usernameOrEmail && (
                <p className="text-sm text-destructive">{errors.usernameOrEmail}</p>
              )}
            </div>
            <div className="space-y-2">
              <Label htmlFor="password">{t("auth.password")}</Label>
              <div className="relative">
                <Input
                  id="password"
                  type={showPassword ? "text" : "password"}
                  placeholder="••••••••"
                  value={form.password}
                  onChange={(e) => setForm({ ...form, password: e.target.value })}
                />
                <button
                  type="button"
                  className="absolute right-3 top-1/2 -translate-y-1/2 text-muted-foreground hover:text-foreground text-xs"
                  onClick={() => setShowPassword(!showPassword)}
                >
                  {showPassword ? t("common.hide") : t("common.show")}
                </button>
              </div>
              {errors.password && (
                <p className="text-sm text-destructive">{errors.password}</p>
              )}
            </div>
          </CardContent>
          <CardFooter className="flex flex-col space-y-4">
            <Button type="submit" className="w-full" disabled={isLoading}>
              {isLoading ? t("common.loading") : t("auth.login")}
            </Button>
            <p className="text-sm text-muted-foreground text-center">
              {t("auth.noAccount")}{" "}
              <Link href="/register" className="text-primary hover:underline">
                {t("auth.register")}
              </Link>
            </p>
          </CardFooter>
        </form>
      </Card>
    </div>
  );
}
