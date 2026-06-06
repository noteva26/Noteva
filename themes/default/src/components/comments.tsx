import {
  AnimatePresence,
  motion,
} from "motion/react";
import {
  useCallback,
  useEffect,
  useMemo,
  useRef,
  useState,
  useTransition,
} from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import PluginSlot from "@/components/plugin-slot";
import {
  AlertCircle,
  CheckCircle2,
  Heart,
  Loader2,
  MessageSquare,
  Send,
} from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { waitForNoteva } from "@/hooks/useNoteva";
import { EmojiPicker } from "@/components/emoji-picker";
import Markdown from "react-markdown";
import { themeCollapseMotion, themeSpring } from "@/lib/motion";
import { cn } from "@/lib/utils";

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

interface CaptchaState {
  provider: "none" | "noteva_pow" | "turnstile" | "hcaptcha" | "cap";
  siteKey: string;
  enabled: boolean;
}

type CaptchaStatus =
  | "idle"
  | "loading"
  | "required"
  | "ready"
  | "expired"
  | "error";

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
  parentId: number | undefined,
  comment: Comment
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
        replies: addCommentToTree(item.replies, parentId, comment),
      };
    }

    return item;
  });
}

function removeCommentFromTree(comments: Comment[], id: number): Comment[] {
  return comments
    .filter((item) => item.id !== id)
    .map((item) => ({
      ...item,
      replies: item.replies?.length
        ? removeCommentFromTree(item.replies, id)
        : item.replies,
    }));
}

function replaceCommentInTree(
  comments: Comment[],
  id: number,
  nextComment: Comment
): Comment[] {
  return comments.map((item) => {
    if (item.id === id) {
      return {
        ...nextComment,
        replies: nextComment.replies?.length
          ? nextComment.replies
          : item.replies || [],
      };
    }

    return {
      ...item,
      replies: item.replies?.length
        ? replaceCommentInTree(item.replies, id, nextComment)
        : item.replies,
    };
  });
}

function updateCommentInTree(
  comments: Comment[],
  id: number,
  updater: (comment: Comment) => Comment
): Comment[] {
  return comments.map((item) => {
    if (item.id === id) return updater(item);

    return {
      ...item,
      replies: item.replies?.length
        ? updateCommentInTree(item.replies, id, updater)
        : item.replies,
    };
  });
}

function findCommentById(comments: Comment[], id: number): Comment | null {
  for (const item of comments) {
    if (item.id === id) return item;
    const found = findCommentById(item.replies || [], id);
    if (found) return found;
  }
  return null;
}

function getCommentDate(comment: Comment) {
  const value = comment.createdAt;
  return value ? new Date(value).toLocaleDateString() : "";
}

function getCommentIndentClass(depth: number) {
  return cn(
    "comment-tree-item",
    depth === 0 ? "comment-tree-root" : "comment-tree-reply",
    depth > MAX_NESTING_DEPTH ? "comment-tree-reply-compact" : ""
  );
}

function countComments(items: Comment[]): number {
  return items.reduce(
    (total, item) => total + 1 + countComments(item.replies || []),
    0
  );
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

function getErrorCode(value: unknown): string | null {
  if (typeof value !== "object" || value === null) return null;
  const record = value as Record<string, unknown>;
  if (typeof record.code === "string" && record.code.trim()) {
    return record.code;
  }
  return getErrorCode(record.data) || getErrorCode(record.error);
}

function getSubmitErrorMessage(error: unknown, fallback: string) {
  return readErrorMessage(error) || fallback;
}

function getCaptchaFailureStatus(error: unknown): CaptchaStatus {
  const code = getErrorCode(error);
  const message = (readErrorMessage(error) || "").toLowerCase();
  if (code === "VALIDATION_ERROR" || message.includes("required")) {
    return "required";
  }
  if (message.includes("expired")) {
    return "expired";
  }
  if (message.includes("captcha") || message.includes("verification")) {
    return "error";
  }
  return "error";
}

function getCaptchaStatusText(
  status: CaptchaStatus,
  t: ReturnType<typeof useTranslation>["t"],
  provider?: CaptchaState["provider"],
) {
  if (status === "ready") return t("common.success");
  if (status === "expired") return t("comment.captchaExpired");
  if (status === "error") return t("comment.captchaLoadFailed");
  if (status === "required") {
    return t("comment.captchaRequired");
  }
  if (provider === "cap") return t("comment.captchaSolving");
  return t("comment.captchaLoading");
}

export function Comments({ articleId, authorId }: CommentsProps) {
  const { t, locale } = useTranslation();
  const mountedRef = useRef(false);
  const captchaContainerRef = useRef<HTMLDivElement>(null);
  const rootTextareaRef = useRef<HTMLTextAreaElement>(null);
  const replyTextareaRef = useRef<HTMLTextAreaElement>(null);
  const [user, setUser] = useState<CurrentUser>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [comments, setComments] = useState<Comment[]>([]);
  const [loading, setLoading] = useState(true);
  const [replyTo, setReplyTo] = useState<number | null>(null);
  const [pendingLikeIds, setPendingLikeIds] = useState<Set<number>>(() => new Set());
  const [captcha, setCaptcha] = useState<CaptchaState | null>(null);
  const [captchaStatus, setCaptchaStatus] = useState<CaptchaStatus>("idle");
  const [captchaToken, setCaptchaToken] = useState("");
  const [captchaRenderKey, setCaptchaRenderKey] = useState(0);
  const [isSubmitting, startSubmitTransition] = useTransition();

  const [form, setForm] = useState<CommentFormState>(EMPTY_FORM);
  const captchaLabels = useMemo(
    () => ({
      initial: t("comment.captchaPowRequired"),
      required: t("comment.captchaPowRequired"),
      verifying: t("comment.captchaSolving"),
      verified: t("comment.captchaPowVerified"),
      retry: t("comment.captchaPowRetry"),
      expired: t("comment.captchaPowExpired"),
      error: t("comment.captchaPowRetry"),
      verifyAria: t("comment.captchaPowRequired"),
      verifyingAria: t("comment.captchaSolving"),
      verifiedAria: t("comment.captchaPowVerified"),
      errorAria: t("comment.captchaPowRetry"),
      brand: "Noteva",
    }),
    [locale]
  );

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

  useEffect(() => {
    if (!replyTo) return;

    const id = window.setTimeout(() => {
      replyTextareaRef.current?.focus();
    }, 60);

    return () => window.clearTimeout(id);
  }, [replyTo]);

  useEffect(() => {
    let active = true;

    const loadCaptcha = async () => {
      const Noteva = await waitForNoteva();
      if (!active || !mountedRef.current || !Noteva?.captcha) return;

      try {
        const config = await Noteva.captcha.getConfig();
        if (!active || !mountedRef.current) return;

        setCaptcha({
          provider: config.provider,
          siteKey: config.siteKey,
          enabled: Boolean(
            config.enabled &&
              (config.provider === "noteva_pow" || config.siteKey)
          ),
        });
        setCaptchaStatus(
          config.enabled
            ? config.provider === "noteva_pow" || config.siteKey
                ? "loading"
                : "idle"
            : "idle"
        );
      } catch {
        if (active && mountedRef.current) {
          setCaptcha(null);
          setCaptchaStatus("idle");
        }
      }
    };

    void loadCaptcha();

    return () => {
      active = false;
    };
  }, []);

  useEffect(() => {
    if (
      !captcha?.enabled ||
      captcha.provider === "none" ||
      !captchaContainerRef.current
    ) {
      setCaptchaStatus("idle");
      return undefined;
    }

    let cancelled = false;
    const provider =
      captcha.provider === "noteva_pow" ||
      captcha.provider === "turnstile" ||
      captcha.provider === "hcaptcha" ||
      captcha.provider === "cap"
        ? captcha.provider
        : undefined;

    if (!provider) {
      setCaptchaStatus("error");
      return undefined;
    }

    const renderCaptcha = async () => {
      const Noteva = await waitForNoteva();
      if (cancelled || !Noteva?.captcha || !captchaContainerRef.current) return;

      try {
        setCaptchaStatus("loading");
        await Noteva.captcha.render(captchaContainerRef.current, {
          siteKey: captcha.siteKey,
          provider,
          action: "comment",
          labels: captchaLabels,
          locale,
          callback: (token: string) => {
            setCaptchaToken(token);
            setCaptchaStatus("ready");
          },
          progressCallback: () => {
            setCaptchaToken("");
            setCaptchaStatus("loading");
          },
          resetCallback: () => {
            setCaptchaToken("");
            setCaptchaStatus("required");
          },
          expiredCallback: () => {
            setCaptchaToken("");
            setCaptchaStatus("expired");
          },
          errorCallback: () => {
            setCaptchaToken("");
            setCaptchaStatus("error");
          },
        });
        if (provider === "noteva_pow" || provider === "cap") {
          setCaptchaStatus("required");
        }
      } catch {
        if (!cancelled && mountedRef.current) {
          setCaptchaStatus("error");
          toast.error(t("comment.captchaLoadFailed"));
        }
      }
    };

    void renderCaptcha();

    return () => {
      cancelled = true;
      void waitForNoteva().then((Noteva) => {
        if (Noteva?.captcha && captchaContainerRef.current) {
          Noteva.captcha.destroy(captchaContainerRef.current);
        }
      });
    };
  }, [captcha?.enabled, captcha?.provider, captcha?.siteKey, captchaLabels, captchaRenderKey]);

  const resetCaptcha = useCallback(async () => {
    if (!captcha?.enabled) return;

    const Noteva = await waitForNoteva();
    if (!Noteva?.captcha) return;

    Noteva.captcha.reset(captchaContainerRef.current || undefined);
    setCaptchaToken("");
    setCaptchaStatus("loading");
    setCaptchaRenderKey((current) => current + 1);
  }, [captcha?.enabled, captcha?.provider]);

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

    let submittedCaptchaToken: string | undefined;
    if (captcha?.enabled) {
      submittedCaptchaToken =
        captchaToken ||
        Noteva.captcha?.getToken(captchaContainerRef.current || undefined) ||
        undefined;
      if (!submittedCaptchaToken) {
        setCaptchaStatus("required");
        toast.error(t("comment.captchaRequired"));
        return;
      }
    }

    startSubmitTransition(async () => {
      const tempId = -Date.now();
      const pendingComment: Comment = {
        id: tempId,
        content: submitted.content,
        createdAt: new Date().toISOString(),
        nickname: isAdmin
          ? user?.displayName || user?.username || t("comment.admin")
          : submitted.nickname,
        avatarUrl: user?.avatar,
        likeCount: 0,
        isLiked: false,
        isAuthor: isAdmin,
        userId: user?.id ?? null,
        replies: [],
        pending: true,
      };

      setComments((current) => addCommentToTree(current, parentId, pendingComment));

      try {
        const created = await Noteva.comments.create({
          articleId,
          content: submitted.content,
          parentId,
          nickname: !isAdmin ? submitted.nickname : undefined,
          email: !isAdmin ? submitted.email || undefined : undefined,
          captchaToken: submittedCaptchaToken,
        });
        toast.success(t("comment.submitSuccess"));
        setForm(EMPTY_FORM);
        setReplyTo(null);
        await resetCaptcha();
        if (created?.status === "approved") {
          setComments((current) =>
            replaceCommentInTree(current, tempId, {
              ...created,
              replies: [],
              pending: false,
            })
          );
          await loadComments();
        } else {
          setComments((current) => removeCommentFromTree(current, tempId));
        }
      } catch (error) {
        setComments((current) => removeCommentFromTree(current, tempId));
        setCaptchaStatus(getCaptchaFailureStatus(error));
        await resetCaptcha();
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
    if (targetType === "comment" && pendingLikeIds.has(targetId)) return;

    let previousComment: Comment | null = null;
    if (targetType === "comment") {
      previousComment = findCommentById(comments, targetId);
      if (!previousComment) return;

      setPendingLikeIds((current) => new Set(current).add(targetId));
      setComments((current) =>
        updateCommentInTree(current, targetId, (comment) => {
          const wasLiked = Boolean(comment.isLiked);
          return {
            ...comment,
            isLiked: !wasLiked,
            likeCount: Math.max(0, (comment.likeCount || 0) + (wasLiked ? -1 : 1)),
          };
        })
      );
    }

    try {
      const result = await Noteva.interactions.like(targetType, targetId);
      if (targetType === "comment") {
        setComments((current) =>
          updateCommentInTree(current, targetId, (comment) => ({
            ...comment,
            isLiked: result.liked,
            likeCount:
              result.likeCount > 0
                ? result.likeCount
                : Math.max(0, comment.likeCount || 0),
          }))
        );
      }
      toast.success(result.liked ? t("comment.liked") : t("comment.unliked"));
    } catch {
      if (targetType === "comment" && previousComment) {
        setComments((current) =>
          updateCommentInTree(current, targetId, () => previousComment as Comment)
        );
      }
      toast.error(t("comment.likeFailed"));
    } finally {
      if (targetType === "comment") {
        setPendingLikeIds((current) => {
          const next = new Set(current);
          next.delete(targetId);
          return next;
        });
      }
    }
  };

  const isAuthorComment = (comment: Comment) => {
    if (comment.isAuthor) return true;
    if (comment.userId && authorId && comment.userId === authorId) return true;
    return false;
  };

  const handleReplyToggle = (comment: Comment) => {
    setReplyTo((current) => (current === comment.id ? null : comment.id));
  };

  const handleCancelReply = () => {
    setReplyTo(null);
    window.setTimeout(() => {
      rootTextareaRef.current?.focus();
    }, 60);
  };

  const renderComment = (comment: Comment, depth = 0) => {
    const isLiked = comment.isLiked ?? false;
    const likeCount = comment.likeCount ?? 0;
    const likeLoading = pendingLikeIds.has(comment.id);

    return (
      <motion.div
        key={comment.id}
        data-comment-id={comment.id}
        className={cn(
          getCommentIndentClass(depth),
          comment.pending && "comment-item-pending"
        )}
        initial={{ opacity: 0, y: 8 }}
        animate={{ opacity: 1, y: 0 }}
        transition={themeSpring}
      >
        <div className="comment-item-body flex gap-3">
          <img
            src={comment.avatarUrl || FALLBACK_AVATAR}
            alt={comment.nickname || t("comment.anonymous")}
            className="comment-avatar h-10 w-10 rounded-full"
            onError={(event) => {
              event.currentTarget.src = FALLBACK_AVATAR;
            }}
          />
          <div className="flex-1">
            <div className="flex items-center gap-2 comment-meta">
              <span className="font-medium">
                {comment.nickname || t("comment.anonymous")}
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
            <div className="comment-actions mt-2 flex items-center gap-4">
              <button
                onClick={() => handleLike("comment", comment.id)}
                disabled={comment.pending || likeLoading}
                className={cn(
                  "comment-action-button flex items-center gap-1 text-sm text-muted-foreground hover:text-red-500 disabled:pointer-events-none disabled:opacity-50",
                  isLiked && "is-liked text-red-500",
                  likeLoading && "is-loading"
                )}
              >
                {likeLoading ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <Heart className={cn("h-4 w-4", isLiked && "fill-current")} />
                )}
                {likeCount}
              </button>
              <button
                onClick={() => handleReplyToggle(comment)}
                disabled={comment.pending}
                className="comment-action-button flex items-center gap-1 text-sm text-muted-foreground hover:text-primary disabled:pointer-events-none disabled:opacity-50"
              >
                <MessageSquare className="h-4 w-4" />
                {t("comment.reply")}
              </button>
            </div>

            <AnimatePresence initial={false}>
              {replyTo === comment.id && (
                <motion.div
                  {...themeCollapseMotion}
                  className="overflow-hidden"
                >
                  <div className="comment-reply-panel mt-3 space-y-2">
                    <div className="comment-reply-target">
                      {t("comment.replyingTo", {
                        name: comment.nickname || t("comment.anonymous"),
                      })}
                    </div>
                    <Textarea
                      ref={replyTextareaRef}
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
                        onClick={handleCancelReply}
                      >
                        {t("common.cancel")}
                      </Button>
                    </div>
                  </div>
                </motion.div>
              )}
            </AnimatePresence>
          </div>
        </div>

        {comment.replies?.map((reply) => renderComment(reply, depth + 1))}
      </motion.div>
    );
  };

  const captchaPanel = captcha?.enabled ? captcha : null;
  const isPowCaptcha = captchaPanel?.provider === "noteva_pow";
  const isCapCaptcha = captchaPanel?.provider === "cap";

  return (
    <Card className="mt-8" data-article-id={articleId}>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <MessageSquare className="h-5 w-5" />
          {t("comment.title")} ({countComments(comments)})
        </CardTitle>
      </CardHeader>
      <CardContent>
        <PluginSlot name="comment_form_before" />

        <div className="space-y-3">
          <div className="relative">
            <Textarea
              ref={rootTextareaRef}
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
                name: user?.displayName || user?.username || t("comment.admin"),
              })}
            </p>
          )}
          {captchaPanel ? (
            <div
              className={cn(
                "comment-captcha-panel",
                isPowCaptcha && "comment-captcha-panel-pow",
                isCapCaptcha && "comment-captcha-panel-cap",
                !isPowCaptcha && "min-h-16",
                !isPowCaptcha && `comment-captcha-panel-${captchaStatus}`
              )}
              data-captcha-status={captchaStatus}
              data-captcha-provider={captchaPanel.provider}
            >
              <div
                className="comment-captcha-widget"
                ref={captchaContainerRef}
              />
              {!isPowCaptcha ? (
                <div className="comment-captcha-status" aria-live="polite">
                  {captchaStatus === "error" ||
                  captchaStatus === "expired" ||
                  captchaStatus === "required" ? (
                    <AlertCircle className="h-3.5 w-3.5" />
                  ) : captchaStatus === "ready" ? (
                    <CheckCircle2 className="h-3.5 w-3.5" />
                  ) : (
                    <Loader2 className="h-3.5 w-3.5 animate-spin" />
                  )}
                  <span>{getCaptchaStatusText(captchaStatus, t, captchaPanel.provider)}</span>
                </div>
              ) : null}
            </div>
          ) : null}
          <Button
            onClick={() => handleSubmit()}
            disabled={isSubmitting}
          >
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
          ) : comments.length === 0 ? (
            <p className="py-4 text-muted-foreground">
              {t("comment.noComments")}
            </p>
          ) : (
            comments.map((comment) => renderComment(comment))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
