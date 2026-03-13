import { create } from "zustand";

// Import built-in locale files
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import en from "./locales/en.json";

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
];

// Built-in messages (compile-time)
const builtinMessages: Record<string, Record<string, unknown>> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en: en,
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
  _version: number;
}

// Read initial locale from localStorage (shared with admin panel)
function getInitialLocale(): Locale {
  if (typeof window !== "undefined") {
    const saved = localStorage.getItem("noteva-locale");
    if (saved) {
      try {
        const parsed = JSON.parse(saved);
        if (parsed.state?.locale) {
          return parsed.state.locale as Locale;
        }
      } catch {}
    }
  }
  return "zh-CN";
}

export const useI18nStore = create<I18nState>((set) => ({
  locale: "zh-CN",
  setLocale: (locale) => {
    set({ locale });
    if (typeof window !== "undefined") {
      localStorage.setItem("noteva-locale", JSON.stringify({ state: { locale } }));
    }
  },
  _version: 0,
}));

// Client-side init
if (typeof window !== "undefined") {
  const initialLocale = getInitialLocale();
  if (initialLocale !== "zh-CN") {
    useI18nStore.setState({ locale: initialLocale });
  }
}

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

  // Fallback to en
  if (!message && locale !== "en") {
    const fallbackMsgs = getMessages("en");
    message = getNestedValue(fallbackMsgs, key);
  }

  if (!message) {
    console.warn(`Missing translation for key: ${key}`);
    return key;
  }

  if (!params) return message;

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

// ── Load custom locales from SDK ──────────────────────────────────────

/**
 * Load custom locales via Noteva SDK.
 * The SDK reads from window.__CUSTOM_LOCALES__ (injected by server).
 * Call once during theme initialization.
 */
export function loadCustomLocales() {
  try {
    const Noteva = (window as any).Noteva;
    if (!Noteva?.i18n) return;

    const customList = Noteva.i18n.getCustomLocales();
    if (!customList || customList.length === 0) return;

    for (const item of customList) {
      if (item.code && item.name && item.translations) {
        customMessages[item.code] = item.translations;
        if (!customLocaleList.find((l) => l.code === item.code)) {
          customLocaleList.push({
            code: item.code,
            name: item.name,
            nativeName: item.name,
            isCustom: true,
          });
        }
      }
    }

    // Bump version to trigger re-renders
    useI18nStore.setState((s) => ({ _version: s._version + 1 }));
  } catch {
    // silent fail
  }
}
