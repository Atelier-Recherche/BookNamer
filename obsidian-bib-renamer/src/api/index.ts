import { BookMetadata, PluginSettings } from '../types';
import { fetchOpenLibraryByIsbn, searchOpenLibrary } from './openlibrary';
import { fetchBnfByIsbn, searchBnf } from './bnf';
import { fetchAmazonByIsbn } from './amazon';
import { fetchCrossrefByDoi, searchCrossref } from './crossref';
import { fetchGoogleBooksByIsbn } from './googlebooks';

function normalizeIsbn(isbn: string): string {
  return isbn.replace(/[-\s]/g, '').toUpperCase();
}

function extractIsbnFromQuery(query: string): string | null {
  const m = query.match(/(?:ISBN(?:-1[03])?[:\s]*)?((?:97[89][-\s]?)?(?:\d[-\s]?){9}[\dXx])/i);
  if (!m) return null;
  const clean = normalizeIsbn(m[1]);
  return clean.length === 10 || clean.length === 13 ? clean : null;
}

function dedupeMetadata(items: BookMetadata[]): BookMetadata[] {
  const seen = new Set<string>();
  const out: BookMetadata[] = [];
  for (const item of items) {
    const key = `${(item.title || '').toLowerCase()}|${(item.authors[0] || '').toLowerCase()}|${item.source}`;
    if (seen.has(key)) continue;
    seen.add(key);
    out.push(item);
  }
  return out;
}

/** Recherche multi-sources alignée sur le desktop Rust (`search_by_query`). */
export async function searchByQuery(
  query: string,
  settings: PluginSettings,
  isbnHint?: string | null
): Promise<BookMetadata[]> {
  const q = query.trim();
  const isbn =
    (isbnHint && normalizeIsbn(isbnHint)) ||
    extractIsbnFromQuery(q) ||
    null;

  const all: BookMetadata[] = [];

  // ── Branche ISBN (haute confiance, toutes sources en parallèle) ────────
  if (isbn) {
    const isbnJobs: Promise<BookMetadata | null>[] = [];

    if (settings.enableOpenLibrary) {
      isbnJobs.push(fetchOpenLibraryByIsbn(isbn).catch(() => null));
    }
    if (settings.enableBnf) {
      isbnJobs.push(fetchBnfByIsbn(isbn).catch(() => null));
    }
    if (settings.enableAmazon) {
      isbnJobs.push(fetchAmazonByIsbn(isbn).catch(() => null));
    }
    if (settings.enableGoogleBooks) {
      isbnJobs.push(
        fetchGoogleBooksByIsbn(isbn, settings.googleBooksApiKey).catch(() => null)
      );
    }

    const isbnHits = await Promise.all(isbnJobs);
    for (const hit of isbnHits) {
      if (hit) all.push(hit);
    }
  }

  // ── Recherche textuelle ────────────────────────────────────────────────
  let textQuery = q;
  if (isbn) {
    textQuery = q
      .replace(new RegExp(isbn.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'ig'), ' ')
      .replace(/ISBN(?:-1[03])?[:\s]*/gi, ' ')
      .replace(/\s+/g, ' ')
      .trim();
  }

  if (textQuery) {
    const title = textQuery;
    const author = '';
    const textJobs: Promise<BookMetadata[]>[] = [];

    if (settings.enableOpenLibrary) {
      textJobs.push(searchOpenLibrary(title, author).catch(() => []));
    }
    if (settings.enableBnf) {
      textJobs.push(searchBnf(textQuery).catch(() => []));
    }
    if (settings.enableCrossref) {
      textJobs.push(searchCrossref(title, author).catch(() => []));
    }

    const textHits = await Promise.all(textJobs);
    for (const list of textHits) {
      all.push(...list);
    }
  }

  // ── Amazon via ISBN découverts dans les autres résultats ───────────────
  if (settings.enableAmazon) {
    const alreadyAmazon = new Set(
      all
        .filter((m) => m.source === 'amazon' && m.isbn)
        .map((m) => normalizeIsbn(m.isbn!))
    );
    const toFetch: string[] = [];
    const seen = new Set<string>();

    const consider = (raw: string | null | undefined) => {
      if (!raw) return;
      const clean = normalizeIsbn(raw);
      if (clean.length < 10 || alreadyAmazon.has(clean) || seen.has(clean)) return;
      seen.add(clean);
      if (toFetch.length < 3) toFetch.push(clean);
    };

    if (isbn) consider(isbn);
    for (const m of all) consider(m.isbn);

    for (const id of toFetch) {
      try {
        const amz = await fetchAmazonByIsbn(id);
        all.push(amz);
      } catch (e) {
        console.log('[booknamer] Amazon follow-up failed:', id, e);
      }
    }
  }

  return dedupeMetadata(all);
}

export async function enrichMetadata(
  metadata: BookMetadata,
  settings: PluginSettings
): Promise<BookMetadata[]> {
  // ── Branche ISBN : toutes les sources (plus d'arrêt au premier succès) ─
  if (metadata.isbn) {
    const isbnResults = await searchByQuery('', settings, metadata.isbn);
    if (isbnResults.length > 0) {
      return isbnResults;
    }
  }

  // ── Branche DOI ─────────────────────────────────────────────────────────
  if (metadata.doi && settings.enableCrossref) {
    try {
      const cr = await fetchCrossrefByDoi(metadata.doi);
      return [cr];
    } catch (e) {
      console.log('[booknamer] Crossref DOI lookup failed:', e);
    }
  }

  // ── Recherche textuelle multi-sources (inclut BNF) ──────────────────────
  const title = metadata.title || '';
  const author = metadata.authors.length > 0 ? metadata.authors[0] : '';
  const query = `${title} ${author}`.trim();

  if (!query) {
    return [];
  }

  return searchByQuery(query, settings, metadata.isbn);
}
