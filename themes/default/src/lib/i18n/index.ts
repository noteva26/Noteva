import { create } from "zustand";

// Import locale files
import zhCN from "./locales/zh-CN.json";
import zhTW from "./locales/zh-TW.json";
import en from "./locales/en.json";

export type Locale = "zh-CN" | "zh-TW" | "en";

export interface LocaleInfo {
  code: Locale;
  name: string;
  nativeName: string;
}

export const locales: LocaleInfo[] = [
  { code: "zh-CN", name: "Simplified Chinese", nativeName: "简体中文" },
  { code: "zh-TW", name: "Traditional Chinese", nativeName: "繁體中文" },
  { code: "en", name: "English", nativeName: "English" },
];

const messages: Record<Locale, typeof zhCN> = {
  "zh-CN": zhCN,
  "zh-TW": zhTW,
  en: en,
};

interface I18nState {
  locale: Locale;
  setLocale: (locale: Locale) => void;
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
  return "zh-CN";
}

export const useI18nStore = create<I18nState>((set) => ({
  locale: "zh-CN",
  setLocale: (locale) => {
    set({ locale });
    // 手动保存到 localStorage
    if (typeof window !== "undefined") {
      localStorage.setItem("noteva-locale", JSON.stringify({ state: { locale } }));
    }
  },
}));

// 客户端初始化
if (typeof window !== "undefined") {
  const initialLocale = getInitialLocale();
  if (initialLocale !== "zh-CN") {
    useI18nStore.setState({ locale: initialLocale });
  }
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
  const message = getNestedValue(messages[locale] as unknown as Record<string, unknown>, key);
  
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
  const { locale, setLocale } = useI18nStore();
  
  const translate = (key: string, params?: Record<string, string | number>): string => {
    const message = getNestedValue(messages[locale] as unknown as Record<string, unknown>, key);
    
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
    locales,
  };
}
