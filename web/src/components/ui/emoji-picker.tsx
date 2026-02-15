import { useState, useRef, useEffect, useCallback, useMemo } from "react";
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
  const containerRef = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<(HTMLDivElement | null)[]>([]);
  const locale = useI18nStore((s) => s.locale);

  const getCatLabel = (cat: typeof EMOJI_CATEGORIES[0]) => cat.label[locale] || cat.label["en"] || cat.label["zh-CN"];

  // Click outside to close
  useEffect(() => {
    const handle = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        onClose();
      }
    };
    document.addEventListener("mousedown", handle);
    return () => document.removeEventListener("mousedown", handle);
  }, [onClose]);

  // Parse Twemoji after render — with size override
  useEffect(() => {
    if (gridRef.current) {
      twemoji.parse(gridRef.current, {
        folder: "svg",
        ext: ".svg",
        attributes: () => ({ style: "width:22px;height:22px" }),
      });
    }
  }, [activeCategory, search]);

  // Parse Twemoji in sidebar icons
  const sidebarRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (sidebarRef.current) {
      twemoji.parse(sidebarRef.current, {
        folder: "svg",
        ext: ".svg",
        attributes: () => ({ style: "width:20px;height:20px" }),
      });
    }
  }, []);

  // Search results
  const searchResults = useMemo(() => {
    if (!search.trim()) return null;
    const q = search.toLowerCase();
    const results: { code: string; emoji: string }[] = [];
    for (const cat of EMOJI_CATEGORIES) {
      for (const [code, emoji] of Object.entries(cat.emojis)) {
        if (code.includes(q) || emoji === q) {
          results.push({ code, emoji });
        }
        if (results.length >= 80) break;
      }
      if (results.length >= 80) break;
    }
    return results;
  }, [search]);

  const handleCategoryClick = useCallback((idx: number) => {
    setSearch("");
    setActiveCategory(idx);
    sectionRefs.current[idx]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

  // Track scroll to update active category
  const handleScroll = useCallback(() => {
    if (search || !gridRef.current) return;
    const top = gridRef.current.scrollTop;
    for (let i = sectionRefs.current.length - 1; i >= 0; i--) {
      const el = sectionRefs.current[i];
      if (el && el.offsetTop - gridRef.current.offsetTop <= top + 8) {
        setActiveCategory(i);
        break;
      }
    }
  }, [search]);

  const handleSelect = (emoji: string) => {
    onSelect(emoji);
  };

  return (
    <div
      ref={containerRef}
      className="flex flex-col w-[360px] h-[300px] bg-popover border rounded-lg shadow-xl overflow-hidden"
    >
      {/* Search bar */}
      <div className="px-2 pt-2 pb-1">
        <input
          type="text"
          placeholder={i18nT("editor.searchEmoji")}
          value={search}
          onChange={(e) => setSearch(e.target.value)}
          className="w-full px-3 py-1.5 text-sm border rounded-md bg-background outline-none focus:ring-1 focus:ring-ring"
          autoFocus
        />
      </div>

      <div className="flex flex-1 min-h-0">
        {/* Category sidebar */}
        <div ref={sidebarRef} className="emoji-picker-sidebar flex flex-col w-11 border-r py-1 gap-1 items-center overflow-y-auto hide-scrollbar">
          {EMOJI_CATEGORIES.map((cat, idx) => (
            <button
              key={cat.id}
              onClick={() => handleCategoryClick(idx)}
              title={getCatLabel(cat)}
              className={`w-9 h-9 flex items-center justify-center rounded-md text-lg hover:bg-muted transition-colors ${
                activeCategory === idx && !search ? "bg-muted ring-1 ring-ring/30" : ""
              }`}
            >
              {cat.icon}
            </button>
          ))}
        </div>

        {/* Emoji grid */}
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
                    onClick={() => handleSelect(emoji)}
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
            EMOJI_CATEGORIES.map((cat, idx) => (
              <div
                key={cat.id}
                ref={(el) => { sectionRefs.current[idx] = el; }}
              >
                <div className="text-xs text-muted-foreground sticky top-0 bg-popover py-1 px-1 font-medium">
                  {getCatLabel(cat)}
                </div>
                <div className="grid grid-cols-8 gap-1">
                  {Object.entries(cat.emojis).map(([code, emoji]) => (
                    <button
                      key={code}
                      onClick={() => handleSelect(emoji)}
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
