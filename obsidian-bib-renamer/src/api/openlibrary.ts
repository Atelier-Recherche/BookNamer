import { requestUrl } from 'obsidian';
import { BookMetadata } from '../types';
import { extractYear } from '../heuristics';

export async function fetchOpenLibraryByIsbn(isbn: string): Promise<BookMetadata> {
  const cleanIsbn = isbn.replace(/[-\s]/g, '');

  // 1. Essayer search.json?q={isbn} (très complet et indexe toutes les éditions)
  try {
    const searchUrl = `https://openlibrary.org/search.json?q=${encodeURIComponent(cleanIsbn)}&limit=1`;
    const searchResp = await requestUrl({ url: searchUrl, method: 'GET' });
    const searchData = searchResp.json;

    if (searchData && Array.isArray(searchData.docs) && searchData.docs.length > 0) {
      const doc = searchData.docs[0];
      if (doc.title) {
        const title = doc.title;
        const authors = Array.isArray(doc.author_name) ? doc.author_name : [];
        const date = doc.first_publish_year ? String(doc.first_publish_year) : null;
        const publisher = Array.isArray(doc.publisher) && doc.publisher.length > 0 ? doc.publisher[0] : null;

        return {
          title,
          authors,
          date,
          publisher,
          isbn: cleanIsbn,
          doi: null,
          source: 'openlibrary',
          confidence: 'high',
        };
      }
    }
  } catch (e) {
    console.log('[booknamer] OpenLibrary search.json failed, trying api/books...', e);
  }

  // 2. Fallback sur api/books?bibkeys=ISBN:...
  const url = `https://openlibrary.org/api/books?bibkeys=ISBN:${encodeURIComponent(cleanIsbn)}&format=json&jscmd=data`;
  const response = await requestUrl({ url, method: 'GET' });
  const data = response.json;
  const key = `ISBN:${cleanIsbn}`;

  if (!data || !data[key]) {
    throw new Error(`ISBN ${isbn} non trouvé sur OpenLibrary`);
  }

  const book = data[key];
  const title = book.title ?? null;
  const authors = Array.isArray(book.authors)
    ? book.authors.map((a: { name: string }) => a.name).filter(Boolean)
    : [];
  const date = book.publish_date ? (extractYear(book.publish_date) ?? book.publish_date) : null;
  const publisher = Array.isArray(book.publishers) && book.publishers.length > 0
    ? book.publishers[0].name
    : null;

  return {
    title,
    authors,
    date,
    publisher,
    isbn: cleanIsbn,
    doi: null,
    source: 'openlibrary',
    confidence: 'high',
  };
}

function normalizeSearchQuery(s: string): string {
  return s
    .normalize('NFD')
    .replace(/[\u0300-\u036f]/g, '')
    .replace(/[,;:!?"']/g, ' ')
    .replace(/\s+/g, ' ')
    .trim()
    .toLowerCase();
}

/** Extrait un indice auteur depuis "Titre Nom, Prénom". */
function authorHintFromQuery(query: string): string | null {
  const idx = query.lastIndexOf(',');
  if (idx < 0) return null;
  const left = query.slice(0, idx).trim();
  const firstName = query.slice(idx + 1).trim();
  const parts = left.split(/\s+/);
  const lastName = parts[parts.length - 1];
  if (!firstName || !lastName || lastName[0] !== lastName[0].toUpperCase()) return null;
  return `${lastName} ${firstName}`;
}

async function searchOpenLibraryOnce(query: string): Promise<BookMetadata[]> {
  const url = `https://openlibrary.org/search.json?q=${encodeURIComponent(query)}&limit=5`;
  const response = await requestUrl({ url, method: 'GET' });
  const data = response.json;

  if (!data || !Array.isArray(data.docs)) {
    return [];
  }

  return data.docs.slice(0, 5).map((doc: any) => {
    const docTitle = doc.title ?? null;
    const authors = Array.isArray(doc.author_name) ? doc.author_name : [];
    const date = doc.first_publish_year ? String(doc.first_publish_year) : null;
    const isbn = Array.isArray(doc.isbn) && doc.isbn.length > 0 ? String(doc.isbn[0]) : null;

    return {
      title: docTitle,
      authors,
      date,
      publisher: null,
      isbn,
      doi: null,
      source: 'openlibrary' as const,
      confidence: 'medium' as const,
    };
  });
}

export async function searchOpenLibrary(title: string, author: string): Promise<BookMetadata[]> {
  const query = `${title} ${author}`.trim();
  if (!query) return [];

  const candidates: string[] = [query];
  const normalized = normalizeSearchQuery(query);
  if (normalized && normalized !== query.toLowerCase()) {
    candidates.push(normalized);
  }

  // Normaliser "Nom, Prénom" → "Prénom Nom" dans la requête auteur
  const authorClean = author.includes(',')
    ? author.split(',').map(p => p.trim()).filter(Boolean).reverse().join(' ')
    : author.trim();
  if (title.trim() && authorClean && `${title} ${authorClean}`.trim() !== query) {
    candidates.push(`${title} ${authorClean}`.trim());
  }

  const authorHint = authorHintFromQuery(query) || (authorClean || null);
  if (authorHint) {
    candidates.push(authorHint);
    const last = authorHint.split(/\s+/)[0];
    if (last) candidates.push(last);
  }

  const seen = new Set<string>();
  for (const candidate of candidates) {
    const key = candidate.toLowerCase();
    if (!candidate.trim() || seen.has(key)) continue;
    seen.add(key);
    try {
      const results = await searchOpenLibraryOnce(candidate);
      if (results.length > 0) return results;
    } catch (e) {
      console.log('[booknamer] OpenLibrary search failed for:', candidate, e);
    }
  }

  return [];
}
