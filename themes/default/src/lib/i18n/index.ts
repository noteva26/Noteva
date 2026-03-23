import { create } from "zustand";

// Import locale files
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

// Legacy export for backward compat
export const locales = builtinLocales;

const builtinMessages: Record<string, Record<string, unknown>> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en: en,
};

// Runtime-loaded custom locale messages
const customMessages: Record<string, Record<string, unknown>> = {};
const customLocaleList: LocaleInfo[] = [];

// Combined getter for all available locales
export function getLocales(): LocaleInfo[] {
  return [...builtinLocales, ...customLocaleList];
}

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Incremented when custom locales are loaded to trigger re-renders */
  _version: number;
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
    if (available.includes(lang)) return lang;
    const prefix = lang.split("-")[0];
    const match = available.find((a) => a === prefix || a.startsWith(prefix + "-"));
    if (match) return match;
  }

  return "en";
}

// 从 localStorage 读取初始值
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
  return detectBrowserLocale();
}

export const useI18nStore = create<I18nState>((set) => ({
  locale: detectBrowserLocale(),
  setLocale: (locale) => {
    set({ locale });
    if (typeof window !== "undefined") {
      localStorage.setItem("noteva-locale", JSON.stringify({ state: { locale } }));
    }
  },
  _version: 0,
}));

// 客户端初始化
if (typeof window !== "undefined") {
  const initialLocale = getInitialLocale();
  if (initialLocale !== detectBrowserLocale()) {
    useI18nStore.setState({ locale: initialLocale });
  }
}

// Get messages for a locale, with fallback to en
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

function registerCustomLocale(code: string, name: string, translations: Record<string, unknown>) {
  customMessages[code] = translations;
  if (!customLocaleList.find((l) => l.code === code)) {
    customLocaleList.push({ code, name, nativeName: name, isCustom: true });
  }
  useI18nStore.setState((s) => ({ _version: s._version + 1 }));
}

/**
 * Load custom locales from window.__CUSTOM_LOCALES__
 * (injected by backend in every theme HTML page)
 */
export function loadCustomLocales() {
  try {
    if (typeof window === "undefined") return;
    // eslint-disable-next-line @typescript-eslint/no-explicit-any
    const data = (window as any).__CUSTOM_LOCALES__;
    if (!Array.isArray(data)) return;

    for (const item of data) {
      if (item.code && item.translations) {
        registerCustomLocale(item.code, item.name, item.translations);
      }
    }
  } catch {
    // Silent fail
  }
}


