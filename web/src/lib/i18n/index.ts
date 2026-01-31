import { create } from "zustand";
import { persist } from "zustand/middleware";

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

export const useI18nStore = create<I18nState>()(
  persist(
    (set) => ({
      locale: "zh-CN",
      setLocale: (locale) => set({ locale }),
    }),
    {
      name: "noteva-locale",
    }
  )
);

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
