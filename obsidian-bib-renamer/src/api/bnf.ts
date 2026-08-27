import { requestUrl } from 'obsidian';
import { BookMetadata } from '../types';
import { extractYear } from '../heuristics';
import { toIsbn10 } from './amazon';

function cleanBnfTitle(title: string): string {
  return title.split('/')[0].trim();
}

function cleanBnfAuthor(author: string): string {
  return author.split('(')[0].split('.')[0].trim();
}

function cleanBnfPublisher(publisher: string): string {
  return publisher.split('(')[0].trim();
}

function extractTag(xml: string, tag: string): string | null {
  const re = new RegExp(`<${tag}[^>]*>([^<]+)</${tag}>`, 'i');
  const m = xml.match(re);
  return m?.[1]?.trim() || null;
}

function extractBnfIsbn(xml: string): string | null {
  const re = /<dc:identifier[^>]*>([^<]+)<\/dc:identifier>/gi;
  let match: RegExpExecArray | null;
  while ((match = re.exec(xml)) !== null) {
    const content = match[1].trim();
    const lower = content.toLowerCase();
    let digits = content.replace(/[^0-9Xx]/g, '').toUpperCase();
    if (lower.includes('isbn')) {
      digits = digits.replace(/^[A-Z]+/, '');
    }
    if (digits.length === 13 && /^\d{13}$/.test(digits)) return digits;
    if (digits.length === 10 && /^\d{9}[\dX]$/.test(digits)) return digits;
  }
  return null;
}

export async function fetchBnfByIsbn(isbn: string): Promise<BookMetadata> {
  const clean = isbn.replace(/[-\s]/g, '');
  const isbn10 = toIsbn10(clean);

  const queries = [
    `bib.isbn all "${clean}"`,
    `bib.anywhere all "${clean}"`,
    `bib.isbn all "${isbn10 || clean}"`,
  ];

  for (const q of queries) {
    const url = `https://catalogue.bnf.fr/api/SRU?version=1.2&operation=searchRetrieve&query=${encodeURIComponent(q)}&recordSchema=dublincore`;

    try {
      const resp = await requestUrl({ url, method: 'GET' });
      const xml = resp.text;

      if (xml.includes('<srw:numberOfRecords>0</srw:numberOfRecords>')) {
        continue;
      }

      const titleRaw = extractTag(xml, 'dc:title');
      const creatorRaw = extractTag(xml, 'dc:creator');
      const dateRaw = extractTag(xml, 'dc:date');
      const publisherRaw = extractTag(xml, 'dc:publisher');

      if (titleRaw) {
        return {
          title: cleanBnfTitle(titleRaw),
          authors: creatorRaw ? [cleanBnfAuthor(creatorRaw)] : [],
          date: dateRaw ? extractYear(dateRaw) : null,
          publisher: publisherRaw ? cleanBnfPublisher(publisherRaw) : null,
          isbn: clean,
          doi: null,
          source: 'bnf',
          confidence: 'high',
        };
      }
    } catch (e) {
      console.log('[obsidian-bib-renamer] BNF lookup error for query:', q, e);
    }
  }

  throw new Error(`ISBN ${isbn} non trouvé sur le catalogue BNF`);
}

/** Recherche textuelle BNF (alignée sur le desktop Rust). */
export async function searchBnf(query: string): Promise<BookMetadata[]> {
  const q = `bib.anywhere all "${query.trim()}"`;
  const url = `https://catalogue.bnf.fr/api/SRU?version=1.2&operation=searchRetrieve&query=${encodeURIComponent(q)}&recordSchema=dublincore&maximumRecords=5`;

  try {
    const resp = await requestUrl({ url, method: 'GET' });
    const xml = resp.text;
    const results: BookMetadata[] = [];

    for (const record of xml.split('<srw:record>').slice(1)) {
      const titleRaw = extractTag(record, 'dc:title');
      if (!titleRaw) continue;
      const creatorRaw = extractTag(record, 'dc:creator');
      const dateRaw = extractTag(record, 'dc:date');
      const publisherRaw = extractTag(record, 'dc:publisher');

      results.push({
        title: cleanBnfTitle(titleRaw),
        authors: creatorRaw ? [cleanBnfAuthor(creatorRaw)] : [],
        date: dateRaw ? extractYear(dateRaw) : null,
        publisher: publisherRaw ? cleanBnfPublisher(publisherRaw) : null,
        isbn: extractBnfIsbn(record),
        doi: null,
        source: 'bnf',
        confidence: 'medium',
      });
    }

    return results;
  } catch (e) {
    console.log('[obsidian-bib-renamer] BNF text search failed:', e);
    return [];
  }
}
