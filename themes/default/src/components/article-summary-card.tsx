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
import {
  getArticleUrl,
  getCategoryUrl,
  getTagUrl,
  highlightSearchText,
  type NotevaArticle,
} from "@/hooks/useNoteva";
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
  const articleUrl = getArticleUrl(article);
  const stats = {
    views: article.viewCount || 0,
    likes: article.likeCount || 0,
    comments: article.commentCount || 0,
  };
  const thumbnail = article.thumbnail || article.coverImage || "";
  const publishedAt = article.publishedAt || article.createdAt;
  const excerpt = article.excerpt || stripMarkup(article.content).trim().slice(0, 180);
  const isPinned = Boolean(article.isPinned);
  const formattedDate = publishedAt ? formatArticleDate(publishedAt, dateLocale) : "";
  const highlightedTitle = highlightSearchText(article.title, highlightQuery);
  const highlightedExcerpt = highlightSearchText(excerpt, highlightQuery);

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
          <CardHeader className="space-y-3 pb-2">
            <div className="flex min-w-0 items-start gap-2">
              {isPinned ? (
                <Badge variant="destructive" className="mt-0.5 shrink-0 gap-1">
                  <Pin className="h-3 w-3" />
                  {t("article.pinned")}
                </Badge>
              ) : null}
              <CardTitle className="min-w-0 flex-1 text-lg leading-snug md:text-xl">
                <Link
                  to={articleUrl}
                  className="decoration-primary/40 underline-offset-4 transition-colors hover:text-primary hover:underline"
                  onFocus={onWarmRoute}
                  onMouseEnter={onWarmRoute}
                >
                  <span dangerouslySetInnerHTML={{ __html: highlightedTitle }} />
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
              <p
                className="mb-4 line-clamp-2 text-[15px] leading-7 text-muted-foreground"
                dangerouslySetInnerHTML={{ __html: highlightedExcerpt }}
              />
            ) : null}

            <div className="flex flex-wrap items-center gap-2">
              {showCategory && article.category ? (
                <Link to={getCategoryUrl(article.category)}>
                  <Badge variant="outline" className="hover:bg-secondary">
                    <Folder className="mr-1 h-3 w-3" />
                    {article.category.name}
                  </Badge>
                </Link>
              ) : null}

              {showTags
                ? article.tags?.slice(0, 3).map((tag) => (
                    <Link key={tag.id} to={getTagUrl(tag)}>
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
