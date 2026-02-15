import { useEffect, useRef, useCallback, forwardRef, useImperativeHandle, useState } from "react";
import { createPortal } from "react-dom";
import Vditor from "vditor";
import "vditor/dist/index.css";
import twemoji from "@twemoji/api";
import { uploadApi } from "@/lib/api";
import { EMOJI_MAP } from "@/lib/emoji-data";
import { EmojiPicker } from "@/components/ui/emoji-picker";
import { useI18nStore, type Locale, t as i18nT } from "@/lib/i18n";

// Map our locale codes to Vditor's lang keys
const VDITOR_LANG_MAP: Record<Locale, "zh_CN" | "zh_TW" | "en_US"> = {
  "zh-CN": "zh_CN",
  "zh-TW": "zh_TW",
  "en": "en_US",
};

// Resolve plugin label: supports string or { "zh-CN": "...", "en": "..." } object
function resolveLabel(label: string | Record<string, string>, locale: Locale): string {
  if (typeof label === "string") return label;
  return label[locale] || label["en"] || label["zh-CN"] || Object.values(label)[0] || "";
}

interface PluginEditorButton {
  id: string;
  label: string | Record<string, string>;
  icon?: string;
  insertBefore: string;
  insertAfter: string;
}

export interface VditorEditorRef {
  getValue: () => string;
  setValue: (value: string) => void;
  insertValue: (value: string, render?: boolean) => void;
  focus: () => void;
  getVditor: () => Vditor | null;
}

interface VditorEditorProps {
  initialValue?: string;
  onChange?: (value: string) => void;
  pluginButtons?: PluginEditorButton[];
  placeholder?: string;
  minHeight?: number;
}

const VditorEditor = forwardRef<VditorEditorRef, VditorEditorProps>(
  ({ initialValue = "", onChange, pluginButtons = [], placeholder, minHeight = 400 }, ref) => {
    const containerRef = useRef<HTMLDivElement>(null);
    const vditorRef = useRef<Vditor | null>(null);
    const onChangeRef = useRef(onChange);
    const initialValueRef = useRef(initialValue);
    const pluginButtonsRef = useRef(pluginButtons);
    const [emojiPickerOpen, setEmojiPickerOpen] = useState(false);
    const [emojiPickerPos, setEmojiPickerPos] = useState({ top: 0, left: 0 });
    const emojiButtonRef = useRef<HTMLElement | null>(null);
    const locale = useI18nStore((s) => s.locale);

    // Keep refs in sync
    onChangeRef.current = onChange;
    pluginButtonsRef.current = pluginButtons;

    useImperativeHandle(ref, () => ({
      getValue: () => vditorRef.current?.getValue() || "",
      setValue: (value: string) => vditorRef.current?.setValue(value),
      insertValue: (value: string, render = true) => vditorRef.current?.insertValue(value, render),
      focus: () => vditorRef.current?.focus(),
      getVditor: () => vditorRef.current,
    }));

    useEffect(() => {
      if (!containerRef.current) return;

      // Save current content if reinitializing (e.g. plugin buttons arrived late)
      const currentValue = vditorRef.current?.getValue() || initialValueRef.current;

      // SVG icon mapping for known plugin button ids (Font Awesome 6 Free)
      const PLUGIN_ICONS: Record<string, string> = {
        "hide-until-reply": '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 448 512" fill="currentColor"><path d="M144 144v48H304V144c0-44.2-35.8-80-80-80s-80 35.8-80 80zM80 192V144C80 64.5 144.5 0 224 0s144 64.5 144 144v48h16c35.3 0 64 28.7 64 64V448c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V256c0-35.3 28.7-64 64-64H80z"/></svg>',
        "insert-video": '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 576 512" fill="currentColor"><path d="M0 128C0 92.7 28.7 64 64 64H320c35.3 0 64 28.7 64 64V384c0 35.3-28.7 64-64 64H64c-35.3 0-64-28.7-64-64V128zM559.1 99.8c10.4 5.6 16.9 16.4 16.9 28.2V384c0 11.8-6.5 22.6-16.9 28.2s-23 5-32.9-1.6l-96-64L416 336.5V175.5l14.2-9.5 96-64c9.8-6.5 22.4-7.2 32.9-1.6z"/></svg>',
        "insert-audio": '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 512 512" fill="currentColor"><path d="M499.1 6.3c8.1 6 12.9 15.6 12.9 25.7v72V368c0 44.2-43 80-96 80s-96-35.8-96-80s43-80 96-80c11.2 0 22 1.6 32 4.6V147L192 223.8V432c0 44.2-43 80-96 80s-96-35.8-96-80s43-80 96-80c11.2 0 22 1.6 32 4.6V200 128c0-14.1 9.3-26.5 22.8-30.5l320-96c9.7-2.9 20.2-1.1 28.3 4.8z"/></svg>',
      };
      // Fallback: FA puzzle-piece
      const PLUGIN_ICON_DEFAULT = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 512 512" fill="currentColor"><path d="M78.6 5C69.1-2.4 55.6-1.5 47.7 7L7 47.7C-1.5 55.6-2.4 69.1 5 78.6s21.9 9.5 30.4 1.6L176 0h0V128l-32 0c-35.3 0-64 28.7-64 64v64H0V176c0-8.8-7.2-16-16-16H-32c-8.8 0-16 7.2-16 16v80H0v64H80V256c0-35.3 28.7-64 64-64h32V64H96L78.6 5z"/></svg>';
      // FA circle-plus for the ⊕ trigger
      const PLUS_CIRCLE_ICON = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 512 512" fill="currentColor"><path d="M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM232 344V280H168c-13.3 0-24-10.7-24-24s10.7-24 24-24h64V168c0-13.3 10.7-24 24-24s24 10.7 24 24v64h64c13.3 0 24 10.7 24 24s-10.7 24-24 24H280v64c0 13.3-10.7 24-24 24s-24-10.7-24-24z"/></svg>';

      const getPluginIcon = (btn: PluginEditorButton) => {
        // SVG string (starts with <)
        if (btn.icon && btn.icon.startsWith("<")) return btn.icon;
        // Font Awesome class name (e.g. "fa-solid fa-lock")
        if (btn.icon && btn.icon.startsWith("fa-")) return `<i class="${btn.icon}"></i>`;
        // Fallback: known id mapping or default
        return PLUGIN_ICONS[btn.id] || PLUGIN_ICON_DEFAULT;
      };

      // Emoji SVG icon (FA face-smile)
      const EMOJI_ICON = '<svg xmlns="http://www.w3.org/2000/svg" width="16" height="16" viewBox="0 0 512 512" fill="currentColor"><path d="M256 512A256 256 0 1 0 256 0a256 256 0 1 0 0 512zM164.1 325.5C182 346.2 212.6 368 256 368s74-21.8 91.9-42.5c5.8-6.7 15.9-7.4 22.6-1.6s7.4 15.9 1.6 22.6C349.8 372.1 311.1 400 256 400s-93.8-27.9-116.1-53.5c-5.8-6.7-5.1-16.8 1.6-22.6s16.8-5.1 22.6 1.6zM144.4 208a32 32 0 1 1 64 0 32 32 0 1 1 -64 0zm192-32a32 32 0 1 1 0 64 32 32 0 1 1 0-64z"/></svg>';

      const toolbar: (string | IMenuItem)[] = [
        "headings", "bold", "italic", "strike", "|",
        "line", "quote", "list", "ordered-list", "check", "|",
        "code", "inline-code", "link", "table", "|",
        "upload",
        {
          name: "emoji-custom",
          tip: i18nT("editor.emoji"),
          icon: EMOJI_ICON,
          click: () => {
            // Find the button element in the toolbar to position the picker
            const btn = containerRef.current?.querySelector('[data-type="emoji-custom"]') as HTMLElement;
            if (btn) {
              emojiButtonRef.current = btn;
              const rect = btn.getBoundingClientRect();
              setEmojiPickerPos({
                top: rect.bottom + 4,
                left: Math.max(8, rect.left - 150), // Center-ish under button
              });
            }
            setEmojiPickerOpen((prev) => !prev);
          },
        },
        "|",
        "undo", "redo", "|",
        "fullscreen",
        "preview",
        "outline",
      ];

      // Build a single ⊕ dropdown with all plugin buttons as sub-items
      if (pluginButtonsRef.current.length > 0) {
        const subToolbar: IMenuItem[] = pluginButtonsRef.current.map((btn) => {
          const label = resolveLabel(btn.label, locale);
          return {
            name: `plugin-${btn.id}`,
            tip: label,
            icon: `${getPluginIcon(btn)}&nbsp;&nbsp;${label}`,
            click: () => {
              if (vditorRef.current) {
                vditorRef.current.insertValue(btn.insertBefore + btn.insertAfter);
              }
            },
          };
        });

        const pluginMenu: IMenuItem = {
          name: "plugin-menu",
          tip: i18nT("editor.plugins"),
          icon: PLUS_CIRCLE_ICON,
          toolbar: subToolbar,
        };

        const fsIdx = toolbar.indexOf("fullscreen");
        toolbar.splice(fsIdx, 0, "|", pluginMenu);
      }

      const isDark = document.documentElement.classList.contains("dark");

      const vditor = new Vditor(containerRef.current, {
        value: currentValue,
        theme: isDark ? "dark" : "classic",
        mode: "ir", // 即时渲染模式 (Typora-like)
        lang: VDITOR_LANG_MAP[locale] || "zh_CN",
        placeholder: placeholder || "",
        minHeight,
        toolbar,
        toolbarConfig: {
          pin: true,
        },
        cache: {
          enable: false, // We manage state ourselves
        },
        hint: {
          emoji: EMOJI_MAP,
          emojiTail: '<a href="https://github.com/jdecked/twemoji" target="_blank" rel="noopener" style="color:#999;font-size:12px">Twemoji</a>',
        },
        preview: {
          theme: {
            current: isDark ? "dark" : "light",
          },
          hljs: {
            lineNumber: true,
          },
          math: {
            engine: "KaTeX",
          },
          transform: (html: string) => {
            let r = html;
            // [video src="..." /]
            r = r.replace(
              /\[video\s+src=(?:&quot;|")([^"&]+)(?:&quot;|")\s*\/?\]/g,
              '<div class="shortcode-video" style="margin:8px 0"><video src="$1" width="100%" controls playsinline></video></div>'
            );
            // [audio src="..." /]
            r = r.replace(
              /\[audio\s+src=(?:&quot;|")([^"&]+)(?:&quot;|")\s*\/?\]/g,
              '<div class="shortcode-audio" style="margin:8px 0"><audio src="$1" controls preload="metadata" style="width:100%"></audio></div>'
            );
            // [hide-until-reply]...[/hide-until-reply]
            r = r.replace(
              /\[hide-until-reply\]([\s\S]*?)\[\/hide-until-reply\]/g,
              '<div class="noteva-hidden-content" style="margin:8px 0"><div style="background:#fff8e1;border:1px solid #ffe082;border-radius:8px;padding:12px 16px;display:flex;align-items:center;gap:8px"><span>🔒</span><span>回复后可见</span></div></div>'
            );
            // [note type="info|warning|error|success"]...[/note]
            r = r.replace(
              /\[note(?:\s+type=(?:&quot;|")(\w+)(?:&quot;|"))?\]([\s\S]*?)\[\/note\]/g,
              (_m: string, type_: string, content: string) => {
                const colors: Record<string, string> = {
                  info: "border-left:4px solid #3b82f6;background:#eff6ff",
                  warning: "border-left:4px solid #f59e0b;background:#fffbeb",
                  error: "border-left:4px solid #ef4444;background:#fef2f2",
                  danger: "border-left:4px solid #ef4444;background:#fef2f2",
                  success: "border-left:4px solid #22c55e;background:#f0fdf4",
                };
                const s = colors[type_] || colors.info;
                return `<div style="${s};border-radius:6px;padding:12px 16px;margin:8px 0">${content}</div>`;
              }
            );
            // [collapse title="..."]...[/collapse]
            r = r.replace(
              /\[collapse(?:\s+title=(?:&quot;|")([^"&]+)(?:&quot;|"))?\]([\s\S]*?)\[\/collapse\]/g,
              '<details style="margin:8px 0;border:1px solid #e5e7eb;border-radius:6px;padding:0"><summary style="padding:10px 16px;cursor:pointer;font-weight:500">$1</summary><div style="padding:8px 16px 12px;border-top:1px solid #e5e7eb">$2</div></details>'
            );
            // [button url="..." target="..."]text[/button]
            r = r.replace(
              /\[button(?:\s+url=(?:&quot;|")([^"&]+)(?:&quot;|"))?(?:\s+target=(?:&quot;|")([^"&]+)(?:&quot;|"))?(?:\s+style=(?:&quot;|")([^"&]+)(?:&quot;|"))?\]([\s\S]*?)\[\/button\]/g,
              '<a href="$1" target="$2" style="display:inline-block;padding:6px 16px;background:#3b82f6;color:#fff;border-radius:6px;text-decoration:none;margin:4px 0;font-size:14px">$4</a>'
            );
            // [code lang="..."]...[/code]
            r = r.replace(
              /\[code(?:\s+lang=(?:&quot;|")([^"&]+)(?:&quot;|"))?\]([\s\S]*?)\[\/code\]/g,
              '<pre style="margin:8px 0;background:#1e1e1e;color:#d4d4d4;border-radius:6px;padding:16px;overflow-x:auto"><code>$2</code></pre>'
            );
            // [quote author="..." source="..."]...[/quote]
            r = r.replace(
              /\[quote(?:\s+author=(?:&quot;|")([^"&]*?)(?:&quot;|"))?(?:\s+source=(?:&quot;|")([^"&]*?)(?:&quot;|"))?\]([\s\S]*?)\[\/quote\]/g,
              (_m: string, author: string, source: string, content: string) => {
                let footer = "";
                if (author) footer += `— ${author}`;
                if (source) footer += `${author ? ", " : ""}<cite>${source}</cite>`;
                const footerHtml = footer ? `<footer style="margin-top:8px;font-size:13px;color:#6b7280">${footer}</footer>` : "";
                return `<blockquote style="margin:8px 0;border-left:4px solid #d1d5db;padding:8px 16px;color:#374151">${content}${footerHtml}</blockquote>`;
              }
            );
            return r;
          },
          parse: (element: HTMLElement) => {
            // Render Unicode emoji as Twemoji in preview
            twemoji.parse(element, { folder: "svg", ext: ".svg" });
          },
        },
        upload: {
          accept: "image/*",
          handler: async (files: File[]) => {
            for (const file of files) {
              try {
                const { data } = await uploadApi.image(file);
                vditorRef.current?.insertValue(`![${file.name}](${data.url})`);
              } catch {
                // Upload failed silently
              }
            }
            return null as any;
          },
        },
        input: (value: string) => {
          onChangeRef.current?.(value);
        },
        after: () => {
          vditorRef.current = vditor;
          // Apply dark mode class
          if (isDark) {
            vditor.setTheme("dark", "dark");
          }
          // Parse emoji in the editor to Twemoji
          const el = containerRef.current;
          if (el) twemoji.parse(el, { folder: "svg", ext: ".svg" });
        },
      });

      return () => {
        vditor.destroy();
        vditorRef.current = null;
      };
    }, [pluginButtons.length, locale]); // Reinit when plugin buttons arrive or locale changes

    // Watch for dark mode changes
    useEffect(() => {
      const observer = new MutationObserver(() => {
        if (!vditorRef.current) return;
        const isDark = document.documentElement.classList.contains("dark");
        vditorRef.current.setTheme(isDark ? "dark" : "classic", isDark ? "dark" : "light");
      });
      observer.observe(document.documentElement, { attributes: true, attributeFilter: ["class"] });
      return () => observer.disconnect();
    }, []);

    return (
      <>
        <div ref={containerRef} className="vditor-container" />
        {emojiPickerOpen && createPortal(
          <div
            className="fixed z-[9999]"
            style={{ top: emojiPickerPos.top, left: emojiPickerPos.left }}
          >
            <EmojiPicker
              onSelect={(emoji) => {
                vditorRef.current?.insertValue(emoji);
              }}
              onClose={() => setEmojiPickerOpen(false)}
            />
          </div>,
          document.body
        )}
      </>
    );
  }
);

VditorEditor.displayName = "VditorEditor";

// Type for Vditor toolbar menu item
interface IMenuItem {
  name: string;
  tip?: string;
  icon?: string;
  click?: () => void;
  toolbar?: IMenuItem[];
}

export default VditorEditor;
