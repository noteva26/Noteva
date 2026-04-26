import { useEffect, useMemo, useState } from "react";
import { List } from "lucide-react";
import { cn } from "@/lib/utils";
import { getNoteva } from "@/hooks/useNoteva";

interface TocItem {
    level: number;
    text: string;
    id: string;
}

interface TocSidebarProps {
    toc: TocItem[];
}

export function TocSidebar({ toc }: TocSidebarProps) {
    const [activeId, setActiveId] = useState<string>("");

    const visibleToc = useMemo(
        () => toc.filter((item) => item.level >= 2 && item.level <= 3),
        [toc]
    );

    const handleClick = (id: string) => {
        const Noteva = getNoteva();
        if (Noteva) {
            Noteva.toc.scrollTo(id, 80);
        } else {
            // fallback
            const el = document.getElementById(id);
            if (el) el.scrollIntoView({ behavior: "smooth", block: "start" });
        }
        setActiveId(id);
    };

    useEffect(() => {
        if (visibleToc.length === 0) return;

        const Noteva = getNoteva();
        if (Noteva) {
            // Use SDK scroll spy
            return Noteva.toc.observe(visibleToc, setActiveId, 100);
        }

        // Fallback: IntersectionObserver
        const headingEls = visibleToc
            .map((item) => document.getElementById(item.id))
            .filter(Boolean) as HTMLElement[];

        if (headingEls.length === 0) return;

        const observer = new IntersectionObserver(
            (entries) => {
                for (const entry of entries) {
                    if (entry.isIntersecting) {
                        setActiveId(entry.target.id);
                        break;
                    }
                }
            },
            { rootMargin: "-80px 0px -60% 0px", threshold: 0.1 }
        );

        headingEls.forEach((el) => observer.observe(el));

        return () => observer.disconnect();
    }, [visibleToc]);

    if (visibleToc.length < 2) return null;

    return (
        <aside className="hidden min-w-0 xl:block">
            <nav className="sticky top-24 max-h-[calc(100vh-8rem)] overflow-y-auto rounded-lg border bg-card/70 p-4 shadow-sm shadow-black/[0.02] backdrop-blur">
                <h4 className="mb-3 flex items-center gap-1.5 text-sm font-semibold text-foreground">
                    <List className="h-4 w-4" />
                    目录
                </h4>
                <ul className="space-y-0.5 border-l text-sm">
                    {visibleToc.map((item) => (
                        <li key={item.id}>
                            <button
                                onClick={() => handleClick(item.id)}
                                title={item.text}
                                className={cn(
                                    "block w-full -ml-px truncate border-l-2 py-1.5 text-left transition-colors",
                                    item.level === 3 ? "pl-6 pr-2" : "pl-3 pr-2",
                                    activeId === item.id
                                        ? "border-primary text-primary font-medium"
                                        : "border-transparent text-muted-foreground hover:text-foreground hover:border-muted-foreground/50"
                                )}
                            >
                                {item.text}
                            </button>
                        </li>
                    ))}
                </ul>
            </nav>
        </aside>
    );
}
