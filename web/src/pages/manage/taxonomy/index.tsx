import { AdminPageHeader } from "@/components/admin/page-header";
import { useTranslation } from "@/lib/i18n";
import CategoriesPage from "@/pages/manage/categories";
import TagsPage from "@/pages/manage/tags";

export default function TaxonomyPage() {
  const { t } = useTranslation();

  return (
    <div className="space-y-5">
      <AdminPageHeader
        title={`${t("manage.categories")} / ${t("manage.tags")}`}
        description={`${t("category.totalCategories")} / ${t("tag.totalTags")}`}
      />

      <div className="grid items-stretch gap-5 xl:grid-cols-2">
        <CategoriesPage embedded />
        <TagsPage embedded />
      </div>
    </div>
  );
}
