import { requestUrl } from 'obsidian';
import { BookMetadata } from '../types';

export function toIsbn10(isbn13: string): string | null {
  const clean = isbn13.replace(/[-\s]/g, '');
  if (clean.startsWith('978') && clean.length === 13) {
    const core = clean.substring(3, 12);
    let sum = 0;
    for (let i = 0; i < 9; i++) {
      sum += parseInt(core[i], 10) * (10 - i);
    }
    const rem = (11 - (sum % 11)) % 11;
    const check = rem === 10 ? 'X' : String(rem);
    return `${core}${check}`;
  } else if (clean.length === 10) {
    return clean;
  }
  return null;
}

function decodeHtml(html: string): string {
  const txt = document.createElement('textarea');
  txt.innerHTML = html;
  return txt.value;
}

export async function fetchAmazonByIsbn(isbn: string): Promise<BookMetadata> {
  const clean = isbn.replace(/[-\s]/g, '');
  const isbn10 = toIsbn10(clean) || clean;

  const urls = [
    `https://www.amazon.fr/dp/${isbn10}`,
    `https://www.amazon.com/dp/${isbn10}`,
  ];

  for (const url of urls) {
    try {
      const resp = await requestUrl({
        url,
        method: 'GET',
        headers: {
          'Accept-Language': 'fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7',
          'User-Agent': 'Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36',
        },
      });

      const html = resp.text;
      if (html.includes('Robot Check') || html.includes('Page introuvable')) {
        continue;
      }

      // Title
      let title: string | null = null;
      const titleMatch = html.match(/id=["']productTitle["'][^>]*>\s*([\s\S]*?)\s*<\/span/i);
      if (titleMatch && titleMatch[1].trim()) {
        title = decodeHtml(titleMatch[1].trim());
      } else {
        const titleTag = html.match(/<title>\s*([\s\S]*?)\s*<\/title>/i);
        if (titleTag && titleTag[1]) {
          let t = titleTag[1].trim();
          t = t.replace(/Amazon\.[a-z]+:?/i, '').replace(/:\s*Livres/i, '').trim();
          if (t.length > 0 && !t.includes('Page introuvable')) {
            title = decodeHtml(t);
          }
        }
      }

      // Author
      let authors: string[] = [];
      const authorMatch = html.match(/class=["']author[^"']*["'][^>]*>[\s\S]*?<a[^>]*>\s*([\s\S]*?)\s*<\/a>/i);
      if (authorMatch && authorMatch[1].trim()) {
        authors = [decodeHtml(authorMatch[1].trim())];
      }

      if (title) {
        return {
          title,
          authors,
          date: null,
          publisher: null,
          isbn: clean,
          doi: null,
          source: 'amazon',
          confidence: 'high',
        };
      }
    } catch (e) {
      console.log('[booknamer] Amazon lookup error for url:', url, e);
    }
  }

  throw new Error(`ISBN ${isbn} non trouvé sur Amazon`);
}
