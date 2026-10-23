use std::collections::HashMap;
use std::io::Read;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use base64::Engine;
use reqwest::blocking::Client;
use serde::de::DeserializeOwned;

use crate::domain::constraints::DiscoverQuery;
use crate::domain::model::{Details, Sort};
use crate::tmdb::models::{ApiDetails, KeywordResults, PagedMovies};

pub const API_BASE: &str = "https://api.themoviedb.org/3";
pub const IMG_BASE: &str = "https://image.tmdb.org/t/p/";
const CANCELLED: &str = "Richiesta interrotta.";

struct CancelReader<'a> {
    inner: reqwest::blocking::Response,
    cancel: &'a AtomicBool,
}

impl Read for CancelReader<'_> {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        if self.cancel.load(Ordering::Relaxed) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::Interrupted,
                "richiesta interrotta",
            ));
        }
        self.inner.read(buf)
    }
}

pub fn poster_url(path: &str, size: &str) -> String {
    format!("{IMG_BASE}{size}{path}")
}

/// TMDB accetta due credenziali diverse, e non sono intercambiabili:
/// - API Key v3 (32 caratteri) -> parametro `api_key`
/// - API Read Access Token v4 (JWT, inizia con `eyJ`) -> header `Authorization: Bearer`
#[derive(Clone, Debug)]
pub enum Credential {
    Query(String),
    Bearer { token: String, fallback_key: Option<String> },
}

/// Estrae il claim `aud` da un JWT: per i token TMDB coincide con la API Key v3.
fn jwt_aud(token: &str) -> Option<String> {
    let payload = token.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload.trim_end_matches('='))
        .ok()?;
    let value: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    value.get("aud")?.as_str().map(|s| s.to_string())
}

impl Credential {
    pub fn detect(raw: &str) -> Self {
        let token = raw.trim();
        let looks_like_jwt =
            token.starts_with("eyJ") && token.split('.').count() == 3 && token.len() > 60;
        if looks_like_jwt {
            let fallback = jwt_aud(token).filter(|s| s.len() == 32);
            Credential::Bearer {
                token: token.to_string(),
                fallback_key: fallback,
            }
        } else {
            Credential::Query(token.to_string())
        }
    }

    pub fn kind(&self) -> &'static str {
        match self {
            Credential::Query(_) => "API Key v3",
            Credential::Bearer { .. } => "Read Access Token v4 (Bearer)",
        }
    }
}

/// Descrizione leggibile della credenziale incollata, per guidare l'utente.
/// Ritorna (messaggio, formato plausibile).
pub fn describe_credential(raw: &str) -> (String, bool) {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return ("Nessuna credenziale inserita.".to_string(), false);
    }
    match Credential::detect(trimmed) {
        Credential::Bearer { fallback_key, .. } => (
            match fallback_key {
                Some(_) => "Read Access Token v4 (Bearer) · la API Key v3 e stata ricavata dal token come ripiego".to_string(),
                None => "Read Access Token v4 (Bearer) riconosciuto".to_string(),
            },
            true,
        ),
        Credential::Query(key) => {
            let hex = key.chars().all(|c| c.is_ascii_hexdigit());
            if key.len() == 32 && hex {
                ("API Key v3 riconosciuta (32 caratteri)".to_string(), true)
            } else if key.starts_with("eyJ") {
                (
                    format!(
                        "Token v4 incompleto: {} caratteri, il JWT completo ne ha oltre 200 (copialo tutto)",
                        key.len()
                    ),
                    false,
                )
            } else {
                (
                    format!(
                        "Formato inatteso: {} caratteri. Attesi 32 esadecimali (API Key v3) oppure un token che inizia con eyJ",
                        key.len()
                    ),
                    false,
                )
            }
        }
    }
}

/// Client TMDB sincrono, con throttling, retry e cache in memoria.
pub struct Tmdb {
    http: Client,
    cred: Credential,
    lang: String,
    region: String,
    throttle: Arc<Mutex<Instant>>,
    list_cache: Arc<Mutex<HashMap<String, PagedMovies>>>,
    details_cache: Arc<Mutex<HashMap<u64, Details>>>,
    keyword_cache: Arc<Mutex<HashMap<String, Option<u64>>>>,
    /// Solo il client della chat lo imposta: stop interrompe richieste e
    /// letture del corpo senza toccare la griglia.
    cancel: Option<Arc<AtomicBool>>,
}

impl Tmdb {
    pub fn new(credential: String, lang: String) -> Self {
        let http = Client::builder()
            .user_agent("CineMatch/0.1 (Windows)")
            .timeout(Duration::from_secs(25))
            .build()
            .unwrap_or_else(|_| Client::new());
        let start = Instant::now()
            .checked_sub(Duration::from_secs(1))
            .unwrap_or_else(Instant::now);
        let region = lang
            .split('-')
            .nth(1)
            .map(|s| s.to_uppercase())
            .unwrap_or_else(|| "US".to_string());
        Self {
            http,
            cred: Credential::detect(&credential),
            lang,
            region,
            throttle: Arc::new(Mutex::new(start)),
            list_cache: Arc::new(Mutex::new(HashMap::new())),
            details_cache: Arc::new(Mutex::new(HashMap::new())),
            keyword_cache: Arc::new(Mutex::new(HashMap::new())),
            cancel: None,
        }
    }

    /// Client della chat: stessa rete e stesse cache, ma con un token di
    /// annullamento tutto suo. La griglia continua a usare l'originale.
    pub fn cancellable(&self) -> (Self, Arc<AtomicBool>) {
        let cancel = Arc::new(AtomicBool::new(false));
        let copy = Self {
            http: self.http.clone(),
            cred: self.cred.clone(),
            lang: self.lang.clone(),
            region: self.region.clone(),
            throttle: self.throttle.clone(),
            list_cache: self.list_cache.clone(),
            details_cache: self.details_cache.clone(),
            keyword_cache: self.keyword_cache.clone(),
            cancel: Some(cancel.clone()),
        };
        (copy, cancel)
    }

    fn cancelled(&self) -> bool {
        self.cancel
            .as_ref()
            .is_some_and(|c| c.load(Ordering::Relaxed))
    }

    /// Codice regione usato per le date di uscita e i titoli locali.
    pub fn region(&self) -> &str {
        &self.region
    }

    /// Prova di lettura: conferma che la credenziale e valida e la rete funziona.
    pub fn ping(&self) -> Result<String, String> {
        let params: Vec<(String, String)> = vec![("page".to_string(), "1".to_string())];
        let page: PagedMovies = self.get("/trending/movie/week", &params)?;
        Ok(format!(
            "{} riconosciuta · letti {} titoli di tendenza",
            self.cred.kind(),
            page.results.len()
        ))
    }

    fn wait(&self) {
        if let Ok(mut last) = self.throttle.lock() {
            let min = Duration::from_millis(75);
            let elapsed = last.elapsed();
            if elapsed < min {
                std::thread::sleep(min - elapsed);
            }
            *last = Instant::now();
        }
    }

    fn get<T: DeserializeOwned>(&self, path: &str, params: &[(String, String)]) -> Result<T, String> {
        let url = format!("{API_BASE}{path}");
        let mut last_err = String::from("richiesta non eseguita");
        let mut auth = self.cred.clone();
        let mut switched = false;
        for attempt in 0..3u32 {
            if self.cancelled() {
                return Err(CANCELLED.into());
            }
            self.wait();
            if self.cancelled() {
                return Err(CANCELLED.into());
            }
            let request = self.http.get(&url).query(&[("language", self.lang.as_str())]);
            let request = match &auth {
                Credential::Query(key) => request.query(&[("api_key", key.as_str())]),
                Credential::Bearer { token, .. } => request.bearer_auth(token),
            };
            let sent = request.query(params).send();
            if self.cancelled() {
                return Err(CANCELLED.into());
            }
            let resp = match sent {
                Ok(r) => r,
                Err(e) => {
                    last_err = format!("Errore di rete: {e}");
                    if attempt < 2 {
                        std::thread::sleep(Duration::from_millis(500 * (attempt as u64 + 1)));
                        continue;
                    }
                    return Err(last_err);
                }
            };
            let status = resp.status();
            if status.as_u16() == 429 || status.is_server_error() {
                last_err = format!("TMDB ha risposto {}", status.as_u16());
                if self.cancelled() {
                    return Err(CANCELLED.into());
                }
                std::thread::sleep(Duration::from_millis(900 * (attempt as u64 + 1)));
                continue;
            }
            if !status.is_success() {
                // Un token v4 contiene la API Key v3 nel claim `aud`: se il Bearer
                // non passa (proxy, header rimossi) si ripiega sulla query string.
                if status.as_u16() == 401 && !switched {
                    if let Credential::Bearer {
                        fallback_key: Some(key),
                        ..
                    } = &auth
                    {
                        auth = Credential::Query(key.clone());
                        switched = true;
                        continue;
                    }
                }
                let body = self.read_text(resp).unwrap_or_default();
                if self.cancelled() {
                    return Err(CANCELLED.into());
                }
                let snippet: String = body.chars().take(180).collect();
                return Err(match status.as_u16() {
                    401 => format!(
                        "TMDB ha rifiutato la credenziale (401) usando {}. \
                         Incolla la API Key v3 (32 caratteri) oppure il Read Access Token v4 per intero \
                         nelle Impostazioni → API di themoviedb.org.",
                        auth.kind()
                    ),
                    404 => format!("Risorsa non trovata (404): {path}"),
                    code => format!("Errore TMDB {code}: {snippet}"),
                });
            }
            let text = self.read_text(resp)?;
            return serde_json::from_str(&text)
                .map_err(|e| format!("Risposta TMDB non leggibile: {e}"));
        }
        Err(last_err)
    }

    /// Legge il corpo a blocchi: se stop arriva durante il download, il
    /// reader restituisce subito un errore e la connessione viene chiusa.
    fn read_text(&self, resp: reqwest::blocking::Response) -> Result<String, String> {
        let Some(cancel) = &self.cancel else {
            return resp
                .text()
                .map_err(|e| format!("Risposta TMDB non leggibile: {e}"));
        };
        let mut reader = CancelReader {
            inner: resp,
            cancel,
        };
        let mut text = String::new();
        match reader.read_to_string(&mut text) {
            Ok(_) => Ok(text),
            Err(_) if self.cancelled() => Err(CANCELLED.into()),
            Err(e) => Err(format!("Risposta TMDB non leggibile: {e}")),
        }
    }

    fn cache_key(path: &str, params: &[(String, String)]) -> String {
        let mut parts = params
            .iter()
            .map(|(k, v)| format!("{k}={v}"))
            .collect::<Vec<_>>();
        parts.sort();
        format!("{path}?{}", parts.join("&"))
    }

    fn list_cached(&self, key: &str) -> Option<PagedMovies> {
        self.list_cache.lock().ok()?.get(key).cloned()
    }

    fn list_store(&self, key: String, value: &PagedMovies) {
        if let Ok(mut c) = self.list_cache.lock() {
            if c.len() > 400 {
                c.clear();
            }
            c.insert(key, value.clone());
        }
    }

    pub fn discover(&self, q: &DiscoverQuery) -> Result<PagedMovies, String> {
        let mut p: Vec<(String, String)> = vec![
            ("include_adult".into(), "false".into()),
            ("sort_by".into(), q.sort.as_tmdb().into()),
            ("page".into(), q.page.max(1).to_string()),
        ];
        if !q.genres.is_empty() {
            let separator = if q.genres_or { "|" } else { "," };
            p.push((
                "with_genres".into(),
                q.genres
                    .iter()
                    .map(|g| g.to_string())
                    .collect::<Vec<_>>()
                    .join(separator),
            ));
        }
        if !q.without_genres.is_empty() {
            p.push((
                "without_genres".into(),
                q.without_genres
                    .iter()
                    .map(|g| g.to_string())
                    .collect::<Vec<_>>()
                    .join(","),
            ));
        }
        if !q.keywords.is_empty() {
            let separator = if q.keywords_or { "|" } else { "," };
            p.push((
                "with_keywords".into(),
                q.keywords
                    .iter()
                    .map(|k| k.to_string())
                    .collect::<Vec<_>>()
                    .join(separator),
            ));
        }
        if !q.countries.is_empty() {
            p.push(("with_origin_country".into(), q.countries.join("|")));
        }
        if let Some((a, b)) = q.year {
            p.push(("primary_release_date.gte".into(), format!("{a}-01-01")));
            p.push(("primary_release_date.lte".into(), format!("{b}-12-31")));
        }
        if let Some(r) = q.min_rating {
            p.push(("vote_average.gte".into(), format!("{:.1}", r)));
            // TMDB rifiuta la query se vote_count.gte compare due volte:
            // il filtro esplicito sui voti, se c'e, sostituisce questo minimo.
            if q.min_votes.is_none() {
                p.push(("vote_count.gte".into(), "25".into()));
            }
        }
        if let Some(r) = q.max_rating {
            p.push(("vote_average.lte".into(), format!("{:.1}", r)));
        }
        if let Some(v) = q.min_votes {
            // il filtro sui voti e piu forte di quello implicito del voto minimo
            p.push(("vote_count.gte".into(), v.to_string()));
        } else if q.sort == Sort::Rating && q.min_rating.is_none() {
            // ordinare per voto senza un minimo di voti farebbe emergere titoli
            // con due voti: una soglia minima tiene l'ordinamento credibile.
            p.push(("vote_count.gte".into(), "50".into()));
        }
        if let Some((a, b)) = q.runtime {
            p.push(("with_runtime.gte".into(), a.to_string()));
            p.push(("with_runtime.lte".into(), b.to_string()));
        }
        let key = Self::cache_key("/discover/movie", &p);
        if let Some(hit) = self.list_cached(&key) {
            return Ok(hit);
        }
        let res: PagedMovies = self.get("/discover/movie", &p)?;
        self.list_store(key, &res);
        Ok(res)
    }

    pub fn search(&self, query: &str, page: u32) -> Result<PagedMovies, String> {
        let p = vec![
            ("query".to_string(), query.to_string()),
            ("include_adult".into(), "false".into()),
            ("page".into(), page.to_string()),
        ];
        let key = Self::cache_key("/search/movie", &p);
        if let Some(hit) = self.list_cached(&key) {
            return Ok(hit);
        }
        let res: PagedMovies = self.get("/search/movie", &p)?;
        self.list_store(key, &res);
        Ok(res)
    }

    /// Ricerca per titolo vincolata all'anno di uscita: serve ad agganciare i
    /// titoli delle filmografie (spesso in inglese) alle schede TMDB.
    pub fn search_year(&self, query: &str, year: u16, page: u32) -> Result<PagedMovies, String> {
        let p = vec![
            ("query".to_string(), query.to_string()),
            ("year".to_string(), year.to_string()),
            ("include_adult".into(), "false".into()),
            ("page".into(), page.to_string()),
        ];
        let key = Self::cache_key("/search/movie", &p);
        if let Some(hit) = self.list_cached(&key) {
            return Ok(hit);
        }
        let res: PagedMovies = self.get("/search/movie", &p)?;
        self.list_store(key, &res);
        Ok(res)
    }

    pub fn trending(&self, page: u32) -> Result<PagedMovies, String> {
        let p: Vec<(String, String)> = vec![("page".into(), page.to_string())];
        let key = Self::cache_key("/trending/movie/week", &p);
        if let Some(hit) = self.list_cached(&key) {
            return Ok(hit);
        }
        let res: PagedMovies = self.get("/trending/movie/week", &p)?;
        self.list_store(key, &res);
        Ok(res)
    }

    pub fn similar(&self, id: u64, page: u32) -> Result<PagedMovies, String> {
        let p: Vec<(String, String)> = vec![("page".into(), page.to_string())];
        let key = Self::cache_key(&format!("/movie/{id}/recommendations"), &p);
        if let Some(hit) = self.list_cached(&key) {
            return Ok(hit);
        }
        let res: PagedMovies = self.get(&format!("/movie/{id}/recommendations"), &p)?;
        self.list_store(key, &res);
        Ok(res)
    }

    /// I voti IMDb possono cambiare dopo un aggiornamento: le schede salvate
    /// vanno rilette per non mostrare il valore vecchio.
    pub fn clear_details_cache(&self) {
        if let Ok(mut c) = self.details_cache.lock() {
            c.clear();
        }
    }

    pub fn details(&self, id: u64) -> Result<Details, String> {        if let Ok(c) = self.details_cache.lock() {
            if let Some(hit) = c.get(&id) {
                return Ok(hit.clone());
            }
        }
        let p = vec![(
            "append_to_response".to_string(),
            "credits,watch/providers,recommendations,external_ids".to_string(),
        )];
        let raw: ApiDetails = self.get(&format!("/movie/{id}"), &p)?;
        let details = raw.into_details(&self.region);
        let details = self.attach_imdb(details);
        if let Ok(mut c) = self.details_cache.lock() {
            if c.len() > 200 {
                c.clear();
            }
            c.insert(id, details.clone());
        }
        Ok(details)
    }

    /// Completamento con il voto IMDb, quando i dati pubblici sono stati scaricati
    /// e il film ha un identificativo IMDb.
    fn attach_imdb(&self, mut details: Details) -> Details {
        if let Some(tconst) = details.movie.imdb_id.clone() {
            details.imdb = crate::infra::imdb::lookup(&tconst);
        }
        details
    }

    pub fn keyword_id(&self, keyword: &str) -> Result<Option<u64>, String> {
        let key = keyword.trim().to_lowercase();
        if let Ok(c) = self.keyword_cache.lock() {
            if let Some(hit) = c.get(&key) {
                return Ok(*hit);
            }
        }
        let mut found = None;
        for variant in keyword_variants(&key) {
            let p = vec![("query".to_string(), variant.clone())];
            let res: KeywordResults = self.get("/search/keyword", &p)?;
            if let Some(id) = res.exact(&variant) {
                found = Some(id);
                break;
            }
        }
        if let Ok(mut c) = self.keyword_cache.lock() {
            c.insert(key, found);
        }
        Ok(found)
    }
}

/// Varianti morfologiche minime per le keyword inglesi dell'AI. Non sono un
/// traduttore: servono solo a non perdere «dogs» quando TMDB indicizza «dog».
fn keyword_variants(keyword: &str) -> Vec<String> {
    let mut out = vec![keyword.to_string()];
    let mut add = |s: String| {
        if !s.is_empty() && !out.contains(&s) {
            out.push(s);
        }
    };
    if let Some(stem) = keyword.strip_suffix("ies") {
        add(format!("{stem}y"));
    }
    if let Some(stem) = keyword.strip_suffix("es") {
        add(stem.to_string());
    }
    if let Some(stem) = keyword.strip_suffix('s') {
        if !stem.ends_with('s') {
            add(stem.to_string());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::{keyword_variants, Tmdb, CANCELLED};
    use std::sync::atomic::Ordering;

    #[test]
    fn annullare_la_chat_ferma_la_richiesta() {
        let (tmdb, cancel) = Tmdb::new("x".into(), "it-IT".into()).cancellable();
        cancel.store(true, Ordering::Relaxed);
        assert_eq!(tmdb.ping().unwrap_err(), CANCELLED);
    }

    #[test]
    fn le_varianti_coprono_i_plurali_inglesi() {
        assert!(keyword_variants("dogs").contains(&"dog".to_string()));
        assert!(keyword_variants("stories").contains(&"story".to_string()));
        assert_eq!(keyword_variants("space travel"), ["space travel"]);
        assert_eq!(keyword_variants("friendship"), ["friendship"]);
    }
}
