import { useEffect } from "react";

interface SeoMeta {
    title?: string;
    description?: string;
    image?: string;
    url?: string;
    type?: "website" | "article";
    siteName?: string;
    publishedTime?: string;
    author?: string;
}

/**
 * 动态注入 Open Graph / Twitter Card meta 标签
 * 用于 SSR 友好的 SEO 优化
 */
export function useSeoMeta(meta: SeoMeta) {
    useEffect(() => {
        const tags: HTMLMetaElement[] = [];
        let originalTitle: string | undefined;

        const setMeta = (property: string, content: string | undefined) => {
            if (!content) return;
            // Find existing or create new
            let el = document.querySelector(`meta[property="${property}"]`) as HTMLMetaElement | null;
            if (!el) {
                el = document.querySelector(`meta[name="${property}"]`) as HTMLMetaElement | null;
            }
            if (!el) {
                el = document.createElement("meta");
                // Twitter uses name, OG uses property
                if (property.startsWith("twitter:")) {
                    el.setAttribute("name", property);
                } else {
                    el.setAttribute("property", property);
                }
                document.head.appendChild(el);
                tags.push(el);
            }
            el.setAttribute("content", content);
        };

        // Set document title
        if (meta.title) {
            originalTitle = document.title;
            document.title = meta.siteName ? `${meta.title} - ${meta.siteName}` : meta.title;
        }

        // Standard meta
        setMeta("description", meta.description);

        // Open Graph
        setMeta("og:title", meta.title);
        setMeta("og:description", meta.description);
        setMeta("og:image", meta.image);
        setMeta("og:url", meta.url || (typeof window !== "undefined" ? window.location.href : undefined));
        setMeta("og:type", meta.type || "website");
        setMeta("og:site_name", meta.siteName);

        // Article specific
        if (meta.type === "article") {
            setMeta("article:published_time", meta.publishedTime);
            setMeta("article:author", meta.author);
        }

        // Twitter Card
        setMeta("twitter:card", meta.image ? "summary_large_image" : "summary");
        setMeta("twitter:title", meta.title);
        setMeta("twitter:description", meta.description);
        setMeta("twitter:image", meta.image);

        return () => {
            // Cleanup: remove tags we created
            for (const tag of tags) {
                if (tag && tag.parentNode) {
                    tag.parentNode.removeChild(tag);
                }
            }
            // Restore original title
            if (originalTitle !== undefined) {
                document.title = originalTitle;
            }
        };
    }, [meta.title, meta.description, meta.image, meta.url, meta.type, meta.siteName, meta.publishedTime, meta.author]);
}
