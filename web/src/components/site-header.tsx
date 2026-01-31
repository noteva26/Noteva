"use client";

import Link from "next/link";
import Image from "next/image";
import { useAuthStore } from "@/lib/store/auth";
import { useSiteStore } from "@/lib/store/site";
import { useTranslation } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuSeparator,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { LanguageSwitcher } from "@/components/language-switcher";
import { ThemeSwitcher } from "@/components/theme-switcher";
import { Settings, LogOut, Menu, X } from "lucide-react";
import { useEffect, useState } from "react";

export function SiteHeader() {
  const { user, isAuthenticated, checkAuth, logout } = useAuthStore();
  const { settings } = useSiteStore();
  const { t } = useTranslation();
  const [mobileMenuOpen, setMobileMenuOpen] = useState(false);

  useEffect(() => {
    checkAuth();
  }, [checkAuth]);

  const handleLogout = async () => {
    await logout();
  };

  const navItems = [
    { href: "/", label: t("nav.home") },
    { href: "/archives", label: t("nav.archive") },
    { href: "/categories", label: t("nav.categories") },
    { href: "/tags", label: t("nav.tags") },
  ];

  return (
    <header className="sticky top-0 z-50 w-full border-b bg-background/95 backdrop-blur supports-[backdrop-filter]:bg-background/60">
      <div className="container flex h-14 items-center">
        <div className="mr-4 flex">
          <Link href="/" className="mr-6 flex items-center space-x-2">
            {settings.site_logo && (
              <Image
                src={settings.site_logo}
                alt={settings.site_name}
                width={28}
                height={28}
                className="rounded"
              />
            )}
            <span className="font-bold text-xl">{settings.site_name}</span>
          </Link>
          <nav className="hidden md:flex items-center space-x-6 text-sm font-medium">
            {navItems.map((item) => (
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
          <LanguageSwitcher />
          <ThemeSwitcher />
          
          {isAuthenticated ? (
            <DropdownMenu>
              <DropdownMenuTrigger asChild>
                <Button variant="ghost" size="sm" className="gap-2">
                  <div className="h-6 w-6 rounded-full bg-primary/10 flex items-center justify-center">
                    <span className="text-xs font-medium">
                      {user?.username?.[0]?.toUpperCase()}
                    </span>
                  </div>
                  <span className="hidden sm:inline">{user?.username}</span>
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
          ) : (
            <div className="hidden md:flex items-center gap-2">
              <Button variant="ghost" size="sm" asChild>
                <Link href="/login">{t("nav.login")}</Link>
              </Button>
              <Button size="sm" asChild>
                <Link href="/register">{t("nav.register")}</Link>
              </Button>
            </div>
          )}
          
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
      
      {/* Mobile Menu */}
      {mobileMenuOpen && (
        <div className="md:hidden border-t">
          <nav className="container py-4 flex flex-col gap-4">
            {navItems.map((item) => (
              <Link
                key={item.href}
                href={item.href}
                className="text-foreground/60 hover:text-foreground"
                onClick={() => setMobileMenuOpen(false)}
              >
                {item.label}
              </Link>
            ))}
            {!isAuthenticated && (
              <div className="flex gap-2 pt-2 border-t">
                <Button variant="outline" size="sm" asChild className="flex-1">
                  <Link href="/login">{t("nav.login")}</Link>
                </Button>
                <Button size="sm" asChild className="flex-1">
                  <Link href="/register">{t("nav.register")}</Link>
                </Button>
              </div>
            )}
          </nav>
        </div>
      )}
    </header>
  );
}
