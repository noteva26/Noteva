import { create } from "zustand";
import { persist } from "zustand/middleware";

// Import built-in locale files
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import en from "./locales/en.json";
import ja from "./locales/ja.json";

export type Locale = string;

export interface LocaleInfo {
  code: string;
  name: string;
  nativeName: string;
  isCustom?: boolean;
}

// Built-in locales
export const builtinLocales: LocaleInfo[] = [
  { code: "zh-CN", name: "Simplified Chinese", nativeName: "简体中文" },
  { code: "zh-TW", name: "Traditional Chinese", nativeName: "繁體中文" },
  { code: "en", name: "English", nativeName: "English" },
  { code: "ja", name: "Japanese", nativeName: "日本語" },
];

// Built-in messages (compile-time)
const builtinMessages: Record<string, Record<string, unknown>> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en: en,
  ja: ja,
};

// Runtime-loaded custom locale messages
const customMessages: Record<string, Record<string, unknown>> = {};

// Custom locale list (populated at runtime)
const customLocaleList: LocaleInfo[] = [];

// Combined getter for all available locales
export function getLocales(): LocaleInfo[] {
  return [...builtinLocales, ...customLocaleList];
}

// Legacy export for backward compat
export const locales = builtinLocales;

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Incremented when custom locales are loaded to trigger re-renders */
  _version: number;
}

export const useI18nStore = create<I18nState>()(
  persist(
    (set) => ({
      locale: "zh-CN",
      setLocale: (locale) => set({ locale }),
      _version: 0,
    }),
    {
      name: "noteva-locale",
    }
  )
);

// Get the messages map for the given locale, with fallback to en
function getMessages(locale: string): Record<string, unknown> {
  return (
    builtinMessages[locale] ||
    customMessages[locale] ||
    builtinMessages["en"] ||
    {}
  );
}

// Get nested value from object using dot notation
function getNestedValue(obj: Record<string, unknown>, path: string): string | undefined {
  const keys = path.split(".");
  let current: unknown = obj;

  for (const key of keys) {
    if (current && typeof current === "object" && key in current) {
      current = (current as Record<string, unknown>)[key];
    } else {
      return undefined;
    }
  }

  return typeof current === "string" ? current : undefined;
}

// Translation function
export function t(key: string, params?: Record<string, string | number>): string {
  const locale = useI18nStore.getState().locale;
  const msgs = getMessages(locale);
  let message = getNestedValue(msgs, key);

  // Fallback to en if missing
  if (!message && locale !== "en") {
    const enMsgs = getMessages("en");
    message = getNestedValue(enMsgs, key);
  }

  if (!message) {
    console.warn(`Missing translation for key: ${key}`);
    return key;
  }

  if (!params) return message;

  // Replace placeholders like {name} with values
  return message.replace(/\{(\w+)\}/g, (_, paramKey) => {
    return params[paramKey]?.toString() ?? `{${paramKey}}`;
  });
}

// Hook for using translations in components
export function useTranslation() {
  const { locale, setLocale, _version } = useI18nStore();

  const translate = (key: string, params?: Record<string, string | number>): string => {
    const msgs = getMessages(locale);
    let message = getNestedValue(msgs, key);

    if (!message && locale !== "en") {
      const enMsgs = getMessages("en");
      message = getNestedValue(enMsgs, key);
    }

    if (!message) {
      console.warn(`Missing translation for key: ${key}`);
      return key;
    }

    if (!params) return message;

    return message.replace(/\{(\w+)\}/g, (_, paramKey) => {
      return params[paramKey]?.toString() ?? `{${paramKey}}`;
    });
  };

  return {
    t: translate,
    locale,
    setLocale,
    locales: getLocales(),
    _version,
  };
}

// ── Dynamic custom locale loading ─────────────────────────────────────

/**
 * Register a custom locale at runtime.
 * Called after fetching from the API.
 */
export function registerCustomLocale(code: string, name: string, translations: Record<string, unknown>) {
  customMessages[code] = translations;

  // Add to custom locale list if not already present
  if (!customLocaleList.find((l) => l.code === code)) {
    customLocaleList.push({ code, name, nativeName: name, isCustom: true });
  }

  // Bump version to trigger re-renders in components that use useTranslation()
  useI18nStore.setState((s) => ({ _version: s._version + 1 }));
}

/**
 * Remove a custom locale from runtime registry.
 */
export function unregisterCustomLocale(code: string) {
  delete customMessages[code];
  const idx = customLocaleList.findIndex((l) => l.code === code);
  if (idx !== -1) customLocaleList.splice(idx, 1);

  // If user was using this locale, reset to default
  if (useI18nStore.getState().locale === code) {
    useI18nStore.getState().setLocale("zh-CN");
  }

  useI18nStore.setState((s) => ({ _version: s._version + 1 }));
}

/**
 * Load all custom locales from backend API.
 * Call this once during app initialization.
 */
export async function loadCustomLocales() {
  try {
    const { localesApi } = await import("../api");
    const res = await localesApi.list();
    const items = res.data.locales;

    // Fetch full translations for each custom locale
    for (const item of items) {
      try {
        const detail = await localesApi.get(item.code);
        registerCustomLocale(item.code, item.name, detail.data.translations as Record<string, unknown>);
      } catch {
        console.warn(`Failed to load custom locale: ${item.code}`);
      }
    }
  } catch {
    // Silent fail - custom locales are optional
  }
}
