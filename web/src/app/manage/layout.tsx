"use client";

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";
import { useAuthStore } from "@/lib/store/auth";
import { useSiteStore } from "@/lib/store/site";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { LanguageSwitcher } from "@/components/language-switcher";
import { ThemeSwitcher } from "@/components/theme-switcher";
import {
  LayoutDashboard,
  FileText,
  FolderTree,
  Tags,
  Settings,
  LogOut,
  Menu,
  X,
  Home,
  Palette,
  FileCode,
  Navigation,
  Puzzle,
} from "lucide-react";

export default function AdminLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  const router = useRouter();
  const pathname = usePathname();
  const { user, isAuthenticated, checkAuth, logout } = useAuthStore();
  const { settings, fetchSettings } = useSiteStore();
  const { t } = useTranslation();
  const [sidebarOpen, setSidebarOpen] = useState(false);
  const [authChecked, setAuthChecked] = useState(false);

  const navItems = [
    { href: "/manage", label: t("manage.dashboard"), icon: LayoutDashboard },
    { href: "/manage/articles", label: t("manage.articles"), icon: FileText },
    { href: "/manage/categories", label: t("manage.categories"), icon: FolderTree },
    { href: "/manage/tags", label: t("manage.tags"), icon: Tags },
    { href: "/manage/pages", label: t("manage.pages"), icon: FileCode },
    { href: "/manage/nav", label: t("manage.nav"), icon: Navigation },
    { href: "/manage/plugins", label: t("manage.plugins"), icon: Puzzle },
    { href: "/manage/themes", label: t("manage.themes"), icon: Palette },
    { href: "/manage/settings", label: t("manage.settings"), icon: Settings },
  ];

  // Check if current page is login or register (handle trailing slash)
  const isAuthPage = false; // Login/register moved to frontend

  // 获取站点设置
  useEffect(() => {
    fetchSettings();
  }, [fetchSettings]);

  // 更新页面标题
  useEffect(() => {
    document.title = `${settings.site_name} - 管理后台`;
  }, [settings.site_name]);

  useEffect(() => {
    // Skip auth check for login/register pages
    if (isAuthPage) {
      setAuthChecked(true);
      return;
    }
    checkAuth().finally(() => setAuthChecked(true));
  }, [checkAuth, isAuthPage]);

  useEffect(() => {
    if (!authChecked || isAuthPage) return;
    if (!isAuthenticated) {
      // Redirect to frontend login page (different origin in production)
      window.location.href = "/login";
    }
  }, [isAuthenticated, isAuthPage, authChecked]);

  const handleLogout = async () => {
    await logout();
    window.location.href = "/";
  };

  // For login/register pages, render without sidebar immediately
  if (isAuthPage) {
    return <>{children}</>;
  }

  // Show loading while checking auth (only on initial load)
  if (!authChecked) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  // Redirect if not authenticated (after auth check is complete)
  if (!isAuthenticated) {
    return (
      <div className="flex h-screen items-center justify-center">
        <div className="animate-spin rounded-full h-8 w-8 border-b-2 border-primary"></div>
      </div>
    );
  }

  return (
    <div className="flex h-screen bg-muted/30">
      {/* Mobile sidebar backdrop */}
      {sidebarOpen && (
        <div
          className="fixed inset-0 z-40 bg-black/50 lg:hidden"
          onClick={() => setSidebarOpen(false)}
        />
      )}

      {/* Sidebar */}
      <aside
        className={cn(
          "fixed inset-y-0 left-0 z-50 w-64 bg-card border-r transform transition-transform duration-200 ease-in-out lg:translate-x-0 lg:static",
          sidebarOpen ? "translate-x-0" : "-translate-x-full"
        )}
      >
        <div className="flex h-full flex-col">
          {/* Logo */}
          <div className="flex h-16 items-center justify-between px-6 border-b">
            <Link href="/manage" className="flex items-center gap-2">
              {settings.site_logo && (
                <Image
                  src={settings.site_logo}
                  alt={settings.site_name}
                  width={32}
                  height={32}
                  className="rounded"
                />
              )}
              <span className="text-xl font-bold">{settings.site_name}</span>
            </Link>
            <button
              className="lg:hidden"
              onClick={() => setSidebarOpen(false)}
            >
              <X className="h-5 w-5" />
            </button>
          </div>

          {/* Navigation */}
          <nav className="flex-1 space-y-1 p-4">
            {navItems.map((item) => {
              const isActive = pathname === item.href || 
                (item.href !== "/manage" && pathname.startsWith(item.href));
              return (
                <Link
                  key={item.href}
                  href={item.href}
                  className={cn(
                    "flex items-center gap-3 rounded-lg px-3 py-2 text-sm transition-colors",
                    isActive
                      ? "bg-primary text-primary-foreground"
                      : "text-muted-foreground hover:bg-muted hover:text-foreground"
                  )}
                  onClick={() => setSidebarOpen(false)}
                >
                  <item.icon className="h-4 w-4" />
                  {item.label}
                </Link>
              );
            })}
          </nav>

          {/* User info */}
          <div className="border-t p-4">
            <div className="flex items-center gap-3 mb-3">
              <div className="h-8 w-8 rounded-full bg-primary/10 flex items-center justify-center">
                <span className="text-sm font-medium">
                  {user?.username?.[0]?.toUpperCase()}
                </span>
              </div>
              <div className="flex-1 min-w-0">
                <p className="text-sm font-medium truncate">{user?.username}</p>
                <p className="text-xs text-muted-foreground truncate">
                  {user?.email}
                </p>
              </div>
            </div>
            <Button
              variant="outline"
              size="sm"
              className="w-full"
              onClick={handleLogout}
            >
              <LogOut className="h-4 w-4 mr-2" />
              {t("nav.logout")}
            </Button>
          </div>
        </div>
      </aside>

      {/* Main content */}
      <div className="flex-1 flex flex-col min-w-0">
        {/* Top header bar */}
        <header className="flex h-16 items-center justify-between gap-4 border-b bg-card px-6">
          <div className="flex items-center gap-4">
            <button className="lg:hidden" onClick={() => setSidebarOpen(true)}>
              <Menu className="h-5 w-5" />
            </button>
            <span className="font-semibold lg:hidden">Noteva</span>
          </div>
          <div className="flex items-center gap-2">
            <Button variant="ghost" size="icon" asChild title={t("nav.home")}>
              <a href="/">
                <Home className="h-5 w-5" />
              </a>
            </Button>
            <LanguageSwitcher />
            <ThemeSwitcher />
          </div>
        </header>

        {/* Page content */}
        <main className="flex-1 overflow-auto p-6">{children}</main>
      </div>
    </div>
  );
}
