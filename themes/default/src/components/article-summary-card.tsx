import type { ReactNode } from "react";
import { Link } from "react-router-dom";
import {
  Calendar,
  Eye,
  Folder,
  Heart,
  MessageSquare,
  Pin,
  Tag,
} from "lucide-react";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { getArticleUrl, getNoteva, type NotevaArticle } from "@/hooks/useNoteva";
import { useTranslation } from "@/lib/i18n";
import { cn } from "@/lib/utils";

interface ArticleSummaryCardProps {
  article: NotevaArticle;
  dateLocale: string;
  className?: string;
  highlightQuery?: string;
  priorityImage?: boolean;
  showCategory?: boolean;
  showTags?: boolean;
  onWarmRoute?: () => void;
}

function stripMarkup(value: string) {
  return value.replace(/<[^>]*>/g, " ").replace(/[#*`>\n\r]/g, " ");
}

function formatArticleDate(value: string, locale: string) {
  const date = new Date(value);
  if (Number.isNaN(date.getTime())) return "";

  return date.toLocaleDateString(locale, {
    year: "numeric",
    month: "short",
    day: "numeric",
  });
}

function highlightText(text: string, query?: string): ReactNode {
  const keyword = query?.trim();
  if (!keyword) return text;

  const escaped = keyword.replace(/[.*+?^${}()|[\]\\]/g, "\\$&");
  const regex = new RegExp(`(${escaped})`, "gi");
  const parts = text.split(regex);

  if (parts.length <= 1) return text;

  return parts.map((part, index) =>
    part.toLowerCase() === keyword.toLowerCase() ? (
      <mark
        key={`${part}-${index}`}
        className="rounded-sm bg-yellow-200 px-0.5 text-inherit dark:bg-yellow-700/60"
      >
        {part}
      </mark>
    ) : (
      part
    )
  );
}

export function ArticleSummaryCard({
  article,
  dateLocale,
  className,
  highlightQuery,
  priorityImage = false,
  showCategory = true,
  showTags = true,
  onWarmRoute,
}: ArticleSummaryCardProps) {
  const { t } = useTranslation();
  const noteva = getNoteva();
  const articleUrl = getArticleUrl(article);
  const stats = noteva?.articles.getStats(article) || {
    views: article.viewCount || 0,
    likes: article.likeCount || 0,
    comments: article.commentCount || 0,
  };
  const thumbnail =
    noteva?.articles.getThumbnail(article) ||
    article.thumbnail ||
    article.coverImage ||
    "";
  const publishedAt =
    noteva?.articles.getDate(article) || article.publishedAt || article.createdAt;
  const excerpt =
    noteva?.articles.getExcerpt(article, 180) ||
    article.excerpt ||
    stripMarkup(article.content).trim().slice(0, 180);
  const isPinned = noteva?.articles.isPinned(article) ?? Boolean(article.isPinned);
  const formattedDate = publishedAt ? formatArticleDate(publishedAt, dateLocale) : "";

  return (
    <Card className={cn("article-card group overflow-hidden", className)} data-article-id={article.id}>
      <div className={cn("grid", thumbnail && "sm:grid-cols-[1fr_13rem]")}>
        {thumbnail ? (
          <Link
            to={articleUrl}
            className="block aspect-[16/9] overflow-hidden bg-muted sm:order-last sm:aspect-auto sm:min-h-[172px]"
            onFocus={onWarmRoute}
            onMouseEnter={onWarmRoute}
          >
            <img
              src={thumbnail}
              alt={article.title}
              className="h-full w-full object-cover transition duration-500 group-hover:scale-[1.03]"
              loading={priorityImage ? "eager" : "lazy"}
            />
          </Link>
        ) : null}

        <div className="min-w-0">
          <CardHeader className="space-y-3 pb-3">
            <div className="flex min-w-0 items-start gap-2">
              {isPinned ? (
                <Badge variant="destructive" className="mt-0.5 shrink-0 gap-1">
                  <Pin className="h-3 w-3" />
                  {t("article.pinned")}
                </Badge>
              ) : null}
              <CardTitle className="min-w-0 flex-1 text-xl leading-snug md:text-2xl">
                <Link
                  to={articleUrl}
                  className="decoration-primary/40 underline-offset-4 transition-colors hover:text-primary hover:underline"
                  onFocus={onWarmRoute}
                  onMouseEnter={onWarmRoute}
                >
                  {highlightText(article.title, highlightQuery)}
                </Link>
              </CardTitle>
            </div>

            <div className="article-meta flex flex-wrap items-center gap-x-4 gap-y-2 text-sm text-muted-foreground">
              {formattedDate ? (
                <span className="inline-flex items-center gap-1">
                  <Calendar className="h-4 w-4" />
                  {formattedDate}
                </span>
              ) : null}
              <span className="inline-flex items-center gap-1">
                <Eye className="h-4 w-4" />
                {stats.views}
              </span>
              <span className="inline-flex items-center gap-1">
                <Heart className="h-4 w-4" />
                {stats.likes}
              </span>
              <span className="inline-flex items-center gap-1">
                <MessageSquare className="h-4 w-4" />
                {stats.comments}
              </span>
            </div>
          </CardHeader>

          <CardContent>
            {excerpt ? (
              <p className="mb-4 line-clamp-2 leading-7 text-muted-foreground">
                {highlightText(excerpt, highlightQuery)}
              </p>
            ) : null}

            <div className="flex flex-wrap items-center gap-2">
              {showCategory && article.category ? (
                <Link to={`/categories?c=${article.category.slug}`}>
                  <Badge variant="outline" className="hover:bg-secondary">
                    <Folder className="mr-1 h-3 w-3" />
                    {article.category.name}
                  </Badge>
                </Link>
              ) : null}

              {showTags
                ? article.tags?.slice(0, 3).map((tag) => (
                    <Link key={tag.id} to={`/tags?t=${tag.slug}`}>
                      <Badge variant="secondary" className="hover:bg-secondary/80">
                        <Tag className="mr-1 h-3 w-3" />
                        {tag.name}
                      </Badge>
                    </Link>
                  ))
                : null}

              {showTags && (article.tags?.length || 0) > 3 ? (
                <Badge variant="secondary">+{(article.tags?.length || 0) - 3}</Badge>
              ) : null}
            </div>
          </CardContent>
        </div>
      </div>
    </Card>
  );
}
