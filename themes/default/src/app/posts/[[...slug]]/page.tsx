import PostClient from "./post-client";

// For static export - generates a single catch-all page
export function generateStaticParams() {
  return [{ slug: [] }];
}

export default function PostPage() {
  return <PostClient />;
}
