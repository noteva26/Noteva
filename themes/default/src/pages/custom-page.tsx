import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { motion } from "motion/react";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "@/lib/i18n";
import { waitForNoteva, type NotevaSDKRef } from "@/hooks/useNoteva";
import PluginSlot from "@/components/plugin-slot";
import { sanitizeHtml } from "@/lib/sanitize-html";
import {
  getThemeListItemMotion,
  themePageContentMotion,
  themePageHeaderMotion,
} from "@/lib/motion";

type Page = Awaited<ReturnType<NotevaSDKRef["pages"]["get"]>>;

function CustomPageSkeleton() {
  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-4xl py-10">
          <Skeleton className="mb-6 h-9 w-24" />
          <Skeleton className="mb-8 h-12 w-3/4" />
          <div className="custom-page-content">
            <Skeleton className="mb-4 h-5 w-full" />
            <Skeleton className="mb-4 h-5 w-11/12" />
            <Skeleton className="mb-8 h-5 w-2/3" />
            <Skeleton className="h-48 w-full rounded-xl" />
          </div>
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}

export default function CustomPage() {
  const { slug } = useParams<{ slug: string }>();
  const navigate = useNavigate();
  const [page, setPage] = useState<Page | null>(null);
  const [siteInfo, setSiteInfo] = useState({ name: "Noteva" });
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);
  const { t } = useTranslation();

  useEffect(() => {
    if (!slug) { setNotFound(true); setLoading(false); return; }

    let active = true;
    const loadSiteInfo = async () => {
      const Noteva = await waitForNoteva();
      if (!active || !Noteva) return;
      try {
        const info = await Noteva.site.getInfo();
        if (active) setSiteInfo({ name: info.name || "Noteva" });
      } catch { }
    };
    void loadSiteInfo();

    return () => {
      active = false;
    };
  }, [slug]);

  useEffect(() => {
    if (page) document.title = `${page.title} - ${siteInfo.name}`;
  }, [page, siteInfo.name]);

  useEffect(() => {
    if (!slug) return;
    let active = true;

    const loadPage = async () => {
      setLoading(true);
      setNotFound(false);

      const Noteva = await waitForNoteva();
      if (!active) return;

      if (!Noteva) {
        setNotFound(true);
        setLoading(false);
        return;
      }

      try {
        const result = await Noteva.pages.get(slug);
        if (active) setPage(result);
      } catch {
        if (active) setNotFound(true);
      } finally {
        if (active) setLoading(false);
      }
    };
    void loadPage();

    return () => {
      active = false;
    };
  }, [slug]);

  if (loading) {
    return <CustomPageSkeleton />;
  }

  if (notFound || !page) {
    return (
      <div className="theme-page-shell relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1"><div className="container py-16 text-center max-w-4xl mx-auto">
          <h1 className="text-4xl font-bold mb-4">{t("error.notFound")}</h1>
          <p className="text-muted-foreground mb-8">{t("error.notFoundDesc")}</p>
          <Button onClick={() => navigate("/")}>{t("error.backHome")}</Button>
        </div></main>
        <SiteFooter />
      </div>
    );
  }

  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <motion.article
          {...themePageContentMotion}
          className="custom-page-shell container mx-auto max-w-4xl py-10"
          data-page-id={page.id}
        >
          <motion.div {...getThemeListItemMotion(0, 0.035)}>
            <Button variant="ghost" size="sm" className="mb-6" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-4 w-4 mr-2" />{t("common.back")}
            </Button>
          </motion.div>
          <motion.header {...themePageHeaderMotion} className="mb-8">
            <h1 className="text-4xl font-semibold leading-tight md:text-5xl">
              {page.title}
            </h1>
          </motion.header>
          <motion.section
            {...getThemeListItemMotion(1, 0.035)}
            className="custom-page-content prose prose-lg dark:prose-invert max-w-none"
          >
              <PluginSlot name="page_content_top" />
              <div className="page-content" dangerouslySetInnerHTML={{ __html: sanitizeHtml(page.html) }} />
              <PluginSlot name="page_content_bottom" />
          </motion.section>
        </motion.article>
      </main>
      <SiteFooter />
    </div>
  );
}
