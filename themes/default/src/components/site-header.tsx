import { Link, useNavigate } from "react-router-dom";
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
import { useEffect, useState, useMemo } from "react";
import { getNoteva } from "@/hooks/useNoteva";

interface NavItem {
  id: number;
  parent_id?: number | null;
  title?: string;
  name?: string;
  nav_type?: string;
  target?: string;
  url?: string;
  open_new_tab?: boolean;
  sort_order?: number;
  order?: number;
  visible?: boolean;
  children?: NavItem[];
}

const BUILTIN_PATHS: Record<string, string> = {
  home: "/",
  archives: "/archives",
  categories: "/categories",
  tags: "/tags",
};

const BUILTIN_I18N: Record<string, string> = {
  home: "nav.home",
  archives: "nav.archive",
  categories: "nav.categories",
  tags: "nav.tags",
};

export function SiteHeader() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [navItems, setNavItems] = useState<NavItem[]>([]);
  const [siteInfo, setSiteInfo] = useState<{ name: string; logo: string } | null>(null);
  const [user, setUser] = useState<any>(null);
  const [isAuthenticated, setIsAuthenticated] = useState(false);
  const [authChecked, setAuthChecked] = useState(false);

  useEffect(() => {
    const config = (window as any).__SITE_CONFIG__;
    if (config) {
      setSiteInfo({ name: config.site_name || "Noteva", logo: config.site_logo || "/logo.png" });
    } else {
      setSiteInfo({ name: "Noteva", logo: "/logo.png" });
    }
  }, []);

  useEffect(() => {
    const loadData = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadData, 50); return; }
      try {
        const info = await Noteva.site.getInfo();
        setSiteInfo({ name: info.name || "Noteva", logo: info.logo || "/logo.png" });

        const nav = await Noteva.site.getNav();
        const convertNavItem = (item: any): NavItem => ({
          id: item.id, parent_id: item.parent_id ?? null,
          title: item.title || item.name, name: item.name || item.title,
          nav_type: item.nav_type, target: item.target || item.url, url: item.url || item.target,
          open_new_tab: item.open_new_tab ?? (item.target === "_blank"),
          sort_order: item.sort_order ?? item.order ?? 0, order: item.order ?? item.sort_order,
          visible: item.visible ?? true, children: item.children?.map(convertNavItem),
        });
        setNavItems((nav || []).map(convertNavItem));

        const currentUser = await Noteva.user.check();
        setUser(currentUser);
        setIsAuthenticated(!!currentUser);
        setAuthChecked(true);
      } catch (err) { console.error(err); setAuthChecked(true); }
    };
    loadData();
  }, []);

  const handleLogout = async () => {
    const Noteva = getNoteva();
    if (!Noteva) return;
    try { await Noteva.user.logout(); setUser(null); setIsAuthenticated(false); } catch {}
  };

  const handleSearch = (e: React.FormEvent) => {
    e.preventDefault();
    if (searchQuery.trim()) navigate(`/?q=${encodeURIComponent(searchQuery.trim())}`);
  };

  const getNavTitle = (item: NavItem): string => {
    // 用户自定义了标题则优先使用
    const customTitle = item.title || item.name || "";
    if (item.nav_type === "builtin") {
      const url = item.target || item.url || "";
      const i18nKey = BUILTIN_I18N[url];
      // 只有标题和 builtin key 相同（未自定义）时才走 i18n
      if (i18nKey && (!customTitle || customTitle === url)) return t(i18nKey);
    }
    return customTitle;
  };

  const getNavHref = (item: NavItem): string | null => {
    const targetUrl = item.target || item.url || "";
    if (item.nav_type === "builtin" && !targetUrl) return null;
    switch (item.nav_type) {
      case "builtin": return BUILTIN_PATHS[targetUrl] || "/";
      case "page": return `/${targetUrl}`;
      case "external": return targetUrl;
      default: return targetUrl || "/";
    }
  };

  const renderNavLink = (item: NavItem) => {
    const href = getNavHref(item);
    if (!href) return <span key={item.id} className="transition-colors text-foreground/60">{getNavTitle(item)}</span>;
    if (item.nav_type === "external") {
      return <a key={item.id} href={href} target={item.open_new_tab ? "_blank" : "_self"} rel={item.open_new_tab ? "noopener noreferrer" : undefined} className="transition-colors hover:text-foreground/80 text-foreground/60">{getNavTitle(item)}</a>;
    }
    return <Link key={item.id} to={href} className="transition-colors hover:text-foreground/80 text-foreground/60">{getNavTitle(item)}</Link>;
  };

  const renderNavItemWithChildren = (item: NavItem) => {
    if (!item.children || item.children.length === 0) return renderNavLink(item);
    const href = getNavHref(item);
    const isGroup = !href;
    return (
      <DropdownMenu key={item.id}>
        <DropdownMenuTrigger className="flex items-center gap-1 transition-colors hover:text-foreground/80 text-foreground/60">
          {getNavTitle(item)}<ChevronDown className="h-3 w-3" />
        </DropdownMenuTrigger>
        <DropdownMenuContent>
          {!isGroup && href && (
            <>
              <DropdownMenuItem asChild>
                {item.nav_type === "external" ? <a href={href} target={item.open_new_tab ? "_blank" : "_self"} rel={item.open_new_tab ? "noopener noreferrer" : undefined}>{getNavTitle(item)}</a> : <Link to={href}>{getNavTitle(item)}</Link>}
              </DropdownMenuItem>
              <DropdownMenuSeparator />
            </>
          )}
          {item.children.map((child) => {
            const childHref = getNavHref(child);
            if (!childHref) return null;
            return (
              <DropdownMenuItem key={child.id} asChild>
                {child.nav_type === "external" ? <a href={childHref} target={child.open_new_tab ? "_blank" : "_self"} rel={child.open_new_tab ? "noopener noreferrer" : undefined}>{getNavTitle(child)}</a> : <Link to={childHref}>{getNavTitle(child)}</Link>}
              </DropdownMenuItem>
            );
          })}
        </DropdownMenuContent>
      </DropdownMenu>
    );
  };

  const defaultNavItems = useMemo(() => [
    { href: "/", label: t("nav.home") },
    { href: "/archives", label: t("nav.archive") },
    { href: "/categories", label: t("nav.categories") },
    { href: "/tags", label: t("nav.tags") },
  ], [t]);

  return (
    <>
      <TopLoader />
      <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
        <div className="container flex h-14 items-center">
          <div className="mr-4 flex">
            <Link to="/" className="mr-6 flex items-center space-x-2">
              {siteInfo ? (
                <>
                  {siteInfo.logo && <img src={siteInfo.logo} alt={siteInfo.name} width={28} height={28} className="rounded" />}
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
                    <Link key={item.href} to={item.href} className="transition-colors hover:text-foreground/80 text-foreground/60">{item.label}</Link>
                  ))}
            </nav>
          </div>
          <div className="flex flex-1 items-center justify-end space-x-2">
            <form onSubmit={handleSearch} className="hidden md:flex items-center">
              <div className="relative">
                <Search className="absolute left-2.5 top-1/2 -translate-y-1/2 h-4 w-4 text-muted-foreground" />
                <Input type="search" placeholder={t("common.search") + "..."} value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="pl-8 w-[180px] h-8" />
              </div>
            </form>
            <LanguageSwitcher />
            <ThemeSwitcher />
            
            {authChecked && isAuthenticated && user?.role === "admin" ? (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm" className="gap-2">
                    {user?.avatar ? (
                      <img src={user.avatar} alt={user.display_name || user.username} width={24} height={24} className="rounded-full" />
                    ) : (
                      <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center">
                        <span className="text-xs font-medium">{(user?.display_name || user?.username)?.[0]?.toUpperCase()}</span>
                      </div>
                    )}
                    <span className="hidden sm:inline">{user?.display_name || user?.username}</span>
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem asChild>
                    <a href="/manage" className="cursor-pointer"><Settings className="mr-2 h-4 w-4" />{t("nav.manage")}</a>
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem onClick={handleLogout} className="cursor-pointer"><LogOut className="mr-2 h-4 w-4" />{t("nav.logout")}</DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            ) : null}
            
            <Button variant="ghost" size="icon" className="md:hidden" onClick={() => setMobileMenuOpen(!mobileMenuOpen)}>
              {mobileMenuOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
            </Button>
          </div>
        </div>
        
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
                    <Input type="search" placeholder={t("common.search") + "..."} value={searchQuery} onChange={(e) => setSearchQuery(e.target.value)} className="pl-8 w-full" />
                  </div>
                </form>
                {navItems.length > 0
                  ? navItems.filter(item => !item.parent_id).map((item) => {
                      const href = getNavHref(item);
                      if (!href) {
                        if (item.children && item.children.length > 0) {
                          return item.children.map((child) => {
                            const childHref = getNavHref(child);
                            if (!childHref) return null;
                            if (child.nav_type === "external") {
                              return <a key={child.id} href={childHref} target={child.open_new_tab ? "_blank" : "_self"} rel={child.open_new_tab ? "noopener noreferrer" : undefined} className="text-foreground/60 hover:text-foreground pl-4" onClick={() => setMobileMenuOpen(false)}>{getNavTitle(child)}</a>;
                            }
                            return <Link key={child.id} to={childHref} className="text-foreground/60 hover:text-foreground pl-4" onClick={() => setMobileMenuOpen(false)}>{getNavTitle(child)}</Link>;
                          });
                        }
                        return null;
                      }
                      if (item.nav_type === "external") {
                        return <a key={item.id} href={href} target={item.open_new_tab ? "_blank" : "_self"} rel={item.open_new_tab ? "noopener noreferrer" : undefined} className="text-foreground/60 hover:text-foreground" onClick={() => setMobileMenuOpen(false)}>{getNavTitle(item)}</a>;
                      }
                      return <Link key={item.id} to={href} className="text-foreground/60 hover:text-foreground" onClick={() => setMobileMenuOpen(false)}>{getNavTitle(item)}</Link>;
                    })
                  : defaultNavItems.map((item) => (
                      <Link key={item.href} to={item.href} className="text-foreground/60 hover:text-foreground" onClick={() => setMobileMenuOpen(false)}>{item.label}</Link>
                    ))}
              </nav>
            </motion.div>
          )}
        </AnimatePresence>
      </header>
    </>
  );
}
