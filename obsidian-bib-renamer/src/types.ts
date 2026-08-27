export type MetadataSource =
  | 'unknown'
  | 'file'
  | 'filename'
  | 'openlibrary'
  | 'crossref'
  | 'googlebooks'
  | 'amazon'
  | 'bnf';

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

export interface NamingTemplate {
  id: string;
  name: string;
  pattern: string;
  is_default: boolean;
}

export interface PluginSettings {
  templates: NamingTemplate[];
  activeTemplateId: string;
  googleBooksApiKey: string;
  enableOpenLibrary: boolean;
  enableBnf: boolean;
  enableAmazon: boolean;
  enableCrossref: boolean;
  enableGoogleBooks: boolean;
  createReadingNote: boolean;
  readingNotesFolder: string;
}

export const DEFAULT_TEMPLATES: NamingTemplate[] = [
  {
    id: 'default',
    name: 'Standard',
    pattern: '{auteur-nom} {auteur-prenom} - {titre} ({date}) [{editeur}]',
    is_default: true,
  },
  {
    id: 'court',
    name: 'Court',
    pattern: '{auteur-nom} - {titre} ({date})',
    is_default: false,
  },
  {
    id: 'article',
    name: 'Article / DOI',
    pattern: '{auteur-nom} {auteur-prenom} - {titre} ({date}) [DOI {doi}]',
    is_default: false,
  },
  {
    id: 'isbn',
    name: 'Livre / ISBN',
    pattern: '{auteur-nom} - {titre} ({date}) [ISBN {isbn}]',
    is_default: false,
  },
];

export const DEFAULT_SETTINGS: PluginSettings = {
  templates: DEFAULT_TEMPLATES,
  activeTemplateId: 'default',
  googleBooksApiKey: '',
  enableOpenLibrary: true,
  enableBnf: true,
  enableAmazon: true,
  enableCrossref: true,
  enableGoogleBooks: false,
  createReadingNote: false,
  readingNotesFolder: '',
};

export const TEMPLATE_VARIABLES = [
  { name: '{auteur-nom}', description: 'Nom de famille du premier auteur' },
  { name: '{auteur-prenom}', description: 'Prénom(s) du premier auteur' },
  { name: '{auteur}', description: 'Premier auteur tel quel (sans reformater)' },
  { name: '{titre}', description: 'Titre de l’ouvrage / article' },
  { name: '{date}', description: 'Année de publication' },
  { name: '{editeur}', description: 'Maison d’édition' },
  { name: '{isbn}', description: 'Identifiant ISBN' },
  { name: '{doi}', description: 'Identifiant DOI' },
  { name: '{extension}', description: 'Extension du fichier (epub, pdf)' },
];
