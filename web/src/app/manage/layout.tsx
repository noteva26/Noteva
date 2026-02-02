"use client";

import { useEffect, useState, Suspense } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import Image from "next/image";
import { usePathname } from "next/navigation";
import { motion } from "motion/react";
import { useAuthStore } from "@/lib/store/auth";
import { useSiteStore } from "@/lib/store/site";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";
import { Button } from "@/components/ui/button";
import { LanguageSwitcher } from "@/components/language-switcher";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { TopLoader } from "@/components/ui/top-loader";
import { authApi } from "@/lib/api";
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
  MessageSquare,
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
  const [mounted, setMounted] = useState(false);

  // Set mounted on client side
  useEffect(() => {
    setMounted(true);
  }, []);

  const navItems = [
    { href: "/manage", label: t("manage.dashboard"), icon: LayoutDashboard },
    { href: "/manage/articles", label: t("manage.articles"), icon: FileText },
    { href: "/manage/categories", label: t("manage.categories"), icon: FolderTree },
    { href: "/manage/tags", label: t("manage.tags"), icon: Tags },
    { href: "/manage/pages", label: t("manage.pages"), icon: FileCode },
    { href: "/manage/nav", label: t("manage.nav"), icon: Navigation },
    { href: "/manage/comments", label: t("manage.comments"), icon: MessageSquare },
    { href: "/manage/plugins", label: t("manage.plugins"), icon: Puzzle },
    { href: "/manage/themes", label: t("manage.themes"), icon: Palette },
    { href: "/manage/settings", label: t("manage.settings"), icon: Settings },
  ];

  // Check if current page is login or setup (these don't need sidebar)
  // Handle both with and without trailing slash
  const normalizedPath = pathname?.replace(/\/$/, '') || '';
  const isAuthPage = normalizedPath === "/manage/login" || 
                     normalizedPath === "/manage/setup" ||
                     normalizedPath.endsWith("/login") ||
                     normalizedPath.endsWith("/setup");

  // 获取站点设置
  useEffect(() => {
    fetchSettings();
  }, [fetchSettings]);

  // 更新页面标题
  useEffect(() => {
    document.title = `${settings.site_name} - 管理后台`;
  }, [settings.site_name]);

  useEffect(() => {
    // Skip auth check for login/setup pages
    if (isAuthPage) {
      setAuthChecked(true);
      return;
    }
    
    const init = async () => {
      try {
        // First check if admin exists
        const { data } = await authApi.hasAdmin();
        if (!data.has_admin) {
          // No admin, redirect to setup
          router.replace("/manage/setup");
          return;
        }
        
        // Check auth
        await checkAuth();
        setAuthChecked(true);
      } catch (error) {
        console.error("Auth check failed:", error);
        setAuthChecked(true);
      }
    };
    
    init();
  }, [checkAuth, isAuthPage, router]);

  useEffect(() => {
    if (!authChecked || isAuthPage) return;
    if (!isAuthenticated) {
      router.replace("/manage/login");
      return;
    }
    // Check if user is admin
    if (user && user.role !== "admin") {
      alert("权限不足：只有管理员可以访问管理后台");
      window.location.href = "/";
    }
  }, [isAuthenticated, isAuthPage, authChecked, user, router]);

  const handleLogout = async () => {
    await logout();
    router.push("/manage/login");
  };

  // For login/setup pages, render without sidebar immediately
  // This check must come FIRST before any loading states
  if (isAuthPage) {
    return <>{children}</>;
  }

  // Before mount, show children to avoid hydration mismatch
  // This prevents flash of loading state on initial render
  if (!mounted) {
    return <>{children}</>;
  }

  // Show loading while checking auth (only after mount)
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
      {/* Top Loading Bar */}
      <Suspense fallback={null}>
        <TopLoader />
      </Suspense>

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

          {/* Navigation with hover animations */}
          <nav className="flex-1 space-y-1 p-4">
            {navItems.map((item) => {
              const isActive = pathname === item.href || 
                (item.href !== "/manage" && pathname.startsWith(item.href));
              return (
                <motion.div
                  key={item.href}
                  whileHover={{ x: isActive ? 0 : 4 }}
                  transition={{ type: "spring", stiffness: 400, damping: 25 }}
                >
                  <Link
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
                </motion.div>
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

        {/* Page content with route transition */}
        <main className="flex-1 overflow-auto p-6">
          {/* Demo mode banner */}
          {settings.demo_mode && (
            <div className="mb-4 rounded-lg bg-amber-500/10 border border-amber-500/20 px-4 py-3 text-amber-700 dark:text-amber-400">
              <div className="flex items-center gap-2">
                <svg className="h-5 w-5" fill="none" viewBox="0 0 24 24" stroke="currentColor">
                  <path strokeLinecap="round" strokeLinejoin="round" strokeWidth={2} d="M13 16h-1v-4h-1m1-4h.01M21 12a9 9 0 11-18 0 9 9 0 0118 0z" />
                </svg>
                <span className="font-medium">Demo Mode</span>
                <span className="text-sm opacity-80">- 这是演示站点，写操作已禁用</span>
              </div>
            </div>
          )}
          <motion.div
            key={pathname}
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{
              type: "spring",
              stiffness: 400,
              damping: 30,
            }}
          >
            {children}
          </motion.div>
        </main>
      </div>
    </div>
  );
}
