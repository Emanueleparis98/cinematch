//! Documenti Wikimedia via API pubblica, senza chiavi o scraping delle pagine.
//! Collegamento identità TMDB -> Wikidata -> Wikipedia, non primo omonimo.
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use reqwest::blocking::Client;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::domain::model::{Details, Movie};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Document {
    pub url: String,
    pub title: String,
    pub text: String,
    pub retrieved_at: u64,
    pub license: String,
}
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
impl Document {
    pub fn tmdb(m: &Movie) -> Self {
        Self {
            url: format!("https://www.themoviedb.org/movie/{}", m.id),
            title: m.title.clone(),
            text: m.overview.clone(),
            retrieved_at: now(),
            license: "TMDB API terms; synopsis".into(),
        }
    }
}

pub struct NarrativeSource {
    http: Client,
}
impl NarrativeSource {
    pub fn new() -> Result<Self, String> {
        let http = Client::builder()
            .user_agent(concat!(
                "CineMatch/",
                env!("CARGO_PKG_VERSION"),
                " (https://github.com/Emanueleparis98/cinematch)"
            ))
            .timeout(Duration::from_secs(12))
            .connect_timeout(Duration::from_secs(5))
            .build()
            .map_err(|_| "Impossibile inizializzare la fonte narrativa.".to_string())?;
        Ok(Self { http })
    }
    fn get(
        &self,
        url: &str,
        params: &[(&str, &str)],
        cancel: &AtomicBool,
    ) -> Result<Value, String> {
        if cancel.load(Ordering::Relaxed) {
            return Err("Richiesta interrotta.".into());
        }
        let response = self
            .http
            .get(url)
            .query(params)
            .send()
            .map_err(|_| "Fonte Wikimedia non raggiungibile.".to_string())?;
        if !response.status().is_success() {
            return Err(format!(
                "Fonte Wikimedia non disponibile ({}).",
                response.status().as_u16()
            ));
        }
        let mut bytes = Vec::new();
        let mut reader = response.take(1_500_001);
        let mut buffer = [0; 8192];
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Richiesta interrotta.".into());
            }
            let count = reader
                .read(&mut buffer)
                .map_err(|_| "Documento Wikimedia incompleto.".to_string())?;
            if count == 0 {
                break;
            }
            bytes.extend_from_slice(&buffer[..count]);
        }
        if bytes.len() > 1_500_000 {
            return Err("Documento Wikimedia troppo grande.".into());
        }
        serde_json::from_slice(&bytes).map_err(|_| "Documento Wikimedia non leggibile.".into())
    }
    pub fn discover(
        &self,
        query: &str,
        page: u32,
        cancel: &AtomicBool,
    ) -> Result<Vec<u64>, String> {
        let offset = page.saturating_sub(1).saturating_mul(6).to_string();
        let query: String = query.chars().take(250).collect();
        let v = self.get(
            "https://en.wikipedia.org/w/api.php",
            &[
                ("action", "query"),
                ("generator", "search"),
                ("gsrsearch", &query),
                ("gsrnamespace", "0"),
                ("gsrlimit", "6"),
                ("gsroffset", &offset),
                ("prop", "pageprops"),
                ("ppprop", "wikibase_item"),
                ("format", "json"),
                ("formatversion", "2"),
            ],
            cancel,
        )?;
        let qids: Vec<&str> = v["query"]["pages"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|p| p["pageprops"]["wikibase_item"].as_str())
            .filter(|q| valid_qid(q))
            .collect();
        if qids.is_empty() {
            return Ok(Vec::new());
        }
        let ids = qids.join("|");
        let entities = self.get(
            "https://www.wikidata.org/w/api.php",
            &[
                ("action", "wbgetentities"),
                ("ids", &ids),
                ("props", "claims"),
                ("format", "json"),
            ],
            cancel,
        )?;
        Ok(tmdb_ids(&entities))
    }
    pub fn for_movie(
        &self,
        details: &Details,
        cancel: &AtomicBool,
    ) -> Result<Option<Document>, String> {
        let Some(qid) = &details.wikidata_id else {
            return Ok(None);
        };
        if !valid_qid(qid) {
            return Ok(None);
        }
        let dir = crate::infra::config::Config::dir().join("narrative-cache-v2");
        let path = dir.join(format!("{}-{qid}.json", details.movie.id));
        if let Ok(bytes) = std::fs::read(&path) {
            if let Ok(doc) = serde_json::from_slice::<Document>(&bytes) {
                if now().saturating_sub(doc.retrieved_at) < 7 * 24 * 3600 {
                    return Ok(Some(doc));
                }
            }
        }
        let v = self.get(
            "https://www.wikidata.org/w/api.php",
            &[
                ("action", "wbgetentities"),
                ("ids", qid),
                ("props", "sitelinks"),
                ("sitefilter", "enwiki|itwiki"),
                ("format", "json"),
            ],
            cancel,
        )?;
        let links = &v["entities"][qid]["sitelinks"];
        let (lang, page) = if let Some(title) = links["enwiki"]["title"].as_str() {
            ("en", title)
        } else if let Some(title) = links["itwiki"]["title"].as_str() {
            ("it", title)
        } else {
            return Ok(None);
        };
        let api = format!("https://{lang}.wikipedia.org/w/api.php");
        // exchars ha un massimo API di 1200: non usarlo per una trama estesa.
        // Limitiamo il corpo in lettura e il testo localmente dopo la risposta.
        let v = self.get(
            &api,
            &[
                ("action", "query"),
                ("prop", "extracts|info"),
                ("inprop", "url"),
                ("explaintext", "1"),
                ("titles", page),
                ("redirects", "1"),
                ("format", "json"),
                ("formatversion", "2"),
            ],
            cancel,
        )?;
        let Some(p) = v["query"]["pages"].as_array().and_then(|p| p.first()) else {
            return Ok(None);
        };
        let Some(text) = p["extract"].as_str().filter(|s| !s.trim().is_empty()) else {
            return Ok(None);
        };
        let Some(url) = p["fullurl"].as_str() else {
            return Ok(None);
        };
        let doc = Document { url:url.into(), title:p["title"].as_str().unwrap_or(page).into(),
            text:text.chars().take(14000).collect(), retrieved_at:now(),
            license:"Wikipedia contributors, CC BY-SA 4.0; https://creativecommons.org/licenses/by-sa/4.0/".into() };
        // Cache limitata: non un archivio mondiale. La scadenza evita che un
        // documento immutabile diventi una verifica permanente della trama.
        let _ = std::fs::create_dir_all(&dir);
        if std::fs::read_dir(&dir)
            .map(|entries| entries.count())
            .unwrap_or(0)
            < 1000
            || path.exists()
        {
            if let Ok(bytes) = serde_json::to_vec(&doc) {
                let _ = std::fs::write(path, bytes);
            }
        }
        Ok(Some(doc))
    }
}
fn valid_qid(qid: &str) -> bool {
    qid.starts_with('Q') && qid.len() > 1 && qid[1..].bytes().all(|b| b.is_ascii_digit())
}

fn tmdb_ids(v: &Value) -> Vec<u64> {
    let mut ids = Vec::new();
    for entity in v["entities"]
        .as_object()
        .into_iter()
        .flat_map(|e| e.values())
    {
        for claim in entity["claims"]["P4947"].as_array().into_iter().flatten() {
            if claim["rank"] == "deprecated" {
                continue;
            }
            if let Some(id) = claim["mainsnak"]["datavalue"]["value"]
                .as_str()
                .and_then(|s| s.parse::<u64>().ok())
                .filter(|id| *id > 0)
            {
                if !ids.contains(&id) {
                    ids.push(id);
                }
            }
        }
    }
    ids
}

#[cfg(test)]
mod tests {
    #[test]
    fn document_search_only_resolves_canonical_non_deprecated_movie_ids() {
        let v = serde_json::json!({"entities":{"Q1":{"claims":{"P4947":[
            {"rank":"normal","mainsnak":{"datavalue":{"value":"123"}}},
            {"rank":"deprecated","mainsnak":{"datavalue":{"value":"999"}}},
            {"rank":"normal","mainsnak":{"datavalue":{"value":"not-an-id"}}}
        ]}},"Q2":{"claims":{"P4947":[{"mainsnak":{"datavalue":{"value":"123"}}}]}}}});
        assert_eq!(super::tmdb_ids(&v), vec![123]);
    }
    #[test]
    fn identifiers_cannot_change_the_api_target_or_cache_path() {
        for id in ["", "Q", "../Q1", "Q1/../", "Q1&site=evil", "Qé"] {
            assert!(!super::valid_qid(id));
        }
        assert!(super::valid_qid("Q12345"));
    }
}
