use std::fs;
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::metadata::{AppConfig, BookMetadata, NamingTemplate, QueueEntry};
use crate::template::apply_template;
use crate::{api, heuristics, parsers};

// ─── Commandes Tauri ──────────────────────────────────────────────────────

/// Scan un chemin (fichier ou dossier) et retourne la liste des entrées de queue.
#[tauri::command]
pub async fn scan_path(path: String) -> Result<Vec<QueueEntry>, String> {
    let p = PathBuf::from(&path);
    let mut entries = Vec::new();

    if p.is_file() {
        if is_supported(&p) {
            entries.push(QueueEntry::new(path));
        }
    } else if p.is_dir() {
        // Scan récursif via walkdir
        for entry in walkdir::WalkDir::new(&p)
            .follow_links(true)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry.file_type().is_file() && is_supported(entry.path()) {
                entries.push(QueueEntry::new(
                    entry.path().to_string_lossy().to_string(),
                ));
            }
        }
    }

    Ok(entries)
}

/// Extrait les métadonnées d'un fichier de la queue.
/// 1. Métadonnées internes (EPUB/PDF)
/// 2. Parsing heuristique du nom brut
/// Retourne une QueueEntry enrichie.
#[tauri::command]
pub async fn extract_metadata(entry: QueueEntry) -> Result<QueueEntry, String> {
    let path = PathBuf::from(&entry.original_path);
    let ext = entry.extension.as_str();

    // 1. Extraction interne
    let mut meta = match ext {
        "epub" => parsers::epub::extract_epub_metadata(&path)
            .unwrap_or_default(),
        "pdf" => parsers::pdf::extract_pdf_metadata(&path)
            .unwrap_or_default(),
        _ => BookMetadata::default(),
    };

    // 2. Parsing heuristique du nom de fichier (enrichissement si champs manquants)
    let parsed = heuristics::parse_filename(&entry.file_name);

    if meta.isbn.is_none() {
        meta.isbn = parsed.isbn;
    }
    if meta.doi.is_none() {
        meta.doi = parsed.doi;
    }
    if meta.date.is_none() {
        meta.date = parsed.year;
    }
    if meta.title.is_none() && !parsed.parts.is_empty() {
        // Heuristique : la partie la plus longue est probablement le titre
        meta.title = parsed.parts.iter().max_by_key(|p| p.len()).cloned();
    }

    let mut updated = entry;
    updated.metadata = Some(meta);
    updated.status = crate::metadata::FileStatus::MetadataFound;
    Ok(updated)
}

/// Lance l'enrichissement API pour une entrée de queue.
/// Retourne une liste de résultats alternatifs (peut en avoir plusieurs si recherche textuelle).
#[tauri::command]
pub async fn enrich_entry(
    entry: QueueEntry,
    config: AppConfig,
) -> Result<QueueEntry, String> {
    let meta = match &entry.metadata {
        Some(m) => m.clone(),
        None => return Err("Métadonnées non extraites".to_string()),
    };

    let results = api::enrich_metadata(
        &meta,
        config.google_books_api_key.as_deref(),
        config.enabled_sources.openlibrary,
        config.enabled_sources.crossref,
        config.enabled_sources.google_books,
        config.enabled_sources.amazon,
        config.enabled_sources.bnf,
    )
    .await
    .map_err(|e| e.to_string())?;

    let mut updated = entry;

    if results.is_empty() {
        updated.status = crate::metadata::FileStatus::NeedsReview;
    } else {
        // Le premier résultat devient les métadonnées principales
        updated.metadata = Some(results[0].clone());
        // Les suivants sont les alternatives
        updated.alternative_results = results[1..].to_vec();
        updated.status = if results.len() > 1 {
            crate::metadata::FileStatus::NeedsReview
        } else {
            crate::metadata::FileStatus::MetadataFound
        };
    }

    Ok(updated)
}

/// Recherche en ligne par titre, auteur ou ISBN depuis la modal
#[tauri::command]
pub async fn search_online(
    query: String,
    config: AppConfig,
    isbn: Option<String>,
) -> Result<Vec<BookMetadata>, String> {
    let client = api::build_client().map_err(|e| e.to_string())?;
    let results = api::search_by_query(
        &client,
        &query,
        config.google_books_api_key.as_deref(),
        config.enabled_sources.openlibrary,
        config.enabled_sources.bnf,
        config.enabled_sources.crossref,
        config.enabled_sources.google_books,
        config.enabled_sources.amazon,
        isbn.as_deref(),
    )
    .await;
    Ok(results)
}

/// Génère le nom proposé pour une entrée selon le template actif.
#[tauri::command]
pub fn generate_proposed_name(
    entry: QueueEntry,
    template: NamingTemplate,
) -> Result<QueueEntry, String> {
    let meta = entry
        .metadata
        .as_ref()
        .ok_or("Pas de métadonnées")?;

    let proposed = apply_template(&template, meta, &entry.extension);
    let mut updated = entry;
    updated.proposed_name = Some(proposed);
    Ok(updated)
}

/// Effectue le renommage physique du fichier sur le disque.
/// Retourne le nouveau chemin absolu.
#[tauri::command]
pub async fn rename_file(entry: QueueEntry) -> Result<QueueEntry, String> {
    let proposed = entry
        .proposed_name
        .as_ref()
        .ok_or("Aucun nom proposé à appliquer")?;

    let original = PathBuf::from(&entry.original_path);
    let parent = original
        .parent()
        .ok_or("Impossible de trouver le dossier parent")?;

    let new_path = parent.join(proposed);

    // Vérifier que le fichier source existe encore
    if !original.exists() {
        return Err(format!("Fichier source introuvable : {:?}", original));
    }

    // Vérifier qu'on ne va pas écraser un fichier existant
    if new_path.exists() && new_path != original {
        return Err(format!(
            "Un fichier avec ce nom existe déjà : {}",
            proposed
        ));
    }

    fs::rename(&original, &new_path).map_err(|e| e.to_string())?;

    let mut updated = entry;
    updated.original_path = new_path.to_string_lossy().to_string();
    updated.file_name = new_path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_string();
    updated.status = crate::metadata::FileStatus::Renamed;
    Ok(updated)
}

/// Charge la configuration depuis le fichier de config utilisateur.
#[tauri::command]
pub fn load_config(app_handle: tauri::AppHandle) -> Result<AppConfig, String> {
    let config_path = get_config_path(&app_handle);
    if config_path.exists() {
        let content = fs::read_to_string(&config_path).map_err(|e| e.to_string())?;
        serde_json::from_str(&content).map_err(|e| e.to_string())
    } else {
        Ok(AppConfig::default())
    }
}

/// Sauvegarde la configuration.
#[tauri::command]
pub fn save_config(app_handle: tauri::AppHandle, config: AppConfig) -> Result<(), String> {
    let config_path = get_config_path(&app_handle);
    if let Some(parent) = config_path.parent() {
        fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let content = serde_json::to_string_pretty(&config).map_err(|e| e.to_string())?;
    fs::write(&config_path, content).map_err(|e| e.to_string())
}

// ─── Helpers ──────────────────────────────────────────────────────────────

fn is_supported(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(|s| s.to_lowercase())
            .as_deref(),
        Some("epub") | Some("pdf")
    )
}

fn get_config_path(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_config_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("config.json")
}
