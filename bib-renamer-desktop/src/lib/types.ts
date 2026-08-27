// Types partagés entre les composants Svelte, mirroirs des structs Rust

export type MetadataSource =
  | 'unknown' | 'file' | 'filename'
  | 'openlibrary' | 'crossref' | 'googlebooks'
  | 'amazon' | 'bnf';

export type Confidence = 'low' | 'medium' | 'high';

export interface BookMetadata {
  title: string | null;
  authors: string[];
  date: string | null;
  publisher: string | null;
  isbn: string | null;
  doi: string | null;
  source: MetadataSource;
  confidence: Confidence;
}

export type FileStatus =
  | 'pending'
  | 'extracting'
  | 'metadata_found'
  | 'needs_review'
  | 'confirmed'
  | 'renamed'
  | { error: string };

export interface QueueEntry {
  id: string;
  original_path: string;
  file_name: string;
  extension: string;
  status: FileStatus;
  metadata: BookMetadata | null;
  alternative_results: BookMetadata[];
  proposed_name: string | null;
}

export interface NamingTemplate {
  id: string;
  name: string;
  pattern: string;
  is_default: boolean;
}

export interface EnabledSources {
  openlibrary: boolean;
  crossref: boolean;
  google_books: boolean;
  amazon: boolean;
  bnf: boolean;
}

export interface AppConfig {
  templates: NamingTemplate[];
  active_template_id: string;
  google_books_api_key: string | null;
  enabled_sources: EnabledSources;
}

// ─── Helpers ───────────────────────────────────────────────────────────────

export function getStatusLabel(status: FileStatus): string {
  if (typeof status === 'object') return 'Erreur';
  const map: Record<string, string> = {
    pending: 'En attente',
    extracting: 'Extraction…',
    metadata_found: 'Métadonnées trouvées',
    needs_review: 'À valider',
    confirmed: 'Confirmé',
    renamed: 'Renommé',
  };
  return map[status] ?? status;
}

export function getStatusClass(status: FileStatus): string {
  if (typeof status === 'object') return 'badge-error';
  const map: Record<string, string> = {
    pending: 'badge-pending',
    extracting: 'badge-extracting',
    metadata_found: 'badge-found',
    needs_review: 'badge-review',
    confirmed: 'badge-found',
    renamed: 'badge-renamed',
  };
  return map[status] ?? 'badge-pending';
}

export function getConfidenceLabel(c: Confidence): string {
  return { low: 'Faible', medium: 'Moyenne', high: 'Haute' }[c] ?? c;
}

export function getSourceLabel(s: MetadataSource): string {
  const map: Record<MetadataSource, string> = {
    unknown: 'Inconnu',
    file: 'Fichier',
    filename: 'Nom de fichier',
    openlibrary: 'OpenLibrary',
    crossref: 'Crossref',
    googlebooks: 'Google Books',
    amazon: 'Amazon',
    bnf: 'BNF (France)',
  };
  return map[s] ?? s;
}

export const TEMPLATE_VARIABLES = [
  { name: '{auteur-nom}', description: 'Nom de famille du premier auteur' },
  { name: '{auteur-prenom}', description: 'Prénom(s) du premier auteur' },
  { name: '{auteur}', description: 'Premier auteur tel quel (sans reformater)' },
  { name: '{titre}', description: 'Titre du livre/article' },
  { name: '{date}', description: 'Année de publication' },
  { name: '{editeur}', description: 'Éditeur' },
  { name: '{isbn}', description: 'ISBN-13' },
  { name: '{doi}', description: 'DOI' },
  { name: '{extension}', description: 'Extension du fichier (epub, pdf)' },
];

export const DEFAULT_CONFIG: AppConfig = {
  templates: [
    { id: 'default', name: 'Standard', pattern: '{auteur-nom} {auteur-prenom} - {titre} ({date}) [{editeur}]', is_default: true },
    { id: 'court', name: 'Court', pattern: '{auteur-nom} - {titre} ({date})', is_default: false },
    { id: 'article', name: 'Article/DOI', pattern: '{auteur-nom} {auteur-prenom} - {titre} ({date}) [DOI {doi}]', is_default: false },
    { id: 'isbn', name: 'Livre/ISBN', pattern: '{auteur-nom} - {titre} ({date}) [ISBN {isbn}]', is_default: false },
  ],
  active_template_id: 'default',
  google_books_api_key: null,
  enabled_sources: {
    openlibrary: true,
    crossref: true,
    google_books: false,
    amazon: true,
    bnf: true,
  },
};
