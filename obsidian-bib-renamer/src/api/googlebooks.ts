import { requestUrl } from 'obsidian';
import { BookMetadata } from '../types';
import { extractYear } from '../heuristics';

export async function fetchGoogleBooksByIsbn(isbn: string, apiKey?: string): Promise<BookMetadata> {
  let url = `https://www.googleapis.com/books/v1/volumes?q=isbn:${encodeURIComponent(isbn)}`;
  if (apiKey && apiKey.trim().length > 0) {
    url += `&key=${encodeURIComponent(apiKey.trim())}`;
  }

  const response = await requestUrl({ url, method: 'GET' });
  const data = response.json;

  if (!data || !Array.isArray(data.items) || data.items.length === 0) {
    throw new Error(`ISBN ${isbn} non trouvé sur Google Books`);
  }

  const item = data.items[0];
  const vi = item.volumeInfo || {};

  const title = vi.title ?? null;
  const authors: string[] = Array.isArray(vi.authors) ? vi.authors : [];
  const date = vi.publishedDate ? (extractYear(vi.publishedDate) ?? vi.publishedDate) : null;
  const publisher = vi.publisher ?? null;

  let foundIsbn = isbn;
  if (Array.isArray(vi.industryIdentifiers)) {
    const idObj = vi.industryIdentifiers.find((i: any) => i.type === 'ISBN_13' || i.type === 'ISBN_10');
    if (idObj && idObj.identifier) {
      foundIsbn = idObj.identifier;
    }
  }

  return {
    title,
    authors,
    date,
    publisher,
    isbn: foundIsbn,
    doi: null,
    source: 'googlebooks',
    confidence: 'high',
  };
}
