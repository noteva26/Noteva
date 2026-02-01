"use client";

import { useTranslation } from "@/lib/i18n";
import { useEffect, useState } from "react";

export function SiteFooter() {
  const { t } = useTranslation();
  const [mounted, setMounted] = useState(false);
  const [footerData, setFooterData] = useState<{ footer: string; name: string } | null>(null);
  
  useEffect(() => {
    setMounted(true);
    // 从注入的配置读取
    const config = (window as any).__SITE_CONFIG__;
    if (config) {
      setFooterData({
        footer: config.site_footer || "",
        name: config.site_name || "Noteva",
      });
    } else {
      setFooterData({ footer: "", name: "Noteva" });
    }
  }, []);
  
  // SSR 阶段显示骨架屏
  if (!footerData) {
    return (
      <footer className="border-t py-6">
        <div className="container">
          <div className="h-5 w-64 mx-auto rounded bg-muted animate-pulse" />
        </div>
      </footer>
    );
  }
  
  // 默认页脚文本
  const defaultFooter = mounted 
    ? `© ${new Date().getFullYear()} ${footerData.name}. ${t("footer.allRightsReserved")}`
    : `© ${new Date().getFullYear()} ${footerData.name}. All rights reserved.`;
  
  return (
    <footer className="border-t py-6">
      <div className="container">
        <p 
          className="text-center text-sm text-muted-foreground"
          dangerouslySetInnerHTML={{ 
            __html: footerData.footer || defaultFooter 
          }}
        />
      </div>
    </footer>
  );
}
