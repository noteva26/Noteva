import { useState, useEffect } from "react";
import { useNavigate } from "react-router-dom";
import { authApi } from "@/lib/api";
import { useAuthStore } from "@/lib/store/auth";
import { useTranslation } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { toast } from "sonner";
import { Loader2, LogIn, Shield } from "lucide-react";

export default function LoginPage() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { login, checkAuth } = useAuthStore();
  const [mounted, setMounted] = useState(false);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [formData, setFormData] = useState({
    usernameOrEmail: "",
    password: "",
  });

  // 2FA state
  const [needs2FA, setNeeds2FA] = useState(false);
  const [challengeToken, setChallengeToken] = useState("");
  const [totpCode, setTotpCode] = useState("");

  // Handle hydration
  useEffect(() => {
    setMounted(true);
  }, []);

  // Check auth status and admin existence
  useEffect(() => {
    if (!mounted) return;

    const init = async () => {
      try {
        // First check if admin exists
        const { data } = await authApi.hasAdmin();
        if (!data.has_admin) {
          // No admin, redirect to setup
          navigate("/manage/setup", { replace: true });
          return;
        }

        // Check if already logged in
        await checkAuth();
        if (useAuthStore.getState().isAuthenticated) {
          navigate("/manage", { replace: true });
          return;
        }

        setLoading(false);
      } catch (error) {
        console.error("Init error:", error);
        setLoading(false);
      }
    };
    init();
  }, [mounted, navigate, checkAuth]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();

    if (!formData.usernameOrEmail.trim() || !formData.password) {
      toast.error(t("auth.invalidCredentials"));
      return;
    }

    setSubmitting(true);
    try {
      await login(formData.usernameOrEmail.trim(), formData.password);
      toast.success(t("auth.loginSuccess"));
      navigate("/manage");
    } catch (error: any) {
      // Handle 2FA challenge
      if (error.is2FA) {
        setChallengeToken(error.challengeToken);
        setNeeds2FA(true);
        setSubmitting(false);
        return;
      }

      const errorCode = error.response?.data?.error?.code;
      const errorDetails = error.response?.data?.error?.details;
      let message = error.response?.data?.error?.message || t("auth.loginFailed");

      // Handle rate limit errors with retry time
      if (errorCode === "RATE_LIMIT" && errorDetails?.retry_after) {
        const retryMinutes = Math.ceil(errorDetails.retry_after / 60);
        if (retryMinutes > 1) {
          message = `${message}（${retryMinutes} ${t("auth.retryMinutes") || "min retry"}）`;
        } else {
          message = `${message}（${errorDetails.retry_after} ${t("auth.retrySeconds") || "sec retry"}）`;
        }
      }

      toast.error(message);
    } finally {
      setSubmitting(false);
    }
  };

  const handle2FAVerify = async (e: React.FormEvent) => {
    e.preventDefault();

    if (totpCode.length < 6) {
      toast.error(t("settings.2faCodeRequired") || "Please enter the 6-digit code");
      return;
    }

    setSubmitting(true);
    try {
      const { data } = await authApi.verify2FA(challengeToken, totpCode.trim());
      // 2FA verified, complete login
      useAuthStore.setState({ user: data.user, isAuthenticated: true });
      toast.success(t("auth.loginSuccess"));
      navigate("/manage");
    } catch (error: any) {
      const message = error.response?.data?.error?.message || t("auth.loginFailed");
      toast.error(message);
      setTotpCode("");
    } finally {
      setSubmitting(false);
    }
  };

  if (loading || !mounted) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background to-muted">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  // 2FA code input screen
  if (needs2FA) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background to-muted p-4">
        <Card className="w-full max-w-md shadow-lg">
          <CardHeader className="text-center space-y-2">
            <div className="mx-auto w-12 h-12 bg-primary/10 rounded-full flex items-center justify-center mb-2">
              <Shield className="h-6 w-6 text-primary" />
            </div>
            <CardTitle className="text-2xl">{t("auth.2faTitle") || "Two-Factor Authentication"}</CardTitle>
            <CardDescription>{t("auth.2faDescription") || "Enter the 6-digit code from your authenticator app"}</CardDescription>
          </CardHeader>
          <CardContent>
            <form onSubmit={handle2FAVerify} className="space-y-4">
              <div className="space-y-2">
                <Label htmlFor="totpCode">{t("settings.2faCode") || "Authenticator Code"}</Label>
                <Input
                  id="totpCode"
                  value={totpCode}
                  onChange={(e) => setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                  placeholder="000000"
                  maxLength={6}
                  className="font-mono text-center text-2xl tracking-[0.5em]"
                  disabled={submitting}
                  autoFocus
                  autoComplete="one-time-code"
                />
              </div>

              <Button type="submit" className="w-full" disabled={submitting || totpCode.length < 6}>
                {submitting ? (
                  <>
                    <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                    {t("common.loading")}
                  </>
                ) : (
                  t("auth.verify") || "Verify"
                )}
              </Button>

              <Button
                type="button"
                variant="ghost"
                className="w-full"
                onClick={() => {
                  setNeeds2FA(false);
                  setChallengeToken("");
                  setTotpCode("");
                }}
              >
                {t("auth.backToLogin") || "Back to login"}
              </Button>
            </form>
          </CardContent>
        </Card>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background to-muted p-4">
      <Card className="w-full max-w-md shadow-lg">
        <CardHeader className="text-center space-y-2">
          <div className="mx-auto w-12 h-12 bg-primary/10 rounded-full flex items-center justify-center mb-2">
            <LogIn className="h-6 w-6 text-primary" />
          </div>
          <CardTitle className="text-2xl">{t("auth.login")}</CardTitle>
          <CardDescription>{t("auth.loginToAccount")}</CardDescription>
        </CardHeader>
        <CardContent>
          <form onSubmit={handleSubmit} className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="usernameOrEmail">{t("auth.usernameOrEmail")}</Label>
              <Input
                id="usernameOrEmail"
                placeholder={t("auth.usernameOrEmail")}
                value={formData.usernameOrEmail}
                onChange={(e) => setFormData({ ...formData, usernameOrEmail: e.target.value })}
                disabled={submitting}
                autoComplete="username"
                autoFocus
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">{t("auth.password")}</Label>
              <Input
                id="password"
                type="password"
                placeholder={t("auth.password")}
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                disabled={submitting}
                autoComplete="current-password"
              />
            </div>

            <Button type="submit" className="w-full" disabled={submitting}>
              {submitting ? (
                <>
                  <Loader2 className="mr-2 h-4 w-4 animate-spin" />
                  {t("common.loading")}
                </>
              ) : (
                t("auth.login")
              )}
            </Button>
          </form>
        </CardContent>
      </Card>
    </div>
  );
}
