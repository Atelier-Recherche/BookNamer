import { BookMetadata, Confidence } from '../types';
import { extractDoi, extractIsbn, extractYear } from '../heuristics';

function decodePdfString(raw: string): string {
  // UTF-16BE detection (\xFE\xFF)
  if (raw.charCodeAt(0) === 0xfe && raw.charCodeAt(1) === 0xff) {
    let result = '';
    for (let i = 2; i < raw.length; i += 2) {
      const code = (raw.charCodeAt(i) << 8) | raw.charCodeAt(i + 1);
      result += String.fromCharCode(code);
    }
    return result.trim();
  }
  // Standard escaped chars
  return raw
    .replace(/\\n/g, '\n')
    .replace(/\\r/g, '\r')
    .replace(/\\t/g, '\t')
    .replace(/\\\(/g, '(')
    .replace(/\\\)/g, ')')
    .replace(/\\\\/g, '\\')
    .trim();
}

export function extractPdfMetadata(arrayBuffer: ArrayBuffer): BookMetadata {
  // Read first 64KB and last 32KB of the PDF file to scan /Info dict, XMP and first pages
  const totalLen = arrayBuffer.byteLength;
  const decoder = new TextDecoder('latin1'); // Latin-1 preserve single byte values

  const headLen = Math.min(65536, totalLen);
  const headBytes = new Uint8Array(arrayBuffer, 0, headLen);
  const headText = decoder.decode(headBytes);

  let tailText = '';
  if (totalLen > headLen) {
    const tailLen = Math.min(32768, totalLen);
    const tailBytes = new Uint8Array(arrayBuffer, totalLen - tailLen, tailLen);
    tailText = decoder.decode(tailBytes);
  }

  const combined = `${headText} ${tailText}`;

  // Extract from /Info dictionary
  const titleMatch = combined.match(/\/Title\s*\(([^)]+)\)/);
  const authorMatch = combined.match(/\/Author\s*\(([^)]+)\)/);
  const creatorMatch = combined.match(/\/Creator\s*\(([^)]+)\)/);
  const producerMatch = combined.match(/\/Producer\s*\(([^)]+)\)/);
  const creationDateMatch = combined.match(/\/CreationDate\s*\(([^)]+)\)/);

  const title = titleMatch ? decodePdfString(titleMatch[1]) : null;

  let authors: string[] = [];
  if (authorMatch) {
    const authStr = decodePdfString(authorMatch[1]);
    if (authStr.includes(';')) {
      authors = authStr.split(';').map((s) => s.trim()).filter(Boolean);
    } else {
      authors = [authStr];
    }
  }

  let date: string | null = null;
  if (creationDateMatch) {
    date = extractYear(creationDateMatch[1]);
  }

  const publisher = producerMatch
    ? decodePdfString(producerMatch[1])
    : creatorMatch
    ? decodePdfString(creatorMatch[1])
    : null;

  // Search ISBN and DOI in PDF header & metadata
  const isbn = extractIsbn(combined);
  const doi = extractDoi(combined);

  let confidence: Confidence = 'low';
  if (title && authors.length > 0) {
    confidence = isbn || doi ? 'high' : 'medium';
  } else if (title || authors.length > 0) {
    confidence = 'low';
  }

  return {
    title,
    authors,
    date,
    publisher,
    isbn,
    doi,
    source: 'file',
    confidence,
  };
}
