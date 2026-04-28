import { Link, useLocation, useNavigate } from "react-router-dom";
import { AnimatePresence, motion } from "motion/react";
import { ChevronDown, LogOut, Menu, Search, Settings, X } from "lucide-react";
import { useEffect, useMemo, useState } from "react";
import { LanguageSwitcher } from "@/components/language-switcher";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Input } from "@/components/ui/input";
import { TopLoader } from "@/components/ui/top-loader";
import {
  getInjectedSiteConfig,
  waitForNoteva,
  type NotevaSDKRef,
  type NotevaUser,
} from "@/hooks/useNoteva";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";

interface NavItem {
  id: number;
  parentId?: number | null;
  title?: string;
  name?: string;
  type?: string;
  target?: string;
  url?: string;
  openNewTab?: boolean;
  order?: number;
  visible?: boolean;
  children?: NavItem[];
}

type SDKNavItem = Awaited<ReturnType<NotevaSDKRef["site"]["getNav"]>>[number];
type NavResponseItem = SDKNavItem;

interface HeaderSiteInfo {
  name: string;
  logo: string;
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

function getBuiltinTargetKey(value: string): string | null {
  const target = value.trim();
  if (!target) return null;

  const normalized = target === "/" ? "home" : target.replace(/^\/+/, "");
  return normalized in BUILTIN_PATHS ? normalized : null;
}

function isSafeExternalHref(value: string): boolean {
  const href = value.trim().toLowerCase();
  return (
    href.startsWith("http://") ||
    href.startsWith("https://") ||
    href.startsWith("mailto:") ||
    href.startsWith("tel:")
  );
}

function getInitialSiteInfo(): HeaderSiteInfo {
  const config = getInjectedSiteConfig();
  return {
    name: config?.site_name || "Noteva",
    logo: config?.site_logo || "/logo.png",
  };
}

function normalizeNavItem(item: NavResponseItem): NavItem {
  const opensNewTab = item.openNewTab ?? false;
  const targetOrUrl = item.target === "_blank" ? item.url : item.target || item.url;

  return {
    id: item.id,
    parentId: item.parentId ?? null,
    title: item.title || item.name,
    name: item.name || item.title,
    type: item.type,
    target: targetOrUrl,
    url: item.url || targetOrUrl,
    openNewTab: opensNewTab,
    order: item.order ?? 0,
    visible: item.visible ?? true,
    children: item.children?.map((child) => normalizeNavItem(child)),
  };
}

function preloadThemeRoute(href: string | null) {
  if (!href || isSafeExternalHref(href) || href === "/") return;

  const path = href.split(/[?#]/)[0]?.replace(/\/+$/, "") || "/";

  switch (path) {
    case "/archives":
      void import("@/pages/archives");
      break;
    case "/categories":
      void import("@/pages/categories");
      break;
    case "/tags":
      void import("@/pages/tags");
      break;
    default:
      void import("@/pages/custom-page");
      break;
  }
}

export function SiteHeader() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const location = useLocation();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [navItems, setNavItems] = useState<NavItem[]>([]);
  const [siteInfo, setSiteInfo] = useState<HeaderSiteInfo>(() =>
    getInitialSiteInfo()
  );
  const [user, setUser] = useState<NotevaUser | null>(null);
  const [authChecked, setAuthChecked] = useState(false);

  useEffect(() => {
    setMobileMenuOpen(false);
  }, [location.pathname]);

  useEffect(() => {
    let active = true;

    const loadData = async () => {
      const noteva = await waitForNoteva();
      if (!active || !noteva) {
        if (active) setAuthChecked(true);
        return;
      }

      void noteva.site
        .getInfo()
        .then((info) => {
          if (!active) return;
          setSiteInfo({
            name: info.name || "Noteva",
            logo: info.logo || "/logo.png",
          });
        })
        .catch(() => {});

      void noteva.site
        .getNav()
        .then((nav) => {
          if (!active) return;
          setNavItems((nav || []).map((item) => normalizeNavItem(item)));
        })
        .catch(() => {
          if (active) setNavItems([]);
        });

      void noteva.user
        .check()
        .then((currentUser) => {
          if (active) setUser(currentUser);
        })
        .catch(() => {
          if (active) setUser(null);
        })
        .finally(() => {
          if (active) setAuthChecked(true);
        });
    };

    void loadData();

    return () => {
      active = false;
    };
  }, []);

  const defaultNavItems = useMemo(
    () => [
      { href: "/", label: t("nav.home") },
      { href: "/archives", label: t("nav.archive") },
      { href: "/categories", label: t("nav.categories") },
      { href: "/tags", label: t("nav.tags") },
    ],
    [t]
  );

  const rootNavItems = useMemo(
    () =>
      navItems
        .filter((item) => !item.parentId && item.visible !== false)
        .sort((a, b) => (a.order ?? 0) - (b.order ?? 0)),
    [navItems]
  );

  const handleLogout = async () => {
    const noteva = await waitForNoteva();
    if (!noteva) return;

    try {
      await noteva.user.logout();
    } finally {
      setUser(null);
    }
  };

  const handleSearch = (event: React.FormEvent) => {
    event.preventDefault();
    const keyword = searchQuery.trim();
    if (!keyword) return;

    setMobileMenuOpen(false);
    navigate(`/?q=${encodeURIComponent(keyword)}`);
  };

  const getNavTitle = (item: NavItem): string => {
    const customTitle = item.title || item.name || "";
    const targetUrl = item.target || item.url || "";

    if (item.type === "builtin") {
      const targetKey = getBuiltinTargetKey(targetUrl);
      const i18nKey = targetKey ? BUILTIN_I18N[targetKey] : null;
      if (i18nKey) return t(i18nKey);
    }

    return customTitle;
  };

  const getNavHref = (item: NavItem): string | null => {
    const targetUrl = item.target || item.url || "";
    if (item.type === "builtin" && !targetUrl) return null;

    switch (item.type) {
      case "builtin":
        return BUILTIN_PATHS[getBuiltinTargetKey(targetUrl) || "home"] || "/";
      case "page":
        return `/${targetUrl.replace(/^\/+/, "")}`;
      case "external":
        return isSafeExternalHref(targetUrl) ? targetUrl : null;
      default:
        return targetUrl || "/";
    }
  };

  const isActiveHref = (href: string | null, type?: string) => {
    if (!href || type === "external") return false;
    if (href === "/") return location.pathname === "/";
    return location.pathname === href || location.pathname.startsWith(`${href}/`);
  };

  const navLinkClass = (active: boolean) =>
    cn(
      "inline-flex h-9 items-center whitespace-nowrap rounded-full px-3 text-sm font-medium transition-colors",
      active
        ? "bg-foreground text-background shadow-sm"
        : "text-muted-foreground hover:bg-muted hover:text-foreground"
    );

  const renderNavLink = (item: NavItem, compact = false) => {
    const href = getNavHref(item);
    const title = getNavTitle(item);

    if (!href) {
      return (
        <span key={item.id} className={navLinkClass(false)}>
          {title}
        </span>
      );
    }

    const className = compact
      ? cn(
          "rounded-md px-3 py-2 text-sm transition-colors",
          isActiveHref(href, item.type)
            ? "bg-muted text-foreground"
            : "text-muted-foreground hover:bg-muted hover:text-foreground"
        )
      : navLinkClass(isActiveHref(href, item.type));

    if (item.type === "external") {
      return (
        <a
          key={item.id}
          href={href}
          target={item.openNewTab ? "_blank" : "_self"}
          rel={item.openNewTab ? "noopener noreferrer" : undefined}
          className={className}
        >
          {title}
        </a>
      );
    }

    return (
      <Link
        key={item.id}
        to={href}
        className={className}
        onMouseEnter={() => preloadThemeRoute(href)}
        onFocus={() => preloadThemeRoute(href)}
      >
        {title}
      </Link>
    );
  };

  const renderNavItemWithChildren = (item: NavItem) => {
    if (!item.children || item.children.length === 0) return renderNavLink(item);

    const href = getNavHref(item);
    const active =
      isActiveHref(href, item.type) ||
      item.children.some((child) => isActiveHref(getNavHref(child), child.type));

    return (
      <DropdownMenu key={item.id}>
        <DropdownMenuTrigger className={cn(navLinkClass(active), "flex items-center gap-1")}>
          {getNavTitle(item)}
          <ChevronDown className="h-3.5 w-3.5" />
        </DropdownMenuTrigger>
        <DropdownMenuContent align="start">
          {href && (
            <>
              <DropdownMenuItem asChild>{renderNavLink(item, true)}</DropdownMenuItem>
              <DropdownMenuSeparator />
            </>
          )}
          {item.children
            .filter((child) => child.visible !== false)
            .map((child) => {
              const childHref = getNavHref(child);
              if (!childHref) return null;
              return (
                <DropdownMenuItem key={child.id} asChild>
                  {renderNavLink(child, true)}
                </DropdownMenuItem>
              );
            })}
        </DropdownMenuContent>
      </DropdownMenu>
    );
  };

  const isAdmin = authChecked && user?.role === "admin";

  return (
    <>
      <TopLoader />
      <header className="sticky top-0 z-50 w-full border-b border-border/70 bg-background/85 shadow-sm shadow-black/[0.02] backdrop-blur-xl supports-[backdrop-filter]:bg-background/70">
        <div className="flex h-16 w-full items-center gap-3 px-4 sm:px-5 lg:px-6 xl:px-8">
          <Link
            to="/"
            className="group mr-1 flex min-w-0 shrink-0 items-center gap-2"
            aria-label={siteInfo.name}
          >
            {siteInfo.logo && (
              <img
                src={siteInfo.logo}
                alt=""
                width={32}
                height={32}
                className="size-8 rounded-md border border-border/70 bg-card object-cover"
              />
            )}
            <span className="max-w-[14rem] truncate text-lg font-semibold tracking-normal transition-colors group-hover:text-primary">
              {siteInfo.name}
            </span>
          </Link>

          <nav className="hidden min-w-0 flex-1 items-center gap-1 md:flex">
            {rootNavItems.length > 0
              ? rootNavItems.map(renderNavItemWithChildren)
              : defaultNavItems.map((item) => (
                  <Link
                    key={item.href}
                    to={item.href}
                    className={navLinkClass(isActiveHref(item.href))}
                    onMouseEnter={() => preloadThemeRoute(item.href)}
                    onFocus={() => preloadThemeRoute(item.href)}
                  >
                    {item.label}
                  </Link>
                ))}
          </nav>

          <div className="ml-auto flex items-center gap-1.5">
            <form onSubmit={handleSearch} className="hidden md:block">
              <div className="relative">
                <Search className="pointer-events-none absolute left-2.5 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                <Input
                  type="search"
                  placeholder={`${t("common.search")}...`}
                  value={searchQuery}
                  onChange={(event) => setSearchQuery(event.target.value)}
                  className="h-9 w-[190px] rounded-full pl-8"
                />
              </div>
            </form>

            <LanguageSwitcher />
            <ThemeSwitcher />

            {isAdmin ? (
              <DropdownMenu>
                <DropdownMenuTrigger asChild>
                  <Button variant="ghost" size="sm" className="gap-2 rounded-full">
                    {user.avatar ? (
                      <img
                        src={user.avatar}
                        alt={user.displayName || user.username}
                        width={24}
                        height={24}
                        className="size-6 rounded-full object-cover"
                      />
                    ) : (
                      <span className="flex size-6 items-center justify-center rounded-full bg-primary/10 text-xs font-medium text-primary">
                        {(user.displayName || user.username)?.[0]?.toUpperCase()}
                      </span>
                    )}
                    <span className="hidden max-w-[7rem] truncate sm:inline">
                      {user.displayName || user.username}
                    </span>
                  </Button>
                </DropdownMenuTrigger>
                <DropdownMenuContent align="end">
                  <DropdownMenuItem asChild>
                    <a href="/manage" className="cursor-pointer">
                      <Settings className="mr-2 h-4 w-4" />
                      {t("nav.manage")}
                    </a>
                  </DropdownMenuItem>
                  <DropdownMenuSeparator />
                  <DropdownMenuItem onClick={handleLogout} className="cursor-pointer">
                    <LogOut className="mr-2 h-4 w-4" />
                    {t("nav.logout")}
                  </DropdownMenuItem>
                </DropdownMenuContent>
              </DropdownMenu>
            ) : null}

            <Button
              variant="ghost"
              size="icon"
              className="md:hidden"
              onClick={() => setMobileMenuOpen((open) => !open)}
              aria-label={mobileMenuOpen ? t("common.hide") : t("common.show")}
            >
              {mobileMenuOpen ? <X className="h-5 w-5" /> : <Menu className="h-5 w-5" />}
            </Button>
          </div>
        </div>

        <AnimatePresence initial={false}>
          {mobileMenuOpen && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              transition={{ duration: 0.18, ease: "easeOut" }}
              className="overflow-hidden border-t border-border/70 md:hidden"
            >
              <nav className="flex w-full flex-col gap-2 px-4 py-4 sm:px-5">
                <form onSubmit={handleSearch} className="mb-2">
                  <div className="relative">
                    <Search className="pointer-events-none absolute left-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
                    <Input
                      type="search"
                      placeholder={`${t("common.search")}...`}
                      value={searchQuery}
                      onChange={(event) => setSearchQuery(event.target.value)}
                      className="w-full pl-9"
                    />
                  </div>
                </form>

                {rootNavItems.length > 0
                  ? rootNavItems.map((item) => (
                      <div key={item.id} className="flex flex-col gap-1">
                        {renderNavLink(item, true)}
                        {item.children
                          ?.filter((child) => child.visible !== false)
                          .map((child) => (
                            <div key={child.id} className="pl-4">
                              {renderNavLink(child, true)}
                            </div>
                          ))}
                      </div>
                    ))
                  : defaultNavItems.map((item) => (
                      <Link
                        key={item.href}
                        to={item.href}
                        className={cn(
                          "rounded-md px-3 py-2 text-sm transition-colors",
                          isActiveHref(item.href)
                            ? "bg-muted text-foreground"
                            : "text-muted-foreground hover:bg-muted hover:text-foreground"
                        )}
                        onMouseEnter={() => preloadThemeRoute(item.href)}
                        onFocus={() => preloadThemeRoute(item.href)}
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
