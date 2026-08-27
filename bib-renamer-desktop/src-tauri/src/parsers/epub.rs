use anyhow::{Context, Result};
use epub::doc::EpubDoc;
use std::io::BufReader;
use std::path::Path;

use crate::metadata::{BookMetadata, Confidence, MetadataSource};

/// Extrait les métadonnées d'un fichier EPUB via le content.opf (balises Dublin Core).
pub fn extract_epub_metadata(path: &Path) -> Result<BookMetadata> {
    let mut doc = EpubDoc::new(path)
        .with_context(|| format!("Impossible d'ouvrir le fichier EPUB : {:?}", path))?;

    // epub v2.x retourne des &MetadataItem ; on accède au champ `.value`
    let title = doc
        .mdata("title")
        .map(|item| clean_string(&item.value));

    let authors = extract_authors(&mut doc);

    let date = doc
        .mdata("date")
        .map(|item| extract_year_from_date(&item.value));

    let publisher = doc
        .mdata("publisher")
        .map(|item| clean_string(&item.value));

    let isbn = extract_epub_isbn(&mut doc);

    // Niveau de confiance
    let confidence = if title.is_some() && !authors.is_empty() {
        if isbn.is_some() || date.is_some() {
            Confidence::High
        } else {
            Confidence::Medium
        }
    } else {
        Confidence::Low
    };

    Ok(BookMetadata {
        title,
        authors,
        date,
        publisher,
        isbn,
        doi: None,
        source: MetadataSource::File,
        confidence,
    })
}

/// Extrait les auteurs depuis les métadonnées EPUB.
fn extract_authors(doc: &mut EpubDoc<BufReader<std::fs::File>>) -> Vec<String> {
    if let Some(item) = doc.mdata("creator") {
        let cleaned = clean_string(&item.value);
        if !cleaned.is_empty() {
            return vec![cleaned];
        }
    }
    if let Some(item) = doc.mdata("author") {
        let cleaned = clean_string(&item.value);
        if !cleaned.is_empty() {
            return vec![cleaned];
        }
    }
    Vec::new()
}

/// Extrait un ISBN depuis les identifiants Dublin Core du fichier EPUB.
fn extract_epub_isbn(doc: &mut EpubDoc<BufReader<std::fs::File>>) -> Option<String> {
    if let Some(item) = doc.mdata("identifier") {
        let identifier_val = &item.value;

        // Chercher directement via regex
        if let Some(isbn) = crate::heuristics::extract_isbn(identifier_val) {
            return Some(isbn);
        }

        // URN : "urn:isbn:9782021478006"
        if let Some(pos) = identifier_val.to_lowercase().find("isbn:") {
            let candidate = &identifier_val[pos + 5..];
            let cleaned: String = candidate.chars().filter(|c| c.is_ascii_digit()).collect();
            if cleaned.len() == 13 || cleaned.len() == 10 {
                return Some(cleaned);
            }
        }
    }
    None
}

/// Nettoie une chaîne de métadonnée.
fn clean_string(s: &str) -> String {
    s.trim()
        .trim_matches(|c| c == '"' || c == '\'')
        .to_string()
}

/// Extrait l'année depuis une date ("2022-05-12" → "2022").
fn extract_year_from_date(date_str: &str) -> String {
    if date_str.len() >= 4 {
        let year_part = &date_str[..4];
        if year_part.chars().all(|c| c.is_ascii_digit()) {
            return year_part.to_string();
        }
    }
    date_str.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_year_from_date() {
        assert_eq!(extract_year_from_date("2022-05-12"), "2022");
        assert_eq!(extract_year_from_date("2019"), "2019");
    }

    #[test]
    fn test_clean_string() {
        assert_eq!(clean_string("  \"Title\" "), "Title");
    }
}
