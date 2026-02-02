import { create } from "zustand";
import { siteApi, SiteSettings } from "@/lib/api";

// 默认设置
const defaultSettings: SiteSettings = {
  site_name: "Noteva",
  site_description: "",
  site_subtitle: "",
  site_logo: "/logo.png",
  site_footer: "",
  demo_mode: false,
};

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
        demo_mode: injected.demo_mode || false,
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
      // 使用公开 API，不需要登录
      const { data } = await siteApi.getInfo();
      const settings = {
        site_name: data.site_name || defaultSettings.site_name,
        site_description: data.site_description || defaultSettings.site_description,
        site_subtitle: data.site_subtitle || defaultSettings.site_subtitle,
        site_logo: data.site_logo || defaultSettings.site_logo,
        site_footer: data.site_footer || defaultSettings.site_footer,
        demo_mode: data.demo_mode || false,
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
