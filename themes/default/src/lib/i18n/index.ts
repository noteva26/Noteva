import { create } from "zustand";

// Import locale files
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import en from "./locales/en.json";
import ja from "./locales/ja.json";
import ko from "./locales/ko.json";
import fr from "./locales/fr.json";
import de from "./locales/de.json";
import es from "./locales/es.json";
import ptBR from "./locales/pt-BR.json";
import ru from "./locales/ru.json";
import it from "./locales/it.json";

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
  { code: "pt-BR", name: "Brazilian Portuguese", nativeName: "Português do Brasil" },
  { code: "ru", name: "Russian", nativeName: "Русский" },
  { code: "it", name: "Italian", nativeName: "Italiano" },
];

// Legacy export for backward compat
export const locales = builtinLocales;

const builtinMessages: Record<string, Record<string, unknown>> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en: en,
  ja: ja,
  ko: ko,
  fr: fr,
  de: de,
  es: es,
  "pt-BR": ptBR,
  ru: ru,
  it: it,
};

// Combined getter for all available locales
export function getLocales(): LocaleInfo[] {
  return builtinLocales;
}

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
  /** Reserved for future async locale loading that needs re-renders */
  _version: number;
}

function applyLocaleSideEffects(locale: Locale) {
  if (typeof document !== "undefined") {
    document.documentElement.lang = locale;
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
    applyLocaleSideEffects(locale);
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
  applyLocaleSideEffects(useI18nStore.getState().locale);
  useI18nStore.subscribe((state) => {
    applyLocaleSideEffects(state.locale);
  });
}

// Get messages for a locale, with fallback to en
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
