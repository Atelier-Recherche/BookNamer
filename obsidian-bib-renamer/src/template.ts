import { BookMetadata, NamingTemplate } from './types';
import { sanitizeFilename } from './heuristics';

/** Découpe un auteur en (nom, prénom). Accepte « Nom, Prénom » ou « Prénom Nom ». */
export function parseAuthorParts(name: string): { nom: string; prenom: string } {
  const trimmed = name.trim();
  if (!trimmed) return { nom: '', prenom: '' };

  if (trimmed.includes(',')) {
    const [nom, ...rest] = trimmed.split(',');
    return { nom: nom.trim(), prenom: rest.join(',').trim() };
  }

  const parts = trimmed.split(/\s+/);
  if (parts.length === 1) return { nom: parts[0], prenom: '' };
  return {
    nom: parts[parts.length - 1],
    prenom: parts.slice(0, -1).join(' '),
  };
}

/** @deprecated préférer parseAuthorParts — ne force plus « Nom, Prénom ». */
export function formatAuthor(name: string): string {
  return name.trim();
}

function replaceOptional(tpl: string, variable: string, value: string): string {
  if (!value) {
    return tpl
      .replace(new RegExp(`\\[${escapeRegex(variable)}\\]`, 'g'), '')
      .replace(new RegExp(`\\(${escapeRegex(variable)}\\)`, 'g'), '')
      .replace(new RegExp(escapeRegex(variable), 'g'), '');
  }
  return tpl.replace(new RegExp(escapeRegex(variable), 'g'), value);
}

function escapeRegex(s: string): string {
  return s.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}

function cleanEmptyBrackets(text: string): string {
  let prev = text;
  while (true) {
    let cleaned = prev.replace(/\[\s*\]|\(\s*\)|\{\s*\}/g, '');
    cleaned = cleaned.replace(/\s{2,}/g, ' ');
    cleaned = cleaned.replace(/\s+-\s+-/g, ' -');
    if (cleaned === prev) break;
    prev = cleaned;
  }
  return prev;
}

export function applyTemplate(
  template: NamingTemplate,
  metadata: BookMetadata,
  extension: string
): string {
  const rawAuteur = metadata.authors.length > 0 ? metadata.authors[0] : 'Auteur inconnu';
  const { nom, prenom } = metadata.authors.length > 0
    ? parseAuthorParts(metadata.authors[0])
    : { nom: 'Auteur inconnu', prenom: '' };

  const titre = metadata.title ?? 'Sans titre';
  const date = metadata.date ?? 's.d.';
  const editeur = metadata.publisher ?? '';
  const isbn = metadata.isbn ?? '';
  const doi = metadata.doi ?? '';
  const ext = extension.replace(/^\.+/, '');

  let result = template.pattern;
  result = result.replace(/\{auteur-nom\}/g, nom);
  result = result.replace(/\{auteur-prenom\}/g, prenom);
  result = result.replace(/\{auteur\}/g, rawAuteur);
  result = result.replace(/\{titre\}/g, titre);
  result = result.replace(/\{date\}/g, date);
  result = result.replace(/\{extension\}/g, ext);

  result = replaceOptional(result, '{editeur}', editeur);
  result = replaceOptional(result, '{isbn}', isbn);
  result = replaceOptional(result, '{doi}', doi);

  result = cleanEmptyBrackets(result).trim();

  const finalName = `${result}.${ext}`;
  return sanitizeFilename(finalName);
}
