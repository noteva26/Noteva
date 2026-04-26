import { startTransition, useActionState, useEffect, useState } from "react";
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

type LoginActionState =
  | { step: "credentials" }
  | { step: "twoFactor"; challengeToken: string };

const INITIAL_LOGIN_STATE: LoginActionState = { step: "credentials" };

interface ApiErrorBody {
  code?: string;
  message?: string;
  details?: unknown;
}

function getApiError(error: unknown): ApiErrorBody | null {
  if (typeof error === "object" && error !== null && "response" in error) {
    const response = (error as { response?: { data?: { error?: ApiErrorBody } } }).response;
    return response?.data?.error || null;
  }
  return null;
}

function isTwoFactorLoginError(error: unknown): error is { is2FA: true; challengeToken: string } {
  return (
    typeof error === "object" &&
    error !== null &&
    "is2FA" in error &&
    (error as { is2FA?: unknown }).is2FA === true &&
    typeof (error as { challengeToken?: unknown }).challengeToken === "string"
  );
}

function getRetryAfter(details: unknown) {
  if (typeof details !== "object" || details === null || !("retry_after" in details)) {
    return null;
  }
  const retryAfter = (details as { retry_after?: unknown }).retry_after;
  return typeof retryAfter === "number" ? retryAfter : null;
}

export default function LoginPage() {
  const navigate = useNavigate();
  const { t } = useTranslation();
  const { login, checkAuth } = useAuthStore();
  const [mounted, setMounted] = useState(false);
  const [loading, setLoading] = useState(true);
  const [formData, setFormData] = useState({
    usernameOrEmail: "",
    password: "",
  });
  const [totpCode, setTotpCode] = useState("");

  useEffect(() => {
    setMounted(true);
  }, []);

  useEffect(() => {
    if (!mounted) return;
    let active = true;

    const init = async () => {
      try {
        const { data } = await authApi.hasAdmin();
        if (!active) return;

        if (!data.has_admin) {
          navigate("/manage/setup", { replace: true });
          return;
        }

        await checkAuth();
        if (!active) return;

        if (useAuthStore.getState().isAuthenticated) {
          navigate("/manage", { replace: true });
          return;
        }

        setLoading(false);
      } catch (error) {
        console.error("Init error:", error);
        if (active) setLoading(false);
      }
    };

    init();
    return () => {
      active = false;
    };
  }, [mounted, navigate, checkAuth]);

  const [loginState, submitLogin, isSubmitting] = useActionState<LoginActionState, FormData>(
    async (previousState, submittedForm) => {
      const intent = submittedForm.get("intent");

      if (intent === "reset") {
        setTotpCode("");
        return INITIAL_LOGIN_STATE;
      }

      if (intent === "verify") {
        const code = String(submittedForm.get("totpCode") || "").trim();
        if (previousState.step !== "twoFactor" || !previousState.challengeToken) {
          return INITIAL_LOGIN_STATE;
        }
        if (code.length < 6) {
          toast.error(t("settings.2faCodeRequired") || "Please enter the 6-digit code");
          return previousState;
        }

        try {
          const { data } = await authApi.verify2FA(previousState.challengeToken, code);
          useAuthStore.setState({ user: data.user, isAuthenticated: true });
          toast.success(t("auth.loginSuccess"));
          navigate("/manage");
          return previousState;
        } catch (error) {
          toast.error(getApiError(error)?.message || t("auth.loginFailed"));
          setTotpCode("");
          return previousState;
        }
      }

      const usernameOrEmail = String(submittedForm.get("usernameOrEmail") || "").trim();
      const password = String(submittedForm.get("password") || "");

      if (!usernameOrEmail || !password) {
        toast.error(t("auth.invalidCredentials"));
        return previousState;
      }

      try {
        await login(usernameOrEmail, password);
        toast.success(t("auth.loginSuccess"));
        navigate("/manage");
        return INITIAL_LOGIN_STATE;
      } catch (error) {
        if (isTwoFactorLoginError(error)) {
          return { step: "twoFactor", challengeToken: error.challengeToken };
        }

        const apiError = getApiError(error);
        const errorCode = apiError?.code;
        const retryAfter = getRetryAfter(apiError?.details);
        let message = apiError?.message || t("auth.loginFailed");

        if (errorCode === "RATE_LIMIT" && retryAfter) {
          const retryMinutes = Math.ceil(retryAfter / 60);
          if (retryMinutes > 1) {
            message = `${message} (${retryMinutes} ${t("auth.retryMinutes") || "min retry"})`;
          } else {
            message = `${message} (${retryAfter} ${t("auth.retrySeconds") || "sec retry"})`;
          }
        }

        toast.error(message);
        return previousState;
      }
    },
    INITIAL_LOGIN_STATE
  );

  const resetLoginStep = () => {
    const resetForm = new FormData();
    resetForm.set("intent", "reset");
    startTransition(() => {
      submitLogin(resetForm);
    });
  };

  if (loading || !mounted) {
    return (
      <div className="min-h-screen flex items-center justify-center bg-gradient-to-br from-background to-muted">
        <Loader2 className="h-8 w-8 animate-spin text-primary" />
      </div>
    );
  }

  if (loginState.step === "twoFactor") {
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
            <form action={submitLogin} className="space-y-4">
              <input type="hidden" name="intent" value="verify" />
              <div className="space-y-2">
                <Label htmlFor="totpCode">{t("settings.2faCode") || "Authenticator Code"}</Label>
                <Input
                  id="totpCode"
                  name="totpCode"
                  value={totpCode}
                  onChange={(e) => setTotpCode(e.target.value.replace(/\D/g, "").slice(0, 6))}
                  placeholder="000000"
                  maxLength={6}
                  className="font-mono text-center text-2xl tracking-[0.5em]"
                  disabled={isSubmitting}
                  autoFocus
                  autoComplete="one-time-code"
                />
              </div>

              <Button type="submit" className="w-full" disabled={isSubmitting || totpCode.length < 6}>
                {isSubmitting ? (
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
                onClick={resetLoginStep}
                disabled={isSubmitting}
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
          <form action={submitLogin} className="space-y-4">
            <input type="hidden" name="intent" value="login" />
            <div className="space-y-2">
              <Label htmlFor="usernameOrEmail">{t("auth.usernameOrEmail")}</Label>
              <Input
                id="usernameOrEmail"
                name="usernameOrEmail"
                placeholder={t("auth.usernameOrEmail")}
                value={formData.usernameOrEmail}
                onChange={(e) => setFormData({ ...formData, usernameOrEmail: e.target.value })}
                disabled={isSubmitting}
                autoComplete="username"
                autoFocus
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="password">{t("auth.password")}</Label>
              <Input
                id="password"
                name="password"
                type="password"
                placeholder={t("auth.password")}
                value={formData.password}
                onChange={(e) => setFormData({ ...formData, password: e.target.value })}
                disabled={isSubmitting}
                autoComplete="current-password"
              />
            </div>

            <Button type="submit" className="w-full" disabled={isSubmitting}>
              {isSubmitting ? (
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
