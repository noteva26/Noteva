import type { NotevaArticle, NotevaSDKRef } from "@/hooks/useNoteva";

const FETCH_ALL_PAGE_SIZE = 100;
const MAX_FETCH_ALL_PAGES = 100;

export async function fetchAllArticles(
  noteva: NotevaSDKRef,
  params: {
    category?: string;
    tag?: string;
    keyword?: string;
    sort?: string;
  } = {}
): Promise<NotevaArticle[]> {
  const articles: NotevaArticle[] = [];

  for (let page = 1; page <= MAX_FETCH_ALL_PAGES; page += 1) {
    const result = await noteva.articles.list({
      ...params,
      page,
      pageSize: FETCH_ALL_PAGE_SIZE,
    });
    const batch = result.articles || [];
    articles.push(...batch);

    const totalPages =
      result.totalPages || Math.max(1, Math.ceil((result.total || 0) / FETCH_ALL_PAGE_SIZE));
    if (page >= totalPages || batch.length === 0) break;
  }

  return articles;
}
