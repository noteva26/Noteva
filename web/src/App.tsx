import { Routes, Route, Navigate } from "react-router-dom";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";

// Layouts
import ManageLayout from "@/pages/manage/layout";

// Pages
import DashboardPage from "@/pages/manage/dashboard";
import ArticlesPage from "@/pages/manage/articles";
import ArticleNewPage from "@/pages/manage/articles/new";
import ArticleEditPage from "@/pages/manage/articles/edit";
import CategoriesPage from "@/pages/manage/categories";
import TagsPage from "@/pages/manage/tags";
import PagesPage from "@/pages/manage/pages";
import NavPage from "@/pages/manage/nav";
import CommentsPage from "@/pages/manage/comments";
import PluginsPage from "@/pages/manage/plugins";
import ThemesPage from "@/pages/manage/themes";
import SecurityPage from "@/pages/manage/security";
import SettingsPage from "@/pages/manage/settings";
import LoginPage from "@/pages/manage/login";
import SetupPage from "@/pages/manage/setup";

export default function App() {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
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
      <Toaster position="top-center" />
    </ThemeProvider>
  );
}
