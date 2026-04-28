import { useState, useRef, useEffect, useCallback, useDeferredValue, useMemo } from "react";
import twemoji from "@twemoji/api";
import { EMOJI_CATEGORIES } from "@/lib/emoji-data";
import { useI18nStore, t as i18nT } from "@/lib/i18n";
import { cn } from "@/lib/utils";

interface EmojiPickerProps {
  onSelect: (emoji: string) => void;
  onClose: () => void;
  className?: string;
  autoFocusSearch?: boolean;
}

export function EmojiPicker({
  onSelect,
  onClose,
  className,
  autoFocusSearch = true,
}: EmojiPickerProps) {
  const [activeCategory, setActiveCategory] = useState(0);
  const [search, setSearch] = useState("");
  const deferredSearch = useDeferredValue(search);
  const containerRef = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const sidebarRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<(HTMLDivElement | null)[]>([]);
  const locale = useI18nStore((state) => state.locale);

  const getCatLabel = useCallback(
    (category: typeof EMOJI_CATEGORIES[0]) =>
      category.label[locale] || category.label.en || category.label["zh-CN"],
    [locale]
  );

  useEffect(() => {
    const handlePointerDown = (event: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(event.target as Node)) {
        onClose();
      }
    };

    document.addEventListener("mousedown", handlePointerDown);
    return () => document.removeEventListener("mousedown", handlePointerDown);
  }, [onClose]);

  useEffect(() => {
    const node = gridRef.current;
    if (!node) return;

    const frame = window.requestAnimationFrame(() => {
      twemoji.parse(node, {
        folder: "svg",
        ext: ".svg",
        attributes: () => ({ style: "width:22px;height:22px" }),
      });
    });

    return () => window.cancelAnimationFrame(frame);
  }, [activeCategory, deferredSearch]);

  useEffect(() => {
    const node = sidebarRef.current;
    if (!node) return;

    const frame = window.requestAnimationFrame(() => {
      twemoji.parse(node, {
        folder: "svg",
        ext: ".svg",
        attributes: () => ({ style: "width:20px;height:20px" }),
      });
    });

    return () => window.cancelAnimationFrame(frame);
  }, []);

  const searchResults = useMemo(() => {
    const query = deferredSearch.trim().toLowerCase();
    if (!query) return null;

    const results: { code: string; emoji: string }[] = [];
    for (const category of EMOJI_CATEGORIES) {
      for (const [code, emoji] of Object.entries(category.emojis)) {
        if (code.includes(query) || emoji === query) {
          results.push({ code, emoji });
        }
        if (results.length >= 80) break;
      }
      if (results.length >= 80) break;
    }
    return results;
  }, [deferredSearch]);

  const handleCategoryClick = useCallback((index: number) => {
    setSearch("");
    setActiveCategory(index);
    sectionRefs.current[index]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

  const handleScroll = useCallback(() => {
    if (search || !gridRef.current) return;

    const top = gridRef.current.scrollTop;
    for (let index = sectionRefs.current.length - 1; index >= 0; index--) {
      const section = sectionRefs.current[index];
      if (section && section.offsetTop - gridRef.current.offsetTop <= top + 8) {
        setActiveCategory((current) => current === index ? current : index);
        break;
      }
    }
  }, [search]);

  return (
    <div
      ref={containerRef}
      className={cn(
        "flex h-[320px] w-[360px] flex-col overflow-hidden rounded-lg border bg-popover shadow-xl",
        className
      )}
    >
      <div className="px-2 pt-2 pb-1">
        <input
          type="text"
          placeholder={i18nT("editor.searchEmoji")}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          className="w-full rounded-md border bg-background px-3 py-1.5 text-sm outline-none focus:ring-1 focus:ring-ring"
          autoFocus={autoFocusSearch}
        />
      </div>

      <div className="flex flex-1 min-h-0">
        <div ref={sidebarRef} className="emoji-picker-sidebar flex w-11 shrink-0 flex-col items-center gap-1 overflow-y-auto border-r py-1 hide-scrollbar">
          {EMOJI_CATEGORIES.map((category, index) => (
            <button
              key={category.id}
              type="button"
              onClick={() => handleCategoryClick(index)}
              title={getCatLabel(category)}
              className={`flex h-9 w-9 shrink-0 items-center justify-center rounded-md text-lg transition-colors hover:bg-muted ${
                activeCategory === index && !search ? "bg-muted ring-1 ring-ring/30" : ""
              }`}
            >
              {category.icon}
            </button>
          ))}
        </div>

        <div
          ref={gridRef}
          className="emoji-picker-grid flex-1 overflow-y-auto px-2 py-1"
          onScroll={handleScroll}
        >
          {searchResults ? (
            <>
              <div className="text-xs text-muted-foreground mb-1 px-1">
                {i18nT("common.search")} ({searchResults.length})
              </div>
              <div className="grid grid-cols-[repeat(auto-fill,minmax(2.25rem,1fr))] gap-1">
                {searchResults.map(({ code, emoji }) => (
                  <button
                    key={code}
                    type="button"
                    onClick={() => onSelect(emoji)}
                    title={`:${code}:`}
                    className="flex h-9 w-9 items-center justify-center justify-self-center rounded-md text-base transition-colors hover:bg-muted"
                  >
                    {emoji}
                  </button>
                ))}
              </div>
              {searchResults.length === 0 && (
                <div className="py-8 text-center text-sm text-muted-foreground">
                  {i18nT("common.noData")}
                </div>
              )}
            </>
          ) : (
            EMOJI_CATEGORIES.map((category, index) => (
              <div
                key={category.id}
                ref={(element) => { sectionRefs.current[index] = element; }}
              >
                <div className="sticky top-0 bg-popover px-1 py-1 text-xs font-medium text-muted-foreground">
                  {getCatLabel(category)}
                </div>
                <div className="grid grid-cols-[repeat(auto-fill,minmax(2.25rem,1fr))] gap-1">
                  {Object.entries(category.emojis).map(([code, emoji]) => (
                    <button
                      key={code}
                      type="button"
                      onClick={() => onSelect(emoji)}
                      title={`:${code}:`}
                      className="flex h-9 w-9 items-center justify-center justify-self-center rounded-md text-base transition-colors hover:bg-muted"
                    >
                      {emoji}
                    </button>
                  ))}
                </div>
              </div>
            ))
          )}
        </div>
      </div>
    </div>
  );
}
