"use client";

import { useSiteStore } from "@/lib/store/site";
import { useTranslation } from "@/lib/i18n";

export function SiteFooter() {
  const { t } = useTranslation();
  const { settings } = useSiteStore();
  
  // 默认页脚文本
  const defaultFooter = `© ${new Date().getFullYear()} ${settings.site_name}. ${t("footer.allRightsReserved")}`;
  
  return (
    <footer className="border-t py-6">
      <div className="container">
        <p 
          className="text-center text-sm text-muted-foreground"
          dangerouslySetInnerHTML={{ 
            __html: settings.site_footer || defaultFooter 
          }}
        />
      </div>
    </footer>
  );
}
