"use client";

import { useEffect, useState } from "react";
import { usersApi, UserAdmin, authApi } from "@/lib/api";
import { Button } from "@/components/ui/button";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Users, Pencil, Trash2, RefreshCw, Loader2, ChevronLeft, ChevronRight, UserCircle } from "lucide-react";
import { Avatar, AvatarFallback, AvatarImage } from "@/components/ui/avatar";
import { toast } from "sonner";
import { useTranslation } from "@/lib/i18n";

export default function UsersPage() {
  const { t } = useTranslation();
  const [users, setUsers] = useState<UserAdmin[]>([]);
  const [loading, setLoading] = useState(true);
  const [currentUserId, setCurrentUserId] = useState<number | null>(null);
  
  // Pagination
  const [page, setPage] = useState(1);
  const [totalPages, setTotalPages] = useState(1);
  const [total, setTotal] = useState(0);
  const perPage = 20;
  
  // Edit dialog
  const [editOpen, setEditOpen] = useState(false);
  const [editingUser, setEditingUser] = useState<UserAdmin | null>(null);
  const [editForm, setEditForm] = useState({ username: "", email: "", role: "", status: "", display_name: "" });
  const [saving, setSaving] = useState(false);
  
  // Delete
  const [deleting, setDeleting] = useState<number | null>(null);

  const fetchCurrentUser = async () => {
    try {
      const { data } = await authApi.me();
      setCurrentUserId(data.id);
    } catch {
      // ignore
    }
  };

  const fetchUsers = async () => {
    setLoading(true);
    try {
      const { data } = await usersApi.list({ page, per_page: perPage });
      setUsers(data?.users || []);
      setTotalPages(data?.total_pages || 1);
      setTotal(data?.total || 0);
    } catch {
      toast.error(t("error.loadFailed"));
      setUsers([]);
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchCurrentUser();
  }, []);

  useEffect(() => {
    fetchUsers();
  }, [page]);

  const openEdit = (user: UserAdmin) => {
    setEditingUser(user);
    setEditForm({
      username: user.username,
      email: user.email,
      role: user.role,
      status: user.status,
      display_name: user.display_name || "",
    });
    setEditOpen(true);
  };

  const handleSave = async () => {
    if (!editingUser) return;
    
    // Check self-ban
    if (editingUser.id === currentUserId && editForm.status === "banned") {
      toast.error(t("user.cannotBanSelf"));
      return;
    }
    
    setSaving(true);
    try {
      await usersApi.update(editingUser.id, editForm);
      toast.success(t("user.updateSuccess"));
      setEditOpen(false);
      fetchUsers();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("user.updateFailed"));
    } finally {
      setSaving(false);
    }
  };

  const handleDelete = async (user: UserAdmin) => {
    if (user.id === currentUserId) {
      toast.error(t("user.cannotDeleteSelf"));
      return;
    }
    
    if (!confirm(t("user.confirmDelete")?.replace("{name}", user.username))) {
      return;
    }
    
    setDeleting(user.id);
    try {
      await usersApi.delete(user.id);
      toast.success(t("user.deleteSuccess"));
      fetchUsers();
    } catch (error: any) {
      toast.error(error.response?.data?.error?.message || t("user.deleteFailed"));
    } finally {
      setDeleting(null);
    }
  };

  const getRoleBadge = (role: string) => {
    const variants: Record<string, "default" | "secondary" | "outline"> = {
      admin: "default",
      editor: "secondary",
      author: "outline",
    };
    const labels: Record<string, string> = {
      admin: t("user.admin"),
      editor: t("user.editor"),
      author: t("user.author"),
    };
    return <Badge variant={variants[role] || "outline"}>{labels[role] || role}</Badge>;
  };

  const getStatusBadge = (status: string) => {
    if (status === "banned") {
      return <Badge variant="destructive">{t("user.banned")}</Badge>;
    }
    return <Badge variant="secondary">{t("user.active")}</Badge>;
  };

  const formatDate = (dateStr: string) => {
    return new Date(dateStr).toLocaleDateString();
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t("user.title")}</h1>
          <p className="text-muted-foreground">{t("user.description")}</p>
        </div>
        <Button variant="outline" onClick={fetchUsers} disabled={loading}>
          <RefreshCw className={`h-4 w-4 mr-2 ${loading ? "animate-spin" : ""}`} />
          {t("common.refresh")}
        </Button>
      </div>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <Users className="h-5 w-5" />
            {t("manage.users")} ({total})
          </CardTitle>
        </CardHeader>
        <CardContent>
          {loading ? (
            <div className="space-y-3">
              {Array.from({ length: 5 }).map((_, i) => (
                <Skeleton key={i} className="h-12 w-full" />
              ))}
            </div>
          ) : users.length === 0 ? (
            <div className="text-center py-12">
              <Users className="h-12 w-12 mx-auto text-muted-foreground mb-4" />
              <p className="text-muted-foreground">{t("user.noUsers")}</p>
            </div>
          ) : (
            <>
              <Table>
                <TableHeader>
                  <TableRow>
                    <TableHead>{t("user.avatar")}</TableHead>
                    <TableHead>{t("user.username")}</TableHead>
                    <TableHead>{t("user.displayName")}</TableHead>
                    <TableHead>{t("user.email")}</TableHead>
                    <TableHead>{t("user.role")}</TableHead>
                    <TableHead>{t("user.status")}</TableHead>
                    <TableHead>{t("user.createdAt")}</TableHead>
                    <TableHead className="text-right">{t("common.edit")}</TableHead>
                  </TableRow>
                </TableHeader>
                <TableBody>
                  {users.map((user) => (
                    <TableRow key={user.id}>
                      <TableCell>
                        <Avatar className="h-8 w-8">
                          <AvatarImage src={user.avatar || undefined} alt={user.username} />
                          <AvatarFallback>
                            {(user.display_name || user.username)?.[0]?.toUpperCase()}
                          </AvatarFallback>
                        </Avatar>
                      </TableCell>
                      <TableCell className="font-medium">{user.username}</TableCell>
                      <TableCell>{user.display_name || "-"}</TableCell>
                      <TableCell>{user.email}</TableCell>
                      <TableCell>{getRoleBadge(user.role)}</TableCell>
                      <TableCell>{getStatusBadge(user.status)}</TableCell>
                      <TableCell>{formatDate(user.created_at)}</TableCell>
                      <TableCell className="text-right">
                        <div className="flex justify-end gap-1">
                          <Button variant="ghost" size="sm" onClick={() => openEdit(user)}>
                            <Pencil className="h-4 w-4" />
                          </Button>
                          <Button
                            variant="ghost"
                            size="sm"
                            onClick={() => handleDelete(user)}
                            disabled={deleting === user.id || user.id === currentUserId}
                          >
                            {deleting === user.id ? (
                              <Loader2 className="h-4 w-4 animate-spin" />
                            ) : (
                              <Trash2 className="h-4 w-4 text-destructive" />
                            )}
                          </Button>
                        </div>
                      </TableCell>
                    </TableRow>
                  ))}
                </TableBody>
              </Table>

              {totalPages > 1 && (
                <div className="flex items-center justify-between mt-4">
                  <p className="text-sm text-muted-foreground">
                    {t("pagination.page")?.replace("{current}", String(page)).replace("{total}", String(totalPages))}
                  </p>
                  <div className="flex gap-2">
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setPage(p => Math.max(1, p - 1))}
                      disabled={page <= 1}
                    >
                      <ChevronLeft className="h-4 w-4" />
                      {t("pagination.prev")}
                    </Button>
                    <Button
                      variant="outline"
                      size="sm"
                      onClick={() => setPage(p => Math.min(totalPages, p + 1))}
                      disabled={page >= totalPages}
                    >
                      {t("pagination.next")}
                      <ChevronRight className="h-4 w-4" />
                    </Button>
                  </div>
                </div>
              )}
            </>
          )}
        </CardContent>
      </Card>

      {/* Edit Dialog */}
      <Dialog open={editOpen} onOpenChange={setEditOpen}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle>{t("user.editUser")}</DialogTitle>
            <DialogDescription>{editingUser?.username}</DialogDescription>
          </DialogHeader>
          
          <div className="space-y-4 py-4">
            <div className="space-y-2">
              <Label htmlFor="username">{t("user.username")}</Label>
              <Input
                id="username"
                value={editForm.username}
                onChange={(e) => setEditForm(f => ({ ...f, username: e.target.value }))}
              />
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="display_name">{t("user.displayName")}</Label>
              <Input
                id="display_name"
                value={editForm.display_name}
                onChange={(e) => setEditForm(f => ({ ...f, display_name: e.target.value }))}
                placeholder={t("user.displayNamePlaceholder")}
              />
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="email">{t("user.email")}</Label>
              <Input
                id="email"
                type="email"
                value={editForm.email}
                onChange={(e) => setEditForm(f => ({ ...f, email: e.target.value }))}
              />
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="role">{t("user.role")}</Label>
              <Select value={editForm.role} onValueChange={(v) => setEditForm(f => ({ ...f, role: v }))}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="admin">{t("user.admin")}</SelectItem>
                  <SelectItem value="editor">{t("user.editor")}</SelectItem>
                  <SelectItem value="author">{t("user.author")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
            
            <div className="space-y-2">
              <Label htmlFor="status">{t("user.status")}</Label>
              <Select value={editForm.status} onValueChange={(v) => setEditForm(f => ({ ...f, status: v }))}>
                <SelectTrigger>
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="active">{t("user.active")}</SelectItem>
                  <SelectItem value="banned">{t("user.banned")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
          
          <DialogFooter>
            <Button variant="outline" onClick={() => setEditOpen(false)}>
              {t("common.cancel")}
            </Button>
            <Button onClick={handleSave} disabled={saving}>
              {saving && <Loader2 className="h-4 w-4 mr-2 animate-spin" />}
              {t("common.save")}
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
