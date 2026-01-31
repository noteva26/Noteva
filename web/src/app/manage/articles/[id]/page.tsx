import EditArticleClient from "./edit-client";

export function generateStaticParams() {
  return [{ id: "0" }];
}

export default function EditArticlePage() {
  return <EditArticleClient />;
}
