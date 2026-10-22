use std::collections::HashSet;
use std::io::BufRead;
use std::sync::Arc;
use std::time::Duration;

use reqwest::blocking::Client;
use serde::Deserialize;

use crate::domain::model::{toggle, Facets, Sort};
use crate::domain::taxonomy::Taxonomy;
use crate::infra::config::AiCfg;

pub mod curator;

/// Interpretazione di una richiesta in linguaggio naturale.
#[derive(Clone, Debug, Default)]
pub struct Intent {
    pub facets: Facets,
    pub free_text: Option<String>,
    pub similar_to: Option<String>,
    /// Modalita dichiarata dall'interprete: «faceted», «title», «similar»,
    /// «mixed» oppure «reset» (in chat: ricominciare da zero).
    pub mode: String,
    pub notes: Vec<String>,
    pub source: String,
}

pub trait IntentProvider: Send + Sync {
    fn label(&self) -> String;
    fn is_llm(&self) -> bool {
        false
    }
    fn parse(&self, text: &str, tax: &Taxonomy) -> Result<Intent, String>;
    fn summarize(&self, _text: &str) -> Result<String, String> {
        Err("Sintesi AI non disponibile con il parser a regole.".into())
    }
    /// Interpreta un messaggio di chat: come `parse`, ma con la memoria dei
    /// messaggi recenti e i filtri attivi, per capire i seguiti («no, senza
    /// commedia», «piu recenti»). Il default ignora la memoria.
    fn parse_chat(
        &self,
        text: &str,
        tax: &Taxonomy,
        _history: &[(String, String)],
        _active: &Facets,
    ) -> Result<Intent, String> {
        self.parse(text, tax)
    }
    /// Curatela della risposta: sceglie fino a 5 film fra i candidati reali e
    /// scrive intro e commenti; `on_delta` riceve il testo dell'intro man mano
    /// che arriva (effetto macchina da scrivere). Il default e deterministico:
    /// la chat funziona anche senza AI.
    fn curate(
        &self,
        req: &curator::Request,
        on_delta: &mut dyn FnMut(&str),
    ) -> Result<curator::Curation, String> {
        let c = curator::deterministic(req);
        on_delta(&c.intro);
        Ok(c)
    }
}

pub fn build_provider(cfg: &AiCfg) -> Arc<dyn IntentProvider> {
    match cfg.provider.as_str() {
        "openai" => Arc::new(LlmProvider::new(LlmConfig {
            kind: LlmKind::OpenAi,
            base_url: if cfg.base_url.trim().is_empty() {
                "https://api.openai.com/v1".to_string()
            } else {
                cfg.base_url.trim().to_string()
            },
            model: if cfg.model.trim().is_empty() {
                "gpt-4o-mini".to_string()
            } else {
                cfg.model.trim().to_string()
            },
            api_key: cfg.api_key.clone(),
        })),
        "ollama" => Arc::new(LlmProvider::new(LlmConfig {
            kind: LlmKind::Ollama,
            base_url: if cfg.base_url.trim().is_empty() {
                "http://localhost:11434".to_string()
            } else {
                cfg.base_url.trim().to_string()
            },
            model: if cfg.model.trim().is_empty() {
                "qwen2.5:3b".to_string()
            } else {
                cfg.model.trim().to_string()
            },
            api_key: String::new(),
        })),
        _ => Arc::new(RuleParser),
    }
}

// ---------------------------------------------------------------- regole ----

const GENRE_WORDS: &[(&str, u32)] = &[
    ("horror", 27),
    ("orrore", 27),
    ("paura", 27),
    ("slasher", 27),
    ("commedia", 35),
    ("comico", 35),
    ("divertente", 35),
    ("dramma", 18),
    ("drammatico", 18),
    ("thriller", 53),
    ("suspense", 53),
    ("noir", 80),
    ("poliziesco", 80),
    ("crimine", 80),
    ("gangster", 80),
    ("fantascienza", 878),
    ("fantascientifico", 878),
    ("sci-fi", 878),
    ("azione", 28),
    ("avventura", 12),
    ("animazione", 16),
    ("animato", 16),
    ("documentario", 99),
    ("guerra", 10752),
    ("western", 37),
    ("famiglia", 10751),
    ("fantasy", 14),
    ("mistero", 9648),
    ("giallo", 9648),
    ("romantico", 10749),
    ("romance", 10749),
    ("musical", 10402),
    ("storico", 36),
    ("biografico", 36),
];

const DECADE_WORDS: &[(&str, &str)] = &[
    ("venti", "1920s"),
    ("trenta", "1930s"),
    ("quaranta", "1940s"),
    ("cinquanta", "1950s"),
    ("sessanta", "1960s"),
    ("settanta", "1970s"),
    ("ottanta", "1980s"),
    ("novanta", "1990s"),
    ("duemila", "2000s"),
];

const MOVEMENT_WORDS: &[(&str, &str)] = &[
    ("neoreal", "neorealismo"),
    ("nuova ondata", "nouvelle_vague"),
    ("nouvelle vague", "nouvelle_vague"),
    ("espressionis", "espressionismo"),
    ("giallo", "giallo_italiano"),
    ("slasher", "slasher_era"),
    ("new hollywood", "new_hollywood"),
    ("dogma 95", "dogme95"),
    ("dogme", "dogme95"),
    ("j-horror", "j_horror"),
    ("jhorror", "j_horror"),
    ("korean", "korean_new_wave"),
    ("coreano", "korean_new_wave"),
    ("hong kong", "hk_action"),
    ("mumblecore", "mumblecore"),
    ("slow cinema", "slow_cinema"),
    ("cinema novo", "novo_cinema"),
    ("nuovo cinema tedesco", "german_new_cinema"),
    ("montaggio sovietic", "soviet_montage"),
    ("cinema politico", "italian_political"),
    ("extremity", "new_french_extremity"),
];

const COUNTRY_WORDS: &[(&str, &str)] = &[
    ("italian", "IT"),
    ("frances", "FR"),
    ("americ", "US"),
    ("statunitens", "US"),
    ("hollywoodian", "US"),
    ("tedesc", "DE"),
    ("germanic", "DE"),
    ("britannic", "GB"),
    ("ingles", "GB"),
    ("giappones", "JP"),
    ("nipponic", "JP"),
    ("corean", "KR"),
    ("cines", "CN"),
    ("taiwanes", "TW"),
    ("brasilian", "BR"),
    ("messican", "MX"),
    ("argentino", "AR"),
    ("sovietic", "RU"),
    ("russ", "RU"),
    ("polacc", "PL"),
    ("danese", "DK"),
    ("svedes", "SE"),
    ("spagnol", "ES"),
    ("iberic", "ES"),
    ("portoghes", "PT"),
    ("turc", "TR"),
    ("iranian", "IR"),
    ("indian", "IN"),
    ("thai", "TH"),
];

pub struct RuleParser;

fn decade_from_year(year: u16) -> Option<String> {
    if !(1888..=2035).contains(&year) {
        return None;
    }
    Some(format!("{}0s", year / 10))
}

fn find_digits(text: &str, start: usize) -> (Option<u32>, usize) {
    let bytes = text.as_bytes();
    let mut i = start;
    while i < bytes.len() && !bytes[i].is_ascii_digit() {
        i += 1;
    }
    if i >= bytes.len() {
        return (None, text.len());
    }
    let begin = i;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        i += 1;
    }
    (text[begin..i].parse::<u32>().ok(), i)
}

fn floor_char_boundary(text: &str, mut at: usize) -> usize {
    at = at.min(text.len());
    while at > 0 && !text.is_char_boundary(at) {
        at -= 1;
    }
    at
}

fn is_negated(text: &str, at: usize) -> bool {
    let from = floor_char_boundary(text, at.saturating_sub(18));
    let ctx = &text[from..at];
    // Negazioni come parole intere: «senza commedia», «non horror». Cercare le
    // lettere dentro le parole produceva falsi: «italiano di guerra».
    const NEGATIONS: [&str; 5] = ["non", "senza", "niente", "no", "mai"];
    ctx.split(|c: char| !c.is_alphanumeric())
        .any(|w| !w.is_empty() && NEGATIONS.contains(&w))
}

/// «almeno 1000 voti», «piu di 5000 votanti»: il numero che precede il
/// riferimento ai voti e una soglia di conteggio, non un giudizio.
fn min_votes_before(lower: &str, at: usize) -> Option<u32> {
    let start = floor_char_boundary(lower, at.saturating_sub(30));
    let window = &lower[start..at];
    let mut best: Option<u32> = None;
    let mut i = 0usize;
    while i < window.len() {
        let (n, next) = find_digits(window, i);
        match n {
            Some(v) => {
                if v > 10 {
                    best = Some(v);
                }
                i = next.max(i + 1);
            }
            None => break,
        }
    }
    best
}

/// Intervallo di giudizio: «voto tra 6 e 8», «da 6 a 8», «fra 7 e 9».
fn rating_range(lower: &str) -> Option<(f32, f32)> {
    let at = lower.find("voto").or_else(|| lower.find("rating"))?;
    let tail = &lower[at..];
    let open = ["tra ", "da ", "fra "]
        .iter()
        .filter_map(|p| tail.find(p))
        .min()?;
    let body = &tail[open..];
    let body = &body[..floor_char_boundary(body, 40)];
    let mut nums: Vec<u32> = Vec::new();
    let mut i = 0usize;
    while i < body.len() && nums.len() < 2 {
        let (n, next) = find_digits(body, i);
        match n {
            Some(v) => {
                if v <= 10 {
                    nums.push(v);
                }
                i = next.max(i + 1);
            }
            None => break,
        }
    }
    if nums.len() == 2 && nums[0] <= nums[1] {
        Some((nums[0] as f32, nums[1] as f32))
    } else {
        None
    }
}

impl RuleParser {
    fn parse_rules(&self, text: &str, tax: &Taxonomy) -> Intent {
        let mut it = Intent {
            source: "regole".into(),
            ..Default::default()
        };
        let lower = text.to_lowercase();
        let mut matched_any = false;

        // --- decenni: forme in cifre (anni 80, 80s, 1980s, 1985) ---
        let mut cursor = 0usize;
        while cursor < lower.len() {
            let (num, next) = find_digits(&lower, cursor);
            cursor = next.max(cursor + 1);
            let Some(n) = num else { break };
            let start = cursor.saturating_sub(n.to_string().len());
            let before = lower[..start]
                .trim_end()
                .trim_end_matches(['\'', '\u{2019}', ' ']);
            let after = &lower[cursor..];
            let decade = match n {
                1900..=2035 if n >= 1000 => decade_from_year(n as u16),
                20..=90 if n % 10 == 0 => {
                    let keyword_close = before.ends_with("anni");
                    let suffix_close =
                        after.starts_with('s') || after.starts_with('\'') || after.starts_with('\u{2019}');
                    if keyword_close || suffix_close {
                        decade_from_year(1900 + n as u16)
                    } else {
                        None
                    }
                }
                _ => None,
            };
            if let Some(d) = decade {
                if tax.decade(&d).is_some() && !it.facets.decades.contains(&d) {
                    it.facets.decades.push(d);
                    matched_any = true;
                }
            }
        }

        // --- decenni: forme in lettere ---
        for (word, id) in DECADE_WORDS {
            if lower.contains(word) && tax.decade(id).is_some() {
                if !it.facets.decades.contains(&id.to_string()) {
                    it.facets.decades.push(id.to_string());
                }
                matched_any = true;
            }
        }

        // --- movimenti ---
        for (needle, id) in MOVEMENT_WORDS {
            if lower.contains(needle) && tax.movement(id).is_some() {
                if !it.facets.movements.iter().any(|m| m == id) {
                    it.facets.movements.push(id.to_string());
                }
                matched_any = true;
            }
        }

        // --- generi (con negazione) ---
        for (word, id) in GENRE_WORDS {
            let mut from = 0usize;
            while let Some(pos) = lower[from..].find(word) {
                let at = from + pos;
                if is_negated(&lower, at) {
                    toggle(&mut it.facets.without_genres, *id);
                } else {
                    if !it.facets.genres.contains(id) {
                        it.facets.genres.push(*id);
                    }
                }
                matched_any = true;
                from = at + word.len();
            }
        }
        it.facets.genres.retain(|g| !it.facets.without_genres.contains(g));

        // --- paesi ---
        for (word, code) in COUNTRY_WORDS {
            if lower.contains(word) {
                if !it.facets.countries.iter().any(|c| c == code) {
                    it.facets.countries.push(code.to_string());
                }
                matched_any = true;
            }
        }

        // --- mood ---
        for mood in &tax.moods {
            let label = mood.label.to_lowercase();
            let kw = mood.keyword.to_lowercase();
            if lower.contains(&label) || lower.contains(&kw) {
                if !it.facets.moods.iter().any(|m| m == &mood.id) {
                    it.facets.moods.push(mood.id.clone());
                }
                matched_any = true;
            }
        }

        // --- voto (minimo, massimo, intervallo) e numero di voti ---
        if lower.contains("capolavor") {
            it.facets.min_rating = Some(8.0);
            matched_any = true;
        }
        if lower.contains("voto alto")
            || lower.contains("voto buon")
            || lower.contains("voto minimo")
            || lower.contains("acclamat")
        {
            it.facets.min_rating = Some(7.0);
            matched_any = true;
        }
        if lower.contains("ben votat")
            || lower.contains("molto votat")
            || lower.contains("tanti voti")
            || lower.contains("molti voti")
            || lower.contains("voti alti")
        {
            it.facets.min_votes = Some(1000);
            matched_any = true;
        }
        if let Some((a, b)) = rating_range(&lower) {
            it.facets.min_rating = Some(a);
            it.facets.max_rating = Some(b);
            matched_any = true;
        } else if lower.contains("vot") || lower.contains("rating") {
            let at = lower
                .find("vot")
                .or_else(|| lower.find("rating"))
                .unwrap_or(0);
            // (a) numero che segue «voto»: e il giudizio, se sta in 0-10
            let after = &lower[at..floor_char_boundary(&lower, at + 24)];
            let (num_after, _) = find_digits(after, 0);
            if let Some(n) = num_after {
                if n <= 10 {
                    // «voto 8», «voti maggiori di 8», «sotto il 6»
                    let ceiling = after.contains("inferior")
                        || after.contains("sotto")
                        || after.contains("fino a")
                        || after.contains("massimo")
                        || after.contains("meno di");
                    if ceiling {
                        it.facets.max_rating = Some(n as f32);
                    } else {
                        it.facets.min_rating = Some(n as f32);
                    }
                    matched_any = true;
                }
            }
            // (b) numero che precede «voti»: e una soglia di conteggio
            let count = ["voti", "votanti"]
                .iter()
                .filter_map(|m| lower.find(m).and_then(|p| min_votes_before(&lower, p)))
                .max();
            if let Some(v) = count {
                it.facets.min_votes = Some(v);
                matched_any = true;
            }
        }

        // --- temi liberi: «che parla di resistenza», «sul tema motociclismo» ---
        const THEME_STOP: [&str; 8] = [
            " che ", " e ", " ed ", " ma ", " con ", " per ", " dove ", " in cui ",
        ];
        const THEME_PREFIXES: [&str; 8] = [
            "parlano di ",
            "parla di ",
            "trattano di ",
            "tratta di ",
            "sul tema ",
            "a tema ",
            "riguardo a ",
            "si occupa di ",
        ];
        for prefix in THEME_PREFIXES {
            let Some(pos) = lower.find(prefix) else { continue };
            let mut rest = lower[pos + prefix.len()..].trim().to_string();
            for art in [
                "il ", "lo ", "la ", "i ", "gli ", "le ", "un ", "uno ", "una ", "dei ", "del ",
                "della ", "delle ", "dello ", "degli ", "l'",
            ] {
                if let Some(stripped) = rest.strip_prefix(art) {
                    rest = stripped.to_string();
                    break;
                }
            }
            let cut = THEME_STOP
                .iter()
                .filter_map(|s| rest.find(s))
                .min()
                .unwrap_or(rest.len());
            let theme = rest[..cut]
                .trim()
                .trim_matches(['.', ',', '"', '\'', '?', '!'])
                .to_string();
            if (3..=40).contains(&theme.chars().count()) {
                // se e gia un mood della tassonomia non lo ripeto come tema libero
                let is_mood = it.facets.moods.iter().any(|id| {
                    tax.mood(id)
                        .map(|m| m.keyword.eq_ignore_ascii_case(&theme))
                        .unwrap_or(false)
                });
                if !is_mood && !it.facets.keywords.contains(&theme) {
                    it.facets.keywords.push(theme);
                }
                matched_any = true;
            }
            break;
        }

        // --- durata ---
        if lower.contains("cortometragg") {
            it.facets.runtime = Some((0, 40));
            matched_any = true;
        } else if lower.contains("sotto i ") && lower.contains("minut") {
            if let Some(pos) = lower.find("sotto i ") {
                let (num, _) = find_digits(&lower, pos);
                if let Some(n) = num {
                    it.facets.runtime = Some((0, n));
                    matched_any = true;
                }
            }
        }

        // --- ordinamento ---
        if lower.contains("recent") || lower.contains("nuovissim") {
            it.facets.sort = Sort::Newest;
        } else if lower.contains("vecch") || lower.contains("classic") {
            it.facets.sort = Sort::Oldest;
        } else if lower.contains("votat") || lower.contains("migliori") {
            it.facets.sort = Sort::Rating;
        }

        // --- "tipo X" / "simile a X" ---
        for prefix in ["tipo ", "simile a ", "in stile ", "come il film ", "stile "] {
            if let Some(pos) = lower.find(prefix) {
                let rest = lower[pos + prefix.len()..].trim();
                let rest = rest.trim_matches(|c| c == '"' || c == '\'');
                if !rest.is_empty() {
                    it.similar_to = Some(rest.to_string());
                    matched_any = true;
                }
                break;
            }
        }

        // --- testo tra virgolette = titolo ---
        if let Some(start) = text.find('"') {
            if let Some(end) = text[start + 1..].find('"') {
                let title = text[start + 1..start + 1 + end].trim();
                if !title.is_empty() {
                    it.free_text = Some(title.to_string());
                    matched_any = true;
                }
            }
        }

        if !matched_any {
            let trimmed = text.trim();
            if !trimmed.is_empty() {
                it.free_text = Some(trimmed.to_string());
                it.notes
                    .push("Nessuna faccia riconosciuta: cerco il testo come titolo.".into());
            }
        } else if it.free_text.is_none() {
            it.notes.push("Ho estratto le facce dalla descrizione.".into());
        }

        if !it.facets.decades.is_empty() {
            let labels = it
                .facets
                .decades
                .iter()
                .filter_map(|d| tax.decade(d))
                .map(|d| d.label.clone())
                .collect::<Vec<_>>()
                .join(", ");
            it.notes.push(format!("Periodo: {labels}"));
        }
        if let Some(r) = it.facets.min_rating {
            it.notes.push(format!("Voto minimo: {r:.1}"));
        }
        if let Some(r) = it.facets.max_rating {
            it.notes.push(format!("Voto massimo: {r:.1}"));
        }
        if let Some(v) = it.facets.min_votes {
            it.notes.push(format!("Almeno {v} voti"));
        }

        it
    }
}

impl IntentProvider for RuleParser {
    fn label(&self) -> String {
        "parser a regole (offline)".into()
    }

    fn parse(&self, text: &str, tax: &Taxonomy) -> Result<Intent, String> {
        Ok(self.parse_rules(text, tax))
    }
}

// ------------------------------------------------------------------- llm ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LlmKind {
    OpenAi,
    Ollama,
}

/// Un modello scoperto interrogando il servizio.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub label: String,
    pub company: String,
    pub context: Option<u64>,
    pub multiplier: Option<f64>,
}

impl ModelInfo {
    /// Voce del menu a tendina: "OpenAI · GPT 6 Astra · 1.050k · x10".
    pub fn menu_label(&self) -> String {
        let mut s = String::new();
        if !self.company.is_empty() {
            s.push_str(&self.company);
            s.push_str(" · ");
        }
        if self.label.is_empty() || self.label == self.id {
            s.push_str(&self.id);
        } else {
            s.push_str(&self.label);
        }
        if let Some(c) = self.context {
            s.push_str(&format!(" · {}k ctx", c / 1000));
        }
        if let Some(m) = self.multiplier {
            s.push_str(&format!(" · x{}", trim_float(m)));
        }
        s
    }
}

/// Descrizione breve del modello scelto, mostrata sotto il menu.
pub fn model_details(info: &ModelInfo) -> String {
    let mut bits: Vec<String> = vec![format!("id `{}`", info.id)];
    if let Some(c) = info.context {
        bits.push(format!("contesto {c} token"));
    }
    if let Some(m) = info.multiplier {
        bits.push(format!("costo relativo x{}", trim_float(m)));
    }
    bits.join(" · ")
}

fn trim_float(v: f64) -> String {
    let s = format!("{v:.2}");
    s.trim_end_matches('0').trim_end_matches('.').to_string()
}

/// Normalizza il collegamento al servizio: accetta sia la base (`.../v1`) sia
/// l'URL completo (`.../v1/chat/completions`), come lo si copia dalla guida.
pub fn normalize_base(raw: &str) -> String {
    let mut s = raw.trim().trim_end_matches('/').to_string();
    for tail in [
        "/chat/completions",
        "/chat",
        "/models",
        "/completions",
        "/api/tags",
        "/api/chat",
    ] {
        if s.to_lowercase().ends_with(tail) {
            let keep = s.len() - tail.len();
            s.truncate(keep);
            break;
        }
    }
    s.trim_end_matches('/').to_string()
}

/// Chiede al servizio l'elenco dei modelli utilizzabili con questa credenziale.
/// Supporta lo standard OpenAI (`GET /models`) e Ollama (`GET /api/tags`).
pub fn list_models(kind: LlmKind, base_url: &str, api_key: &str) -> Result<Vec<ModelInfo>, String> {
    let base = normalize_base(base_url);
    let (url, bearer) = match kind {
        LlmKind::OpenAi => {
            if base.is_empty() {
                return Err("Inserisci prima il collegamento del servizio.".into());
            }
            (format!("{base}/models"), !api_key.trim().is_empty())
        }
        LlmKind::Ollama => {
            let base = if base.is_empty() {
                "http://localhost:11434".to_string()
            } else {
                base
            };
            (format!("{base}/api/tags"), false)
        }
    };

    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| Client::new());
    let mut req = http.get(&url);
    if bearer {
        req = req.bearer_auth(api_key.trim());
    }
    let resp = req
        .send()
        .map_err(|e| format!("Servizio non raggiungibile ({url}): {e}"))?;
    let status = resp.status();
    let text = resp.text().unwrap_or_default();
    if !status.is_success() {
        let snippet: String = text.chars().take(160).collect();
        return Err(match status.as_u16() {
            401 | 403 => format!(
                "Credenziale rifiutata ({}). Controlla la chiave API per {url}.",
                status.as_u16()
            ),
            404 => format!(
                "Il servizio non espone l'elenco modelli ({url}). Scrivi il nome del modello a mano."
            ),
            code => format!("Errore {code} da {url}: {snippet}"),
        });
    }
    let value: serde_json::Value =
        serde_json::from_str(&text).map_err(|e| format!("Risposta non leggibile: {e}"))?;
    let arr = value
        .get("data")
        .and_then(|d| d.as_array())
        .or_else(|| value.get("models").and_then(|m| m.as_array()))
        .or_else(|| value.as_array())
        .ok_or_else(|| {
            format!(
                "Elenco modelli in un formato non riconosciuto: {}",
                text.chars().take(160).collect::<String>()
            )
        })?;

    let mut out: Vec<ModelInfo> = Vec::new();
    for item in arr {
        let id = item
            .get("id")
            .and_then(|v| v.as_str())
            .or_else(|| item.get("name").and_then(|v| v.as_str()))
            .or_else(|| item.as_str());
        let Some(id) = id else { continue };
        let label = item
            .get("display_name")
            .or_else(|| item.get("label"))
            .and_then(|v| v.as_str())
            .unwrap_or(id)
            .to_string();
        let company = item
            .get("company")
            .or_else(|| item.get("owned_by"))
            .or_else(|| item.get("publisher"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let context = item
            .get("context_window")
            .or_else(|| item.get("context_length"))
            .or_else(|| item.get("context"))
            .and_then(|v| v.as_u64());
        let multiplier = item.get("multiplier").and_then(|v| {
            v.as_f64().or_else(|| {
                v.as_str()
                    .map(|s| s.replace(',', "."))
                    .and_then(|s| s.parse::<f64>().ok())
            })
        });
        out.push(ModelInfo {
            id: id.to_string(),
            label,
            company,
            context,
            multiplier,
        });
    }
    if out.is_empty() {
        return Err("Il servizio ha risposto ma non espone modelli utilizzabili.".into());
    }
    out.sort_by(|a, b| {
        a.company
            .to_lowercase()
            .cmp(&b.company.to_lowercase())
            .then(a.id.to_lowercase().cmp(&b.id.to_lowercase()))
    });
    out.dedup_by(|a, b| a.id == b.id);
    Ok(out)
}

/// Fra i modelli scoperti sceglie quello che costa meno: buon predefinito per
/// un compito semplice come estrarre facce da una frase.
pub fn cheapest_models(models: &[ModelInfo]) -> Option<ModelInfo> {
    models
        .iter()
        .filter(|m| m.multiplier.is_some())
        .min_by(|a, b| {
            a.multiplier
                .partial_cmp(&b.multiplier)
                .unwrap_or(std::cmp::Ordering::Equal)
        })
        .cloned()
}

/// Aggiunto al prompt d'interpretazione quando si conversa in chat: il
/// messaggio puo essere un seguito, quindi lo stato dei filtri va restituito
/// completo (non solo le differenze).
const CHAT_ADDENDUM: &str = "Stai conversando con l'utente in una chat: il messaggio puo essere un seguito («no, senza commedia», «piu recenti», «e se fossero italiani?»).\n\
Ricevi i filtri attivi e i messaggi recenti.\n\
Restituisci lo stato COMPLETO dei filtri aggiornato: conserva i filtri attivi che il messaggio non tocca, aggiungi i nuovi vincoli e ometti quelli che l'utente toglie o contraddice. Non limitarti alle differenze.\n\
Se il messaggio e una domanda su un film (regista, anno, curiosita) oppure nomina un film senza chiederne la ricerca, metti il titolo in `similar_to`: i film proposti saranno legati a quello e la risposta alla domanda la dara il curatore.\n\
Se il messaggio non riguarda il cinema (una domanda generale, un saluto, una curiosita), usa \"intent\":\"off_topic\" e NON mettere la frase in `title` o `similar_to`: il curatore rispondera in una riga e proporra cinque film. Se la domanda ha un legame naturale con il cinema, aggiungi il filtro piu adatto (per «qual e la capitale della Francia?» -> countries [\"FR\"]; per «chi ha vinto i mondiali?» -> keywords [\"calcio\"]); altrimenti lascia i filtri come sono.\n\
Se la richiesta e vaga («sorprendimi», «qualcosa di bello») non inventare un titolo: usa \"intent\":\"faceted\" con pochi vincoli o nessuno.\n\
Se l'utente chiede esplicitamente di ricominciare da zero, usa \"intent\":\"reset\" e restituisci i filtri vuoti.";

/// Prompt del curatore: sceglie fra candidati reali e scrive i commenti.
const CURATOR_SYSTEM: &str = "Sei il curatore di CineMatch, un'app che consiglia film. Ricevi la richiesta dell'utente, i filtri interpretati e un elenco di film reali candidati (con id TMDB).\n\
Scegli i film che rispondono meglio alla richiesta e scrivi i commenti.\n\
Rispondi ESCLUSIVAMENTE con un oggetto JSON di questa forma:\n\
{\"intro\":\"...\",\"picks\":[{\"id\":123,\"perche\":\"...\",\"consiglio\":\"...\"}]}\n\
Tassativo:\n\
- usa solo gli id dell'elenco candidati: mai inventare film o id;\n\
- scegli esattamente 5 film (o tutti, se i candidati sono meno di 5), in ordine dal piu adatto al meno adatto;\n\
- «intro»: 1-2 frasi in italiano, massimo 240 caratteri: se l'utente ha fatto una domanda (anche fuori tema) rispondi in una riga e collega ai film; altrimenti presenta la scelta citando i criteri;\n\
- «perche»: 1-2 frasi, massimo 220 caratteri: spiega perche il film risponde ai criteri della richiesta; non ripetere anno, paese, voto o numero di voti gia presenti nella scheda; se un criterio non e rispettato dillo con onesta;\n\
- «consiglio»: 1 frase pratica e personale, massimo 150 caratteri, specifica per questo film e diversa da quella degli altri film scelti (come guardarlo, a cosa fare attenzione, sottotitoli o doppiaggio); evita consigli generici e imperativi assoluti: preferisci «se puoi», «prova a»;\n\
- testo semplice, niente markdown, niente spoiler.";

/// Fotografia leggibile dei filtri attivi, per i prompt della chat.
fn facets_digest(tax: &Taxonomy, f: &Facets) -> String {
    let mut parts: Vec<String> = Vec::new();
    let movements: Vec<String> = f
        .movements
        .iter()
        .filter_map(|id| tax.movement(id).map(|m| format!("{} ({})", m.label, m.id)))
        .collect();
    if !movements.is_empty() {
        parts.push(format!("movimenti: {}", movements.join(", ")));
    }
    let decades: Vec<String> = f
        .decades
        .iter()
        .filter_map(|id| tax.decade(id).map(|d| format!("{} ({})", d.label, d.id)))
        .collect();
    if !decades.is_empty() {
        parts.push(format!("decenni: {}", decades.join(", ")));
    }
    let genres: Vec<String> = f
        .genres
        .iter()
        .map(|g| match tax.genre_name(*g) {
            Some(name) => format!("{g} ({name})"),
            None => g.to_string(),
        })
        .collect();
    if !genres.is_empty() {
        parts.push(format!("generi: {}", genres.join(", ")));
    }
    let without: Vec<String> = f
        .without_genres
        .iter()
        .map(|g| match tax.genre_name(*g) {
            Some(name) => format!("{g} ({name})"),
            None => g.to_string(),
        })
        .collect();
    if !without.is_empty() {
        parts.push(format!("generi da escludere: {}", without.join(", ")));
    }
    let countries: Vec<String> = f
        .countries
        .iter()
        .map(|c| match tax.country_name(c) {
            Some(name) => format!("{c} ({name})"),
            None => c.clone(),
        })
        .collect();
    if !countries.is_empty() {
        parts.push(format!("paesi: {}", countries.join(", ")));
    }
    let moods: Vec<String> = f
        .moods
        .iter()
        .filter_map(|id| tax.mood(id).map(|m| format!("{} ({})", m.label, m.id)))
        .collect();
    if !moods.is_empty() {
        parts.push(format!("mood: {}", moods.join(", ")));
    }
    if !f.keywords.is_empty() {
        parts.push(format!("temi: {}", f.keywords.join(", ")));
    }
    if let Some(r) = f.min_rating {
        parts.push(format!("voto_minimo: {r}"));
    }
    if let Some(r) = f.max_rating {
        parts.push(format!("voto_massimo: {r}"));
    }
    if let Some(v) = f.min_votes {
        parts.push(format!("voti_minimi: {v}"));
    }
    if let Some((a, b)) = f.runtime {
        parts.push(format!("durata: {a}-{b} min"));
    }
    if parts.is_empty() {
        "nessuno".to_string()
    } else {
        parts.join(" · ")
    }
}

#[derive(Clone, Debug)]
pub struct LlmConfig {
    pub kind: LlmKind,
    pub base_url: String,
    pub model: String,
    pub api_key: String,
}

#[derive(Deserialize, Default)]
struct LlmFacets {
    #[serde(default)]
    decades: Vec<String>,
    #[serde(default)]
    movements: Vec<String>,
    #[serde(default)]
    genres: Vec<u32>,
    #[serde(default)]
    without_genres: Vec<u32>,
    #[serde(default)]
    countries: Vec<String>,
    #[serde(default)]
    moods: Vec<String>,
    /// Temi liberi: nomi di argomento (non id), risolti sul dizionario TMDB.
    #[serde(default)]
    keywords: Vec<String>,
    #[serde(default)]
    min_rating: Option<f32>,
    #[serde(default)]
    max_rating: Option<f32>,
    #[serde(default)]
    min_votes: Option<u32>,
    #[serde(default)]
    runtime: Option<Vec<u32>>,
    #[serde(default)]
    sort: Option<String>,
}

#[derive(Deserialize, Default)]
struct LlmAnswer {
    #[serde(default)]
    intent: String,
    #[serde(default)]
    facets: LlmFacets,
    #[serde(default)]
    title: Option<String>,
    #[serde(default)]
    similar_to: Option<String>,
    #[serde(default)]
    reason: Option<String>,
}

pub struct LlmProvider {
    cfg: LlmConfig,
    http: Client,
}

impl LlmProvider {
    pub fn new(mut cfg: LlmConfig) -> Self {
        cfg.base_url = normalize_base(&cfg.base_url);
        let http = Client::builder()
            .timeout(Duration::from_secs(60))
            .build()
            .unwrap_or_else(|_| Client::new());
        Self { cfg, http }
    }

    /// Prova reale: chiede al modello di interpretare una frase e controlla che
    /// risponda in JSON conforme. Verifica insieme collegamento, credenziale e
    /// modello — senza salvare nulla.
    pub fn probe(&self, tax: &Taxonomy) -> Result<String, String> {
        let raw = self.chat(
            &Self::system_prompt(tax),
            "Richiesta utente: giallo italiano anni 80, voto alto e almeno 2000 voti, che parla di vendetta",
            600,
        )?;
        let intent = Self::parse_json(&raw, tax)?;
        let mut bits: Vec<String> = Vec::new();
        if !intent.facets.movements.is_empty() {
            bits.push(format!("movimenti: {}", intent.facets.movements.join(", ")));
        }
        if !intent.facets.decades.is_empty() {
            bits.push(format!("periodo: {}", intent.facets.decades.join(", ")));
        }
        if !intent.facets.genres.is_empty() {
            let names = intent
                .facets
                .genres
                .iter()
                .map(|g| tax.genre_name(*g).unwrap_or_else(|| g.to_string()))
                .collect::<Vec<_>>()
                .join(", ");
            bits.push(format!("generi: {names}"));
        }
        if !intent.facets.countries.is_empty() {
            bits.push(format!("paesi: {}", intent.facets.countries.join(", ")));
        }
        if let Some(r) = intent.facets.min_rating {
            bits.push(format!("voto ≥ {r}"));
        }
        if let Some(r) = intent.facets.max_rating {
            bits.push(format!("voto ≤ {r}"));
        }
        if let Some(v) = intent.facets.min_votes {
            bits.push(format!("voti ≥ {v}"));
        }
        if !intent.facets.keywords.is_empty() {
            bits.push(format!("temi: {}", intent.facets.keywords.join(", ")));
        }
        let detail = if bits.is_empty() {
            "nessuna faccia riconosciuta".to_string()
        } else {
            bits.join(" · ")
        };
        Ok(format!("{} · JSON conforme · {detail}", self.cfg.model))
    }

    fn system_prompt(tax: &Taxonomy) -> String {
        format!(
            "Sei il motore di interpretazione di un'app di ricerca film. Traduci la richiesta dell'utente in JSON.\n\
             L'utente puo scrivere in modo libero e colloquiale: estrai tutti i vincoli che riconosci e ignora il resto.\n\
             Usa SOLO gli identificatori elencati qui sotto; per gli argomenti che non sono in elenco usa `keywords`.\n\n{}\n\n\
             Rispondi esclusivamente con un oggetto JSON con questa forma:\n\
             {{\"intent\":\"faceted|title|similar|mixed|reset|off_topic\",\"facets\":{{\"decades\":[],\"movements\":[],\"genres\":[],\"without_genres\":[],\"countries\":[],\"moods\":[],\"keywords\":[],\"min_rating\":null,\"max_rating\":null,\"min_votes\":null,\"runtime\":null,\"sort\":null}},\"title\":null,\"similar_to\":null,\"reason\":\"\"}}\n\
             Come riempire i campi:\n\
             - `decades`: id dei decenni («anni 80» -> \"1980s\"). Se scegli un movimento, NON aggiungere decenni incompatibili col suo periodo.\n\
             - `movements`: la corrente cinematografica quando la richiesta la nomina o la descrive.\n\
             - `genres`: id numerici TMDB. «giallo» o «mistero» -> 9648; «horror» -> 27; «commedia» -> 35; «guerra» -> 10752.\n\
             - `without_genres`: generi da ESCLUDERE («senza commedia» -> 35).\n\
             - `moods`: temi della tassonomia, solo se la richiesta li nomina (id dell'elenco).\n\
             - `keywords`: al massimo 3 argomenti liberi che non sono generi ne mood («che parla di resistenza» -> [\"resistenza\"]; «film sulle moto» -> [\"motociclismo\"]). Sostantivi brevi in italiano, minuscoli; mai identificatori inventati.\n\
             - `min_rating` / `max_rating`: giudizio 0-10. «voto alto»/«ben giudicati» -> 7; «capolavori» -> 8; «tra 6 e 8» -> 6 e 8; «sotto il 6» -> max 6.\n\
             - `min_votes`: numero minimo di voti («ben votati» -> 1000; «almeno 5000 voti» -> 5000).\n\
             - `runtime`: intervallo in minuti [minimo, massimo]. «corti»/«cortometraggi» -> [0,40]; «sotto i 90 minuti» -> [0,90]; «lunghi» -> [150,400].\n\
             - `sort`: \"vote_average.desc\" (migliori), \"vote_count.desc\" (piu votati), \"primary_release_date.desc\" (recenti), \"primary_release_date.asc\" (classici) oppure null.\n\
             - `intent`: la modalita della ricerca; «reset» solo in chat, quando l'utente chiede esplicitamente di ricominciare da zero.\n\
             - `title`: solo se l'utente cerca un film preciso. `similar_to`: solo se chiede film simili a un titolo.\n\
             - `reason`: una frase in italiano che spiega l'interpretazione mostrata all'utente.\n\
             Genera solo combinazioni storicamente sensate; non inventare identificatori; usa null quando un campo non serve.",
            tax.prompt_summary()
        )
    }

    /// Corpo della richiesta OpenAI. Il ragionamento e' disattivato: per
    /// estrarre facce e scrivere cinque commenti non serve, e su alcuni
    /// modelli costa decine di secondi.
    fn openai_body(
        &self,
        system: &str,
        user: &str,
        stream: bool,
        max_tokens: u32,
    ) -> serde_json::Value {
        let mut body = serde_json::json!({
            "model": self.cfg.model,
            "temperature": if stream { 0.4 } else { 0.1 },
            // Due dialetti per lo stesso interruttore: il primo e' standard
            // OpenAI, il secondo lo usano i proxy che servono modelli con
            // «thinking». Se il servizio non li conosce, `openai_send`
            // riprova senza.
            "reasoning_effort": "none",
            "chat_template_kwargs": { "enable_thinking": false },
            "max_tokens": max_tokens,
            "messages": [
                { "role": "system", "content": system },
                { "role": "user", "content": user }
            ]
        });
        if stream {
            body["stream"] = serde_json::json!(true);
        } else {
            body["stream"] = serde_json::json!(false);
            body["response_format"] = serde_json::json!({ "type": "json_object" });
        }
        body
    }

    /// Invia al servizio OpenAI-compatibile. Se il servizio rifiuta i
    /// parametri facoltativi (ragionamento/tetto token), riprova una volta
    /// senza: gli altri servizi continuano a funzionare.
    fn openai_send(
        &self,
        system: &str,
        user: &str,
        stream: bool,
        max_tokens: u32,
    ) -> Result<reqwest::blocking::Response, String> {
        let url = format!(
            "{}/chat/completions",
            self.cfg.base_url.trim_end_matches('/')
        );
        let timeout = Duration::from_secs(if stream { 40 } else { 20 });
        let mut body = self.openai_body(system, user, stream, max_tokens);
        let mut retried = false;
        loop {
            let mut req = self.http.post(&url).json(&body).timeout(timeout);
            if !self.cfg.api_key.trim().is_empty() {
                req = req.bearer_auth(self.cfg.api_key.trim());
            }
            let resp = req
                .send()
                .map_err(|e| format!("LLM non raggiungibile: {e}"))?;
            if resp.status().as_u16() != 400 || retried {
                return Ok(resp);
            }
            // Il servizio non conosce uno dei parametri facoltativi: li tolgo
            // e riprovo una volta sola. Se anche il secondo tentativo e' 400,
            // l'errore arriva al chiamante con il corpo del servizio.
            if let Some(obj) = body.as_object_mut() {
                obj.remove("reasoning_effort");
                obj.remove("chat_template_kwargs");
                obj.remove("max_tokens");
            }
            retried = true;
        }
    }

    fn chat(&self, system: &str, user: &str, max_tokens: u32) -> Result<String, String> {
        match self.cfg.kind {
            LlmKind::OpenAi => {
                let resp = self.openai_send(system, user, false, max_tokens)?;
                let status = resp.status();
                let text = resp.text().unwrap_or_default();
                if !status.is_success() {
                    return Err(format!(
                        "LLM ha risposto {}: {}",
                        status.as_u16(),
                        text.chars().take(200).collect::<String>()
                    ));
                }
                let v: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("JSON LLM non valido: {e}"))?;
                v["choices"][0]["message"]["content"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| "Risposta LLM senza contenuto".to_string())
            }
            LlmKind::Ollama => {
                let url = format!("{}/api/chat", self.cfg.base_url.trim_end_matches('/'));
                let body = serde_json::json!({
                    "model": self.cfg.model,
                    "stream": false,
                    "format": "json",
                    "options": { "temperature": 0.1, "num_predict": max_tokens },
                    "messages": [
                        { "role": "system", "content": system },
                        { "role": "user", "content": user }
                    ]
                });
                let resp = self
                    .http
                    .post(&url)
                    .json(&body)
                    .send()
                    .map_err(|e| format!("Ollama non raggiungibile: {e}"))?;
                let status = resp.status();
                let text = resp.text().unwrap_or_default();
                if !status.is_success() {
                    return Err(format!(
                        "Ollama ha risposto {}: {}",
                        status.as_u16(),
                        text.chars().take(200).collect::<String>()
                    ));
                }
                let v: serde_json::Value =
                    serde_json::from_str(&text).map_err(|e| format!("JSON Ollama non valido: {e}"))?;
                v["message"]["content"]
                    .as_str()
                    .map(|s| s.to_string())
                    .ok_or_else(|| "Risposta Ollama senza contenuto".to_string())
            }
        }
    }

    /// Chiamata con streaming (SSE per servizi OpenAI-compatibili). Ogni
    /// frammento di testo viene passato a `on_delta`; ritorna il testo
    /// completo. Se il servizio ignora `stream` ripiega sulla risposta JSON
    /// singola, cosi non si rompe nulla.
    fn chat_stream(
        &self,
        system: &str,
        user: &str,
        max_tokens: u32,
        on_delta: &mut dyn FnMut(&str),
    ) -> Result<String, String> {
        match self.cfg.kind {
            LlmKind::OpenAi => {
                let resp = self.openai_send(system, user, true, max_tokens)?;
                let status = resp.status();
                if !status.is_success() {
                    let text = resp.text().unwrap_or_default();
                    return Err(format!(
                        "LLM ha risposto {}: {}",
                        status.as_u16(),
                        text.chars().take(200).collect::<String>()
                    ));
                }
                let mut full = String::new();
                let mut plain = String::new();
                let mut saw_sse = false;
                for line in std::io::BufReader::new(resp).lines() {
                    let line = line.map_err(|e| format!("Lettura streaming fallita: {e}"))?;
                    if let Some(data) = line.strip_prefix("data:") {
                        saw_sse = true;
                        let data = data.trim();
                        if data == "[DONE]" {
                            break;
                        }
                        if let Ok(v) = serde_json::from_str::<serde_json::Value>(data) {
                            if let Some(chunk) = v["choices"][0]["delta"]["content"].as_str() {
                                if !chunk.is_empty() {
                                    full.push_str(chunk);
                                    on_delta(chunk);
                                }
                            }
                        }
                    } else if !line.trim().is_empty() {
                        plain.push_str(&line);
                        plain.push('\n');
                    }
                }
                if !saw_sse {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(plain.trim()) {
                        if let Some(text) = v["choices"][0]["message"]["content"].as_str() {
                            return Ok(text.to_string());
                        }
                    }
                    if full.is_empty() {
                        return Err("Streaming LLM vuoto".into());
                    }
                }
                Ok(full)
            }
            LlmKind::Ollama => {
                let url = format!("{}/api/chat", self.cfg.base_url.trim_end_matches('/'));
                let body = serde_json::json!({
                    "model": self.cfg.model,
                    "stream": true,
                    "options": { "temperature": 0.4, "num_predict": max_tokens },
                    "messages": [
                        { "role": "system", "content": system },
                        { "role": "user", "content": user }
                    ]
                });
                let resp = self
                    .http
                    .post(&url)
                    .json(&body)
                    .timeout(Duration::from_secs(150))
                    .send()
                    .map_err(|e| format!("Ollama non raggiungibile: {e}"))?;
                let status = resp.status();
                if !status.is_success() {
                    let text = resp.text().unwrap_or_default();
                    return Err(format!(
                        "Ollama ha risposto {}: {}",
                        status.as_u16(),
                        text.chars().take(200).collect::<String>()
                    ));
                }
                let mut full = String::new();
                for line in std::io::BufReader::new(resp).lines() {
                    let line = line.map_err(|e| format!("Lettura streaming fallita: {e}"))?;
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }
                    let Ok(v) = serde_json::from_str::<serde_json::Value>(trimmed) else {
                        continue;
                    };
                    if let Some(chunk) = v["message"]["content"].as_str() {
                        if !chunk.is_empty() {
                            full.push_str(chunk);
                            on_delta(chunk);
                        }
                    }
                    if v["done"].as_bool().unwrap_or(false) {
                        break;
                    }
                }
                if full.is_empty() {
                    return Err("Streaming Ollama vuoto".into());
                }
                Ok(full)
            }
        }
    }

    /// Il payload che il curatore riceve: richiesta, filtri, memoria recente
    /// e candidati reali. JSON compatto, cosi il modello resta ancorato ai dati.
    fn curator_payload(req: &curator::Request) -> String {
        let candidates: Vec<serde_json::Value> = req
            .candidates
            .iter()
            .map(|c| {
                serde_json::json!({
                    "id": c.id,
                    "titolo": c.title,
                    "anno": c.year,
                    "voto": c.rating,
                    "voti": c.votes,
                    "generi": c.genres,
                    "trama": c.overview,
                })
            })
            .collect();
        let history: Vec<String> = req
            .history
            .iter()
            .map(|(role, text)| format!("{role}: {text}"))
            .collect();
        serde_json::json!({
            "richiesta": req.user_text,
            "filtri_interpretati": req.understood,
            "conversazione_recente": history,
            "candidati": candidates,
        })
        .to_string()
    }

    fn parse_json(raw: &str, tax: &Taxonomy) -> Result<Intent, String> {
        let cleaned = raw
            .trim()
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim();
        let ans: LlmAnswer =
            serde_json::from_str(cleaned).map_err(|e| format!("JSON non conforme: {e}"))?;
        let mut facets = Facets::default();
        for d in ans.facets.decades {
            if tax.decade(&d).is_some() {
                facets.decades.push(d);
            }
        }
        for m in ans.facets.movements {
            if tax.movement(&m).is_some() {
                facets.movements.push(m);
            }
        }
        for g in ans.facets.genres {
            if tax.genre_name(g).is_some() {
                facets.genres.push(g);
            }
        }
        for g in ans.facets.without_genres {
            if tax.genre_name(g).is_some() {
                facets.without_genres.push(g);
            }
        }
        for c in ans.facets.countries {
            if tax.country_name(&c).is_some() {
                facets.countries.push(c);
            }
        }
        for m in ans.facets.moods {
            if tax.mood(&m).is_some() {
                facets.moods.push(m);
            }
        }
        // Temi liberi: nomi di argomento, non identificatori. Non si verificano
        // qui: li risolve la ricerca sul dizionario parole chiave di TMDB.
        for k in ans.facets.keywords {
            let k = k.trim().to_lowercase();
            if !k.is_empty()
                && k.chars().count() <= 40
                && facets.keywords.len() < 3
                && !facets.keywords.contains(&k)
            {
                facets.keywords.push(k);
            }
        }
        facets.min_rating = ans.facets.min_rating.map(|r| r.clamp(0.0, 10.0));
        facets.max_rating = ans.facets.max_rating.map(|r| r.clamp(0.0, 10.0));
        facets.min_votes = ans.facets.min_votes.filter(|v| *v > 0);
        if let Some(rt) = ans.facets.runtime {
            if rt.len() == 2 && rt[0] <= rt[1] {
                facets.runtime = Some((rt[0], rt[1]));
            }
        }
        facets.sort = match ans.facets.sort.as_deref() {
            Some("vote_average.desc") | Some("rating") => Sort::Rating,
            Some("vote_count.desc") | Some("votes") => Sort::Votes,
            Some("primary_release_date.desc") | Some("newest") => Sort::Newest,
            Some("primary_release_date.asc") | Some("oldest") => Sort::Oldest,
            _ => Sort::Popularity,
        };
        let mut notes = Vec::new();
        if let Some(r) = ans.reason {
            if !r.trim().is_empty() {
                notes.push(r);
            }
        }
        let free_text = if ans.title.as_deref().map(|t| !t.trim().is_empty()).unwrap_or(false) {
            ans.title
        } else {
            None
        };
        Ok(Intent {
            facets,
            free_text,
            similar_to: ans.similar_to.filter(|s| !s.trim().is_empty()),
            mode: ans.intent.trim().to_lowercase(),
            notes,
            source: "AI".into(),
        })
    }
}

impl IntentProvider for LlmProvider {
    fn label(&self) -> String {
        format!(
            "AI ({} · {})",
            match self.cfg.kind {
                LlmKind::OpenAi => "cloud",
                LlmKind::Ollama => "locale",
            },
            self.cfg.model
        )
    }

    fn is_llm(&self) -> bool {
        true
    }

    fn parse(&self, text: &str, tax: &Taxonomy) -> Result<Intent, String> {
        let system = Self::system_prompt(tax);
        let user = format!("Richiesta utente: {text}");
        let raw = match self.chat(&system, &user, 600) {
            Ok(r) => r,
            Err(e) => {
                let mut it = RuleParser.parse_rules(text, tax);
                it.notes.push(format!("AI non disponibile ({e}): uso il parser a regole."));
                return Ok(it);
            }
        };
        match Self::parse_json(&raw, tax) {
            Ok(mut it) => {
                if it.facets.is_empty() && it.free_text.is_none() && it.similar_to.is_none() {
                    let mut fb = RuleParser.parse_rules(text, tax);
                    fb.notes.push("L'AI non ha prodotto facce utili: interpreto con le regole.".into());
                    return Ok(fb);
                }
                it.notes.insert(0, "Interpretato dal modello AI.".into());
                Ok(it)
            }
            Err(e) => {
                let mut it = RuleParser.parse_rules(text, tax);
                it.notes.push(format!("Output AI non valido ({e}): uso il parser a regole."));
                Ok(it)
            }
        }
    }

    fn summarize(&self, text: &str) -> Result<String, String> {
        if text.trim().is_empty() {
            return Err("Nessuna descrizione da riassumere.".into());
        }
        let system = "Riassumi in italiano, in massimo due frasi, senza spoiler e senza ripetere il titolo.";
        self.chat(system, text, 300)
            .map(|s| s.trim().trim_matches('"').to_string())
    }

    fn parse_chat(
        &self,
        text: &str,
        tax: &Taxonomy,
        history: &[(String, String)],
        active: &Facets,
    ) -> Result<Intent, String> {
        let system = format!("{}\n\n{}", Self::system_prompt(tax), CHAT_ADDENDUM);
        let mut user = String::new();
        user.push_str(&format!("Filtri attivi: {}\n", facets_digest(tax, active)));
        if history.is_empty() {
            user.push_str("Conversazione recente: nessun messaggio precedente.\n");
        } else {
            user.push_str("Conversazione recente:\n");
            for (role, line) in history {
                user.push_str(&format!("- {role}: {line}\n"));
            }
        }
        user.push_str(&format!("\nMessaggio: {text}"));
        let raw = match self.chat(&system, &user, 600) {
            Ok(r) => r,
            Err(e) => {
                let mut it = RuleParser.parse_rules(text, tax);
                it.notes
                    .push(format!("AI non disponibile ({e}): uso il parser a regole."));
                return Ok(it);
            }
        };
        match Self::parse_json(&raw, tax) {
            Ok(mut it) => {
                // Un seguito senza vincoli non deve azzerare i filtri attivi:
                // li conserva (a meno che l'utente voglia ricominciare).
                if it.mode != "reset"
                    && it.facets.is_empty()
                    && it.free_text.is_none()
                    && it.similar_to.is_none()
                    && !active.is_empty()
                {
                    it.facets = active.clone();
                    it.notes.push("Ho mantenuto i filtri gia attivi.".into());
                }
                it.notes.insert(0, "Interpretato dal modello AI.".into());
                Ok(it)
            }
            Err(e) => {
                let mut it = RuleParser.parse_rules(text, tax);
                it.notes
                    .push(format!("Output AI non valido ({e}): uso il parser a regole."));
                Ok(it)
            }
        }
    }

    fn curate(
        &self,
        req: &curator::Request,
        on_delta: &mut dyn FnMut(&str),
    ) -> Result<curator::Curation, String> {
        if req.candidates.is_empty() {
            return Err("nessun candidato da valutare".into());
        }
        let user = Self::curator_payload(req);
        let mut full = String::new();
        let mut intro = curator::IntroStream::default();
        let mut emitted = false;
        let streamed = self.chat_stream(CURATOR_SYSTEM, &user, 1600, &mut |chunk: &str| {
            full.push_str(chunk);
            let delta = intro.feed(&full);
            if !delta.is_empty() {
                emitted = true;
                on_delta(&delta);
            }
        });
        let raw = streamed?;
        if raw.trim().is_empty() {
            return Err("Streaming LLM vuoto".into());
        }
        let valid: HashSet<u64> = req.candidates.iter().map(|c| c.id).collect();
        let mut c = curator::parse_curation(&raw, &valid)?;
        if !emitted && !c.intro.is_empty() {
            on_delta(&c.intro.clone());
        }
        if c.intro.is_empty() {
            c.intro = curator::intro_text(c.picks.len(), &req.understood);
        }
        Ok(c)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn richiesta_libera_neorealismo_guerra_voto() {
        let tax = Taxonomy::load();
        let it = RuleParser.parse_rules(
            "film del neorealismo italiano di guerra con voti maggiori di 8",
            &tax,
        );
        assert!(it.facets.movements.contains(&"neorealismo".to_string()));
        assert!(it.facets.countries.contains(&"IT".to_string()));
        assert!(it.facets.genres.contains(&10752), "il genere guerra entra");
        assert_eq!(it.facets.min_rating, Some(8.0));
    }

    #[test]
    fn tema_libero_dalla_frase() {
        let tax = Taxonomy::load();
        let it = RuleParser.parse_rules("un film che parla di resistenza partigiana", &tax);
        assert!(
            it.facets.keywords.iter().any(|k| k.starts_with("resistenza")),
            "il tema libero si estrae: {:?}",
            it.facets.keywords
        );
        assert!(
            it.free_text.is_none(),
            "una frase col tema non diventa una ricerca per titolo"
        );
    }

    #[test]
    fn il_corpo_openai_spegne_il_ragionamento() {
        let provider = LlmProvider::new(LlmConfig {
            kind: LlmKind::OpenAi,
            base_url: "http://localhost".into(),
            model: "test".into(),
            api_key: String::new(),
        });
        let body = provider.openai_body("system", "user", false, 600);
        assert_eq!(body["reasoning_effort"].as_str(), Some("none"));
        assert_eq!(
            body["chat_template_kwargs"]["enable_thinking"].as_bool(),
            Some(false)
        );
        assert_eq!(body["max_tokens"].as_u64(), Some(600));
        assert_eq!(body["response_format"]["type"].as_str(), Some("json_object"));
        assert_eq!(body["stream"].as_bool(), Some(false));

        let stream = provider.openai_body("system", "user", true, 1600);
        assert_eq!(stream["stream"].as_bool(), Some(true));
        assert!(stream.get("response_format").is_none());
    }

    #[test]
    fn la_negazione_e_una_parola_non_delle_lettere() {
        let tax = Taxonomy::load();
        let it = RuleParser.parse_rules("film italiano di guerra", &tax);
        assert!(
            it.facets.genres.contains(&10752),
            "«italiano di guerra» non e una negazione"
        );
        let it2 = RuleParser.parse_rules("commedia italiana senza guerra", &tax);
        assert!(
            it2.facets.without_genres.contains(&10752),
            "«senza guerra» e una negazione"
        );
    }

    #[test]
    fn numero_di_voti_non_e_il_giudizio() {
        let tax = Taxonomy::load();
        let it = RuleParser.parse_rules("thriller americano con almeno 5000 voti", &tax);
        assert_eq!(it.facets.min_votes, Some(5000));
        assert_eq!(it.facets.min_rating, None);
    }

    #[test]
    fn intervallo_di_voto() {
        let tax = Taxonomy::load();
        let it = RuleParser.parse_rules("commedia italiana con voto tra 6 e 8", &tax);
        assert_eq!(it.facets.min_rating, Some(6.0));
        assert_eq!(it.facets.max_rating, Some(8.0));
    }

    #[test]
    fn voto_sotto_la_soglia() {
        let tax = Taxonomy::load();
        let it = RuleParser.parse_rules("horror con voto inferiore a 6", &tax);
        assert_eq!(it.facets.max_rating, Some(6.0));
        assert_eq!(it.facets.min_rating, None);
    }

    #[test]
    fn ben_votati_e_un_minimo_di_voti() {
        let tax = Taxonomy::load();
        let it = RuleParser.parse_rules("film ben votati degli anni 90", &tax);
        assert_eq!(it.facets.min_votes, Some(1000));
        assert_eq!(it.facets.min_rating, None);
    }

    #[test]
    fn domanda_fuori_tema_resta_fuori_dal_titolo() {
        let tax = Taxonomy::load();
        let raw = r#"{"intent":"off_topic","facets":{"countries":["FR"]},"title":null,"similar_to":null,"reason":"Fuori tema"}"#;
        let it = LlmProvider::parse_json(raw, &tax).expect("JSON valido");
        assert_eq!(it.mode, "off_topic");
        assert!(it.free_text.is_none(), "la domanda non diventa un titolo");
        assert!(it.facets.countries.contains(&"FR".to_string()));
    }

    #[test]
    fn testo_unicode_non_fa_panicere_il_parser() {
        let tax = Taxonomy::load();
        let emoji = "🎬".repeat(10);
        for text in [
            format!("un film horror con voto tra 6 e 8 {emoji}"),
            format!("un film con voto {emoji}"),
            format!("{}voti", "🎬".repeat(8)),
            format!("{}horror", "🎬".repeat(6)),
            "İİ tipo".to_string(),
        ] {
            let _ = RuleParser.parse_rules(&text, &tax);
        }

        let it = RuleParser.parse_rules("İİ tipo Suspiria", &tax);
        assert_eq!(it.similar_to.as_deref(), Some("suspiria"));
    }
}
