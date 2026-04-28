import { useEffect, useState } from "react";
import { getInjectedSiteConfig } from "@/hooks/useNoteva";
import { useTranslation } from "@/lib/i18n";
import { sanitizeHtml } from "@/lib/sanitize-html";

interface FooterData {
  footer: string;
  name: string;
}

function getInitialFooterData(): FooterData {
  const config = getInjectedSiteConfig();
  return {
    footer: config?.site_footer || "",
    name: config?.site_name || "Noteva",
  };
}

export function SiteFooter() {
  const { t } = useTranslation();
  const [footerData, setFooterData] = useState<FooterData>(() =>
    getInitialFooterData()
  );

  useEffect(() => {
    setFooterData(getInitialFooterData());
  }, []);

  const year = new Date().getFullYear();

  return (
    <footer className="mt-10 border-t border-border/70 bg-muted/25 py-8">
      <div className="container">
        {footerData.footer ? (
          <p
            className="text-center text-sm leading-6 text-muted-foreground [&_a]:text-foreground [&_a]:underline-offset-4 hover:[&_a]:underline"
            dangerouslySetInnerHTML={{ __html: sanitizeHtml(footerData.footer) }}
          />
        ) : (
          <p className="text-center text-sm leading-6 text-muted-foreground">
            &copy; {year} {footerData.name}. {t("footer.allRightsReserved")}
          </p>
        )}
      </div>
    </footer>
  );
}
