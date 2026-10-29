use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::ai::{
    build_provider, classify_chat_action, curator, list_models, llm_config, model_details,
    ChatAction, Cleared, Intent, IntentProvider, LlmKind, LlmProvider, ModelInfo, Theme,
    ThemeConcept, DEEPSEEK_BASE, DEEPSEEK_MODEL, OPENROUTER_BASE, OPENROUTER_MODEL,
};
use crate::domain::constraints::{self, Prune};
use crate::domain::model::{toggle, Details, Facets, Movie, Sort, TrackState};
use crate::domain::suggestions;
use crate::domain::taxonomy::Taxonomy;
use crate::infra::config::Config;
use crate::infra::winclock;
use crate::integrations::simkl;
use crate::tmdb::client::{describe_credential, poster_url, Tmdb};
use crate::tmdb::models::{PagedMovies, Person};
use crate::ui::facets::{FacetAction, Section};
use crate::update;
use crate::ui::images::ImageStore;
use crate::ui::{chat, detail, facets, fx, grid, lumen, theme, unlock};

const MAX_QUERY_CHARS: usize = 300;
/// Pagine TMDB lette per una ricerca per titolo con filtri locali.
const TITLE_PAGE_CAP: u32 = 3;
/// Larghezza iniziale del pannello filtri. Egui ricorda poi l'eventuale
/// ridimensionamento fatto dall'utente.
const FILTERS_WIDTH: f32 = 360.0;
/// Il cartellone "In sala" della home si aggiorna una volta all'ora.
const IN_SALA_REFRESH: Duration = Duration::from_secs(60 * 60);
/// Se il cartellone non arriva, riprova tra 15 minuti.
const IN_SALA_RETRY: Duration = Duration::from_secs(15 * 60);

struct Results {
    facets: Facets,
    removed: Vec<String>,
    /// Come e stata impostata la ricerca (paesi implicati, temi…):
    /// informazioni, non vincoli rilassati.
    info: Vec<String>,
    ai_notes: Vec<String>,
    movies: Vec<Movie>,
    total: u32,
    total_pages: u32,
    headline: String,
    error: Option<String>,
    keyword_pairs: Vec<(String, u64)>,
    /// Intento interpretato in questa ricerca, riusato per evitare chiamate AI ripetute.
    intent: Option<(String, Intent)>,
}

#[allow(clippy::large_enum_variant)] // messaggi effimeri: boxare ogni variante aggiungerebbe indirezione
enum Msg {
    Results { gen: u64, payload: Box<Results> },
    /// Compromessi calcolati dopo i risultati: la griglia li mostra appena
    /// arrivano, senza aspettare le chiamate TMDB aggiuntive.
    Unlocks { gen: u64, unlocks: Vec<unlock::Unlock> },
    Details { gen: u64, res: Result<Details, String> },
    Check { res: Result<String, String> },
    Models { gen: u64, res: Result<Vec<ModelInfo>, String> },
    AiCheck { res: Result<String, String> },
    /// Esito del download dei voti IMDb.
    Imdb { res: Result<String, String> },
    /// Avanzamento del download dei voti IMDb (byte scompattati).
    ImdbProgress { done: u64 },
    /// Film in sala per la home (indipendenti dalla griglia).
    NowPlaying { gen: u64, res: Result<Vec<Movie>, String> },
    /// Frammento dell'intro della chat (effetto macchina da scrivere).
    ChatStream { gen: u64, index: usize, chunk: String },
    /// Avanzamento della pipeline di chat: 1 interprete, 2 TMDB, 3 curatela.
    ChatStage { gen: u64, index: usize, stage: u8 },
    /// Risposta di chat completata (o fallita).
    ChatDone { gen: u64, payload: Box<ChatPayload> },
    /// Esito del controllo del manifesto aggiornamenti.
    UpdateChecked(Result<Option<update::Manifest>, String>),
    /// Avanzamento del download del setup.
    UpdateProgress { done: u64, total: u64 },
    /// Setup scaricato e verificato (o errore).
    UpdateDownloaded(Result<PathBuf, String>),
    /// Codice dispositivo SIMKL ottenuto (o errore).
    SimklCode {
        gen: u64,
        res: Result<simkl::DeviceCode, String>,
    },
    /// Autorizzazione SIMKL completata (o errore).
    SimklAuth {
        gen: u64,
        res: Result<simkl::Tokens, String>,
    },
    /// Sincronizzazione libreria SIMKL completata.
    SimklSynced {
        gen: u64,
        res: Result<(simkl::Store, simkl::Tokens, simkl::SyncReport), String>,
    },
    /// Scrittura su SIMKL completata.
    SimklWritten {
        gen: u64,
        res: Result<(simkl::Tokens, simkl::WriteOutcome), String>,
    },
    /// Poster TMDB risolto per una card SIMKL della home.
    Poster { id: u64, url: Option<String> },
}

#[derive(Default)]
struct UpdateUi {
    checking: bool,
    last_check: Option<Instant>,
    available: Option<update::Manifest>,
    downloaded: Option<PathBuf>,
    progress: Option<(u64, u64)>,
    error: Option<String>,
    status: Option<String>,
    /// Ultima versione gia annunciata con un toast.
    announced: Option<String>,
}

/// Le due viste della zona centrale: la chat (casa) e la griglia (esplorazione).
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Chat,
    Griglia,
}

/// Tutto cio che serve alla vista per disegnare una risposta di chat.
struct ChatPayload {
    index: usize,
    intro: String,
    cards: Vec<chat::Card>,
    understood: String,
    limit: Option<String>,
    notes: Vec<String>,
    total: u32,
    facets: Facets,
    keyword_pairs: Vec<(String, u64)>,
    /// Contesto effettivo prima della ricerca: serve ai seguiti e al retry.
    context: Intent,
    /// La griglia puo rappresentare questi criteri senza perdere i temi.
    grid_ready: bool,
    /// Intento interpretato, riusato dalla griglia senza nuove chiamate AI.
    intent: Option<(String, Intent)>,
    error: Option<String>,
}

/// Esito della ricerca della chat: candidati reali, criteri davvero usati e
/// una spiegazione onesta quando non tutti i criteri sono coperti.
pub struct ChatPoolResult {
    pub movies: Vec<Movie>,
    pub total: u32,
    pub facets: Facets,
    pub keyword_pairs: Vec<(String, u64)>,
    pub limit: Option<String>,
    pub grid_ready: bool,
    pub reasons: HashMap<u64, String>,
    pub relevance: HashMap<u64, f32>,
}

struct PersonPool {
    result: ChatPoolResult,
    label: String,
    person: String,
}

#[derive(Clone)]
struct SettingsDraft {
    tmdb_key: String,
    provider: String,
    deepseek_key: String,
    deepseek_model: String,
    effort: String,
    openrouter_key: String,
    openrouter_model: String,
    ollama_url: String,
    ollama_model: String,
    language: String,
    simkl_client_id: String,
    simkl_hide_watched: bool,
}

impl SettingsDraft {
    fn from(cfg: &Config) -> Self {
        Self {
            tmdb_key: cfg.tmdb_api_key.clone(),
            provider: cfg.ai.provider.clone(),
            deepseek_key: cfg.ai.api_key.clone(),
            deepseek_model: cfg.ai.model.clone(),
            effort: cfg.ai.effort.clone(),
            openrouter_key: cfg.ai.openrouter_api_key.clone(),
            openrouter_model: cfg.ai.openrouter_model.clone(),
            ollama_url: cfg.ai.base_url.clone(),
            ollama_model: cfg.ai.ollama_model.clone(),
            language: cfg.language.clone(),
            simkl_client_id: cfg.simkl.client_id.clone(),
            simkl_hide_watched: cfg.simkl.hide_watched,
        }
    }

    fn ai_cfg(&self) -> crate::infra::config::AiCfg {
        crate::infra::config::AiCfg {
            provider: self.provider.clone(),
            base_url: self.ollama_url.trim().to_string(),
            model: self.deepseek_model.trim().to_string(),
            api_key: self.deepseek_key.trim().to_string(),
            effort: self.effort.trim().to_string(),
            openrouter_api_key: self.openrouter_key.trim().to_string(),
            openrouter_model: self.openrouter_model.trim().to_string(),
            ollama_model: self.ollama_model.trim().to_string(),
        }
    }

    fn can_detect(&self) -> bool {
        match self.provider.as_str() {
            "deepseek" => !self.deepseek_key.trim().is_empty(),
            "openrouter" => true,
            "ollama" => true,
            _ => false,
        }
    }
}

/// Minuscole, senza punteggiatura: il confronto fra titoli non dipende da
/// virgolette, accenti o spazi doppi.
fn normalize_title(raw: &str) -> String {
    raw.to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn last_cards(entries: &[chat::Entry]) -> Vec<chat::Card> {
    entries
        .iter()
        .rev()
        .find(|e| !e.answer.cards.is_empty())
        .map(|e| e.answer.cards.clone())
        .unwrap_or_default()
}

fn looks_like_explain(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["perche", "perché", "spiega", "spiegami", "come mai"]
        .iter()
        .any(|cue| lower.contains(cue))
}

fn looks_like_compare(text: &str) -> bool {
    let lower = text.to_lowercase();
    ["confronta", "confronto", "differenza tra", "meglio tra"]
        .iter()
        .any(|cue| lower.contains(cue))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PersonRole {
    Any,
    Director,
    Actor,
}

#[derive(Clone)]
struct PersonQuery {
    name: String,
    role: PersonRole,
}

fn is_generic_person_name(name: &str) -> bool {
    let words = normalize_title(name);
    if words.split_whitespace().count() != 1 {
        return false;
    }
    const GENERIC: [&str; 21] = [
        "film",
        "cinema",
        "commedia",
        "horror",
        "dramma",
        "thriller",
        "azione",
        "romantico",
        "fantascienza",
        "animazione",
        "documentario",
        "guerra",
        "amore",
        "famiglia",
        "cani",
        "gatti",
        "cane",
        "gatto",
        "mare",
        "spazio",
        "viaggio",
    ];
    GENERIC.contains(&words.as_str())
}

fn person_query(text: &str) -> Option<PersonQuery> {
    let lower = text.to_lowercase();
    const CUES: [(&str, PersonRole); 13] = [
        ("film diretti da ", PersonRole::Director),
        ("diretto da ", PersonRole::Director),
        ("diretti da ", PersonRole::Director),
        ("regia di ", PersonRole::Director),
        ("regista ", PersonRole::Director),
        ("registi ", PersonRole::Director),
        ("film interpretati da ", PersonRole::Actor),
        ("interpretato da ", PersonRole::Actor),
        ("interpretati da ", PersonRole::Actor),
        ("attrice ", PersonRole::Actor),
        ("attore ", PersonRole::Actor),
        ("film con ", PersonRole::Actor),
        ("film di ", PersonRole::Any),
    ];
    for (cue, role) in CUES {
        let Some(pos) = lower.find(cue) else { continue };
        let rest = &text[pos + cue.len()..];
        let end = rest
            .find(['?', '.', ',', ';', '!', '\n'])
            .unwrap_or(rest.len());
        let mut name = rest[..end]
            .trim()
            .trim_matches(['"', '“', '”', '\'', ' '])
            .to_string();
        if let Some((first, _)) = name.split_once(" e ") {
            name = first.trim().to_string();
        }
        if name.len() < 3
            || name.len() > 80
            || is_generic_person_name(&name)
            || normalize_title(&name).is_empty()
        {
            continue;
        }
        return Some(PersonQuery { name, role });
    }
    None
}

fn choose_person<'a>(results: &'a [Person], query: &PersonQuery) -> Option<&'a Person> {
    let target = normalize_title(&query.name);
    let exact = results
        .iter()
        .filter(|p| normalize_title(&p.name) == target)
        .collect::<Vec<_>>();
    let pool = if exact.is_empty() {
        results
            .iter()
            .filter(|p| {
                let name = normalize_title(&p.name);
                target.len() >= 5 && (name.contains(&target) || target.contains(&name))
            })
            .collect::<Vec<_>>()
    } else {
        exact
    };
    if pool.is_empty() {
        return None;
    }
    let score = |p: &Person| {
        let role_bonus = match query.role {
            PersonRole::Director if p.known_for_department.eq_ignore_ascii_case("Directing") => 0.5,
            PersonRole::Actor if p.known_for_department.eq_ignore_ascii_case("Acting") => 0.5,
            _ => 0.0,
        };
        p.popularity + role_bonus
    };
    pool.into_iter().max_by(|a, b| {
        score(a)
            .partial_cmp(&score(b))
            .unwrap_or(std::cmp::Ordering::Equal)
    })
}

fn seen_title_in_query(text: &str) -> Option<String> {
    let lower = text.to_lowercase();
    const CUES: [&str; 6] = [
        "ho gia visto ",
        "ho già visto ",
        "l'ho gia visto ",
        "l'ho già visto ",
        "ho visto ",
        "l'ho visto ",
    ];
    for cue in CUES {
        let Some(pos) = lower.find(cue) else { continue };
        let rest = &text[pos + cue.len()..];
        let end = rest
            .find([',', '.', '?', ';', '!', '\n'])
            .unwrap_or(rest.len());
        let mut title = rest[..end].trim().trim_matches(['"', '“', '”', '\'', ' ']).to_string();
        if let Some((first, _)) = title.split_once(" e ") {
            title = first.trim().to_string();
        }
        if title.len() >= 3 {
            return Some(title);
        }
    }
    None
}

fn cards_mentioned(entries: &[chat::Entry], text: &str) -> Vec<chat::Card> {
    let hay = normalize_title(text);
    let mut found: Vec<(usize, chat::Card)> = Vec::new();
    for card in entries
        .iter()
        .flat_map(|e| e.answer.cards.iter())
    {
        let title = normalize_title(&card.movie.title);
        if title.len() < 4 {
            continue;
        }
        if let Some(pos) = hay.find(&title) {
            if !found.iter().any(|(_, c)| c.movie.id == card.movie.id) {
                found.push((pos, card.clone()));
            }
        }
    }
    found.sort_by_key(|(pos, _)| *pos);
    found.into_iter().map(|(_, card)| card).collect()
}

fn explanation_text(tax: &Taxonomy, card: &chat::Card) -> String {
    let mut lines: Vec<String> = Vec::new();
    let year = card
        .movie
        .year
        .map(|y| format!(" ({y})"))
        .unwrap_or_default();
    lines.push(format!("«{}»{}", card.movie.title, year));
    if let Some(meta) = &card.meta {
        let mut facts: Vec<String> = Vec::new();
        if let Some(dir) = &meta.director {
            facts.push(format!("regia di {dir}"));
        }
        if let Some(runtime) = meta.runtime {
            facts.push(format!("{runtime} min"));
        }
        if !facts.is_empty() {
            lines.push(facts.join(" · "));
        }
    }
    let genres = card
        .movie
        .genre_ids
        .iter()
        .filter_map(|g| tax.genre_name(*g))
        .take(3)
        .collect::<Vec<_>>();
    if !genres.is_empty() {
        lines.push(genres.join(" · "));
    }
    if card.movie.votes > 0 {
        lines.push(format!(
            "TMDB {:.1} su {} voti.",
            card.movie.rating,
            theme::thousands(card.movie.votes)
        ));
    }
    if !card.perche.trim().is_empty() {
        lines.push(card.perche.trim().to_string());
    }
    if !card.consiglio.trim().is_empty() {
        lines.push(format!("Consiglio: {}", card.consiglio.trim()));
    }
    if let Some(meta) = &card.meta {
        if !meta.flatrate.is_empty() {
            lines.push(format!(
                "In abbonamento su {}.",
                meta.flatrate.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            ));
        } else if !meta.rent.is_empty() {
            lines.push(format!(
                "A noleggio su {}.",
                meta.rent.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    lines.join("\n")
}

fn comparison_text(tax: &Taxonomy, a: &chat::Card, b: &chat::Card) -> String {
    let line = |card: &chat::Card| -> String {
        let mut bits = vec![format!("«{}»", card.movie.title)];
        if let Some(year) = card.movie.year {
            bits.push(format!("{year}"));
        }
        if let Some(meta) = &card.meta {
            if let Some(dir) = &meta.director {
                bits.push(format!("di {dir}"));
            }
            if let Some(runtime) = meta.runtime {
                bits.push(format!("{runtime} min"));
            }
        }
        if card.movie.votes > 0 {
            bits.push(format!(
                "TMDB {:.1} ({} voti)",
                card.movie.rating,
                theme::thousands(card.movie.votes)
            ));
        }
        let genres = card
            .movie
            .genre_ids
            .iter()
            .filter_map(|g| tax.genre_name(*g))
            .take(3)
            .collect::<Vec<_>>();
        if !genres.is_empty() {
            bits.push(genres.join(" · "));
        }
        format!("• {}", bits.join(" · "))
    };
    let mut lines = vec![
        "Ecco i dati che ho, senza inventare altro:".to_string(),
        line(a),
        line(b),
    ];
    let mut notes: Vec<String> = Vec::new();
    if (a.movie.rating - b.movie.rating).abs() >= 0.2 && a.movie.votes > 0 && b.movie.votes > 0 {
        let (better, worse) = if a.movie.rating > b.movie.rating {
            (a, b)
        } else {
            (b, a)
        };
        notes.push(format!(
            "«{}» ha il voto TMDB più alto ({:.1} contro {:.1}).",
            better.movie.title, better.movie.rating, worse.movie.rating
        ));
    }
    let runtime = |card: &chat::Card| card.meta.as_ref().and_then(|m| m.runtime);
    if let (Some(ra), Some(rb)) = (runtime(a), runtime(b)) {
        if ra.abs_diff(rb) >= 15 {
            let short = if ra < rb { a } else { b };
            notes.push(format!(
                "«{}» è più corto ({} min contro {}).",
                short.movie.title,
                ra.min(rb),
                ra.max(rb)
            ));
        }
    }
    if a.movie.votes > 0 && b.movie.votes > 0 && a.movie.votes.abs_diff(b.movie.votes) >= 1000 {
        let (popular, other) = if a.movie.votes > b.movie.votes {
            (a, b)
        } else {
            (b, a)
        };
        notes.push(format!(
            "«{}» ha molti più voti ({} contro {}).",
            popular.movie.title,
            theme::thousands(popular.movie.votes),
            theme::thousands(other.movie.votes)
        ));
    }
    if notes.is_empty() {
        notes.push("Sono due proposte vicine: scegli in base al genere o alla durata.".into());
    }
    lines.push(notes.join(" "));
    lines.join("\n")
}

/// Se il modello scritto non esiste più, sceglie il predefinito del provider.
fn preferred_model(provider: &str, current: &str, models: &[ModelInfo]) -> Option<String> {
    if models.iter().any(|m| m.id == current.trim()) {
        return None;
    }
    let preferred = match provider {
        "deepseek" => DEEPSEEK_MODEL,
        "openrouter" => OPENROUTER_MODEL,
        _ => "",
    };
    models
        .iter()
        .find(|m| m.id == preferred)
        .or_else(|| models.first())
        .map(|m| m.id.clone())
}

fn effort_picker(ui: &mut egui::Ui, effort: &mut String, model: Option<&ModelInfo>) {
    const LEVELS: [(&str, &str); 7] = [
        ("max", "Massimo"),
        ("xhigh", "Molto alto"),
        ("high", "Alto"),
        ("medium", "Medio"),
        ("low", "Basso"),
        ("minimal", "Minimo"),
        ("none", "Disattivato"),
    ];
    let support = model.and_then(|m| m.reasoning.as_ref());
    if model.is_some() && support.is_none() {
        ui.label(
            egui::RichText::new(
                "Questo modello non espone lo sforzo di ragionamento: usa il suo comportamento predefinito.",
            )
            .size(10.0)
            .color(theme::DIM),
        );
        return;
    }
    let allowed = |id: &str| match support {
        // DeepSeek diretto: i cinque livelli storici.
        None => matches!(id, "max" | "high" | "medium" | "low" | "none"),
        Some(s) => {
            (s.efforts.is_empty() || s.efforts.iter().any(|e| e == id))
                && !(id == "none" && s.mandatory)
        }
    };
    if let Some(s) = support {
        // Il livello salvato potrebbe non esistere per questo modello.
        if !allowed(effort) {
            let next = s
                .default_effort
                .as_deref()
                .filter(|d| allowed(d))
                .or_else(|| LEVELS.iter().map(|(id, _)| *id).find(|id| allowed(id)));
            if let Some(next) = next {
                *effort = next.to_string();
            }
        }
    }
    ui.horizontal_wrapped(|ui| {
        ui.label(
            egui::RichText::new("Sforzo di ragionamento")
                .size(11.0)
                .color(theme::DIM),
        );
        for (id, label) in LEVELS {
            if allowed(id) {
                ui.selectable_value(
                    effort,
                    id.to_string(),
                    egui::RichText::new(label).size(11.0),
                );
            }
        }
    });
    ui.label(
        egui::RichText::new(if support.is_some() {
            "Il modello dichiara solo i livelli qui sopra. Massimo costa più token ed è più lento."
        } else {
            "Massimo ragiona di più: risposte più curate, ma più lente e più token."
        })
        .size(10.0)
        .color(theme::DIM),
    );
}

fn model_picker(ui: &mut egui::Ui, field: &mut String, models: &[ModelInfo], hint: &str) {
    if models.is_empty() {
        ui.add(
            egui::TextEdit::singleline(field)
                .hint_text(hint)
                .desired_width(f32::INFINITY),
        );
        ui.label(
            egui::RichText::new(
                "Premi «Rileva modelli» per vedere l'elenco completo di quelli usabili con questa chiave.",
            )
            .size(10.0)
            .color(theme::DIM),
        );
        return;
    }
    let selected = models
        .iter()
        .find(|m| m.id == field.trim())
        .map(|m| m.menu_label())
        .unwrap_or_else(|| field.clone());
    egui::ComboBox::from_id_salt("model_pick")
        .selected_text(egui::RichText::new(selected).size(11.5))
        .width(ui.available_width())
        .height(380.0)
        .show_ui(ui, |ui| {
            for m in models {
                if ui
                    .selectable_label(
                        field.trim() == m.id,
                        egui::RichText::new(m.menu_label()).size(11.5),
                    )
                    .clicked()
                {
                    *field = m.id.clone();
                }
            }
        });
    if let Some(m) = models.iter().find(|m| m.id == field.trim()) {
        ui.label(
            egui::RichText::new(model_details(m))
                .size(10.0)
                .color(theme::DIM),
        );
    }
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new("oppure id a mano:")
                .size(10.0)
                .color(theme::DIM),
        );
        ui.add(egui::TextEdit::singleline(field).desired_width(230.0));
    });
}

pub struct App {
    ctx: egui::Context,
    tax: Arc<Taxonomy>,
    tmdb: Option<Arc<Tmdb>>,
    provider: Arc<dyn IntentProvider>,
    cfg: Config,
    /// «gg/mm hh:mm» del binario in esecuzione: si vede subito se la finestra
    /// in Impostazioni si distingue subito la build appena fatta da una vecchia.
    build: String,
    rx: Receiver<Msg>,
    tx: Sender<Msg>,
    gen: u64,
    images: ImageStore,

    /// Vista centrale: la chat (default) o la griglia.
    mode: ViewMode,
    chat: Vec<chat::Entry>,
    chat_gen: u64,
    chat_busy: bool,
    /// Token della risposta in corso: stop e «Nuova chat» lo alzano.
    chat_cancel: Option<Arc<AtomicBool>>,
    /// Contesto dei seguiti della chat: separato dai filtri della griglia.
    chat_context: Intent,
    /// Dopo un invio la vista torna in fondo, anche se si era scrollato su.
    chat_jump: bool,

    /// Bozza del composer della chat: separata dalla ricerca della griglia.
    chat_draft: String,
    /// Quattro spunti locali della casa, rigenerati dai filtri attivi.
    suggestions: Vec<suggestions::Suggestion>,
    /// Catalogo gia' validato per lo stato corrente dei filtri.
    suggestion_pool: Vec<suggestions::Suggestion>,
    suggestion_base: Facets,
    suggestion_cycle: u64,
    /// Istante della comparsa e prossimo cambio (secondi di egui).
    suggestion_shown_at: f64,
    suggestion_next_at: f64,
    /// Se il mouse e' su una card, la rotazione aspetta.
    suggestion_hovered: bool,
    /// Film in sala per la home: indipendenti dai risultati della griglia.
    home_now_playing: Vec<Movie>,
    home_now_playing_busy: bool,
    home_now_playing_next: Option<Instant>,
    home_now_playing_gen: u64,
    /// Ricerca TMDB della griglia: resta com'e quando si passa alla chat.
    grid_query: String,
    /// Ricerca dentro il pannello filtri (non tocca la ricerca TMDB).
    facet_query: String,
    /// Guida alle correnti aperta in una finestra dedicata.
    guide_open: bool,
    /// Pannello filtri visibile; il toggle sta nella barra di comando.
    filters_open: bool,
    facets: Facets,
    prune: Prune,
    messages: Vec<String>,
    info: Vec<String>,
    ai_notes: Vec<String>,

    movies: Vec<Movie>,
    total: u32,
    total_pages: u32,
    page: u32,
    /// La prossima griglia parte dall'alto: nuova ricerca, filtri o pagina.
    grid_reset_scroll: bool,
    loading: bool,
    headline: String,
    /// Ultimo esito di rete: None = non ancora provato, Some(false) = offline.
    net: Option<bool>,
    /// Compromessi «Sblocca» per la ricerca corrente, se la griglia è quasi vuota.
    unlocks: Vec<unlock::Unlock>,

    detail: Option<Details>,
    detail_loading: bool,
    /// Generazioni delle richieste scheda: una risposta vecchia non
    /// deve sovrascrivere il film aperto dopo.
    detail_gen: u64,

    keyword_map: Vec<(String, u64)>,
    dirty: Option<Instant>,
    /// Widget che deve ricevere il fuoco al prossimo frame (composer o ricerca).
    focus: Option<ViewMode>,
    show_settings: bool,
    settings: SettingsDraft,
    settings_check: Option<String>,
    /// Collegamento SIMKL: token (nel Credential Manager), libreria locale,
    /// codice di autorizzazione in corso e stato dell'ultima sincronizzazione.
    simkl_tokens: Option<simkl::Tokens>,
    simkl_store: simkl::Store,
    /// Picks della home calcolate quando lo store cambia, non a ogni frame.
    simkl_cards: Option<(Vec<simkl::Playback>, Vec<simkl::TrackedMovie>)>,
    /// Poster TMDB delle card SIMKL della home: None = in caricamento.
    poster_cache: HashMap<u64, Option<String>>,
    simkl_auth: Option<simkl::DeviceCode>,
    simkl_cancel: Arc<AtomicBool>,
    /// Cambia a ogni collegamento/ scollegamento: i risultati vecchi non
    /// devono ricollegare l'account o sovrascrivere lo store.
    simkl_epoch: u64,
    simkl_syncing: bool,
    simkl_busy: bool,
    /// Scrittura in corso, per aggiornare subito lo stato locale.
    simkl_pending: Option<(u64, simkl::WriteOp)>,
    simkl_status: Option<String>,
    simkl_error: Option<String>,
    update: UpdateUi,
    models: Vec<ModelInfo>,
    models_gen: u64,
    models_status: Option<String>,
    detecting: bool,
    ai_check: Option<String>,
    ai_checking: bool,
    ai_cache: Option<(String, Intent)>,
    toast: Option<(String, Instant)>,
    boot_done: bool,

    imdb_busy: bool,
    imdb_progress: Option<(u64, u64)>,
    imdb_status: Option<String>,
    /// I font CJK (titoli giapponesi, coreani) si caricano una volta sola.
    cjk_loaded: bool,
}

impl App {
    pub fn new(ctx: &egui::Context, cfg: Config) -> Self {
        let tax = Arc::new(Taxonomy::load());
        let provider = build_provider(&cfg.ai);
        let tmdb = Self::build_tmdb(&cfg);
        let (tx, rx) = channel();
        let facets = Facets::default();
        let prune = constraints::pruning(&tax, &facets);
        let settings = SettingsDraft::from(&cfg);
        let simkl_tokens = simkl::load_tokens().ok().flatten();
        let simkl_store = simkl::load_store();
        let mut app = Self {
            ctx: ctx.clone(),
            tax,
            tmdb,
            provider,
            cfg,
            build: winclock::build_stamp(),
            rx,
            tx,
            gen: 0,
            images: ImageStore::new(),
            mode: ViewMode::Chat,
            chat: Vec::new(),
            chat_gen: 0,
            chat_busy: false,
            chat_cancel: None,
            chat_context: Intent::default(),
            chat_jump: false,
            chat_draft: String::new(),
            suggestions: Vec::new(),
            suggestion_pool: Vec::new(),
            suggestion_base: Facets::default(),
            suggestion_cycle: 0,
            suggestion_shown_at: 0.0,
            suggestion_next_at: 0.0,
            suggestion_hovered: false,
            home_now_playing: Vec::new(),
            home_now_playing_busy: false,
            home_now_playing_next: None,
            home_now_playing_gen: 0,
            grid_query: String::new(),
            facet_query: String::new(),
            guide_open: false,
            filters_open: true,
            facets,
            prune,
            messages: Vec::new(),
            info: Vec::new(),
            ai_notes: Vec::new(),
            movies: Vec::new(),
            total: 0,
            total_pages: 0,
            page: 1,
            grid_reset_scroll: false,
            loading: false,
            headline: String::new(),
            net: None,
            unlocks: Vec::new(),
            detail: None,
            detail_loading: false,
            detail_gen: 0,
            keyword_map: Vec::new(),
            dirty: None,
            focus: None,
            show_settings: false,
            settings,
            settings_check: None,
            simkl_tokens,
            simkl_store,
            simkl_cards: None,
            poster_cache: HashMap::new(),
            simkl_auth: None,
            simkl_cancel: Arc::new(AtomicBool::new(false)),
            simkl_epoch: 0,
            simkl_syncing: false,
            simkl_busy: false,
            simkl_pending: None,
            simkl_status: None,
            simkl_error: None,
            update: UpdateUi::default(),
            models: Vec::new(),
            models_gen: 0,
            models_status: None,
            detecting: false,
            ai_check: None,
            ai_checking: false,
            ai_cache: None,
            toast: None,
            boot_done: false,
            imdb_busy: false,
            imdb_progress: None,
            imdb_status: None,
            cjk_loaded: false,
        };
        // Se c'e gia un servizio AI configurato, l'elenco modelli si carica da solo
        // all'avvio: aprendo le Impostazioni lo trovi pronto.
        if app.settings.can_detect() {
            app.detect_models();
        }
        // La libreria SIMKL si aggiorna all'avvio, ma non più di una volta
        // ogni 15 minuti: niente polling in background.
        if app.simkl_tokens.is_some() && simkl::sync_due(app.simkl_store.last_sync) {
            app.start_simkl_sync(false);
        }
        app
    }

    fn build_tmdb(cfg: &Config) -> Option<Arc<Tmdb>> {
        if cfg.has_key() {
            Some(Arc::new(Tmdb::new(
                cfg.tmdb_api_key.trim().to_string(),
                cfg.language.clone(),
            )))
        } else {
            None
        }
    }

    // ----------------------------------------------------------- messaggi ----

    fn poll(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Results { gen, payload } => {
                    if gen != self.gen {
                        continue;
                    }
                    self.loading = false;
                    let mut p = *payload;
                    let net_ok = p.error.is_none();
                    if self.cfg.simkl.hide_watched {
                        let before = p.movies.len();
                        p.movies.retain(|m| {
                            self.simkl_store
                                .get(m.id)
                                .map(|t| t.status != "completed")
                                .unwrap_or(true)
                        });
                        let hidden = before - p.movies.len();
                        if hidden > 0 {
                            p.total = p.total.saturating_sub(hidden as u32);
                            p.info
                                .push(format!("{hidden} film già visti nascosti."));
                        }
                    }
                    self.facets = p.facets;
                    self.prune = constraints::pruning(&self.tax, &self.facets);
                    self.messages = p.removed;
                    self.info = p.info;
                    self.ai_notes = p.ai_notes;
                    for pair in p.keyword_pairs {
                        if !self.keyword_map.iter().any(|(k, _)| k == &pair.0) {
                            self.keyword_map.push(pair);
                        }
                    }
                    self.headline = p.headline;
                    if p.intent.is_some() {
                        self.ai_cache = p.intent;
                    }
                    self.net = Some(net_ok);
                    if let Some(err) = p.error {
                        self.movies.clear();
                        self.total = 0;
                        self.total_pages = 0;
                        self.toast(err);
                    } else {
                        self.total = p.total;
                        self.total_pages = p.total_pages;
                        if self.total_pages <= 1 {
                            self.page = 1;
                        }
                        self.movies = p.movies;
                        self.ensure_fonts_for_results();
                        self.apply_track_flags();
                        if self.movies.is_empty() {
                            self.toast("Nessun risultato: prova ad allargare i filtri.");
                        }
                    }
                }
                Msg::Unlocks { gen, unlocks } => {
                    if gen == self.gen {
                        self.unlocks = unlocks;
                        self.ctx.request_repaint();
                    }
                }
                Msg::Details { gen, res } => {
                    if gen != self.detail_gen {
                        continue;
                    }
                    self.detail_loading = false;
                    // Un 404 o un JSON rotto non significano "offline": il
                    // badge lo aggiorna solo una risposta riuscita.
                    if res.is_ok() {
                        self.net = Some(true);
                    }
                    match res {
                        Ok(d) => {
                            {
                                let texts: Vec<&str> = std::iter::once(d.movie.title.as_str())
                                    .chain(std::iter::once(d.movie.original_title.as_str()))
                                    .chain(d.recommendations.iter().map(|m| m.title.as_str()))
                                    .collect();
                                crate::ui::fonts::ensure_cjk(
                                    &self.ctx,
                                    &mut self.cjk_loaded,
                                    &texts,
                                );
                            }
                            self.detail = Some(d);
                            self.apply_track_flags();
                        }
                        Err(e) => self.toast(e),
                    }
                }
                Msg::Check { res } => {
                    if res.is_ok() {
                        self.net = Some(true);
                    }
                    self.settings_check = Some(match res {
                        Ok(m) => format!("✔ {m}"),
                        Err(e) => format!("✕ {e}"),
                    });
                }
                Msg::Models { gen, res } => {
                    if gen != self.models_gen {
                        continue;
                    }
                    self.detecting = false;
                    match res {
                        Ok(models) => {
                            let count = models.len();
                            let provider = self.settings.provider.clone();
                            let current = match provider.as_str() {
                                "deepseek" => self.settings.deepseek_model.clone(),
                                "openrouter" => self.settings.openrouter_model.clone(),
                                "ollama" => self.settings.ollama_model.clone(),
                                _ => String::new(),
                            };
                            if let Some(id) = preferred_model(&provider, &current, &models) {
                                match provider.as_str() {
                                    "deepseek" => self.settings.deepseek_model = id,
                                    "openrouter" => self.settings.openrouter_model = id,
                                    "ollama" => self.settings.ollama_model = id,
                                    _ => {}
                                }
                            }
                            self.models_status =
                                Some(format!("✔ {count} modelli disponibili"));
                            self.models = models;
                        }
                        Err(e) => {
                            self.models_status = Some(format!("✕ {e}"));
                            self.models.clear();
                        }
                    }
                }
                Msg::AiCheck { res } => {
                    self.ai_checking = false;
                    self.ai_check = Some(match res {
                        Ok(m) => format!("✔ {m}"),
                        Err(e) => format!("✕ {e}"),
                    });
                }
                Msg::Imdb { res } => {
                    self.imdb_busy = false;
                    self.imdb_progress = None;
                    if res.is_ok() {
                        if let Some(tmdb) = &self.tmdb {
                            tmdb.clear_details_cache();
                        }
                    }
                    self.imdb_status = Some(match res {
                        Ok(m) => format!("✔ {m}"),
                        Err(e) => format!("✕ {e}"),
                    });
                }
                Msg::ImdbProgress { done } => {
                    if self.imdb_busy {
                        self.imdb_progress = Some((done, 0));
                    }
                }
                Msg::NowPlaying { gen, res } => {
                    if gen != self.home_now_playing_gen {
                        continue;
                    }
                    self.home_now_playing_busy = false;
                    match res {
                        Ok(movies) => {
                            if !movies.is_empty() {
                                self.home_now_playing = movies;
                            }
                            self.home_now_playing_next =
                                Some(Instant::now() + IN_SALA_REFRESH);
                        }
                        Err(_) => {
                            self.home_now_playing_next = Some(Instant::now() + IN_SALA_RETRY);
                        }
                    }
                }
                Msg::UpdateChecked(res) => {
                    self.update.checking = false;
                    self.update.last_check = Some(Instant::now());
                    match res {
                        Ok(Some(m)) => {
                            let announce =
                                self.update.announced.as_deref() != Some(m.version.as_str());
                            self.update.status =
                                Some(format!("Disponibile la versione {}.", m.version));
                            self.update.error = None;
                            self.update.downloaded = None;
                            self.update.progress = None;
                            self.update.available = Some(m.clone());
                            if announce {
                                self.update.announced = Some(m.version.clone());
                                self.toast(format!(
                                    "CineMatch {} è disponibile: aggiorna dalle Impostazioni.",
                                    m.version
                                ));
                            }
                        }
                        Ok(None) => {
                            self.update.available = None;
                            self.update.downloaded = None;
                            self.update.progress = None;
                            self.update.error = None;
                            self.update.status = Some("Sei alla versione più recente.".into());
                        }
                        Err(e) => {
                            self.update.error = Some(format!("Controllo non riuscito: {e}"));
                            self.update.status = None;
                        }
                    }
                }
                Msg::UpdateProgress { done, total } => {
                    self.update.progress = Some((done, total));
                    self.update.downloaded = None;
                    self.update.error = None;
                }
                Msg::UpdateDownloaded(res) => {
                    self.update.progress = None;
                    match res {
                        Ok(path) => {
                            self.update.downloaded = Some(path);
                            self.update.error = None;
                            self.update.status =
                                Some("Aggiornamento pronto per l'installazione.".into());
                        }
                        Err(e) => {
                            self.update.error = Some(e.clone());
                            self.update.status = None;
                            self.toast(e);
                        }
                    }
                }
                Msg::SimklCode { gen, res } => {
                    if gen != self.simkl_epoch {
                        continue;
                    }
                    match res {
                        Ok(code) => {
                            self.simkl_auth = Some(code);
                            self.simkl_status = None;
                            self.simkl_error = None;
                        }
                        Err(e) => {
                            self.simkl_auth = None;
                            self.simkl_status = None;
                            self.simkl_error = Some(e);
                        }
                    }
                }
                Msg::SimklAuth { gen, res } => {
                    if gen != self.simkl_epoch {
                        continue;
                    }
                    self.simkl_auth = None;
                    match res {
                        Ok(tokens) => {
                            if let Err(e) = simkl::save_tokens(&tokens) {
                                self.simkl_status = None;
                                self.simkl_error = Some(e);
                                continue;
                            }
                            self.simkl_tokens = Some(tokens);
                            self.simkl_error = None;
                            self.simkl_status =
                                Some("SIMKL collegato. Sincronizzo la libreria…".into());
                            self.start_simkl_sync(true);
                        }
                        Err(e) => {
                            self.simkl_status = None;
                            self.simkl_error = Some(e);
                        }
                    }
                }
                Msg::SimklSynced { gen, res } => {
                    if gen != self.simkl_epoch {
                        // Un refresh puo' aver salvato token vecchi dopo lo
                        // scollegamento: riallinea il Credential Manager.
                        self.restore_simkl_tokens();
                        continue;
                    }
                    self.simkl_syncing = false;
                    match res {
                        Ok((store, tokens, _report)) => {
                            let changed = self
                                .simkl_tokens
                                .as_ref()
                                .map(|t| {
                                    t.access_token != tokens.access_token
                                        || t.refresh_token != tokens.refresh_token
                                })
                                .unwrap_or(true);
                            if changed {
                                if let Err(e) = simkl::save_tokens(&tokens) {
                                    self.simkl_error = Some(e);
                                }
                            }
                            self.simkl_tokens = Some(tokens);
                            if let Err(e) = simkl::save_store(&store) {
                                self.simkl_error = Some(e);
                            }
                            self.simkl_store = store;
                            self.simkl_cards = None;
                            self.simkl_status = None;
                            self.apply_track_flags();
                        }
                        Err(e) => {
                            self.simkl_status = None;
                            if e.contains("ricollega") || e.contains("rifiutato") {
                                self.simkl_tokens = None;
                                let _ = simkl::clear_tokens();
                            } else {
                                // Il refresh puo' aver salvato token nuovi prima
                                // che una richiesta successiva fallisse.
                                self.reload_simkl_tokens();
                            }
                            self.simkl_error = Some(e);
                        }
                    }
                }
                Msg::SimklWritten { gen, res } => {
                    if gen != self.simkl_epoch {
                        self.restore_simkl_tokens();
                        continue;
                    }
                    self.simkl_busy = false;
                    let pending = self.simkl_pending.take();
                    match res {
                        Ok((tokens, outcome)) => {
                            let changed = self
                                .simkl_tokens
                                .as_ref()
                                .map(|t| {
                                    t.access_token != tokens.access_token
                                        || t.refresh_token != tokens.refresh_token
                                })
                                .unwrap_or(true);
                            if changed {
                                if let Err(e) = simkl::save_tokens(&tokens) {
                                    self.simkl_error = Some(e);
                                }
                            }
                            self.simkl_tokens = Some(tokens);
                            self.simkl_error = None;
                            self.simkl_status = Some("Salvato su SIMKL.".into());
                            if let Some((id, op)) = pending {
                                self.patch_track(id, op, &outcome);
                            }
                            self.start_simkl_sync(false);
                        }
                        Err(e) => {
                            self.simkl_status = None;
                            if e.contains("ricollega") || e.contains("rifiutato") {
                                self.simkl_tokens = None;
                                let _ = simkl::clear_tokens();
                            } else {
                                self.reload_simkl_tokens();
                            }
                            self.simkl_error = Some(e);
                        }
                    }
                }
                Msg::Poster { id, url } => {
                    self.poster_cache.insert(id, url);
                    self.ctx.request_repaint();
                }
                Msg::ChatStream { gen, index, chunk } => {
                    if gen != self.chat_gen {
                        continue;
                    }
                    if let Some(entry) = self.chat.get_mut(index) {
                        if entry.answer.state == chat::State::Busy {
                            entry.answer.intro.push_str(&chunk);
                        }
                    }
                }
                Msg::ChatStage { gen, index, stage } => {
                    if gen != self.chat_gen {
                        continue;
                    }
                    if let Some(entry) = self.chat.get_mut(index) {
                        if entry.answer.state == chat::State::Busy {
                            entry.answer.stage = entry.answer.stage.max(stage);
                        }
                    }
                }
                Msg::ChatDone { gen, payload } => {
                    if gen != self.chat_gen {
                        continue;
                    }
                    self.chat_busy = false;
                    // Dopo la risposta il campo torna pronto: l'utente puo'
                    // scrivere subito senza doverlo cliccare.
                    if self.mode == ViewMode::Chat && !self.show_settings && !self.guide_open {
                        self.focus = Some(ViewMode::Chat);
                    }
                    let mut p = *payload;
                    if self.cfg.simkl.hide_watched {
                        let before = p.cards.len();
                        p.cards.retain(|c| {
                            self.simkl_store
                                .get(c.movie.id)
                                .map(|t| t.status != "completed")
                                .unwrap_or(true)
                        });
                        let hidden = before - p.cards.len();
                        if hidden > 0 {
                            p.notes
                                .push(format!("{hidden} film già visti nascosti."));
                        }
                    }
                    // La chat non tocca la griglia: l'unico passaggio esplicito
                    // e «Vedi tutti in griglia». Qui si aggiorna solo il
                    // contesto interno dei seguiti.
                    self.chat_context = p.context.clone();
                    for pair in p.keyword_pairs.clone() {
                        if !self.keyword_map.iter().any(|(k, _)| k == &pair.0) {
                            self.keyword_map.push(pair);
                        }
                    }
                    let titles: Vec<String> = p
                        .cards
                        .iter()
                        .flat_map(|c| [c.movie.title.clone(), c.movie.original_title.clone()])
                        .collect();
                    let text_refs: Vec<&str> = titles.iter().map(|t| t.as_str()).collect();
                    crate::ui::fonts::ensure_cjk(&self.ctx, &mut self.cjk_loaded, &text_refs);
                    if let Some(entry) = self.chat.get_mut(p.index) {
                        entry.answer.state = chat::State::Done;
                        entry.answer.intro = p.intro;
                        entry.answer.understood = p.understood;
                        entry.answer.limit = p.limit;
                        entry.answer.notes = p.notes;
                        entry.answer.cards = p.cards;
                        entry.answer.total = p.total;
                        entry.answer.error = p.error;
                        entry.answer.facets = p.facets;
                        entry.answer.keyword_pairs = p.keyword_pairs;
                        entry.answer.context = Some(p.context);
                        entry.answer.grid_ready = p.grid_ready;
                        entry.answer.intent = p.intent;
                        entry.answer.done_at = self.ctx.input(|i| i.time);
                    }
                    self.apply_track_flags();
                    self.ctx.request_repaint();
                }
            }
        }
    }

    fn tick(&mut self) {        if let Some(at) = self.dirty {
            if at.elapsed() >= Duration::from_millis(320) {
                // Nota: questa via non chiama l'AI (le facce sono gia state scelte a
                // mano), quindi il ritardo resta breve anche con un modello attivo.
                self.dirty = None;
                self.page = 1;
                self.launch_search(false);
            } else {
                self.ctx.request_repaint_after(Duration::from_millis(70));
            }
        }
    }

    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }

    /// Se tra i risultati ci sono titoli in scritture non latine (giapponese,
    /// coreano), carica i font di sistema che le coprono: una volta sola.
    fn ensure_fonts_for_results(&mut self) {
        let mut texts: Vec<&str> = self
            .movies
            .iter()
            .flat_map(|m| [m.title.as_str(), m.original_title.as_str()])
            .collect();
        for entry in &self.chat {
            for card in &entry.answer.cards {
                texts.push(card.movie.title.as_str());
                texts.push(card.movie.original_title.as_str());
                texts.push(card.movie.overview.as_str());
            }
        }
        crate::ui::fonts::ensure_cjk(&self.ctx, &mut self.cjk_loaded, &texts);
    }

    // ----------------------------------------------------------- ricerca -----

    fn spawn_search(&mut self, with_ai: bool) {
        let Some(tmdb) = self.tmdb.clone() else {
            self.show_settings = true;
            self.toast("Inserisci la chiave API TMDB per cercare.");
            return;
        };
        self.gen += 1;
        let gen = self.gen;
        self.loading = true;
        self.grid_reset_scroll = true;
        self.unlocks.clear();

        let text = self
            .grid_query
            .trim()
            .chars()
            .take(MAX_QUERY_CHARS)
            .collect::<String>();
        let base = self.facets.clone();
        let known = self.keyword_map.clone();
        let cached = self.ai_cache.clone();
        let tax = self.tax.clone();
        let provider = self.provider.clone();
        let page = self.page;
        let tx = self.tx.clone();

        std::thread::spawn(move || {
            let mut intent: Option<Intent> = None;
            let mut fresh = false;
            if with_ai && !text.is_empty() {
                // Riusa l'interpretazione gia fatta per la stessa frase: nessuna
                // chiamata AI ripetuta (le pagine successive sono gratis).
                if let Some((cached_text, cached_intent)) = &cached {
                    if cached_text == &text {
                        intent = Some(cached_intent.clone());
                    }
                }
                if intent.is_none() {
                    if let Ok(it) = provider.parse(&text, &tax) {
                        intent = Some(it);
                        fresh = true;
                    }
                }
            }
            let mut eff = base.clone();
            if let Some(it) = &intent {
                merge_facets(&mut eff, &it.facets, &it.cleared);
            }
            let (eff, removed) = constraints::sanitize(&tax, &eff);

            let mut title = intent.as_ref().and_then(|i| i.free_text.clone());
            let similar = intent.as_ref().and_then(|i| i.similar_to.clone());
            let ai_notes = intent.as_ref().map(|i| i.notes.clone()).unwrap_or_default();

            if title.is_none()
                && similar.is_none()
                && !text.is_empty()
                && eff.is_empty()
                && intent
                    .as_ref()
                    .map(|i| i.cleared.is_empty())
                    .unwrap_or(true)
            {
                title = Some(text.clone());
            }

            let mut payload = Results {
                facets: eff.clone(),
                removed,
                info: Vec::new(),
                ai_notes,
                movies: Vec::new(),
                total: 0,
                total_pages: 0,
                headline: String::new(),
                error: None,
                keyword_pairs: Vec::new(),
                intent: None,
            };

            // Parole chiave: mood, temi liberi e parole proprie delle correnti.
            // Quello che non si risolve su TMDB si dice e si ignora, senza
            // bloccare la ricerca.
            let mut kw = known;
            resolve_keywords(
                &tmdb,
                &tax,
                &eff,
                &mut kw,
                &mut payload.keyword_pairs,
                &mut payload.info,
            );

            if fresh {
                payload.intent = intent.clone().map(|it| (text.clone(), it));
            }

            let quality_newest =
                similar.is_none() && title.is_none() && eff.sort == Sort::Newest;
            let quality_visual = similar.is_none()
                && title.is_none()
                && matches!(eff.sort, Sort::Newest | Sort::Upcoming);
            let mut discover_used = false;
            let outcome: Result<(Vec<Movie>, u32, u32, String), String> =
                if let Some(name) = similar {
                    match tmdb.search(&name, 1) {
                        Ok(p) => match p.results.first() {
                            Some(first) => {
                                let seed = first.title.clone();
                                match tmdb.similar(first.id, page) {
                                    Ok(rec) => {
                                        let movies = filter_movies(
                                            rec.results.into_iter().map(Movie::from).collect(),
                                            &tax,
                                            &eff,
                                        );
                                        let (total, pages) = if eff.is_empty() {
                                            (rec.total_results, rec.total_pages)
                                        } else {
                                            (movies.len() as u32, 1)
                                        };
                                        Ok((movies, total, pages, format!("Simili a «{seed}»")))
                                    }
                                    Err(e) => Err(e),
                                }
                            }
                            None => Err(format!("Nessun film trovato per «{name}».")),
                        },
                        Err(e) => Err(e),
                    }
                } else if let Some(t) = title {
                    if eff.is_empty() {
                        match tmdb.search(&t, page) {
                            Ok(p) => Ok((
                                p.results.into_iter().map(Movie::from).collect(),
                                p.total_results,
                                p.total_pages,
                                format!("Risultati per «{t}»"),
                            )),
                            Err(e) => Err(e),
                        }
                    } else {
                        match search_title_pages(&tmdb, &t) {
                            Ok((raw, total_results)) => {
                                let raw_len = raw.len();
                                let filtered = filter_movies(raw, &tax, &eff);
                                let filtered_len = filtered.len();
                                let limits = local_filter_limits(&tax, &eff);
                                let mut note = format!(
                                    "Risultati per «{t}» · filtri applicati localmente ({filtered_len}/{raw_len})"
                                );
                                if !limits.is_empty() {
                                    note.push_str(&format!(" · limiti: {}", limits.join(", ")));
                                }
                                if total_results as usize > raw_len {
                                    note.push_str(&format!(
                                        " · prime {TITLE_PAGE_CAP} pagine di {total_results}"
                                    ));
                                }
                                if filtered.is_empty() {
                                    payload.ai_notes.push(
                                        "La ricerca per titolo non supporta i filtri lato server: nessun risultato compatibile nelle prime pagine.".into(),
                                    );
                                }
                                Ok((filtered, filtered_len as u32, 1, note))
                            }
                            Err(e) => Err(e),
                        }
                    }
                } else if wants_trending(&eff) {
                    match tmdb.trending(page) {
                        Ok(p) => Ok((
                            p.results.into_iter().map(Movie::from).collect(),
                            p.total_results,
                            p.total_pages,
                            "Tendenze della settimana".into(),
                        )),
                        Err(e) => Err(e),
                    }
                } else {
                    discover_used = true;
                    discover_with_relaxation(&tmdb, &tax, &eff, &kw, page, &mut payload.removed)
                };

            match outcome {
                Ok((mut movies, total, pages, headline)) => {
                    if quality_visual {
                        let before = movies.len();
                        movies.retain(|m| m.poster_path.is_some());
                        if movies.len() < before {
                            payload.info.push(format!(
                                "{} titoli senza poster nascosti.",
                                before - movies.len()
                            ));
                        }
                    }
                    if quality_newest
                        && eff.runtime.is_none()
                        && eff.min_votes.is_none()
                        && eff.min_rating.is_none()
                    {
                        payload.info.push(
                            "Piu recenti: solo film usciti, con poster, durata almeno 60 minuti e 50 voti."
                                .into(),
                        );
                    }
                    if quality_visual && eff.sort == Sort::Upcoming {
                        payload.info.push(
                            "Prossimamente: ordinato per popolarita, senza schede senza poster; per i film futuri voti e durata non sono ancora affidabili."
                                .into(),
                        );
                    }
                    payload.movies = movies;
                    payload.total = total;
                    payload.total_pages = pages;
                    payload.headline = headline;
                }
                Err(e) => {
                    payload.headline = "Errore".into();
                    payload.error = Some(e);
                }
            }

            let grid_len = payload.movies.len();
            let seen = payload
                .movies
                .iter()
                .map(|m| m.id)
                .collect::<HashSet<_>>();
            let current_total = payload.total;
            let unlockable = discover_used
                && page == 1
                && payload.error.is_none()
                // Se il motore ha già allentato qualcosa, la nota spiega il
                // compromesso: qui si mostrano solo rinunce scelte dall'utente.
                && payload.removed.is_empty()
                && grid_len <= 3;
            let _ = tx.send(Msg::Results {
                gen,
                payload: Box::new(payload),
            });

            // I risultati partono subito; i compromessi arrivano dopo, perché
            // ogni variante è una chiamata TMDB. Meglio una griglia pronta e
            // la striscia «Sblocca» un istante più tardi che il contrario.
            if unlockable {
                let unlocks =
                    unlock_options(&tmdb, &tax, &eff, &kw, current_total, &seen);
                if !unlocks.is_empty() {
                    let _ = tx.send(Msg::Unlocks { gen, unlocks });
                }
            }
        });
        self.ctx.request_repaint();
    }

    // -------------------------------------------------------------- chat -----

    /// Cambia vista senza perdere lo stato dell'altra: la chat resta in
    /// memoria, la griglia conserva ricerca, filtri e larghezza del pannello.
    fn switch_mode(&mut self, mode: ViewMode) {
        if self.mode == mode {
            return;
        }
        // Il widget della vecchia vista sparisce: il testo non deve restare
        // agganciato a un campo che non c'e piu.
        if let Some(id) = self.ctx.memory(|m| m.focused()) {
            self.ctx.memory_mut(|m| m.surrender_focus(id));
        }
        self.mode = mode;
        self.focus = None;
        if mode == ViewMode::Chat {
            // La scheda e la guida sono strumenti della griglia: in chat
            // lasciano tutto lo spazio alla conversazione.
            self.close_detail();
            self.guide_open = false;
        }
    }

    /// Prepara le quattro card della casa. Se i filtri della griglia sono
    /// cambiati ricostruisce il catalogo; altrimenti fa avanzare il ciclo.
    /// Tutto avviene in memoria e con la sola tassonomia: nessuna AI, nessuna
    /// richiesta TMDB prima che l'utente invii un messaggio.
    fn refresh_home_suggestions(&mut self, now: f64) {
        let changed = self.suggestion_base != self.facets;
        if changed || self.suggestion_pool.is_empty() || self.suggestions.is_empty() {
            self.suggestion_pool = suggestions::candidates(&self.tax, &self.facets);
            self.suggestion_base = self.facets.clone();
            self.suggestion_cycle = 0;
            self.suggestions = suggestions::pick(&self.suggestion_pool, 0, &[], 4);
            self.suggestion_shown_at = now;
            self.suggestion_next_at = now + suggestions::ROTATION_SECS;
            return;
        }
        if now < self.suggestion_next_at {
            return;
        }
        if self.suggestion_hovered {
            // Non cambiare una card sotto il mouse: si riprova tra poco.
            self.suggestion_next_at = now + 0.8;
            return;
        }
        let avoid = self
            .suggestions
            .iter()
            .map(|s| s.prompt.clone())
            .collect::<Vec<_>>();
        self.suggestion_cycle = self.suggestion_cycle.wrapping_add(1);
        self.suggestions =
            suggestions::pick(&self.suggestion_pool, self.suggestion_cycle, &avoid, 4);
        self.suggestion_shown_at = now;
        self.suggestion_next_at = now + suggestions::ROTATION_SECS;
    }

    /// Aggiunge una risposta solo testo: spiegazioni e confronti non sono
    /// nuove ricerche e non devono toccare il contesto.
    fn push_text_answer(&mut self, user: String, intro: String, understood: String) {
        if self.chat_busy {
            self.toast("Attendi la risposta in corso.");
            return;
        }
        let now = self.ctx.input(|i| i.time);
        self.chat.push(chat::Entry {
            user,
            answer: chat::Answer {
                intro,
                understood,
                done_at: now,
                ..Default::default()
            },
        });
        self.chat_jump = true;
        self.suggestion_hovered = false;
        self.focus = Some(ViewMode::Chat);
    }

    fn handle_meta_command(&mut self, text: &str) -> bool {
        if looks_like_compare(text) {
            let mentioned = cards_mentioned(&self.chat, text);
            let latest = last_cards(&self.chat);
            let pair = if mentioned.len() >= 2 {
                Some((mentioned[0].clone(), mentioned[1].clone()))
            } else if let Some(first) = mentioned.first() {
                latest
                    .iter()
                    .find(|c| c.movie.id != first.movie.id)
                    .map(|second| (first.clone(), second.clone()))
            } else if latest.len() >= 2 {
                Some((latest[0].clone(), latest[1].clone()))
            } else {
                None
            };
            if let Some((a, b)) = pair {
                self.compare_cards(a, b);
            } else {
                self.push_text_answer(
                    text.to_string(),
                    "Per un confronto mi servono due film già proposti: prima chiedimi un consiglio."
                        .into(),
                    "confronto".into(),
                );
            }
            return true;
        }
        if looks_like_explain(text) {
            let mentioned = cards_mentioned(&self.chat, text);
            let latest = last_cards(&self.chat);
            let lower = text.to_lowercase();
            let frame = ["questo", "questa", "questi", "il primo", "la prima", "consigli"]
                .iter()
                .any(|cue| lower.contains(cue));
            let card = mentioned
                .into_iter()
                .next()
                .or_else(|| if frame || latest.len() == 1 { latest.first().cloned() } else { None });
            if let Some(card) = card {
                self.explain_card(card);
                return true;
            }
            if latest.is_empty() {
                self.push_text_answer(
                    text.to_string(),
                    "Prima chiedimi un consiglio: poi posso spiegarti perché ho scelto un film."
                        .into(),
                    "spiegazione".into(),
                );
                return true;
            }
        }
        false
    }

    fn explain_card(&mut self, card: chat::Card) {
        let user = format!("Perché «{}»?", card.movie.title);
        let intro = explanation_text(&self.tax, &card);
        self.push_text_answer(user, intro, "spiegazione".into());
    }

    fn compare_cards(&mut self, a: chat::Card, b: chat::Card) {
        let user = format!(
            "Confronta «{}» e «{}»",
            a.movie.title, b.movie.title
        );
        let intro = comparison_text(&self.tax, &a, &b);
        self.push_text_answer(user, intro, "confronto".into());
    }

    /// Invia il contenuto del composer come messaggio di chat.
    fn send_chat(&mut self) {
        if self.chat_busy {
            self.toast("Attendi la risposta in corso…");
            return;
        }
        let text = self.chat_draft.trim().to_string();
        if text.is_empty() {
            self.toast("Scrivi cosa vuoi vedere: anche in modo libero.");
            return;
        }
        let text = if text.chars().count() > chat::MAX_PROMPT_CHARS {
            self.toast(format!(
                "Messaggio accorciato a {} caratteri.",
                chat::MAX_PROMPT_CHARS
            ));
            text.chars().take(chat::MAX_PROMPT_CHARS).collect()
        } else {
            text
        };
        self.chat_draft.clear();
        self.chat_jump = true;
        self.suggestion_hovered = false;
        // Spiegazioni e confronti sono comandi locali: non sono nuove
        // ricerche e funzionano anche senza una chiamata AI.
        if self.handle_meta_command(&text) {
            self.focus = Some(ViewMode::Chat);
            return;
        }
        if self.tmdb.is_none() {
            self.show_settings = true;
            self.toast("Inserisci la chiave API TMDB per cercare.");
            return;
        }
        // «Altri» e simili: stessi criteri, ma senza i film gia mostrati.
        let action = classify_chat_action(&text, &self.chat_context);
        let exclude: Vec<u64> = if action == ChatAction::Replace {
            self.chat
                .iter()
                .flat_map(|e| e.answer.cards.iter().map(|c| c.movie.id))
                .collect()
        } else {
            Vec::new()
        };
        self.spawn_chat(text, exclude, None);
        // Il composer resta a fuoco: la chat e una conversazione, si resta li.
        self.focus = Some(ViewMode::Chat);
    }

    /// Ricomincia da zero: la conversazione vive in memoria, niente file.
    fn new_chat(&mut self) {
        if let Some(cancel) = self.chat_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.chat.clear();
        self.chat_gen += 1;
        self.chat_busy = false;
        self.chat_context = Intent::default();
        self.chat_draft.clear();
        self.chat_jump = false;
        self.suggestion_hovered = false;
        self.focus = Some(ViewMode::Chat);
    }

    /// Interrompe la risposta: il worker alza bandiera, chiude lo streaming e
    /// non avvia altre richieste; i messaggi vecchi restano fuori generazione.
    fn stop_chat(&mut self) {
        if !self.chat_busy {
            return;
        }
        if let Some(cancel) = &self.chat_cancel {
            cancel.store(true, Ordering::Relaxed);
        }
        self.chat_gen += 1;
        self.chat_busy = false;
        if let Some(entry) = self
            .chat
            .iter_mut()
            .rev()
            .find(|e| e.answer.state == chat::State::Busy)
        {
            entry.answer.state = chat::State::Done;
            entry.answer.error = Some("Risposta interrotta.".into());
            entry.answer.done_at = self.ctx.input(|i| i.time);
        }
        self.toast("Risposta interrotta.");
    }

    /// Memoria breve per l'interprete: ultimi scambi, compressi in una riga.
    /// `upto` esclude il messaggio appena inviato, che il prompt passa a parte.
    fn chat_history(&self, upto: usize) -> Vec<(String, String)> {
        let end = upto.min(self.chat.len());
        let start = end.saturating_sub(3);
        let mut out: Vec<(String, String)> = Vec::new();
        for entry in &self.chat[start..end] {
            out.push(("utente".into(), entry.user.clone()));
            let a = &entry.answer;
            let mut line = String::new();
            if !a.understood.is_empty() {
                line.push_str(&format!("Criteri: {}", a.understood));
            }
            if !a.cards.is_empty() {
                let titles = a
                    .cards
                    .iter()
                    .map(|c| c.movie.title.clone())
                    .collect::<Vec<_>>()
                    .join(", ");
                if !line.is_empty() {
                    line.push_str(" · ");
                }
                line.push_str(&format!("Ho proposto: {titles}"));
            } else if !a.intro.is_empty() {
                if !line.is_empty() {
                    line.push_str(" · ");
                }
                line.push_str(&a.intro);
            }
            if !line.is_empty() {
                out.push(("assistente".into(), theme::truncate(&line, 220)));
            }
        }
        out
    }

    fn spawn_chat(&mut self, text: String, exclude: Vec<u64>, reuse: Option<Intent>) {
        let index = self.chat.len();
        self.chat.push(chat::Entry {
            user: text.clone(),
            answer: chat::Answer::busy(),
        });
        self.spawn_chat_worker(index, text, exclude, reuse);
    }

    /// Riprova una risposta: stessi criteri, esclusi i film già mostrati.
    fn regenerate(&mut self, index: usize) {
        if self.chat_busy {
            self.toast("Attendi la risposta in corso…");
            return;
        }
        let Some(entry) = self.chat.get(index) else {
            return;
        };
        let text = entry.user.clone();
        let exclude: Vec<u64> = entry.answer.cards.iter().map(|c| c.movie.id).collect();
        let reuse = entry
            .answer
            .context
            .clone()
            .or_else(|| entry.answer.intent.as_ref().map(|(_, it)| it.clone()));
        if let Some(entry) = self.chat.get_mut(index) {
            entry.answer = chat::Answer::busy();
        }
        self.spawn_chat_worker(index, text, exclude, reuse);
    }

    /// «Vedi tutti in griglia»: i filtri capiti dalla chat diventano la
    /// ricerca della griglia, riusando l'intento (nessuna chiamata AI nuova).
    fn show_all_in_grid(&mut self, index: usize) {
        let Some(entry) = self.chat.get(index) else {
            return;
        };
        let user = entry.user.clone();
        if !entry.answer.grid_ready || entry.answer.facets.is_empty() {
            self.toast("Questa richiesta non si traduce in filtri di griglia.");
            return;
        }
        let facets = entry.answer.facets.clone();
        let pairs = entry.answer.keyword_pairs.clone();
        let intent = entry.answer.context.clone();
        self.facets = facets;
        for pair in pairs {
            if !self.keyword_map.iter().any(|(k, _)| k == &pair.0) {
                self.keyword_map.push(pair);
            }
        }
        if let Some(intent) = intent {
            self.ai_cache = Some((user.clone(), intent));
        }
        self.prune = constraints::pruning(&self.tax, &self.facets);
        self.grid_query = user.chars().take(MAX_QUERY_CHARS).collect();
        self.page = 1;
        self.switch_mode(ViewMode::Griglia);
        self.launch_search(true);
    }

    /// Il cuore della chat: interpreta, cerca su TMDB, fa scegliere al
    /// curatore, completa le schede dei film. Tutto fuori dal thread UI.
    fn spawn_chat_worker(
        &mut self,
        index: usize,
        text: String,
        exclude: Vec<u64>,
        reuse: Option<Intent>,
    ) {
        let Some((tmdb, cancel)) = self.tmdb.as_ref().map(|t| {
            let (client, cancel) = t.cancellable();
            (Arc::new(client), cancel)
        }) else {
            self.show_settings = true;
            return;
        };
        self.chat_cancel = Some(cancel.clone());
        self.chat_gen += 1;
        let gen = self.chat_gen;
        self.chat_busy = true;

        let base = self.chat_context.clone();
        let known = self.keyword_map.clone();
        let tax = self.tax.clone();
        let provider = self.provider.clone();
        let history = self.chat_history(index);
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();

        std::thread::spawn(move || {
            let mut payload = ChatPayload {
                index,
                intro: String::new(),
                cards: Vec::new(),
                understood: String::new(),
                limit: None,
                notes: Vec::new(),
                total: 0,
                facets: Facets::default(),
                keyword_pairs: Vec::new(),
                context: base.clone(),
                grid_ready: false,
                intent: None,
                error: None,
            };

            // 1) Interpreta il messaggio. Il contesto dei seguiti e locale
            //    alla chat: i filtri della griglia non c'entrano.
            let reuse_ctx = reuse.is_some();
            let pre_action = classify_chat_action(&text, &base);
            let pre_follow = pre_action.is_contextual();
            let empty = Intent::default();
            // L'AI vede sempre la memoria e decide lei se il messaggio e un
            // seguito; le regole hanno bisogno della classificazione locale.
            let parse_active = if provider.is_llm() || pre_follow {
                &base
            } else {
                &empty
            };
            let parse_history: &[(String, String)] =
                if provider.is_llm() || pre_follow { &history } else { &[] };
            let intent = match reuse {
                Some(it) => Some(it),
                None => match provider.parse_chat_cancellable(
                    &text,
                    &tax,
                    parse_history,
                    parse_active,
                    &cancel,
                ) {
                    Ok(it) => Some(it),
                    Err(_) if cancel.load(Ordering::Relaxed) => return,
                    Err(e) => {
                        payload.notes.push(format!("Interprete non disponibile: {e}"));
                        None
                    }
                },
            };
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let mut action = intent
                .as_ref()
                .map(|i| i.action)
                .unwrap_or(pre_action);
            if let Some(it) = &intent {
                // Retrocompatibilita con interpreti che non restituiscono
                // `action`: la classificazione locale resta la rete di sicurezza.
                if action == ChatAction::New
                    && pre_follow
                    && it.is_empty()
                    && it.mode != "reset"
                    && it.mode != "off_topic"
                {
                    action = pre_action;
                }
            }
            let follow = reuse_ctx || action.is_contextual();
            let off_topic = action == ChatAction::OffTopic
                || intent
                    .as_ref()
                    .map(|i| i.mode.eq_ignore_ascii_case("off_topic"))
                    .unwrap_or(false);
            let seen_title = seen_title_in_query(&text);
            let person_lookup = person_query(&text)
                .or_else(|| {
                    intent
                        .as_ref()
                        .and_then(|i| i.person.clone())
                        .map(|name| PersonQuery {
                            name,
                            role: PersonRole::Any,
                        })
                })
                .or_else(|| {
                    if follow {
                        base.person.as_ref().map(|name| PersonQuery {
                            name: name.clone(),
                            role: PersonRole::Any,
                        })
                    } else {
                        None
                    }
                });
            let mut eff = if follow {
                base.facets.clone()
            } else {
                Facets::default()
            };
            let mut themes = if follow { base.themes.clone() } else { Vec::new() };
            if let Some(it) = &intent {
                if reuse_ctx {
                    eff = it.facets.clone();
                    themes = it.themes.clone();
                } else if action == ChatAction::Reset
                    || it.mode.eq_ignore_ascii_case("reset")
                {
                    eff = Facets::default();
                    themes.clear();
                } else if provider.is_llm() && it.source == "AI" {
                    // L'AI di chat riceve il contesto e, per un seguito,
                    // restituisce lo stato completo. Se non ha capito nulla,
                    // il contesto attivo resta la base.
                    if action.is_contextual() && it.is_empty() && !base.is_empty() {
                        eff = base.facets.clone();
                        themes = base.themes.clone();
                        payload
                            .notes
                            .push("Ho mantenuto il contesto gia attivo.".into());
                    } else {
                        eff = it.facets.clone();
                        themes = it.themes.clone();
                    }
                } else if action.is_contextual() {
                    // Il parser a regole descrive solo il messaggio: va unito
                    // al contesto, che un seguito esplicito non deve perdere.
                    merge_facets(&mut eff, &it.facets, &it.cleared);
                    merge_themes(&mut themes, &it.themes);
                } else {
                    eff = it.facets.clone();
                    themes = it.themes.clone();
                }
                payload.notes.extend(it.notes.clone());
            }
            let (eff, removed) = constraints::sanitize(&tax, &eff);
            payload.notes.extend(removed);
            let context_free = intent.as_ref().and_then(|i| i.free_text.clone());
            let context_similar = intent
                .as_ref()
                .and_then(|i| i.similar_to.clone())
                .or_else(|| {
                    if action.is_contextual() {
                        base.similar_to.clone()
                    } else {
                        None
                    }
                });
            let title = context_free.clone();
            let similar = context_similar.clone();
            payload.context = Intent {
                facets: eff.clone(),
                themes: themes.clone(),
                free_text: context_free,
                similar_to: context_similar,
                action,
                ..Default::default()
            };
            payload.facets = eff.clone();
            payload.understood = describe_chat(&tax, &eff, &themes);
            if let Some(it) = intent.clone() {
                payload.intent = Some((text.clone(), it));
            }

            // 2) Candidati reali. Se l'interprete non ha riconosciuto un
            //    titolo, una frase o una domanda non diventano titoli.
            let _ = tx.send(Msg::ChatStage {
                gen,
                index,
                stage: 1,
            });
            ctx.request_repaint();

            let mut person_pool = None;
            let mut person_label = None;
            let mut person_name = None;
            if let Some(query) = &person_lookup {
                match chat_person_pool(
                    &tmdb,
                    &tax,
                    &eff,
                    query,
                    seen_title.as_deref(),
                    &mut payload.notes,
                ) {
                    Ok(Some(found)) => {
                        person_label = Some(found.label.clone());
                        person_name = Some(found.person.clone());
                        person_pool = Some(found.result);
                    }
                    Ok(None) => {
                        payload.notes.push(format!(
                            "Non ho riconosciuto «{}» come regista o attore.",
                            query.name
                        ));
                    }
                    Err(_) if cancel.load(Ordering::Relaxed) => return,
                    Err(e) => {
                        payload.error = Some(e);
                        let _ = tx.send(Msg::ChatDone {
                            gen,
                            payload: Box::new(payload),
                        });
                        ctx.request_repaint();
                        return;
                    }
                }
            }
            let pool = if let Some(found) = person_pool {
                found
            } else {
                match chat_pool(
                    &tmdb,
                    &tax,
                    &eff,
                    &themes,
                    &known,
                    title.as_deref(),
                    similar.as_deref(),
                    &mut payload.notes,
                ) {
                    Ok(p) => p,
                    Err(_) if cancel.load(Ordering::Relaxed) => return,
                    Err(e) => {
                        payload.error = Some(e);
                        let _ = tx.send(Msg::ChatDone {
                            gen,
                            payload: Box::new(payload),
                        });
                        ctx.request_repaint();
                        return;
                    }
                }
            };
            let ChatPoolResult {
                mut movies,
                total,
                facets: used_facets,
                keyword_pairs,
                limit,
                grid_ready,
                reasons,
                relevance,
            } = pool;
            payload.total = total;
            payload.facets = used_facets.clone();
            payload.keyword_pairs = keyword_pairs;
            payload.limit = limit.clone();
            payload.grid_ready = grid_ready;
            payload.understood = describe_chat(&tax, &used_facets, &themes);
            if let Some(label) = &person_label {
                payload.understood = label.clone();
                payload.context.person = person_name.clone();
                payload.context.free_text = None;
                payload.context.similar_to = None;
            } else {
                if themes.is_empty() {
                    if let Some(s) = similar.as_deref() {
                        payload.understood = format!("simili a «{s}»");
                    } else if let Some(t) = title.as_deref() {
                        payload.understood =
                            if payload.understood.eq_ignore_ascii_case("ricerca libera") {
                                format!("titolo «{t}»")
                            } else {
                                format!("titolo «{t}» · {}", payload.understood)
                            };
                    }
                }
                if off_topic && used_facets.is_empty() && themes.is_empty() {
                    // Una domanda senza legame con i filtri mostra i film del
                    // momento: la riga «Capito» lo dice, senza inventare criteri.
                    payload.understood = "film del momento".into();
                }
            }
            movies.retain(|m| !exclude.contains(&m.id));
            movies.truncate(20);
            if cancel.load(Ordering::Relaxed) {
                return;
            }

            if movies.is_empty() {
                payload.intro = "Non ho trovato film che corrispondano davvero a questa richiesta. Preferisco non riempire la lista con titoli a caso.".into();
                let _ = tx.send(Msg::ChatDone {
                    gen,
                    payload: Box::new(payload),
                });
                ctx.request_repaint();
                return;
            }

            // 3) Curatela: sceglie fino a 5 film reali e scrive i commenti (streaming).
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            let _ = tx.send(Msg::ChatStage {
                gen,
                index,
                stage: 2,
            });
            ctx.request_repaint();
            let make_candidate = |m: &Movie| {
                candidate_of(
                    &tax,
                    m,
                    reasons.get(&m.id).cloned(),
                    relevance.get(&m.id).copied().unwrap_or(1.0),
                )
            };
            let req = curator::Request {
                user_text: text.clone(),
                understood: payload.understood.clone(),
                limit: limit.clone(),
                history: if follow { history.clone() } else { Vec::new() },
                candidates: movies.iter().map(make_candidate).collect(),
            };
            let _ = tx.send(Msg::ChatStage {
                gen,
                index,
                stage: 3,
            });
            ctx.request_repaint();
            let cur = match provider.curate_cancellable(
                &req,
                &cancel,
                &mut |chunk: &str| {
                    let _ = tx.send(Msg::ChatStream {
                        gen,
                        index,
                        chunk: chunk.to_string(),
                    });
                    ctx.request_repaint();
                },
            ) {
                Ok(c) => c,
                Err(_) if cancel.load(Ordering::Relaxed) => return,
                Err(e) => {
                    // La chat funziona anche senza AI: scelta deterministica.
                    payload
                        .notes
                        .push(format!("Curatela AI non disponibile ({e}): scelta automatica."));
                    curator::deterministic(&req)
                }
            };
            payload.intro = cur.intro;
            if cancel.load(Ordering::Relaxed) {
                return;
            }

            // 4) Completa le scelte e carica la scheda dei film mostrati
            //    (regia, durata, disponibilita, IMDb): solo per loro.
            let chosen: Vec<(u64, String, String)> = cur
                .picks
                .into_iter()
                .map(|p| (p.id, p.perche, p.consiglio))
                .collect();
            // Le schede sono indipendenti: le richieste in parallelo
            // riducono l'attesa senza cambiare l'ordine delle card.
            let cards = std::thread::scope(|scope| {
                let mut pending = Vec::with_capacity(chosen.len());
                for (id, perche, consiglio) in chosen {
                    let tmdb = tmdb.clone();
                    pending.push((
                        id,
                        perche,
                        consiglio,
                        scope.spawn(move || tmdb.details(id)),
                    ));
                }
                let mut cards = Vec::with_capacity(pending.len());
                for (id, perche, consiglio, handle) in pending {
                    let Some(movie) = movies.iter().find(|m| m.id == id).cloned() else {
                        continue;
                    };
                    let mut card = chat::Card {
                        movie,
                        perche,
                        consiglio,
                        meta: None,
                    };
                    if let Ok(d) = handle
                        .join()
                        .unwrap_or_else(|_| Err("scheda non disponibile".into()))
                    {
                        card.meta = Some(chat::CardMeta {
                            director: d.director.clone(),
                            runtime: d.runtime,
                            flatrate: d.flatrate.clone(),
                            rent: d.rent.clone(),
                            imdb: d.imdb,
                        });
                        card.movie = d.movie;
                    }
                    cards.push(card);
                }
                cards
            });
            payload.cards = cards;
            if cancel.load(Ordering::Relaxed) {
                return;
            }
            if payload.cards.is_empty() {
                payload.error = Some("Non sono riuscito a preparare le schede dei film.".into());
            }
            // Il rilassamento puo produrre la stessa nota per ogni pagina:
            // in chat si mostra una volta sola.
            let mut seen: HashSet<String> = HashSet::new();
            payload.notes.retain(|n| seen.insert(n.clone()));

            let _ = tx.send(Msg::ChatDone {
                gen,
                payload: Box::new(payload),
            });
            ctx.request_repaint();
        });
        self.ctx.request_repaint();
    }

    fn apply_chat_actions(&mut self, a: chat::Actions) {
        if let Some(id) = a.open_detail {
            // In chat la scheda è un'anteprima centrale: la conversazione
            // resta al suo posto e non si salta alla griglia.
            self.open_detail(id);
        }
        if let Some(title) = a.similar_to {
            self.grid_query = format!("film simili a «{title}»");
            self.switch_mode(ViewMode::Griglia);
            self.page = 1;
            self.launch_search(true);
        }
        if a.stop {
            self.stop_chat();
        }
        if let Some(i) = a.retry {
            self.regenerate(i);
        }
        if let Some(i) = a.show_all {
            self.show_all_in_grid(i);
        }
        if a.new_chat {
            self.new_chat();
        }
    }

    // ------------------------------------------------------------ simkl ----

    fn simkl_client_id(&self) -> String {
        let typed = self.settings.simkl_client_id.trim();
        if !typed.is_empty() {
            return typed.to_string();
        }
        let configured = self.cfg.simkl.client_id.trim();
        if !configured.is_empty() {
            return configured.to_string();
        }
        simkl::CLIENT_ID.to_string()
    }

    fn start_simkl_auth(&mut self) {
        if self.simkl_auth.is_some() {
            return;
        }
        let client_id = self.simkl_client_id();
        if client_id.is_empty() {
            self.simkl_error = Some(
                "Inserisci il Client ID dell'app SIMKL (lo trovi nelle impostazioni sviluppatore)."
                    .into(),
            );
            return;
        }
        if self.cfg.simkl.client_id != client_id {
            self.cfg.simkl.client_id = client_id.clone();
            let _ = self.cfg.save();
        }
        self.simkl_epoch += 1;
        let gen = self.simkl_epoch;
        self.simkl_cancel.store(false, Ordering::Relaxed);
        self.simkl_error = None;
        self.simkl_status = Some("Apro il collegamento SIMKL…".into());
        let cancel = self.simkl_cancel.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let code = match simkl::device_start(&client_id) {
                Ok(c) => c,
                Err(e) => {
                    let _ = tx.send(Msg::SimklCode { gen, res: Err(e) });
                    ctx.request_repaint();
                    return;
                }
            };
            let deadline = Instant::now() + Duration::from_secs(code.expires_in.min(900));
            let _ = tx.send(Msg::SimklCode {
                gen,
                res: Ok(code.clone()),
            });
            ctx.request_repaint();
            let mut interval = code.interval.max(1);
            loop {
                if cancel.load(Ordering::Relaxed) || Instant::now() >= deadline {
                    break;
                }
                std::thread::sleep(Duration::from_secs(interval));
                if cancel.load(Ordering::Relaxed) {
                    return;
                }
                match simkl::device_poll(&client_id, &code.device_code) {
                    Ok(simkl::Poll::Tokens(tokens)) => {
                        let _ = tx.send(Msg::SimklAuth {
                            gen,
                            res: Ok(tokens),
                        });
                        ctx.request_repaint();
                        return;
                    }
                    Ok(simkl::Poll::Pending) => {}
                    Ok(simkl::Poll::SlowDown) => interval += 5,
                    Ok(simkl::Poll::Expired) => {
                        let _ = tx.send(Msg::SimklAuth {
                            gen,
                            res: Err("Codice scaduto: riprova il collegamento.".into()),
                        });
                        ctx.request_repaint();
                        return;
                    }
                    Err(e) => {
                        let _ = tx.send(Msg::SimklAuth { gen, res: Err(e) });
                        ctx.request_repaint();
                        return;
                    }
                }
            }
            if !cancel.load(Ordering::Relaxed) {
                let _ = tx.send(Msg::SimklAuth {
                    gen,
                    res: Err("Tempo scaduto: riprova il collegamento.".into()),
                });
                ctx.request_repaint();
            }
        });
    }

    fn cancel_simkl_auth(&mut self) {
        self.simkl_epoch += 1;
        self.simkl_cancel.store(true, Ordering::Relaxed);
        self.simkl_auth = None;
        self.simkl_status = Some("Collegamento SIMKL annullato.".into());
    }

    fn start_simkl_sync(&mut self, force: bool) {
        if self.simkl_syncing || self.simkl_busy {
            return;
        }
        let Some(tokens) = self.simkl_tokens.clone() else {
            self.simkl_error = Some("Collega prima SIMKL dalle Impostazioni.".into());
            return;
        };
        let client_id = self.simkl_client_id();
        if client_id.is_empty() {
            self.simkl_error = Some("Manca il Client ID SIMKL.".into());
            return;
        }
        self.simkl_syncing = true;
        self.simkl_error = None;
        self.simkl_status = Some("Sincronizzo la libreria SIMKL…".into());
        let gen = self.simkl_epoch;
        let store = self.simkl_store.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = simkl::sync(&client_id, tokens, store, force);
            let _ = tx.send(Msg::SimklSynced { gen, res });
            ctx.request_repaint();
        });
    }

    fn start_simkl_write(&mut self, movie: &Movie, op: simkl::WriteOp) -> bool {
        if self.simkl_busy || self.simkl_syncing {
            return false;
        }
        let Some(tokens) = self.simkl_tokens.clone() else {
            self.simkl_error = Some("Collega prima SIMKL dalle Impostazioni.".into());
            return false;
        };
        let client_id = self.simkl_client_id();
        if client_id.is_empty() {
            self.simkl_error = Some("Manca il Client ID SIMKL.".into());
            return false;
        }
        self.simkl_busy = true;
        self.simkl_pending = Some((movie.id, op));
        self.simkl_error = None;
        self.simkl_status = Some("Salvo su SIMKL…".into());
        let gen = self.simkl_epoch;
        let target = simkl::WriteMovie::from_movie(movie);
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = simkl::write(&client_id, tokens, &target, op);
            let _ = tx.send(Msg::SimklWritten { gen, res });
            ctx.request_repaint();
        });
        true
    }

    fn simkl_write_current(&mut self, op: simkl::WriteOp) {
        let Some(d) = self.detail.clone() else {
            return;
        };
        self.start_simkl_write(&d.movie, op);
    }

    fn reload_simkl_tokens(&mut self) {
        if let Ok(Some(tokens)) = simkl::load_tokens() {
            self.simkl_tokens = Some(tokens);
        }
    }

    /// Riallinea il Credential Manager allo stato corrente dopo un messaggio
    /// SIMKL di un collegamento ormai vecchio.
    fn restore_simkl_tokens(&self) {
        match &self.simkl_tokens {
            Some(tokens) => {
                let _ = simkl::save_tokens(tokens);
            }
            None => {
                let _ = simkl::clear_tokens();
            }
        }
    }

    fn disconnect_simkl(&mut self) {
        self.simkl_epoch += 1;
        self.simkl_cancel.store(true, Ordering::Relaxed);
        if let Err(e) = simkl::clear_tokens() {
            self.simkl_error = Some(e);
            return;
        }
        self.simkl_tokens = None;
        self.simkl_auth = None;
        self.simkl_syncing = false;
        self.simkl_busy = false;
        self.simkl_pending = None;
        self.simkl_store = simkl::Store::default();
        self.simkl_cards = None;
        let _ = simkl::save_store(&self.simkl_store);
        self.simkl_status = Some("SIMKL scollegato: la copia locale è stata rimossa.".into());
    }

    fn ensure_simkl_posters(&mut self) {
        let Some(tmdb) = self.tmdb.clone() else {
            return;
        };
        let (resume, watchlist) = self.simkl_cards();
        let mut ids: Vec<u64> = resume.iter().map(|p| p.tmdb).collect();
        ids.extend(watchlist.iter().map(|m| m.tmdb));
        ids.sort_unstable();
        ids.dedup();
        for id in ids {
            if self.poster_cache.contains_key(&id) {
                continue;
            }
            self.poster_cache.insert(id, None);
            let tmdb = tmdb.clone();
            let tx = self.tx.clone();
            let ctx = self.ctx.clone();
            std::thread::spawn(move || {
                let url = tmdb
                    .details(id)
                    .ok()
                    .and_then(|d| d.movie.poster_path)
                    .map(|p| poster_url(&p, "w342"));
                let _ = tx.send(Msg::Poster { id, url });
                ctx.request_repaint();
            });
        }
    }

    fn simkl_cards(&mut self) -> (Vec<simkl::Playback>, Vec<simkl::TrackedMovie>) {
        if let Some(cards) = &self.simkl_cards {
            return cards.clone();
        }
        let cards = (
            self.simkl_store
                .resume_picks(12)
                .into_iter()
                .cloned()
                .collect(),
            self.simkl_store
                .watchlist_picks(12)
                .into_iter()
                .cloned()
                .collect(),
        );
        self.simkl_cards = Some(cards.clone());
        cards
    }

    fn simkl_resume_cards(&mut self) -> Vec<chat::PersonalCard> {
        let (resume, _) = self.simkl_cards();
        resume
            .into_iter()
            .map(|playback| {
                let percent = playback.progress.round() as u32;
                chat::PersonalCard {
                    title: if playback.title.is_empty() {
                        "Film".into()
                    } else {
                        playback.title.clone()
                    },
                    meta: match playback.year {
                        Some(year) => format!("{percent}% · {year}"),
                        None => format!("{percent}%"),
                    },
                    movie_id: playback.tmdb,
                    progress: Some((playback.progress / 100.0).clamp(0.0, 1.0)),
                    poster: self
                        .poster_cache
                        .get(&playback.tmdb)
                        .cloned()
                        .flatten(),
                }
            })
            .collect()
    }

    fn simkl_watchlist_cards(&mut self) -> Vec<chat::PersonalCard> {
        let (_, watchlist) = self.simkl_cards();
        watchlist
            .into_iter()
            .map(|movie| chat::PersonalCard {
                title: if movie.title.is_empty() {
                    "Film".into()
                } else {
                    movie.title.clone()
                },
                meta: match movie.year {
                    Some(year) => format!("DA VEDERE · {year}"),
                    None => "DA VEDERE".into(),
                },
                movie_id: movie.tmdb,
                progress: None,
                poster: self.poster_cache.get(&movie.tmdb).cloned().flatten(),
            })
            .collect()
    }

    fn apply_track_flags(&mut self) {
        let store = &self.simkl_store;
        let flag = |id: u64| track_state(store, id);
        for movie in &mut self.movies {
            movie.track = flag(movie.id);
        }
        for entry in &mut self.chat {
            for card in &mut entry.answer.cards {
                card.movie.track = flag(card.movie.id);
            }
        }
        if let Some(detail) = &mut self.detail {
            detail.movie.track = flag(detail.movie.id);
            for movie in &mut detail.recommendations {
                movie.track = flag(movie.id);
            }
        }
    }

    fn patch_track(&mut self, movie_id: u64, op: simkl::WriteOp, outcome: &simkl::WriteOutcome) {
        let key = movie_id.to_string();
        match op {
            simkl::WriteOp::Remove => {
                self.simkl_store.movies.remove(&key);
            }
            simkl::WriteOp::Watchlist => {
                let entry = self.simkl_store.movies.entry(key).or_insert_with(|| {
                    simkl::TrackedMovie {
                        simkl_id: 0,
                        tmdb: movie_id,
                        slug: String::new(),
                        status: String::new(),
                        rating: None,
                        watched_at: None,
                        added_at: None,
                        imdb: None,
                        title: String::new(),
                        year: None,
                    }
                });
                entry.status = "plantowatch".into();
            }
            simkl::WriteOp::Watched => {
                let entry = self.simkl_store.movies.entry(key).or_insert_with(|| {
                    simkl::TrackedMovie {
                        simkl_id: 0,
                        tmdb: movie_id,
                        slug: String::new(),
                        status: String::new(),
                        rating: None,
                        watched_at: None,
                        added_at: None,
                        imdb: None,
                        title: String::new(),
                        year: None,
                    }
                });
                entry.status = outcome
                    .status
                    .clone()
                    .unwrap_or_else(|| "completed".into());
            }
            simkl::WriteOp::Rate(value) => {
                let entry = self.simkl_store.movies.entry(key).or_insert_with(|| {
                    simkl::TrackedMovie {
                        simkl_id: 0,
                        tmdb: movie_id,
                        slug: String::new(),
                        status: String::new(),
                        rating: None,
                        watched_at: None,
                        added_at: None,
                        imdb: None,
                        title: String::new(),
                        year: None,
                    }
                });
                entry.rating = Some(value);
                if entry.status.is_empty() {
                    entry.status = "completed".into();
                }
                if let Some(status) = &outcome.status {
                    entry.status = status.clone();
                }
            }
            simkl::WriteOp::ClearRating => {
                if let Some(entry) = self.simkl_store.movies.get_mut(&key) {
                    entry.rating = None;
                }
            }
        }
        self.simkl_cards = None;
        self.apply_track_flags();
    }

    fn simkl_track(&self, movie: &Movie) -> detail::TrackInfo {
        match self.simkl_store.get(movie.id) {
            Some(tracked) => detail::TrackInfo {
                status: Some(tracked.status.clone()),
                rating: tracked.rating,
                url: Some(if tracked.simkl_id > 0 {
                    simkl::link_to(tracked)
                } else {
                    simkl::search_url(&movie.title)
                }),
                connected: self.simkl_tokens.is_some(),
                busy: self.simkl_busy || self.simkl_syncing,
            },
            None => detail::TrackInfo {
                url: Some(simkl::search_url(&movie.title)),
                connected: self.simkl_tokens.is_some(),
                busy: self.simkl_busy || self.simkl_syncing,
                ..Default::default()
            },
        }
    }

    fn apply_detail_actions(
        &mut self,
        actions: Vec<detail::DetailAction>,
        current_id: u64,
        title: &str,
        from_chat: bool,
    ) {
        for action in actions {
            match action {
                detail::DetailAction::Open(id) => {
                    if id == current_id {
                        if from_chat {
                            self.switch_mode(ViewMode::Griglia);
                        }
                        if self.detail.is_some() {
                            self.page = 1;
                            self.grid_query = title.to_string();
                            self.facets = Facets {
                                sort: self.facets.sort,
                                ..Default::default()
                            };
                            self.facet_query.clear();
                            self.resanitize();
                            self.launch_search(false);
                        }
                    } else {
                        self.open_detail(id);
                    }
                }
                detail::DetailAction::OpenUrl(url) => open_url(&url),
                detail::DetailAction::Close => self.close_detail(),
                detail::DetailAction::MarkWatched => {
                    self.simkl_write_current(simkl::WriteOp::Watched)
                }
                detail::DetailAction::AddWatchlist => {
                    self.simkl_write_current(simkl::WriteOp::Watchlist)
                }
                detail::DetailAction::RemoveTracking => {
                    self.simkl_write_current(simkl::WriteOp::Remove)
                }
                detail::DetailAction::Rate(value) => {
                    self.simkl_write_current(simkl::WriteOp::Rate(value))
                }
                detail::DetailAction::ClearRating => {
                    self.simkl_write_current(simkl::WriteOp::ClearRating)
                }
            }
        }
    }

    /// Avvia la ricerca. `with_ai` = interpreta il testo con l'interprete attivo.
    fn launch_search(&mut self, with_ai: bool) {
        self.spawn_search(with_ai);
    }

    fn load_trending(&mut self) {
        if self.movies.is_empty() && !self.loading {
            self.page = 1;
            self.launch_search(false);
        }
    }

    /// Carica il cartellone "In sala" se la lista e' scaduta. La home la
    /// chiama a ogni frame, ma la richiesta parte solo ogni ora.
    fn maybe_refresh_now_playing(&mut self) {
        if self.home_now_playing_busy || self.tmdb.is_none() {
            return;
        }
        if self
            .home_now_playing_next
            .is_some_and(|next| Instant::now() < next)
        {
            return;
        }
        self.start_now_playing();
    }

    fn start_now_playing(&mut self) {
        let Some(tmdb) = self.tmdb.clone() else {
            return;
        };
        self.home_now_playing_busy = true;
        self.home_now_playing_gen = self.home_now_playing_gen.wrapping_add(1);
        let gen = self.home_now_playing_gen;
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = tmdb.now_playing().map(|p| {
                in_sala_movies(p.results.into_iter().map(Movie::from).collect())
            });
            let _ = tx.send(Msg::NowPlaying { gen, res });
            ctx.request_repaint();
        });
    }

    fn close_detail(&mut self) {
        self.detail = None;
        self.detail_loading = false;
        self.detail_gen = self.detail_gen.wrapping_add(1);
    }

    /// Un solo pannello laterale per volta: filtri e scheda non si contendono
    /// la griglia. Aprire i filtri chiude la scheda.
    fn toggle_filters(&mut self) {
        if self.filters_open {
            self.filters_open = false;
        } else {
            self.close_detail();
            self.filters_open = true;
        }
    }

    fn open_detail(&mut self, id: u64) {
        let Some(tmdb) = self.tmdb.clone() else { return };
        self.close_detail();
        self.filters_open = false;
        self.detail_loading = true;
        let gen = self.detail_gen;
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = tmdb.details(id);
            let _ = tx.send(Msg::Details { gen, res });
            ctx.request_repaint();
        });
    }

    // ------------------------------------------------------------ facce ------

    fn resanitize(&mut self) {
        let (f, msgs) = constraints::sanitize(&self.tax, &self.facets);
        self.facets = f;
        if !msgs.is_empty() {
            self.toast(msgs[0].clone());
        }
        self.messages = msgs;
        self.prune = constraints::pruning(&self.tax, &self.facets);
    }

    fn apply_actions(&mut self, actions: Vec<FacetAction>) {
        if !actions.is_empty() {
            // Toccare i filtri a mano e esplorazione: si torna alla griglia.
            self.unlocks.clear();
            self.switch_mode(ViewMode::Griglia);
        }
        for action in actions {
            match action {
                FacetAction::Decade(id) => toggle(&mut self.facets.decades, id),
                FacetAction::Movement(id) => toggle(&mut self.facets.movements, id),
                FacetAction::Genre(id) => {
                    if self.facets.without_genres.contains(&id) {
                        toggle(&mut self.facets.without_genres, id);
                    } else {
                        toggle(&mut self.facets.genres, id);
                    }
                }
                FacetAction::Country(code) => toggle(&mut self.facets.countries, code),
                FacetAction::Mood(id) => toggle(&mut self.facets.moods, id),
                FacetAction::Keyword(name) => toggle(&mut self.facets.keywords, name),
                FacetAction::Rating(v) => self.facets.min_rating = v,
                FacetAction::MaxRating(v) => self.facets.max_rating = v,
                FacetAction::Votes(v) => self.facets.min_votes = v,
                FacetAction::Runtime(v) => self.facets.runtime = v,
                // Guida: la combinazione di filtri che descrive una corrente.
                // Si aggiunge a quello che c'e gia, cosi l'utente vede le facce
                // accendersi e puo togliere cio che non vuole.
                FacetAction::Combo { decades, countries, movements } => {
                    for d in decades {
                        if !self.facets.decades.contains(&d) {
                            self.facets.decades.push(d);
                        }
                    }
                    for c in countries {
                        if !self.facets.countries.contains(&c) {
                            self.facets.countries.push(c);
                        }
                    }
                    for m in movements {
                        if !self.facets.movements.contains(&m) {
                            self.facets.movements.push(m);
                        }
                    }
                }
                FacetAction::Clear(Section::All) => {
                    let sort = self.facets.sort;
                    self.facets = Facets { sort, ..Default::default() };
                    self.grid_query.clear();
                    self.facet_query.clear();
                    self.ai_notes.clear();
                }
                FacetAction::Clear(Section::Period) => self.facets.decades.clear(),
                FacetAction::Clear(Section::Genres) => {
                    self.facets.genres.clear();
                    self.facets.without_genres.clear();
                }
                FacetAction::Clear(Section::Countries) => self.facets.countries.clear(),
                FacetAction::Clear(Section::Moods) => {
                    self.facets.moods.clear();
                    self.facets.keywords.clear();
                }
                FacetAction::Clear(Section::Metrics) => {
                    self.facets.min_rating = None;
                    self.facets.max_rating = None;
                    self.facets.min_votes = None;
                    self.facets.runtime = None;
                }
            }
        }
        self.resanitize();
        if !self.tmdb.is_some() {
            return;
        }
        self.dirty = Some(Instant::now());
    }

    /// Applica un compromesso: toglie un solo vincolo e riparte subito.
    fn apply_unlock(&mut self, index: usize) {
        let Some(u) = self.unlocks.get(index).cloned() else {
            return;
        };
        self.facets = u.facets;
        self.unlocks.clear();
        self.page = 1;
        self.resanitize();
        if self.tmdb.is_some() {
            self.launch_search(false);
        }
    }

    /// Verifica della credenziale AI: prova una vera interpretazione con il
    /// collegamento e il modello scritti nel modulo, senza salvare nulla.
    fn check_ai(&mut self) {
        let ai = self.settings.ai_cfg();
        let Some(config) = llm_config(&ai) else {
            self.ai_check = Some("✕ Scegli un interprete AI.".to_string());
            return;
        };
        if self.settings.provider == "deepseek" && ai.api_key.trim().is_empty() {
            self.ai_check = Some("✕ Inserisci la chiave API DeepSeek.".to_string());
            return;
        }
        if self.settings.provider == "openrouter" && ai.openrouter_api_key.trim().is_empty() {
            self.ai_check = Some("✕ Inserisci la chiave API OpenRouter.".to_string());
            return;
        }
        self.ai_checking = true;
        self.ai_check = Some("Prova in corso…".to_string());
        let tax = self.tax.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let provider = LlmProvider::new(config);
            let res = provider.probe(&tax);
            let _ = tx.send(Msg::AiCheck { res });
            ctx.request_repaint();
        });
    }

    /// Interroga il servizio e riempie l'elenco dei modelli utilizzabili.
    fn detect_models(&mut self) {
        self.models_gen = self.models_gen.wrapping_add(1);
        let gen = self.models_gen;
        let (kind, base, key) = match self.settings.provider.as_str() {
            "deepseek" => {
                if self.settings.deepseek_key.trim().is_empty() {
                    self.models_status = Some("✕ Inserisci la chiave API DeepSeek.".to_string());
                    self.models.clear();
                    return;
                }
                (
                    LlmKind::OpenAi,
                    DEEPSEEK_BASE.to_string(),
                    self.settings.deepseek_key.trim().to_string(),
                )
            }
            "openrouter" => (
                LlmKind::OpenAi,
                OPENROUTER_BASE.to_string(),
                self.settings.openrouter_key.trim().to_string(),
            ),
            "ollama" => (
                LlmKind::Ollama,
                if self.settings.ollama_url.trim().is_empty() {
                    "http://localhost:11434".to_string()
                } else {
                    self.settings.ollama_url.trim().to_string()
                },
                String::new(),
            ),
            _ => return,
        };
        self.detecting = true;
        self.ai_check = None;
        self.models_status = Some("Interrogo il servizio…".to_string());
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = list_models(kind, &base, &key);
            let _ = tx.send(Msg::Models { gen, res });
            ctx.request_repaint();
        });
    }

    /// Verifica della credenziale senza salvare: usa la chiave scritta nel campo.
    fn check_credentials(&mut self) {
        let key = self.settings.tmdb_key.trim().to_string();
        if key.is_empty() {
            self.settings_check = Some("✕ Inserisci prima una credenziale.".to_string());
            return;
        }
        self.settings_check = Some("Verifica in corso…".to_string());
        let lang = self.settings.language.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let tmdb = Tmdb::new(key, lang);
            let res = tmdb.ping();
            let _ = tx.send(Msg::Check { res });
            ctx.request_repaint();
        });
    }

    /// Scarica i voti IMDb in background e aggiorna la scheda delle Impostazioni.
    fn start_imdb_download(&mut self) {
        if self.imdb_busy {
            return;
        }
        self.imdb_busy = true;
        self.imdb_status = None;
        self.imdb_progress = Some((0, 0));
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            // l'avanzamento arriva a blocchi di circa 2 MB: non serve di piu
            let cb = |done: u64| {
                if done % (2 * 1_048_576) < 262_144 {
                    let _ = tx.send(Msg::ImdbProgress { done });
                }
            };
            let res = crate::infra::imdb::download(&cb);
            let _ = tx.send(Msg::Imdb { res });
            ctx.request_repaint();
        });
    }

    // --------------------------------------------------------- aggiornamenti ----

    fn maybe_check_updates(&mut self) {
        if !update::enabled()
            || self.update.checking
            || self.update.available.is_some()
            || self.update.progress.is_some()
        {
            return;
        }
        let due = self
            .update
            .last_check
            .map(|at| at.elapsed() >= Duration::from_secs(6 * 60 * 60))
            .unwrap_or(true);
        if due {
            self.start_update_check(false);
        }
    }

    fn start_update_check(&mut self, manual: bool) {
        if !update::enabled() {
            if manual {
                self.update.status =
                    Some("Gli aggiornamenti automatici sono disponibili nell'installer.".into());
            }
            return;
        }
        if self.update.checking || self.update.progress.is_some() {
            return;
        }
        self.update.checking = true;
        self.update.error = None;
        if manual {
            self.update.status = Some("Controllo aggiornamenti…".into());
        }
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = update::check();
            let _ = tx.send(Msg::UpdateChecked(res));
            ctx.request_repaint();
        });
    }

    fn start_update_download(&mut self) {
        if self.update.progress.is_some() || self.update.downloaded.is_some() {
            return;
        }
        let Some(manifest) = self.update.available.clone() else {
            return;
        };
        self.update.error = None;
        self.update.status = Some("Scarico il setup…".into());
        self.update.progress = Some((0, manifest.installer.size));
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = update::download_installer(&manifest, &mut |done, total| {
                let _ = tx.send(Msg::UpdateProgress { done, total });
                ctx.request_repaint();
            });
            let _ = tx.send(Msg::UpdateDownloaded(res));
            ctx.request_repaint();
        });
    }

    fn install_update(&mut self) {
        let Some(path) = self.update.downloaded.clone() else {
            return;
        };
        match update::launch_installer(&path) {
            Ok(()) => {
                self.ctx
                    .send_viewport_cmd(egui::viewport::ViewportCommand::Close);
            }
            Err(e) => {
                self.update.error = Some(e.clone());
                self.toast(e);
            }
        }
    }

    fn save_settings(&mut self) {
        let cfg = Config {
            tmdb_api_key: self.settings.tmdb_key.trim().to_string(),
            language: self.settings.language.clone(),
            ai: self.settings.ai_cfg(),
            simkl: crate::infra::config::SimklCfg {
                client_id: {
                    let typed = self.settings.simkl_client_id.trim();
                    if typed.is_empty() {
                        self.cfg.simkl.client_id.trim().to_string()
                    } else {
                        typed.to_string()
                    }
                },
                hide_watched: self.settings.simkl_hide_watched,
            },
        };
        match cfg.save() {
            Ok(()) => {
                self.cfg = cfg.clone();
                self.tmdb = Self::build_tmdb(&cfg);
                self.home_now_playing.clear();
                self.home_now_playing_busy = false;
                self.home_now_playing_next = None;
                self.home_now_playing_gen = self.home_now_playing_gen.wrapping_add(1);
                self.provider = build_provider(&cfg.ai);
                self.keyword_map.clear();
                self.ai_cache = None;
                self.models_gen = self.models_gen.wrapping_add(1);
                self.models.clear();
                self.models_status = None;
                self.ai_check = None;
                if self.settings.can_detect() {
                    self.detect_models();
                }
                self.toast(format!("Impostazioni salvate · interprete: {}", self.provider.label()));
                if self.tmdb.is_some() {
                    self.page = 1;
                    self.launch_search(false);
                }
            }
            Err(e) => self.toast(e),
        }
    }

    // --------------------------------------------------------------- ui ------

    /// Nome breve dell'interprete per la barra di stato.
    fn provider_short(&self) -> &'static str {
        if !self.provider.is_llm() {
            return "REGOLE";
        }
        match self.cfg.ai.provider.as_str() {
            "deepseek" => "DEEPSEEK",
            "openrouter" => "OPENROUTER",
            "ollama" => "OLLAMA",
            _ => "REGOLE",
        }
    }

    /// Top strip LUMEN: marchio, contesto, controlli finestra.
    fn header(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("header")
            .exact_size(48.0)
            .frame(egui::Frame::NONE.fill(lumen::INK))
            .show(ui, |ui| {
                let r = ui.max_rect();
                let drag = ui.interact(
                    r,
                    egui::Id::unique("window-drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ui.ctx()
                        .send_viewport_cmd(egui::viewport::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(
                        egui::viewport::ViewportCommand::Maximized(!max),
                    );
                }

                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(lumen::display(22.0, "CINE"));
                    ui.label(lumen::display_color(22.0, "MATCH", lumen::EMBER));
                    ui.add_space(18.0);
                    let (kicker, title) = if self.show_settings {
                        ("", "IMPOSTAZIONI")
                    } else if self.mode == ViewMode::Griglia {
                        ("SALA 02", "CATALOGO")
                    } else {
                        ("SALA 01", "CHAT")
                    };
                    if !kicker.is_empty() {
                        ui.label(lumen::mono(9.0, kicker).color(lumen::FAINT));
                        ui.add_space(8.0);
                    }
                    ui.label(lumen::mono(9.0, title).color(lumen::IVORY));

                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let cap = 38.0;
                            if window_caption(ui, CaptionIcon::Close, "Chiudi (Alt+F4)", cap)
                                .clicked()
                            {
                                ui.ctx().send_viewport_cmd(
                                    egui::viewport::ViewportCommand::Close,
                                );
                            }
                            let max = ui
                                .ctx()
                                .input(|i| i.viewport().maximized.unwrap_or(false));
                            let (icon, tip) = if max {
                                (CaptionIcon::Restore, "Ripristina")
                            } else {
                                (CaptionIcon::Maximize, "Ingrandisci")
                            };
                            if window_caption(ui, icon, tip, cap).clicked() {
                                ui.ctx().send_viewport_cmd(
                                    egui::viewport::ViewportCommand::Maximized(!max),
                                );
                            }
                            if window_caption(ui, CaptionIcon::Minimize, "Riduci a icona", cap)
                                .clicked()
                            {
                                ui.ctx().send_viewport_cmd(
                                    egui::viewport::ViewportCommand::Minimized(true),
                                );
                            }
                        },
                    );
                });

                ui.painter().line_segment(
                    [r.left_bottom(), r.right_bottom()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
            });
    }

    /// Rail LUMEN: Chat, Griglia, Nuova chat, Impostazioni.
    fn rail(&mut self, ui: &mut egui::Ui) {
        let mut switch = None;
        let mut new_chat = false;
        let mut settings = false;
        egui::Panel::left("rail")
            .exact_size(72.0)
            .resizable(false)
            .frame(egui::Frame::NONE.fill(lumen::DEEP))
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [r.right_top(), r.right_bottom()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
                ui.add_space(12.0);
                if lumen::nav_item(
                    ui,
                    lumen::Icon::Home,
                    "Chat",
                    self.mode == ViewMode::Chat && !self.show_settings,
                )
                .clicked()
                {
                    switch = Some(ViewMode::Chat);
                }
                if lumen::nav_item(
                    ui,
                    lumen::Icon::Grid,
                    "Griglia",
                    self.mode == ViewMode::Griglia && !self.show_settings,
                )
                .clicked()
                {
                    switch = Some(ViewMode::Griglia);
                }
                if lumen::nav_item(ui, lumen::Icon::New, "Nuova", false).clicked() {
                    new_chat = true;
                    switch = Some(ViewMode::Chat);
                }
                let settings_resp = lumen::nav_item(
                    ui,
                    lumen::Icon::Settings,
                    "Setup",
                    self.show_settings,
                );
                if self.update.available.is_some() {
                    let dot = settings_resp.rect.right_top() + egui::vec2(-10.0, 10.0);
                    ui.painter().circle_filled(dot, 3.0, lumen::EMBER);
                }
                if settings_resp.clicked() {
                    settings = true;
                }
                ui.with_layout(
                    egui::Layout::bottom_up(egui::Align::LEFT),
                    |ui| {
                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            ui.add_space(12.0);
                            ui.label(
                                lumen::mono(
                                    9.0,
                                    format!("v{}", update::current_version()),
                                )
                                .color(lumen::FAINT),
                            );
                        });
                    },
                );
            });
        if let Some(mode) = switch {
            self.show_settings = false;
            self.switch_mode(mode);
        }
        if new_chat {
            self.show_settings = false;
            self.switch_mode(ViewMode::Chat);
            self.new_chat();
        }
        if settings {
            self.show_settings = !self.show_settings;
        }
    }

    /// Toolbar della griglia, dentro il contenuto a destra della rail.
    fn grid_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.mode != ViewMode::Griglia || self.show_settings {
            return;
        }
        let compact = ui.ctx().content_rect().width() < 1000.0;
        let want_focus = self.focus == Some(ViewMode::Griglia);
        let sort_before = self.facets.sort;
        let mut do_search = false;
        let mut toggle_filters = false;
        let mut refresh = false;

        egui::Panel::top("grid-toolbar")
            .exact_size(52.0)
            .frame(egui::Frame::NONE.fill(lumen::INK))
            .show(ui, |ui| {
                ui.spacing_mut().combo_height = 360.0;
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [r.left_bottom(), r.right_bottom()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(lumen::mono(11.0, "CATALOGO").color(lumen::IVORY));
                    if !compact {
                        ui.add_space(8.0);
                        let count = if self.loading {
                            "CERCO…".to_string()
                        } else {
                            format!("{} FILM", theme::thousands(self.total))
                        };
                        ui.label(lumen::mono(9.0, count).color(lumen::FAINT));
                    }
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 8.0;
                            ui.add_space(14.0);
                            if lumen::icon_button(
                                ui,
                                lumen::Icon::Refresh,
                                34.0,
                                "Aggiorna la griglia",
                            )
                            .clicked()
                            {
                                refresh = true;
                            }
                            egui::ComboBox::from_id_salt("grid-sort")
                                .selected_text(
                                    egui::RichText::new(self.facets.sort.label()).size(11.0),
                                )
                                .width(if compact { 120.0 } else { 150.0 })
                                .icon(|ui, rect, _visuals, _open| {
                                    let c = rect.center();
                                    let stroke = egui::Stroke::new(1.2_f32, lumen::MUTED);
                                    ui.painter().line_segment(
                                        [c + egui::vec2(-4.0, -2.0), c],
                                        stroke,
                                    );
                                    ui.painter().line_segment(
                                        [c + egui::vec2(4.0, -2.0), c],
                                        stroke,
                                    );
                                })
                                .show_ui(ui, |ui| {
                                    for s in Sort::ALL {
                                        ui.selectable_value(
                                            &mut self.facets.sort,
                                            s,
                                            s.label(),
                                        );
                                    }
                                });
                            let active = self.facets.active_count();
                            let label = if active > 0 {
                                format!("Filtri {active}")
                            } else {
                                "Filtri".to_string()
                            };
                            if lumen::button_sized(ui, &label, false, 92.0).clicked() {
                                toggle_filters = true;
                            }
                            if lumen::button_sized(ui, "Cerca", true, 80.0).clicked() {
                                do_search = true;
                            }
                            let w = ui.available_width().clamp(140.0, 380.0);
                            let hint = if compact {
                                "Cerca o descrivi…"
                            } else if self.provider.is_llm() {
                                "Descrivi cosa vuoi vedere…"
                            } else {
                                "Titolo, corrente, tema…"
                            };
                            let field_id = egui::Id::unique("grid-query");
                            let focused = ui.memory(|m| m.has_focus(field_id));
                            let frame = egui::Frame::NONE
                                .fill(lumen::DEEP)
                                .stroke(egui::Stroke::new(
                                    1.0_f32,
                                    if focused { lumen::EMBER } else { lumen::BORDER },
                                ))
                                .corner_radius(lumen::radius(lumen::RADIUS_SM))
                                .inner_margin(egui::Margin::symmetric(10, 7));
                            let resp = frame
                                .show(ui, |ui| {
                                    ui.add_sized(
                                        [(w - 20.0).max(80.0), 20.0],
                                        egui::TextEdit::singleline(&mut self.grid_query)
                                            .id(field_id)
                                            .frame(egui::Frame::NONE)
                                            .char_limit(MAX_QUERY_CHARS)
                                            .hint_text(hint)
                                            .font(egui::TextStyle::Body),
                                    )
                                })
                                .inner;
                            if want_focus {
                                resp.request_focus();
                            }
                            if resp.changed() && !self.provider.is_llm() {
                                self.dirty = Some(Instant::now());
                            }
                            if resp.lost_focus()
                                && ui.input(|i| i.key_pressed(egui::Key::Enter))
                            {
                                do_search = true;
                            }
                        },
                    );
                });
            });

        if want_focus {
            self.focus = None;
        }
        if self.facets.sort != sort_before {
            self.page = 1;
            self.dirty = Some(Instant::now());
        }
        if toggle_filters {
            self.toggle_filters();
        }
        if refresh {
            self.resanitize();
            self.page = 1;
            self.dirty = Some(Instant::now());
        }
        if do_search {
            self.page = 1;
            self.launch_search(true);
        }
    }

    /// Barra di stato LUMEN.
    fn status_bar(&mut self, ui: &mut egui::Ui) {
        egui::Panel::bottom("status")
            .exact_size(28.0)
            .frame(egui::Frame::NONE.fill(lumen::INK))
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [r.left_top(), r.right_top()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.add_space(14.0);
                    let busy = self.loading
                        || self.detail_loading
                        || self.chat_busy
                        || self.update.checking
                        || self.detecting
                        || self.ai_checking
                        || self.imdb_busy;
                    let (color, state) = if busy {
                        (lumen::EMBER, "LAVORO")
                    } else {
                        match self.net {
                            Some(false) => (lumen::DANGER, "OFFLINE"),
                            Some(true) => (lumen::CELADON, "PRONTO"),
                            None => (lumen::FAINT, "PRONTO"),
                        }
                    };
                    let (dot, _) = ui.allocate_exact_size(
                        egui::vec2(6.0, 6.0),
                        egui::Sense::hover(),
                    );
                    ui.painter().circle_filled(dot.center(), 2.5, color);
                    ui.label(lumen::mono(9.0, state).color(lumen::MUTED));
                    ui.label(lumen::mono(9.0, "·").color(lumen::FAINT));
                    ui.label(
                        lumen::mono(
                            9.0,
                            if self.tmdb.is_some() {
                                "TMDB"
                            } else {
                                "TMDB NON CONFIGURATO"
                            },
                        )
                        .color(lumen::FAINT),
                    );
                    ui.label(lumen::mono(9.0, "·").color(lumen::FAINT));
                    ui.label(lumen::mono(9.0, self.provider_short()).color(lumen::FAINT));
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.add_space(14.0);
                            ui.label(
                                lumen::mono(
                                    9.0,
                                    format!("v{}", update::current_version()),
                                )
                                .color(lumen::MUTED),
                            );
                            if self.update.available.is_some() {
                                ui.label(
                                    lumen::mono(9.0, "AGGIORNAMENTO")
                                        .color(lumen::EMBER),
                                );
                            }
                        },
                    );
                });
            });
    }


    fn left_panel(&mut self, ui: &mut egui::Ui) {
        // Il pannello e uno strumento della griglia: in chat sparisce del
        // tutto, ma lo stato (aperto, filtri, larghezza) resta intatto.
        if self.mode != ViewMode::Griglia || !self.filters_open || self.show_settings {
            return;
        }
        let (left_max, _) = side_panel_widths(ui.ctx().content_rect().width());
        egui::Panel::left("facets")
            .resizable(true)
            .default_size(FILTERS_WIDTH.min(left_max))
            .size_range(260.0..=left_max)
            .frame(
                egui::Frame::NONE
                    .fill(lumen::INK)
                    .inner_margin(egui::Margin::symmetric(12, 10)),
            )
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [r.right_top(), r.right_bottom()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
                // La testata (titolo, ricerca, filtri attivi) sta fuori dallo
                // ScrollArea: resta ferma mentre si scorrono le sezioni.
                let mut actions = facets::header(
                    ui,
                    &self.tax,
                    &self.facets,
                    &mut self.facet_query,
                    &mut self.guide_open,
                );
                ui.add_space(4.0);
                let (line, _) = ui.allocate_exact_size(
                    egui::vec2(ui.available_width(), 1.0),
                    egui::Sense::hover(),
                );
                ui.painter().rect_filled(
                    line,
                    lumen::radius(0.0),
                    theme::alpha(theme::BORDER, 170),
                );
                ui.add_space(2.0);
                egui::ScrollArea::vertical()
                    .id_salt("facets-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        actions.extend(facets::sections(
                            ui,
                            &self.tax,
                            &self.facets,
                            &self.prune,
                            &mut self.facet_query,
                            &mut self.guide_open,
                            &self.messages,
                            &self.info,
                            &self.ai_notes,
                        ));
                    });
                if !actions.is_empty() {
                    self.apply_actions(actions);
                }
            });
    }

    fn right_panel(&mut self, ui: &mut egui::Ui) {
        // La scheda appartiene alla griglia; la chat resta a tutta larghezza.
        if self.mode != ViewMode::Griglia || self.show_settings {
            return;
        }
        // I filtri hanno la precedenza: mai due pannelli aperti insieme.
        if self.filters_open {
            return;
        }
        let (_, right_max) = side_panel_widths(ui.ctx().content_rect().width());
        let Some(d) = self.detail.clone() else {
            if self.detail_loading {
                egui::Panel::right("detail_loading")
                    .default_size(380.0_f32.min(right_max))
                    .frame(
                        egui::Frame::NONE
                            .fill(lumen::INK)
                            .inner_margin(egui::Margin::same(14)),
                    )
                    .show(ui, |ui| {
                        // Segnaposto animati: la scheda sta arrivando.
                        let now = ui.ctx().input(|i| i.time);
                        ui.add_space(8.0);
                        let w = ui.available_width();
                        let (poster, _) =
                            ui.allocate_exact_size(egui::vec2(w, 170.0), egui::Sense::hover());
                        fx::shimmer(ui, poster, 14.0, now);
                        ui.add_space(12.0);
                        for (i, part) in [0.9_f32, 0.6, 0.75, 0.4].iter().enumerate() {
                            let (line, _) = ui
                                .allocate_exact_size(egui::vec2(w * part, 13.0), egui::Sense::hover());
                            fx::shimmer(ui, line, 6.0, now + 0.25 * i as f64);
                            ui.add_space(7.0);
                        }
                        ui.add_space(4.0);
                        ui.label(
                            egui::RichText::new("Carico la scheda…")
                                .size(11.5)
                                .color(theme::DIM),
                        );
                    });
            }
            return;
        };
        egui::Panel::right("detail")
            .resizable(true)
            .default_size(392.0_f32.min(right_max))
            .size_range(280.0..=right_max)
            .frame(
                egui::Frame::NONE
                    .fill(lumen::INK)
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [r.left_top(), r.left_bottom()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
                let track = self.simkl_track(&d.movie);
                let actions = detail::panel(
                    ui,
                    &self.images,
                    &d,
                    false,
                    &track,
                );
                self.apply_detail_actions(actions, d.movie.id, &d.movie.title, false);
            });
    }

    /// Anteprima centrale della scheda in chat: un dialogo sopra la
    /// conversazione, senza cambiare vista.
    fn chat_detail_preview(&mut self, ctx: &egui::Context) {
        if self.mode != ViewMode::Chat || self.show_settings || self.guide_open {
            return;
        }
        if self.detail.is_none() && !self.detail_loading {
            return;
        }
        let screen = ctx.content_rect();
        let area_rect = egui::Rect::from_min_max(
            egui::pos2(screen.left() + 72.0, screen.top() + 48.0),
            egui::pos2(screen.right(), screen.bottom() - 28.0),
        );
        let modal_w = (area_rect.width() - 48.0).clamp(300.0, 440.0);
        let modal_h = (area_rect.height() - 24.0).clamp(280.0, 520.0);
        let modal = egui::Rect::from_center_size(
            area_rect.center(),
            egui::vec2(modal_w, modal_h),
        );
        let d = self.detail.clone();
        let mut actions = Vec::new();
        egui::Area::new(egui::Id::unique("chat-detail-preview"))
            .order(egui::Order::Foreground)
            .fixed_pos(area_rect.min)
            .show(ctx, |ui| {
                ui.set_clip_rect(area_rect);
                ui.painter().rect_filled(
                    area_rect,
                    egui::CornerRadius::ZERO,
                    lumen::alpha(lumen::INK, 205),
                );
                // Blocca la chat sotto il velo.
                let _ = ui.interact(
                    area_rect,
                    egui::Id::unique("chat-detail-scrim"),
                    egui::Sense::click(),
                );
                ui.painter().rect_filled(
                    modal,
                    lumen::radius(lumen::RADIUS),
                    lumen::INK,
                );
                ui.painter().rect_stroke(
                    modal,
                    lumen::radius(lumen::RADIUS),
                    egui::Stroke::new(1.0_f32, lumen::BORDER_HI), egui::StrokeKind::Inside);
                let inner = modal.shrink(16.0);
                if let Some(d) = &d {
                    let track = self.simkl_track(&d.movie);
                    ui.scope_builder(
                        egui::UiBuilder::new().max_rect(inner),
                        |ui| {
                            ui.set_min_height(inner.height());
                            actions = detail::panel(
                                ui,
                                &self.images,
                                d,
                                true,
                                &track,
                            );
                        },
                    );
                } else {
                    ui.painter().text(
                        modal.center(),
                        egui::Align2::CENTER_CENTER,
                        "Carico la scheda…",
                        egui::FontId::new(9.0, egui::FontFamily::Name(crate::ui::fonts::LUMEN_MONO.into())),
                        lumen::FAINT,
                    );
                }
            });
        if let Some(d) = &d {
            self.apply_detail_actions(actions, d.movie.id, &d.movie.title, true);
        }
    }

    fn central(&mut self, ui: &mut egui::Ui) {
        let chat = self.mode == ViewMode::Chat && !self.show_settings;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::TRANSPARENT)
                    // In chat il composer arriva fino al piè di pagina; la
                    // griglia conserva il suo respiro sopra e sotto.
                    .inner_margin(egui::Margin::symmetric(
                        if chat { 0 } else { 14 },
                        if chat { 0 } else { 10 },
                    )),
            )
            .show(ui, |ui| {
                if self.show_settings {
                    self.settings_page(ui);
                    return;
                }
                if self.tmdb.is_none() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(
                            egui::RichText::new("Serve la chiave API di TMDB")
                                .size(20.0)
                                .strong(),
                        );
                        ui.add_space(6.0);
                        ui.label(
                            egui::RichText::new(
                                "Serve una credenziale gratuita di TMDB: la API Key v3 (32 caratteri)\noppure il Read Access Token v4 (inizia con eyJ). La riconosco da sola.",
                            )
                            .size(12.5)
                            .color(theme::DIM),
                        );
                        ui.add_space(10.0);
                        if ui
                            .add(
                                egui::Button::new(
                                    egui::RichText::new("Apri impostazioni").size(13.0),
                                )
                                .fill(theme::ACCENT)
                                .corner_radius(lumen::radius(10.0)),
                            )
                            .clicked()
                        {
                            self.show_settings = true;
                        }
                        if ui
                            .hyperlink_to(
                                egui::RichText::new("themoviedb.org → API").size(12.0),
                                "https://www.themoviedb.org/settings/api",
                            )
                            .clicked()
                        {
                            open_url("https://www.themoviedb.org/settings/api");
                        }
                    });
                    return;
                }

                if self.mode == ViewMode::Chat {
                    let ctx = ui.ctx().clone();
                    self.chat_central(ui, &ctx);
                } else {
                    self.grid_central(ui);
                }
            });
    }

    /// Casa e conversazione LUMEN. Il composer della conversazione vive in un
    /// pannello fisso sopra la barra di stato; la casa ha il comando in alto.
    fn chat_central(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        self.ensure_fonts_for_results();
        self.ensure_simkl_posters();
        let now = ctx.input(|i| i.time);
        if self.chat.is_empty() {
            self.maybe_refresh_now_playing();
            self.refresh_home_suggestions(now);
            let want_focus = self.focus == Some(ViewMode::Chat);
            let resume_cards = self.simkl_resume_cards();
            let watchlist_cards = self.simkl_watchlist_cards();
            let actions = chat::home(
                ui,
                &self.images,
                &self.home_now_playing,
                &mut self.chat_draft,
                self.chat_busy,
                self.provider.is_llm(),
                want_focus,
                &self.tax,
                &self.suggestions,
                &resume_cards,
                &watchlist_cards,
            );
            if want_focus {
                self.focus = None;
            }
            if let Some(id) = actions.open_detail {
                self.open_detail(id);
            }
            self.suggestion_hovered = actions.suggestions_hovered;
            if let Some(text) = actions.prefill {
                self.chat_draft = text;
                self.focus = Some(ViewMode::Chat);
            }
            if actions.surprise {
                let visible: Vec<String> = self
                    .suggestions
                    .iter()
                    .map(|s| s.prompt.clone())
                    .collect();
                self.chat_draft = suggestions::surprise(
                    &self.suggestion_pool,
                    &visible,
                    self.suggestion_cycle,
                );
                self.focus = Some(ViewMode::Chat);
            }
            if actions.send {
                self.send_chat();
            }
            if !self.suggestion_hovered {
                let wait = (self.suggestion_next_at - now).max(0.05);
                ctx.request_repaint_after(Duration::from_secs_f64(wait));
            }
        } else {
            let actions = {
                let images = &self.images;
                let tax = &self.tax;
                let entries = &self.chat;
                let busy = self.chat_busy;
                let jump = &mut self.chat_jump;
                chat::view(
                    ui, images, tax, entries, busy, now, jump, 20.0,
                )
            };
            self.apply_chat_actions(actions);
        }
    }

    /// Composer della conversazione: pannello fisso sopra la barra di stato,
    /// allineato alla colonna dei risultati.
    fn chat_composer(&mut self, ui: &mut egui::Ui) {
        if self.mode != ViewMode::Chat
            || self.chat.is_empty()
            || self.show_settings
            || self.guide_open
        {
            return;
        }
        let avail = (ui.ctx().content_rect().width() - 72.0).max(200.0);
        let pad = if avail < 900.0 { 20.0 } else { 34.0 };
        let width = (avail - pad * 2.0).min(1100.0);
        let want_focus = self.focus == Some(ViewMode::Chat);
        let mut send = false;
        // ID diverso dal TextEdit "chat-composer": se coincidono, lo stato del
        // pannello sovrascrive il fuoco del campo e non si riesce piu a scrivere.
        egui::Panel::bottom("chat-composer-panel")
            .exact_size(64.0)
            .frame(egui::Frame::NONE.fill(lumen::INK))
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [r.left_top(), r.right_top()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
                ui.add_space(11.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(pad);
                    if chat::composer(
                        ui,
                        &mut self.chat_draft,
                        self.chat_busy,
                        want_focus,
                        width,
                    ) {
                        send = true;
                    }
                });
            });
        if want_focus {
            self.focus = None;
        }
        if send {
            self.send_chat();
        }
    }


    /// Griglia a tutta altezza: conteggio e comandi stanno nella barra di
    /// comando, quindi qui restano i poster.
    fn grid_central(&mut self, ui: &mut egui::Ui) {
        if !self.loading && self.movies.len() <= 3 && !self.unlocks.is_empty() {
            if let Some(i) = unlock::strip(ui, &self.unlocks) {
                self.apply_unlock(i);
                return;
            }
            ui.add_space(12.0);
        }
        let selected = self.detail.as_ref().map(|d| d.movie.id);
        let reset_scroll = std::mem::take(&mut self.grid_reset_scroll);
        let clicked = grid::grid(ui, &self.images, &self.movies, selected, reset_scroll);
        if let Some(id) = clicked {
            self.open_detail(id);
        }
        if self.movies.is_empty() && !self.loading {
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                ui.label(
                    egui::RichText::new("Nessun risultato con questi vincoli")
                        .size(15.0)
                        .color(theme::DIM),
                );
                ui.label(
                    egui::RichText::new(
                        "Togli una faccia dal pannello: il motore di vincoli spiega cosa era in conflitto.",
                    )
                    .size(12.0)
                    .color(theme::DIM),
                );
            });
        }
    }

    /// Pager fisso sopra la barra di stato: pagina corrente a sinistra,
    /// frecce a destra. Non ruba spazio quando c'è una pagina sola.
    fn grid_pager(&mut self, ui: &mut egui::Ui) {
        if self.mode != ViewMode::Griglia || self.total_pages <= 1 || self.show_settings {
            return;
        }
        let pages = self.total_pages.max(1);
        let page = self.page.min(pages);
        let prev = page > 1 && !self.loading;
        let next = page < pages && !self.loading;
        let mut target = None;
        egui::Panel::bottom("grid-pager")
            .exact_size(36.0)
            .frame(egui::Frame::NONE.fill(lumen::INK))
            .show(ui, |ui| {
                let r = ui.max_rect();
                ui.painter().line_segment(
                    [r.left_top(), r.right_top()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.label(
                        lumen::mono(9.0, format!("PAGINA {page} DI {pages}"))
                            .color(lumen::FAINT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if pager_button(
                            ui,
                            lumen::Icon::ArrowRight,
                            next,
                            "Pagina successiva",
                        )
                        .clicked()
                        {
                            target = Some((page + 1).min(pages));
                        }
                        ui.add_space(6.0);
                        if pager_button(
                            ui,
                            lumen::Icon::ArrowLeft,
                            prev,
                            "Pagina precedente",
                        )
                        .clicked()
                        {
                            target = Some(page.saturating_sub(1));
                        }
                    });
                });
            });
        if let Some(page) = target {
            self.page = page;
            self.spawn_search(false);
        }
    }

    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let mut close = false;
        let mut save = false;
        let mut check = false;
        let mut detect = false;
        let mut probe = false;
        let mut open_api_page = false;
        let mut open_deepseek_page = false;
        let mut open_openrouter_page = false;
        let mut update_check = false;
        let mut update_download = false;
        let mut update_install = false;
        let mut simkl_connect = false;
        let mut simkl_sync = false;
        let mut simkl_disconnect = false;
        let mut simkl_cancel = false;
        egui::ScrollArea::vertical()
            .id_salt("settings-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = ui.available_width().min(760.0);
                let pad = ((ui.available_width() - width) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(pad);
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.add_space(24.0);
                            ui.horizontal(|ui| {
                                ui.vertical(|ui| {
                                    ui.label(
                                        lumen::mono(10.0, "IMPOSTAZIONI").color(lumen::EMBER),
                                    );
                                    ui.add_space(5.0);
                                    ui.label(lumen::display(30.0, "Preferenze"));
                                });
                                ui.with_layout(
                                    egui::Layout::right_to_left(egui::Align::Center),
                                    |ui| {
                                        if lumen::link(ui, "Chiudi ×", lumen::MUTED).clicked() {
                                            close = true;
                                        }
                                    },
                                );
                            });
                            ui.add_space(4.0);
                            ui.label(
                                lumen::body(
                                    11.5,
                                    "Chiavi, interprete, tracciamento e aggiornamenti restano su questo PC.",
                                )
                                .color(lumen::MUTED),
                            );
                            ui.add_space(14.0);
                            lumen::hairline(ui);
                            ui.add_space(14.0);
                let connected = self.simkl_tokens.is_some();
                ui.horizontal(|ui| {
                    ui.label(lumen::mono(10.0, "TRACCIAMENTO").color(lumen::IVORY));
                    if connected {
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if lumen::button_sized(
                                    ui,
                                    "Sincronizza ora",
                                    !self.simkl_syncing,
                                    132.0,
                                )
                                .clicked()
                                {
                                    simkl_sync = true;
                                }
                            },
                        );
                    }
                });
                ui.label(
                    lumen::body(
                        11.5,
                        "CineMatch legge e aggiorna visti, lista e voti.",
                    )
                    .color(lumen::MUTED),
                );
                ui.add_space(6.0);
                if let Some(status) = &self.simkl_status {
                    ui.label(lumen::mono(10.0, status).color(lumen::CELADON));
                }
                if let Some(err) = &self.simkl_error {
                    ui.label(lumen::body(11.0, err).color(lumen::DANGER));
                }
                if self.simkl_syncing {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(
                            egui::RichText::new("Sincronizzo la libreria…")
                                .size(10.5)
                                .color(theme::DIM),
                        );
                    });
                }
                if let Some(code) = &self.simkl_auth {
                    ui.add_space(6.0);
                    ui.label(lumen::display(20.0, "Collega SIMKL"));
                    ui.label(
                        lumen::body(
                            11.5,
                            "Apri simkl.com/pin e inserisci questo codice:",
                        )
                        .color(lumen::MUTED),
                    );
                    ui.add_space(2.0);
                    ui.label(lumen::mono(24.0, &code.user_code).color(lumen::EMBER));
                    let link = if code.verification_uri_complete.is_empty() {
                        code.verification_uri.clone()
                    } else {
                        code.verification_uri_complete.clone()
                    };
                    if lumen::link(ui, "Apri la pagina di SIMKL", lumen::EMBER).clicked() {
                        open_url(&link);
                    }
                    if lumen::link(ui, "Annulla", lumen::MUTED).clicked() {
                        simkl_cancel = true;
                    }
                } else if connected {
                    ui.label(
                        lumen::mono(
                            10.0,
                            format!(
                                "{} VISTI · {} DA VEDERE · {}",
                                self.simkl_store.watched(),
                                self.simkl_store.watchlist(),
                                ago_label(self.simkl_store.last_sync)
                            ),
                        )
                        .color(lumen::FAINT),
                    );
                    ui.add_space(2.0);
                    if lumen::link(ui, "Scollega SIMKL", lumen::MUTED).clicked() {
                        simkl_disconnect = true;
                    }
                } else {
                    ui.label(
                        lumen::body(
                            11.5,
                            "Collega il tuo account con un codice. CineMatch legge visti, lista e voti; il token resta nel Windows Credential Manager.",
                        )
                        .color(lumen::MUTED),
                    );
                    ui.add_space(4.0);
                    if lumen::button_sized(ui, "Connetti SIMKL", true, 168.0).clicked() {
                        simkl_connect = true;
                    }
                    if !self.simkl_store.movies.is_empty() {
                        ui.add_space(2.0);
                        ui.label(
                            lumen::mono(
                                9.5,
                                format!(
                                    "Copia locale: {} film (in attesa di ricollegare SIMKL)",
                                    self.simkl_store.movies.len()
                                ),
                            )
                            .color(lumen::FAINT),
                        );
                    }
                }
                ui.add_space(6.0);
                ui.checkbox(
                    &mut self.settings.simkl_hide_watched,
                    egui::RichText::new("Nascondi i film già visti in griglia e chat")
                        .size(11.0)
                        .color(theme::DIM),
                );
                ui.add_space(14.0);
                lumen::hairline(ui);
                ui.add_space(14.0);
                if update::enabled() {
                    ui.horizontal(|ui| {
                        ui.label(lumen::mono(10.0, "AGGIORNAMENTI").color(lumen::IVORY));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if self.update.checking {
                                    ui.spinner();
                                    ui.label(
                                        egui::RichText::new("Controllo…")
                                            .size(10.5)
                                            .color(theme::DIM),
                                    );
                                } else if ui
                                    .add_enabled(
                                        self.update.progress.is_none(),
                                        egui::Button::new(
                                            egui::RichText::new("Controlla").size(11.0),
                                        )
                                        .corner_radius(lumen::radius(9.0)),
                                    )
                                    .clicked()
                                {
                                    update_check = true;
                                }
                            },
                        );
                    });
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(format!(
                                "CineMatch {}",
                                update::current_version()
                            ))
                            .size(11.0)
                            .color(theme::DIM),
                        );
                        if let Some(m) = &self.update.available {
                            ui.label(
                                egui::RichText::new(format!("→ {}", m.version))
                                    .size(11.0)
                                    .strong()
                                    .color(theme::ACCENT_2),
                            );
                        }
                    });
                    if let Some(m) = self.update.available.clone() {
                        if !m.notes.trim().is_empty() {
                            ui.label(
                                egui::RichText::new(theme::truncate(&m.notes, 220))
                                    .size(10.5)
                                    .color(theme::DIM),
                            );
                        }
                        if !m.notes_url.is_empty() {
                            ui.hyperlink_to(
                                egui::RichText::new("Note di versione")
                                    .size(10.5)
                                    .color(theme::ACCENT_3),
                                &m.notes_url,
                            );
                        }
                        if let Some((done, total)) = self.update.progress {
                            let fraction = if total > 0 {
                                (done as f32 / total as f32).clamp(0.0, 1.0)
                            } else {
                                0.0
                            };
                            ui.add(
                                egui::ProgressBar::new(fraction)
                                    .show_percentage()
                                    .desired_width(ui.available_width()),
                            );
                            ui.label(
                                egui::RichText::new(format!(
                                    "{:.1} / {:.1} MB",
                                    done as f64 / 1_048_576.0,
                                    total as f64 / 1_048_576.0
                                ))
                                .size(10.0)
                                .color(theme::DIM),
                            );
                        } else if self.update.downloaded.is_some() {
                            if theme::primary_button(
                                ui,
                                "Installa e riavvia",
                                egui::vec2(150.0, 26.0),
                            )
                            .clicked()
                            {
                                update_install = true;
                            }
                            ui.label(
                                egui::RichText::new(
                                    "Il setup conserva configurazione, chiavi e dati locali.",
                                )
                                .size(10.0)
                                .color(theme::DIM),
                            );
                        } else if theme::primary_button(
                            ui,
                            "Scarica e installa",
                            egui::vec2(150.0, 26.0),
                        )
                        .clicked()
                        {
                            update_download = true;
                        }
                    }
                    if let Some(err) = &self.update.error {
                        ui.label(
                            egui::RichText::new(theme::truncate(err, 180))
                                .size(10.5)
                                .color(theme::DANGER),
                        );
                    } else if let Some(status) = &self.update.status {
                        ui.label(
                            egui::RichText::new(theme::truncate(status, 180))
                                .size(10.5)
                                .color(theme::DIM),
                        );
                    }
                    ui.add_space(8.0);
                    ui.separator();
                }
                ui.label(lumen::mono(10.0, "TMDB").color(lumen::EMBER));
                ui.add_space(2.0);
                ui.label(
                    egui::RichText::new("Credenziale TMDB: API Key v3 (32 caratteri) oppure Read Access Token v4 (inizia con eyJ)")
                        .size(11.0)
                        .color(theme::DIM),
                );
                ui.add(
                    egui::TextEdit::singleline(&mut self.settings.tmdb_key)
                        .password(true)
                        .desired_width(f32::INFINITY),
                );
                {
                    let (hint, plausible) = describe_credential(&self.settings.tmdb_key);
                    let color = if self.settings.tmdb_key.trim().is_empty() {
                        theme::DIM
                    } else if plausible {
                        theme::OK
                    } else {
                        theme::WARN
                    };
                    ui.label(egui::RichText::new(hint).size(10.5).color(color));
                }
                if let Some(check) = &self.settings_check {
                    let color = if check.starts_with('✔') {
                        theme::OK
                    } else if check.starts_with("Verifica") {
                        theme::DIM
                    } else {
                        theme::DANGER
                    };
                    ui.label(egui::RichText::new(theme::truncate(check, 150)).size(10.5).color(color));
                }
                if ui
                    .hyperlink_to(
                        egui::RichText::new("Ottieni una chiave gratuita ↗").size(11.0),
                        "https://www.themoviedb.org/settings/api",
                    )
                    .clicked()
                {
                    open_api_page = true;
                }

                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(lumen::display(24.0, "Interprete delle richieste"));
                ui.add_space(2.0);
                ui.label(
                    lumen::body(
                        11.0,
                        "Le regole funzionano offline. Un modello è opzionale: i film arrivano sempre da TMDB.",
                    )
                    .color(lumen::MUTED),
                );
                ui.add_space(6.0);
                let previous_provider = self.settings.provider.clone();
                ui.horizontal_wrapped(|ui| {
                    for (id, label) in [
                        ("rules", "Regole offline"),
                        ("deepseek", "DeepSeek"),
                        ("openrouter", "OpenRouter"),
                        ("ollama", "Ollama (locale)"),
                    ] {
                        ui.selectable_value(
                            &mut self.settings.provider,
                            id.to_string(),
                            egui::RichText::new(label).size(11.5),
                        );
                    }
                });
                if self.settings.provider != previous_provider {
                    self.models_gen = self.models_gen.wrapping_add(1);
                    self.models.clear();
                    self.models_status = None;
                    self.ai_check = None;
                    self.detecting = false;
                    detect = self.settings.can_detect();
                }

                if self.settings.provider == "deepseek" {
                    ui.label(
                        egui::RichText::new("Chiave API DeepSeek (resta su questo PC)")
                            .size(11.0)
                            .color(theme::DIM),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.deepseek_key)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                    if ui
                        .hyperlink_to(
                            egui::RichText::new("Crea o copia la chiave su platform.deepseek.com")
                                .size(10.5),
                            "https://platform.deepseek.com/api_keys",
                        )
                        .clicked()
                    {
                        open_deepseek_page = true;
                    }
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Modello DeepSeek").size(11.0).color(theme::DIM));
                    model_picker(
                        ui,
                        &mut self.settings.deepseek_model,
                        &self.models,
                        DEEPSEEK_MODEL,
                    );
                } else if self.settings.provider == "openrouter" {
                    ui.label(
                        egui::RichText::new("Chiave API OpenRouter (resta su questo PC)")
                            .size(11.0)
                            .color(theme::DIM),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.openrouter_key)
                            .password(true)
                            .desired_width(f32::INFINITY),
                    );
                    if ui
                        .hyperlink_to(
                            egui::RichText::new("Crea o copia la chiave su openrouter.ai")
                                .size(10.5),
                            "https://openrouter.ai/keys",
                        )
                        .clicked()
                    {
                        open_openrouter_page = true;
                    }
                    ui.add_space(4.0);
                    ui.label(
                        egui::RichText::new("Modello OpenRouter")
                            .size(11.0)
                            .color(theme::DIM),
                    );
                    model_picker(
                        ui,
                        &mut self.settings.openrouter_model,
                        &self.models,
                        OPENROUTER_MODEL,
                    );
                    ui.label(
                        egui::RichText::new(
                            "L'elenco mostra tutti i modelli pubblicati da OpenRouter: scegli quello che vuoi usare.",
                        )
                        .size(10.0)
                        .color(theme::DIM),
                    );
                } else if self.settings.provider == "ollama" {
                    ui.label(
                        egui::RichText::new("Collegamento Ollama")
                            .size(11.0)
                            .color(theme::DIM),
                    );
                    ui.add(
                        egui::TextEdit::singleline(&mut self.settings.ollama_url)
                            .hint_text("http://localhost:11434")
                            .desired_width(f32::INFINITY),
                    );
                    ui.add_space(4.0);
                    ui.label(egui::RichText::new("Modello Ollama").size(11.0).color(theme::DIM));
                    model_picker(
                        ui,
                        &mut self.settings.ollama_model,
                        &self.models,
                        "qwen2.5:3b",
                    );
                }

                if self.settings.provider == "deepseek" {
                    ui.add_space(6.0);
                    effort_picker(ui, &mut self.settings.effort, None);
                } else if self.settings.provider == "openrouter" {
                    ui.add_space(6.0);
                    let model = self
                        .models
                        .iter()
                        .find(|m| m.id == self.settings.openrouter_model)
                        .cloned();
                    effort_picker(ui, &mut self.settings.effort, model.as_ref());
                }

                if self.settings.provider != "rules" {
                    ui.add_space(6.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        let detect_btn = ui
                            .add_enabled_ui(!self.detecting, |ui| {
                                lumen::button_sized(ui, "Rileva modelli", false, 130.0)
                            })
                            .inner;
                        if detect_btn.clicked() {
                            detect = true;
                        }
                        let probe_btn = ui
                            .add_enabled_ui(!self.ai_checking, |ui| {
                                lumen::button_sized(ui, "Verifica risposta AI", false, 170.0)
                            })
                            .inner;
                        if probe_btn.clicked() {
                            probe = true;
                        }
                    });
                    if let Some(status) = &self.models_status {
                        let color = if status.starts_with('✔') {
                            theme::OK
                        } else if status.starts_with("Interrogo") {
                            theme::DIM
                        } else {
                            theme::DANGER
                        };
                        ui.label(
                            egui::RichText::new(theme::truncate(status, 160))
                                .size(10.5)
                                .color(color),
                        );
                    }
                    if let Some(status) = &self.ai_check {
                        let color = if status.starts_with('✔') {
                            theme::OK
                        } else if status.starts_with("Prova") {
                            theme::DIM
                        } else {
                            theme::DANGER
                        };
                        ui.label(
                            egui::RichText::new(theme::truncate(status, 160))
                                .size(10.5)
                                .color(color),
                        );
                    }
                }

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label(lumen::label("Lingua"));
                    for (id, label) in [("it-IT", "Italiano"), ("en-US", "Inglese")] {
                        ui.selectable_value(
                            &mut self.settings.language,
                            id.to_string(),
                            egui::RichText::new(label).size(11.5),
                        );
                    }
                });

                // ---- dati locali: voti IMDb ----
                ui.add_space(8.0);
                ui.separator();
                ui.add_space(4.0);
                ui.label(lumen::mono(10.0, "DATI LOCALI").color(lumen::IVORY));
                ui.add_space(2.0);
                let ready = crate::infra::imdb::is_ready();
                ui.horizontal(|ui| {
                    let label = if ready {
                        "Aggiorna i voti IMDb (7 MB)"
                    } else {
                        "Scarica i voti IMDb (7 MB)"
                    };
                    let imdb_btn = ui
                        .add_enabled_ui(!self.imdb_busy, |ui| {
                            lumen::button_sized(ui, label, false, 200.0)
                        })
                        .inner;
                    if imdb_btn
                        .on_hover_text(
                            "Dati pubblici IMDb (datasets.imdbws.com): nessuna chiave, nessun limite. Consente di mostrare il voto IMDb accanto a quello TMDB.",
                        )
                        .clicked()
                    {
                        self.start_imdb_download();
                    }
                    ui.add_space(8.0);
                    if ready {
                        ui.label(
                            egui::RichText::new(format!(
                                "✔ presenti · {:.0} MB",
                                crate::infra::imdb::data_size_mb()
                            ))
                            .size(10.5)
                            .color(theme::DIM),
                        );
                    } else {
                        ui.label(
                            egui::RichText::new("facoltativo")
                                .size(10.5)
                                .color(theme::DIM),
                        );
                    }
                });
                if self.imdb_busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        let text = match self.imdb_progress {
                            Some((done, total)) if total > 0 => format!(
                                "scarico e scompato: {:.1} / {:.1} MB",
                                done as f64 / 1_048_576.0,
                                total as f64 / 1_048_576.0
                            ),
                            Some((done, _)) => {
                                format!("scarico: {:.1} MB", done as f64 / 1_048_576.0)
                            }
                            None => "preparo il download…".to_string(),
                        };
                        ui.label(egui::RichText::new(text).size(10.5).color(theme::DIM));
                    });
                }
                if let Some(msg) = &self.imdb_status {
                    ui.label(egui::RichText::new(msg).size(10.5).color(theme::DIM));
                }

                ui.add_space(10.0);
                lumen::hairline(ui);
                ui.add_space(10.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    if lumen::button_sized(ui, "Salva", true, 100.0).clicked() {
                        save = true;
                    }
                    if lumen::button_sized(ui, "Verifica credenziale", false, 160.0).clicked()
                    {
                        check = true;
                    }
                    ui.label(
                        lumen::mono(9.0, format!("config: {}", Config::path().display()))
                            .color(lumen::FAINT),
                    );
                });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(lumen::mono(9.0, format!("build {}", self.build)).color(lumen::FAINT));
                    ui.add_space(10.0);
                    ui.label(
                        lumen::mono(
                            9.0,
                            "This product uses the TMDB API but is not endorsed or certified by TMDB.",
                        )
                        .color(lumen::MUTED),
                    );
                });
                ui.add_space(24.0);
                        },
                    );
                });
            });

        if open_api_page {
            open_url("https://www.themoviedb.org/settings/api");
        }
        if open_deepseek_page {
            open_url("https://platform.deepseek.com/api_keys");
        }
        if open_openrouter_page {
            open_url("https://openrouter.ai/keys");
        }
        if update_check {
            self.start_update_check(true);
        }
        if update_download {
            self.start_update_download();
        }
        if update_install {
            self.install_update();
        }
        if check {
            self.check_credentials();
        }
        if detect {
            self.detect_models();
        }
        if probe {
            self.check_ai();
        }
        if save {
            self.save_settings();
        }
        if simkl_connect {
            self.start_simkl_auth();
        }
        if simkl_sync {
            self.start_simkl_sync(false);
        }
        if simkl_disconnect {
            self.disconnect_simkl();
        }
        if simkl_cancel {
            self.cancel_simkl_auth();
        }
        if close {
            self.show_settings = false;
        }
    }

    fn toast_overlay(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = self.toast.clone() else { return };
        let t = at.elapsed().as_secs_f32();
        let life = 5.0_f32;
        if t > life {
            self.toast = None;
            return;
        }
        // Entra scivolando dal basso, esce in dissolvenza: niente scatti.
        let enter = (t / 0.22).clamp(0.0, 1.0);
        let fade = ((life - t) / 0.45).clamp(0.0, 1.0);
        let a = (enter * fade).clamp(0.0, 1.0);
        // In conversazione il composer occupa il fondo: il toast sale sopra.
        let lift = if self.mode == ViewMode::Chat
            && !self.chat.is_empty()
            && !self.show_settings
            && !self.guide_open
        {
            100.0
        } else {
            74.0
        };
        let y = -lift + (1.0 - enter) * 20.0;
        egui::Area::new(egui::Id::unique("toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, y))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                ui.set_opacity(a);
                egui::Frame::NONE
                    .fill(theme::mix(theme::CARD_HOVER, theme::PANEL, 0.35))
                    .stroke(egui::Stroke::new(1.0_f32, theme::alpha(theme::ACCENT, 150)))
                    .corner_radius(lumen::radius(13.0))
                    .shadow(egui::epaint::Shadow {
                        offset: [0, 12],
                        blur: 34,
                        spread: 0,
                        color: egui::Color32::from_black_alpha(150),
                    })
                    .inner_margin(egui::Margin::symmetric(14, 9))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            fx::dot(ui, theme::ACCENT, 0.0);
                            ui.label(
                                egui::RichText::new(theme::truncate(&msg, 140))
                                    .size(12.0)
                                    .color(theme::TEXT),
                            );
                        });
                    });
            });
        // Ridisegna solo per l'entrata, la dissolvenza e la scadenza: un toast
        // fermo non deve tenere sveglio il renderer.
        let wait = if t < 0.3 || t > life - 0.5 {
            Duration::from_millis(40)
        } else {
            Duration::from_secs_f64((life - 0.5 - t).max(0.05) as f64)
        };
        ctx.request_repaint_after(wait);
    }
}

impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.update_ui(ui, frame);
    }
}

impl App {
    fn update_ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let ctx = ui.ctx().clone();
        self.poll();
        if self.images.poll() {
            ctx.request_repaint();
        }
        self.tick();
        window_resize_edges(&ctx);
        let now = ctx.input(|i| i.time);
        fx::background(&ctx, now);
        self.shortcuts(&ctx);
        if !self.boot_done {
            self.boot_done = true;
            if self.tmdb.is_some() {
                self.load_trending();
            } else {
                self.show_settings = true;
            }
        }
        self.maybe_check_updates();
        self.header(ui);
        self.status_bar(ui);
        self.rail(ui);
        self.grid_toolbar(ui);
        self.chat_composer(ui);
        self.left_panel(ui);
        self.right_panel(ui);
        self.grid_pager(ui);
        self.central(ui);
        self.chat_detail_preview(&ctx);
        if self.guide_open {
            let actions = facets::guide_window(&ctx, &self.tax, &mut self.guide_open);
            if !actions.is_empty() {
                self.apply_actions(actions);
            }
        }
        self.toast_overlay(&ctx);
        if self.loading
            || self.detail_loading
            || self.chat_busy
            || self.update.checking
            || self.update.progress.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }
}

impl App {
    /// Scorciatoie globali: «/» porta il fuoco nel campo della vista attiva,
    /// Ctrl+1/2 cambiano vista, Ctrl+B apre i filtri solo nella griglia.
    fn shortcuts(&mut self, ctx: &egui::Context) {
        let typing = ctx.memory(|m| m.focused().is_some());
        // «/» e' un tasto fisico su alcune tastiere e Shift+7 su quella
        // italiana: riconosciamo entrambi e togliamo il carattere dagli eventi,
        // cosi' la scorciatoia non finisce dentro al campo appena focalizzato.
        let slash = ctx.input(|i| {
            i.key_pressed(egui::Key::Slash)
                || i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Text(t) if t == "/"))
        });
        if !typing && slash {
            self.focus = Some(self.mode);
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Slash);
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(_)));
            });
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num1)) {
            self.switch_mode(ViewMode::Chat);
        }
        if ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::Num2)) {
            self.switch_mode(ViewMode::Griglia);
        }
        // Ctrl+B resta attivo anche con un widget a fuoco: e' il toggle
        // della sidebar, non un comando di testo. In chat non esiste.
        if self.mode == ViewMode::Griglia
            && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::B))
        {
            self.toggle_filters();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.guide_open {
                self.guide_open = false;
            } else if self.show_settings {
                self.show_settings = false;
            } else if self.detail.is_some() || self.detail_loading {
                self.close_detail();
            } else if self.filters_open {
                self.filters_open = false;
            } else if let Some(id) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        }
    }
}

fn merge_themes(base: &mut Vec<Theme>, extra: &[Theme]) {
    for theme in extra {
        if let Some(existing) = base
            .iter_mut()
            .find(|t| t.label.eq_ignore_ascii_case(&theme.label))
        {
            for concept in &theme.concepts {
                if !existing
                    .concepts
                    .iter()
                    .any(|c| c.label.eq_ignore_ascii_case(&concept.label))
                {
                    existing.concepts.push(concept.clone());
                }
            }
            for title in &theme.seed_titles {
                if !existing.seed_titles.contains(title) {
                    existing.seed_titles.push(title.clone());
                }
            }
        } else {
            base.push(theme.clone());
        }
    }
    base.truncate(2);
}

fn merge_facets(base: &mut Facets, extra: &Facets, cleared: &Cleared) {
    for v in &extra.decades {
        if !base.decades.contains(v) {
            base.decades.push(v.clone());
        }
    }
    for v in &extra.movements {
        if !base.movements.contains(v) {
            base.movements.push(v.clone());
        }
    }
    for v in &extra.genres {
        if !base.genres.contains(v) {
            base.genres.push(*v);
        }
    }
    for v in &extra.without_genres {
        if !base.without_genres.contains(v) {
            base.without_genres.push(*v);
        }
    }
    // I paesi si sostituiscono: «solo italiani» significa cambiare paese,
    // non aggiungere l'Italia a un filtro precedente.
    if !extra.countries.is_empty() {
        base.countries = extra.countries.clone();
    }
    for v in &extra.moods {
        if !base.moods.contains(v) {
            base.moods.push(v.clone());
        }
    }
    for v in &extra.keywords {
        if !base.keywords.contains(v) {
            base.keywords.push(v.clone());
        }
    }
    if cleared.min_rating {
        base.min_rating = None;
    } else if extra.min_rating.is_some() {
        base.min_rating = extra.min_rating;
    }
    if cleared.max_rating {
        base.max_rating = None;
    } else if extra.max_rating.is_some() {
        base.max_rating = extra.max_rating;
    }
    if cleared.min_votes {
        base.min_votes = None;
    } else if extra.min_votes.is_some() {
        base.min_votes = extra.min_votes;
    }
    if cleared.runtime {
        base.runtime = None;
    } else if extra.runtime.is_some() {
        base.runtime = extra.runtime;
    }
    if extra.sort != Sort::default() {
        base.sort = extra.sort;
    }
}

/// Legge fino a `TITLE_PAGE_CAP` pagine di una ricerca per titolo quando i
/// filtri sono locali: TMDB non li applica e la prima pagina puo' non
/// contenerli.
fn search_title_pages(tmdb: &Tmdb, query: &str) -> Result<(Vec<Movie>, u32), String> {
    let mut raw = Vec::new();
    let mut total = 0u32;
    for page in 1..=TITLE_PAGE_CAP {
        let found = if page == 1 {
            tmdb.search(query, page)?
        } else {
            match tmdb.search(query, page) {
                Ok(found) => found,
                Err(_) => break,
            }
        };
        total = found.total_results;
        raw.extend(found.results.into_iter().map(Movie::from));
        if page >= found.total_pages {
            break;
        }
    }
    Ok((dedupe(raw), total))
}

/// Il collage "In sala" usa solo film con poster, al massimo sei.
fn in_sala_movies(movies: Vec<Movie>) -> Vec<Movie> {
    movies
        .into_iter()
        .filter(|m| m.poster_path.is_some())
        .take(6)
        .collect()
}

fn filter_movies(movies: Vec<Movie>, tax: &Taxonomy, f: &Facets) -> Vec<Movie> {
    if f.is_empty() {
        return movies;
    }
    // Usa la stessa traduzione della query server: generi, anno e paesi non
    // devono cambiare significato tra griglia e ricerca per titolo.
    let query = constraints::to_discover(tax, f, &[], 1).ok();
    let year = match &query {
        Some(q) => q.year,
        None => constraints::year_range(tax, f).ok().flatten(),
    };
    let genres: &[u32] = query
        .as_ref()
        .map(|q| q.genres.as_slice())
        .unwrap_or(f.genres.as_slice());
    let genres_or = query.as_ref().map(|q| q.genres_or).unwrap_or(false);
    let without: &[u32] = query
        .as_ref()
        .map(|q| q.without_genres.as_slice())
        .unwrap_or(f.without_genres.as_slice());
    let countries = query
        .as_ref()
        .map(|q| q.countries.clone())
        .unwrap_or_else(|| constraints::effective_countries(tax, f));
    let langs = countries
        .iter()
        .filter_map(|c| constraints::country_language(c))
        .collect::<Vec<&'static str>>();
    let min_rating = query.as_ref().and_then(|q| q.min_rating).or(f.min_rating);
    let max_rating = query.as_ref().and_then(|q| q.max_rating).or(f.max_rating);
    let min_votes = query.as_ref().and_then(|q| q.min_votes).or(f.min_votes);

    movies
        .into_iter()
        .filter(|m| {
            if let Some((a, b)) = year {
                match m.year {
                    Some(y) if y >= a && y <= b => {}
                    _ => return false,
                }
            }
            if !genres.is_empty() {
                let ok = if genres_or {
                    genres.iter().any(|g| m.genre_ids.contains(g))
                } else {
                    genres.iter().all(|g| m.genre_ids.contains(g))
                };
                if !ok {
                    return false;
                }
            }
            if m.genre_ids.iter().any(|g| without.contains(g)) {
                return false;
            }
            // Il paese non e un campo della ricerca per titolo: la lingua
            // originale e solo una stima. La UI lo dichiara.
            if !langs.is_empty() && !langs.contains(&m.lang.as_str()) {
                return false;
            }
            if let Some(r) = min_rating {
                if m.rating < r {
                    return false;
                }
            }
            // Un voto 0 significa «nessun voto»: non e un giudizio basso.
            if let Some(r) = max_rating {
                if m.rating > 0.0 && m.rating > r {
                    return false;
                }
            }
            if let Some(v) = min_votes {
                if m.votes < v {
                    return false;
                }
            }
            true
        })
        .collect()
}

/// Criteri che la ricerca per titolo applica solo in parte o non applica.
fn local_filter_limits(tax: &Taxonomy, f: &Facets) -> Vec<&'static str> {
    let mut out = Vec::new();
    if f.runtime.is_some() {
        out.push("durata");
    }
    if !f.moods.is_empty() {
        out.push("mood");
    }
    if !f.keywords.is_empty() {
        out.push("temi");
    }
    if !f.countries.is_empty() {
        out.push("paese (stimato dalla lingua)");
    }
    if constraints::decade_gap(tax, f) {
        out.push("periodi non contigui (la ricerca include gli anni in mezzo)");
    }
    out
}

/// Un solo vincolo tolto per volta: è il compromesso più piccolo che sblocca
/// altri film. Le varianti si ordinano per risultati, così vince la rinuncia
/// più leggera; le tre migliori bastano a scegliere senza una tabella.
fn unlock_options(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    facets: &Facets,
    kw: &[(String, u64)],
    current_total: u32,
    seen: &HashSet<u64>,
) -> Vec<unlock::Unlock> {
    let mut out = Vec::new();
    for (label, relaxed) in unlock_candidates(tax, facets) {
        let Ok(q) = constraints::to_discover(tax, &relaxed, kw, 1) else {
            continue;
        };
        let Ok(page) = tmdb.discover(&q) else {
            continue;
        };
        // Una variante che non aggiunge nemmeno un film non è un compromesso.
        if page.total_results <= current_total {
            continue;
        }
        let mut preview = page
            .results
            .iter()
            .filter(|m| !seen.contains(&m.id))
            .map(|m| m.title.clone())
            .take(2)
            .collect::<Vec<_>>();
        if preview.is_empty() {
            preview = page
                .results
                .iter()
                .take(2)
                .map(|m| m.title.clone())
                .collect();
        }
        out.push(unlock::Unlock {
            label,
            facets: relaxed,
            total: page.total_results,
            preview,
        });
    }
    out.sort_by_key(|u| u.total);
    out.truncate(3);
    out
}

/// Le rinunce possibili: ognuna tocca una sola categoria di filtri.
fn unlock_candidates(tax: &Taxonomy, f: &Facets) -> Vec<(String, Facets)> {
    let mut out = Vec::new();
    if let Some(v) = f.min_rating {
        let mut c = f.clone();
        c.min_rating = None;
        out.push((format!("Voto minimo {v:.1}"), c));
    }
    if let Some(v) = f.max_rating {
        let mut c = f.clone();
        c.max_rating = None;
        out.push((format!("Voto massimo {v:.1}"), c));
    }
    if let Some(v) = f.min_votes {
        let mut c = f.clone();
        c.min_votes = None;
        out.push((format!("Voti minimi {}", theme::thousands(v)), c));
    }
    if let Some((a, b)) = f.runtime {
        let mut c = f.clone();
        c.runtime = None;
        out.push((format!("Durata {}", runtime_label(a, b)), c));
    }
    if !f.without_genres.is_empty() {
        let mut c = f.clone();
        c.without_genres.clear();
        let label = if f.without_genres.len() == 1 {
            format!(
                "Genere escluso {}",
                tax.genre_name(f.without_genres[0])
                    .unwrap_or_else(|| f.without_genres[0].to_string())
            )
        } else {
            format!("Generi esclusi ({})", f.without_genres.len())
        };
        out.push((label, c));
    }
    if !f.genres.is_empty() {
        let mut c = f.clone();
        c.genres.clear();
        let label = if f.genres.len() == 1 {
            format!(
                "Genere {}",
                tax.genre_name(f.genres[0])
                    .unwrap_or_else(|| f.genres[0].to_string())
            )
        } else {
            format!("Generi scelti ({})", f.genres.len())
        };
        out.push((label, c));
    }
    if !f.decades.is_empty() {
        let mut c = f.clone();
        c.decades.clear();
        let label = if f.decades.len() == 1 {
            format!(
                "Periodo {}",
                tax.decade(&f.decades[0])
                    .map(|d| d.label.clone())
                    .unwrap_or_else(|| f.decades[0].clone())
            )
        } else {
            format!("Periodi scelti ({})", f.decades.len())
        };
        out.push((label, c));
    }
    if !f.countries.is_empty() {
        let mut c = f.clone();
        c.countries.clear();
        let label = if f.countries.len() == 1 {
            format!("Paese {}", tax.country_label(&f.countries[0]))
        } else {
            format!("Paesi scelti ({})", f.countries.len())
        };
        out.push((label, c));
    }
    if !f.moods.is_empty() {
        let mut c = f.clone();
        c.moods.clear();
        let label = if f.moods.len() == 1 {
            format!(
                "Mood {}",
                tax.mood(&f.moods[0])
                    .map(|m| m.label.clone())
                    .unwrap_or_else(|| f.moods[0].clone())
            )
        } else {
            format!("Mood scelti ({})", f.moods.len())
        };
        out.push((label, c));
    }
    if !f.keywords.is_empty() {
        let mut c = f.clone();
        c.keywords.clear();
        let label = if f.keywords.len() == 1 {
            format!("Tema {}", f.keywords[0])
        } else {
            format!("Temi scelti ({})", f.keywords.len())
        };
        out.push((label, c));
    }
    if !f.movements.is_empty() {
        let mut c = f.clone();
        c.movements.clear();
        let label = if f.movements.len() == 1 {
            format!(
                "Corrente {}",
                tax.movement(&f.movements[0])
                    .map(|m| m.label.clone())
                    .unwrap_or_else(|| f.movements[0].clone())
            )
        } else {
            format!("Correnti scelte ({})", f.movements.len())
        };
        out.push((label, c));
    }
    out
}

fn runtime_label(a: u32, b: u32) -> String {
    if a == 0 {
        format!("fino a {b} min")
    } else if b >= 600 {
        format!("oltre {a} min")
    } else {
        format!("{a}–{b} min")
    }
}

/// R5 · validazione sui dati reali. Se i vincoli chiesti producono zero titoli,
/// li allento uno alla volta — voto, poi generi, poi parole chiave tematiche —
/// e annoto esattamente cosa ho tolto, cosi l'utente vede il ragionamento.
pub fn discover_with_relaxation(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    facets: &Facets,
    kw: &[(String, u64)],
    page: u32,
    removed: &mut Vec<String>,
) -> Result<(Vec<Movie>, u32, u32, String), String> {
    let base = constraints::to_discover(tax, facets, kw, page)?;
    if constraints::decade_gap(tax, facets) {
        removed.push(
            "I periodi scelti non sono contigui: la ricerca copre anche gli anni in mezzo."
                .into(),
        );
    }
    let headline = describe(tax, facets, base.year);
    let pack = |p: PagedMovies, suffix: Option<String>| -> (Vec<Movie>, u32, u32, String) {
        (
            p.results.into_iter().map(Movie::from).collect(),
            p.total_results,
            p.total_pages,
            match suffix {
                Some(s) => format!("{headline} · {s}"),
                None => headline.clone(),
            },
        )
    };

    let first = tmdb.discover(&base)?;
    if first.total_results > 0 {
        return Ok(pack(first, None));
    }

    // Passo 1: i filtri sul giudizio sono i piu arbitrari, li tolgo per primi.
    if base.min_rating.is_some() || base.max_rating.is_some() || base.min_votes.is_some() {
        let mut softened = base.clone();
        softened.min_rating = None;
        softened.max_rating = None;
        softened.min_votes = None;
        let p = tmdb.discover(&softened)?;
        if p.total_results > 0 {
            let mut names: Vec<&str> = Vec::new();
            if base.min_rating.is_some() {
                names.push("voto minimo");
            }
            if base.max_rating.is_some() {
                names.push("voto massimo");
            }
            if base.min_votes.is_some() {
                names.push("numero di voti");
            }
            removed.push(format!(
                "Nessun titolo con quei filtri sul giudizio ({}): li ho tolti.",
                names.join(", ")
            ));
            return Ok(pack(p, Some("giudizio non filtrato".into())));
        }
    }

    // Passo 2: su TMDB i generi di un movimento sono etichette alternative, non
    // requisiti simultanei: li unisco a quelli chiesti e li passo in OR.
    let mut genres = base.genres.clone();
    for id in &facets.movements {
        if let Some(m) = tax.movement(id) {
            for g in &m.genres_core {
                if !genres.contains(g) {
                    genres.push(*g);
                }
            }
        }
    }
    if !genres.is_empty() {
        let mut widened = base.clone();
        widened.min_rating = None;
        widened.max_rating = None;
        widened.min_votes = None;
        widened.genres = genres.clone();
        widened.genres_or = true;
        let p = tmdb.discover(&widened)?;
        if p.total_results > 0 {
            removed.push(
                "Combinazione molto stretta: ho trattato i generi come alternative (Horror oppure Thriller…), includendo i generi tipici del movimento."
                    .into(),
            );
            return Ok(pack(p, Some("generi in alternativa".into())));
        }
    }

    // Passo 3: le parole chiave tematiche sono filtri AND molto severi.
    if !kw.is_empty() {
        let mut widened = base.clone();
        widened.min_rating = None;
        widened.max_rating = None;
        widened.min_votes = None;
        widened.genres = genres;
        widened.genres_or = true;
        widened.keywords.clear();
        let p = tmdb.discover(&widened)?;
        if p.total_results > 0 {
            removed.push(
                "Le parole chiave tematiche non avevano corrispondenze: le ho rimosse mantenendo periodo, paese e generi."
                    .into(),
            );
            return Ok(pack(p, Some("temi non filtrati".into())));
        }
    }

    Ok(pack(first, Some("nessun risultato".into())))
}

/// Risolve su TMDB le parole chiave di mood, temi liberi e correnti: quella
/// che non si risolve viene annotata e ignorata, senza bloccare la ricerca.
fn resolve_keywords(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    eff: &Facets,
    kw: &mut Vec<(String, u64)>,
    pairs: &mut Vec<(String, u64)>,
    notes: &mut Vec<String>,
) {
    for (id, word) in constraints::unresolved_keywords(tax, eff, kw.as_slice()) {
        if let Ok(Some(v)) = tmdb.keyword_id(&word) {
            kw.push((id.clone(), v));
            pairs.push((id, v));
        }
    }
    for name in constraints::unresolved_themes(eff, kw.as_slice()) {
        match tmdb.keyword_id(&name) {
            Ok(Some(v)) => {
                kw.push((name.clone(), v));
                pairs.push((name, v));
            }
            Ok(None) => notes.push(format!(
                "Il tema «{name}» non ha una parola chiave su TMDB: l'ho lasciato fuori."
            )),
            Err(e) => notes.push(format!("Tema «{name}» non verificato: {e}")),
        }
    }
    // La parola chiave propria di una corrente (es. lo slasher) e l'identita
    // della corrente: va in OR con gli altri argomenti.
    for (id, word) in constraints::unresolved_movement_keywords(tax, eff, kw.as_slice()) {
        match tmdb.keyword_id(&word) {
            Ok(Some(v)) => {
                kw.push((id.clone(), v));
                pairs.push((id, v));
            }
            Ok(None) => notes.push(format!(
                "La corrente si riconosce dalla parola chiave TMDB «{word}», che non esiste: la cerco per periodo e paese."
            )),
            Err(e) => notes.push(format!(
                "Parola chiave «{word}» della corrente non verificata: {e}"
            )),
        }
    }
}

/// Costruisce il pool della chat. I temi liberi sono criteri semantici, non
/// filtri di griglia: si cercano concetto per concetto e si tengono solo i
/// film che ne coprono davvero una parte.
#[allow(clippy::too_many_arguments)]
pub fn chat_pool(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    eff: &Facets,
    themes: &[Theme],
    known: &[(String, u64)],
    title: Option<&str>,
    similar: Option<&str>,
    notes: &mut Vec<String>,
) -> Result<ChatPoolResult, String> {
    let mut kw = known.to_vec();
    let mut pairs = Vec::new();
    resolve_keywords(tmdb, tax, eff, &mut kw, &mut pairs, notes);

    if !themes.is_empty() {
        return chat_theme_pool(tmdb, tax, eff, themes, &kw, pairs, notes);
    }

    let (movies, total, _headline, exact) =
        chat_candidates(tmdb, tax, eff, &kw, title, similar, notes)?;
    let relevance = movies.iter().map(|m| (m.id, 1.0)).collect();
    let limit = if movies.is_empty() {
        Some("Nessun film corrisponde ai criteri: prova a togliere un vincolo.".into())
    } else if !exact {
        Some("I criteri esatti non bastavano: ho allargato solo quanto necessario.".into())
    } else {
        None
    };
    Ok(ChatPoolResult {
        movies,
        total,
        facets: eff.clone(),
        keyword_pairs: pairs,
        limit,
        // Titolo, «simile a» e criteri allargati non sono rappresentabili in
        // griglia: il pulsante mostrerebbe risultati diversi dalla chat.
        grid_ready: !eff.is_empty()
            && title.is_none()
            && similar.is_none()
            && exact,
        reasons: HashMap::new(),
        relevance,
    })
}

/// La griglia senza criteri e con l'ordinamento predefinito mostra le
/// tendenze; qualunque altro ordinamento deve passare da discover, che
/// accetta `sort_by`. `Sort` non fa parte di `Facets::is_empty`.
fn wants_trending(f: &Facets) -> bool {
    f.is_empty() && f.sort == Sort::default()
}

fn sort_movies(movies: &mut [Movie], sort: Sort) {
    movies.sort_by(|a, b| match sort {
        Sort::Newest => b.year.unwrap_or(0).cmp(&a.year.unwrap_or(0)),
        Sort::Oldest => a.year.unwrap_or(0).cmp(&b.year.unwrap_or(0)),
        Sort::Rating => b
            .rating
            .partial_cmp(&a.rating)
            .unwrap_or(std::cmp::Ordering::Equal),
        Sort::Votes => b.votes.cmp(&a.votes),
        Sort::Popularity | Sort::Upcoming => {
            let sa = a.rating * (1.0 + a.votes as f32).ln();
            let sb = b.rating * (1.0 + b.votes as f32).ln();
            sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
        }
    });
}

/// Cerca la filmografia di un regista o attore. `Ok(None)` significa che il
/// nome non è stato riconosciuto: in quel caso il chiamante può ripiegare
/// sulla ricerca normale.
fn chat_person_pool(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    eff: &Facets,
    query: &PersonQuery,
    seen: Option<&str>,
    _notes: &mut Vec<String>,
) -> Result<Option<PersonPool>, String> {
    let results = tmdb.search_person(&query.name)?;
    let Some(person) = choose_person(&results, query) else {
        return Ok(None);
    };

    let credits = tmdb.person_credits(person.id)?;
    let directed: Vec<Movie> = credits
        .crew
        .iter()
        .filter(|c| c.job.eq_ignore_ascii_case("Director"))
        .map(|c| Movie::from(c.movie.clone()))
        .collect();
    let acted: Vec<Movie> = credits.cast.iter().cloned().map(Movie::from).collect();
    let (chosen, verb) = match query.role {
        PersonRole::Director => (directed, "diretto da"),
        PersonRole::Actor => (acted, "interpretato da"),
        PersonRole::Any => {
            if person.known_for_department.eq_ignore_ascii_case("Directing")
                || (person.known_for_department != "Acting" && !directed.is_empty())
            {
                (directed, "diretto da")
            } else {
                (acted, "interpretato da")
            }
        }
    };
    let mut movies = dedupe(chosen);
    if let Some(seen) = seen {
        let key = normalize_title(seen);
        movies.retain(|m| {
            let title = normalize_title(&m.title);
            let original = normalize_title(&m.original_title);
            !(title == key
                || original == key
                || (key.len() >= 4
                    && (key.contains(&title) || title.contains(&key))))
        });
    }
    let mut movies = filter_movies(movies, tax, eff);
    sort_movies(&mut movies, eff.sort);
    let total = movies.len() as u32;
    let limit = if total == 0 {
        Some(format!("Nessun film {verb} «{}» con questi criteri.", person.name))
    } else {
        None
    };
    movies.truncate(20);
    let reasons = movies
        .iter()
        .map(|m| (m.id, format!("{verb} {}", person.name)))
        .collect::<HashMap<_, _>>();
    let relevance = movies.iter().map(|m| (m.id, 1.0)).collect();
    let label = match verb {
        "diretto da" => format!("film diretti da «{}»", person.name),
        _ => format!("film con «{}»", person.name),
    };
    Ok(Some(PersonPool {
        result: ChatPoolResult {
            movies,
            total,
            facets: eff.clone(),
            keyword_pairs: Vec::new(),
            limit,
            grid_ready: false,
            reasons,
            relevance,
        },
        label,
        person: person.name.clone(),
    }))
}

struct ThemeCandidate {
    movie: Movie,
    matched: Vec<bool>,
}

fn has_word(hay: &str, term: &str) -> bool {
    if term.is_empty() {
        return false;
    }
    hay.match_indices(term).any(|(at, _)| {
        let before = hay[..at].chars().next_back();
        let after = hay[at + term.len()..].chars().next();
        !before.map(|c| c.is_alphanumeric()).unwrap_or(false)
            && !after.map(|c| c.is_alphanumeric()).unwrap_or(false)
    })
}

fn concept_matches(text: &str, concept: &ThemeConcept) -> bool {
    let hay = text.to_lowercase();
    concept
        .terms
        .iter()
        .any(|term| has_word(&hay, term.trim().to_lowercase().as_str()))
}

fn add_theme_movie(
    pool: &mut HashMap<u64, ThemeCandidate>,
    concepts: &[ThemeConcept],
    movie: Movie,
    hint: Option<usize>,
) {
    let hay = format!(
        "{} {} {}",
        movie.title, movie.original_title, movie.overview
    );
    let matched = concepts
        .iter()
        .map(|c| concept_matches(&hay, c))
        .collect::<Vec<_>>();
    if hint.is_none() && !matched.iter().any(|m| *m) {
        return;
    }
    let entry = pool.entry(movie.id).or_insert_with(|| ThemeCandidate {
        movie,
        matched: vec![false; concepts.len()],
    });
    if let Some(i) = hint {
        if let Some(slot) = entry.matched.get_mut(i) {
            *slot = true;
        }
    }
    for (i, hit) in matched.into_iter().enumerate() {
        if hit {
            entry.matched[i] = true;
        }
    }
}

fn chat_theme_pool(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    eff: &Facets,
    themes: &[Theme],
    hard_kw: &[(String, u64)],
    hard_pairs: Vec<(String, u64)>,
    notes: &mut Vec<String>,
) -> Result<ChatPoolResult, String> {
    let concepts: Vec<ThemeConcept> = themes
        .iter()
        .flat_map(|t| t.concepts.iter().cloned())
        .take(4)
        .collect();
    if concepts.is_empty() {
        return Ok(ChatPoolResult {
            movies: Vec::new(),
            total: 0,
            facets: eff.clone(),
            keyword_pairs: hard_pairs,
            limit: Some("Il tema non è stato tradotto in concetti verificabili.".into()),
            grid_ready: false,
            reasons: HashMap::new(),
            relevance: HashMap::new(),
        });
    }

    let mut queries: Vec<Vec<u64>> = Vec::with_capacity(concepts.len());
    for concept in &concepts {
        let mut ids: Vec<u64> = Vec::new();
        for keyword in &concept.keywords {
            match tmdb.keyword_id(keyword) {
                Ok(Some(id)) => {
                    if !ids.contains(&id) {
                        ids.push(id);
                    }
                }
                Ok(None) => notes.push(format!(
                    "La keyword TMDB «{keyword}» per «{}» non esiste: uso solo la trama.",
                    concept.label
                )),
                Err(e) => notes.push(format!(
                    "Keyword «{keyword}» per «{}» non verificata: {e}",
                    concept.label
                )),
            }
        }
        queries.push(ids);
    }

    let mut pool: HashMap<u64, ThemeCandidate> = HashMap::new();
    let mut total = 0u32;
    for (index, ids) in queries.iter().enumerate() {
        // Una keyword per query: TMDB non sa combinare «hard AND (a OR b)».
        // Cosi le alternative del concetto restano davvero alternative.
        for id in ids {
            let mut q = constraints::to_discover(tax, eff, hard_kw, 1)?;
            if !q.keywords.contains(id) {
                q.keywords.push(*id);
            }
            q.keywords_or = false;
            let page = tmdb.discover(&q)?;
            total = total.saturating_add(page.total_results);
            for movie in page.results.into_iter().map(Movie::from) {
                add_theme_movie(&mut pool, &concepts, movie, Some(index));
            }
        }
    }

    // Titoli proposti dall'AI e ricerca del tema: si accettano solo se la
    // trama conferma almeno un concetto.
    for theme in themes {
        if !theme.label.trim().is_empty() {
            if let Ok(page) = tmdb.search(&theme.label, 1) {
                for movie in page.results.into_iter().take(5).map(Movie::from) {
                    if eff.is_empty() || !filter_movies(vec![movie.clone()], tax, eff).is_empty() {
                        add_theme_movie(&mut pool, &concepts, movie, None);
                    }
                }
            }
        }
        for seed in &theme.seed_titles {
            let Ok(page) = tmdb.search(seed, 1) else { continue };
            let Some(first) = page.results.into_iter().next() else { continue };
            let movie = Movie::from(first);
            if !eff.is_empty() && filter_movies(vec![movie.clone()], tax, eff).is_empty() {
                continue;
            }
            add_theme_movie(&mut pool, &concepts, movie, None);
        }
    }

    let n = concepts.len();
    // Due concetti bastano per il confronto (cane/gatto); con tre o piu
    // concetti chiediamo almeno due segnali, altrimenti il "piu vicino" e
    // solo un film qualsiasi che parla d'altro.
    let need = if n <= 1 { n } else if n == 2 { 1 } else { 2 };
    let mut ranked: Vec<ThemeCandidate> = pool.into_values().collect();
    // I match completi restano in cima; se sono meno di cinque, i parziali
    // dichiarati completano la scelta senza spacciarsi per corrispondenze piene.
    ranked.retain(|c| c.matched.iter().filter(|m| **m).count() >= need);
    ranked.sort_by(|a, b| {
        let ca = a.matched.iter().filter(|m| **m).count();
        let cb = b.matched.iter().filter(|m| **m).count();
        cb.cmp(&ca).then_with(|| match eff.sort {
            Sort::Newest => b.movie.year.cmp(&a.movie.year),
            Sort::Oldest => a.movie.year.cmp(&b.movie.year),
            Sort::Rating => b
                .movie
                .rating
                .partial_cmp(&a.movie.rating)
                .unwrap_or(std::cmp::Ordering::Equal),
            Sort::Votes => b.movie.votes.cmp(&a.movie.votes),
            Sort::Popularity | Sort::Upcoming => {
                let sa = a.movie.rating as f64 * (1.0 + a.movie.votes as f64).ln();
                let sb = b.movie.rating as f64 * (1.0 + b.movie.votes as f64).ln();
                sb.partial_cmp(&sa).unwrap_or(std::cmp::Ordering::Equal)
            }
        })
    });

    let mut reasons = HashMap::new();
    let mut relevance = HashMap::new();
    for c in &ranked {
        let covered = concepts
            .iter()
            .zip(c.matched.iter())
            .filter(|(_, hit)| **hit)
            .map(|(c, _)| c.label.as_str())
            .collect::<Vec<_>>();
        let missing = concepts
            .iter()
            .zip(c.matched.iter())
            .filter(|(_, hit)| !**hit)
            .map(|(c, _)| c.label.as_str())
            .collect::<Vec<_>>();
        let reason = if missing.is_empty() {
            format!("Copre tutti i concetti: {}.", covered.join(", "))
        } else if covered.is_empty() {
            format!("Non copre i concetti: {}.", missing.join(", "))
        } else {
            format!(
                "Copre {}; manca {}.",
                covered.join(", "),
                missing.join(", ")
            )
        };
        reasons.insert(c.movie.id, reason);
        relevance.insert(c.movie.id, covered.len() as f32 / n as f32);
    }

    let count = ranked.len();
    let exact_count = ranked
        .iter()
        .filter(|c| c.matched.iter().filter(|m| **m).count() == n)
        .count();
    let movies = ranked.into_iter().map(|c| c.movie).take(20).collect::<Vec<_>>();
    let limit = if movies.is_empty() {
        Some(format!(
            "Nessun film copre almeno {} dei {} concetti: non lo sostituisco con titoli a caso.",
            need, n
        ))
    } else if exact_count == 0 {
        Some(format!(
            "Nessun film copre tutti i {n} concetti: questi sono i più vicini."
        ))
    } else if exact_count < 5 && count > exact_count {
        Some(format!(
            "Solo {exact_count} film {} tutti i criteri; gli altri sono i più vicini.",
            if exact_count == 1 { "copre" } else { "coprono" }
        ))
    } else if exact_count < 5 {
        Some(format!(
            "Solo {exact_count} film {} davvero tutti i criteri; non aggiungo titoli a caso.",
            if exact_count == 1 { "copre" } else { "coprono" }
        ))
    } else {
        None
    };
    let total = total.max(movies.len() as u32);
    Ok(ChatPoolResult {
        movies,
        total,
        facets: eff.clone(),
        keyword_pairs: hard_pairs,
        limit,
        grid_ready: false,
        reasons,
        relevance,
    })
}

/// Candidati reali per titolo, simili, tendenze o facce dure. Il rilassamento
/// e dichiarato: non si sostituiscono i criteri stretti con titoli estranei.
fn chat_candidates(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    eff: &Facets,
    kw: &[(String, u64)],
    title: Option<&str>,
    similar: Option<&str>,
    notes: &mut Vec<String>,
) -> Result<(Vec<Movie>, u32, String, bool), String> {
    if let Some(name) = similar {
        let p = tmdb.search(name, 1)?;
        let Some(first) = p.results.first() else {
            return Err(format!("Nessun film trovato per «{name}»."));
        };
        let seed = first.title.clone();
        let rec = tmdb.similar(first.id, 1)?;
        let movies = filter_movies(
            dedupe(rec.results.into_iter().map(Movie::from).collect()),
            tax,
            eff,
        );
        if movies.is_empty() {
            return Err(format!(
                "Nessun film simile a «{seed}» con i filtri attivi."
            ));
        }
        let total = if eff.is_empty() {
            rec.total_results
        } else {
            movies.len() as u32
        };
        return Ok((movies, total, format!("Simili a «{seed}»"), true));
    }
    if let Some(t) = title {
        if eff.is_empty() {
            let p1 = tmdb.search(t, 1)?;
            let total = p1.total_results;
            let mut movies: Vec<Movie> = p1.results.into_iter().map(Movie::from).collect();
            if p1.total_pages > 1 && movies.len() < 20 {
                if let Ok(p2) = tmdb.search(t, 2) {
                    movies.extend(p2.results.into_iter().map(Movie::from));
                }
            }
            let movies = dedupe(movies);
            if movies.is_empty() && looks_like_question(t) {
                // Solo per una frase che sembra una domanda: una vera ricerca
                // per titolo non viene sostituita dai film del momento.
                notes.push(
                    "La frase sembra una domanda, non un titolo: propongo i film del momento."
                        .into(),
                );
                let p = tmdb.trending(1)?;
                return Ok((
                    dedupe(p.results.into_iter().map(Movie::from).collect()),
                    p.total_results,
                    "Tendenze della settimana".into(),
                    false,
                ));
            }
            return Ok((movies, total, format!("Risultati per «{t}»"), true));
        }
        let (raw, _) = search_title_pages(tmdb, t)?;
        let movies = filter_movies(raw, tax, eff);
        let total = movies.len() as u32;
        return Ok((movies, total, format!("Risultati per «{t}»"), true));
    }
    if wants_trending(eff) {
        let p = tmdb.trending(1)?;
        return Ok((
            dedupe(p.results.into_iter().map(Movie::from).collect()),
            p.total_results,
            "Tendenze della settimana".into(),
            true,
        ));
    }
    let mut relaxed = Vec::new();
    let (m1, total, pages, headline) =
        discover_with_relaxation(tmdb, tax, eff, kw, 1, &mut relaxed)?;
    let mut movies = m1;
    if pages > 1 && movies.len() < 20 {
        if let Ok((m2, _, _, _)) =
            discover_with_relaxation(tmdb, tax, eff, kw, 2, &mut relaxed)
        {
            movies.extend(m2);
        }
    }
    let exact = relaxed.is_empty();
    notes.extend(relaxed);
    Ok((dedupe(movies), total, headline, exact))
}

fn dedupe(movies: Vec<Movie>) -> Vec<Movie> {
    let mut seen: HashSet<u64> = HashSet::new();
    movies.into_iter().filter(|m| seen.insert(m.id)).collect()
}

/// Una frase che sembra una domanda, non un titolo: usata solo quando la
/// ricerca per titolo non trova nulla, per non chiudere con zero film.
fn looks_like_question(text: &str) -> bool {
    let t = text.trim().to_lowercase();
    if t.contains('?') {
        return true;
    }
    const STARTS: [&str; 8] = [
        "chi ", "cosa ", "che cosa", "qual", "quando ", "dove ", "come ", "perch",
    ];
    STARTS.iter().any(|p| t.starts_with(p))
}

/// Il film come lo vede il curatore: pochi campi, trama troncata.
fn candidate_of(
    tax: &Taxonomy,
    m: &Movie,
    reason: Option<String>,
    relevance: f32,
) -> curator::Candidate {
    curator::Candidate {
        id: m.id,
        title: m.title.clone(),
        year: m.year,
        rating: m.rating,
        votes: m.votes,
        genres: m
            .genre_ids
            .iter()
            .filter_map(|g| tax.genre_name(*g))
            .collect::<Vec<_>>()
            .join(", "),
        overview: theme::truncate(&m.overview, 240),
        reason,
        relevance,
    }
}

fn describe(tax: &Taxonomy, f: &Facets, year: Option<(u16, u16)>) -> String {
    let mut parts: Vec<String> = Vec::new();
    for id in &f.movements {
        if let Some(m) = tax.movement(id) {
            parts.push(m.label.clone());
        }
    }
    if !f.decades.is_empty() {
        let labels = f
            .decades
            .iter()
            .filter_map(|d| tax.decade(d))
            .map(|d| d.label.clone())
            .collect::<Vec<_>>()
            .join(" + ");
        parts.push(labels);
    } else if let Some((a, b)) = year {
        parts.push(format!("{a}–{b}"));
    }
    if !f.genres.is_empty() {
        let g = f
            .genres
            .iter()
            .filter_map(|g| tax.genre_name(*g))
            .collect::<Vec<_>>()
            .join(", ");
        parts.push(g);
    }
    let countries = constraints::effective_countries(tax, f);
    if !countries.is_empty() {
        parts.push(
            countries
                .iter()
                .map(|c| tax.country_label(c))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    if !f.moods.is_empty() {
        let moods = f
            .moods
            .iter()
            .filter_map(|m| tax.mood(m))
            .map(|m| m.label.clone())
            .collect::<Vec<_>>()
            .join(" + ");
        if !moods.is_empty() {
            parts.push(moods);
        }
    }
    let rating = match (f.min_rating, f.max_rating) {
        (Some(a), Some(b)) => Some(format!("voto {a:.1}–{b:.1}")),
        (Some(a), None) => Some(format!("voto ≥ {a:.1}")),
        (None, Some(b)) => Some(format!("voto ≤ {b:.1}")),
        (None, None) => None,
    };
    if let Some(r) = rating {
        parts.push(r);
    }
    if let Some((min, max)) = f.runtime {
        parts.push(if min == 0 {
            format!("fino a {max} min")
        } else {
            format!("{min}-{max} min")
        });
    }
    if let Some(v) = f.min_votes {
        parts.push(format!("≥ {} voti", theme::thousands(v)));
    }
    if !f.keywords.is_empty() {
        parts.push(format!("temi: {}", f.keywords.join(", ")));
    }
    if parts.is_empty() {
        "Ricerca libera".to_string()
    } else {
        parts.join(" · ")
    }
}

fn describe_chat(tax: &Taxonomy, f: &Facets, themes: &[Theme]) -> String {
    let base = describe(tax, f, None);
    if themes.is_empty() {
        return base;
    }
    let labels = themes
        .iter()
        .map(|t| t.label.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    if base == "Ricerca libera" {
        format!("temi: {labels}")
    } else {
        format!("{base} · temi: {labels}")
    }
}

/// Frecce del pager: quadrato piatto, spento quando la pagina non esiste.
fn pager_button(
    ui: &mut egui::Ui,
    kind: lumen::Icon,
    enabled: bool,
    tip: &str,
) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(30.0, 22.0), sense);
    let hover = if enabled {
        ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12)
    } else {
        0.0
    };
    ui.painter().rect_filled(
        rect,
        lumen::radius(lumen::RADIUS_SM),
        if hover > 0.01 {
            lumen::RAISED
        } else {
            lumen::DEEP
        },
    );
    ui.painter().rect_stroke(
        rect,
        lumen::radius(lumen::RADIUS_SM),
        egui::Stroke::new(
            1.0_f32,
            if hover > 0.01 {
                lumen::BORDER_HI
            } else {
                lumen::BORDER
            },
        ), egui::StrokeKind::Inside);
    lumen::icon(
        ui.painter(),
        egui::Rect::from_center_size(rect.center(), egui::vec2(14.0, 14.0)),
        kind,
        if enabled {
            lumen::mix(lumen::MUTED, lumen::IVORY, hover)
        } else {
            lumen::FAINT
        },
    );
    lumen::focus(ui, &resp, lumen::RADIUS_SM);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text(tip)
    } else {
        resp
    }
}

/// Larghezza massima dei pannelli laterali. Si apre un pannello per volta,
/// quindi a 800 px la griglia conserva almeno tre card anche con i filtri.
fn side_panel_widths(screen_w: f32) -> (f32, f32) {
    let left = (screen_w * 0.30).clamp(260.0, 380.0);
    let right = (screen_w * 0.30).clamp(288.0, 420.0);
    (left, right)
}

/// Bordo entro cui il puntatore prende il ridimensionamento della finestra.
const RESIZE_EDGE: f32 = 6.0;

fn resize_edge(
    rect: egui::Rect,
    pos: egui::Pos2,
) -> Option<egui::viewport::ResizeDirection> {
    use egui::viewport::ResizeDirection as Dir;
    let left = pos.x <= rect.left() + RESIZE_EDGE;
    let right = pos.x >= rect.right() - RESIZE_EDGE;
    let top = pos.y <= rect.top() + RESIZE_EDGE;
    let bottom = pos.y >= rect.bottom() - RESIZE_EDGE;
    match (left, right, top, bottom) {
        (true, _, true, _) => Some(Dir::NorthWest),
        (_, true, true, _) => Some(Dir::NorthEast),
        (true, _, _, true) => Some(Dir::SouthWest),
        (_, true, _, true) => Some(Dir::SouthEast),
        (true, _, _, _) => Some(Dir::West),
        (_, true, _, _) => Some(Dir::East),
        (_, _, true, _) => Some(Dir::North),
        (_, _, _, true) => Some(Dir::South),
        _ => None,
    }
}

/// Senza decorazioni Windows non disegna piu' i bordi: li gestiamo noi.
fn window_resize_edges(ctx: &egui::Context) {
    if ctx.input(|i| i.viewport().maximized.unwrap_or(false)) {
        return;
    }
    let Some(pos) = ctx.pointer_hover_pos() else {
        return;
    };
    let Some(dir) = resize_edge(ctx.viewport_rect(), pos) else {
        return;
    };
    use egui::viewport::ResizeDirection as Dir;
    ctx.set_cursor_icon(match dir {
        Dir::North | Dir::South => egui::CursorIcon::ResizeVertical,
        Dir::East | Dir::West => egui::CursorIcon::ResizeHorizontal,
        Dir::NorthWest | Dir::SouthEast => egui::CursorIcon::ResizeNwSe,
        Dir::NorthEast | Dir::SouthWest => egui::CursorIcon::ResizeNeSw,
    });
    if ctx.input(|i| i.pointer.primary_pressed()) {
        ctx.send_viewport_cmd(egui::viewport::ViewportCommand::BeginResize(dir));
    }
}

#[derive(Clone, Copy)]
enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

/// Controllo finestra della titlebar custom. Le icone sono disegnate a linee:
/// niente glifi di sistema mancanti e spessore coerente a ogni DPI. Il Button
/// (vuoto) conserva fuoco e attivazione da tastiera.
fn window_caption(
    ui: &mut egui::Ui,
    icon: CaptionIcon,
    tip: &str,
    cap: f32,
) -> egui::Response {
    let resp = ui.add(
        egui::Button::new("")
            .frame(false)
            .min_size(egui::vec2(cap, 40.0)),
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    if hover > 0.01 {
        let fill = if matches!(icon, CaptionIcon::Close) {
            theme::mix(lumen::SURFACE, theme::DANGER, hover)
        } else {
            theme::mix(lumen::INK, lumen::RAISED, hover)
        };
        ui.painter()
            .rect_filled(resp.rect, egui::CornerRadius::ZERO, fill);
    }
    let color = if matches!(icon, CaptionIcon::Close) {
        theme::mix(theme::DIM, egui::Color32::WHITE, hover)
    } else {
        theme::mix(theme::DIM, theme::TEXT, hover)
    };
    let stroke = egui::Stroke::new(1.4_f32, color);
    let c = resp.rect.center();
    let p = ui.painter();
    match icon {
        CaptionIcon::Minimize => {
            p.line_segment(
                [c + egui::vec2(-4.2, 0.5), c + egui::vec2(4.2, 0.5)],
                stroke,
            );
        }
        CaptionIcon::Maximize => {
            p.rect_stroke(
                egui::Rect::from_center_size(c, egui::vec2(8.5, 8.5)),
                lumen::radius(0.5),
                stroke, egui::StrokeKind::Inside);
        }
        CaptionIcon::Restore => {
            let back = egui::Rect::from_center_size(
                c + egui::vec2(1.5, -1.5),
                egui::vec2(7.0, 7.0),
            );
            let front = egui::Rect::from_center_size(
                c + egui::vec2(-1.5, 1.5),
                egui::vec2(7.0, 7.0),
            );
            // Del riquadro dietro restano visibili solo alto e lato destro.
            p.line_segment([back.left_top(), back.right_top()], stroke);
            p.line_segment([back.right_top(), back.right_bottom()], stroke);
            p.rect_stroke(front, lumen::radius(0.5), stroke, egui::StrokeKind::Inside);
        }
        CaptionIcon::Close => {
            let d = 4.2;
            p.line_segment(
                [c + egui::vec2(-d, -d), c + egui::vec2(d, d)],
                stroke,
            );
            p.line_segment(
                [c + egui::vec2(d, -d), c + egui::vec2(-d, d)],
                stroke,
            );
        }
    }
    theme::focus_ring(ui, &resp, 4.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tip)
}

fn track_state(store: &simkl::Store, id: u64) -> TrackState {
    match store.get(id) {
        Some(tracked) => TrackState {
            watched: tracked.status == "completed",
            watchlist: tracked.status == "plantowatch",
            rating: tracked.rating,
        },
        None => TrackState::default(),
    }
}

fn ago_label(ts: i64) -> String {    if ts <= 0 {
        return "mai".into();
    }
    let mins = (simkl::now_secs() - ts).max(0) / 60;
    if mins < 1 {
        "adesso".into()
    } else if mins < 60 {
        format!("{mins} min fa")
    } else {
        format!("{} h fa", mins / 60)
    }
}

fn allowed_url(url: &str) -> bool {    let trimmed = url.trim();
    if trimmed.chars().any(|c| c.is_control() || c.is_whitespace()) {
        return false;
    }
    let lower = trimmed.to_ascii_lowercase();
    let Some(rest) = lower
        .strip_prefix("https://")
        .or_else(|| lower.strip_prefix("http://"))
    else {
        return false;
    };
    !rest.is_empty() && !rest.starts_with('/')
}

fn open_url(url: &str) {
    if !allowed_url(url) {
        return;
    }
    #[cfg(target_os = "windows")]
    {
        // explorer apre l'URL nel browser predefinito senza passare da cmd.exe,
        // che interpreterebbe i metacaratteri di un link TMDB non fidato.
        let _ = std::process::Command::new("explorer").arg(url.trim()).spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
    }
}

#[cfg(test)]
mod tests {
    use super::{
        allowed_url, describe, filter_movies, has_word, in_sala_movies, looks_like_question,
        merge_facets, resize_edge, side_panel_widths, sort_movies, unlock_candidates,
        wants_trending, Cleared, Facets, Movie, Sort,
    };
    use crate::domain::taxonomy::Taxonomy;
    use egui::viewport::ResizeDirection as Dir;

    #[test]
    fn un_ordinamento_non_predefinito_non_usa_le_tendenze() {
        let mut f = Facets::default();
        assert!(wants_trending(&f));
        f.sort = Sort::Rating;
        assert!(!wants_trending(&f));
    }

    #[test]
    fn ogni_sblocco_toglie_un_solo_vincolo() {
        let tax = Taxonomy::load();
        let f = Facets {
            genres: vec![18],
            min_votes: Some(500),
            runtime: Some((90, 120)),
            ..Default::default()
        };
        let options = unlock_candidates(&tax, &f);
        assert_eq!(options.len(), 3, "una variante per vincolo");

        let voti = options
            .iter()
            .find(|(l, _)| l.starts_with("Voti minimi"))
            .expect("voti");
        assert!(voti.1.min_votes.is_none());
        assert_eq!(voti.1.runtime, f.runtime);
        assert_eq!(voti.1.genres, f.genres);

        let durata = options
            .iter()
            .find(|(l, _)| l.starts_with("Durata"))
            .expect("durata");
        assert!(durata.1.runtime.is_none());
        assert_eq!(durata.1.min_votes, f.min_votes);
        assert_eq!(durata.1.genres, f.genres);
    }

    #[test]
    fn prossimamente_ordina_per_popolarita_non_per_data() {
        let noto = Movie {
            id: 1,
            year: Some(2026),
            rating: 8.0,
            votes: 5_000,
            ..Default::default()
        };
        let oscuro = Movie {
            id: 2,
            year: Some(2027),
            rating: 5.0,
            votes: 5,
            ..Default::default()
        };
        let mut v = vec![oscuro, noto];
        sort_movies(&mut v, Sort::Upcoming);
        assert_eq!(v[0].id, 1);
    }

    #[test]
    fn il_filtro_locale_usa_i_generi_in_and() {
        let tax = Taxonomy::load();
        let f = Facets {
            genres: vec![27, 35],
            ..Default::default()
        };
        let only_horror = Movie {
            id: 1,
            genre_ids: vec![27],
            ..Default::default()
        };
        let both = Movie {
            id: 2,
            genre_ids: vec![27, 35],
            ..Default::default()
        };
        let out = filter_movies(vec![only_horror, both], &tax, &f);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, 2);
    }

    #[test]
    fn il_filtro_locale_rispetta_senza_genere() {
        let tax = Taxonomy::load();
        let f = Facets {
            without_genres: vec![35],
            ..Default::default()
        };
        let comedy = Movie {
            id: 1,
            genre_ids: vec![35],
            ..Default::default()
        };
        let drama = Movie {
            id: 2,
            genre_ids: vec![18],
            ..Default::default()
        };
        let out = filter_movies(vec![comedy, drama], &tax, &f);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].id, 2);
    }

    #[test]
    fn la_durata_compare_nei_criteri() {
        let f = Facets {
            runtime: Some((0, 100)),
            ..Default::default()
        };
        assert!(describe(&Taxonomy::load(), &f, None).contains("100 min"));
    }

    #[test]
    fn un_nuovo_paese_sostituisce_quello_precedente() {
        let mut base = Facets {
            countries: vec!["FR".into()],
            ..Default::default()
        };
        let extra = Facets {
            countries: vec!["IT".into()],
            ..Default::default()
        };
        merge_facets(&mut base, &extra, &Cleared::default());
        assert_eq!(base.countries, ["IT"]);
    }

    #[test]
    fn togliere_un_filtro_lo_rimuove_dal_contesto() {
        let mut base = Facets {
            min_rating: Some(8.0),
            min_votes: Some(500),
            ..Default::default()
        };
        let extra = Facets::default();
        let cleared = Cleared {
            min_rating: true,
            ..Default::default()
        };
        merge_facets(&mut base, &extra, &cleared);
        assert!(base.min_rating.is_none());
        assert_eq!(base.min_votes, Some(500), "gli altri filtri restano");
    }

    #[test]
    fn in_sala_tiene_solo_i_film_con_poster_e_al_massimo_sei() {
        let with_poster = |id: u64| Movie {
            id,
            poster_path: Some(format!("/{id}.jpg")),
            ..Default::default()
        };
        let mut movies = vec![Movie {
            id: 0,
            ..Default::default()
        }];
        for id in 1..=8 {
            movies.push(with_poster(id));
        }
        let out = in_sala_movies(movies);
        assert_eq!(out.len(), 6);
        assert!(out.iter().all(|m| m.poster_path.is_some()));
        assert_eq!(out[0].id, 1, "i film senza poster non occupano un buco");
    }

    #[test]
    fn il_tema_cerca_parole_intere() {
        assert!(has_word("un film sul mare", "mare"));
        assert!(!has_word("vorrei amare il cinema", "mare"));
        assert!(has_word("space travel verso l'ignoto", "space travel"));
    }

    #[test]
    fn i_bordi_ridimensionano_la_finestra() {
        let r = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        assert_eq!(resize_edge(r, egui::pos2(2.0, 2.0)), Some(Dir::NorthWest));
        assert_eq!(resize_edge(r, egui::pos2(798.0, 2.0)), Some(Dir::NorthEast));
        assert_eq!(resize_edge(r, egui::pos2(2.0, 598.0)), Some(Dir::SouthWest));
        assert_eq!(resize_edge(r, egui::pos2(798.0, 598.0)), Some(Dir::SouthEast));
        assert_eq!(resize_edge(r, egui::pos2(400.0, 2.0)), Some(Dir::North));
        assert_eq!(resize_edge(r, egui::pos2(400.0, 598.0)), Some(Dir::South));
        assert_eq!(resize_edge(r, egui::pos2(2.0, 300.0)), Some(Dir::West));
        assert_eq!(resize_edge(r, egui::pos2(798.0, 300.0)), Some(Dir::East));
        assert_eq!(resize_edge(r, egui::pos2(400.0, 300.0)), None);
        assert_eq!(resize_edge(r, egui::pos2(7.0, 300.0)), None);
    }

    #[test]
    fn i_pannelli_lasciano_spazio_alla_griglia() {
        // Con un solo pannello aperto, a 800 px restano tre colonne.
        let (left, right) = side_panel_widths(800.0);
        assert!(800.0 - left >= 480.0);
        assert!(800.0 - right >= 480.0);
        // A 1000 px restano quattro colonne, a 1220 cinque.
        let (left, right) = side_panel_widths(1000.0);
        assert!(1000.0 - left >= 644.0);
        assert!(1000.0 - right >= 644.0);
        let (left, right) = side_panel_widths(1220.0);
        assert!(1220.0 - left >= 808.0);
        assert!(1220.0 - right >= 808.0);
        assert_eq!(side_panel_widths(1920.0), (380.0, 420.0));
    }

    #[test]
    fn una_domanda_non_e_un_titolo() {
        assert!(looks_like_question("qual e la capitale della Francia?"));
        assert!(looks_like_question("chi ha diretto Suspiria"));
        assert!(!looks_like_question("Il padrino"));
        assert!(!looks_like_question("Blade Runner 2049"));
    }

    #[test]
    fn si_aprono_solo_link_web() {
        assert!(allowed_url("https://www.themoviedb.org/movie/1"));
        assert!(allowed_url("http://example.com/?a=1&b=2"));
        assert!(!allowed_url("javascript:alert(1)"));
        assert!(!allowed_url("file:///C:/Windows/System32/calc.exe"));
        assert!(!allowed_url("https://example.com/\ncalc"));
        assert!(!allowed_url("https://"));
    }
}

#[cfg(test)]
mod phase1_tests {
    use super::*;
    use crate::domain::model::Movie;

    fn card(id: u64, title: &str, year: u16, rating: f32, runtime: u32) -> chat::Card {
        chat::Card {
            movie: Movie {
                id,
                title: title.into(),
                original_title: title.into(),
                year: Some(year),
                rating,
                votes: 10_000,
                genre_ids: vec![18],
                ..Default::default()
            },
            perche: format!("«{title}» rientra nei criteri della ricerca."),
            consiglio: "Apri la scheda per i dettagli.".into(),
            meta: Some(chat::CardMeta {
                director: Some("Regista".into()),
                runtime: Some(runtime),
                ..Default::default()
            }),
        }
    }

    #[test]
    fn gli_omonimi_si_scelgono_dal_ruolo() {
        let people = vec![
            Person {
                id: 1,
                name: "Vittorio De Sica".into(),
                known_for_department: "Directing".into(),
                popularity: 1.4,
            },
            Person {
                id: 2,
                name: "Vittorio De Sica".into(),
                known_for_department: "Acting".into(),
                popularity: 0.3,
            },
        ];
        let any = PersonQuery {
            name: "Vittorio De Sica".into(),
            role: PersonRole::Any,
        };
        let actor = PersonQuery {
            role: PersonRole::Actor,
            ..any.clone()
        };
        assert_eq!(choose_person(&people, &any).map(|p| p.id), Some(1));
        // Il De Sica famoso è anche attore: il ruolo non deve far scegliere
        // l'omonimo attore minore.
        assert_eq!(choose_person(&people, &actor).map(|p| p.id), Some(1));

        let comparable = vec![
            Person {
                id: 3,
                name: "Mario Rossi".into(),
                known_for_department: "Directing".into(),
                popularity: 1.0,
            },
            Person {
                id: 4,
                name: "Mario Rossi".into(),
                known_for_department: "Acting".into(),
                popularity: 0.9,
            },
        ];
        let mario = PersonQuery {
            name: "Mario Rossi".into(),
            role: PersonRole::Actor,
        };
        assert_eq!(choose_person(&comparable, &mario).map(|p| p.id), Some(4));

        // Omonimi con lo stesso ruolo: vince il più popolare; nessun
        // risultato resta un caso gestito.
        let two_directors = vec![
            Person {
                id: 5,
                name: "Mario Rossi".into(),
                known_for_department: "Directing".into(),
                popularity: 1.0,
            },
            Person {
                id: 6,
                name: "Mario Rossi".into(),
                known_for_department: "Directing".into(),
                popularity: 2.0,
            },
        ];
        let director = PersonQuery {
            name: "Mario Rossi".into(),
            role: PersonRole::Director,
        };
        assert_eq!(
            choose_person(&two_directors, &director).map(|p| p.id),
            Some(6)
        );
        assert!(choose_person(&[], &director).is_none());
    }

    #[test]
    fn riconosce_registi_attori_e_il_film_appena_visto() {
        let q = person_query("ho visto ladri di biciclette, altri film di vittorio de sica?")
            .expect("persona");
        assert_eq!(q.name.to_lowercase(), "vittorio de sica");
        assert_eq!(q.role, PersonRole::Any);
        assert_eq!(
            seen_title_in_query("ho visto ladri di biciclette, altri film di vittorio de sica?")
                .as_deref(),
            Some("ladri di biciclette")
        );

        assert_eq!(
            person_query("film con Vittorio De Sica").map(|q| q.role),
            Some(PersonRole::Actor)
        );
        assert_eq!(
            person_query("regia di Vittorio De Sica").map(|q| q.role),
            Some(PersonRole::Director)
        );
        assert!(person_query("film di guerra").is_none());
    }

    #[test]
    fn spiegazione_e_confronto_restano_sui_dati() {
        let tax = Taxonomy::load();
        let a = card(1, "Il Padrino", 1972, 8.7, 175);
        let b = card(2, "Il Padrino - Parte II", 1974, 8.6, 202);

        let exp = explanation_text(&tax, &a);
        assert!(exp.contains("Il Padrino"));
        assert!(exp.contains("Consiglio") || exp.contains("Consiglio:"));

        let cmp = comparison_text(&tax, &a, &b);
        assert!(cmp.contains("Il Padrino"));
        assert!(cmp.contains("Parte II"));
        assert!(cmp.contains("più corto"));
    }

    #[test]
    fn i_titoli_menzionati_si_trovano_nella_chat() {
        let a = card(1, "Il Padrino", 1972, 8.7, 175);
        let b = card(2, "Quei bravi ragazzi", 1990, 8.5, 145);
        let entries = vec![chat::Entry {
            user: "consigli".into(),
            answer: chat::Answer {
                cards: vec![a, b],
                ..Default::default()
            },
        }];
        let mentioned = cards_mentioned(&entries, "confronta Il Padrino e Quei bravi ragazzi");
        assert_eq!(mentioned.len(), 2);
    }
}
