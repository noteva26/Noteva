import { useEffect, useMemo, useState } from "react";
import { Link, useNavigate } from "react-router-dom";
import { motion } from "motion/react";
import { ExternalLink, MapPin, UserRound } from "lucide-react";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { waitForNoteva } from "@/hooks/useNoteva";
import { useTranslation } from "@/lib/i18n";
import { sanitizeHtml } from "@/lib/sanitize-html";
import PluginSlot from "@/components/plugin-slot";
import {
  getThemeListItemMotion,
  themeHoverLift,
  themePageHeaderMotion,
} from "@/lib/motion";

type AboutProfile = Awaited<ReturnType<NonNullable<typeof window.Noteva>["about"]["get"]>>;
type CustomPageData = Awaited<ReturnType<NonNullable<typeof window.Noteva>["pages"]["get"]>>;

function AboutSkeleton() {
  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-5xl py-10">
          <div className="grid gap-6 md:grid-cols-[18rem_minmax(0,1fr)]">
            <Skeleton className="h-72 rounded-lg" />
            <div className="space-y-4">
              <Skeleton className="h-10 w-3/4" />
              <Skeleton className="h-5 w-full" />
              <Skeleton className="h-5 w-10/12" />
              <Skeleton className="h-32 w-full rounded-lg" />
            </div>
          </div>
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}

function getHost(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

export default function AboutPage() {
  const { t } = useTranslation();
  const navigate = useNavigate();
  const [profile, setProfile] = useState<AboutProfile | null>(null);
  const [customPage, setCustomPage] = useState<CustomPageData | null>(null);
  const [siteName, setSiteName] = useState("Noteva");
  const [loading, setLoading] = useState(true);
  const [notFound, setNotFound] = useState(false);

  useEffect(() => {
    let active = true;

    const load = async () => {
      setLoading(true);
      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setNotFound(true);
        setLoading(false);
        return;
      }

      try {
        const siteInfo = await noteva.site.getInfo().catch(() => null);
        if (!active) return;
        setSiteName(siteInfo?.name || "Noteva");

        const about = await noteva.about.get();
        if (!active) return;
        setProfile(about);
      } catch {
        try {
          const page = await noteva.pages.get("about");
          if (!active) return;
          setCustomPage(page);
          setNotFound(false);
        } catch {
          if (active) setNotFound(true);
        }
      } finally {
        if (active) setLoading(false);
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (profile || customPage) {
      document.title = `${customPage?.title || t("about.title")} - ${siteName}`;
    }
  }, [customPage, profile, siteName, t]);

  const visibleSocialLinks = useMemo(
    () => (profile?.socialLinks || []).filter((link) => link.url),
    [profile]
  );
  const visibleTimeline = useMemo(
    () =>
      (profile?.timeline || []).filter(
        (item) => item.title || item.date || item.description
      ),
    [profile]
  );

  if (loading) {
    return <AboutSkeleton />;
  }

  if (customPage) {
    return (
      <div className="theme-page-shell relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <motion.article
            {...getThemeListItemMotion(0, 0.035)}
            className="custom-page-shell container mx-auto max-w-4xl py-10"
            data-page-id={customPage.id}
          >
            <motion.div {...getThemeListItemMotion(0, 0.035)}>
              <Button variant="ghost" size="sm" className="mb-6" onClick={() => navigate(-1)}>
                {t("common.back")}
              </Button>
            </motion.div>
            <motion.header {...themePageHeaderMotion} className="mb-8">
              <h1 className="text-4xl font-semibold leading-tight md:text-5xl">
                {customPage.title}
              </h1>
            </motion.header>
            <motion.section
              {...getThemeListItemMotion(1, 0.035)}
              className="custom-page-content prose prose-lg dark:prose-invert max-w-none"
            >
              <PluginSlot name="page_content_top" />
              <div
                className="page-content"
                dangerouslySetInnerHTML={{ __html: sanitizeHtml(customPage.html) }}
              />
              <PluginSlot name="page_content_bottom" />
            </motion.section>
          </motion.article>
        </main>
        <SiteFooter />
      </div>
    );
  }

  if (notFound || !profile) {
    return (
      <div className="theme-page-shell relative flex min-h-screen flex-col">
        <SiteHeader />
        <main className="flex-1">
          <div className="container mx-auto max-w-4xl py-16 text-center">
            <h1 className="mb-4 text-4xl font-semibold">{t("error.notFound")}</h1>
            <p className="mb-8 text-muted-foreground">{t("error.notFoundDesc")}</p>
            <Button asChild>
              <Link to="/">{t("error.backHome")}</Link>
            </Button>
          </div>
        </main>
        <SiteFooter />
      </div>
    );
  }

  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-5xl py-10">
          <motion.header {...themePageHeaderMotion} className="mb-8">
            <p className="mb-2 flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <UserRound className="h-4 w-4" />
              {t("about.title")}
            </p>
            <h1 className="text-3xl font-semibold md:text-4xl">
              {profile.displayName || siteName}
            </h1>
            {profile.headline ? (
              <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
                {profile.headline}
              </p>
            ) : null}
          </motion.header>

          <div className="grid gap-6 md:grid-cols-[18rem_minmax(0,1fr)]">
            <motion.aside {...getThemeListItemMotion(0, 0.035)} className="space-y-4">
              <Card>
                <CardContent className="p-5">
                  <div className="flex flex-col items-center text-center">
                    <div className="flex h-24 w-24 items-center justify-center overflow-hidden rounded-full border bg-muted">
                      {profile.avatar ? (
                        <img
                          src={profile.avatar}
                          alt=""
                          className="h-full w-full object-cover"
                          loading="lazy"
                        />
                      ) : (
                        <UserRound className="h-9 w-9 text-muted-foreground" />
                      )}
                    </div>
                    <h2 className="mt-4 text-lg font-semibold">
                      {profile.displayName || siteName}
                    </h2>
                    {profile.location ? (
                      <p className="mt-1 flex items-center justify-center gap-1.5 text-sm text-muted-foreground">
                        <MapPin className="h-3.5 w-3.5" />
                        {profile.location}
                      </p>
                    ) : null}
                  </div>

                  {profile.website || visibleSocialLinks.length > 0 ? (
                    <div className="mt-5 space-y-2 border-t pt-4">
                      {profile.website ? (
                        <a
                          href={profile.website}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                        >
                          <span className="truncate">{getHost(profile.website)}</span>
                          <ExternalLink className="h-3.5 w-3.5 shrink-0" />
                        </a>
                      ) : null}
                      {visibleSocialLinks.map((link, index) => (
                        <a
                          key={`${link.url}-${index}`}
                          href={link.url}
                          target="_blank"
                          rel="noopener noreferrer"
                          className="flex items-center justify-between gap-2 rounded-md px-2 py-1.5 text-sm text-muted-foreground transition-colors hover:bg-muted hover:text-foreground"
                        >
                          <span className="truncate">{link.label || getHost(link.url)}</span>
                          <ExternalLink className="h-3.5 w-3.5 shrink-0" />
                        </a>
                      ))}
                    </div>
                  ) : null}
                </CardContent>
              </Card>
            </motion.aside>

            <div className="space-y-6">
              {profile.bio ? (
                <motion.section {...getThemeListItemMotion(1, 0.035)}>
                  <Card>
                    <CardContent className="p-5">
                      <h2 className="mb-3 text-xl font-semibold">{t("about.bioTitle")}</h2>
                      <p className="whitespace-pre-line text-sm leading-7 text-muted-foreground">
                        {profile.bio}
                      </p>
                    </CardContent>
                  </Card>
                </motion.section>
              ) : null}

              {visibleTimeline.length > 0 ? (
                <motion.section {...getThemeListItemMotion(2, 0.035)}>
                  <Card>
                    <CardContent className="p-5">
                      <h2 className="mb-4 text-xl font-semibold">{t("about.timeline")}</h2>
                      <div className="space-y-4">
                        {visibleTimeline.map((item, index) => (
                          <motion.div
                            key={`${item.date}-${item.title}-${index}`}
                            className="relative border-l pl-4"
                            whileHover={themeHoverLift}
                          >
                            <span className="absolute -left-[5px] top-1.5 h-2.5 w-2.5 rounded-full bg-primary" />
                            {item.date ? (
                              <p className="text-xs font-medium text-primary">{item.date}</p>
                            ) : null}
                            {item.title ? (
                              <h3 className="mt-1 font-medium">{item.title}</h3>
                            ) : null}
                            {item.description ? (
                              <p className="mt-1 text-sm leading-6 text-muted-foreground">
                                {item.description}
                              </p>
                            ) : null}
                          </motion.div>
                        ))}
                      </div>
                    </CardContent>
                  </Card>
                </motion.section>
              ) : null}

              {profile.extraHtml ? (
                <motion.section
                  {...getThemeListItemMotion(3, 0.035)}
                  className="custom-page-content prose prose-lg dark:prose-invert max-w-none"
                >
                  <div
                    className="page-content"
                    dangerouslySetInnerHTML={{ __html: sanitizeHtml(profile.extraHtml) }}
                  />
                </motion.section>
              ) : null}
            </div>
          </div>
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
