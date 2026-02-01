"use client";

import Link from "next/link";
import Image from "next/image";
import { useRouter } from "next/navigation";
import { motion, AnimatePresence } from "motion/react";
import { useTranslation } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { LanguageSwitcher } from "@/components/language-switcher";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { TopLoader } from "@/components/ui/top-loader";
import { Settings, LogOut, Menu, X, Search, ChevronDown } from "lucide-react";
import { useEffect, useState, useMemo, Suspense } from "react";
import { getNoteva } from "@/hooks/useNoteva";

// 导航项类型（兼容 SDK 和后端返回格式）
interface NavItem {
  id: number;
  parent_id?: number | null;
  title?: string;
  name?: string;  // SDK 返回 name
  nav_type?: string;
  target?: string;
  url?: string;   // SDK 返回 url
  open_new_tab?: boolean;
  sort_order?: number;
  order?: number; // SDK 返回 order
  visible?: boolean;
  children?: NavItem[];
}

const BUILTIN_PATHS: Record<string, string> = {
  home: "/",
  archives: "/archives",
  categories: "/categories",
  tags: "/tags",
};

export function SiteHeader() {
  const { t } = useTranslation();
  const router = useRouter();
  const [mounted, setMounted] = useState(false);
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [navItems, setNavItems] = useState<NavItem[]>([]);
  // 初始值为 null，避免 SSR 时使用默认值导致闪烁
  const [siteInfo, setSiteInfo] = useState<{ name: string; logo: string } | null>(null);
  const [user, setUser] = useState<any>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [authChecked, setAuthChecked] = useState(false);

  // 客户端挂载后立即从注入的配置读取，避免闪烁
  useEffect(() => {
    setMounted(true);
    // 立即从 __SITE_CONFIG__ 读取（此时 script 已执行）
    const config = (window as any).__SITE_CONFIG__;
    if (config) {
      setSiteInfo({
        name: config.site_name || "Noteva",
        logo: config.site_logo || "/logo.png",
      });
    } else {
      // 没有注入配置时使用默认值
      setSiteInfo({ name: "Noteva", logo: "/logo.png" });
    }
  }, []);

  useEffect(() => {
    if (!mounted) return;

    const loadData = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        setTimeout(loadData, 50);
        return;
      }

      try {
        // 加载站点信息（如果注入的配置不完整，从 API 补充）
        const info = await Noteva.site.getInfo();
        setSiteInfo({
          name: info.name || "Noteva",
          logo: info.logo || "/logo.png",
        });

        // 加载导航
        const nav = await Noteva.site.getNav();
        
        // 递归转换导航项
        const convertNavItem = (item: any): NavItem => ({
          id: item.id,
          parent_id: item.parent_id ?? null,
          title: item.title || item.name,
          name: item.name || item.title,
          nav_type: item.nav_type,
          target: item.target || item.url,
          url: item.url || item.target,
          open_new_tab: item.open_new_tab ?? (item.target === "_blank"),
          sort_order: item.sort_order ?? item.order ?? 0,
          order: item.order ?? item.sort_order,
          visible: item.visible ?? true,
          children: item.children?.map(convertNavItem),
        });
        
        setNavItems((nav || []).map(convertNavItem));

        // 检查用户登录状态
        const currentUser = await Noteva.user.check();
        setUser(currentUser);
        setIsAuthenticated(!!currentUser);
        setAuthChecked(true);
      } catch (err) {
        console.error(err);
        setAuthChecked(true);
      }
    };

    loadData();
  }, [mounted]);

  const handleLogout = async () => {
    const Noteva = getNoteva();
    if (!Noteva) return;

    try {
      await Noteva.user.logout();
      setUser(null);
      setIsAuthenticated(false);
    } catch {}
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) {
      router.push(`/?q=${encodeURIComponent(searchQuery.trim())}`);
    }
  };

  const getNavHref = (item: NavItem): string | null => {
    // 获取目标 URL（兼容 target 和 url 字段）
    const targetUrl = item.target || item.url || "";
    
    // Group items (builtin with empty target) have no link
    if (item.nav_type === "builtin" && !targetUrl) {
      return null;
    }
    switch (item.nav_type) {
      case "builtin":
        return BUILTIN_PATHS[targetUrl] || "/";
      case "page":
        return `/${targetUrl}`;
      case "external":
        return targetUrl;
      default:
        // 如果没有 nav_type，直接使用 url
        return targetUrl || "/";
    }
  };

  const renderNavLink = (item: NavItem) => {
    const href = getNavHref(item);
    
    // Group items without href - just show text (shouldn't happen for leaf items)
    if (!href) {
      return (
        <span
          key={item.id}
          className="transition-colors text-foreground/60"
        >
          {item.title}
        </span>
      );
    }
    
    const isExternal = item.nav_type === "external";

    if (isExternal) {
      return (
        <a
          key={item.id}
          href={href}
          target={item.open_new_tab ? "_blank" : "_self"}
          rel={item.open_new_tab ? "noopener noreferrer" : undefined}
          className="transition-colors hover:text-foreground/80 text-foreground/60"
        >
          {item.title}
        </a>
      );
    }

    return (
      <Link
        key={item.id}
        href={href}
        className="transition-colors hover:text-foreground/80 text-foreground/60"
      >
        {item.title}
      </Link>
    );
  };

  const renderNavItemWithChildren = (item: NavItem) => {
    if (!item.children || item.children.length === 0) {
      return renderNavLink(item);
    }

    const href = getNavHref(item);
    const isGroup = !href; // Group items have no link

    return (
      <DropdownMenu key={item.id}>
        <DropdownMenuTrigger className="flex items-center gap-1 transition-colors hover:text-foreground/80 text-foreground/60">
          {item.title}
          <ChevronDown className="h-3 w-3" />
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          {/* If parent has a link, show it as first item */}
          {!isGroup && href && (
            <>
              <DropdownMenuItem asChild>
                {item.nav_type === "external" ? (
                  <a
                    href={href}
                    target={item.open_new_tab ? "_blank" : "_self"}
                    rel={item.open_new_tab ? "noopener noreferrer" : undefined}
                  >
                    {item.title}
                  </a>
                ) : (
                  <Link href={href}>{item.title}</Link>
                )}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
            </>
          )}
          {item.children.map((child) => {
            const childHref = getNavHref(child);
            if (!childHref) return null;
            
            const isExternal = child.nav_type === "external";

            if (isExternal) {
              return (
                <DropdownMenuItem key={child.id} asChild>
                  <a
                    href={childHref}
                    target={child.open_new_tab ? "_blank" : "_self"}
                    rel={child.open_new_tab ? "noopener noreferrer" : undefined}
                  >
                    {child.title}
                  </a>
                </DropdownMenuItem>
              );
            }

            return (
              <DropdownMenuItem key={child.id} asChild>
                <Link href={childHref}>{child.title}</Link>
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuContent>
      </DropdownMenu>
    );
  };

  // Fallback nav items if API fails - 使用静态默认值避免水合错误
  const defaultNavItems = useMemo(() => [
    { href: "/", label: mounted ? t("nav.home") : "首页" },
    { href: "/archives", label: mounted ? t("nav.archive") : "归档" },
    { href: "/categories", label: mounted ? t("nav.categories") : "分类" },
    { href: "/tags", label: mounted ? t("nav.tags") : "标签" },
  ], [mounted, t]);

  return (
    <>
      {/* Top Loading Bar */}
      <Suspense fallback={null}>
        <TopLoader />
      </Suspense>
      
      <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-14 items-center">
        <div className="mr-4 flex">
          <Link href="/" className="mr-6 flex items-center space-x-2">
            {siteInfo ? (
              <>
                {siteInfo.logo && (
                  <Image
                    src={siteInfo.logo}
                    alt={siteInfo.name}
                    width={28}
                    height={28}
                    className="rounded"
                  />
                )}
                <span className="font-bold text-xl">{siteInfo.name}</span>
              </>
            ) : (
              <>
                <div className="w-7 h-7 rounded bg-muted animate-pulse" />
                <div className="w-24 h-6 rounded bg-muted animate-pulse" />
              </>
            )}
          </Link>
          <nav className="hidden md:flex items-center space-x-6 text-sm font-medium">
            {navItems.length > 0
              ? navItems.filter(item => !item.parent_id).map(renderNavItemWithChildren)
              : defaultNavItems.map((item) => (
                  <Link
                    key={item.href}
                    href={item.href}
                    className="transition-colors hover:text-foreground/80 text-foreground/60"
                  >
                    {item.label}
                  </Link>
                ))}
          </nav>
        </div>
        <div className="flex flex-1 items-center justify-end space-x-2">
          <form onSubmit={handleSearch} className="hidden md:flex items-center">
            <div className="relative">
              <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
              <Input
                type="search"
                placeholder={t("common.search") + "..."}
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                className="pl-8 w-[180px] h-8"
              />
            </div>
          </form>
          <LanguageSwitcher />
          <ThemeSwitcher />
          
          {authChecked && isAuthenticated && user?.role === "admin" ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="gap-2">
                  {user?.avatar ? (
                    <Image
                      src={user.avatar}
                      alt={user.display_name || user.username}
                      width={24}
                      height={24}
                      className="rounded-full"
                    />
                  ) : (
                    <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center">
                      <span className="text-xs font-medium">
                        {(user?.display_name || user?.username)?.[0]?.toUpperCase()}
                      </span>
                    </div>
                  )}
                  <span className="hidden sm:inline">{user?.display_name || user?.username}</span>
                </Button>
              </DropdownMenuTrigger>
              <DropdownMenuContent align="end">
                <DropdownMenuItem asChild>
                  <Link href="/manage" className="cursor-pointer">
                    <Settings className="mr-2 h-4 w-4" />
                    {t("nav.manage")}
                  </Link>
                </DropdownMenuItem>
                <DropdownMenuSeparator />
                <DropdownMenuItem onClick={handleLogout} className="cursor-pointer">
                  <LogOut className="mr-2 h-4 w-4" />
                  {t("nav.logout")}
                </DropdownMenuItem>
              </DropdownMenuContent>
            </DropdownMenu>
          ) : null}
          
          {/* Mobile Menu Button */}
          <Button
            variant="ghost"
            size="icon"
            className="md:hidden"
            onClick={() => setMobileMenuOpen(!mobileMenuOpen)}
          >
            {mobileMenuOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
          </Button>
        </div>
      </div>
      
      {/* Mobile Menu with Animation */}
      <AnimatePresence>
        {mobileMenuOpen && (
          <motion.div
            initial={{ opacity: 0, height: 0 }}
            animate={{ opacity: 1, height: "auto" }}
            exit={{ opacity: 0, height: 0 }}
            transition={{ duration: 0.2, ease: "easeOut" }}
            className="md:hidden border-t overflow-hidden"
          >
            <nav className="container py-4 flex flex-col gap-4">
              <form onSubmit={handleSearch} className="flex items-center">
                <div className="relative flex-1">
                  <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                  <Input
                    type="search"
                    placeholder={t("common.search") + "..."}
                    value={searchQuery}
                    onChange={(e) => setSearchQuery(e.target.value)}
                    className="pl-8 w-full"
                  />
                </div>
              </form>
              {navItems.length > 0
                ? navItems.filter(item => !item.parent_id).map((item) => {
                    const href = getNavHref(item);
                    
                    // Skip group items without href in mobile menu
                    if (!href) {
                      // Show children directly for group items
                      if (item.children && item.children.length > 0) {
                        return item.children.map((child) => {
                          const childHref = getNavHref(child);
                          if (!childHref) return null;
                        const isChildExternal = child.nav_type === "external";
                        if (isChildExternal) {
                          return (
                            <a
                              key={child.id}
                              href={childHref}
                              target={child.open_new_tab ? "_blank" : "_self"}
                              rel={child.open_new_tab ? "noopener noreferrer" : undefined}
                              className="text-foreground/60 hover:text-foreground pl-4"
                              onClick={() => setMobileMenuOpen(false)}
                            >
                              {child.title}
                            </a>
                          );
                        }
                        return (
                          <Link
                            key={child.id}
                            href={childHref}
                            className="text-foreground/60 hover:text-foreground pl-4"
                            onClick={() => setMobileMenuOpen(false)}
                          >
                            {child.title}
                          </Link>
                        );
                      });
                    }
                    return null;
                  }
                  
                  const isExternal = item.nav_type === "external";

                  if (isExternal) {
                    return (
                      <a
                        key={item.id}
                        href={href}
                        target={item.open_new_tab ? "_blank" : "_self"}
                        rel={item.open_new_tab ? "noopener noreferrer" : undefined}
                        className="text-foreground/60 hover:text-foreground"
                        onClick={() => setMobileMenuOpen(false)}
                      >
                        {item.title}
                      </a>
                    );
                  }

                  return (
                    <Link
                      key={item.id}
                      href={href}
                      className="text-foreground/60 hover:text-foreground"
                      onClick={() => setMobileMenuOpen(false)}
                    >
                      {item.title}
                    </Link>
                  );
                })
              : defaultNavItems.map((item) => (
                  <Link
                    key={item.href}
                    href={item.href}
                    className="text-foreground/60 hover:text-foreground"
                    onClick={() => setMobileMenuOpen(false)}
                  >
                    {item.label}
                  </Link>
                ))}
            </nav>
          </motion.div>
        )}
      </AnimatePresence>
    </header>
    </>
  );
}
