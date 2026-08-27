use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::Deserialize;

use crate::metadata::{BookMetadata, Confidence, MetadataSource};

// ─── Structures de désérialisation des réponses API ───────────────────────

// OpenLibrary /api/books
#[derive(Deserialize, Debug)]
struct OlBook {
    title: Option<String>,
    authors: Option<Vec<OlAuthor>>,
    publish_date: Option<String>,
    publishers: Option<Vec<OlPublisher>>,
}

#[derive(Deserialize, Debug)]
struct OlAuthor {
    name: String,
}

#[derive(Deserialize, Debug)]
struct OlPublisher {
    name: String,
}

// Crossref /works/{doi}
#[derive(Deserialize, Debug)]
struct CrossrefResponse {
    message: CrossrefMessage,
}

#[derive(Deserialize, Debug)]
struct CrossrefMessage {
    title: Option<Vec<String>>,
    author: Option<Vec<CrossrefAuthor>>,
    created: Option<CrossrefDate>,
    publisher: Option<String>,
    #[serde(rename = "DOI")]
    doi: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CrossrefAuthor {
    given: Option<String>,
    family: Option<String>,
}

#[derive(Deserialize, Debug)]
struct CrossrefDate {
    #[serde(rename = "date-parts")]
    date_parts: Vec<Vec<u32>>,
}

// Crossref search results
#[derive(Deserialize, Debug)]
struct CrossrefSearchResponse {
    message: CrossrefSearchMessage,
}

#[derive(Deserialize, Debug)]
struct CrossrefSearchMessage {
    items: Vec<CrossrefMessage>,
}

// Google Books /volumes
#[derive(Deserialize, Debug)]
struct GBooksResponse {
    items: Option<Vec<GBooksItem>>,
}

#[derive(Deserialize, Debug)]
struct GBooksItem {
    #[serde(rename = "volumeInfo")]
    volume_info: GBooksVolumeInfo,
}

#[derive(Deserialize, Debug)]
struct GBooksVolumeInfo {
    title: Option<String>,
    authors: Option<Vec<String>>,
    #[serde(rename = "publishedDate")]
    published_date: Option<String>,
    publisher: Option<String>,
    #[serde(rename = "industryIdentifiers")]
    industry_identifiers: Option<Vec<GBooksIdentifier>>,
}

#[derive(Deserialize, Debug)]
struct GBooksIdentifier {
    #[serde(rename = "type")]
    id_type: String,
    identifier: String,
}

// ─── Client HTTP partagé ──────────────────────────────────────────────────

/// Crée un client reqwest avec headers réalistes
pub fn build_client() -> Result<Client> {
    Ok(Client::builder()
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0.0.0 Safari/537.36")
        .timeout(std::time::Duration::from_secs(12))
        .build()?)
}

// ─── OpenLibrary ──────────────────────────────────────────────────────────

/// Recherche par ISBN sur OpenLibrary (essaie search.json puis api/books)
pub async fn fetch_openlibrary_by_isbn(client: &Client, isbn: &str) -> Result<BookMetadata> {
    let clean_isbn = isbn.replace(['-', ' '], "");

    // 1. Essayer search.json?q={isbn} (très complet et indexe toutes les éditions)
    let search_url = format!("https://openlibrary.org/search.json?q={}&limit=1", clean_isbn);
    if let Ok(resp) = client.get(&search_url).send().await {
        if let Ok(json) = resp.json::<serde_json::Value>().await {
            if let Some(doc) = json["docs"].as_array().and_then(|a| a.first()) {
                if let Some(title) = doc["title"].as_str() {
                    let authors: Vec<String> = doc["author_name"]
                        .as_array()
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                        .unwrap_or_default();
                    let date = doc["first_publish_year"].as_u64().map(|y| y.to_string());
                    let publisher = doc["publisher"]
                        .as_array()
                        .and_then(|a| a.first())
                        .and_then(|v| v.as_str())
                        .map(str::to_string);

                    return Ok(BookMetadata {
                        title: Some(title.to_string()),
                        authors,
                        date,
                        publisher,
                        isbn: Some(clean_isbn),
                        doi: None,
                        source: MetadataSource::OpenLibrary,
                        confidence: Confidence::High,
                    });
                }
            }
        }
    }

    // 2. Fallback sur api/books?bibkeys=ISBN:...
    let url = format!(
        "https://openlibrary.org/api/books?bibkeys=ISBN:{}&format=json&jscmd=data",
        clean_isbn
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    let key = format!("ISBN:{}", clean_isbn);

    let book = resp
        .get(&key)
        .ok_or_else(|| anyhow!("ISBN {} non trouvé sur OpenLibrary", isbn))?;

    let book: OlBook = serde_json::from_value(book.clone())?;

    Ok(BookMetadata {
        title: book.title,
        authors: book
            .authors
            .unwrap_or_default()
            .into_iter()
            .map(|a| a.name)
            .collect(),
        date: book.publish_date.and_then(|d| extract_year_from_str(&d)),
        publisher: book
            .publishers
            .and_then(|p| p.into_iter().next())
            .map(|p| p.name),
        isbn: Some(clean_isbn),
        doi: None,
        source: MetadataSource::OpenLibrary,
        confidence: Confidence::High,
    })
}

/// Recherche textuelle sur OpenLibrary (titre / auteur / mots-clés).
/// Enchaîne plusieurs variantes de requête si la première ne donne rien
/// (comme le plugin Obsidian, avec repli accent / auteur).
pub async fn search_openlibrary(client: &Client, query: &str) -> Result<Vec<BookMetadata>> {
    let q = query.trim();
    if q.is_empty() {
        return Ok(Vec::new());
    }

    let mut candidates = vec![q.to_string()];

    let normalized = normalize_search_query(q);
    if normalized != q {
        candidates.push(normalized.clone());
    }

    // "Titre Nom, Prénom" → essayer "Nom Prénom" puis "Nom"
    if let Some(author) = author_hint_from_query(q) {
        candidates.push(author.clone());
        if let Some(last) = author.split_whitespace().next() {
            candidates.push(last.to_string());
        }
    }

    let mut seen = std::collections::HashSet::new();
    for candidate in candidates {
        if !seen.insert(candidate.to_lowercase()) {
            continue;
        }
        let results = search_openlibrary_once(client, &candidate).await?;
        if !results.is_empty() {
            return Ok(results);
        }
    }

    Ok(Vec::new())
}

async fn search_openlibrary_once(client: &Client, query: &str) -> Result<Vec<BookMetadata>> {
    let url = format!(
        "https://openlibrary.org/search.json?q={}&limit=5",
        urlencoding::encode(query.trim())
    );

    let resp: serde_json::Value = client.get(&url).send().await?.json().await?;
    let docs = resp["docs"].as_array().cloned().unwrap_or_default();

    let results = docs
        .into_iter()
        .take(5)
        .filter_map(|doc| {
            let title = doc["title"].as_str()?.to_string();
            let authors: Vec<String> = doc["author_name"]
                .as_array()
                .map(|a| a.iter().filter_map(|v| v.as_str().map(str::to_string)).collect())
                .unwrap_or_default();
            let date = doc["first_publish_year"]
                .as_u64()
                .map(|y| y.to_string());
            let publisher = doc["publisher"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .map(str::to_string);
            let isbn = doc["isbn"]
                .as_array()
                .and_then(|a| a.first())
                .and_then(|v| v.as_str())
                .map(str::to_string);

            Some(BookMetadata {
                title: Some(title),
                authors,
                date,
                publisher,
                isbn,
                doi: None,
                source: MetadataSource::OpenLibrary,
                confidence: Confidence::Medium,
            })
        })
        .collect();

    Ok(results)
}

// ─── BNF (Bibliothèque Nationale de France - SRU XML) ─────────────────────

/// Recherche d'un livre par ISBN sur le catalogue de la BNF
pub async fn fetch_bnf_by_isbn(client: &Client, isbn: &str) -> Result<BookMetadata> {
    let clean = isbn.replace(['-', ' '], "");
    let isbn10 = to_isbn10(&clean);

    let queries = vec![
        format!("bib.isbn all \"{}\"", clean),
        format!("bib.anywhere all \"{}\"", clean),
        format!("bib.isbn all \"{}\"", isbn10.as_deref().unwrap_or(&clean)),
    ];

    for q in queries {
        let url = format!(
            "https://catalogue.bnf.fr/api/SRU?version=1.2&operation=searchRetrieve&query={}&recordSchema=dublincore",
            urlencoding::encode(&q)
        );

        if let Ok(resp) = client.get(&url).send().await {
            if let Ok(xml) = resp.text().await {
                if xml.contains("<srw:numberOfRecords>0</srw:numberOfRecords>") {
                    continue;
                }

                let title = extract_xml_tag(&xml, "dc:title").map(|t| clean_bnf_title(&t));
                let creator = extract_xml_tag(&xml, "dc:creator").map(|c| clean_bnf_author(&c));
                let date = extract_xml_tag(&xml, "dc:date").and_then(|d| extract_year_from_str(&d));
                let publisher = extract_xml_tag(&xml, "dc:publisher").map(|p| clean_bnf_publisher(&p));

                if title.is_some() {
                    return Ok(BookMetadata {
                        title,
                        authors: creator.into_iter().collect(),
                        date,
                        publisher,
                        isbn: Some(clean),
                        doi: None,
                        source: MetadataSource::Bnf,
                        confidence: Confidence::High,
                    });
                }
            }
        }
    }

    Err(anyhow!("ISBN {} non trouvé sur la BNF", isbn))
}

/// Recherche textuelle sur le catalogue de la BNF
pub async fn search_bnf(client: &Client, query: &str) -> Result<Vec<BookMetadata>> {
    let q = format!("bib.anywhere all \"{}\"", query.trim());
    let url = format!(
        "https://catalogue.bnf.fr/api/SRU?version=1.2&operation=searchRetrieve&query={}&recordSchema=dublincore&maximumRecords=5",
        urlencoding::encode(&q)
    );

    let mut results = Vec::new();
    if let Ok(resp) = client.get(&url).send().await {
        if let Ok(xml) = resp.text().await {
            // Découper par notice <srw:record>
            for record in xml.split("<srw:record>").skip(1) {
                let title = extract_xml_tag(record, "dc:title").map(|t| clean_bnf_title(&t));
                let creator = extract_xml_tag(record, "dc:creator").map(|c| clean_bnf_author(&c));
                let date = extract_xml_tag(record, "dc:date").and_then(|d| extract_year_from_str(&d));
                let publisher =
                    extract_xml_tag(record, "dc:publisher").map(|p| clean_bnf_publisher(&p));
                let isbn = extract_bnf_isbn(record);

                if let Some(t) = title {
                    results.push(BookMetadata {
                        title: Some(t),
                        authors: creator.into_iter().collect(),
                        date,
                        publisher,
                        isbn,
                        doi: None,
                        source: MetadataSource::Bnf,
                        confidence: Confidence::Medium,
                    });
                }
            }
        }
    }

    Ok(results)
}


fn extract_bnf_isbn(xml: &str) -> Option<String> {
    let mut rest = xml;
    while let Some(pos) = rest.find("<dc:identifier") {
        rest = &rest[pos..];
        let Some(end) = rest.find("</dc:identifier>") else { break };
        let chunk = &rest[..end];
        rest = &rest[end + "</dc:identifier>".len()..];
        let content = chunk.rsplit('>').next().unwrap_or("").trim();
        let lower = content.to_ascii_lowercase();
        let digits: String = content
            .chars()
            .filter(|c| c.is_ascii_alphanumeric())
            .collect::<String>()
            .to_uppercase();
        let candidate = if lower.contains("isbn") {
            digits
                .trim_start_matches(|c: char| c.is_ascii_alphabetic())
                .to_string()
        } else {
            digits
        };
        if candidate.len() == 13 && candidate.chars().all(|c| c.is_ascii_digit()) {
            return Some(candidate);
        }
        if candidate.len() == 10 {
            let body_ok = candidate[..9].chars().all(|c| c.is_ascii_digit());
            let check = candidate.chars().last().unwrap();
            if body_ok && (check.is_ascii_digit() || check == 'X') {
                return Some(candidate);
            }
        }
    }
    None
}

fn extract_xml_tag(xml: &str, tag: &str) -> Option<String> {
    let open = format!("<{}>", tag);
    let close = format!("</{}>", tag);
    if let Some(start) = xml.find(&open) {
        if let Some(end) = xml[start + open.len()..].find(&close) {
            let content = &xml[start + open.len()..start + open.len() + end];
            return Some(html_escape::decode_html_entities(content).to_string());
        }
    }
    None
}

fn clean_bnf_title(title: &str) -> String {
    let t = title.split('/').next().unwrap_or(title);
    t.trim().to_string()
}

fn clean_bnf_author(author: &str) -> String {
    let a = author.split('(').next().unwrap_or(author);
    let a = a.split('.').next().unwrap_or(a);
    a.trim().to_string()
}

fn clean_bnf_publisher(publ: &str) -> String {
    let p = publ.split('(').next().unwrap_or(publ);
    p.trim().to_string()
}

// ─── Amazon ───────────────────────────────────────────────────────────────

/// Recherche par ISBN sur Amazon (via page produit Amazon.fr / Amazon.com)
pub async fn fetch_amazon_by_isbn(client: &Client, isbn: &str) -> Result<BookMetadata> {
    let clean = isbn.replace(['-', ' '], "");
    let isbn10 = to_isbn10(&clean).unwrap_or_else(|| clean.clone());

    let urls = vec![
        format!("https://www.amazon.fr/dp/{}", isbn10),
        format!("https://www.amazon.com/dp/{}", isbn10),
    ];

    for url in urls {
        let resp = client
            .get(&url)
            .header(
                "Accept-Language",
                "fr-FR,fr;q=0.9,en-US;q=0.8,en;q=0.7",
            )
            .header(
                "Accept",
                "text/html,application/xhtml+xml,application/xml;q=0.9,*/*;q=0.8",
            )
            .send()
            .await;

        if let Ok(res) = resp {
            if res.status().is_success() {
                if let Ok(html) = res.text().await {
                    // Pages trop courtes = soft-block / captcha / redirection vide
                    if html.len() < 2000
                        || html.contains("Robot Check")
                        || html.contains("Page introuvable")
                        || html.contains("/errors/validateCaptcha")
                    {
                        continue;
                    }

                    let title = extract_amazon_title(&html);
                    let author = extract_amazon_author(&html);

                    if let Some(t) = title {
                        return Ok(BookMetadata {
                            title: Some(t),
                            authors: author.into_iter().collect(),
                            date: None,
                            publisher: None,
                            isbn: Some(clean),
                            doi: None,
                            source: MetadataSource::Amazon,
                            confidence: Confidence::High,
                        });
                    }
                }
            }
        }
    }

    Err(anyhow!("ISBN {} non trouvé sur Amazon", isbn))
}

fn extract_amazon_title(html: &str) -> Option<String> {
    let re_id = regex::Regex::new(r#"id=["']productTitle["'][^>]*>\s*([\s\S]*?)\s*</span"#).ok()?;
    if let Some(caps) = re_id.captures(html) {
        let t = caps[1].trim();
        if !t.is_empty() {
            return Some(html_escape::decode_html_entities(t).to_string());
        }
    }

    let re_title_tag = regex::Regex::new(r#"<title>\s*([\s\S]*?)\s*</title>"#).ok()?;
    if let Some(caps) = re_title_tag.captures(html) {
        let mut t = caps[1].trim().to_string();
        t = t.replace("Amazon.fr :", "").replace("Amazon.fr:", "").replace("Amazon.com:", "").replace(": Livres", "");
        t = t.trim().to_string();
        if !t.is_empty() && !t.contains("Page introuvable") && !t.contains("Robot Check") {
            return Some(html_escape::decode_html_entities(&t).to_string());
        }
    }
    None
}

fn extract_amazon_author(html: &str) -> Option<String> {
    let re = regex::Regex::new(r#"class=["']author[^"']*["'][^>]*>[\s\S]*?<a[^>]*>\s*([\s\S]*?)\s*</a>"#).ok()?;
    if let Some(caps) = re.captures(html) {
        let a = caps[1].trim();
        if !a.is_empty() {
            return Some(html_escape::decode_html_entities(a).to_string());
        }
    }
    None
}

/// Convertit un ISBN-13 (préfixe 978) en ISBN-10
pub fn to_isbn10(isbn13: &str) -> Option<String> {
    let clean = isbn13.replace(['-', ' '], "");
    if clean.starts_with("978") && clean.len() == 13 {
        let core = &clean[3..12];
        let mut sum = 0;
        for (i, c) in core.chars().enumerate() {
            let d = c.to_digit(10)?;
            sum += d * (10 - i as u32);
        }
        let rem = (11 - (sum % 11)) % 11;
        let check = if rem == 10 { 'X' } else { char::from_digit(rem, 10)? };
        Some(format!("{}{}", core, check))
    } else if clean.len() == 10 {
        Some(clean)
    } else {
        None
    }
}

// ─── Crossref ─────────────────────────────────────────────────────────────

/// Recherche par DOI sur Crossref.
pub async fn fetch_crossref_by_doi(client: &Client, doi: &str) -> Result<BookMetadata> {
    let url = format!("https://api.crossref.org/works/{}", urlencoding::encode(doi));
    let resp: CrossrefResponse = client.get(&url).send().await?.json().await?;
    Ok(crossref_message_to_metadata(resp.message, Confidence::High))
}

/// Recherche textuelle sur Crossref
pub async fn search_crossref(client: &Client, query: &str) -> Result<Vec<BookMetadata>> {
    let url = format!(
        "https://api.crossref.org/works?query.bibliographic={}&rows=4",
        urlencoding::encode(query.trim())
    );

    let resp: CrossrefSearchResponse = client.get(&url).send().await?.json().await?;
    Ok(resp
        .message
        .items
        .into_iter()
        .map(|m| crossref_message_to_metadata(m, Confidence::Medium))
        .collect())
}

fn crossref_message_to_metadata(msg: CrossrefMessage, confidence: Confidence) -> BookMetadata {
    let title = msg.title.and_then(|t| t.into_iter().next());
    let authors: Vec<String> = msg
        .author
        .unwrap_or_default()
        .into_iter()
        .map(|a| match (a.given, a.family) {
            (Some(g), Some(f)) => format!("{} {}", g, f),
            (None, Some(f)) => f,
            (Some(g), None) => g,
            (None, None) => String::new(),
        })
        .filter(|s| !s.is_empty())
        .collect();

    let date = msg
        .created
        .and_then(|d| d.date_parts.into_iter().next())
        .and_then(|parts| parts.into_iter().next())
        .map(|y| y.to_string());

    BookMetadata {
        title,
        authors,
        date,
        publisher: msg.publisher,
        isbn: None,
        doi: msg.doi,
        source: MetadataSource::Crossref,
        confidence,
    }
}

// ─── Google Books (optionnel) ─────────────────────────────────────────────

/// Recherche par ISBN sur Google Books API
pub async fn fetch_google_books_by_isbn(
    client: &Client,
    isbn: &str,
    api_key: Option<&str>,
) -> Result<BookMetadata> {
    let mut url = format!(
        "https://www.googleapis.com/books/v1/volumes?q=isbn:{}",
        isbn
    );
    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            url.push_str(&format!("&key={}", key.trim()));
        }
    }

    let resp: GBooksResponse = client.get(&url).send().await?.json().await?;
    let item = resp
        .items
        .and_then(|items| items.into_iter().next())
        .ok_or_else(|| anyhow!("ISBN {} non trouvé sur Google Books", isbn))?;

    let vi = item.volume_info;
    let isbn_found = vi
        .industry_identifiers
        .as_ref()
        .and_then(|ids| {
            ids.iter()
                .find(|id| id.id_type == "ISBN_13" || id.id_type == "ISBN_10")
        })
        .map(|id| id.identifier.clone())
        .unwrap_or_else(|| isbn.to_string());

    Ok(BookMetadata {
        title: vi.title,
        authors: vi.authors.unwrap_or_default(),
        date: vi.published_date.and_then(|d| extract_year_from_str(&d)),
        publisher: vi.publisher,
        isbn: Some(isbn_found),
        doi: None,
        source: MetadataSource::GoogleBooks,
        confidence: Confidence::High,
    })
}

/// Recherche textuelle sur Google Books
pub async fn search_google_books(
    client: &Client,
    query: &str,
    api_key: Option<&str>,
) -> Result<Vec<BookMetadata>> {
    let mut url = format!(
        "https://www.googleapis.com/books/v1/volumes?q={}&maxResults=4",
        urlencoding::encode(query.trim())
    );
    if let Some(key) = api_key {
        if !key.trim().is_empty() {
            url.push_str(&format!("&key={}", key.trim()));
        }
    }

    let resp: GBooksResponse = client.get(&url).send().await?.json().await?;
    let items = resp.items.unwrap_or_default();

    let results = items
        .into_iter()
        .map(|item| {
            let vi = item.volume_info;
            let isbn_found = vi.industry_identifiers.as_ref().and_then(|ids| {
                ids.iter()
                    .find(|id| id.id_type == "ISBN_13" || id.id_type == "ISBN_10")
                    .map(|id| id.identifier.clone())
            });

            BookMetadata {
                title: vi.title,
                authors: vi.authors.unwrap_or_default(),
                date: vi.published_date.and_then(|d| extract_year_from_str(&d)),
                publisher: vi.publisher,
                isbn: isbn_found,
                doi: None,
                source: MetadataSource::GoogleBooks,
                confidence: Confidence::Medium,
            }
        })
        .collect();

    Ok(results)
}

// ─── Recherche textuelle directe (pour la modal et le fallback) ───────────

/// Recherche multi-sources (ISBN si détecté + texte).
/// Amazon n'a pas de recherche texte fiable : il n'est interrogé que par ISBN
/// (aligné sur le plugin Obsidian).
pub async fn search_by_query(
    client: &Client,
    query: &str,
    google_books_key: Option<&str>,
    enabled_openlibrary: bool,
    enabled_bnf: bool,
    enabled_crossref: bool,
    enabled_google_books: bool,
    enabled_amazon: bool,
    isbn_hint: Option<&str>,
) -> Vec<BookMetadata> {
    let q = query.trim();
    let isbn = isbn_hint
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.replace(['-', ' '], ""))
        .or_else(|| extract_isbn_from_query(q));

    let mut all = Vec::new();

    // ── Branche ISBN (haute confiance, inclut Amazon) ─────────────────────
    if let Some(ref isbn) = isbn {
        let (ol_isbn, bnf_isbn, amz_isbn, gb_isbn) = tokio::join!(
            async {
                if enabled_openlibrary {
                    fetch_openlibrary_by_isbn(client, isbn).await.ok()
                } else {
                    None
                }
            },
            async {
                if enabled_bnf {
                    fetch_bnf_by_isbn(client, isbn).await.ok()
                } else {
                    None
                }
            },
            async {
                if enabled_amazon {
                    fetch_amazon_by_isbn(client, isbn).await.ok()
                } else {
                    None
                }
            },
            async {
                if enabled_google_books {
                    fetch_google_books_by_isbn(client, isbn, google_books_key)
                        .await
                        .ok()
                } else {
                    None
                }
            }
        );

        for m in [ol_isbn, bnf_isbn, amz_isbn, gb_isbn].into_iter().flatten() {
            all.push(m);
        }
    }

    let text_query = if let Some(ref isbn) = isbn {
        let stripped = strip_isbn_from_query(q, isbn);
        if stripped.is_empty() {
            None
        } else {
            Some(stripped)
        }
    } else if !q.is_empty() {
        Some(q.to_string())
    } else {
        None
    };

    if let Some(text_q) = text_query {
        let (ol_res, bnf_res, cr_res, gb_res) = tokio::join!(
            async {
                if enabled_openlibrary {
                    search_openlibrary(client, &text_q).await.unwrap_or_default()
                } else {
                    Vec::new()
                }
            },
            async {
                if enabled_bnf {
                    search_bnf(client, &text_q).await.unwrap_or_default()
                } else {
                    Vec::new()
                }
            },
            async {
                if enabled_crossref {
                    search_crossref(client, &text_q).await.unwrap_or_default()
                } else {
                    Vec::new()
                }
            },
            async {
                if enabled_google_books {
                    search_google_books(client, &text_q, google_books_key)
                        .await
                        .unwrap_or_default()
                } else {
                    Vec::new()
                }
            }
        );

        all.extend(ol_res);
        all.extend(bnf_res);
        all.extend(cr_res);
        all.extend(gb_res);
    }

    // Amazon n'a pas de recherche texte fiable : compléter via les ISBN
    // trouvés par les autres sources (ou l'ISBN fourni).
    if enabled_amazon {
        let already_amazon: std::collections::HashSet<String> = all
            .iter()
            .filter(|m| matches!(m.source, MetadataSource::Amazon))
            .filter_map(|m| m.isbn.as_ref().map(|i| i.replace(['-', ' '], "")))
            .collect();

        let mut seen_isbn = std::collections::HashSet::new();
        let mut to_fetch: Vec<String> = Vec::new();

        let mut push_isbn = |raw: &str| {
            let clean = raw.replace(['-', ' '], "");
            if clean.len() >= 10
                && !already_amazon.contains(&clean)
                && seen_isbn.insert(clean.clone())
                && to_fetch.len() < 3
            {
                to_fetch.push(clean);
            }
        };

        if let Some(ref isbn) = isbn {
            push_isbn(isbn);
        }
        for m in &all {
            if let Some(ref i) = m.isbn {
                push_isbn(i);
            }
        }

        for isbn in to_fetch {
            if let Ok(m) = fetch_amazon_by_isbn(client, &isbn).await {
                all.push(m);
            }
        }
    }

    dedupe_metadata(all)
}

// ─── Cascade principale ───────────────────────────────────────────────────

/// Orchestre la cascade complète d'appels API
pub async fn enrich_metadata(
    metadata: &BookMetadata,
    google_books_key: Option<&str>,
    enabled_openlibrary: bool,
    enabled_crossref: bool,
    enabled_google_books: bool,
    enabled_amazon: bool,
    enabled_bnf: bool,
) -> Result<Vec<BookMetadata>> {
    let client = build_client()?;

    // ── 1. Branche ISBN : toutes les sources en parallèle (plus d'arrêt anticipé)
    if let Some(isbn) = &metadata.isbn {
        let isbn_results = search_by_query(
            &client,
            "",
            google_books_key,
            enabled_openlibrary,
            enabled_bnf,
            enabled_crossref,
            enabled_google_books,
            enabled_amazon,
            Some(isbn),
        )
        .await;

        if !isbn_results.is_empty() {
            return Ok(isbn_results);
        }
    }

    // ── 2. Branche DOI ────────────────────────────────────────────────────
    if let Some(doi) = &metadata.doi {
        if enabled_crossref {
            if let Ok(m) = fetch_crossref_by_doi(&client, doi).await {
                return Ok(vec![m]);
            }
        }
    }

    // ── 3. Fallback automatique par Recherche textuelle ────────────────────
    let title = metadata.title.as_deref().unwrap_or("");
    let author = metadata.authors.first().map(|s| s.as_str()).unwrap_or("");
    let query = if !title.is_empty() && !author.is_empty() {
        format!("{} {}", title, author)
    } else if !title.is_empty() {
        title.to_string()
    } else if !author.is_empty() {
        author.to_string()
    } else {
        String::new()
    };

    if !query.is_empty() {
        return Ok(search_by_query(
            &client,
            &query,
            google_books_key,
            enabled_openlibrary,
            enabled_bnf,
            enabled_crossref,
            enabled_google_books,
            enabled_amazon,
            metadata.isbn.as_deref(),
        )
        .await);
    }

    Ok(Vec::new())
}

// ─── Helpers ─────────────────────────────────────────────────────────────

fn normalize_search_query(s: &str) -> String {
    let mapped: String = s
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'À' | 'Á' | 'Â' | 'Ä' | 'Ã' => 'a',
            'è' | 'é' | 'ê' | 'ë' | 'È' | 'É' | 'Ê' | 'Ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' | 'Ì' | 'Í' | 'Î' | 'Ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' | 'Ò' | 'Ó' | 'Ô' | 'Ö' | 'Õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' | 'Ù' | 'Ú' | 'Û' | 'Ü' => 'u',
            'ç' | 'Ç' => 'c',
            'ñ' | 'Ñ' => 'n',
            'ÿ' | 'Ÿ' => 'y',
            'ß' => 's',
            ',' | ';' | ':' | '!' | '?' | '"' | '\'' => ' ',
            other => other.to_ascii_lowercase(),
        })
        .collect();
    mapped.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// Extrait un indice auteur depuis "Titre Nom, Prénom".
fn author_hint_from_query(query: &str) -> Option<String> {
    let (left, right) = query.rsplit_once(',')?;
    let first_name = right.trim();
    let last_name = left.trim().split_whitespace().last()?;
    if first_name.is_empty() || last_name.is_empty() {
        return None;
    }
    let starts_upper = last_name
        .chars()
        .next()
        .map(|c| c.is_uppercase())
        .unwrap_or(false);
    if !starts_upper {
        return None;
    }
    Some(format!("{} {}", last_name, first_name))
}

fn extract_isbn_from_query(query: &str) -> Option<String> {
    let re = regex::Regex::new(
        r"(?i)(?:ISBN(?:-1[03])?[:\s]*)?((?:97[89][-\s]?)?(?:\d[-\s]?){9}[\dXx])",
    )
    .ok()?;
    let caps = re.captures(query)?;
    let raw = caps.get(1)?.as_str();
    let clean: String = raw.chars().filter(|c| c.is_ascii_alphanumeric()).collect();
    let clean = clean.to_uppercase();
    if clean.len() == 10 || clean.len() == 13 {
        Some(clean)
    } else {
        None
    }
}

fn strip_isbn_from_query(query: &str, isbn: &str) -> String {
    let mut out = query.to_string();
    for variant in [
        isbn.to_string(),
        isbn.to_lowercase(),
        isbn
            .chars()
            .enumerate()
            .flat_map(|(i, c)| {
                if i > 0 && i % 3 == 0 {
                    vec!['-', c]
                } else {
                    vec![c]
                }
            })
            .collect::<String>(),
    ] {
        out = out.replace(&variant, " ");
    }
    out = regex::Regex::new(r"(?i)ISBN(?:-1[03])?[:\s]*")
        .map(|re| re.replace_all(&out, " ").to_string())
        .unwrap_or(out);
    out.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn dedupe_metadata(items: Vec<BookMetadata>) -> Vec<BookMetadata> {
    let mut seen = std::collections::HashSet::new();
    let mut out = Vec::new();
    for item in items {
        let key = format!(
            "{}|{}|{:?}",
            item.title.as_deref().unwrap_or("").to_lowercase(),
            item.authors.first().map(|a| a.to_lowercase()).unwrap_or_default(),
            item.source
        );
        if seen.insert(key) {
            out.push(item);
        }
    }
    out
}

fn extract_year_from_str(s: &str) -> Option<String> {
    crate::heuristics::extract_year(s).or_else(|| {
        if s.len() >= 4 && s[..4].chars().all(|c| c.is_ascii_digit()) {
            Some(s[..4].to_string())
        } else {
            None
        }
    })
}

// Décodage HTML simple
mod html_escape {
    pub fn decode_html_entities(s: &str) -> String {
        let mut out = s
            .replace("&amp;", "&")
            .replace("&lt;", "<")
            .replace("&gt;", ">")
            .replace("&quot;", "\"")
            .replace("&#34;", "\"")
            .replace("&#39;", "'")
            .replace("&apos;", "'")
            .replace("&nbsp;", " ")
            .replace("&#x27;", "'");

        // Entités numériques décimales restantes : &#NNN;
        if let Ok(re) = regex::Regex::new(r"&#(\d+);") {
            out = re
                .replace_all(&out, |caps: &regex::Captures| {
                    caps.get(1)
                        .and_then(|m| m.as_str().parse::<u32>().ok())
                        .and_then(char::from_u32)
                        .map(|c| c.to_string())
                        .unwrap_or_else(|| caps[0].to_string())
                })
                .to_string();
        }
        out
    }
}
