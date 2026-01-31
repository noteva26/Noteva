"use client";

import { useSiteStore } from "@/lib/store/site";
import { useTranslation } from "@/lib/i18n";
import { useEffect, useState } from "react";

// 从后端注入的配置读取初始值，避免闪烁
const getInitialFooter = () => {
  if (typeof window !== "undefined") {
    const config = (window as any).__SITE_CONFIG__;
    if (config) {
      return {
        footer: config.site_footer || "",
        name: config.site_name || "Noteva",
      };
    }
  }
  return { footer: "", name: "Noteva" };
};

export function SiteFooter() {
  const { t } = useTranslation();
  const { settings, loaded } = useSiteStore();
  const [mounted, setMounted] = useState(false);
  const [initialData] = useState(getInitialFooter);
  
  useEffect(() => {
    setMounted(true);
  }, []);
  
  // 使用已加载的设置或初始注入的配置
  const siteName = loaded ? settings.site_name : initialData.name;
  const siteFooter = loaded ? settings.site_footer : initialData.footer;
  
  // 默认页脚文本 - 只在客户端挂载后使用翻译
  const defaultFooter = mounted 
    ? `© ${new Date().getFullYear()} ${siteName}. ${t("footer.allRightsReserved")}`
    : `© ${new Date().getFullYear()} ${siteName}. All rights reserved.`;
  
  return (
    <footer className="border-t py-6">
      <div className="container">
        <p 
          className="text-center text-sm text-muted-foreground"
          dangerouslySetInnerHTML={{ 
            __html: siteFooter || defaultFooter 
          }}
        />
      </div>
    </footer>
  );
}
