"use client";

import { useEffect, useState } from "react";
import { pluginsApi, Plugin, PluginSettingsSchema } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Switch } from "@/components/ui/switch";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Textarea } from "@/components/ui/textarea";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Settings, Puzzle, Code, Save } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";

export default function PluginsPage() {
  const { t } = useTranslation();
  const [plugins, setPlugins] = useState<Plugin[]>([]);
  const [loading, setLoading] = useState(true);
  const [toggling, setToggling] = useState<string | null>(null);
  
  // Settings sheet state
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [selectedPlugin, setSelectedPlugin] = useState<Plugin | null>(null);
  const [schema, setSchema] = useState<PluginSettingsSchema | null>(null);
  const [values, setValues] = useState<Record<string, unknown>>({});
  const [saving, setSaving] = useState(false);

  const fetchPlugins = async () => {
    setLoading(true);
    try {
      const { data } = await pluginsApi.list();
      setPlugins(data?.plugins || []);
    } catch (error) {
      toast.error(t("error.loadFailed"));
      setPlugins([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchPlugins();
  }, []);

  const handleToggle = async (plugin: Plugin) => {
    setToggling(plugin.id);
    try {
      await pluginsApi.toggle(plugin.id, !plugin.enabled);
      toast.success(plugin.enabled ? t("plugin.disableSuccess") : t("plugin.enableSuccess"));
      fetchPlugins();
    } catch (error) {
      toast.error(t("plugin.toggleFailed"));
    } finally {
      setToggling(null);
    }
  };

  const openSettings = async (plugin: Plugin) => {
    setSelectedPlugin(plugin);
    setSettingsOpen(true);
    try {
      const { data } = await pluginsApi.getSettings(plugin.id);
      setSchema(data?.schema || null);
      setValues(data?.values || {});
    } catch (error) {
      toast.error(t("error.loadFailed"));
    }
  };

  const handleSaveSettings = async () => {
    if (!selectedPlugin) return;
    setSaving(true);
    try {
      await pluginsApi.updateSettings(selectedPlugin.id, values);
      toast.success(t("plugin.saveSuccess"));
      setSettingsOpen(false);
    } catch (error) {
      toast.error(t("plugin.saveFailed"));
    } finally {
      setSaving(false);
    }
  };

  const updateValue = (key: string, value: unknown) => {
    setValues((prev) => ({ ...prev, [key]: value }));
  };

  const renderField = (field: PluginSettingsSchema["sections"][0]["fields"][0]) => {
    const value = values[field.id] ?? field.default ?? "";

    switch (field.type) {
      case "text":
        return (
          <Input
            id={field.id}
            value={value as string}
            onChange={(e) => updateValue(field.id, e.target.value)}
          />
        );
      case "textarea":
        return (
          <Textarea
            id={field.id}
            value={value as string}
            onChange={(e) => updateValue(field.id, e.target.value)}
            rows={4}
          />
        );
      case "number":
        return (
          <Input
            id={field.id}
            type="number"
            value={value as number}
            min={field.min}
            max={field.max}
            onChange={(e) => updateValue(field.id, Number(e.target.value))}
          />
        );
      case "switch":
        return (
          <Switch
            id={field.id}
            checked={value as boolean}
            onCheckedChange={(checked) => updateValue(field.id, checked)}
          />
        );
      case "select":
        return (
          <Select value={value as string} onValueChange={(v) => updateValue(field.id, v)}>
            <SelectTrigger>
              <SelectValue />
            </SelectTrigger>
            <SelectContent>
              {field.options?.map((opt) => (
                <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
              ))}
            </SelectContent>
          </Select>
        );
      case "color":
        return (
          <div className="flex items-center gap-2">
            <Input
              type="color"
              value={value as string}
              onChange={(e) => updateValue(field.id, e.target.value)}
              className="w-12 h-10 p-1"
            />
            <Input
              value={value as string}
              onChange={(e) => updateValue(field.id, e.target.value)}
              className="flex-1"
            />
          </div>
        );
      default:
        return (
          <Input
            id={field.id}
            value={value as string}
            onChange={(e) => updateValue(field.id, e.target.value)}
          />
        );
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold">{t("plugin.title")}</h1>
        <p className="text-muted-foreground">{t("plugin.description")}</p>
      </div>

      {loading ? (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {Array.from({ length: 3 }).map((_, i) => (
            <Card key={i}>
              <CardHeader>
                <Skeleton className="h-5 w-32" />
                <Skeleton className="h-4 w-48" />
              </CardHeader>
              <CardContent>
                <Skeleton className="h-4 w-full" />
              </CardContent>
            </Card>
          ))}
        </div>
      ) : plugins.length === 0 ? (
        <Card>
          <CardContent className="py-12 text-center">
            <Puzzle className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
            <p className="text-muted-foreground">{t("plugin.noPlugins")}</p>
          </CardContent>
        </Card>
      ) : (
        <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
          {plugins.map((plugin) => (
            <Card key={plugin.id} className={!plugin.enabled ? "opacity-60" : ""}>
              <CardHeader className="pb-3">
                <div className="flex items-start justify-between">
                  <div className="space-y-1">
                    <CardTitle className="text-lg flex items-center gap-2">
                      <Puzzle className="h-4 w-4" />
                      {plugin.name}
                    </CardTitle>
                    <CardDescription className="text-xs">
                      {t("plugin.version")}: {plugin.version}
                      {plugin.author && ` · ${t("plugin.author")}: ${plugin.author}`}
                    </CardDescription>
                  </div>
                  <Switch
                    checked={plugin.enabled}
                    onCheckedChange={() => handleToggle(plugin)}
                    disabled={toggling === plugin.id}
                  />
                </div>
              </CardHeader>
              <CardContent className="space-y-4">
                <p className="text-sm text-muted-foreground">
                  {plugin.description || "No description"}
                </p>
                
                {plugin.shortcodes.length > 0 && (
                  <div className="flex flex-wrap gap-1">
                    <Code className="h-4 w-4 text-muted-foreground mr-1" />
                    {plugin.shortcodes.map((sc) => (
                      <Badge key={sc} variant="secondary" className="text-xs">[{sc}]</Badge>
                    ))}
                  </div>
                )}

                <div className="flex items-center justify-between pt-2 border-t">
                  <Badge variant={plugin.enabled ? "success" : "secondary"}>
                    {plugin.enabled ? t("plugin.enabled") : t("plugin.disabled")}
                  </Badge>
                  {plugin.has_settings && (
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => openSettings(plugin)}
                      disabled={!plugin.enabled}
                    >
                      <Settings className="h-4 w-4 mr-1" />
                      {t("plugin.settings")}
                    </Button>
                  )}
                </div>
              </CardContent>
            </Card>
          ))}
        </div>
      )}

      {/* Settings Dialog */}
      <Dialog open={settingsOpen} onOpenChange={setSettingsOpen}>
        <DialogContent className="max-h-[80vh] overflow-y-auto">
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <Puzzle className="h-5 w-5" />
              {selectedPlugin?.name}
            </DialogTitle>
            <DialogDescription>{t("plugin.settingsTitle")}</DialogDescription>
          </DialogHeader>
          
          <div className="mt-4 space-y-6">
            {schema?.sections?.length ? (
              <>
                {schema.sections.map((section) => (
                  <div key={section.id} className="space-y-4">
                    <h3 className="font-medium">{section.title}</h3>
                    {section.fields.map((field) => (
                      <div key={field.id} className="space-y-2">
                        <div className="flex items-center justify-between">
                          <Label htmlFor={field.id}>{field.label}</Label>
                          {field.type === "switch" && renderField(field)}
                        </div>
                        {field.type !== "switch" && renderField(field)}
                        {field.description && (
                          <p className="text-xs text-muted-foreground">{field.description}</p>
                        )}
                      </div>
                    ))}
                  </div>
                ))}
                <Button onClick={handleSaveSettings} disabled={saving} className="w-full">
                  <Save className="h-4 w-4 mr-2" />
                  {t("plugin.saveSettings")}
                </Button>
              </>
            ) : (
              <p className="text-center text-muted-foreground py-8">{t("plugin.noSettings")}</p>
            )}
          </div>
        </DialogContent>
      </Dialog>
    </div>
  );
}
