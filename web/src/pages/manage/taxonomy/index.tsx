import { motion } from "motion/react";
import { useTranslation } from "@/lib/i18n";
import CategoriesPage from "@/pages/manage/categories";
import TagsPage from "@/pages/manage/tags";

export default function TaxonomyPage() {
  const { t } = useTranslation();

  return (
    <div className="space-y-6">
      <motion.div
        initial={{ opacity: 0, y: -10 }}
        animate={{ opacity: 1, y: 0 }}
        transition={{ duration: 0.4 }}
        className="space-y-4"
      >
        <div>
          <h1 className="text-3xl font-bold">
            {t("manage.categories")} / {t("manage.tags")}
          </h1>
          <p className="text-muted-foreground">
            {t("category.totalCategories")} / {t("tag.totalTags")}
          </p>
        </div>
      </motion.div>

      <div className="grid gap-6 xl:grid-cols-[minmax(0,0.95fr)_minmax(0,1.05fr)]">
        <CategoriesPage embedded />
        <TagsPage embedded />
      </div>
    </div>
  );
}
