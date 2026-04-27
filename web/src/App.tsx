import { lazy, Suspense } from "react";
import { Routes, Route, Navigate } from "react-router-dom";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import { TopLoader, TopLoaderFallback } from "@/components/ui/top-loader";
import { managePageLoaders } from "@/lib/manage-routes";

// Layout loaded eagerly (shared shell)
import ManageLayout from "@/pages/manage/layout";

// Lazy-loaded pages
const DashboardPage = lazy(managePageLoaders.dashboard);
const ArticlesPage = lazy(managePageLoaders.articles);
const ArticleNewPage = lazy(managePageLoaders.articleNew);
const ArticleEditPage = lazy(managePageLoaders.articleEdit);
const TaxonomyPage = lazy(managePageLoaders.taxonomy);
const PagesPage = lazy(managePageLoaders.pages);
const NavPage = lazy(managePageLoaders.nav);
const CommentsPage = lazy(managePageLoaders.comments);
const PluginsPage = lazy(managePageLoaders.plugins);
const ThemesPage = lazy(managePageLoaders.themes);
const SecurityPage = lazy(managePageLoaders.security);
const FilesPage = lazy(managePageLoaders.files);
const SettingsPage = lazy(managePageLoaders.settings);
const LoginPage = lazy(managePageLoaders.login);
const SetupPage = lazy(managePageLoaders.setup);

export default function App() {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
      <TopLoader />
      <Suspense fallback={<TopLoaderFallback />}>
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
            <Route path="taxonomy" element={<TaxonomyPage />} />
            <Route path="categories" element={<Navigate to="/manage/taxonomy" replace />} />
            <Route path="tags" element={<Navigate to="/manage/taxonomy" replace />} />
            <Route path="pages" element={<PagesPage />} />
            <Route path="nav" element={<NavPage />} />
            <Route path="comments" element={<CommentsPage />} />
            <Route path="plugins" element={<PluginsPage />} />
            <Route path="themes" element={<ThemesPage />} />
            <Route path="security" element={<SecurityPage />} />
            <Route path="files" element={<FilesPage />} />
            <Route path="settings" element={<SettingsPage />} />
          </Route>
        </Routes>
      </Suspense>
      <Toaster position="top-center" />
    </ThemeProvider>
  );
}
