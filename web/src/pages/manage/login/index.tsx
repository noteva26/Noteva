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
import { Loader2, LogIn } from "lucide-react";

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
      const errorCode = error.response?.data?.error?.code;
      const errorDetails = error.response?.data?.error?.details;
      let message = error.response?.data?.error?.message || t("auth.loginFailed");
      
      // Handle rate limit errors with retry time
      if (errorCode === "RATE_LIMIT" && errorDetails?.retry_after) {
        const retryMinutes = Math.ceil(errorDetails.retry_after / 60);
        if (retryMinutes > 1) {
          message = `${message}（${retryMinutes} 分钟后重试）`;
        } else {
          message = `${message}（${errorDetails.retry_after} 秒后重试）`;
        }
      }
      
      toast.error(message);
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
