use crate::heuristics::sanitize_filename;
use crate::metadata::{BookMetadata, NamingTemplate};

/// Applique un template de nommage aux métadonnées d'un livre.
///
/// Variables supportées :
/// - `{auteur}` → premier auteur tel quel (sans reformater)
/// - `{auteur-nom}` → nom de famille
/// - `{auteur-prenom}` → prénom(s)
/// - `{titre}` → titre ou "Sans titre"
/// - `{date}` → année ou "s.d."
/// - `{editeur}` → éditeur ou vide
/// - `{isbn}` → ISBN ou vide
/// - `{doi}` → DOI ou vide
/// - `{extension}` → extension sans point (epub, pdf…)
pub fn apply_template(
    template: &NamingTemplate,
    metadata: &BookMetadata,
    extension: &str,
) -> String {
    let (auteur_nom, auteur_prenom) = first_author_parts(&metadata.authors);
    let auteur = metadata
        .authors
        .first()
        .cloned()
        .unwrap_or_else(|| "Auteur inconnu".to_string());

    let titre = metadata
        .title
        .clone()
        .unwrap_or_else(|| "Sans titre".to_string());

    let date = metadata.date.clone().unwrap_or_else(|| "s.d.".to_string());

    let editeur = metadata.publisher.clone().unwrap_or_default();
    let isbn = metadata.isbn.clone().unwrap_or_default();
    let doi = metadata.doi.clone().unwrap_or_default();
    let ext = extension.trim_start_matches('.');

    let mut result = template.pattern.clone();

    // Remplacement des variables (ordre : plus spécifiques d'abord)
    result = result.replace("{auteur-nom}", &auteur_nom);
    result = result.replace("{auteur-prenom}", &auteur_prenom);
    result = result.replace("{auteur}", &auteur);
    result = result.replace("{titre}", &titre);
    result = result.replace("{date}", &date);
    result = result.replace("{extension}", ext);

    // Variables qui peuvent être vides → nettoyage des balises orphelines
    result = replace_optional(&result, "{editeur}", &editeur);
    result = replace_optional(&result, "{isbn}", &isbn);
    result = replace_optional(&result, "{doi}", &doi);

    // Nettoyer les espaces multiples laissés par prénom/nom vides
    let result = clean_empty_brackets(&result);

    // Ajouter l'extension
    let final_name = format!("{}.{}", result.trim(), ext);

    // Sanitiser pour tous les OS
    sanitize_filename(&final_name)
}

/// Découpe le premier auteur en (nom, prénom).
/// Accepte « Nom, Prénom » ou « Prénom Nom ».
pub fn parse_author_parts(name: &str) -> (String, String) {
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return (String::new(), String::new());
    }

    if trimmed.contains(',') {
        let mut parts = trimmed.splitn(2, ',');
        let nom = parts.next().unwrap_or("").trim().to_string();
        let prenom = parts.next().unwrap_or("").trim().to_string();
        return (nom, prenom);
    }

    let parts: Vec<&str> = trimmed.split_whitespace().collect();
    match parts.as_slice() {
        [] => (String::new(), String::new()),
        [only] => ((*only).to_string(), String::new()),
        [prenom @ .., nom] => (nom.to_string(), prenom.join(" ")),
    }
}

fn first_author_parts(authors: &[String]) -> (String, String) {
    match authors.first() {
        Some(a) => parse_author_parts(a),
        None => ("Auteur inconnu".to_string(), String::new()),
    }
}

/// Remplace une variable dans le template, en supprimant son contexte si elle est vide.
///
/// Par exemple, si `{editeur}` est vide et que le pattern contient `[{editeur}]`,
/// le crochet entier `[...]` est supprimé pour éviter `[]` dans le nom final.
fn replace_optional(template: &str, var: &str, value: &str) -> String {
    if value.is_empty() {
        // Supprimer `[{var}]`, `({var})`, `{var}` et les séparateurs orphelins
        let bracketed = [
            format!("[{}]", var),
            format!("({})", var),
            format!("{{{}}}", var.trim_matches('{').trim_matches('}')),
        ];
        let mut result = template.to_string();
        for pattern in &bracketed {
            result = result.replace(pattern, "");
        }
        result.replace(var, "")
    } else {
        template.replace(var, value)
    }
}

/// Nettoie les crochets/parenthèses vides laissés après remplacement de champs vides.
fn clean_empty_brackets(s: &str) -> String {
    let mut result = s.to_string();
    loop {
        let cleaned = regex::Regex::new(r"\[\s*\]|\(\s*\)|\{\s*\}")
            .unwrap()
            .replace_all(&result, "")
            .to_string();
        let cleaned = regex::Regex::new(r"\s{2,}")
            .unwrap()
            .replace_all(&cleaned, " ")
            .to_string();
        // " - " or leading/trailing separators left by empty prenom
        let cleaned = regex::Regex::new(r"\s+-\s+-")
            .unwrap()
            .replace_all(&cleaned, " -")
            .to_string();
        if cleaned == result {
            break;
        }
        result = cleaned;
    }
    result
}

/// Prévisualise le nom généré sans l'écrire sur le disque.
pub fn preview_name(
    template: &NamingTemplate,
    metadata: &BookMetadata,
    extension: &str,
) -> String {
    apply_template(template, metadata, extension)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metadata::{BookMetadata, NamingTemplate};

    fn make_meta() -> BookMetadata {
        BookMetadata {
            title: Some("La vérité nue".to_string()),
            authors: vec!["Hans Blumenberg".to_string()],
            date: Some("2022".to_string()),
            publisher: Some("Seuil Jeunesse".to_string()),
            isbn: Some("9782021478006".to_string()),
            doi: None,
            ..Default::default()
        }
    }

    #[test]
    fn test_parse_author_parts() {
        assert_eq!(
            parse_author_parts("Hans Blumenberg"),
            ("Blumenberg".into(), "Hans".into())
        );
        assert_eq!(
            parse_author_parts("Blumenberg, Hans"),
            ("Blumenberg".into(), "Hans".into())
        );
        assert_eq!(
            parse_author_parts("Jean-Paul Sartre"),
            ("Sartre".into(), "Jean-Paul".into())
        );
    }

    #[test]
    fn test_standard_template() {
        let tpl = NamingTemplate {
            id: "test".into(),
            name: "Test".into(),
            pattern: "{auteur-nom} {auteur-prenom} - {titre} ({date}) [{editeur}]".into(),
            is_default: true,
        };
        let result = apply_template(&tpl, &make_meta(), "epub");
        assert!(result.starts_with("Blumenberg Hans - "));
        assert!(result.contains("La vérité nue"));
        assert!(result.contains("2022"));
        assert!(result.contains("Seuil Jeunesse"));
        assert!(result.ends_with(".epub"));
        assert!(!result.contains(','));
    }

    #[test]
    fn test_auteur_raw_no_forced_comma() {
        let tpl = NamingTemplate {
            id: "test".into(),
            name: "Test".into(),
            pattern: "{auteur} - {titre}".into(),
            is_default: false,
        };
        let result = apply_template(&tpl, &make_meta(), "epub");
        assert!(result.starts_with("Hans Blumenberg - "));
        assert!(!result.contains("Blumenberg, Hans"));
    }

    #[test]
    fn test_empty_publisher_cleans_brackets() {
        let tpl = NamingTemplate {
            id: "test".into(),
            name: "Test".into(),
            pattern: "{auteur-nom} - {titre} ({date}) [{editeur}]".into(),
            is_default: false,
        };
        let mut meta = make_meta();
        meta.publisher = None;
        let result = apply_template(&tpl, &meta, "epub");
        assert!(!result.contains("[]"));
        assert!(!result.contains("[  ]"));
    }

    #[test]
    fn test_sanitize_special_chars() {
        let tpl = NamingTemplate {
            id: "test".into(),
            name: "Test".into(),
            pattern: "{titre}".into(),
            is_default: false,
        };
        let mut meta = make_meta();
        meta.title = Some("Titre: Avec / Slashes * Partout".to_string());
        let result = apply_template(&tpl, &meta, "pdf");
        assert!(!result.contains('/'));
        assert!(!result.contains(':'));
        assert!(!result.contains('*'));
    }
}
