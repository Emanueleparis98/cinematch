//! Collegamento a SIMKL: accesso, sincronizzazione della libreria film e
//! stato locale. Solo film: CineMatch non tratta serie o anime.

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::infra::config::Config;

pub const API_BASE: &str = "https://api.simkl.com";
/// Client ID pubblico dell'app CineMatch. Non è un segreto: identifica solo
/// l'app nella pagina di approvazione SIMKL.
pub const CLIENT_ID: &str = "9e433f82efc6a6302de302df697962b60ad79c9c934fcc5a50f1b63f3356d691";
const APP_NAME: &str = "cinematch";

fn version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

#[derive(Clone, Debug, Serialize, Deserialize, Default, PartialEq)]
pub struct Tokens {
    pub access_token: String,
    pub refresh_token: String,
    /// Secondi Unix. Il token SIMKL dura 7 giorni.
    pub expires_at: i64,
}

impl Tokens {
    pub fn needs_refresh(&self) -> bool {
        self.expires_at <= now() + 120
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct DeviceCode {
    pub device_code: String,
    pub user_code: String,
    pub verification_uri: String,
    #[serde(default)]
    pub verification_uri_complete: String,
    pub expires_in: u64,
    pub interval: u64,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TrackedMovie {
    pub simkl_id: u64,
    #[serde(default)]
    pub tmdb: u64,
    #[serde(default)]
    pub slug: String,
    /// "completed" | "plantowatch" | "dropped"
    #[serde(default)]
    pub status: String,
    #[serde(default)]
    pub rating: Option<u8>,
    #[serde(default)]
    pub watched_at: Option<String>,
    #[serde(default)]
    pub added_at: Option<String>,
    #[serde(default)]
    pub imdb: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub year: Option<u16>,
}

/// Un film iniziato e messo in pausa su un'altra app.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Playback {
    pub tmdb: u64,
    #[serde(default)]
    pub imdb: Option<String>,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub year: Option<u16>,
    #[serde(default)]
    pub progress: f32,
    #[serde(default)]
    pub paused_at: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Store {
    /// Ultima risposta di `/sync/activities`, necessaria per i delta.
    #[serde(default)]
    pub activities: Option<serde_json::Value>,
    /// Chiave: id TMDB del film.
    #[serde(default)]
    pub movies: HashMap<String, TrackedMovie>,
    /// Film in pausa, per la riga "Riprendi".
    #[serde(default)]
    pub playbacks: Vec<Playback>,
    /// Anche una lista vuota e' un dato: evita di richiederla a ogni sync.
    #[serde(default)]
    pub playbacks_loaded: bool,
    #[serde(default)]
    pub last_sync: i64,
}

impl Store {
    pub fn get(&self, tmdb_id: u64) -> Option<&TrackedMovie> {
        self.movies.get(&tmdb_id.to_string())
    }

    pub fn watched(&self) -> usize {
        self.movies
            .values()
            .filter(|m| m.status == "completed")
            .count()
    }

    pub fn watchlist(&self) -> usize {
        self.movies
            .values()
            .filter(|m| m.status == "plantowatch")
            .count()
    }

    /// Fino a `count` film da vedere, i più recenti prima.
    pub fn watchlist_picks(&self, count: usize) -> Vec<&TrackedMovie> {
        let mut list: Vec<&TrackedMovie> = self
            .movies
            .values()
            .filter(|m| m.status == "plantowatch")
            .collect();
        list.sort_by(|a, b| {
            b.added_at
                .as_deref()
                .unwrap_or("")
                .cmp(a.added_at.as_deref().unwrap_or(""))
                .then_with(|| a.title.cmp(&b.title))
        });
        list.truncate(count);
        list
    }

    /// Fino a `count` film in pausa, i più recenti prima.
    pub fn resume_picks(&self, count: usize) -> Vec<&Playback> {
        let mut list: Vec<&Playback> = self
            .playbacks
            .iter()
            .filter(|p| p.progress < 98.0)
            .collect();
        list.sort_by(|a, b| {
            b.paused_at
                .as_deref()
                .unwrap_or("")
                .cmp(a.paused_at.as_deref().unwrap_or(""))
        });
        list.truncate(count);
        list
    }
}

#[derive(Clone, Debug, Default)]
pub struct SyncReport {
    pub watched: usize,
    pub watchlist: usize,
    pub last_sync: i64,
}

#[derive(Debug, PartialEq)]
pub enum Poll {
    Pending,
    SlowDown,
    Expired,
    Tokens(Tokens),
}

// ------------------------------------------------------------------ store -----

fn store_path() -> std::path::PathBuf {
    Config::dir().join("simkl.json")
}

pub fn load_store() -> Store {
    let path = store_path();
    let mut store: Store = std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())
        .unwrap_or_default();
    // Le cache scritte prima della 0.4.3 non hanno il TMDB dentro la voce:
    // la chiave della mappa lo contiene gia'.
    for (key, movie) in store.movies.iter_mut() {
        if movie.tmdb == 0 {
            if let Ok(id) = key.parse() {
                movie.tmdb = id;
            }
        }
    }
    store
}

pub fn save_store(store: &Store) -> Result<(), String> {
    let dir = Config::dir();
    std::fs::create_dir_all(&dir).map_err(|e| format!("Cartella dati non creabile: {e}"))?;
    let path = store_path();
    let tmp = path.with_extension("json.tmp");
    let raw = serde_json::to_string(store).map_err(|e| e.to_string())?;
    std::fs::write(&tmp, raw).map_err(|e| format!("Salvataggio SIMKL non riuscito: {e}"))?;
    std::fs::rename(&tmp, &path).map_err(|e| format!("Salvataggio SIMKL non riuscito: {e}"))
}

// ---------------------------------------------------------------- tokens -----

#[cfg(target_os = "windows")]
fn token_entry() -> Result<keyring::Entry, String> {
    keyring::Entry::new("CineMatch", "simkl").map_err(|e| format!("Credenziale non accessibile: {e}"))
}

pub fn load_tokens() -> Result<Option<Tokens>, String> {
    #[cfg(target_os = "windows")]
    {
        let entry = token_entry()?;
        match entry.get_password() {
            Ok(raw) => Ok(serde_json::from_str(&raw).ok()),
            Err(keyring::Error::NoEntry) => Ok(None),
            Err(e) => Err(format!("Credenziale SIMKL non leggibile: {e}")),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(None)
    }
}

pub fn save_tokens(tokens: &Tokens) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        let raw = serde_json::to_string(tokens).map_err(|e| e.to_string())?;
        token_entry()?
            .set_password(&raw)
            .map_err(|e| format!("Credenziale SIMKL non salvata: {e}"))
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = tokens;
        Err("Credenziali SIMKL non supportate su questa piattaforma.".into())
    }
}

pub fn clear_tokens() -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        match token_entry()?.delete_credential() {
            Ok(()) => Ok(()),
            Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Credenziale SIMKL non rimossa: {e}")),
        }
    }
    #[cfg(not(target_os = "windows"))]
    {
        Ok(())
    }
}

// ------------------------------------------------------------------ http -----

fn http() -> reqwest::blocking::Client {
    reqwest::blocking::Client::builder()
        .timeout(std::time::Duration::from_secs(25))
        .user_agent(format!("CineMatch/{} (Windows)", version()))
        .build()
        .unwrap_or_else(|_| reqwest::blocking::Client::new())
}

fn url(base: &str, path: &str, client_id: &str) -> String {
    format!(
        "{}{path}{}client_id={}&app-name={}&app-version={}",
        base.trim_end_matches('/'),
        if path.contains('?') { '&' } else { '?' },
        urlencode(client_id),
        APP_NAME,
        version()
    )
}

fn urlencode(raw: &str) -> String {
    let mut out = String::new();
    for byte in raw.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

fn error_text(status: u16, body: &str) -> String {
    let snippet: String = body.chars().take(180).collect();
    match status {
        401 => "SIMKL ha rifiutato il token: ricollega l'account.".into(),
        412 => "Client ID SIMKL non valido. Controlla le impostazioni dell'app.".into(),
        429 => "SIMKL ha chiesto di rallentare: riprova tra poco.".into(),
        _ => format!("SIMKL ha risposto {status}: {snippet}"),
    }
}

// -------------------------------------------------------------- auth flow -----

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    refresh_token: String,
    expires_in: i64,
}

fn tokens_from(t: TokenResponse) -> Tokens {
    Tokens {
        access_token: t.access_token,
        refresh_token: t.refresh_token,
        expires_at: now() + t.expires_in.max(60) - 60,
    }
}

pub fn device_start(client_id: &str) -> Result<DeviceCode, String> {
    device_start_at(API_BASE, client_id)
}

fn device_start_at(base: &str, client_id: &str) -> Result<DeviceCode, String> {
    let resp = http()
        .post(format!("{}/oauth2/device", base.trim_end_matches('/')))
        .form(&[
            ("client_id", client_id.trim()),
            ("scope", "media:read media:write"),
        ])
        .send()
        .map_err(|e| format!("SIMKL non raggiungibile: {e}"))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        if matches!(status, 401 | 403 | 412) {
            return Err(
                "Client ID SIMKL non valido o app non abilitata. Controlla la registrazione su simkl.com/settings/developer."
                    .into(),
            );
        }
        return Err(error_text(status, &body));
    }
    serde_json::from_str(&body).map_err(|e| format!("Risposta SIMKL non leggibile: {e}"))
}

pub fn device_poll(client_id: &str, device_code: &str) -> Result<Poll, String> {
    device_poll_at(API_BASE, client_id, device_code)
}

fn device_poll_at(base: &str, client_id: &str, device_code: &str) -> Result<Poll, String> {
    let resp = http()
        .post(format!("{}/oauth2/token", base.trim_end_matches('/')))
        .form(&[
            ("grant_type", "urn:ietf:params:oauth:grant-type:device_code"),
            ("client_id", client_id.trim()),
            ("device_code", device_code),
        ])
        .send()
        .map_err(|e| format!("SIMKL non raggiungibile: {e}"))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if (200..300).contains(&status) {
        let t: TokenResponse =
            serde_json::from_str(&body).map_err(|e| format!("Token SIMKL non leggibile: {e}"))?;
        return Ok(Poll::Tokens(tokens_from(t)));
    }
    if let Ok(v) = serde_json::from_str::<serde_json::Value>(&body) {
        match v.get("error").and_then(|e| e.as_str()).unwrap_or("") {
            "authorization_pending" => return Ok(Poll::Pending),
            "slow_down" => return Ok(Poll::SlowDown),
            "expired_token" => return Ok(Poll::Expired),
            _ => {}
        }
    }
    Err(error_text(status, &body))
}

pub fn refresh(client_id: &str, refresh_token: &str) -> Result<Tokens, String> {
    refresh_at(API_BASE, client_id, refresh_token)
}

fn refresh_at(base: &str, client_id: &str, refresh_token: &str) -> Result<Tokens, String> {
    let resp = http()
        .post(format!("{}/oauth2/token", base.trim_end_matches('/')))
        .form(&[
            ("grant_type", "refresh_token"),
            ("client_id", client_id.trim()),
            ("refresh_token", refresh_token),
        ])
        .send()
        .map_err(|e| format!("SIMKL non raggiungibile: {e}"))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_text(status, &body));
    }
    let t: TokenResponse =
        serde_json::from_str(&body).map_err(|e| format!("Token SIMKL non leggibile: {e}"))?;
    Ok(tokens_from(t))
}

// ------------------------------------------------------------------ sync -----

fn get_json(
    base: &str,
    path: &str,
    client_id: &str,
    token: &str,
) -> Result<serde_json::Value, String> {
    let resp = http()
        .get(url(base, path, client_id))
        .bearer_auth(token)
        .send()
        .map_err(|e| format!("SIMKL non raggiungibile: {e}"))?;
    let status = resp.status().as_u16();
    let body = resp.text().unwrap_or_default();
    if !(200..300).contains(&status) {
        return Err(error_text(status, &body));
    }
    serde_json::from_str(&body).map_err(|e| format!("Risposta SIMKL non leggibile: {e}"))
}

fn id_string(v: Option<&serde_json::Value>) -> Option<String> {
    match v? {
        serde_json::Value::String(s) if !s.trim().is_empty() => Some(s.clone()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn tracked_from(item: &serde_json::Value) -> Option<(String, TrackedMovie)> {
    let movie = item.get("movie")?;
    let ids = movie.get("ids")?;
    let tmdb = id_string(ids.get("tmdb"))?;
    let tmdb_num = tmdb.parse::<u64>().ok()?;
    let simkl_id = ids
        .get("simkl")
        .and_then(|v| v.as_u64().or_else(|| v.as_str()?.parse().ok()))?;
    let rating = item
        .get("user_rating")
        .and_then(|v| v.as_u64())
        .map(|v| v.clamp(1, 10) as u8);
    Some((
        tmdb,
        TrackedMovie {
            simkl_id,
            tmdb: tmdb_num,
            slug: ids
                .get("slug")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            status: item
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            rating,
            watched_at: item
                .get("last_watched_at")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            added_at: item
                .get("added_to_watchlist_at")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            imdb: ids
                .get("imdb")
                .and_then(|v| v.as_str())
                .map(str::to_string),
            title: movie
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            year: movie
                .get("year")
                .and_then(|v| v.as_u64())
                .map(|v| v as u16),
        },
    ))
}

fn merge_items(store: &mut Store, value: &serde_json::Value) {
    if let Some(items) = value.get("movies").and_then(|v| v.as_array()) {
        for item in items {
            if let Some((tmdb, tracked)) = tracked_from(item) {
                store.movies.insert(tmdb, tracked);
            }
        }
    }
}

fn playback_from(item: &serde_json::Value) -> Option<Playback> {
    let movie = item.get("movie")?;
    let ids = movie.get("ids")?;
    let tmdb = id_string(ids.get("tmdb"))?.parse::<u64>().ok()?;
    Some(Playback {
        tmdb,
        imdb: ids
            .get("imdb")
            .and_then(|v| v.as_str())
            .map(str::to_string),
        title: movie
            .get("title")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string(),
        year: movie
            .get("year")
            .and_then(|v| v.as_u64())
            .map(|v| v as u16),
        progress: item
            .get("progress")
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0) as f32,
        paused_at: item
            .get("paused_at")
            .or_else(|| item.get("watched_at"))
            .and_then(|v| v.as_str())
            .map(str::to_string),
    })
}

fn parse_playbacks(value: &serde_json::Value) -> Vec<Playback> {
    let items = value
        .as_array()
        .or_else(|| value.get("movies").and_then(|v| v.as_array()))
        .or_else(|| value.get("playback").and_then(|v| v.as_array()));
    items
        .map(|items| items.iter().filter_map(playback_from).collect())
        .unwrap_or_default()
}

/// Sincronizza la libreria film. Ritorna lo store aggiornato e i token
/// eventualmente rinnovati. Non salva nulla: lo fa il chiamante solo dopo
/// che ogni richiesta e' andata a buon fine.
pub fn sync(
    client_id: &str,
    tokens: Tokens,
    store: Store,
    force_full: bool,
) -> Result<(Store, Tokens, SyncReport), String> {
    sync_at(API_BASE, client_id, tokens, store, force_full)
}

fn sync_at(
    base: &str,
    client_id: &str,
    mut tokens: Tokens,
    mut store: Store,
    force_full: bool,
) -> Result<(Store, Tokens, SyncReport), String> {
    if tokens.needs_refresh() {
        tokens = refresh_at(base, client_id, &tokens.refresh_token)?;
    }

    let old = store.activities.clone();
    let activities = get_json(base, "/sync/activities", client_id, &tokens.access_token)?;
    let new_all = activities
        .get("all")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let old_all = store
        .activities
        .as_ref()
        .and_then(|a| a.get("all"))
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let first = force_full || store.activities.is_none();

    if first {
        let value = get_json(
            base,
            "/sync/all-items/movies",
            client_id,
            &tokens.access_token,
        )?;
        store.movies.clear();
        merge_items(&mut store, &value);
    } else if new_all != old_all {
        let delta = get_json(
            base,
            &format!("/sync/all-items/movies?date_from={}", urlencode(&old_all)),
            client_id,
            &tokens.access_token,
        )?;
        merge_items(&mut store, &delta);
    }

    let removed_moved = old
        .as_ref()
        .map(|a| {
            a.get("movies")
                .and_then(|m| m.get("removed_from_list"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or(None)
        != activities
            .get("movies")
            .and_then(|m| m.get("removed_from_list"))
            .and_then(|v| v.as_str());

    if !first && removed_moved {
        let ids = get_json(
            base,
            "/sync/all-items/movies?extended=simkl_ids_only",
            client_id,
            &tokens.access_token,
        )?;
        let mut alive = std::collections::HashSet::new();
        if let Some(items) = ids.get("movies").and_then(|v| v.as_array()) {
            for item in items {
                if let Some(id) = item
                    .get("movie")
                    .and_then(|m| m.get("ids"))
                    .and_then(|i| i.get("simkl"))
                    .and_then(|v| v.as_u64())
                {
                    alive.insert(id);
                }
            }
        }
        store.movies.retain(|_, m| alive.contains(&m.simkl_id));
    }

    let rated_moved = old
        .as_ref()
        .map(|a| {
            a.get("movies")
                .and_then(|m| m.get("rated_at"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or(None)
        != activities
            .get("movies")
            .and_then(|m| m.get("rated_at"))
            .and_then(|v| v.as_str());

    if !first && !old_all.is_empty() && rated_moved {
        let ratings = get_json(
            base,
            &format!("/sync/ratings/movies?date_from={}", urlencode(&old_all)),
            client_id,
            &tokens.access_token,
        )?;
        merge_items(&mut store, &ratings);
    }

    let playback_moved = old
        .as_ref()
        .map(|a| {
            a.get("movies")
                .and_then(|m| m.get("playback"))
                .and_then(|v| v.as_str())
        })
        .unwrap_or(None)
        != activities
            .get("movies")
            .and_then(|m| m.get("playback"))
            .and_then(|v| v.as_str());
    if first || playback_moved || !store.playbacks_loaded {
        let playbacks = get_json(
            base,
            "/sync/playback/movies",
            client_id,
            &tokens.access_token,
        )?;
        store.playbacks = parse_playbacks(&playbacks);
        store.playbacks_loaded = true;
    }

    store.activities = Some(activities);
    store.last_sync = now();

    let report = SyncReport {
        watched: store.watched(),
        watchlist: store.watchlist(),
        last_sync: store.last_sync,
    };
    Ok((store, tokens, report))
}

/// Verifica che il voto salvato appartenga ancora a un item della libreria.
pub fn rating_of(store: &Store, tmdb_id: u64) -> Option<u8> {
    store.get(tmdb_id).and_then(|m| m.rating)
}

pub fn status_of(store: &Store, tmdb_id: u64) -> Option<&str> {
    store.get(tmdb_id).map(|m| m.status.as_str())
}

pub fn link_to(tracked: &TrackedMovie) -> String {
    format!(
        "https://simkl.com/movies/{}/{}",
        tracked.simkl_id, tracked.slug
    )
}

/// Ricerca SIMKL per i film non ancora tracciati.
pub fn search_url(title: &str) -> String {
    format!("https://simkl.com/search/?q={}", urlencode(title))
}

/// Un film abbastanza completo per una scrittura su SIMKL.
#[derive(Clone, Debug)]
pub struct WriteMovie {
    pub tmdb: u64,
    pub imdb: Option<String>,
    pub title: String,
    pub year: Option<u16>,
}

impl WriteMovie {
    pub fn from_movie(movie: &crate::domain::model::Movie) -> Self {
        Self {
            tmdb: movie.id,
            imdb: movie.imdb_id.clone(),
            title: movie.title.clone(),
            year: movie.year,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WriteOp {
    Watched,
    Watchlist,
    Remove,
    Rate(u8),
    ClearRating,
}

#[derive(Clone, Debug, Default)]
pub struct WriteOutcome {
    pub status: Option<String>,
}

fn movie_json(movie: &WriteMovie) -> serde_json::Value {
    let mut ids = serde_json::Map::new();
    ids.insert("tmdb".into(), serde_json::json!(movie.tmdb.to_string()));
    if let Some(imdb) = &movie.imdb {
        if !imdb.trim().is_empty() {
            ids.insert("imdb".into(), serde_json::json!(imdb));
        }
    }
    let mut item = serde_json::json!({ "ids": ids });
    if !movie.title.trim().is_empty() {
        item["title"] = serde_json::json!(movie.title);
    }
    if let Some(year) = movie.year {
        item["year"] = serde_json::json!(year);
    }
    item
}

fn not_found(response: &serde_json::Value) -> bool {
    response
        .get("not_found")
        .and_then(|n| n.get("movies"))
        .and_then(|m| m.as_array())
        .map(|items| !items.is_empty())
        .unwrap_or(false)
}

fn response_status(response: &serde_json::Value) -> Option<String> {
    response
        .get("added")?
        .get("statuses")?
        .as_array()?
        .first()?
        .get("response")?
        .get("status")?
        .as_str()
        .map(str::to_string)
}

fn post_json(
    base: &str,
    path: &str,
    client_id: &str,
    token: &str,
    body: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let resp = http()
        .post(url(base, path, client_id))
        .bearer_auth(token)
        .json(body)
        .send()
        .map_err(|e| format!("SIMKL non raggiungibile: {e}"))?;
    let status = resp.status().as_u16();
    let text = resp.text().unwrap_or_default();
    if (200..300).contains(&status) {
        return serde_json::from_str(&text)
            .map_err(|e| format!("Risposta SIMKL non leggibile: {e}"));
    }
    if status == 400 && text.contains("rate_limit") {
        return Err("SIMKL ha chiesto di rallentare: riprova tra poco.".into());
    }
    Err(error_text(status, &text))
}

pub fn write(
    client_id: &str,
    tokens: Tokens,
    movie: &WriteMovie,
    op: WriteOp,
) -> Result<(Tokens, WriteOutcome), String> {
    write_at(API_BASE, client_id, tokens, movie, op)
}

fn write_at(
    base: &str,
    client_id: &str,
    mut tokens: Tokens,
    movie: &WriteMovie,
    op: WriteOp,
) -> Result<(Tokens, WriteOutcome), String> {
    if tokens.needs_refresh() {
        tokens = refresh_at(base, client_id, &tokens.refresh_token)?;
    }
    if let WriteOp::Rate(rating) = op {
        if !(1..=10).contains(&rating) {
            return Err("Il voto deve essere tra 1 e 10.".into());
        }
    }
    let mut item = movie_json(movie);
    let (path, body) = match op {
        WriteOp::Watched => (
            "/sync/history",
            serde_json::json!({ "movies": [item] }),
        ),
        WriteOp::Watchlist => {
            item["to"] = serde_json::json!("plantowatch");
            (
                "/sync/add-to-list",
                serde_json::json!({ "movies": [item] }),
            )
        }
        WriteOp::Remove => (
            "/sync/history/remove",
            serde_json::json!({ "movies": [item] }),
        ),
        WriteOp::Rate(rating) => {
            item["rating"] = serde_json::json!(rating);
            (
                "/sync/ratings",
                serde_json::json!({ "movies": [item] }),
            )
        }
        WriteOp::ClearRating => (
            "/sync/ratings/remove",
            serde_json::json!({ "movies": [item] }),
        ),
    };

    let mut retried = false;
    loop {
        match post_json(base, path, client_id, &tokens.access_token, &body) {
            Ok(response) => {
                if not_found(&response) {
                    return Err("SIMKL non ha riconosciuto questo film.".into());
                }
                let status = response_status(&response);
                let outcome = WriteOutcome {
                    status: match op {
                        WriteOp::Watched => Some(status.unwrap_or_else(|| "completed".into())),
                        WriteOp::Watchlist => Some("plantowatch".into()),
                        WriteOp::Remove | WriteOp::ClearRating => None,
                        WriteOp::Rate(_) => status,
                    },
                };
                return Ok((tokens, outcome));
            }
            Err(e) if !retried && e.contains("rallentare") => {
                // SIMKL serializza le scritture per 20 secondi: un solo
                // tentativo dopo una pausa, poi l'errore arriva all'utente.
                retried = true;
                std::thread::sleep(std::time::Duration::from_secs(3));
            }
            Err(e) => return Err(e),
        }
    }
}

/// La sincronizzazione automatica parte all'avvio ma non più spesso di una
/// volta ogni 15 minuti, come chiede SIMKL.
pub fn sync_due(last_sync: i64) -> bool {
    last_sync == 0 || now() - last_sync >= 15 * 60
}

pub fn now_secs() -> i64 {
    now()
}

/// Etichetta breve per la scheda.
pub fn status_label(status: &str) -> &'static str {
    match status {
        "completed" => "VISTO",
        "plantowatch" => "DA VEDERE",
        "dropped" => "SCARTATO",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::sync::{Arc, Mutex};

    use super::*;

    fn sample() -> serde_json::Value {
        serde_json::json!({
            "movies": [
                {
                    "status": "completed",
                    "last_watched_at": "2026-01-02T20:00:00Z",
                    "user_rating": 9,
                    "movie": {
                        "title": "Il buono, il brutto, il cattivo",
                        "year": 1966,
                        "ids": {
                            "simkl": 123,
                            "slug": "the-good-the-bad-and-the-ugly",
                            "imdb": "tt0060196",
                            "tmdb": "429"
                        }
                    }
                },
                {
                    "status": "plantowatch",
                    "user_rating": null,
                    "movie": {
                        "title": "C'era una volta il West",
                        "year": 1968,
                        "ids": { "simkl": 456, "slug": "once-upon-a-time-in-the-west", "tmdb": 335 }
                    }
                }
            ]
        })
    }

    fn mock_server(
        responses: Vec<(u16, String)>,
    ) -> (String, Arc<Mutex<Vec<String>>>, std::thread::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").expect("porta libera");
        let addr = listener.local_addr().expect("indirizzo");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let seen_worker = seen.clone();
        let handle = std::thread::spawn(move || {
            for (status, body) in responses {
                let (mut stream, _) = listener.accept().expect("richiesta");
                let mut raw = Vec::new();
                let mut buf = [0u8; 2048];
                let mut header_end: Option<usize> = None;
                let mut content_len = 0usize;
                loop {
                    let n = match stream.read(&mut buf) {
                        Ok(0) | Err(_) => break,
                        Ok(n) => n,
                    };
                    raw.extend_from_slice(&buf[..n]);
                    if header_end.is_none() {
                        if let Some(pos) = raw.windows(4).position(|w| w == b"\r\n\r\n") {
                            header_end = Some(pos + 4);
                            let head = String::from_utf8_lossy(&raw[..pos]);
                            content_len = head
                                .lines()
                                .find_map(|line| {
                                    let (key, value) = line.split_once(':')?;
                                    if key.eq_ignore_ascii_case("content-length") {
                                        value.trim().parse().ok()
                                    } else {
                                        None
                                    }
                                })
                                .unwrap_or(0);
                        }
                    }
                    if let Some(end) = header_end {
                        if raw.len() >= end + content_len {
                            break;
                        }
                    }
                }
                seen_worker
                    .lock()
                    .expect("registro")
                    .push(String::from_utf8_lossy(&raw).to_string());
                let reason = match status {
                    200 => "OK",
                    201 => "Created",
                    400 => "Bad Request",
                    401 => "Unauthorized",
                    _ => "Error",
                };
                let response = format!(
                    "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes());
            }
        });
        (format!("http://{addr}"), seen, handle)
    }

    #[test]
    fn legge_libreria_e_voti() {
        let mut store = Store::default();
        merge_items(&mut store, &sample());
        assert_eq!(store.movies.len(), 2);
        assert_eq!(store.watched(), 1);
        assert_eq!(store.watchlist(), 1);
        let good = store.get(429).expect("film TMDB 429");
        assert_eq!(good.status, "completed");
        assert_eq!(good.rating, Some(9));
        assert_eq!(good.simkl_id, 123);
        assert!(link_to(good).contains("/movies/123/"));
    }

    #[test]
    fn la_ricerca_simkl_codifica_il_titolo() {
        assert_eq!(
            search_url("Bad Apples"),
            "https://simkl.com/search/?q=Bad%20Apples"
        );
        assert_eq!(
            search_url("L'isola degli è"),
            "https://simkl.com/search/?q=L%27isola%20degli%20%C3%A8"
        );
    }

    #[test]
    fn un_delta_aggiorna_solo_quel_film() {
        let mut store = Store::default();
        merge_items(&mut store, &sample());
        let delta = serde_json::json!({
            "movies": [{
                "status": "dropped",
                "user_rating": 4,
                "movie": { "title": "X", "year": 2000, "ids": { "simkl": 999, "tmdb": "77" } }
            }]
        });
        merge_items(&mut store, &delta);
        assert_eq!(store.movies.len(), 3);
        assert_eq!(store.get(77).unwrap().status, "dropped");
    }

    #[test]
    fn un_voto_fuori_scala_viene_riportato_nel_range() {
        let mut item = sample()["movies"][0].clone();
        item["user_rating"] = serde_json::json!(14);
        let (_, tracked) = tracked_from(&item).expect("item");
        assert_eq!(tracked.rating, Some(10));
    }

    #[test]
    fn il_token_scaduto_va_rinnovato() {
        let mut t = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 30,
        };
        assert!(t.needs_refresh());
        t.expires_at = now() + 3600;
        assert!(!t.needs_refresh());
    }

    #[test]
    fn le_etichette_sono_le_tre_dei_film() {
        assert_eq!(status_label("completed"), "VISTO");
        assert_eq!(status_label("plantowatch"), "DA VEDERE");
        assert_eq!(status_label("dropped"), "SCARTATO");
        assert_eq!(status_label("watching"), "");
    }

    #[test]
    fn avvia_il_codice_dispositivo() {
        let body = r#"{"device_code":"D","user_code":"ABCD-EFGH","verification_uri":"https://simkl.com/pin","verification_uri_complete":"https://simkl.com/pin?user_code=ABCD-EFGH","expires_in":900,"interval":5}"#;
        let (base, seen, handle) = mock_server(vec![(200, body.to_string())]);
        let code = device_start_at(&base, "client 1").expect("codice");
        handle.join().expect("server");
        assert_eq!(code.user_code, "ABCD-EFGH");
        let raw = seen.lock().unwrap().join("\n");
        assert!(raw.starts_with("POST /oauth2/device HTTP/1.1"));
        assert!(raw
            .to_lowercase()
            .contains("content-type: application/x-www-form-urlencoded"));
        assert!(raw.contains("client_id=client+1"));
        assert!(raw.contains("scope=media%3Aread+media%3Awrite"));
    }

    #[test]
    fn il_polling_restituisce_i_token() {
        let body = r#"{"access_token":"a","refresh_token":"r","expires_in":1000}"#;
        let (base, _seen, handle) = mock_server(vec![(200, body.to_string())]);
        let poll = device_poll_at(&base, "client", "device").expect("poll");
        handle.join().expect("server");
        match poll {
            Poll::Tokens(t) => assert_eq!(t.access_token, "a"),
            other => panic!("atteso Tokens, trovato {other:?}"),
        }
    }

    #[test]
    fn la_prima_sincronizzazione_legge_attivita_e_film() {
        let activities = serde_json::json!({
            "all": "2026-01-01T00:00:00Z",
            "movies": {
                "all": "2026-01-01T00:00:00Z",
                "completed": "2026-01-01T00:00:00Z",
                "plantowatch": "2026-01-01T00:00:00Z",
                "dropped": null,
                "rated_at": null,
                "removed_from_list": null
            }
        });
        let playbacks = serde_json::json!([{
            "progress": 42.5,
            "paused_at": "2026-01-03T20:00:00Z",
            "movie": {
                "title": "Il buono, il brutto, il cattivo",
                "year": 1966,
                "ids": { "simkl": 123, "tmdb": "429", "imdb": "tt0060196" }
            }
        }]);
        let (base, seen, handle) = mock_server(vec![
            (200, activities.to_string()),
            (200, sample().to_string()),
            (200, playbacks.to_string()),
        ]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        let (store, tokens2, report) = sync_at(
            &base,
            "client",
            tokens.clone(),
            Store::default(),
            false,
        )
        .expect("sync");
        handle.join().expect("server");
        assert_eq!(tokens2, tokens);
        assert_eq!(report.watched, 1);
        assert_eq!(report.watchlist, 1);
        assert_eq!(store.movies.len(), 2);
        assert_eq!(store.playbacks.len(), 1);
        assert!((store.playbacks[0].progress - 42.5).abs() < 0.01);
        assert_eq!(store.playbacks[0].tmdb, 429);
        assert_eq!(
            store.activities.as_ref().unwrap()["all"],
            "2026-01-01T00:00:00Z"
        );
        assert!(store.last_sync > 0);
        let raw = seen.lock().unwrap().join("\n").to_lowercase();
        assert!(raw.contains("get /sync/activities?client_id=client&app-name=cinematch"));
        assert!(raw.contains("get /sync/all-items/movies?client_id=client"));
        assert!(raw.contains("authorization: bearer a"));
    }

    fn write_movie() -> WriteMovie {
        WriteMovie {
            tmdb: 429,
            imdb: Some("tt0060196".into()),
            title: "Il buono, il brutto, il cattivo".into(),
            year: Some(1966),
        }
    }

    #[test]
    fn segna_visto_con_gli_id_giusti() {
        let body = r#"{"added":{"movies":1,"statuses":[{"response":{"status":"completed"}}]},"not_found":{"movies":[]}}"#;
        let (base, seen, handle) = mock_server(vec![(201, body.to_string())]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        let (out_tokens, outcome) =
            write_at(&base, "client", tokens.clone(), &write_movie(), WriteOp::Watched)
                .expect("scrittura");
        handle.join().expect("server");
        assert_eq!(out_tokens, tokens);
        assert_eq!(outcome.status.as_deref(), Some("completed"));
        let raw = seen.lock().unwrap().join("\n").to_lowercase();
        assert!(raw.starts_with("post /sync/history?client_id=client"));
        assert!(raw.contains("\"tmdb\":\"429\""));
        assert!(raw.contains("\"imdb\":\"tt0060196\""));
        assert!(raw.contains("authorization: bearer a"));
    }

    #[test]
    fn aggiunge_a_da_vedere() {
        let body = r#"{"added":{"movies":[{"to":"plantowatch"}]},"not_found":{"movies":[]}}"#;
        let (base, seen, handle) = mock_server(vec![(201, body.to_string())]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        let (_, outcome) = write_at(
            &base,
            "client",
            tokens,
            &write_movie(),
            WriteOp::Watchlist,
        )
        .expect("watchlist");
        handle.join().expect("server");
        assert_eq!(outcome.status.as_deref(), Some("plantowatch"));
        let raw = seen.lock().unwrap().join("\n");
        assert!(raw.starts_with("POST /sync/add-to-list?client_id=client"));
        assert!(raw.contains("\"to\":\"plantowatch\""));
    }

    #[test]
    fn vota_e_toglie_il_voto() {
        let body = r#"{"added":{"movies":1,"statuses":[{"response":{"status":"completed"}}]},"not_found":{"movies":[]}}"#;
        let (base, seen, handle) = mock_server(vec![(201, body.to_string())]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        let (_, outcome) = write_at(
            &base,
            "client",
            tokens,
            &write_movie(),
            WriteOp::Rate(9),
        )
        .expect("voto");
        handle.join().expect("server");
        assert_eq!(outcome.status.as_deref(), Some("completed"));
        let raw = seen.lock().unwrap().join("\n");
        assert!(raw.starts_with("POST /sync/ratings?client_id=client"));
        assert!(raw.contains("\"rating\":9"));

        let body = r#"{"deleted":{"movies":1},"not_found":{"movies":[]}}"#;
        let (base, seen, handle) = mock_server(vec![(201, body.to_string())]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        write_at(
            &base,
            "client",
            tokens,
            &write_movie(),
            WriteOp::ClearRating,
        )
        .expect("togli voto");
        handle.join().expect("server");
        let raw = seen.lock().unwrap().join("\n");
        assert!(raw.starts_with("POST /sync/ratings/remove?client_id=client"));
    }

    #[test]
    fn rimuove_dalla_libreria_e_rifiuta_i_film_sconosciuti() {
        let body = r#"{"deleted":{"movies":1},"not_found":{"movies":[]}}"#;
        let (base, seen, handle) = mock_server(vec![(201, body.to_string())]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        write_at(&base, "client", tokens, &write_movie(), WriteOp::Remove)
            .expect("rimozione");
        handle.join().expect("server");
        assert!(seen
            .lock()
            .unwrap()
            .join("\n")
            .starts_with("POST /sync/history/remove?client_id=client"));

        let body = r#"{"added":{"movies":0},"not_found":{"movies":[{"ids":{"tmdb":"1"}}]}}"#;
        let (base, _seen, handle) = mock_server(vec![(201, body.to_string())]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        let err = write_at(&base, "client", tokens, &write_movie(), WriteOp::Watched)
            .unwrap_err();
        handle.join().expect("server");
        assert!(err.contains("non ha riconosciuto"));
    }

    #[test]
    fn la_lista_da_vedere_parte_dalla_piu_recente() {
        let movie = |title: &str, status: &str, added_at: &str| TrackedMovie {
            simkl_id: 1,
            tmdb: 1,
            slug: String::new(),
            status: status.into(),
            rating: None,
            watched_at: None,
            added_at: Some(added_at.into()),
            imdb: None,
            title: title.into(),
            year: None,
        };
        let mut store = Store::default();
        store
            .movies
            .insert("1".into(), movie("vecchio", "plantowatch", "2026-01-01T00:00:00Z"));
        store.movies.insert(
            "2".into(),
            movie("nuovo", "plantowatch", "2026-02-01T00:00:00Z"),
        );
        store
            .movies
            .insert("3".into(), movie("visto", "completed", "2026-03-01T00:00:00Z"));
        let picks = store.watchlist_picks(4);
        assert_eq!(picks.len(), 2);
        assert_eq!(picks[0].title, "nuovo");
        assert_eq!(picks[1].title, "vecchio");
    }

    #[test]
    fn il_delta_rimuove_i_film_spariti_e_aggiorna_i_voti() {
        let mut store = Store::default();
        store.activities = Some(serde_json::json!({
            "all": "2026-01-01T00:00:00Z",
            "movies": { "removed_from_list": "r1", "rated_at": "q1" }
        }));
        let tracked = |simkl_id: u64| TrackedMovie {
            simkl_id,
            tmdb: 0,
            slug: String::new(),
            status: "completed".into(),
            rating: None,
            watched_at: None,
            added_at: None,
            imdb: None,
            title: String::new(),
            year: None,
        };
        store.movies.insert("429".into(), tracked(123));
        store.movies.insert("335".into(), tracked(456));
        let activities = serde_json::json!({
            "all": "2026-01-02T00:00:00Z",
            "movies": { "removed_from_list": "r2", "rated_at": "q2" }
        });
        let ids = serde_json::json!({
            "movies": [{ "movie": { "ids": { "simkl": 123 } } }]
        });
        let ratings = serde_json::json!({
            "movies": [{
                "status": "completed",
                "user_rating": 10,
                "movie": { "title": "Il buono", "year": 1966, "ids": { "simkl": 123, "tmdb": "429" } }
            }]
        });
        let (base, _seen, handle) = mock_server(vec![
            (200, activities.to_string()),
            (200, "{}".into()),
            (200, ids.to_string()),
            (200, ratings.to_string()),
            (200, "[]".into()),
        ]);
        let tokens = Tokens {
            access_token: "a".into(),
            refresh_token: "r".into(),
            expires_at: now() + 3600,
        };
        let (store, _, report) =
            sync_at(&base, "client", tokens, store, false).expect("delta");
        handle.join().expect("server");
        assert_eq!(store.movies.len(), 1);
        assert_eq!(store.get(429).unwrap().rating, Some(10));
        assert_eq!(report.watched, 1);
    }
}
