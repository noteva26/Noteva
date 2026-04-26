/**
 * Shared settings renderer for plugins and themes.
 * Renders a settings form from a PluginSettingsSchema plus values.
 */
import { useRef } from "react";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Switch } from "@/components/ui/switch";
import { Textarea } from "@/components/ui/textarea";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Button } from "@/components/ui/button";
import { Accordion, AccordionContent, AccordionItem, AccordionTrigger } from "@/components/ui/accordion";
import { ChevronDown, ChevronUp, Plus, X, GripVertical } from "lucide-react";
import type { PluginSettingsSchema, PluginSettingsField } from "@/lib/api";

function loc(value: string | Record<string, string> | undefined): string {
  if (!value) return "";
  if (typeof value === "string") return value;

  const lang = document.documentElement.lang || "en";
  return value[lang] || value[lang.split("-")[0]] || value.en || Object.values(value)[0] || "";
}

export function parseSettingsValues(raw: Record<string, unknown>): Record<string, unknown> {
  const parsed: Record<string, unknown> = {};

  for (const [key, value] of Object.entries(raw)) {
    if (typeof value === "string") {
      if (value === "true") {
        parsed[key] = true;
        continue;
      }
      if (value === "false") {
        parsed[key] = false;
        continue;
      }
      if ((value.startsWith("[") && value.endsWith("]")) || (value.startsWith("{") && value.endsWith("}"))) {
        try {
          parsed[key] = JSON.parse(value);
          continue;
        } catch {
          // Keep the original string if this only looks like JSON.
        }
      }
    }

    parsed[key] = value;
  }

  return parsed;
}

interface ArrayFieldEditorProps {
  value: Record<string, unknown>[];
  onChange: (value: Record<string, unknown>[]) => void;
  itemFields: NonNullable<PluginSettingsField["itemFields"]>;
}

const createArrayItemId = () =>
  typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `${Date.now()}-${Math.random()}`;

function moveArrayItem<T>(items: T[], from: number, to: number) {
  if (to < 0 || to >= items.length) return items;
  const item = items[from];
  if (!item) return items;

  const withoutItem = items.filter((_, index) => index !== from);
  return [
    ...withoutItem.slice(0, to),
    item,
    ...withoutItem.slice(to),
  ];
}

function ArrayFieldEditor({ value, onChange, itemFields }: ArrayFieldEditorProps) {
  const items = Array.isArray(value) ? value : [];
  const itemIdsRef = useRef<string[]>([]);

  while (itemIdsRef.current.length < items.length) {
    itemIdsRef.current.push(createArrayItemId());
  }
  if (itemIdsRef.current.length > items.length) {
    itemIdsRef.current = itemIdsRef.current.slice(0, items.length);
  }

  const addItem = () => {
    const newItem: Record<string, unknown> = {};
    itemFields.forEach((field) => {
      newItem[field.id] = "";
    });
    itemIdsRef.current = [...itemIdsRef.current, createArrayItemId()];
    onChange([...items, newItem]);
  };

  const removeItem = (index: number) => {
    itemIdsRef.current = itemIdsRef.current.filter((_, itemIndex) => itemIndex !== index);
    onChange(items.filter((_, itemIndex) => itemIndex !== index));
  };

  const updateItem = (index: number, fieldId: string, nextValue: unknown) => {
    onChange(
      items.map((item, itemIndex) =>
        itemIndex === index ? { ...item, [fieldId]: nextValue } : item
      )
    );
  };

  const moveItem = (from: number, to: number) => {
    if (to < 0 || to >= items.length) return;
    itemIdsRef.current = moveArrayItem(itemIdsRef.current, from, to);
    onChange(moveArrayItem(items, from, to));
  };

  return (
    <div className="space-y-3">
      {items.map((item, index) => (
        <div key={itemIdsRef.current[index]} className="border rounded-lg p-3 space-y-2 bg-muted/30">
          <div className="flex items-center justify-between">
            <div className="flex items-center gap-2">
              <GripVertical className="h-4 w-4 text-muted-foreground" />
              <span className="text-sm font-medium text-muted-foreground">#{index + 1}</span>
            </div>
            <div className="flex items-center gap-1">
              <Button type="button" variant="ghost" size="icon" className="h-7 w-7" onClick={() => moveItem(index, index - 1)} disabled={index === 0} title="Move up">
                <ChevronUp className="h-4 w-4" />
              </Button>
              <Button type="button" variant="ghost" size="icon" className="h-7 w-7" onClick={() => moveItem(index, index + 1)} disabled={index === items.length - 1} title="Move down">
                <ChevronDown className="h-4 w-4" />
              </Button>
              <Button type="button" variant="ghost" size="icon" className="h-7 w-7 text-destructive hover:text-destructive" onClick={() => removeItem(index)}>
                <X className="h-4 w-4" />
              </Button>
            </div>
          </div>

          <div className="grid gap-2" style={{ gridTemplateColumns: itemFields.length <= 2 ? `repeat(${itemFields.length}, 1fr)` : "repeat(2, 1fr)" }}>
            {itemFields.map((field) => (
              <Input
                key={field.id}
                type={field.type === "number" ? "number" : "text"}
                placeholder={loc(field.placeholder) || loc(field.label) + (field.required ? " *" : "")}
                value={(item[field.id] as string) || ""}
                onChange={(event) => updateItem(index, field.id, field.type === "number" ? Number(event.target.value) : event.target.value)}
              />
            ))}
          </div>
        </div>
      ))}
      <Button type="button" variant="outline" className="w-full" onClick={addItem}>
        <Plus className="h-4 w-4 mr-2" />
        Add item
      </Button>
    </div>
  );
}

interface SettingsFieldProps {
  field: PluginSettingsField;
  value: unknown;
  onChange: (value: unknown) => void;
}

export function SettingsField({ field, value, onChange }: SettingsFieldProps) {
  const fieldValue = value ?? field.default ?? "";

  switch (field.type) {
    case "switch":
      return (
        <Switch
          id={field.id}
          checked={Boolean(fieldValue)}
          onCheckedChange={onChange}
        />
      );
    case "textarea":
      return (
        <Textarea
          id={field.id}
          value={String(fieldValue)}
          onChange={(event) => onChange(event.target.value)}
          rows={field.rows ?? 4}
          maxLength={field.maxLength}
        />
      );
    case "select":
      return (
        <Select value={String(fieldValue)} onValueChange={onChange}>
          <SelectTrigger><SelectValue /></SelectTrigger>
          <SelectContent>
            {field.options?.map((option) => (
              <SelectItem key={option.value} value={option.value}>{loc(option.label)}</SelectItem>
            ))}
          </SelectContent>
        </Select>
      );
    case "number":
      return (
        <Input
          id={field.id}
          type="number"
          value={String(fieldValue)}
          onChange={(event) => onChange(Number(event.target.value))}
          min={field.min}
          max={field.max}
        />
      );
    case "color":
      return (
        <div className="flex items-center gap-2">
          <Input
            type="color"
            value={String(fieldValue)}
            onChange={(event) => onChange(event.target.value)}
            className="w-12 h-10 p-1"
          />
          <Input
            value={String(fieldValue)}
            onChange={(event) => onChange(event.target.value)}
            className="flex-1"
          />
        </div>
      );
    case "array":
      return field.itemFields ? (
        <ArrayFieldEditor
          value={fieldValue as Record<string, unknown>[]}
          onChange={onChange as (value: Record<string, unknown>[]) => void}
          itemFields={field.itemFields}
        />
      ) : null;
    default:
      return (
        <Input
          id={field.id}
          type={field.secret ? "password" : "text"}
          value={String(fieldValue)}
          onChange={(event) => onChange(event.target.value)}
          maxLength={field.maxLength}
          placeholder={field.secret ? "********" : undefined}
          onFocus={(event) => {
            if (field.secret && event.target.value === "********") {
              onChange("");
            }
          }}
        />
      );
  }
}

interface SettingsRendererProps {
  schema: PluginSettingsSchema | null;
  values: Record<string, unknown>;
  onChange: (values: Record<string, unknown>) => void;
  emptyMessage?: string;
}

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

  const defaultOpen = schema.sections[0]?.id;

  return (
    <Accordion type="multiple" defaultValue={defaultOpen ? [defaultOpen] : []} className="w-full">
      {schema.sections.map((section) => (
        <AccordionItem key={section.id} value={section.id}>
          <AccordionTrigger className="text-sm">{loc(section.title)}</AccordionTrigger>
          <AccordionContent>
            <div className="space-y-4">
              {section.fields.map((field) => (
                <div key={field.id} className="space-y-2">
                  {field.type === "switch" ? (
                    <div className="flex items-center justify-between">
                      <div className="space-y-0.5">
                        <Label htmlFor={field.id}>{loc(field.label)}</Label>
                        {field.description && (
                          <p className="text-xs text-muted-foreground">{loc(field.description)}</p>
                        )}
                      </div>
                      <SettingsField
                        field={field}
                        value={values[field.id]}
                        onChange={(nextValue) => updateValue(field.id, nextValue)}
                      />
                    </div>
                  ) : (
                    <>
                      <Label htmlFor={field.id}>{loc(field.label)}</Label>
                      <SettingsField
                        field={field}
                        value={values[field.id]}
                        onChange={(nextValue) => updateValue(field.id, nextValue)}
                      />
                      {field.description && (
                        <p className="text-xs text-muted-foreground">{loc(field.description)}</p>
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
