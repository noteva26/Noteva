/**
 * Shared settings renderer for plugins and themes.
 * Renders a settings form from a PluginSettingsSchema + values.
 */
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import { Plus, X, List, GripVertical } from "lucide-react";
import type { PluginSettingsSchema, PluginSettingsField } from "@/lib/api";
import { useTranslation } from "@/lib/i18n";

// --- Value parsing ---

/** Parse raw settings values (strings from backend) into proper JS types */
export function parseSettingsValues(raw: Record<string, unknown>): Record<string, unknown> {
  const parsed: Record<string, unknown> = {};
  for (const [k, v] of Object.entries(raw)) {
    if (typeof v === "string") {
      if (v === "true") { parsed[k] = true; continue; }
      if (v === "false") { parsed[k] = false; continue; }
      if ((v.startsWith("[") && v.endsWith("]")) || (v.startsWith("{") && v.endsWith("}"))) {
        try { parsed[k] = JSON.parse(v); continue; } catch { /* keep as string */ }
      }
    }
    parsed[k] = v;
  }
  return parsed;
}

// --- Array field editor ---

interface ArrayFieldEditorProps {
  value: Record<string, unknown>[];
  onChange: (v: Record<string, unknown>[]) => void;
  itemFields: NonNullable<PluginSettingsField["itemFields"]>;
}

function ArrayFieldEditor({ value, onChange, itemFields }: ArrayFieldEditorProps) {
  const { t } = useTranslation();
  const items = Array.isArray(value) ? value : [];

  const addItem = () => {
    const newItem: Record<string, unknown> = {};
    itemFields.forEach(f => { newItem[f.id] = ""; });
    onChange([...items, newItem]);
  };

  const removeItem = (index: number) => onChange(items.filter((_, i) => i !== index));

  const updateItem = (index: number, fieldId: string, val: unknown) => {
    const next = [...items];
    next[index] = { ...next[index], [fieldId]: val };
    onChange(next);
  };

  const moveItem = (from: number, to: number) => {
    if (to < 0 || to >= items.length) return;
    const next = [...items];
    const [item] = next.splice(from, 1);
    next.splice(to, 0, item);
    onChange(next);
  };

  return (
    <div className="space-y-3">
      {items.map((item, index) => (
        <div key={index} className="border rounded-lg p-3 space-y-2 bg-muted/30">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <GripVertical className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-medium text-muted-foreground">#{index + 1}</span>
            </div>
            <div className="flex items-center gap-1">
              <Button type="button" variant="ghost" size="icon" className="h-7 w-7" onClick={() => moveItem(index, index - 1)} disabled={index === 0}>
                <span className="text-xs">↑</span>
              </Button>
              <Button type="button" variant="ghost" size="icon" className="h-7 w-7" onClick={() => moveItem(index, index + 1)} disabled={index === items.length - 1}>
                <span className="text-xs">↓</span>
              </Button>
              <Button type="button" variant="ghost" size="icon" className="h-7 w-7 text-destructive hover:text-destructive" onClick={() => removeItem(index)}>
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>
          <div className="grid gap-2" style={{ gridTemplateColumns: itemFields.length <= 2 ? `repeat(${itemFields.length}, 1fr)` : "repeat(2, 1fr)" }}>
            {itemFields.map(field => (
              <Input
                key={field.id}
                type={field.type === "number" ? "number" : "text"}
                placeholder={field.placeholder || field.label + (field.required ? " *" : "")}
                value={(item[field.id] as string) || ""}
                onChange={(e) => updateItem(index, field.id, field.type === "number" ? Number(e.target.value) : e.target.value)}
              />
            ))}
          </div>
        </div>
      ))}
      <Button type="button" variant="outline" className="w-full" onClick={addItem}>
        <Plus className="h-4 w-4 mr-2" />
        {t("common.addItem") || "Add Item"}
      </Button>
    </div>
  );
}

// --- Single field renderer ---

interface SettingsFieldProps {
  field: PluginSettingsField;
  value: unknown;
  onChange: (value: unknown) => void;
}

export function SettingsField({ field, value, onChange }: SettingsFieldProps) {
  const v = value ?? field.default ?? "";

  switch (field.type) {
    case "switch":
      return (
        <Switch
          id={field.id}
          checked={!!v}
          onCheckedChange={onChange}
        />
      );
    case "textarea":
      return (
        <Textarea
          id={field.id}
          value={String(v)}
          onChange={(e) => onChange(e.target.value)}
          rows={4}
        />
      );
    case "select":
      return (
        <Select value={String(v)} onValueChange={onChange}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {field.options?.map((opt) => (
              <SelectItem key={opt.value} value={opt.value}>{opt.label}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      );
    case "number":
      return (
        <Input
          id={field.id}
          type="number"
          value={String(v)}
          onChange={(e) => onChange(Number(e.target.value))}
          min={field.min}
          max={field.max}
        />
      );
    case "color":
      return (
        <div className="flex items-center gap-2">
          <Input
            type="color"
            value={String(v)}
            onChange={(e) => onChange(e.target.value)}
            className="w-12 h-10 p-1"
          />
          <Input
            value={String(v)}
            onChange={(e) => onChange(e.target.value)}
            className="flex-1"
          />
        </div>
      );
    case "array":
      return field.itemFields ? (
        <ArrayFieldEditor
          value={v as Record<string, unknown>[]}
          onChange={onChange as (v: Record<string, unknown>[]) => void}
          itemFields={field.itemFields}
        />
      ) : null;
    default:
      // text, image, radio, checkbox fallback to text input
      return (
        <Input
          id={field.id}
          type={field.secret ? "password" : "text"}
          value={String(v)}
          onChange={(e) => onChange(e.target.value)}
          placeholder={field.secret ? "••••••••" : undefined}
          onFocus={(e) => {
            if (field.secret && e.target.value === "••••••••") {
              onChange("");
            }
          }}
        />
      );
  }
}

// --- Full settings form ---

interface SettingsRendererProps {
  schema: PluginSettingsSchema | null;
  values: Record<string, unknown>;
  onChange: (values: Record<string, unknown>) => void;
  emptyMessage?: string;
}

/**
 * Renders a complete settings form with accordion sections.
 * Used by both plugin and theme settings panels.
 */
export function SettingsRenderer({ schema, values, onChange, emptyMessage }: SettingsRendererProps) {
  if (!schema?.sections?.length) {
    return (
      <p className="text-center text-muted-foreground py-8">
        {emptyMessage || "No settings available"}
      </p>
    );
  }

  const updateValue = (key: string, value: unknown) => {
    onChange({ ...values, [key]: value });
  };

  // Default open the first section
  const defaultOpen = schema.sections[0]?.id;

  return (
    <Accordion type="multiple" defaultValue={defaultOpen ? [defaultOpen] : []} className="w-full">
      {schema.sections.map((section) => (
        <AccordionItem key={section.id} value={section.id}>
          <AccordionTrigger className="text-sm">{section.title}</AccordionTrigger>
          <AccordionContent>
            <div className="space-y-4">
              {section.fields.map((field) => (
                <div key={field.id} className="space-y-2">
                  {field.type === "switch" ? (
                    <div className="flex items-center justify-between">
                      <div className="space-y-0.5">
                        <Label htmlFor={field.id}>{field.label}</Label>
                        {field.description && (
                          <p className="text-xs text-muted-foreground">{field.description}</p>
                        )}
                      </div>
                      <SettingsField
                        field={field}
                        value={values[field.id]}
                        onChange={(v) => updateValue(field.id, v)}
                      />
                    </div>
                  ) : (
                    <>
                      <Label htmlFor={field.id}>{field.label}</Label>
                      <SettingsField
                        field={field}
                        value={values[field.id]}
                        onChange={(v) => updateValue(field.id, v)}
                      />
                      {field.description && (
                        <p className="text-xs text-muted-foreground">{field.description}</p>
                      )}
                    </>
                  )}
                </div>
              ))}
            </div>
          </AccordionContent>
        </AccordionItem>
      ))}
    </Accordion>
  );
}
