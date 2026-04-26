import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import {
  AlertCircle,
  CheckCircle2,
  ExternalLink,
  Loader2,
  RefreshCw,
  RotateCw,
} from "lucide-react";
import packageInfo from "../../../package.json";
import { adminApi, type UpdateCheckResponse } from "@/lib/api";
import { getApiErrorMessage } from "@/lib/api-error";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { toast } from "sonner";

const CACHE_KEY = "noteva-update-check";
const CACHE_TTL_MS = 12 * 60 * 60 * 1000;
const RELEASES_URL = "https://github.com/noteva26/Noteva/releases";

interface CachedUpdateCheck {
  checkedAt: number;
  info: UpdateCheckResponse;
}

function readCachedUpdate(): CachedUpdateCheck | null {
  try {
    const raw = localStorage.getItem(CACHE_KEY);
    if (!raw) return null;
    const parsed = JSON.parse(raw) as Partial<CachedUpdateCheck>;
    if (!parsed.checkedAt || !parsed.info) return null;
    return parsed as CachedUpdateCheck;
  } catch {
    return null;
  }
}

function writeCachedUpdate(info: UpdateCheckResponse) {
  if (info.error) return;
  localStorage.setItem(
    CACHE_KEY,
    JSON.stringify({
      checkedAt: Date.now(),
      info,
    } satisfies CachedUpdateCheck)
  );
}

function normalizeVersion(version: string | null | undefined) {
  if (!version) return "";
  return version.startsWith("v") ? version : `v${version}`;
}

export function VersionStatus() {
  const { t } = useTranslation();
  const cached = useMemo(readCachedUpdate, []);
  const [open, setOpen] = useState(false);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [checking, setChecking] = useState(false);
  const [performingUpdate, setPerformingUpdate] = useState(false);
  const [updateRestarting, setUpdateRestarting] = useState(false);
  const [updateInfo, setUpdateInfo] = useState<UpdateCheckResponse | null>(
    () => cached?.info ?? null
  );
  const [checkedAt, setCheckedAt] = useState<number | null>(
    () => cached?.checkedAt ?? null
  );
  const autoCheckedRef = useRef(false);

  const currentVersion = normalizeVersion(updateInfo?.current_version || packageInfo.version);
  const latestVersion = normalizeVersion(updateInfo?.latest_version);
  const hasUpdate = updateInfo?.update_available === true;
  const hasError = Boolean(updateInfo?.error);
  const releaseUrl = updateInfo?.release_url || RELEASES_URL;

  const status = useMemo(() => {
    if (updateRestarting) {
      return {
        label: t("version.restarting"),
        className: "text-amber-600 dark:text-amber-400",
        dotClassName: "bg-amber-500",
        icon: Loader2,
        spinning: true,
      };
    }
    if (checking) {
      return {
        label: t("version.checking"),
        className: "text-blue-600 dark:text-blue-400",
        dotClassName: "bg-blue-500",
        icon: Loader2,
        spinning: true,
      };
    }
    if (hasUpdate) {
      return {
        label: t("version.updateAvailable"),
        className: "text-primary",
        dotClassName: "bg-primary",
        icon: RotateCw,
        spinning: false,
      };
    }
    if (hasError) {
      return {
        label: t("version.checkFailed"),
        className: "text-destructive",
        dotClassName: "bg-destructive",
        icon: AlertCircle,
        spinning: false,
      };
    }
    if (updateInfo) {
      return {
        label: t("version.upToDate"),
        className: "text-green-600 dark:text-green-400",
        dotClassName: "bg-green-500",
        icon: CheckCircle2,
        spinning: false,
      };
    }
    return {
      label: t("version.notChecked"),
      className: "text-muted-foreground",
      dotClassName: "bg-muted-foreground/50",
      icon: RefreshCw,
      spinning: false,
    };
  }, [checking, hasError, hasUpdate, t, updateInfo, updateRestarting]);

  const StatusIcon = status.icon;

  const checkedAtLabel = useMemo(() => {
    if (!checkedAt) return null;
    const time = new Date(checkedAt).toLocaleTimeString(undefined, {
      hour: "2-digit",
      minute: "2-digit",
    });
    return t("version.checkedAt", { time });
  }, [checkedAt, t]);

  const checkUpdate = useCallback(
    async (silent = false) => {
      setChecking(true);
      try {
        const { data } = await adminApi.checkUpdate();
        const now = Date.now();
        setUpdateInfo(data);
        setCheckedAt(now);
        writeCachedUpdate(data);

        if (!silent) {
          if (data.error) {
            toast.error(data.error);
          } else if (data.update_available) {
            toast.success(t("settings.updateAvailable"));
          } else {
            toast.info(t("settings.noUpdate"));
          }
        }
      } catch (error) {
        if (!silent) {
          toast.error(getApiErrorMessage(error, t("settings.checkUpdateFailed")));
        }
        setUpdateInfo((current) =>
          current
            ? {
                ...current,
                error: getApiErrorMessage(error, t("settings.checkUpdateFailed")),
              }
            : {
                current_version: packageInfo.version,
                latest_version: null,
                update_available: false,
                release_url: null,
                release_notes: null,
                release_date: null,
                error: getApiErrorMessage(error, t("settings.checkUpdateFailed")),
              }
        );
      } finally {
        setChecking(false);
      }
    },
    [t]
  );

  useEffect(() => {
    if (autoCheckedRef.current) return;
    autoCheckedRef.current = true;

    const cachedUpdate = readCachedUpdate();
    if (cachedUpdate && Date.now() - cachedUpdate.checkedAt < CACHE_TTL_MS) {
      return;
    }

    const timer = window.setTimeout(() => {
      void checkUpdate(true);
    }, 1800);

    return () => window.clearTimeout(timer);
  }, [checkUpdate]);

  useEffect(() => {
    if (!updateRestarting) return;

    let cancelled = false;
    const poll = async () => {
      await new Promise((resolve) => window.setTimeout(resolve, 5000));
      while (!cancelled) {
        try {
          await fetch("/api/v1/site/info", { cache: "no-store" });
          if (!cancelled) {
            toast.success(t("settings.updateRestartDone"));
            await new Promise((resolve) => window.setTimeout(resolve, 1500));
            window.location.reload();
          }
          return;
        } catch {
          await new Promise((resolve) => window.setTimeout(resolve, 3000));
        }
      }
    };

    void poll();

    return () => {
      cancelled = true;
    };
  }, [t, updateRestarting]);

  const handleOpenChange = useCallback(
    (nextOpen: boolean) => {
      setOpen(nextOpen);
      if (nextOpen && !updateInfo && !checking) {
        void checkUpdate(true);
      }
    },
    [checkUpdate, checking, updateInfo]
  );

  const handleConfirmUpdate = useCallback(async () => {
    if (!updateInfo?.latest_version) return;

    setConfirmOpen(false);
    setOpen(false);
    setPerformingUpdate(true);
    try {
      await adminApi.performUpdate(updateInfo.latest_version);
      toast.success(t("settings.updateSuccess"));
      setUpdateRestarting(true);
    } catch (error) {
      toast.error(getApiErrorMessage(error, t("settings.updateFailed")));
    } finally {
      setPerformingUpdate(false);
    }
  }, [t, updateInfo?.latest_version]);

  return (
    <>
      <DropdownMenu open={open} onOpenChange={handleOpenChange}>
        <DropdownMenuTrigger asChild>
          <button
            type="button"
            className={cn(
              "flex w-full items-center justify-between rounded-lg border border-transparent px-2.5 py-2 text-left text-xs transition-colors",
              "text-muted-foreground hover:border-border hover:bg-muted/70 hover:text-foreground",
              hasUpdate && "border-primary/20 bg-primary/5 text-primary"
            )}
            title={t("version.openMenu")}
          >
            <span className="flex min-w-0 items-center gap-2">
              <span className={cn("h-2 w-2 rounded-full", status.dotClassName)} />
              <span className="font-mono font-medium">{currentVersion}</span>
            </span>
            <span className={cn("truncate pl-2", status.className)}>{status.label}</span>
          </button>
        </DropdownMenuTrigger>
        <DropdownMenuContent side="top" align="start" sideOffset={8} className="w-64 p-0">
          <div className="flex items-center justify-between border-b px-3 py-2.5">
            <div>
              <p className="text-sm font-medium">{t("version.title")}</p>
              {checkedAtLabel && (
                <p className="mt-0.5 text-xs text-muted-foreground">{checkedAtLabel}</p>
              )}
            </div>
            <Button
              variant="ghost"
              size="icon"
              className="h-7 w-7"
              onClick={() => void checkUpdate(false)}
              disabled={checking || performingUpdate || updateRestarting}
              title={t("version.checkNow")}
            >
              {checking ? (
                <Loader2 className="h-3.5 w-3.5 animate-spin" />
              ) : (
                <RefreshCw className="h-3.5 w-3.5" />
              )}
            </Button>
          </div>

          <div className="space-y-3 p-3">
            <div className="grid grid-cols-[auto_1fr] gap-x-3 gap-y-1.5 text-xs">
              <span className="text-muted-foreground">{t("version.current")}</span>
              <span className="text-right font-mono font-medium">{currentVersion}</span>
              {latestVersion && (
                <>
                  <span className="text-muted-foreground">{t("version.latest")}</span>
                  <span className="text-right font-mono font-medium">{latestVersion}</span>
                </>
              )}
            </div>

            <div
              className={cn(
                "flex items-start gap-2 rounded-lg border px-3 py-2 text-xs",
                hasUpdate && "border-primary/25 bg-primary/5",
                hasError && "border-destructive/25 bg-destructive/5",
                updateInfo && !hasUpdate && !hasError && "border-green-500/25 bg-green-500/5"
              )}
            >
              <StatusIcon
                className={cn("mt-0.5 h-4 w-4 shrink-0", status.className, status.spinning && "animate-spin")}
              />
              <div className="min-w-0 space-y-0.5">
                <p className={cn("font-medium", status.className)}>{status.label}</p>
                {hasError && updateInfo?.error ? (
                  <p className="line-clamp-2 text-muted-foreground">{updateInfo.error}</p>
                ) : (
                  <p className="text-muted-foreground">
                    {hasUpdate ? t("version.updateHint") : t("version.stableHint")}
                  </p>
                )}
              </div>
            </div>

            <div className="flex items-center gap-2">
              {hasUpdate && updateInfo?.latest_version && (
                <Button
                  size="sm"
                  className="h-8 flex-1"
                  onClick={() => setConfirmOpen(true)}
                  disabled={performingUpdate || updateRestarting}
                >
                  {performingUpdate ? (
                    <Loader2 className="mr-1.5 h-3.5 w-3.5 animate-spin" />
                  ) : (
                    <RotateCw className="mr-1.5 h-3.5 w-3.5" />
                  )}
                  {performingUpdate ? t("settings.updating") : t("settings.performUpdate")}
                </Button>
              )}
              <Button variant="outline" size="sm" className="h-8 flex-1" asChild>
                <a href={releaseUrl} target="_blank" rel="noopener noreferrer">
                  <ExternalLink className="mr-1.5 h-3.5 w-3.5" />
                  {t("version.viewRelease")}
                </a>
              </Button>
            </div>
          </div>
        </DropdownMenuContent>
      </DropdownMenu>

      <AlertDialog open={confirmOpen} onOpenChange={setConfirmOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle className="flex items-center gap-2">
              <RotateCw className="h-5 w-5 text-primary" />
              {t("version.confirmTitle")}
            </AlertDialogTitle>
            <AlertDialogDescription>
              {t("version.confirmDescription", {
                version: updateInfo?.latest_version || "",
              })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel disabled={performingUpdate}>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={handleConfirmUpdate} disabled={performingUpdate}>
              {performingUpdate ? (
                <Loader2 className="mr-2 h-4 w-4 animate-spin" />
              ) : (
                <RotateCw className="mr-2 h-4 w-4" />
              )}
              {t("settings.performUpdate")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
