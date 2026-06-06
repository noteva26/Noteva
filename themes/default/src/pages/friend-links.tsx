import { useEffect, useMemo, useState } from "react";
import { motion } from "motion/react";
import { ExternalLink, Link2, Sparkles } from "lucide-react";
import { SiteFooter } from "@/components/site-footer";
import { SiteHeader } from "@/components/site-header";
import { Card, CardContent } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { waitForNoteva } from "@/hooks/useNoteva";
import { useTranslation } from "@/lib/i18n";
import {
  getThemeListItemMotion,
  themeHoverLift,
  themePageHeaderMotion,
} from "@/lib/motion";

type FriendLink = Awaited<ReturnType<NonNullable<typeof window.Noteva>["friendLinks"]["list"]>>[number];

const SKELETON_KEYS = ["friend-a", "friend-b", "friend-c", "friend-d", "friend-e", "friend-f"];

function getHostname(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

function getInitials(name: string) {
  const trimmed = name.trim();
  if (!trimmed) return "?";
  const chars = Array.from(trimmed);
  return chars.slice(0, 2).join("").toUpperCase();
}

function groupLinks(links: FriendLink[]) {
  const groups = new Map<string, FriendLink[]>();
  links.forEach((link) => {
    const category = link.category?.trim() || "default";
    if (!groups.has(category)) groups.set(category, []);
    groups.get(category)?.push(link);
  });

  return Array.from(groups.entries()).map(([category, items]) => ({
    category,
    links: [...items].sort((a, b) => {
      if (a.isRecommended !== b.isRecommended) return a.isRecommended ? -1 : 1;
      return (a.sortOrder ?? 0) - (b.sortOrder ?? 0) || a.name.localeCompare(b.name);
    }),
  }));
}

function FriendLinksSkeleton() {
  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-5xl py-10">
          <Skeleton className="mb-3 h-5 w-28" />
          <Skeleton className="mb-8 h-10 w-56" />
          <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
            {SKELETON_KEYS.map((key) => (
              <Skeleton key={key} className="h-32 rounded-lg" />
            ))}
          </div>
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}

export default function FriendLinksPage() {
  const { t } = useTranslation();
  const [links, setLinks] = useState<FriendLink[]>([]);
  const [siteName, setSiteName] = useState("Noteva");
  const [loading, setLoading] = useState(true);
  const [failedLogos, setFailedLogos] = useState<Set<number>>(() => new Set());

  useEffect(() => {
    let active = true;

    const load = async () => {
      setLoading(true);
      const noteva = await waitForNoteva();
      if (!active) return;

      if (!noteva) {
        setLinks([]);
        setLoading(false);
        return;
      }

      try {
        const [friendLinks, siteInfo] = await Promise.all([
          noteva.friendLinks.list(),
          noteva.site.getInfo().catch(() => null),
        ]);
        if (!active) return;
        setLinks(friendLinks);
        setSiteName(siteInfo?.name || "Noteva");
      } catch {
        if (active) setLinks([]);
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
    document.title = `${t("friendLinks.title")} - ${siteName}`;
  }, [siteName, t]);

  const groups = useMemo(() => groupLinks(links), [links]);

  if (loading) {
    return <FriendLinksSkeleton />;
  }

  return (
    <div className="theme-page-shell relative flex min-h-screen flex-col">
      <SiteHeader />
      <main className="flex-1">
        <div className="container mx-auto max-w-5xl py-10">
          <motion.header {...themePageHeaderMotion} className="mb-8">
            <p className="mb-2 flex items-center gap-2 text-sm font-medium text-muted-foreground">
              <Link2 className="h-4 w-4" />
              {t("friendLinks.count", { count: links.length })}
            </p>
            <h1 className="text-3xl font-semibold md:text-4xl">
              {t("friendLinks.title")}
            </h1>
            <p className="mt-3 max-w-2xl text-sm leading-6 text-muted-foreground">
              {t("friendLinks.description")}
            </p>
          </motion.header>

          {links.length === 0 ? (
            <Card className="border-dashed">
              <CardContent className="py-16 text-center text-muted-foreground">
                {t("friendLinks.empty")}
              </CardContent>
            </Card>
          ) : (
            <div className="space-y-10">
              {groups.map((group, groupIndex) => (
                <motion.section key={group.category} {...getThemeListItemMotion(groupIndex)}>
                  <div className="mb-4 flex items-center gap-2">
                    <h2 className="text-xl font-semibold">
                      {group.category === "default"
                        ? t("friendLinks.uncategorized")
                        : group.category}
                    </h2>
                    <span className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
                      {group.links.length}
                    </span>
                  </div>

                  <div className="grid gap-4 sm:grid-cols-2 lg:grid-cols-3">
                    {group.links.map((link, index) => (
                      <motion.a
                        key={link.id}
                        href={link.url}
                        target="_blank"
                        rel="noopener noreferrer"
                        className="group block rounded-lg border bg-card p-4 transition-colors hover:border-primary/60 hover:bg-muted/25"
                        {...getThemeListItemMotion(index, 0.025)}
                        whileHover={themeHoverLift}
                      >
                        <div className="flex items-start gap-3">
                          <div className="flex h-12 w-12 shrink-0 items-center justify-center overflow-hidden rounded-xl border bg-muted text-sm font-semibold">
                            {link.logo && !failedLogos.has(link.id) ? (
                              <img
                                src={link.logo}
                                alt=""
                                className="h-full w-full object-cover"
                                loading="lazy"
                                onError={() =>
                                  setFailedLogos((current) => {
                                    const next = new Set(current);
                                    next.add(link.id);
                                    return next;
                                  })
                                }
                              />
                            ) : (
                              getInitials(link.name)
                            )}
                          </div>

                          <div className="min-w-0 flex-1">
                            <div className="flex items-start gap-2">
                              <h3 className="min-w-0 flex-1 truncate font-semibold">
                                {link.name}
                              </h3>
                              {link.isRecommended ? (
                                <Sparkles className="mt-0.5 h-4 w-4 shrink-0 text-primary" />
                              ) : (
                                <ExternalLink className="mt-0.5 h-4 w-4 shrink-0 text-muted-foreground opacity-0 transition-opacity group-hover:opacity-100" />
                              )}
                            </div>
                            <p className="mt-0.5 truncate text-xs text-muted-foreground">
                              {getHostname(link.url)}
                            </p>
                            {link.description ? (
                              <p className="mt-2 line-clamp-2 text-sm leading-6 text-muted-foreground">
                                {link.description}
                              </p>
                            ) : null}
                          </div>
                        </div>
                      </motion.a>
                    ))}
                  </div>
                </motion.section>
              ))}
            </div>
          )}
        </div>
      </main>
      <SiteFooter />
    </div>
  );
}
