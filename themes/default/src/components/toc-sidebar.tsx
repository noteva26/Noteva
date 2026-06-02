import { useEffect, useMemo, useState } from "react";
import { cn } from "@/lib/utils";
import { getNoteva } from "@/hooks/useNoteva";
import { useTranslation } from "@/lib/i18n";

interface TocItem {
    level: number;
    text: string;
    id: string;
}

interface TocSidebarProps {
    toc: TocItem[];
}

export function TocSidebar({ toc }: TocSidebarProps) {
    const { t } = useTranslation();
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
        <aside className="default-toc-sidebar hidden min-w-0 xl:block">
            <nav className="default-toc-nav">
                <h4 className="default-toc-title">{t("article.toc")}</h4>
                <ul className="default-toc-list">
                    {visibleToc.map((item) => (
                        <li key={item.id}>
                            <button
                                onClick={() => handleClick(item.id)}
                                title={item.text}
                                className={cn(
                                    "default-toc-button",
                                    item.level === 3 ? "default-toc-button-child" : "default-toc-button-root",
                                    activeId === item.id
                                        ? "is-active"
                                        : ""
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
