import { useEffect, useState, useTransition } from "react";
import {
  ExternalLink,
  Loader2,
  MapPin,
  Plus,
  Save,
  Trash2,
  UserRound,
} from "lucide-react";
import { toast } from "sonner";
import { AdminPageHeader } from "@/components/admin/page-header";
import { DataSyncBadge } from "@/components/admin/data-sync-bar";
import { aboutApi, type AboutProfile } from "@/lib/api";
import { getApiErrorMessage } from "@/lib/api-error";
import { useTranslation } from "@/lib/i18n";
import { AvatarUpload } from "@/components/ui/avatar-upload";
import { Button } from "@/components/ui/button";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Skeleton } from "@/components/ui/skeleton";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";

const emptyProfile: AboutProfile = {
  enabled: false,
  nav_enabled: false,
  display_name: "",
  avatar: "",
  headline: "",
  bio: "",
  location: "",
  website: "",
  social_links: [],
  timeline: [],
  extra_markdown: "",
};

function normalizeProfile(profile?: Partial<AboutProfile>): AboutProfile {
  return {
    ...emptyProfile,
    ...profile,
    social_links: profile?.social_links || [],
    timeline: profile?.timeline || [],
  };
}

export default function AboutManagePage() {
  const { t } = useTranslation();
  const [profile, setProfile] = useState<AboutProfile>(emptyProfile);
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [saving, startSavingTransition] = useTransition();

  useEffect(() => {
    let active = true;

    const load = async () => {
      try {
        setLoading(true);
        const { data } = await aboutApi.get();
        if (!active) return;
        setProfile(normalizeProfile(data.profile));
      } catch (error) {
        if (active) toast.error(getApiErrorMessage(error, t("about.loadFailed")));
      } finally {
        if (active) {
          setLoading(false);
          setHasLoaded(true);
        }
      }
    };

    void load();

    return () => {
      active = false;
    };
  }, [t]);

  const updateField = <K extends keyof AboutProfile>(key: K, value: AboutProfile[K]) => {
    setProfile((current) => ({ ...current, [key]: value }));
  };

  const addSocialLink = () => {
    setProfile((current) => ({
      ...current,
      social_links: [...current.social_links, { label: "", url: "", icon: "" }],
    }));
  };

  const updateSocialLink = (
    index: number,
    key: "label" | "url" | "icon",
    value: string
  ) => {
    setProfile((current) => ({
      ...current,
      social_links: current.social_links.map((link, itemIndex) =>
        itemIndex === index ? { ...link, [key]: value } : link
      ),
    }));
  };

  const removeSocialLink = (index: number) => {
    setProfile((current) => ({
      ...current,
      social_links: current.social_links.filter((_, itemIndex) => itemIndex !== index),
    }));
  };

  const addTimelineItem = () => {
    setProfile((current) => ({
      ...current,
      timeline: [...current.timeline, { title: "", date: "", description: "" }],
    }));
  };

  const updateTimelineItem = (
    index: number,
    key: "title" | "date" | "description",
    value: string
  ) => {
    setProfile((current) => ({
      ...current,
      timeline: current.timeline.map((item, itemIndex) =>
        itemIndex === index ? { ...item, [key]: value } : item
      ),
    }));
  };

  const removeTimelineItem = (index: number) => {
    setProfile((current) => ({
      ...current,
      timeline: current.timeline.filter((_, itemIndex) => itemIndex !== index),
    }));
  };

  const handleSave = () => {
    startSavingTransition(async () => {
      try {
        const { data } = await aboutApi.update(profile);
        setProfile(normalizeProfile(data.profile));
        toast.success(t("about.saveSuccess"));
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("about.saveFailed")));
      }
    });
  };

  const showInitialLoading = loading && !hasLoaded;

  return (
    <div className="space-y-6">
      <AdminPageHeader
        title={t("about.manageTitle")}
        description={t("about.manageDescription")}
        actions={
          <Button onClick={handleSave} disabled={saving || showInitialLoading}>
            {saving ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Save className="mr-2 h-4 w-4" />
            )}
            {t("common.save")}
          </Button>
        }
      />
      <DataSyncBadge active={loading && hasLoaded} label={t("common.loading")} />

      {showInitialLoading ? (
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_22rem]">
          <Skeleton className="h-96 rounded-lg" />
          <Skeleton className="h-72 rounded-lg" />
        </div>
      ) : (
        <div className="grid gap-4 lg:grid-cols-[minmax(0,1fr)_22rem]">
          <div className="space-y-4">
            <Card>
              <CardHeader>
                <CardTitle>{t("about.basicInfo")}</CardTitle>
                <CardDescription>{t("about.basicInfoDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="flex items-center justify-between gap-4 rounded-lg border p-4">
                    <div className="space-y-1">
                      <Label>{t("about.enabled")}</Label>
                      <p className="text-sm text-muted-foreground">{t("about.enabledHint")}</p>
                    </div>
                    <Switch
                      checked={profile.enabled}
                      onCheckedChange={(value) => updateField("enabled", value)}
                    />
                  </div>
                  <div className="flex items-center justify-between gap-4 rounded-lg border p-4">
                    <div className="space-y-1">
                      <Label>{t("about.navEnabled")}</Label>
                      <p className="text-sm text-muted-foreground">{t("about.navEnabledHint")}</p>
                    </div>
                    <Switch
                      checked={profile.nav_enabled}
                      onCheckedChange={(value) => updateField("nav_enabled", value)}
                      disabled={!profile.enabled}
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <Label>{t("about.avatar")}</Label>
                  <AvatarUpload
                    value={profile.avatar}
                    onChange={(value) => updateField("avatar", value)}
                  />
                </div>

                <div className="grid gap-4 sm:grid-cols-2">
                  <div className="space-y-2">
                    <Label>{t("about.displayName")}</Label>
                    <Input
                      value={profile.display_name}
                      onChange={(event) => updateField("display_name", event.target.value)}
                      placeholder="Noteva"
                    />
                  </div>
                  <div className="space-y-2">
                    <Label>{t("about.location")}</Label>
                    <Input
                      value={profile.location}
                      onChange={(event) => updateField("location", event.target.value)}
                      placeholder={t("about.locationPlaceholder")}
                    />
                  </div>
                </div>

                <div className="space-y-2">
                  <Label>{t("about.headline")}</Label>
                  <Input
                    value={profile.headline}
                    onChange={(event) => updateField("headline", event.target.value)}
                    placeholder={t("about.headlinePlaceholder")}
                  />
                </div>

                <div className="space-y-2">
                  <Label>{t("about.bio")}</Label>
                  <Textarea
                    value={profile.bio}
                    onChange={(event) => updateField("bio", event.target.value)}
                    rows={4}
                    placeholder={t("about.bioPlaceholder")}
                  />
                </div>

                <div className="space-y-2">
                  <Label>{t("about.website")}</Label>
                  <Input
                    value={profile.website}
                    onChange={(event) => updateField("website", event.target.value)}
                    placeholder="https://example.com"
                  />
                </div>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>{t("about.socialLinks")}</CardTitle>
                <CardDescription>{t("about.socialLinksDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                {profile.social_links.map((link, index) => (
                  <div key={index} className="grid gap-2 rounded-lg border p-3 sm:grid-cols-[1fr_1.5fr_0.8fr_auto]">
                    <Input
                      value={link.label}
                      onChange={(event) => updateSocialLink(index, "label", event.target.value)}
                      placeholder={t("about.socialLabel")}
                    />
                    <Input
                      value={link.url}
                      onChange={(event) => updateSocialLink(index, "url", event.target.value)}
                      placeholder="https://"
                    />
                    <Input
                      value={link.icon}
                      onChange={(event) => updateSocialLink(index, "icon", event.target.value)}
                      placeholder={t("about.socialIcon")}
                    />
                    <Button
                      type="button"
                      variant="ghost"
                      size="icon"
                      onClick={() => removeSocialLink(index)}
                      aria-label={t("common.delete")}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                ))}
                <Button type="button" variant="outline" onClick={addSocialLink}>
                  <Plus className="mr-2 h-4 w-4" />
                  {t("about.addSocialLink")}
                </Button>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>{t("about.timeline")}</CardTitle>
                <CardDescription>{t("about.timelineDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-3">
                {profile.timeline.map((item, index) => (
                  <div key={index} className="space-y-2 rounded-lg border p-3">
                    <div className="grid gap-2 sm:grid-cols-[0.85fr_1fr_auto]">
                      <Input
                        value={item.date}
                        onChange={(event) =>
                          updateTimelineItem(index, "date", event.target.value)
                        }
                        placeholder="2026"
                      />
                      <Input
                        value={item.title}
                        onChange={(event) =>
                          updateTimelineItem(index, "title", event.target.value)
                        }
                        placeholder={t("about.timelineTitle")}
                      />
                      <Button
                        type="button"
                        variant="ghost"
                        size="icon"
                        onClick={() => removeTimelineItem(index)}
                        aria-label={t("common.delete")}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </div>
                    <Textarea
                      value={item.description}
                      onChange={(event) =>
                        updateTimelineItem(index, "description", event.target.value)
                      }
                      rows={2}
                      placeholder={t("about.timelineDescription")}
                    />
                  </div>
                ))}
                <Button type="button" variant="outline" onClick={addTimelineItem}>
                  <Plus className="mr-2 h-4 w-4" />
                  {t("about.addTimelineItem")}
                </Button>
              </CardContent>
            </Card>

            <Card>
              <CardHeader>
                <CardTitle>{t("about.extraMarkdown")}</CardTitle>
                <CardDescription>{t("about.extraMarkdownDesc")}</CardDescription>
              </CardHeader>
              <CardContent>
                <Textarea
                  value={profile.extra_markdown}
                  onChange={(event) => updateField("extra_markdown", event.target.value)}
                  rows={8}
                  placeholder={t("about.extraMarkdownPlaceholder")}
                />
              </CardContent>
            </Card>
          </div>

          <aside className="space-y-4 lg:sticky lg:top-6 lg:self-start">
            <Card>
              <CardHeader>
                <CardTitle>{t("common.preview")}</CardTitle>
                <CardDescription>{t("about.previewDesc")}</CardDescription>
              </CardHeader>
              <CardContent className="space-y-4">
                <div className="flex items-center gap-3">
                  <div className="flex h-14 w-14 items-center justify-center overflow-hidden rounded-full border bg-muted">
                    {profile.avatar ? (
                      <img src={profile.avatar} alt="" className="h-full w-full object-cover" />
                    ) : (
                      <UserRound className="h-6 w-6 text-muted-foreground" />
                    )}
                  </div>
                  <div className="min-w-0">
                    <p className="truncate font-medium">
                      {profile.display_name || t("about.displayName")}
                    </p>
                    <p className="truncate text-sm text-muted-foreground">
                      {profile.headline || t("about.headlinePlaceholder")}
                    </p>
                  </div>
                </div>

                {profile.bio ? (
                  <p className="text-sm leading-6 text-muted-foreground">{profile.bio}</p>
                ) : null}

                <div className="space-y-2 text-sm text-muted-foreground">
                  {profile.location ? (
                    <p className="flex items-center gap-2">
                      <MapPin className="h-4 w-4" />
                      {profile.location}
                    </p>
                  ) : null}
                  {profile.website ? (
                    <a
                      href={profile.website}
                      target="_blank"
                      rel="noopener noreferrer"
                      className="flex items-center gap-2 hover:text-foreground"
                    >
                      <ExternalLink className="h-4 w-4" />
                      {profile.website}
                    </a>
                  ) : null}
                </div>

                <div className="rounded-lg border bg-muted/30 p-3 text-xs text-muted-foreground">
                  {profile.enabled
                    ? profile.nav_enabled
                      ? t("about.previewEnabledWithNav")
                      : t("about.previewEnabled")
                    : t("about.previewDisabled")}
                </div>
              </CardContent>
            </Card>
          </aside>
        </div>
      )}
    </div>
  );
}
