import { lazy, Suspense } from "react";
import { Routes, Route } from "react-router-dom";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import PluginSlot from "@/components/plugin-slot";

import HomePage from "@/pages/home";

const ArchivesPage = lazy(() => import("@/pages/archives"));
const CategoriesPage = lazy(() => import("@/pages/categories"));
const TagsPage = lazy(() => import("@/pages/tags"));
const PostPage = lazy(() => import("@/pages/post"));
const CustomPage = lazy(() => import("@/pages/custom-page"));

function RouteFallback() {
  return (
    <div className="min-h-screen bg-background">
      <div className="fixed left-0 right-0 top-0 z-[100] h-0.5 overflow-hidden bg-primary/15">
        <div className="h-full w-1/2 animate-pulse bg-primary" />
      </div>
      <div className="container mx-auto max-w-4xl px-4 py-8">
        <div className="mx-auto mb-8 h-10 w-64 rounded skeleton-shimmer" />
        <div className="space-y-6">
          <div className="h-36 rounded-lg border bg-card skeleton-shimmer" />
          <div className="h-36 rounded-lg border bg-card skeleton-shimmer" />
        </div>
      </div>
    </div>
  );
}

export default function App() {
  return (
    <ThemeProvider
      attribute="class"
      defaultTheme="system"
      enableSystem
      disableTransitionOnChange
    >
      {/* body_start 插槽 */}
      <PluginSlot name="body_start" />

      <Suspense fallback={<RouteFallback />}>
        <Routes>
          <Route path="/" element={<HomePage />} />
          <Route path="/archives" element={<ArchivesPage />} />
          <Route path="/categories" element={<CategoriesPage />} />
          <Route path="/tags" element={<TagsPage />} />
          <Route path="/posts/*" element={<PostPage />} />
          <Route path="/:slug" element={<CustomPage />} />
        </Routes>
      </Suspense>

      <Toaster position="top-center" />

      {/* body_end 插槽 */}
      <PluginSlot name="body_end" />
    </ThemeProvider>
  );
}
