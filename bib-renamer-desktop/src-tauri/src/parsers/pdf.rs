use anyhow::{Context, Result};
use lopdf::Document;
use std::path::Path;

use crate::metadata::{BookMetadata, Confidence, MetadataSource};

/// Extrait les métadonnées d'un fichier PDF.
///
/// Stratégie :
/// 1. Lecture du dictionnaire `/Info` via lopdf (Title, Author, Subject, CreationDate).
/// 2. Si insuffisant, extraction du texte des 2 premières pages pour chercher ISBN/DOI cachés.
pub fn extract_pdf_metadata(path: &Path) -> Result<BookMetadata> {
    let doc = Document::load(path)
        .with_context(|| format!("Impossible d'ouvrir le fichier PDF : {:?}", path))?;

    let (title, authors, date, publisher) = extract_info_dict(&doc);
    let isbn_from_info = authors
        .first()
        .and_then(|_| None::<String>)
        .or_else(|| extract_identifiers_from_info(&doc).0);

    // Extraction dans le dictionnaire /Info
    let (isbn_info, doi_info) = extract_identifiers_from_info(&doc);

    // Extraction textuelle des premières pages si les identifiants sont absents
    let (isbn_text, doi_text) = if isbn_info.is_none() || doi_info.is_none() {
        extract_identifiers_from_text(&doc)
    } else {
        (None, None)
    };

    let isbn = isbn_info.or(isbn_text);
    let doi = doi_info.or(doi_text);

    let confidence = if title.is_some() && !authors.is_empty() {
        if isbn.is_some() || doi.is_some() {
            Confidence::High
        } else {
            Confidence::Medium
        }
    } else if title.is_some() || !authors.is_empty() {
        Confidence::Low
    } else {
        Confidence::Low
    };

    // Éviter l'avertissement du compilateur sur isbn_from_info
    let _ = isbn_from_info;

    Ok(BookMetadata {
        title,
        authors,
        date,
        publisher,
        isbn,
        doi,
        source: MetadataSource::File,
        confidence,
    })
}

/// Lit le dictionnaire `/Info` du PDF et retourne les champs de base.
fn extract_info_dict(doc: &Document) -> (Option<String>, Vec<String>, Option<String>, Option<String>) {
    let trailer = &doc.trailer;

    // Récupérer le dictionnaire Info
    let info_ref = match trailer.get(b"Info") {
        Ok(lopdf::Object::Reference(r)) => *r,
        _ => return (None, Vec::new(), None, None),
    };

    let info = match doc.get_object(info_ref) {
        Ok(lopdf::Object::Dictionary(d)) => d,
        _ => return (None, Vec::new(), None, None),
    };

    let get_str = |key: &[u8]| -> Option<String> {
        info.get(key).ok().and_then(|obj| match obj {
            lopdf::Object::String(bytes, _) => {
                decode_pdf_string(bytes)
            }
            _ => None,
        })
    };

    let title = get_str(b"Title").map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    let author_raw = get_str(b"Author");
    let authors: Vec<String> = author_raw
        .map(|s| {
            // Les auteurs peuvent être séparés par ; ou ,
            if s.contains(';') {
                s.split(';').map(|a| a.trim().to_string()).filter(|a| !a.is_empty()).collect()
            } else {
                vec![s.trim().to_string()]
            }
        })
        .unwrap_or_default();

    let date = get_str(b"CreationDate").and_then(|d| extract_pdf_year(&d));
    let publisher = get_str(b"Producer")
        .or_else(|| get_str(b"Creator"))
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());

    (title, authors, date, publisher)
}

/// Cherche ISBN et DOI dans les champs de métadonnées du PDF.
fn extract_identifiers_from_info(doc: &Document) -> (Option<String>, Option<String>) {
    let trailer = &doc.trailer;
    let info_ref = match trailer.get(b"Info") {
        Ok(lopdf::Object::Reference(r)) => *r,
        _ => return (None, None),
    };
    let info = match doc.get_object(info_ref) {
        Ok(lopdf::Object::Dictionary(d)) => d,
        _ => return (None, None),
    };

    // Concaténer tous les champs string pour chercher les identifiants
    let all_text: String = info
        .iter()
        .filter_map(|(_, v)| match v {
            lopdf::Object::String(bytes, _) => decode_pdf_string(bytes),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(" ");

    let isbn = crate::heuristics::extract_isbn(&all_text);
    let doi = crate::heuristics::extract_doi(&all_text);
    (isbn, doi)
}

/// Extrait le texte des 2 premières pages du PDF et y cherche ISBN/DOI.
/// Utilisé comme fallback quand le dictionnaire /Info est vide ou pauvre.
fn extract_identifiers_from_text(doc: &Document) -> (Option<String>, Option<String>) {
    let pages = doc.get_pages();
    let mut combined_text = String::new();

    for (i, (page_num, page_id)) in pages.iter().enumerate() {
        if i >= 2 {
            break;
        }
        if let Ok(text) = doc.extract_text(&[*page_num]) {
            combined_text.push(' ');
            combined_text.push_str(&text);
        }
        // Silence the unused variable warning
        let _ = page_id;
    }

    let isbn = crate::heuristics::extract_isbn(&combined_text);
    let doi = crate::heuristics::extract_doi(&combined_text);
    (isbn, doi)
}

/// Décode une chaîne PDF (UTF-16BE ou PDFDocEncoding).
fn decode_pdf_string(bytes: &[u8]) -> Option<String> {
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        // UTF-16BE BOM
        let utf16: Vec<u16> = bytes[2..]
            .chunks_exact(2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]))
            .collect();
        String::from_utf16(&utf16).ok()
    } else {
        // PDFDocEncoding approximé par Latin-1
        Some(bytes.iter().map(|&b| b as char).collect())
    }
}

/// Extrait l'année depuis une date PDF (format D:YYYYMMDDHHmmSS ou ISO).
fn extract_pdf_year(date: &str) -> Option<String> {
    // Format PDF standard : D:20221231235959
    let d = date.trim_start_matches("D:");
    if d.len() >= 4 && d[..4].chars().all(|c| c.is_ascii_digit()) {
        return Some(d[..4].to_string());
    }
    // Fallback : chercher une année dans la chaîne
    crate::heuristics::extract_year(date)
}
