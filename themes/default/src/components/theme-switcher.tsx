"use client";

import { useTheme } from "next-themes";
import { useTranslation } from "@/lib/i18n";
import { Button } from "@/components/ui/button";
import {
  DropdownMenu,
  DropdownMenuContent,
  DropdownMenuItem,
  DropdownMenuTrigger,
} from "@/components/ui/dropdown-menu";
import { Sun, Moon, Monitor, Check } from "lucide-react";

export function ThemeSwitcher() {
  const { theme, setTheme } = useTheme();
  const { t } = useTranslation();

  const themes = [
    { value: "light", label: t("theme.light"), icon: Sun },
    { value: "dark", label: t("theme.dark"), icon: Moon },
    { value: "system", label: t("theme.system"), icon: Monitor },
  ];

  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <Button variant="ghost" size="icon" title={t("settings.appearance")}>
          <Sun className="h-5 w-5 rotate-0 scale-100 transition-all dark:-rotate-90 dark:scale-0" />
          <Moon className="absolute h-5 w-5 rotate-90 scale-0 transition-all dark:rotate-0 dark:scale-100" />
          <span className="sr-only">{t("settings.appearance")}</span>
        </Button>
      </DropdownMenuTrigger>
      <DropdownMenuContent align="end">
        {themes.map((item) => (
          <DropdownMenuItem
            key={item.value}
            onClick={() => setTheme(item.value)}
            className="flex items-center justify-between"
          >
            <span className="flex items-center gap-2">
              <item.icon className="h-4 w-4" />
              {item.label}
            </span>
            {theme === item.value && <Check className="h-4 w-4 ml-2" />}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  );
}
