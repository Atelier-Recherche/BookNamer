import { requestUrl } from 'obsidian';
import { BookMetadata } from '../types';

function parseCrossrefMessage(msg: any, confidence: 'high' | 'medium'): BookMetadata {
  const title = Array.isArray(msg.title) && msg.title.length > 0 ? msg.title[0] : null;
  const authors: string[] = Array.isArray(msg.author)
    ? msg.author
        .map((a: any) => {
          if (a.given && a.family) return `${a.given} ${a.family}`;
          if (a.family) return a.family;
          if (a.given) return a.given;
          return a.name || '';
        })
        .filter((s: string) => s.length > 0)
    : [];

  let date: string | null = null;
  if (msg.created && Array.isArray(msg.created['date-parts']) && msg.created['date-parts'].length > 0) {
    const parts = msg.created['date-parts'][0];
    if (Array.isArray(parts) && parts.length > 0) {
      date = String(parts[0]);
    }
  }

  const publisher = msg.publisher ?? null;
  const doi = msg.DOI ?? null;

  return {
    title,
    authors,
    date,
    publisher,
    isbn: null,
    doi,
    source: 'crossref',
    confidence,
  };
}

export async function fetchCrossrefByDoi(doi: string): Promise<BookMetadata> {
  const url = `https://api.crossref.org/works/${encodeURIComponent(doi)}`;
  const response = await requestUrl({ url, method: 'GET' });
  const data = response.json;

  if (!data || !data.message) {
    throw new Error(`DOI ${doi} non trouvé sur Crossref`);
  }

  return parseCrossrefMessage(data.message, 'high');
}

export async function searchCrossref(title: string, author: string): Promise<BookMetadata[]> {
  const query = `${title} ${author}`.trim();
  const url = `https://api.crossref.org/works?query.bibliographic=${encodeURIComponent(query)}&rows=3`;
  const response = await requestUrl({ url, method: 'GET' });
  const data = response.json;

  if (!data || !data.message || !Array.isArray(data.message.items)) {
    return [];
  }

  return data.message.items.map((item: any) => parseCrossrefMessage(item, 'medium'));
}
