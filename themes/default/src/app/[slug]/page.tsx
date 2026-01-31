import PageClient from "./page-client";

// For static export - generates placeholder pages
export function generateStaticParams() {
  return [{ slug: "_" }];
}

export default function CustomPage() {
  return <PageClient />;
}
