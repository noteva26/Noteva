import { useState, useRef, useEffect, useCallback, useDeferredValue, useMemo } from "react";
import twemoji from "@twemoji/api";
import { EMOJI_CATEGORIES } from "@/lib/emoji-data";
import { useI18nStore, t as i18nT } from "@/lib/i18n";

interface EmojiPickerProps {
  onSelect: (emoji: string) => void;
  onClose: () => void;
}

export function EmojiPicker({ onSelect, onClose }: EmojiPickerProps) {
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
      className="flex flex-col w-[360px] h-[300px] bg-popover border rounded-lg shadow-xl overflow-hidden"
    >
      <div className="px-2 pt-2 pb-1">
        <input
          type="text"
          placeholder={i18nT("editor.searchEmoji")}
          value={search}
          onChange={(event) => setSearch(event.target.value)}
          className="w-full px-3 py-1.5 text-sm border rounded-md bg-background outline-none focus:ring-1 focus:ring-ring"
          autoFocus
        />
      </div>

      <div className="flex flex-1 min-h-0">
        <div ref={sidebarRef} className="emoji-picker-sidebar flex flex-col w-11 border-r py-1 gap-1 items-center overflow-y-auto hide-scrollbar">
          {EMOJI_CATEGORIES.map((category, index) => (
            <button
              key={category.id}
              type="button"
              onClick={() => handleCategoryClick(index)}
              title={getCatLabel(category)}
              className={`w-9 h-9 flex items-center justify-center rounded-md text-lg hover:bg-muted transition-colors ${
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
              <div className="grid grid-cols-8 gap-1">
                {searchResults.map(({ code, emoji }) => (
                  <button
                    key={code}
                    type="button"
                    onClick={() => onSelect(emoji)}
                    title={`:${code}:`}
                    className="w-9 h-9 flex items-center justify-center text-base hover:bg-muted rounded-md cursor-pointer transition-colors"
                  >
                    {emoji}
                  </button>
                ))}
              </div>
              {searchResults.length === 0 && (
                <div className="text-sm text-muted-foreground text-center py-8">
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
                <div className="text-xs text-muted-foreground sticky top-0 bg-popover py-1 px-1 font-medium">
                  {getCatLabel(category)}
                </div>
                <div className="grid grid-cols-8 gap-1">
                  {Object.entries(category.emojis).map(([code, emoji]) => (
                    <button
                      key={code}
                      type="button"
                      onClick={() => onSelect(emoji)}
                      title={`:${code}:`}
                      className="w-9 h-9 flex items-center justify-center text-base hover:bg-muted rounded-md cursor-pointer transition-colors"
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
