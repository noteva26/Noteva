import { create } from "zustand";
import { persist } from "zustand/middleware";
import { useCallback } from "react";

// Import built-in locale files
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import en from "./locales/en.json";

export type Locale = string;

export interface LocaleInfo {
  code: string;
  name: string;
  nativeName: string;
}

// Built-in locales
export const builtinLocales: LocaleInfo[] = [
  { code: "zh-CN", name: "Simplified Chinese", nativeName: "简体中文" },
  { code: "zh-TW", name: "Traditional Chinese", nativeName: "繁體中文" },
  { code: "en", name: "English", nativeName: "English" },
  { code: "ja", name: "Japanese", nativeName: "日本語" },
  { code: "ko", name: "Korean", nativeName: "한국어" },
  { code: "fr", name: "French", nativeName: "Français" },
  { code: "de", name: "German", nativeName: "Deutsch" },
  { code: "es", name: "Spanish", nativeName: "Español" },
  { code: "pt-BR", name: "Portuguese (Brazil)", nativeName: "Português (Brasil)" },
  { code: "ru", name: "Russian", nativeName: "Русский" },
  { code: "it", name: "Italian", nativeName: "Italiano" },
];

// Built-in messages (compile-time)
const builtinMessages: Record<string, Record<string, unknown>> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en: en,
};

const builtinLocaleLoaders: Record<string, () => Promise<Record<string, unknown>>> = {
  ja: () => import("./locales/ja.json").then((module) => module.default as Record<string, unknown>),
  ko: () => import("./locales/ko.json").then((module) => module.default as Record<string, unknown>),
  fr: () => import("./locales/fr.json").then((module) => module.default as Record<string, unknown>),
  de: () => import("./locales/de.json").then((module) => module.default as Record<string, unknown>),
  es: () => import("./locales/es.json").then((module) => module.default as Record<string, unknown>),
  "pt-BR": () => import("./locales/pt-BR.json").then((module) => module.default as Record<string, unknown>),
  ru: () => import("./locales/ru.json").then((module) => module.default as Record<string, unknown>),
  it: () => import("./locales/it.json").then((module) => module.default as Record<string, unknown>),
};

// Combined getter for all available locales
export function getLocales(): LocaleInfo[] {
  return builtinLocales;
}

// Legacy export for backward compat
export const locales = builtinLocales;

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Incremented when lazy locale chunks are loaded to trigger re-renders */
  _version: number;
}

function applyLocaleSideEffects(locale: Locale) {
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
  }
}

async function loadBuiltInLocale(locale: string) {
  if (builtinMessages[locale]) return;
  const loader = builtinLocaleLoaders[locale];
  if (!loader) return;

  try {
    builtinMessages[locale] = await loader();
    useI18nStore.setState((state) => ({ _version: state._version + 1 }));
  } catch {
    console.warn(`Failed to load locale: ${locale}`);
  }
}

/**
 * Detect browser language and match against available locales.
 * Matching priority: exact match (zh-CN) → language prefix (zh → zh-CN) → fallback to en
 */
function detectBrowserLocale(): Locale {
  if (typeof navigator === "undefined") return "en";

  const langs = navigator.languages?.length ? navigator.languages : [navigator.language];
  const available = builtinLocales.map((l) => l.code);

  for (const lang of langs) {
    // Exact match: zh-CN → zh-CN
    if (available.includes(lang)) return lang;
    // Prefix match: zh → zh-CN
    const prefix = lang.split("-")[0];
    const match = available.find((a) => a === prefix || a.startsWith(prefix + "-"));
    if (match) return match;
  }

  return "en";
}

export const useI18nStore = create<I18nState>()(
  persist(
    (set) => ({
      locale: detectBrowserLocale(),
      setLocale: (locale) => {
        applyLocaleSideEffects(locale);
        set({ locale });
        void loadBuiltInLocale(locale);
      },
      _version: 0,
    }),
    {
      name: "noteva-locale",
    }
  )
);

if (typeof window !== "undefined") {
  applyLocaleSideEffects(useI18nStore.getState().locale);
  void loadBuiltInLocale(useI18nStore.getState().locale);
  useI18nStore.subscribe((state) => {
    applyLocaleSideEffects(state.locale);
  });
}

// Get the messages map for the given locale, with fallback to en
function getMessages(locale: string): Record<string, unknown> {
  return (
    builtinMessages[locale] ||
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

  const translate = useCallback(
    (key: string, params?: Record<string, string | number>): string => {
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
    },
    [locale, _version]
  );

  return {
    t: translate,
    locale,
    setLocale,
    locales: getLocales(),
    _version,
  };
}
