import { lazy, Suspense } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";

// Layout loaded eagerly (shared shell)
import ManageLayout from "@/pages/manage/layout";

// Lazy-loaded pages
const DashboardPage = lazy(() => import("@/pages/manage/dashboard"));
const ArticlesPage = lazy(() => import("@/pages/manage/articles"));
const ArticleNewPage = lazy(() => import("@/pages/manage/articles/new"));
const ArticleEditPage = lazy(() => import("@/pages/manage/articles/edit"));
const CategoriesPage = lazy(() => import("@/pages/manage/categories"));
const TagsPage = lazy(() => import("@/pages/manage/tags"));
const PagesPage = lazy(() => import("@/pages/manage/pages"));
const NavPage = lazy(() => import("@/pages/manage/nav"));
const CommentsPage = lazy(() => import("@/pages/manage/comments"));
const PluginsPage = lazy(() => import("@/pages/manage/plugins"));
const ThemesPage = lazy(() => import("@/pages/manage/themes"));
const SecurityPage = lazy(() => import("@/pages/manage/security"));
const SettingsPage = lazy(() => import("@/pages/manage/settings"));
const LoginPage = lazy(() => import("@/pages/manage/login"));
const SetupPage = lazy(() => import("@/pages/manage/setup"));

export default function App() {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
      <Suspense>
        <Routes>
          {/* Root redirect */}
          <Route path="/" element={<Navigate to="/manage" replace />} />

          {/* Auth pages (no sidebar) */}
          <Route path="/manage/login" element={<LoginPage />} />
          <Route path="/manage/setup" element={<SetupPage />} />

          {/* Admin pages (with sidebar) */}
          <Route path="/manage" element={<ManageLayout />}>
            <Route index element={<DashboardPage />} />
            <Route path="articles" element={<ArticlesPage />} />
            <Route path="articles/new" element={<ArticleNewPage />} />
            <Route path="articles/:id" element={<ArticleEditPage />} />
            <Route path="categories" element={<CategoriesPage />} />
            <Route path="tags" element={<TagsPage />} />
            <Route path="pages" element={<PagesPage />} />
            <Route path="nav" element={<NavPage />} />
            <Route path="comments" element={<CommentsPage />} />
            <Route path="plugins" element={<PluginsPage />} />
            <Route path="themes" element={<ThemesPage />} />
            <Route path="security" element={<SecurityPage />} />
            <Route path="settings" element={<SettingsPage />} />
          </Route>
        </Routes>
      </Suspense>
      <Toaster position="top-center" />
    </ThemeProvider>
  );
}
