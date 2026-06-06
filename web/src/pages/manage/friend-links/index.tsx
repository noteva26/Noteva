import { useEffect, useMemo, useState, useTransition } from "react";
import { ExternalLink, Eye, EyeOff, Pencil, Plus, Trash2, Upload } from "lucide-react";
import { toast } from "sonner";
import { AdminPageHeader } from "@/components/admin/page-header";
import { DataSyncBadge, DataSyncBar } from "@/components/admin/data-sync-bar";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Card, CardContent } from "@/components/ui/card";
import {
  Dialog,
  DialogContent,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Textarea } from "@/components/ui/textarea";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from "@/components/ui/alert-dialog";
import { api } from "@/lib/api";
import { getApiErrorMessage } from "@/lib/api-error";
import { formatDateTime } from "@/lib/format";
import { useTranslation } from "@/lib/i18n";

interface FriendLink {
  id: number;
  name: string;
  url: string;
  logo?: string | null;
  description?: string | null;
  category?: string | null;
  sort_order: number;
  status: "pending" | "approved" | "rejected" | "hidden" | string;
  is_recommended: boolean;
  created_at: string;
  updated_at: string;
}

interface FriendLinkForm {
  name: string;
  url: string;
  logo: string;
  description: string;
  category: string;
  sort_order: number;
  status: string;
  is_recommended: boolean;
}

const CATEGORY_ALL = "__all__";

const emptyForm: FriendLinkForm = {
  name: "",
  url: "",
  logo: "",
  description: "",
  category: "",
  sort_order: 0,
  status: "approved",
  is_recommended: false,
};

function getInitials(name: string) {
  const chars = Array.from(name.trim());
  return chars.slice(0, 2).join("").toUpperCase() || "?";
}

function getHostname(url: string) {
  try {
    return new URL(url).hostname.replace(/^www\./, "");
  } catch {
    return url;
  }
}

function statusVariant(status: string): "default" | "secondary" | "destructive" | "outline" {
  if (status === "approved") return "default";
  if (status === "hidden") return "secondary";
  if (status === "rejected") return "destructive";
  return "outline";
}

function readToggleSetting(value: unknown, defaultValue = true) {
  if (typeof value !== "string") return defaultValue;
  const normalized = value.trim().toLowerCase();
  return !["false", "0", "no", "off"].includes(normalized);
}

export default function FriendLinksManagePage() {
  const { t, locale } = useTranslation();
  const [links, setLinks] = useState<FriendLink[]>([]);
  const [loading, setLoading] = useState(true);
  const [hasLoaded, setHasLoaded] = useState(false);
  const [isMutating, startMutationTransition] = useTransition();
  const [categoryFilter, setCategoryFilter] = useState(CATEGORY_ALL);
  const [dialogOpen, setDialogOpen] = useState(false);
  const [deleteDialogOpen, setDeleteDialogOpen] = useState(false);
  const [editingLink, setEditingLink] = useState<FriendLink | null>(null);
  const [deletingLink, setDeletingLink] = useState<FriendLink | null>(null);
  const [form, setForm] = useState<FriendLinkForm>(emptyForm);
  const [navEnabled, setNavEnabled] = useState(true);

  const loadLinks = async () => {
    try {
      setLoading(true);
      const [linksRes, settingsRes] = await Promise.all([
        api.get<{ links: FriendLink[] }>("/admin/friend-links"),
        api.get<{ friend_links_nav_enabled?: string }>("/admin/settings").catch(() => null),
      ]);
      setLinks(linksRes.data.links || []);
      setNavEnabled(readToggleSetting(settingsRes?.data.friend_links_nav_enabled));
    } catch (error) {
      toast.error(getApiErrorMessage(error, t("friendLinks.loadFailed")));
    } finally {
      setLoading(false);
      setHasLoaded(true);
    }
  };

  useEffect(() => {
    void loadLinks();
  }, []);

  const categories = useMemo(() => {
    const values = links
      .map((link) => link.category?.trim())
      .filter((value): value is string => Boolean(value));
    return Array.from(new Set(values)).sort((a, b) => a.localeCompare(b));
  }, [links]);

  const filteredLinks = useMemo(() => {
    const list =
      categoryFilter === CATEGORY_ALL
        ? links
        : links.filter((link) => (link.category || "") === categoryFilter);
    return [...list].sort(
      (a, b) =>
        (a.category || "").localeCompare(b.category || "") ||
        a.sort_order - b.sort_order ||
        a.name.localeCompare(b.name)
    );
  }, [categoryFilter, links]);

  const openCreateDialog = () => {
    setEditingLink(null);
    const nextSort =
      links.length === 0 ? 0 : Math.max(...links.map((link) => link.sort_order)) + 1;
    setForm({ ...emptyForm, sort_order: nextSort });
    setDialogOpen(true);
  };

  const openEditDialog = (link: FriendLink) => {
    setEditingLink(link);
    setForm({
      name: link.name,
      url: link.url,
      logo: link.logo || "",
      description: link.description || "",
      category: link.category || "",
      sort_order: link.sort_order,
      status: link.status,
      is_recommended: link.is_recommended,
    });
    setDialogOpen(true);
  };

  const buildPayload = () => ({
    name: form.name,
    url: form.url,
    logo: form.logo.trim() ? form.logo : null,
    description: form.description.trim() ? form.description : null,
    category: form.category.trim() ? form.category : null,
    sort_order: Number(form.sort_order) || 0,
    status: form.status,
    is_recommended: form.is_recommended,
  });

  const handleSubmit = () => {
    if (!form.name.trim() || !form.url.trim()) {
      toast.error(t("friendLinks.fillRequired"));
      return;
    }

    startMutationTransition(async () => {
      try {
        const payload = buildPayload();
        if (editingLink) {
          const { data } = await api.put<{ link: FriendLink }>(
            `/admin/friend-links/${editingLink.id}`,
            payload
          );
          setLinks((current) =>
            current.map((link) => (link.id === editingLink.id ? data.link : link))
          );
          toast.success(t("friendLinks.updateSuccess"));
        } else {
          const { data } = await api.post<{ link: FriendLink }>("/admin/friend-links", payload);
          setLinks((current) => [...current, data.link]);
          toast.success(t("friendLinks.createSuccess"));
        }
        setDialogOpen(false);
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("friendLinks.saveFailed")));
      }
    });
  };

  const handleDelete = () => {
    if (!deletingLink) return;
    const target = deletingLink;

    startMutationTransition(async () => {
      try {
        await api.delete(`/admin/friend-links/${target.id}`);
        setLinks((current) => current.filter((link) => link.id !== target.id));
        setDeleteDialogOpen(false);
        setDeletingLink(null);
        toast.success(t("friendLinks.deleteSuccess"));
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("friendLinks.deleteFailed")));
      }
    });
  };

  const handleImportLegacy = () => {
    startMutationTransition(async () => {
      try {
        const { data } = await api.post<{
          imported: number;
          skipped: number;
          links: FriendLink[];
        }>("/admin/friend-links/import/friendlinks-plugin");

        if (data.imported > 0) {
          setLinks((current) => [...current, ...data.links]);
          toast.success(
            t("friendLinks.importSuccess", {
              count: data.imported,
              skipped: data.skipped,
            })
          );
        } else {
          toast.info(t("friendLinks.importNothing", { skipped: data.skipped }));
        }
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("friendLinks.importFailed")));
      }
    });
  };

  const handleNavEnabledChange = (checked: boolean) => {
    const previous = navEnabled;
    setNavEnabled(checked);
    startMutationTransition(async () => {
      try {
        await api.put("/admin/settings", {
          friend_links_nav_enabled: checked ? "true" : "false",
        });
        toast.success(t("friendLinks.navUpdated"));
      } catch (error) {
        setNavEnabled(previous);
        toast.error(getApiErrorMessage(error, t("friendLinks.updateFailed")));
      }
    });
  };

  const toggleVisible = (link: FriendLink) => {
    const status = link.status === "hidden" ? "approved" : "hidden";
    startMutationTransition(async () => {
      try {
        const { data } = await api.put<{ link: FriendLink }>(`/admin/friend-links/${link.id}`, {
          status,
        });
        setLinks((current) =>
          current.map((item) => (item.id === link.id ? data.link : item))
        );
      } catch (error) {
        toast.error(getApiErrorMessage(error, t("friendLinks.updateFailed")));
      }
    });
  };

  const showInitialLoading = loading && !hasLoaded;
  const isSyncing = loading && hasLoaded;

  return (
    <div className="space-y-6">
      <AdminPageHeader
        title={t("friendLinks.manageTitle")}
        description={t("friendLinks.manageDescription")}
        actions={
          <div className="flex flex-wrap items-center gap-2">
            <Button variant="outline" onClick={handleImportLegacy} disabled={isMutating}>
              <Upload className="mr-2 h-4 w-4" />
              {t("friendLinks.importLegacy")}
            </Button>
            <Button onClick={openCreateDialog}>
              <Plus className="mr-2 h-4 w-4" />
              {t("friendLinks.add")}
            </Button>
          </div>
        }
      />
      <DataSyncBadge active={isSyncing} label={t("common.loading")} />

      <Card>
        <CardContent className="flex flex-col gap-3 p-4 sm:flex-row sm:items-center sm:justify-between">
          <div className="space-y-1">
            <Label>{t("friendLinks.navEnabled")}</Label>
            <p className="text-sm text-muted-foreground">{t("friendLinks.navEnabledHint")}</p>
          </div>
          <Switch
            checked={navEnabled}
            onCheckedChange={handleNavEnabledChange}
            disabled={isMutating || showInitialLoading}
          />
        </CardContent>
      </Card>

      <Card>
        <CardContent className="p-0">
          <div className="flex flex-col gap-3 border-b p-4 sm:flex-row sm:items-center sm:justify-between">
            <DataSyncBar active={isSyncing} label={t("common.loading")} className="sm:max-w-xs" />
            <div className="flex items-center gap-2">
              <Label className="text-sm text-muted-foreground">{t("friendLinks.category")}</Label>
              <Select value={categoryFilter} onValueChange={setCategoryFilter}>
                <SelectTrigger className="w-44">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value={CATEGORY_ALL}>{t("common.all")}</SelectItem>
                  {categories.map((category) => (
                    <SelectItem key={category} value={category}>
                      {category}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
          </div>

          {showInitialLoading ? (
            <div className="p-6 text-sm text-muted-foreground">{t("common.loading")}</div>
          ) : (
            <Table className={isSyncing ? "opacity-70 transition-opacity" : undefined}>
              <TableHeader>
                <TableRow>
                  <TableHead>{t("friendLinks.site")}</TableHead>
                  <TableHead>{t("friendLinks.category")}</TableHead>
                  <TableHead>{t("friendLinks.status")}</TableHead>
                  <TableHead>{t("friendLinks.sortOrder")}</TableHead>
                  <TableHead>{t("friendLinks.updatedAt")}</TableHead>
                  <TableHead className="text-right">{t("common.edit")}</TableHead>
                </TableRow>
              </TableHeader>
              <TableBody>
                {filteredLinks.map((link) => (
                  <TableRow key={link.id}>
                    <TableCell>
                      <div className="flex min-w-0 items-center gap-3">
                        <Avatar className="h-10 w-10 rounded-lg border">
                          {link.logo ? <AvatarImage src={link.logo} alt="" /> : null}
                          <AvatarFallback className="rounded-lg text-xs">
                            {getInitials(link.name)}
                          </AvatarFallback>
                        </Avatar>
                        <div className="min-w-0">
                          <div className="flex items-center gap-2">
                            <span className="truncate font-medium">{link.name}</span>
                            {link.is_recommended ? (
                              <Badge variant="secondary">{t("friendLinks.recommended")}</Badge>
                            ) : null}
                          </div>
                          <a
                            href={link.url}
                            target="_blank"
                            rel="noopener noreferrer"
                            className="mt-0.5 inline-flex max-w-xs items-center gap-1 truncate text-xs text-muted-foreground hover:text-foreground"
                          >
                            {getHostname(link.url)}
                            <ExternalLink className="h-3 w-3 shrink-0" />
                          </a>
                        </div>
                      </div>
                    </TableCell>
                    <TableCell>{link.category || t("friendLinks.uncategorized")}</TableCell>
                    <TableCell>
                      <Badge variant={statusVariant(link.status)}>
                        {t(`friendLinks.status_${link.status}`)}
                      </Badge>
                    </TableCell>
                    <TableCell>{link.sort_order}</TableCell>
                    <TableCell>{formatDateTime(link.updated_at, locale)}</TableCell>
                    <TableCell className="text-right">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => toggleVisible(link)}
                        disabled={isMutating}
                        title={link.status === "hidden" ? t("common.show") : t("common.hide")}
                      >
                        {link.status === "hidden" ? (
                          <EyeOff className="h-4 w-4" />
                        ) : (
                          <Eye className="h-4 w-4" />
                        )}
                      </Button>
                      <Button variant="ghost" size="icon" onClick={() => openEditDialog(link)}>
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => {
                          setDeletingLink(link);
                          setDeleteDialogOpen(true);
                        }}
                      >
                        <Trash2 className="h-4 w-4" />
                      </Button>
                    </TableCell>
                  </TableRow>
                ))}
                {filteredLinks.length === 0 ? (
                  <TableRow>
                    <TableCell colSpan={6} className="h-32 text-center text-muted-foreground">
                      {t("friendLinks.noLinks")}
                    </TableCell>
                  </TableRow>
                ) : null}
              </TableBody>
            </Table>
          )}
        </CardContent>
      </Card>

      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="sm:max-w-2xl">
          <DialogHeader>
            <DialogTitle>
              {editingLink ? t("friendLinks.edit") : t("friendLinks.add")}
            </DialogTitle>
          </DialogHeader>

          <div className="grid gap-4 sm:grid-cols-2">
            <div className="space-y-2">
              <Label>{t("friendLinks.name")}</Label>
              <Input
                value={form.name}
                onChange={(event) => setForm({ ...form, name: event.target.value })}
                placeholder="Noteva"
              />
            </div>
            <div className="space-y-2">
              <Label>{t("friendLinks.url")}</Label>
              <Input
                value={form.url}
                onChange={(event) => setForm({ ...form, url: event.target.value })}
                placeholder="https://example.com"
              />
            </div>
            <div className="space-y-2">
              <Label>{t("friendLinks.logo")}</Label>
              <Input
                value={form.logo}
                onChange={(event) => setForm({ ...form, logo: event.target.value })}
                placeholder="https://example.com/logo.png"
              />
            </div>
            <div className="space-y-2">
              <Label>{t("friendLinks.category")}</Label>
              <Input
                value={form.category}
                onChange={(event) => setForm({ ...form, category: event.target.value })}
                placeholder={t("friendLinks.uncategorized")}
              />
            </div>
            <div className="space-y-2">
              <Label>{t("friendLinks.status")}</Label>
              <Select
                value={form.status}
                onValueChange={(status) => setForm({ ...form, status })}
              >
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="approved">{t("friendLinks.status_approved")}</SelectItem>
                  <SelectItem value="pending">{t("friendLinks.status_pending")}</SelectItem>
                  <SelectItem value="hidden">{t("friendLinks.status_hidden")}</SelectItem>
                  <SelectItem value="rejected">{t("friendLinks.status_rejected")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            <div className="space-y-2">
              <Label>{t("friendLinks.sortOrder")}</Label>
              <Input
                type="number"
                value={form.sort_order}
                onChange={(event) =>
                  setForm({ ...form, sort_order: Number(event.target.value) || 0 })
                }
              />
            </div>
            <div className="space-y-2 sm:col-span-2">
              <Label>{t("friendLinks.descriptionLabel")}</Label>
              <Textarea
                value={form.description}
                onChange={(event) => setForm({ ...form, description: event.target.value })}
                rows={3}
              />
            </div>
            <div className="flex items-center gap-2 sm:col-span-2">
              <Switch
                checked={form.is_recommended}
                onCheckedChange={(value) => setForm({ ...form, is_recommended: value })}
              />
              <Label>{t("friendLinks.recommended")}</Label>
            </div>
          </div>

          <DialogFooter>
            <Button variant="outline" onClick={() => setDialogOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button onClick={handleSubmit} disabled={isMutating}>
              {t("common.save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>

      <AlertDialog open={deleteDialogOpen} onOpenChange={setDeleteDialogOpen}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>{t("common.confirm")}</AlertDialogTitle>
            <AlertDialogDescription>
              {t("friendLinks.confirmDelete", { name: deletingLink?.name || "" })}
            </AlertDialogDescription>
          </AlertDialogHeader>
          <AlertDialogFooter>
            <AlertDialogCancel>{t("common.cancel")}</AlertDialogCancel>
            <AlertDialogAction onClick={handleDelete} disabled={isMutating}>
              {t("common.delete")}
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </div>
  );
}
