use once_cell::sync::Lazy;
use regex::Regex;

// ─── Regex compilées une seule fois ────────────────────────────────────────

static RE_MD5: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b[a-f0-9]{32}\b").unwrap());

static RE_NOISE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"(?i)(anna'?s?\s*archive|libgen|z-?lib(?:rary)?|zlibrary|download|bookfi|b-ok|genesis)").unwrap()
});

static RE_ISBN13: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:isbn(?:-13)?[:\s]*)?(97[89][\d\-]{10,14})\b").unwrap());

static RE_ISBN10: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?i)\b(?:isbn(?:-10)?[:\s]*)?([\dX]{9}[\dX])\b").unwrap());

static RE_DOI: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(10\.\d{4,9}/[-._;()/:A-Za-z0-9]+)\b").unwrap());

static RE_YEAR: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\b(19[5-9]\d|20[0-2]\d)\b").unwrap());

/// Séparateurs de champs dans un nom de fichier brut
static RE_SEPARATORS: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"\s*--\s*|\s+-\s+|_").unwrap());

// ─── Résultats de l'analyse heuristique ────────────────────────────────────

#[derive(Debug, Default)]
pub struct ParsedFilename {
    pub cleaned: String,
    pub isbn: Option<String>,
    pub doi: Option<String>,
    pub year: Option<String>,
    /// Champs inférés après split sur séparateurs (titre probable, auteur probable…)
    pub parts: Vec<String>,
}

// ─── Fonctions publiques ────────────────────────────────────────────────────

/// Nettoie et analyse un nom de fichier brut.
/// Retourne les identifiants détectés et les parties du nom découpées.
pub fn parse_filename(raw: &str) -> ParsedFilename {
    // Retirer l'extension
    let stem = strip_extension(raw);

    // 1. Extraction des identifiants AVANT nettoyage (pour préserver les patterns)
    let isbn = extract_isbn(&stem);
    let doi = extract_doi(&stem);
    let year = extract_year(&stem);

    // 2. Nettoyage des parasites
    let cleaned = clean_noise(&stem);

    // 3. Découpage en parties
    let parts = split_parts(&cleaned);

    ParsedFilename {
        cleaned,
        isbn,
        doi,
        year,
        parts,
    }
}

/// Extrait un ISBN-13 ou ISBN-10 depuis une chaîne.
pub fn extract_isbn(text: &str) -> Option<String> {
    if let Some(cap) = RE_ISBN13.captures(text) {
        let raw = cap[1].replace(['-', ' '], "");
        if raw.len() == 13 {
            return Some(raw);
        }
    }
    if let Some(cap) = RE_ISBN10.captures(text) {
        let raw = cap[1].to_uppercase();
        if raw.len() == 10 {
            return Some(raw);
        }
    }
    None
}

/// Extrait un DOI depuis une chaîne.
pub fn extract_doi(text: &str) -> Option<String> {
    RE_DOI
        .captures(text)
        .map(|cap| cap[1].trim_end_matches('.').to_string())
}

/// Extrait une année depuis une chaîne.
pub fn extract_year(text: &str) -> Option<String> {
    RE_YEAR.find(text).map(|m| m.as_str().to_string())
}

/// Nettoie les parasites d'un texte (hashes MD5, mots-clés LibGen, etc.)
fn clean_noise(text: &str) -> String {
    let s = RE_MD5.replace_all(text, "");
    let s = RE_NOISE.replace_all(&s, "");
    // Nettoyer les parenthèses/crochets vides résultants
    let s = Regex::new(r"\(\s*\)|\[\s*\]|\{\s*\}")
        .unwrap()
        .replace_all(&s, "");
    // Normaliser les espaces multiples
    let s = Regex::new(r"\s{2,}").unwrap().replace_all(&s, " ");
    s.trim().to_string()
}

/// Découpe un nom de fichier nettoyé en parties (auteur, titre, date…)
fn split_parts(text: &str) -> Vec<String> {
    RE_SEPARATORS
        .split(text)
        .map(|p| p.trim().to_string())
        .filter(|p| !p.is_empty())
        .collect()
}

/// Retire l'extension d'un nom de fichier.
fn strip_extension(filename: &str) -> String {
    if let Some(pos) = filename.rfind('.') {
        filename[..pos].to_string()
    } else {
        filename.to_string()
    }
}

/// Sanitise une chaîne pour qu'elle soit valide comme nom de fichier sur tous les OS.
/// Remplace les caractères interdits par un espace ou les supprime.
pub fn sanitize_filename(s: &str) -> String {
    // Caractères interdits : / \ : * ? " < > | et caractères de contrôle
    let forbidden = Regex::new(r#"[/\\:*?"<>|]"#).unwrap();
    let s = forbidden.replace_all(s, "");
    // Normaliser les espaces multiples
    let s = Regex::new(r"\s{2,}").unwrap().replace_all(&s, " ");
    // Retirer les points et espaces en début/fin (problèmes Windows)
    s.trim_matches(|c| c == '.' || c == ' ').to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_isbn13() {
        assert_eq!(
            extract_isbn("La vérité nue 9782021478006 blah"),
            Some("9782021478006".to_string())
        );
    }

    #[test]
    fn test_extract_doi() {
        let text = "Some article 10.1016/j.cell.2022.01.001 here";
        assert_eq!(
            extract_doi(text),
            Some("10.1016/j.cell.2022.01.001".to_string())
        );
    }

    #[test]
    fn test_extract_year() {
        assert_eq!(extract_year("Blumenberg 2022 Seuil"), Some("2022".to_string()));
    }

    #[test]
    fn test_clean_noise() {
        let input = "La vérité nue d3b07384d113edec49eaa6238ad5ff00 anna's archive 2022";
        let parsed = parse_filename(input);
        assert!(!parsed.cleaned.contains("d3b07384"));
        assert!(!parsed.cleaned.to_lowercase().contains("anna"));
    }

    #[test]
    fn test_parse_parts() {
        let input = "Blumenberg, Hans -- La vérité nue -- 2022.epub";
        let parsed = parse_filename(input);
        assert!(parsed.parts.len() >= 2);
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("File: Name? With* Bad/Chars"),
            "File Name With BadChars"
        );
    }
}
