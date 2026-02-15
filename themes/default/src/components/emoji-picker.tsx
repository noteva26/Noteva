"use client";

import { useState, useRef, useEffect, useMemo, useCallback } from "react";
import { Button } from "@/components/ui/button";
import { Smile } from "lucide-react";
import { getNoteva } from "@/hooks/useNoteva";

interface ResolvedCategory {
  id: string;
  label: string;
  icon: string;
  emojis: Record<string, string>;
}

interface EmojiPickerProps {
  onSelect: (emoji: string) => void;
}

export function EmojiPicker({ onSelect }: EmojiPickerProps) {
  const [open, setOpen] = useState(false);
  const [activeCategory, setActiveCategory] = useState(0);
  const [search, setSearch] = useState("");
  const [categories, setCategories] = useState<ResolvedCategory[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const gridRef = useRef<HTMLDivElement>(null);
  const sectionRefs = useRef<(HTMLDivElement | null)[]>([]);

  // Load categories from SDK when picker opens
  useEffect(() => {
    if (!open) return;
    const load = () => {
      const Noteva = getNoteva();
      if (Noteva?.emoji) {
        setCategories(Noteva.emoji.getCategories());
      } else {
        setTimeout(load, 50);
      }
    };
    load();
  }, [open]);

  // Click outside to close
  useEffect(() => {
    const handle = (e: MouseEvent) => {
      if (containerRef.current && !containerRef.current.contains(e.target as Node)) {
        setOpen(false);
      }
    };
    if (open) document.addEventListener("mousedown", handle);
    return () => document.removeEventListener("mousedown", handle);
  }, [open]);

  // Parse Twemoji via SDK after render
  useEffect(() => {
    if (!open) return;
    const Noteva = getNoteva();
    if (gridRef.current && Noteva?.emoji) {
      Noteva.emoji.parse(gridRef.current, {
        attributes: () => ({ style: "width:22px;height:22px" }),
      });
    }
  }, [open, activeCategory, search]);

  // Parse Twemoji in sidebar
  const sidebarRef = useRef<HTMLDivElement>(null);
  useEffect(() => {
    if (!open) return;
    const Noteva = getNoteva();
    if (sidebarRef.current && Noteva?.emoji) {
      Noteva.emoji.parse(sidebarRef.current, {
        attributes: () => ({ style: "width:20px;height:20px" }),
      });
    }
  }, [open]);

  // Search results
  const searchResults = useMemo(() => {
    if (!search.trim() || categories.length === 0) return null;
    const q = search.toLowerCase();
    const results: { code: string; emoji: string }[] = [];
    for (const cat of categories) {
      for (const [code, emoji] of Object.entries(cat.emojis)) {
        if (code.includes(q) || emoji === q) {
          results.push({ code, emoji });
        }
        if (results.length >= 80) break;
      }
      if (results.length >= 80) break;
    }
    return results;
  }, [search, categories]);

  const handleCategoryClick = useCallback((idx: number) => {
    setSearch("");
    setActiveCategory(idx);
    sectionRefs.current[idx]?.scrollIntoView({ behavior: "smooth", block: "start" });
  }, []);

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
    setOpen(false);
  };

  return (
    <div ref={containerRef} className="relative inline-block">
      <Button
        type="button"
        variant="ghost"
        size="sm"
        onClick={() => setOpen(!open)}
        title="Emoji"
      >
        <Smile className="h-4 w-4" />
      </Button>

      {open && (
        <div className="absolute z-50 bottom-full mb-2 right-0 flex flex-col w-[340px] h-[280px] bg-popover border rounded-lg shadow-xl overflow-hidden">
          {categories.length === 0 ? (
            <div className="flex items-center justify-center h-full text-sm text-muted-foreground">Loading...</div>
          ) : (
            <>
          {/* Search */}
          <div className="px-2 pt-2 pb-1">
            <input
              type="text"
              placeholder="🔍"
              value={search}
              onChange={(e) => setSearch(e.target.value)}
              className="w-full px-3 py-1.5 text-sm border rounded-md bg-background outline-none focus:ring-1 focus:ring-ring"
              autoFocus
            />
          </div>

          <div className="flex flex-1 min-h-0">
            {/* Category sidebar */}
            <div ref={sidebarRef} className="flex flex-col w-10 border-r py-1 gap-0.5 items-center overflow-y-auto">
              {categories.map((cat, idx) => (
                <button
                  key={cat.id}
                  onClick={() => handleCategoryClick(idx)}
                  title={cat.label}
                  className={`w-8 h-8 flex items-center justify-center rounded text-base hover:bg-muted transition-colors ${
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
              className="flex-1 overflow-y-auto px-2 py-1"
              onScroll={handleScroll}
            >
              {searchResults ? (
                <>
                  <div className="grid grid-cols-8 gap-1">
                    {searchResults.map(({ code, emoji }) => (
                      <button
                        key={code}
                        onClick={() => handleSelect(emoji)}
                        title={`:${code}:`}
                        className="w-8 h-8 flex items-center justify-center text-lg hover:bg-muted rounded-md cursor-pointer transition-colors"
                      >
                        {emoji}
                      </button>
                    ))}
                  </div>
                  {searchResults.length === 0 && (
                    <div className="text-sm text-muted-foreground text-center py-8">😶</div>
                  )}
                </>
              ) : (
                categories.map((cat, idx) => (
                  <div key={cat.id} ref={(el) => { sectionRefs.current[idx] = el; }}>
                    <div className="text-xs text-muted-foreground sticky top-0 bg-popover py-1 px-1 font-medium">
                      {cat.label}
                    </div>
                    <div className="grid grid-cols-8 gap-1">
                      {Object.entries(cat.emojis).map(([code, emoji]) => (
                        <button
                          key={code}
                          onClick={() => handleSelect(emoji)}
                          title={`:${code}:`}
                          className="w-8 h-8 flex items-center justify-center text-lg hover:bg-muted rounded-md cursor-pointer transition-colors"
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
            </>
          )}
        </div>
      )}
    </div>
  );
}
