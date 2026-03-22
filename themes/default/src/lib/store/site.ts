/**
 * Site Store - 基于 SDK 的站点设置管理
 * 使用 Noteva SDK 替代直接 API 调用
 */
import { create } from "zustand";

interface SiteSettings {
  site_name: string;
  site_description: string;
  site_subtitle: string;
  site_logo: string;
  site_footer: string;
  email_verification_enabled?: string;
  [key: string]: string | undefined;
}

// 默认设置
const defaultSettings: SiteSettings = {
  site_name: "Noteva",
  site_description: "",
  site_subtitle: "",
  site_logo: "/logo.png",
  site_footer: "",
};

// 获取 SDK 实例
function getNoteva() {
  if (typeof window !== "undefined" && window.Noteva) {
    return window.Noteva;
  }
  return null;
}

// 从 window.__SITE_CONFIG__ 读取后端注入的配置
const getInjectedSettings = (): SiteSettings | null => {
  if (typeof window === "undefined") return null;
  try {
    const injected = (window as any).__SITE_CONFIG__;
    if (injected) {
      return {
        site_name: injected.site_name || defaultSettings.site_name,
        site_description: injected.site_description || defaultSettings.site_description,
        site_subtitle: injected.site_subtitle || defaultSettings.site_subtitle,
        site_logo: injected.site_logo || defaultSettings.site_logo,
        site_footer: injected.site_footer || defaultSettings.site_footer,
      };
    }
  } catch {
    // ignore
  }
  return null;
};

interface SiteState {
  settings: SiteSettings;
  loaded: boolean;
  loading: boolean;
  fetchSettings: () => Promise<void>;
  updateSettings: (settings: SiteSettings) => void;
}

export const useSiteStore = create<SiteState>((set, get) => ({
  settings: getInjectedSettings() || defaultSettings,
  loaded: !!getInjectedSettings(),
  loading: false,

  fetchSettings: async () => {
    // 如果已经加载或正在加载，跳过
    if (get().loaded || get().loading) return;
    
    set({ loading: true });
    try {
      // 等待 SDK 就绪
      const sdk = getNoteva();
      if (!sdk) {
        // SDK 未加载，等待后重试
        await new Promise<void>((resolve) => {
          const check = () => {
            if (getNoteva()) {
              resolve();
            } else {
              setTimeout(check, 50);
            }
          };
          check();
        });
      }
      
      const noteva = getNoteva()!;
      await noteva.ready();
      
      const data = await noteva.site.getInfo();
      const settings: SiteSettings = {
        site_name: data.name || data.site_name || defaultSettings.site_name,
        site_description: data.description || data.site_description || defaultSettings.site_description,
        site_subtitle: data.subtitle || data.site_subtitle || defaultSettings.site_subtitle,
        site_logo: data.logo || data.site_logo || defaultSettings.site_logo,
        site_footer: data.footer || data.site_footer || defaultSettings.site_footer,
        email_verification_enabled: (data as any).email_verification_enabled || "false",
      };
      set({ settings, loaded: true, loading: false });
    } catch {
      set({ loaded: true, loading: false });
    }
  },

  updateSettings: (settings: SiteSettings) => {
    set({ settings, loaded: true });
  },
}));

export type { SiteSettings };
