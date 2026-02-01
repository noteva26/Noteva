"use client";

import { useState, useEffect } from "react";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Textarea } from "@/components/ui/textarea";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import PluginSlot from "@/components/plugin-slot";
import { Heart, MessageSquare, Send, Loader2 } from "lucide-react";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";
import { getNoteva } from "@/hooks/useNoteva";
import { EmojiPicker } from "@/components/emoji-picker";

interface Comment {
  id: number;
  article_id: number;
  user_id: number | null;
  parent_id: number | null;
  nickname: string | null;
  email: string | null;
  content: string;
  status: "pending" | "approved" | "spam";
  created_at: string;
  avatar_url: string;
  like_count: number;
  is_liked: boolean;
  is_author?: boolean;
  replies?: Comment[];
}

interface CommentsProps {
  articleId: number;
  authorId?: number;
}

export function Comments({ articleId, authorId }: CommentsProps) {
  const { t } = useTranslation();
  const [user, setUser] = useState<any>(null);
  const [isAdmin, setIsAdmin] = useState(false);
  const [comments, setComments] = useState<Comment[]>([]);
  const [loading, setLoading] = useState(true);
  const [submitting, setSubmitting] = useState(false);
  const [replyTo, setReplyTo] = useState<number | null>(null);
  
  const [form, setForm] = useState({
    nickname: "",
    email: "",
    content: "",
  });

  useEffect(() => {
    // 检查用户登录状态
    const checkUser = async () => {
      const Noteva = getNoteva();
      if (!Noteva) {
        setTimeout(checkUser, 50);
        return;
      }
      try {
        const currentUser = await Noteva.user.check();
        setUser(currentUser);
        setIsAdmin(currentUser?.role === "admin");
      } catch {
        setUser(null);
        setIsAdmin(false);
      }
    };
    checkUser();
  }, []);

  useEffect(() => {
    loadComments();
  }, [articleId]);

  const loadComments = async () => {
    const Noteva = getNoteva();
    if (!Noteva) {
      setTimeout(loadComments, 50);
      return;
    }

    try {
      const result = await Noteva.api.get(`/comments/${articleId}`);
      setComments(result.comments || []);
    } catch (err) {
      console.error("Failed to load comments:", err);
    } finally {
      setLoading(false);
    }
  };

  const handleSubmit = async (parentId?: number) => {
    if (!form.content.trim()) {
      toast.error(t("comment.contentRequired"));
      return;
    }
    
    // 游客需要填写昵称，管理员不需要
    if (!isAdmin && !form.nickname.trim()) {
      toast.error(t("comment.nicknameRequired"));
      return;
    }

    const Noteva = getNoteva();
    if (!Noteva) return;

    setSubmitting(true);
    try {
      const input: any = {
        article_id: articleId,
        content: form.content,
        parent_id: parentId,
      };
      
      // 游客模式：使用表单中的昵称和邮箱
      // 管理员模式：后端会自动使用登录用户信息
      if (!isAdmin) {
        input.nickname = form.nickname;
        input.email = form.email;
      }

      await Noteva.api.post('/comments', input);
      toast.success(t("comment.submitSuccess"));
      setForm({ nickname: "", email: "", content: "" });
      setReplyTo(null);
      loadComments();
      
      // 触发评论创建后钩子（用于插件如"回复可见"）
      Noteva.hooks.trigger("comment_after_create", { articleId, parentId });
      Noteva.events.emit("comment:create", { articleId, parentId });
    } catch (err: any) {
      toast.error(err.data?.error || t("comment.submitFailed"));
    } finally {
      setSubmitting(false);
    }
  };

  const handleLike = async (targetType: "article" | "comment", targetId: number) => {
    const Noteva = getNoteva();
    if (!Noteva) return;

    try {
      const result = await Noteva.api.post('/like', { 
        target_type: targetType, 
        target_id: targetId 
      });
      if (targetType === "comment") {
        loadComments();
      }
      toast.success(result.liked ? t("comment.liked") : t("comment.unliked"));
    } catch (err) {
      toast.error(t("comment.likeFailed"));
    }
  };

  // 判断是否为作者评论
  const isAuthorComment = (comment: Comment) => {
    // 后端返回的 is_author 标识
    if (comment.is_author) return true;
    // 或者 user_id 匹配文章作者
    if (comment.user_id && authorId && comment.user_id === authorId) return true;
    return false;
  };

  const renderComment = (comment: Comment, isReply = false) => (
    <div key={comment.id} className={`${isReply ? "ml-8 mt-4" : "mt-4"}`}>
      <div className="flex gap-3">
        <img
          src={comment.avatar_url}
          alt={comment.nickname || "User"}
          className="w-10 h-10 rounded-full"
        />
        <div className="flex-1">
          <div className="flex items-center gap-2">
            <span className="font-medium">{comment.nickname || "Anonymous"}</span>
            {isAuthorComment(comment) && (
              <span className="px-1.5 py-0.5 text-xs font-medium bg-primary text-primary-foreground rounded">
                {t("comment.authorTag")}
              </span>
            )}
            <span className="text-sm text-muted-foreground">
              {new Date(comment.created_at).toLocaleDateString()}
            </span>
          </div>
          <p className="mt-1 text-sm">{comment.content}</p>
          <div className="flex items-center gap-4 mt-2">
            <button
              onClick={() => handleLike("comment", comment.id)}
              className={`flex items-center gap-1 text-sm ${comment.is_liked ? "text-red-500" : "text-muted-foreground"} hover:text-red-500`}
            >
              <Heart className={`h-4 w-4 ${comment.is_liked ? "fill-current" : ""}`} />
              {comment.like_count}
            </button>
            <button
              onClick={() => setReplyTo(replyTo === comment.id ? null : comment.id)}
              className="flex items-center gap-1 text-sm text-muted-foreground hover:text-primary"
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
                onChange={(e) => setForm((f) => ({ ...f, content: e.target.value }))}
                rows={2}
              />
              {!isAdmin && (
                <div className="flex gap-2">
                  <Input
                    placeholder={t("comment.nickname")}
                    value={form.nickname}
                    onChange={(e) => setForm((f) => ({ ...f, nickname: e.target.value }))}
                  />
                  <Input
                    placeholder={t("comment.email")}
                    value={form.email}
                    onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))}
                  />
                </div>
              )}
              <div className="flex gap-2">
                <Button size="sm" onClick={() => handleSubmit(comment.id)} disabled={submitting}>
                  {submitting && <Loader2 className="h-4 w-4 mr-1 animate-spin" />}
                  {t("comment.submit")}
                </Button>
                <Button size="sm" variant="ghost" onClick={() => setReplyTo(null)}>
                  {t("common.cancel")}
                </Button>
              </div>
            </div>
          )}
        </div>
      </div>
      
      {comment.replies?.map((reply) => renderComment(reply, true))}
    </div>
  );

  return (
    <Card className="mt-8">
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <MessageSquare className="h-5 w-5" />
          {t("comment.title")} ({comments.length})
        </CardTitle>
      </CardHeader>
      <CardContent>
        {/* comment_form_before 插槽 - 登录提示、规则说明 */}
        <PluginSlot name="comment_form_before" />
        
        {/* Comment form */}
        <div className="space-y-3">
          <div className="relative">
            <Textarea
              placeholder={t("comment.placeholder")}
              value={form.content}
              onChange={(e) => setForm((f) => ({ ...f, content: e.target.value }))}
              rows={3}
            />
            <div className="absolute bottom-2 right-2">
              <EmojiPicker onSelect={(emoji) => setForm((f) => ({ ...f, content: f.content + emoji }))} />
            </div>
          </div>
          {!isAdmin && (
            <div className="flex gap-2">
              <Input
                placeholder={t("comment.nickname")}
                value={form.nickname}
                onChange={(e) => setForm((f) => ({ ...f, nickname: e.target.value }))}
              />
              <Input
                placeholder={t("comment.emailOptional")}
                value={form.email}
                onChange={(e) => setForm((f) => ({ ...f, email: e.target.value }))}
              />
            </div>
          )}
          {isAdmin && (
            <p className="text-sm text-muted-foreground">
              {t("comment.postingAsAdmin", { name: user?.display_name || user?.username })}
            </p>
          )}
          <Button onClick={() => handleSubmit()} disabled={submitting}>
            {submitting ? (
              <Loader2 className="h-4 w-4 mr-2 animate-spin" />
            ) : (
              <Send className="h-4 w-4 mr-2" />
            )}
            {t("comment.submit")}
          </Button>
        </div>
        
        {/* comment_form_after 插槽 - 表情选择器、快捷回复 */}
        <PluginSlot name="comment_form_after" />

        {/* Comments list */}
        <div className="mt-6 divide-y">
          {loading ? (
            <p className="text-muted-foreground py-4">{t("common.loading")}</p>
          ) : comments.length === 0 ? (
            <p className="text-muted-foreground py-4">{t("comment.noComments")}</p>
          ) : (
            comments.map((comment) => renderComment(comment))
          )}
        </div>
      </CardContent>
    </Card>
  );
}
