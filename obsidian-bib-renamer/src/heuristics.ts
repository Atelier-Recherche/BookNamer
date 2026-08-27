export interface ParsedFilename {
  cleaned: string;
  isbn: string | null;
  doi: string | null;
  year: string | null;
  parts: string[];
}

const RE_MD5 = /\b[a-f0-9]{32}\b/gi;
const RE_NOISE = /\b(anna'?s?\s*archive|libgen|z-?lib(?:rary)?|zlibrary|download|bookfi|b-ok|genesis)\b/gi;
const RE_ISBN13 = /\b(?:isbn(?:-13)?[:\s]*)?(97[89][\d\-]{10,14})\b/i;
const RE_ISBN10 = /\b(?:isbn(?:-10)?[:\s]*)?([\dX]{9}[\dX])\b/i;
const RE_DOI = /\b(10\.\d{4,9}\/[-._;()/:A-Za-z0-9]+)\b/;
const RE_YEAR = /\b(19[5-9]\d|20[0-2]\d)\b/;
const RE_SEPARATORS = /\s*--\s*|\s+-\s+|_/;
const RE_FORBIDDEN_CHARS = /[/\\:*?"<>|]/g;

export function extractIsbn(text: string): string | null {
  const m13 = text.match(RE_ISBN13);
  if (m13 && m13[1]) {
    const raw = m13[1].replace(/[-\s]/g, '');
    if (raw.length === 13) return raw;
  }
  const m10 = text.match(RE_ISBN10);
  if (m10 && m10[1]) {
    const raw = m10[1].toUpperCase();
    if (raw.length === 10) return raw;
  }
  return null;
}

export function extractDoi(text: string): string | null {
  const m = text.match(RE_DOI);
  if (m && m[1]) {
    return m[1].replace(/\.+$/, '');
  }
  return null;
}

export function extractYear(text: string): string | null {
  const m = text.match(RE_YEAR);
  return m ? m[1] : null;
}

export function cleanNoise(text: string): string {
  let s = text.replace(RE_MD5, '').replace(RE_NOISE, '');
  s = s.replace(/\(\s*\)|\[\s*\]|\{\s*\}/g, '');
  s = s.replace(/\s{2,}/g, ' ');
  return s.trim();
}

export function stripExtension(filename: string): string {
  const pos = filename.lastIndexOf('.');
  return pos > 0 ? filename.substring(0, pos) : filename;
}

export function parseFilename(raw: string): ParsedFilename {
  const stem = stripExtension(raw);
  const isbn = extractIsbn(stem);
  const doi = extractDoi(stem);
  const year = extractYear(stem);
  const cleaned = cleanNoise(stem);
  const parts = cleaned
    .split(RE_SEPARATORS)
    .map((p) => p.trim())
    .filter((p) => p.length > 0);

  return {
    cleaned,
    isbn,
    doi,
    year,
    parts,
  };
}

export function sanitizeFilename(filename: string): string {
  let s = filename.replace(RE_FORBIDDEN_CHARS, '');
  s = s.replace(/\s{2,}/g, ' ');
  return s.replace(/^[\s.]+|[\s.]+$/g, '');
}
