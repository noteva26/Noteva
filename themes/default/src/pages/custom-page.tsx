import { useEffect, useState } from "react";
import { useParams, useNavigate } from "react-router-dom";
import { SiteHeader } from "@/components/site-header";
import { SiteFooter } from "@/components/site-footer";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Button } from "@/components/ui/button";
import { ArrowLeft } from "lucide-react";
import { useTranslation } from "@/lib/i18n";
import { getNoteva } from "@/hooks/useNoteva";

interface Page {
  id: number; slug: string; title: string; content: string;
  content_html?: string; html?: string; status: string;
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

    const loadSiteInfo = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadSiteInfo, 50); return; }
      try {
        const info = await Noteva.site.getInfo();
        setSiteInfo({ name: info.name || "Noteva" });
      } catch {}
    };
    loadSiteInfo();
  }, [slug]);

  useEffect(() => {
    if (page) document.title = `${page.title} - ${siteInfo.name}`;
  }, [page, siteInfo.name]);

  useEffect(() => {
    if (!slug) return;
    const loadPage = async () => {
      const Noteva = getNoteva();
      if (!Noteva) { setTimeout(loadPage, 50); return; }
      try {
        const result = await Noteva.api.get(`/page/${slug}`);
        setPage(result.page);
      } catch (err) {
        console.error(err);
        setNotFound(true);
      } finally { setLoading(false); }
    };
    loadPage();
  }, [slug]);

  const getHtml = (p: Page) => p.content_html || p.html || "";

  if (loading) {
    return (
      <div className="relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1"><div className="container py-8 max-w-4xl mx-auto">
          <Skeleton className="h-10 w-3/4 mb-4" /><Skeleton className="h-6 w-1/2 mb-8" /><Skeleton className="h-64 w-full" />
        </div></main>
        <SiteFooter />
      </div>
    );
  }

  if (notFound || !page) {
    return (
      <div className="relative flex min-h-screen flex-col">
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
    <div className="relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <article className="container py-8 max-w-4xl mx-auto">
          <Button variant="ghost" size="sm" className="mb-6" onClick={() => navigate(-1)}>
            <ArrowLeft className="h-4 w-4 mr-2" />{t("common.back")}
          </Button>
          <header className="mb-8"><h1 className="text-4xl font-bold">{page.title}</h1></header>
          <Card>
            <CardContent className="prose prose-lg dark:prose-invert max-w-none p-6 md:p-8">
              <div dangerouslySetInnerHTML={{ __html: getHtml(page) }} />
            </CardContent>
          </Card>
        </article>
      </main>
      <SiteFooter />
    </div>
  );
}
