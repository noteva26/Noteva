import { useActionState, useEffect, useState } from "react";
import { useNavigate } from "react-router-dom";
import { authApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";
import { Loader2, Sparkles } from "lucide-react";
import { useTranslation } from "@/lib/i18n";

function getApiError(error: unknown) {
  if (typeof error === "object" && error !== null && "response" in error) {
    const response = (error as { response?: { data?: { error?: { code?: string; message?: string } } } }).response;
    return response?.data?.error || null;
  }
  return null;
}

export default function SetupPage() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const [mounted, setMounted] = useState(false);
  const [formData, setFormData] = useState({
    username: "",
    email: "",
    password: "",
    confirmPassword: "",
  });

  useEffect(() => {
    setMounted(true);
  }, []);

  const [, submitSetup, isSubmitting] = useActionState<null, FormData>(
    async () => {
      const username = formData.username.trim();
      const email = formData.email.trim();

      if (!username) {
        toast.error(t("auth.username") + " " + t("common.error"));
        return null;
      }
      if (!email || !email.includes("@")) {
        toast.error(t("auth.email") + " " + t("common.error"));
        return null;
      }
      if (formData.password.length < 8) {
        toast.error(t("auth.passwordTooShort"));
        return null;
      }
      if (formData.password !== formData.confirmPassword) {
        toast.error(t("auth.passwordMismatch"));
        return null;
      }

      try {
        await authApi.register(username, email, formData.password);
        toast.success(t("setup.success"));
        navigate("/manage");
      } catch (error) {
        const apiError = getApiError(error);
        const errorCode = apiError?.code;
        const message = apiError?.message || t("setup.error");

        if (errorCode === "FORBIDDEN") {
          toast.error(message);
          window.setTimeout(() => {
            navigate("/manage/login");
          }, 2000);
          return null;
        }

        toast.error(message);
      }

      return null;
    },
    null
  );

  if (!mounted) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background to-muted">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background to-muted p-4">
      <Card className="w-full max-w-md shadow-lg">
        <CardHeader className="text-center space-y-2">
          <div className="mx-auto w-12 h-12 bg-primary/10 rounded-full flex items-center justify-center mb-2">
            <Sparkles className="h-6 w-6 text-primary" />
          </div>
          <CardTitle className="text-2xl">{t("setup.welcome")}</CardTitle>
          <CardDescription>{t("setup.description")}</CardDescription>
        </CardHeader>
        <CardContent>
          <form action={submitSetup} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="username">{t("setup.username")}</Label>
              <Input
                id="username"
                name="username"
                placeholder={t("setup.usernamePlaceholder")}
                value={formData.username}
                onChange={(e) => setFormData({ ...formData, username: e.target.value })}
                disabled={isSubmitting}
                autoComplete="username"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="email">{t("auth.email")}</Label>
              <Input
                id="email"
                name="email"
                type="email"
                placeholder="admin@example.com"
                value={formData.email}
                onChange={(e) => setFormData({ ...formData, email: e.target.value })}
                disabled={isSubmitting}
                autoComplete="email"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">{t("setup.password")}</Label>
              <Input
                id="password"
                name="password"
                type="password"
                placeholder={t("setup.passwordPlaceholder")}
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                disabled={isSubmitting}
                autoComplete="new-password"
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="confirmPassword">{t("setup.confirmPassword")}</Label>
              <Input
                id="confirmPassword"
                name="confirmPassword"
                type="password"
                placeholder={t("setup.confirmPasswordPlaceholder")}
                value={formData.confirmPassword}
                onChange={(e) => setFormData({ ...formData, confirmPassword: e.target.value })}
                disabled={isSubmitting}
                autoComplete="new-password"
              />
            </div>

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("setup.creating")}
                </>
              ) : (
                t("setup.submit")
              )}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
