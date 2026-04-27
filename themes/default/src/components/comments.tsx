import {
  useCallback,
  useEffect,
  useOptimistic,
  useRef,
  useState,
  useTransition,
} from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import PluginSlot from "@/components/plugin-slot";
import { Heart, Loader2, MessageSquare, Send } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { waitForNoteva } from "@/hooks/useNoteva";
import { EmojiPicker } from "@/components/emoji-picker";
import Markdown from "react-markdown";

const FALLBACK_AVATAR = "https://www.gravatar.com/avatar/?d=mp&s=80";
const MAX_NESTING_DEPTH = 4;

interface Comment {
  id: number;
  content: string;
  createdAt?: string;
  nickname?: string | null;
  avatarUrl?: string;
  likeCount?: number;
  isLiked?: boolean;
  isAuthor?: boolean;
  userId?: number | null;
  replies?: Comment[];
  pending?: boolean;
}

interface CommentsProps {
  articleId: number;
  authorId?: number;
}

interface CommentFormState {
  nickname: string;
  email: string;
  content: string;
}

interface OptimisticCommentAction {
  parentId?: number;
  comment: Comment;
}

type CurrentUser = Awaited<
  ReturnType<NonNullable<typeof window.Noteva>["user"]["check"]>
>;

const EMPTY_FORM: CommentFormState = {
  nickname: "",
  email: "",
  content: "",
};

function addCommentToTree(
  comments: Comment[],
  { parentId, comment }: OptimisticCommentAction
): Comment[] {
  if (!parentId) {
    return [...comments, comment];
  }

  return comments.map((item) => {
    if (item.id === parentId) {
      return { ...item, replies: [...(item.replies || []), comment] };
    }

    if (item.replies?.length) {
      return {
        ...item,
        replies: addCommentToTree(item.replies, { parentId, comment }),
      };
    }

    return item;
  });
}

function getCommentDate(comment: Comment) {
  const value = comment.createdAt;
  return value ? new Date(value).toLocaleDateString() : "";
}

function getCommentIndentClass(depth: number) {
  if (depth > MAX_NESTING_DEPTH) {
    return "mt-3 pl-4 border-l-2 border-muted";
  }

  if (depth > 0) {
    return "ml-6 mt-3 pl-4 border-l-2 border-muted";
  }

  return "mt-4";
}

function readErrorMessage(value: unknown): string | null {
  if (typeof value === "string") {
    return value.trim() ? value : null;
  }

  if (typeof value !== "object" || value === null) {
    return null;
  }

  const record = value as Record<string, unknown>;
  const direct = readErrorMessage(record.message);
  if (direct) return direct;

  const error = readErrorMessage(record.error);
  if (error) return error;

  return readErrorMessage(record.data);
}

function getSubmitErrorMessage(error: unknown, fallback: string) {
  return readErrorMessage(error) || fallback;
}

export function Comments({ articleId, authorId }: CommentsProps) {
  const { t } = useTranslation();
  const mountedRef = useRef(false);
  const [user, setUser] = useState<CurrentUser>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [comments, setComments] = useState<Comment[]>([]);
  const [loading, setLoading] = useState(true);
  const [replyTo, setReplyTo] = useState<number | null>(null);
  const [isSubmitting, startSubmitTransition] = useTransition();
  const [optimisticComments, addOptimisticComment] = useOptimistic(
    comments,
    addCommentToTree
  );

  const [form, setForm] = useState<CommentFormState>(EMPTY_FORM);

  useEffect(() => {
    mountedRef.current = true;

    return () => {
      mountedRef.current = false;
    };
  }, []);

  useEffect(() => {
    let active = true;

    const checkUser = async () => {
      const Noteva = await waitForNoteva();
      if (!active || !mountedRef.current) return;

      if (!Noteva) {
        setUser(null);
        setIsAdmin(false);
        return;
      }

      try {
        const currentUser = await Noteva.user.check();
        if (!active || !mountedRef.current) return;

        setUser(currentUser);
        setIsAdmin(currentUser?.role === "admin");
      } catch {
        if (!active || !mountedRef.current) return;

        setUser(null);
        setIsAdmin(false);
      }
    };

    void checkUser();

    return () => {
      active = false;
    };
  }, []);

  const loadComments = useCallback(async () => {
    const Noteva = await waitForNoteva();
    if (!Noteva) {
      if (mountedRef.current) {
        setComments([]);
        setLoading(false);
      }
      return;
    }

    try {
      const result = await Noteva.comments.list(articleId);
      if (mountedRef.current) {
        setComments(result || []);
      }
    } catch {
      if (mountedRef.current) {
        setComments([]);
      }
    } finally {
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, [articleId]);

  useEffect(() => {
    if (mountedRef.current) {
      setLoading(true);
    }

    void loadComments();
  }, [loadComments]);

  const handleSubmit = async (parentId?: number) => {
    const submitted = {
      nickname: form.nickname.trim(),
      email: form.email.trim(),
      content: form.content,
    };

    if (!submitted.content.trim()) {
      toast.error(t("comment.contentRequired"));
      return;
    }

    if (!isAdmin && !submitted.nickname) {
      toast.error(t("comment.nicknameRequired"));
      return;
    }

    const Noteva = await waitForNoteva();
    if (!Noteva) return;

    startSubmitTransition(async () => {
      addOptimisticComment({
        parentId,
        comment: {
          id: -Date.now(),
          content: submitted.content,
          createdAt: new Date().toISOString(),
          nickname: isAdmin
            ? user?.displayName || user?.username || "Admin"
            : submitted.nickname,
          avatarUrl: user?.avatar,
          likeCount: 0,
          isLiked: false,
          isAuthor: isAdmin,
          userId: user?.id ?? null,
          replies: [],
          pending: true,
        },
      });

      try {
        await Noteva.comments.create({
          articleId,
          content: submitted.content,
          parentId,
          nickname: !isAdmin ? submitted.nickname : undefined,
          email: !isAdmin ? submitted.email || undefined : undefined,
        });
        toast.success(t("comment.submitSuccess"));
        setForm(EMPTY_FORM);
        setReplyTo(null);
        await loadComments();

      } catch (error) {
        setComments((current) => [...current]);
        toast.error(getSubmitErrorMessage(error, t("comment.submitFailed")));
      }
    });
  };

  const handleLike = async (
    targetType: "article" | "comment",
    targetId: number
  ) => {
    const Noteva = await waitForNoteva();
    if (!Noteva) return;

    try {
      const result = await Noteva.interactions.like(targetType, targetId);
      if (targetType === "comment") {
        await loadComments();
      }
      toast.success(result.liked ? t("comment.liked") : t("comment.unliked"));
    } catch {
      toast.error(t("comment.likeFailed"));
    }
  };

  const isAuthorComment = (comment: Comment) => {
    if (comment.isAuthor) return true;
    if (comment.userId && authorId && comment.userId === authorId) return true;
    return false;
  };

  const renderComment = (comment: Comment, depth = 0) => {
    const isLiked = comment.isLiked ?? false;
    const likeCount = comment.likeCount ?? 0;

    return (
      <div
        key={comment.id}
        data-comment-id={comment.id}
        className={getCommentIndentClass(depth)}
      >
        <div className="flex gap-3">
          <img
            src={comment.avatarUrl || FALLBACK_AVATAR}
            alt={comment.nickname || "User"}
            className="h-10 w-10 rounded-full"
            onError={(event) => {
              event.currentTarget.src = FALLBACK_AVATAR;
            }}
          />
          <div className="flex-1">
            <div className="flex items-center gap-2 comment-meta">
              <span className="font-medium">
                {comment.nickname || "Anonymous"}
              </span>
              {isAuthorComment(comment) && (
                <span className="rounded bg-primary px-1.5 py-0.5 text-xs font-medium text-primary-foreground">
                  {t("comment.authorTag")}
                </span>
              )}
              {comment.pending && (
                <Loader2 className="h-3 w-3 animate-spin text-muted-foreground" />
              )}
              <span className="text-sm text-muted-foreground">
                {getCommentDate(comment)}
              </span>
            </div>
            <div className="mt-1 max-w-none text-sm prose prose-sm dark:prose-invert prose-p:my-1 prose-pre:my-1 comment-content">
              <Markdown>{comment.content}</Markdown>
            </div>
            <div className="mt-2 flex items-center gap-4 comment-actions">
              <button
                onClick={() => handleLike("comment", comment.id)}
                disabled={comment.pending}
                className={`flex items-center gap-1 text-sm ${
                  isLiked ? "text-red-500" : "text-muted-foreground"
                } hover:text-red-500 disabled:pointer-events-none disabled:opacity-50`}
              >
                <Heart className={`h-4 w-4 ${isLiked ? "fill-current" : ""}`} />
                {likeCount}
              </button>
              <button
                onClick={() =>
                  setReplyTo(replyTo === comment.id ? null : comment.id)
                }
                disabled={comment.pending}
                className="flex items-center gap-1 text-sm text-muted-foreground hover:text-primary disabled:pointer-events-none disabled:opacity-50"
              >
                <MessageSquare className="h-4 w-4" />
                {t("comment.reply")}
              </button>
            </div>

            {replyTo === comment.id && (
              <div className="mt-3 space-y-2">
                <Textarea
                  placeholder={t("comment.replyPlaceholder")}
                  value={form.content}
                  onChange={(event) =>
                    setForm((current) => ({
                      ...current,
                      content: event.target.value,
                    }))
                  }
                  rows={2}
                />
                {!isAdmin && (
                  <div className="flex gap-2">
                    <Input
                      placeholder={t("comment.nickname")}
                      value={form.nickname}
                      onChange={(event) =>
                        setForm((current) => ({
                          ...current,
                          nickname: event.target.value,
                        }))
                      }
                    />
                    <Input
                      placeholder={t("comment.email")}
                      value={form.email}
                      onChange={(event) =>
                        setForm((current) => ({
                          ...current,
                          email: event.target.value,
                        }))
                      }
                    />
                  </div>
                )}
                <div className="flex gap-2">
                  <Button
                    size="sm"
                    onClick={() => handleSubmit(comment.id)}
                    disabled={isSubmitting}
                  >
                    {isSubmitting && (
                      <Loader2 className="mr-1 h-4 w-4 animate-spin" />
                    )}
                    {t("comment.submit")}
                  </Button>
                  <Button
                    size="sm"
                    variant="ghost"
                    onClick={() => setReplyTo(null)}
                  >
                    {t("common.cancel")}
                  </Button>
                </div>
              </div>
            )}
          </div>
        </div>

        {comment.replies?.map((reply) => renderComment(reply, depth + 1))}
      </div>
    );
  };

  return (
    <Card className="mt-8" data-article-id={articleId}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <MessageSquare className="h-5 w-5" />
          {t("comment.title")} ({optimisticComments.length})
        </CardTitle>
      </CardHeader>
      <CardContent>
        <PluginSlot name="comment_form_before" />

        <div className="space-y-3">
          <div className="relative">
            <Textarea
              placeholder={t("comment.placeholder")}
              value={form.content}
              onChange={(event) =>
                setForm((current) => ({
                  ...current,
                  content: event.target.value,
                }))
              }
              rows={3}
            />
            <div className="absolute bottom-2 right-2">
              <EmojiPicker
                onSelect={(emoji) =>
                  setForm((current) => ({
                    ...current,
                    content: current.content + emoji,
                  }))
                }
              />
            </div>
          </div>
          {!isAdmin && (
            <div className="flex gap-2">
              <Input
                placeholder={t("comment.nickname")}
                value={form.nickname}
                onChange={(event) =>
                  setForm((current) => ({
                    ...current,
                    nickname: event.target.value,
                  }))
                }
              />
              <Input
                placeholder={t("comment.emailOptional")}
                value={form.email}
                onChange={(event) =>
                  setForm((current) => ({
                    ...current,
                    email: event.target.value,
                  }))
                }
              />
            </div>
          )}
          {isAdmin && (
            <p className="text-sm text-muted-foreground">
              {t("comment.postingAsAdmin", {
                name: user?.displayName || user?.username || "Admin",
              })}
            </p>
          )}
          <Button onClick={() => handleSubmit()} disabled={isSubmitting}>
            {isSubmitting ? (
              <Loader2 className="mr-2 h-4 w-4 animate-spin" />
            ) : (
              <Send className="mr-2 h-4 w-4" />
            )}
            {t("comment.submit")}
          </Button>
        </div>

        <PluginSlot name="comment_form_after" />

        <div className="mt-6 divide-y">
          {loading ? (
            <p className="py-4 text-muted-foreground">{t("common.loading")}</p>
          ) : optimisticComments.length === 0 ? (
            <p className="py-4 text-muted-foreground">
              {t("comment.noComments")}
            </p>
          ) : (
            optimisticComments.map((comment) => renderComment(comment))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
