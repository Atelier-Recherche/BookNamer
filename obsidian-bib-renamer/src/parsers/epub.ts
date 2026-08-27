import JSZip from 'jszip';
import { BookMetadata, Confidence } from '../types';
import { extractIsbn, extractYear } from '../heuristics';

export async function extractEpubMetadata(arrayBuffer: ArrayBuffer): Promise<BookMetadata> {
  const zip = await JSZip.loadAsync(arrayBuffer);

  // 1. Locate rootfile from META-INF/container.xml
  let opfPath = 'content.opf';
  const containerFile = zip.file('META-INF/container.xml');
  if (containerFile) {
    const containerXml = await containerFile.async('text');
    const match = containerXml.match(/full-path=["']([^"']+)["']/i);
    if (match && match[1]) {
      opfPath = match[1];
    }
  }

  // 2. Read OPF file
  let opfFile = zip.file(opfPath);
  if (!opfFile) {
    // Fallback: search for any .opf file in the zip
    const files = Object.keys(zip.files);
    const found = files.find((f) => f.toLowerCase().endsWith('.opf'));
    if (found) {
      opfFile = zip.file(found);
    }
  }

  if (!opfFile) {
    throw new Error('Fichier content.opf introuvable dans le paquet EPUB');
  }

  const opfXml = await opfFile.async('text');

  // Parse XML using DOMParser (available in Obsidian Web environment)
  const parser = new DOMParser();
  const doc = parser.parseFromString(opfXml, 'application/xml');

  // Helper to extract text from tag
  const getTagText = (tag: string): string | null => {
    const el = doc.querySelector(`metadata > ${tag}`) || doc.querySelector(tag);
    return el ? el.textContent?.trim() ?? null : null;
  };

  const getTagsText = (tag: string): string[] => {
    const elements = doc.querySelectorAll(`metadata > ${tag}`);
    const res: string[] = [];
    elements.forEach((el) => {
      const text = el.textContent?.trim();
      if (text) res.push(text);
    });
    return res;
  };

  const title = getTagText('dc\\:title') || getTagText('title');
  let authors = getTagsText('dc\\:creator');
  if (authors.length === 0) {
    authors = getTagsText('creator');
  }
  if (authors.length === 0) {
    const auth = getTagText('dc\\:author') || getTagText('author');
    if (auth) authors = [auth];
  }

  const rawDate = getTagText('dc\\:date') || getTagText('date');
  const date = rawDate ? (extractYear(rawDate) ?? rawDate) : null;
  const publisher = getTagText('dc\\:publisher') || getTagText('publisher');

  // Identifiers / ISBN
  const identifiers = getTagsText('dc\\:identifier').concat(getTagsText('identifier'));
  let isbn: string | null = null;
  for (const id of identifiers) {
    const found = extractIsbn(id);
    if (found) {
      isbn = found;
      break;
    }
  }

  let confidence: Confidence = 'low';
  if (title && authors.length > 0) {
    confidence = isbn || date ? 'high' : 'medium';
  }

  return {
    title,
    authors,
    date,
    publisher,
    isbn,
    doi: null,
    source: 'file',
    confidence,
  };
}
