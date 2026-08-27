use serde::{Deserialize, Serialize};

/// Métadonnées unifiées d'un livre ou article.
/// Toutes les sources (fichier, nom brut, API) convergent vers cette structure.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct BookMetadata {
    pub title: Option<String>,
    pub authors: Vec<String>,
    /// Année ou date complète (YYYY ou YYYY-MM-DD)
    pub date: Option<String>,
    pub publisher: Option<String>,
    pub isbn: Option<String>,
    pub doi: Option<String>,
    pub source: MetadataSource,
    pub confidence: Confidence,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum MetadataSource {
    #[default]
    Unknown,
    File,
    Filename,
    OpenLibrary,
    Crossref,
    GoogleBooks,
    Amazon,
    Bnf,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Confidence {
    #[default]
    Low,
    Medium,
    High,
}

/// État d'un fichier dans la file de traitement
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum FileStatus {
    Pending,
    Extracting,
    MetadataFound,
    NeedsReview,
    Confirmed,
    Renamed,
    Error(String),
}

/// Entrée dans la queue de traitement
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QueueEntry {
    pub id: String,
    pub original_path: String,
    pub file_name: String,
    pub extension: String,
    pub status: FileStatus,
    pub metadata: Option<BookMetadata>,
    /// Autres résultats API si la recherche textuelle renvoie plusieurs résultats
    pub alternative_results: Vec<BookMetadata>,
    pub proposed_name: Option<String>,
}

impl QueueEntry {
    pub fn new(path: String) -> Self {
        use std::path::Path;
        let p = Path::new(&path);
        let file_name = p
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();
        let extension = p
            .extension()
            .unwrap_or_default()
            .to_string_lossy()
            .to_lowercase();
        let id = uuid_from_path(&path);
        QueueEntry {
            id,
            original_path: path,
            file_name,
            extension,
            status: FileStatus::Pending,
            metadata: None,
            alternative_results: Vec::new(),
            proposed_name: None,
        }
    }
}

fn uuid_from_path(path: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut h = DefaultHasher::new();
    path.hash(&mut h);
    format!("{:016x}", h.finish())
}

/// Template de nommage sauvegardé
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NamingTemplate {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub is_default: bool,
}

impl NamingTemplate {
    /// Templates préconfigurés fournis avec l'application
    pub fn defaults() -> Vec<NamingTemplate> {
        vec![
            NamingTemplate {
                id: "default".into(),
                name: "Standard".into(),
                pattern: "{auteur-nom} {auteur-prenom} - {titre} ({date}) [{editeur}]".into(),
                is_default: true,
            },
            NamingTemplate {
                id: "court".into(),
                name: "Court".into(),
                pattern: "{auteur-nom} - {titre} ({date})".into(),
                is_default: false,
            },
            NamingTemplate {
                id: "article".into(),
                name: "Article / DOI".into(),
                pattern: "{auteur-nom} {auteur-prenom} - {titre} ({date}) [DOI {doi}]".into(),
                is_default: false,
            },
            NamingTemplate {
                id: "isbn".into(),
                name: "Livre / ISBN".into(),
                pattern: "{auteur-nom} - {titre} ({date}) [ISBN {isbn}]".into(),
                is_default: false,
            },
        ]
    }
}

/// Configuration globale de l'application
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub templates: Vec<NamingTemplate>,
    pub active_template_id: String,
    /// Clé Google Books API (optionnelle)
    pub google_books_api_key: Option<String>,
    /// Sources API activées
    pub enabled_sources: EnabledSources,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EnabledSources {
    pub openlibrary: bool,
    pub crossref: bool,
    pub google_books: bool,
    pub amazon: bool,
    pub bnf: bool,
}

impl Default for AppConfig {
    fn default() -> Self {
        AppConfig {
            templates: NamingTemplate::defaults(),
            active_template_id: "default".into(),
            google_books_api_key: None,
            enabled_sources: EnabledSources {
                openlibrary: true,
                crossref: true,
                google_books: false,
                amazon: true,
                bnf: true,
            },
        }
    }
}
