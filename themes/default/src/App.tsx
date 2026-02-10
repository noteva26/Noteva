import { Routes, Route } from "react-router-dom";
import { ThemeProvider } from "@/components/theme-provider";
import { Toaster } from "@/components/ui/sonner";
import PluginSlot from "@/components/plugin-slot";

// Pages
import HomePage from "@/pages/home";
import ArchivesPage from "@/pages/archives";
import CategoriesPage from "@/pages/categories";
import TagsPage from "@/pages/tags";
import PostPage from "@/pages/post";
import CustomPage from "@/pages/custom-page";

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

      <Routes>
        <Route path="/" element={<HomePage />} />
        <Route path="/archives" element={<ArchivesPage />} />
        <Route path="/categories" element={<CategoriesPage />} />
        <Route path="/tags" element={<TagsPage />} />
        <Route path="/posts/*" element={<PostPage />} />
        <Route path="/:slug" element={<CustomPage />} />
      </Routes>

      <Toaster position="top-center" />

      {/* body_end 插槽 */}
      <PluginSlot name="body_end" />
    </ThemeProvider>
  );
}
