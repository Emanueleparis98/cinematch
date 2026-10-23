use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::ai::{
    build_provider, classify_chat_action, curator, list_models, llm_config, model_details,
    ChatAction, Intent, IntentProvider, LlmKind, LlmProvider, ModelInfo, Theme, ThemeConcept,
    DEEPSEEK_BASE, DEEPSEEK_MODEL, OPENROUTER_BASE, OPENROUTER_MODEL,
};
use crate::domain::constraints::{self, Prune};
use crate::domain::model::{toggle, Details, Facets, Movie, Sort};
use crate::domain::suggestions;
use crate::domain::taxonomy::Taxonomy;
use crate::infra::config::Config;
use crate::infra::winclock;
use crate::tmdb::client::{describe_credential, Tmdb};
use crate::tmdb::models::PagedMovies;
use crate::ui::facets::{FacetAction, Section};
use crate::update;
use crate::ui::images::ImageStore;
use crate::ui::{chat, detail, facets, fx, grid, theme};

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

enum Msg {
    Results { gen: u64, payload: Box<Results> },
    Details { gen: u64, res: Result<Details, String> },
    Summary { gen: u64, res: Result<String, String> },
    Check { res: Result<String, String> },
    Models { gen: u64, res: Result<Vec<ModelInfo>, String> },
    AiCheck { res: Result<String, String> },
    /// Esito del download dei voti IMDb.
    Imdb { res: Result<String, String> },
    /// Avanzamento del download dei voti IMDb (byte scompattati).
    ImdbProgress { done: u64 },
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
    /// Contesto effettivo prima della ricerca: serve ai seguiti e a «Rigenera».
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

#[derive(Clone)]
struct MemoryItem {
    title: String,
    id: Option<u64>,
}

/// Film gia visti o scartati in questa sessione. La chiave e il titolo
/// normalizzato, cosi lo stesso film non entra due volte.
#[derive(Clone, Default)]
struct FilmMemory {
    seen: BTreeMap<String, MemoryItem>,
    rejected: BTreeMap<String, MemoryItem>,
}

impl FilmMemory {
    fn count(&self) -> usize {
        self.seen.len() + self.rejected.len()
    }

    fn mark(&mut self, seen: bool, id: Option<u64>, title: &str) {
        let key = normalize_title(title);
        if key.is_empty() {
            return;
        }
        if let Some(id) = id {
            self.seen.retain(|_, item| item.id != Some(id));
            self.rejected.retain(|_, item| item.id != Some(id));
        }
        self.seen.remove(&key);
        self.rejected.remove(&key);
        let target = if seen {
            &mut self.seen
        } else {
            &mut self.rejected
        };
        target.insert(
            key,
            MemoryItem {
                title: title.trim().to_string(),
                id,
            },
        );
    }

    fn forget_seen(&mut self, key: &str) {
        self.seen.remove(key);
    }

    fn forget_rejected(&mut self, key: &str) {
        self.rejected.remove(key);
    }

    fn snapshot(&self) -> MemorySnapshot {
        let mut snap = MemorySnapshot::default();
        for item in self.seen.values().chain(self.rejected.values()) {
            if let Some(id) = item.id {
                snap.ids.insert(id);
            }
            let key = normalize_title(&item.title);
            if !key.is_empty() {
                snap.titles.insert(key);
            }
        }
        snap
    }
}

#[derive(Clone, Default)]
struct MemorySnapshot {
    ids: HashSet<u64>,
    titles: HashSet<String>,
}

impl MemorySnapshot {
    fn contains(&self, movie: &Movie) -> bool {
        self.ids.contains(&movie.id)
            || self.titles.contains(&normalize_title(&movie.title))
            || self
                .titles
                .contains(&normalize_title(&movie.original_title))
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum MemoryKind {
    Seen,
    Rejected,
}

fn parse_memory_command(text: &str) -> Option<(MemoryKind, Option<String>)> {
    let lower = text.to_lowercase();
    const SEEN: &[&str] = &[
        "ho gia visto",
        "ho già visto",
        "l'ho gia visto",
        "l'ho già visto",
        "ho visto",
        "l'ho visto",
        "gia visto",
        "già visto",
    ];
    const REJECTED: &[&str] = &[
        "non mi interessa",
        "non mi interessano",
        "non mi piacciono",
        "non mi piace",
        "non voglio vedere",
        "scartalo",
        "scarta",
        "escludi",
        "boccia",
    ];
    for (kind, cues) in [(MemoryKind::Seen, SEEN), (MemoryKind::Rejected, REJECTED)] {
        for cue in cues {
            if let Some(pos) = lower.find(cue) {
                return Some((kind, memory_target(&text[pos + cue.len()..])));
            }
        }
    }
    None
}

fn memory_target(raw: &str) -> Option<String> {
    let mut t = raw.trim().trim_matches(['"', '“', '”', '\'', ' ']);
    let lower = t.to_lowercase();
    for prefix in ["il film ", "la film ", "un film ", "film "] {
        if lower.starts_with(prefix) {
            t = t[prefix.len()..].trim();
            break;
        }
    }
    let lower = t.to_lowercase();
    if lower.is_empty()
        || ["questo", "questa", "questi", "queste", "il primo", "la prima"]
            .iter()
            .any(|p| lower == *p || lower.starts_with(&format!("{p} ")))
    {
        None
    } else {
        Some(t.trim_matches(['.', ',', ';', ':', '!', '?', ' ']).to_string())
    }
}

fn is_generic_target(raw: &str) -> bool {
    const GENERIC: [&str; 21] = [
        "film",
        "commedia",
        "horror",
        "dramma",
        "thriller",
        "azione",
        "romantico",
        "fantascienza",
        "animazione",
        "documentario",
        "finale",
        "trama",
        "regia",
        "attori",
        "genere",
        "italiano",
        "francese",
        "americano",
        "inglese",
        "giapponese",
        "coreano",
    ];
    normalize_title(raw)
        .split_whitespace()
        .any(|w| GENERIC.contains(&w))
}

fn find_card_by_title<'a>(
    entries: &'a [chat::Entry],
    target: &str,
) -> Option<&'a chat::Card> {
    let target = normalize_title(target);
    if target.is_empty() {
        return None;
    }
    let mut best: Option<&chat::Card> = None;
    let mut best_len = 0usize;
    for entry in entries.iter().rev() {
        for card in &entry.answer.cards {
            let title = normalize_title(&card.movie.title);
            let original = normalize_title(&card.movie.original_title);
            let hit = title == target
                || original == target
                || (target.len() >= 4
                    && (target.contains(&title) || title.contains(&target)));
            if hit && title.len() >= best_len {
                best = Some(card);
                best_len = title.len();
            }
        }
    }
    best
}

fn find_card_by_id(entries: &[chat::Entry], id: u64) -> Option<chat::Card> {
    entries
        .iter()
        .rev()
        .flat_map(|e| e.answer.cards.iter())
        .find(|c| c.movie.id == id)
        .cloned()
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
    /// Film visti/scartati: valgono per tutte le ricerche di questa sessione.
    memory: FilmMemory,
    show_memory: bool,
    /// Dopo un invio la vista torna in fondo, anche se si era scrollato su.
    chat_jump: bool,
    /// Composer ridotto a barra mentre si legge la conversazione.
    chat_composer_collapsed: bool,
    /// Ultimo offset dello scroll della chat: serve a capire la direzione.
    chat_scroll_last: f32,

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
    /// Ricerca TMDB della griglia: resta com'e quando si passa alla chat.
    grid_query: String,
    /// Ricerca dentro il pannello filtri (non tocca la ricerca TMDB).
    facet_query: String,
    /// Guida alle correnti aperta in una finestra dedicata.
    guide_open: bool,
    /// Pannello filtri visibile; il toggle sta nella barra di comando.
    filters_open: bool,
    /// Ultima larghezza del pannello, ricordata quando lo si minimizza.
    filters_width: f32,
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

    detail: Option<Details>,
    detail_loading: bool,
    /// Generazioni delle richieste scheda/sintesi: una risposta vecchia non
    /// deve sovrascrivere il film aperto dopo.
    detail_gen: u64,
    summary_gen: u64,
    summary: Option<String>,
    summarizing: bool,

    keyword_map: Vec<(String, u64)>,
    dirty: Option<Instant>,
    /// Widget che deve ricevere il fuoco al prossimo frame (composer o ricerca).
    focus: Option<ViewMode>,
    show_settings: bool,
    settings: SettingsDraft,
    settings_check: Option<String>,
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
            memory: FilmMemory::default(),
            show_memory: false,
            chat_jump: false,
            chat_composer_collapsed: false,
            chat_scroll_last: 0.0,
            chat_draft: String::new(),
            suggestions: Vec::new(),
            suggestion_pool: Vec::new(),
            suggestion_base: Facets::default(),
            suggestion_cycle: 0,
            suggestion_shown_at: 0.0,
            suggestion_next_at: 0.0,
            suggestion_hovered: false,
            grid_query: String::new(),
            facet_query: String::new(),
            guide_open: false,
            filters_open: true,
            filters_width: 300.0,
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
            detail: None,
            detail_loading: false,
            detail_gen: 0,
            summary_gen: 0,
            summary: None,
            summarizing: false,
            keyword_map: Vec::new(),
            dirty: None,
            focus: None,
            show_settings: false,
            settings,
            settings_check: None,
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
                    let p = *payload;
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
                    if let Some(err) = p.error {
                        self.movies.clear();
                        self.total = 0;
                        self.total_pages = 0;
                        self.toast(err);
                    } else {
                        self.total = p.total;
                        self.total_pages = p.total_pages;
                        self.movies = p.movies;
                        self.ensure_fonts_for_results();
                        if self.movies.is_empty() {
                            self.toast("Nessun risultato: prova ad allargare i filtri.");
                        }
                    }
                }
                Msg::Details { gen, res } => {
                    if gen != self.detail_gen {
                        continue;
                    }
                    self.detail_loading = false;
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
                        }
                        Err(e) => self.toast(e),
                    }
                }
                Msg::Summary { gen, res } => {
                    if gen != self.summary_gen {
                        continue;
                    }
                    self.summarizing = false;
                    match res {
                        Ok(s) => self.summary = Some(s),
                        Err(e) => self.toast(e),
                    }
                }
                Msg::Check { res } => {
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
                    let p = *payload;
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
        let texts: Vec<&str> = self
            .movies
            .iter()
            .flat_map(|m| [m.title.as_str(), m.original_title.as_str()])
            .collect();
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

        let text = self.grid_query.trim().to_string();
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
                merge_facets(&mut eff, &it.facets);
            }
            let (eff, removed) = constraints::sanitize(&tax, &eff);

            let mut title = intent.as_ref().and_then(|i| i.free_text.clone());
            let similar = intent.as_ref().and_then(|i| i.similar_to.clone());
            let ai_notes = intent.as_ref().map(|i| i.notes.clone()).unwrap_or_default();

            if title.is_none()
                && similar.is_none()
                && !text.is_empty()
                && eff.is_empty()
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

            let outcome: Result<(Vec<Movie>, u32, u32, String), String> =
                if let Some(name) = similar {
                    match tmdb.search(&name, 1) {
                        Ok(p) => match p.results.first() {
                            Some(first) => {
                                let seed = first.title.clone();
                                match tmdb.similar(first.id, page) {
                                    Ok(rec) => Ok((
                                        rec.results.into_iter().map(Movie::from).collect(),
                                        rec.total_results,
                                        rec.total_pages,
                                        format!("Simili a «{seed}»"),
                                    )),
                                    Err(e) => Err(e),
                                }
                            }
                            None => Err(format!("Nessun film trovato per «{name}».")),
                        },
                        Err(e) => Err(e),
                    }
                } else if let Some(t) = title {
                    match tmdb.search(&t, page) {
                        Ok(p) => {
                            let raw = p.results.into_iter().map(Movie::from).collect::<Vec<_>>();
                            let filtered = filter_movies(raw.clone(), &tax, &eff);
                            let mut note = format!("Risultati per «{t}»");
                            if !eff.is_empty() {
                                note.push_str(&format!(
                                    " · filtri applicati localmente ({}/{})",
                                    filtered.len(),
                                    raw.len()
                                ));
                                if filtered.is_empty() {
                                    payload.ai_notes.push(
                                        "La ricerca per titolo non supporta i filtri lato server: nessun risultato compatibile nella prima pagina.".into(),
                                    );
                                }
                            }
                            Ok((filtered, p.total_results, p.total_pages, note))
                        }
                        Err(e) => Err(e),
                    }
                } else if eff.is_empty() {
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
                    discover_with_relaxation(&tmdb, &tax, &eff, &kw, page, &mut payload.removed)
                };

            match outcome {
                Ok((movies, total, pages, headline)) => {
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

            let _ = tx.send(Msg::Results {
                gen,
                payload: Box::new(payload),
            });
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
            self.chat_composer_collapsed = false;
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

    /// Aggiunge una risposta solo testo: spiegazioni, confronti e comandi di
    /// memoria non sono nuove ricerche e non devono toccare il contesto.
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
        self.chat_composer_collapsed = false;
        self.chat_scroll_last = 0.0;
        self.suggestion_hovered = false;
        self.focus = Some(ViewMode::Chat);
    }

    fn handle_memory_command(&mut self, text: &str) -> bool {
        let Some((kind, target)) = parse_memory_command(text) else {
            return false;
        };
        let target = match target {
            Some(t) if !t.trim().is_empty() => t,
            _ => match last_cards(&self.chat).first() {
                Some(card) => card.movie.title.clone(),
                None => {
                    self.push_text_answer(
                        text.to_string(),
                        "Dimmi il titolo esatto del film che hai visto o che vuoi scartare.".into(),
                        "memoria".into(),
                    );
                    return true;
                }
            },
        };
        let card = find_card_by_title(&self.chat, &target).cloned();
        let (id, title) = if let Some(card) = card {
            (Some(card.movie.id), card.movie.title)
        } else {
            if is_generic_target(&target) || normalize_title(&target).is_empty() {
                return false;
            }
            (None, target)
        };
        self.memory.mark(kind == MemoryKind::Seen, id, &title);
        if let Some(id) = id {
            for entry in &mut self.chat {
                entry.answer.cards.retain(|c| c.movie.id != id);
            }
        }
        let stato = if kind == MemoryKind::Seen {
            "già visto"
        } else {
            "scartato"
        };
        self.push_text_answer(
            text.to_string(),
            format!("Segno «{title}» come {stato}: lo escludo dalle prossime proposte."),
            "memoria".into(),
        );
        true
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

    fn explain_movie(&mut self, id: u64) {
        if let Some(card) = find_card_by_id(&self.chat, id) {
            self.explain_card(card);
        } else {
            self.toast("Non ho più la scheda di quel film.");
        }
    }

    fn explain_card(&mut self, card: chat::Card) {
        let user = format!("Perché «{}»?", card.movie.title);
        let intro = explanation_text(&self.tax, &card);
        self.push_text_answer(user, intro, "spiegazione".into());
    }

    fn compare_answer(&mut self, index: usize) {
        let Some(entry) = self.chat.get(index) else {
            return;
        };
        if entry.answer.cards.len() < 2 {
            self.toast("Servono almeno due film da confrontare.");
            return;
        }
        let a = entry.answer.cards[0].clone();
        let b = entry.answer.cards[1].clone();
        self.compare_cards(a, b);
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
        self.chat_draft.clear();
        self.chat_jump = true;
        self.chat_composer_collapsed = false;
        self.suggestion_hovered = false;
        // Memoria, spiegazioni e confronti sono comandi locali: non sono
        // nuove ricerche e funzionano anche senza una chiamata AI.
        if self.handle_memory_command(&text) || self.handle_meta_command(&text) {
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
        let memory = self.memory.snapshot();
        self.spawn_chat(text, exclude, None, memory);
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
        self.chat_composer_collapsed = false;
        self.chat_scroll_last = 0.0;
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

    fn spawn_chat(
        &mut self,
        text: String,
        exclude: Vec<u64>,
        reuse: Option<Intent>,
        memory: MemorySnapshot,
    ) {
        let index = self.chat.len();
        self.chat.push(chat::Entry {
            user: text.clone(),
            answer: chat::Answer::busy(),
        });
        self.spawn_chat_worker(index, text, exclude, reuse, memory);
    }

    /// «Rigenera»: stessi criteri, altri film (esclude quelli mostrati).
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
        let memory = self.memory.snapshot();
        self.spawn_chat_worker(index, text, exclude, reuse, memory);
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
        self.grid_query = user;
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
        memory: MemorySnapshot,
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
                    merge_facets(&mut eff, &it.facets);
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

            let pool = match chat_pool(
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
            let before_filter = movies.len();
            movies.retain(|m| !exclude.contains(&m.id));
            if title.is_none() {
                movies.retain(|m| !memory.contains(m));
            }
            if movies.len() < before_filter {
                payload
                    .notes
                    .push("Ho escluso i film già visti o scartati.".into());
            }
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
            // La scheda vive nella griglia: il click su una card della chat
            // deve portare l'utente dove la scheda viene davvero mostrata.
            self.switch_mode(ViewMode::Griglia);
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
        if let Some(i) = a.regenerate {
            self.regenerate(i);
        }
        if let Some(i) = a.retry {
            self.regenerate(i);
        }
        if let Some(id) = a.explain {
            self.explain_movie(id);
        }
        if let Some(i) = a.compare {
            self.compare_answer(i);
        }
        if let Some(i) = a.show_all {
            self.show_all_in_grid(i);
        }
        if a.new_chat {
            self.new_chat();
        }
        if let Some(text) = a.suggestion {
            // Gli spunti non partono da soli: riempiono il composer e basta.
            self.chat_draft = text;
            self.focus = Some(ViewMode::Chat);
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

    fn close_detail(&mut self) {
        self.detail = None;
        self.summary = None;
        self.detail_loading = false;
        self.summarizing = false;
        self.detail_gen = self.detail_gen.wrapping_add(1);
        self.summary_gen = self.summary_gen.wrapping_add(1);
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

    fn spawn_summary(&mut self) {
        let Some(d) = self.detail.clone() else { return };
        if d.movie.overview.trim().is_empty() {
            return;
        }
        self.summarizing = true;
        self.summary_gen = self.summary_gen.wrapping_add(1);
        let gen = self.summary_gen;
        let provider = self.provider.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let res = provider.summarize(&d.movie.overview);
            let _ = tx.send(Msg::Summary { gen, res });
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
        };
        match cfg.save() {
            Ok(()) => {
                self.cfg = cfg.clone();
                self.tmdb = Self::build_tmdb(&cfg);
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

    /// Barra di comando unica: marchio, Chat/Griglia e, in Griglia, ricerca e
    /// comandi. Fa anche da titlebar (trascinamento e controlli finestra);
    /// il resto del chrome vive nel piè di pagina.
    fn header(&mut self, ctx: &egui::Context) {
        let griglia = self.mode == ViewMode::Griglia;
        let want_focus = griglia && self.focus == Some(ViewMode::Griglia);
        let compact = ctx.screen_rect().width() < 1120.0;
        let cap = if compact { 22.0 } else { 24.0 };
        let mut do_search = false;
        let mut toggle_filters = false;
        let mut new_chat = false;
        let mut show_memory = false;

        egui::TopBottomPanel::top("header")
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL_SOFT)
                    .inner_margin(egui::Margin::symmetric(14.0, 7.0)),
            )
            .show(ctx, |ui| {
                // La barra fa anche da titlebar: il fondo e' trascinabile, i
                // controlli aggiunti dopo restano cliccabili.
                let r = ui.max_rect();
                let drag_rect = egui::Rect::from_min_max(
                    egui::pos2(r.left() + 6.0, r.top() + 6.0),
                    egui::pos2(r.right() - 6.0, r.bottom()),
                );
                let drag = ui.interact(
                    drag_rect,
                    egui::Id::new("window-drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(
                        egui::viewport::ViewportCommand::StartDrag,
                    );
                }
                if drag.double_clicked() {
                    let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx().send_viewport_cmd(
                        egui::viewport::ViewportCommand::Maximized(!max),
                    );
                }
                // Niente cursore Grab: su Windows diventa la croce a quattro
                // frecce anche quando il puntatore e' sopra i pulsanti.

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(egui::RichText::new("Cine").size(19.0).strong());
                    ui.label(
                        egui::RichText::new("Match")
                            .size(19.0)
                            .strong()
                            .color(theme::ACCENT),
                    );
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.add_space(14.0);

                    if let Some(mode) = mode_switch(ui, self.mode) {
                        self.switch_mode(mode);
                    }

                    if griglia {
                        ui.add_space(12.0);
                        // Una sola barra comandi: la ricerca si prende lo
                        // spazio che resta, i comandi restano a destra.
                        // Il gruppo a destra include anche i tre controlli
                        // finestra, quindi va riservato per intero.
                        let captions = cap * 3.0 + 2.0 * 2.0 + 10.0;
                        let fixed = if compact {
                            26.0 + 96.0 + 40.0 + 28.0 + captions + 28.0
                                + 5.0 * 8.0
                                + 16.0
                        } else {
                            64.0 + 96.0 + 156.0 + 28.0 + 92.0 + 18.0
                                + captions
                                + 28.0
                                + 6.0 * 8.0
                                + 16.0
                        };
                        let search_w = (ui.available_width() - fixed).max(150.0);
                        let hint = if compact {
                            "Cerca o descrivi…"
                        } else if self.provider.is_llm() {
                            "Descrivi cosa vuoi vedere… (Invio: interpreta l'AI)"
                        } else {
                            "Titolo, corrente, tema… (Invio: cerca)"
                        };
                        let field_id = egui::Id::new("grid-query");
                        let focused = ui.memory(|m| m.has_focus(field_id));
                        let outer = egui::Frame::none()
                            .fill(if focused {
                                theme::mix(theme::CARD, theme::ACCENT_DIM, 0.45)
                            } else {
                                theme::CARD
                            })
                            .stroke(egui::Stroke::new(
                                1.0_f32,
                                if focused { theme::ACCENT } else { theme::BORDER },
                            ))
                            .rounding(egui::Rounding::same(8.0))
                            .inner_margin(egui::Margin::symmetric(10.0, 3.0))
                            .show(ui, |ui| {
                                ui.add_sized(
                                    [search_w - 20.0, 20.0],
                                    egui::TextEdit::singleline(&mut self.grid_query)
                                        .id(field_id)
                                        .frame(false)
                                        .hint_text(hint)
                                        .font(egui::TextStyle::Body),
                                )
                            });
                        let resp = outer.inner;
                        if focused {
                            fx::ring(ui, outer.response.rect, 8.0, theme::ACCENT, 0.8);
                        }
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
                        if compact {
                            if compact_search(ui).clicked() {
                                do_search = true;
                            }
                        } else if theme::primary_button(ui, "Cerca", egui::vec2(64.0, 26.0))
                            .clicked()
                        {
                            do_search = true;
                        }
                        if facets::panel_toggle(
                            ui,
                            self.filters_open,
                            self.facets.active_count(),
                        )
                        .clicked()
                        {
                            toggle_filters = true;
                        }
                        let sort_label = if compact {
                            "⇅".to_string()
                        } else {
                            format!("Ordina: {}", self.facets.sort.label())
                        };
                        let sort = egui::ComboBox::from_id_salt("grid-sort")
                            .selected_text(
                                egui::RichText::new(sort_label).size(11.5),
                            )
                            .width(if compact { 40.0 } else { 150.0 })
                            .show_ui(ui, |ui| {
                                for s in Sort::ALL {
                                    ui.selectable_value(
                                        &mut self.facets.sort,
                                        s,
                                        s.label(),
                                    );
                                }
                            });
                        if compact {
                            sort.response.on_hover_text("Ordina");
                        }
                        if ui
                            .add_sized(
                                [26.0, 26.0],
                                egui::Button::new(
                                    egui::RichText::new("↻").size(13.0).color(theme::ACCENT_3),
                                )
                                .frame(false),
                            )
                            .on_hover_text("Aggiorna la griglia")
                            .clicked()
                        {
                            self.resanitize();
                            self.page = 1;
                            self.dirty = Some(Instant::now());
                        }
                        if !compact {
                            let count = if self.loading {
                                "Cerco…".to_string()
                            } else {
                                format!("{} risultati", theme::thousands(self.total))
                            };
                            let label = ui.label(
                                egui::RichText::new(count).size(10.0).color(theme::DIM),
                            );
                            if !self.headline.is_empty() {
                                label.on_hover_text(&self.headline);
                            }
                            if self.loading {
                                ui.spinner();
                            }
                        }
                    }

                    // A destra un unico gruppo: controlli finestra, impostazioni
                    // e, in Chat, la nuova chat. I widget aggiunti dopo vincono
                    // i clic sul fondo trascinabile.
                    ui.with_layout(
                        egui::Layout::right_to_left(egui::Align::Center),
                        |ui| {
                            ui.spacing_mut().item_spacing.x = 2.0;
                            ui.spacing_mut().button_padding = egui::vec2(2.0, 3.0);
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
                            ui.add_space(6.0);
                            let update_ready = self.update.available.is_some();
                            let gear = ui
                                .add_sized(
                                    [28.0, 28.0],
                                    egui::Button::new(
                                        egui::RichText::new("⚙").size(14.0),
                                    )
                                    .frame(false),
                                )
                                .on_hover_text(if update_ready {
                                    "Impostazioni · aggiornamento disponibile"
                                } else {
                                    "Impostazioni"
                                });
                            if update_ready {
                                let dot = gear.rect.right_top() + egui::vec2(-3.0, 3.0);
                                ui.painter().circle_filled(dot, 3.2, theme::ACCENT_2);
                                ui.painter().circle_stroke(
                                    dot,
                                    3.2,
                                    egui::Stroke::new(1.0_f32, theme::PANEL_SOFT),
                                );
                            }
                            if gear.clicked() {
                                self.show_settings = true;
                            }
                            if !griglia {
                                ui.spacing_mut().item_spacing.x = 8.0;
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new("Nuova chat").size(11.0),
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
                                        .rounding(egui::Rounding::same(9.0))
                                        .min_size(egui::vec2(0.0, 26.0)),
                                    )
                                    .on_hover_text(
                                        "Svuota la conversazione (la memoria dei film resta)",
                                    )
                                    .clicked()
                                {
                                    new_chat = true;
                                }
                                let memory_count = self.memory.count();
                                let memory_label = if compact {
                                    "M".to_string()
                                } else if memory_count > 0 {
                                    format!("Memoria · {memory_count}")
                                } else {
                                    "Memoria".to_string()
                                };
                                if ui
                                    .add(
                                        egui::Button::new(
                                            egui::RichText::new(memory_label).size(11.0),
                                        )
                                        .fill(egui::Color32::TRANSPARENT)
                                        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
                                        .rounding(egui::Rounding::same(9.0))
                                        .min_size(egui::vec2(0.0, 26.0)),
                                    )
                                    .on_hover_text(
                                        "Film visti, scartati e cronologia della conversazione",
                                    )
                                    .clicked()
                                {
                                    show_memory = true;
                                }
                                if self.chat_busy {
                                    ui.add_space(2.0);
                                    let now = ui.ctx().input(|i| i.time);
                                    fx::dot(ui, theme::ACCENT, theme::pulse(now, 5.0));
                                }
                            }
                        },
                    );
                });

                // Filo di luce sul bordo inferiore: separa senza pesare.
                let r = ui.min_rect();
                let y = r.bottom() + 4.0;
                ui.painter().line_segment(
                    [
                        egui::pos2(ctx.screen_rect().left(), y),
                        egui::pos2(ctx.screen_rect().right(), y),
                    ],
                    egui::Stroke::new(1.0_f32, theme::alpha(theme::BORDER, 160)),
                );
            });

        if want_focus {
            self.focus = None;
        }
        if toggle_filters {
            self.toggle_filters();
        }
        if do_search {
            self.page = 1;
            self.launch_search(true);
        }
        if new_chat {
            self.new_chat();
        }
        if show_memory {
            self.show_memory = true;
        }
    }

    fn left_panel(&mut self, ctx: &egui::Context) {
        // Il pannello e uno strumento della griglia: in chat sparisce del
        // tutto, ma lo stato (aperto, filtri, larghezza) resta intatto.
        if self.mode != ViewMode::Griglia || !self.filters_open {
            return;
        }
        let (left_max, _) = side_panel_widths(ctx.screen_rect().width());
        let panel = egui::SidePanel::left("facets")
            .resizable(true)
            .default_width(self.filters_width.min(left_max))
            .width_range(260.0..=left_max)
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL_SOFT)
                    .inner_margin(egui::Margin::symmetric(12.0, 10.0)),
            )
            .show(ctx, |ui| {
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
                    egui::Rounding::same(0.0),
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
        self.filters_width = panel.response.rect.width();
    }

    fn right_panel(&mut self, ctx: &egui::Context) {
        // La scheda appartiene alla griglia; la chat resta a tutta larghezza.
        if self.mode != ViewMode::Griglia {
            return;
        }
        // I filtri hanno la precedenza: mai due pannelli aperti insieme.
        if self.filters_open {
            return;
        }
        let (_, right_max) = side_panel_widths(ctx.screen_rect().width());
        let Some(d) = self.detail.clone() else {
            if self.detail_loading {
                egui::SidePanel::right("detail_loading")
                    .default_width(380.0_f32.min(right_max))
                    .frame(
                        egui::Frame::none()
                            .fill(theme::PANEL_SOFT)
                            .inner_margin(egui::Margin::same(14.0)),
                    )
                    .show(ctx, |ui| {
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
        egui::SidePanel::right("detail")
            .resizable(true)
            .default_width(392.0_f32.min(right_max))
            .width_range(280.0..=right_max)
            .frame(
                egui::Frame::none()
                    .fill(theme::PANEL_SOFT)
                    .inner_margin(egui::Margin::symmetric(14.0, 10.0)),
            )
            .show(ctx, |ui| {
                let ai_available = self.provider.is_llm();
                let summary = self.summary.clone();
                let actions = detail::panel(
                    ui,
                    &self.images,
                    &d,
                    summary.as_deref(),
                    self.summarizing,
                    ai_available,
                );
                for action in actions {
                    match action {
                        detail::DetailAction::Open(id) => {
                            if id == d.movie.id {
                                if self.detail.is_some() {
                                    self.page = 1;
                                    self.grid_query = d.movie.title.clone();
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
                        detail::DetailAction::Summarize => self.spawn_summary(),
                        detail::DetailAction::OpenUrl(url) => open_url(&url),
                        detail::DetailAction::Close => {
                            self.close_detail();
                        }
                    }
                }
            });
    }

    fn central(&mut self, ctx: &egui::Context) {
        let chat = self.mode == ViewMode::Chat;
        egui::CentralPanel::default()
            .frame(
                egui::Frame::none()
                    .fill(egui::Color32::TRANSPARENT)
                    // In chat il composer arriva fino al piè di pagina; la
                    // griglia conserva il suo respiro sopra e sotto.
                    .inner_margin(egui::Margin::symmetric(14.0, if chat { 0.0 } else { 10.0 })),
            )
            .show(ctx, |ui| {
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
                                .rounding(egui::Rounding::same(10.0)),
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
                    self.chat_central(ui, ctx);
                } else {
                    self.grid_central(ui);
                }
            });
    }

    /// Casa e conversazione: la casa ha i quattro spunti al centro e il
    /// composer sotto, oppure ancorato in basso sulle finestre basse; appena
    /// si invia, la conversazione occupa lo spazio e il composer resta in basso.
    fn chat_central(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        if self.chat.is_empty() {
            self.refresh_home_suggestions(now);
            let want_focus = self.focus == Some(ViewMode::Chat);
            let actions = chat::home(
                ui,
                &mut self.chat_draft,
                self.chat_busy,
                self.provider.is_llm(),
                want_focus,
                &self.suggestions,
                now,
                self.suggestion_shown_at,
            );
            if want_focus {
                self.focus = None;
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
            if now - self.suggestion_shown_at < 0.45 {
                ctx.request_repaint();
            }
            if !self.suggestion_hovered {
                let wait = (self.suggestion_next_at - now).max(0.05);
                ctx.request_repaint_after(Duration::from_secs_f64(wait));
            }
        } else {
            // La conversazione sta in una colonna centrata, larga al massimo
            // come la casa: su schermi grandi non si allarga all'infinito.
            let width = ui.available_width().min(980.0);
            let pad = ((ui.available_width() - width) * 0.5).max(0.0);
            let height = ui.available_height();
            let mut actions = None;
            {
                let images = &self.images;
                let tax = &self.tax;
                let entries = &self.chat;
                let busy = self.chat_busy;
                let jump = &mut self.chat_jump;
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(pad);
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            actions = Some(chat::view(
                                ui,
                                images,
                                tax,
                                entries,
                                busy,
                                now,
                                jump,
                                112.0,
                            ));
                        },
                    );
                });
            }
            if let Some(actions) = actions {
                let at_bottom = actions.at_bottom;
                let delta = actions.scroll_offset - self.chat_scroll_last;
                self.chat_scroll_last = actions.scroll_offset;
                let typing = ctx.memory(|m| m.has_focus(egui::Id::new("chat-composer")));
                if at_bottom {
                    self.chat_composer_collapsed = false;
                } else if delta < -18.0 && !typing {
                    // Si sta leggendo verso l'alto: il composer si riduce.
                    self.chat_composer_collapsed = true;
                }
                self.apply_chat_actions(actions);
            }

            // Il composer vive in un'Area sopra la conversazione: disegnato
            // dopo lo ScrollArea, vince sempre l'hit-test del mouse. La
            // conversazione lascia 112 px in fondo (vedi chat::view).
            if !self.chat.is_empty() && !self.show_settings && !self.guide_open {
                if self.focus == Some(ViewMode::Chat) {
                    self.chat_composer_collapsed = false;
                }
                let want_focus = self.focus == Some(ViewMode::Chat);
                let area_w = (ui.max_rect().width() - 56.0).clamp(240.0, 820.0);
                let mut send = false;
                let mut expand = false;
                egui::Area::new(egui::Id::new("chat-composer-dock"))
                    .order(egui::Order::Foreground)
                    .anchor(
                        egui::Align2::CENTER_BOTTOM,
                        egui::vec2(0.0, -8.0),
                    )
                    .show(ctx, |ui| {
                        ui.set_width(area_w);
                        if self.chat_composer_collapsed {
                            expand = chat::composer_mini(
                                ui,
                                &self.chat_draft,
                                self.chat_busy,
                                area_w,
                            );
                        } else {
                            send = chat::composer(
                                ui,
                                &mut self.chat_draft,
                                self.chat_busy,
                                want_focus,
                                area_w,
                            );
                        }
                    });
                if expand {
                    self.chat_composer_collapsed = false;
                    self.focus = Some(ViewMode::Chat);
                }
                if want_focus {
                    self.focus = None;
                }
                if send {
                    self.send_chat();
                }
            } else if self.focus == Some(ViewMode::Chat) {
                self.focus = None;
            }
        }

        // Un velo brevissimo accompagna il passaggio casa <-> conversazione:
        // il composer si sposta in basso senza uno stacco secco.
        let t = ctx.animate_bool_with_time(
            egui::Id::new("chat-view-fade"),
            !self.chat.is_empty(),
            0.18,
        );
        let veil = if self.chat.is_empty() { t } else { 1.0 - t };
        if veil > 0.01 {
            ui.painter().rect_filled(
                ui.max_rect(),
                egui::Rounding::same(0.0),
                theme::alpha(theme::BG, (veil * 120.0) as u8),
            );
        }
    }

    /// Griglia a tutta altezza: conteggio e comandi stanno nella barra di
    /// comando, quindi qui restano i poster.
    fn grid_central(&mut self, ui: &mut egui::Ui) {
        ui.add_space(4.0);
        if self.total_pages > 1 {
            self.grid_pager(ui);
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

    /// Pager compatto sopra la griglia: sta vicino ai risultati.
    fn grid_pager(&mut self, ui: &mut egui::Ui) {
        let mut target = None;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!(
                    "Pagina {} di {}",
                    self.page.min(self.total_pages.max(1)),
                    self.total_pages
                ))
                .size(10.5)
                .color(theme::DIM),
            );
            if ui
                .add_enabled(
                    self.page > 1 && !self.loading,
                    egui::Button::new(egui::RichText::new("‹").size(15.0))
                        .frame(false)
                        .min_size(egui::vec2(24.0, 22.0)),
                )
                .on_hover_text("Pagina precedente")
                .clicked()
            {
                target = Some(self.page.saturating_sub(1));
            }
            if ui
                .add_enabled(
                    self.page < self.total_pages && !self.loading,
                    egui::Button::new(egui::RichText::new("›").size(15.0))
                        .frame(false)
                        .min_size(egui::vec2(24.0, 22.0)),
                )
                .on_hover_text("Pagina successiva")
                .clicked()
            {
                target = Some((self.page + 1).min(self.total_pages));
            }
        });
        if let Some(page) = target {
            self.page = page;
            self.spawn_search(false);
        }
        ui.add_space(2.0);
    }

    fn memory_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
        let mut forget_seen: Option<String> = None;
        let mut forget_rejected: Option<String> = None;
        let mut clear_seen = false;
        let mut clear_rejected = false;
        egui::Window::new("Memoria e cronologia")
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .vscroll(true)
            .default_width(560.0)
            .show(ctx, |ui| {
                ui.label(
                    egui::RichText::new("Film visti")
                        .size(12.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                if self.memory.seen.is_empty() {
                    ui.label(
                        egui::RichText::new("Nessun film segnato come visto.")
                            .size(11.0)
                            .color(theme::DIM),
                    );
                } else {
                    for (key, item) in &self.memory.seen {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&item.title).size(11.5));
                            if ui.small_button("Dimentica").clicked() {
                                forget_seen = Some(key.clone());
                            }
                        });
                    }
                }
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new("Film scartati")
                        .size(12.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                if self.memory.rejected.is_empty() {
                    ui.label(
                        egui::RichText::new("Nessun film scartato.")
                            .size(11.0)
                            .color(theme::DIM),
                    );
                } else {
                    for (key, item) in &self.memory.rejected {
                        ui.horizontal(|ui| {
                            ui.label(egui::RichText::new(&item.title).size(11.5));
                            if ui.small_button("Dimentica").clicked() {
                                forget_rejected = Some(key.clone());
                            }
                        });
                    }
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    if !self.memory.seen.is_empty() && ui.small_button("Svuota visti").clicked() {
                        clear_seen = true;
                    }
                    if !self.memory.rejected.is_empty()
                        && ui.small_button("Svuota scartati").clicked()
                    {
                        clear_rejected = true;
                    }
                });
                ui.label(
                    egui::RichText::new(
                        "La memoria vale per questa sessione. I film visti e scartati non vengono riproposti nelle ricerche.",
                    )
                    .size(10.0)
                    .color(theme::DIM),
                );
                ui.add_space(8.0);
                ui.separator();
                ui.label(
                    egui::RichText::new("Cronologia")
                        .size(12.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                if self.chat.is_empty() {
                    ui.label(
                        egui::RichText::new("Nessuna conversazione ancora.")
                            .size(11.0)
                            .color(theme::DIM),
                    );
                } else {
                    for entry in self.chat.iter().rev().take(30) {
                        ui.label(
                            egui::RichText::new(format!("Tu: {}", entry.user))
                                .size(11.0)
                                .strong(),
                        );
                        let a = &entry.answer;
                        if !a.cards.is_empty() {
                            let titles = a
                                .cards
                                .iter()
                                .map(|c| c.movie.title.as_str())
                                .collect::<Vec<_>>()
                                .join(", ");
                            ui.label(
                                egui::RichText::new(format!("Consigliati: {titles}"))
                                    .size(10.5)
                                    .color(theme::DIM),
                            );
                        } else if !a.intro.is_empty() {
                            ui.label(
                                egui::RichText::new(theme::truncate(&a.intro, 180))
                                    .size(10.5)
                                    .color(theme::DIM),
                            );
                        }
                        ui.add_space(5.0);
                    }
                }
            });
        if let Some(key) = forget_seen {
            self.memory.forget_seen(&key);
        }
        if let Some(key) = forget_rejected {
            self.memory.forget_rejected(&key);
        }
        if clear_seen {
            self.memory.seen.clear();
        }
        if clear_rejected {
            self.memory.rejected.clear();
        }
        if !open {
            self.show_memory = false;
        }
    }

    fn settings_window(&mut self, ctx: &egui::Context) {
        let mut open = true;
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
        egui::Window::new("Impostazioni")
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .vscroll(true)
            .default_width(440.0)
            .show(ctx, |ui| {
                if update::enabled() {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new("AGGIORNAMENTI")
                                .size(11.5)
                                .strong()
                                .color(theme::DIM),
                        );
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
                                        .rounding(egui::Rounding::same(9.0)),
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
                ui.label(
                    egui::RichText::new("TMDB")
                        .size(12.0)
                        .strong()
                        .color(theme::ACCENT),
                );
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
                ui.label(
                    egui::RichText::new("Interprete delle richieste")
                        .size(12.0)
                        .strong()
                        .color(theme::ACCENT),
                );
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
                    ui.horizontal_wrapped(|ui| {
                        ui.label(
                            egui::RichText::new("Sforzo di ragionamento")
                                .size(11.0)
                                .color(theme::DIM),
                        );
                        for (id, label) in [
                            ("max", "Massimo"),
                            ("high", "Alto"),
                            ("medium", "Medio"),
                            ("low", "Basso"),
                            ("none", "Disattivato"),
                        ] {
                            ui.selectable_value(
                                &mut self.settings.effort,
                                id.to_string(),
                                egui::RichText::new(label).size(11.0),
                            );
                        }
                    });
                    ui.label(
                        egui::RichText::new(
                            "Massimo ragiona di più: risposte più curate, ma più lente e più token.",
                        )
                        .size(10.0)
                        .color(theme::DIM),
                    );
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

                if self.settings.provider != "rules" {
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        if ui
                            .add_enabled(
                                !self.detecting,
                                egui::Button::new(egui::RichText::new("Rileva modelli").size(12.0))
                                    .rounding(egui::Rounding::same(10.0)),
                            )
                            .clicked()
                        {
                            detect = true;
                        }
                        if ui
                            .add_enabled(
                                !self.ai_checking,
                                egui::Button::new(egui::RichText::new("Verifica risposta AI").size(12.0))
                                    .rounding(egui::Rounding::same(10.0)),
                            )
                            .clicked()
                        {
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
                    ui.label(
                        egui::RichText::new("Lingua dei contenuti")
                            .size(11.5)
                            .color(theme::DIM),
                    );
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
                ui.label(
                    egui::RichText::new("DATI LOCALI")
                        .size(11.5)
                        .strong()
                        .color(theme::DIM),
                );
                ui.add_space(2.0);
                let ready = crate::infra::imdb::is_ready();
                ui.horizontal(|ui| {
                    let label = if ready {
                        "Aggiorna i voti IMDb (7 MB)"
                    } else {
                        "Scarica i voti IMDb (7 MB)"
                    };
                    if ui
                        .add_enabled(
                            !self.imdb_busy,
                            egui::Button::new(egui::RichText::new(label).size(12.0))
                                .rounding(egui::Rounding::same(10.0)),
                        )
                        .on_hover_text(
                            "Dati pubblici IMDb (datasets.imdbws.com): nessuna chiave, nessun limite. Consente di mostrare il voto IMDb accanto a quello TMDB.",
                        )
                        .clicked()
                    {
                        self.start_imdb_download();
                    }
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
                ui.horizontal(|ui| {
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Salva").size(13.0))
                                .fill(theme::ACCENT)
                                .rounding(egui::Rounding::same(10.0)),
                        )
                        .clicked()
                    {
                        save = true;
                    }
                    if ui
                        .add(
                            egui::Button::new(egui::RichText::new("Verifica credenziale").size(12.0))
                                .rounding(egui::Rounding::same(10.0)),
                        )
                        .clicked()
                    {
                        check = true;
                    }
                    ui.label(
                        egui::RichText::new(format!("config: {}", Config::path().display()))
                            .size(10.0)
                            .color(theme::DIM),
                    );
                });

                ui.add_space(8.0);
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    ui.label(
                        egui::RichText::new(format!("build {}", self.build))
                            .size(9.5)
                            .color(theme::FAINT),
                    );
                    ui.label(
                        egui::RichText::new(
                            "This product uses the TMDB API but is not endorsed or certified by TMDB.",
                        )
                        .size(9.5)
                        .color(theme::DIM),
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
        if !open {
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
            if self.chat_composer_collapsed {
                62.0
            } else {
                120.0
            }
        } else {
            46.0
        };
        let y = -lift + (1.0 - enter) * 20.0;
        egui::Area::new(egui::Id::new("toast"))
            .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, y))
            .order(egui::Order::Foreground)
            .interactable(false)
            .show(ctx, |ui| {
                ui.set_opacity(a);
                egui::Frame::none()
                    .fill(theme::mix(theme::CARD_HOVER, theme::PANEL, 0.35))
                    .stroke(egui::Stroke::new(1.0_f32, theme::alpha(theme::ACCENT, 150)))
                    .rounding(egui::Rounding::same(13.0))
                    .shadow(egui::epaint::Shadow {
                        offset: egui::vec2(0.0, 12.0),
                        blur: 34.0,
                        spread: 0.0,
                        color: egui::Color32::from_black_alpha(150),
                    })
                    .inner_margin(egui::Margin::symmetric(14.0, 9.0))
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
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll();
        if self.images.poll() {
            ctx.request_repaint();
        }
        self.tick();
        window_resize_edges(ctx);
        let now = ctx.input(|i| i.time);
        fx::background(ctx, now);
        self.shortcuts(ctx);
        if !self.boot_done {
            self.boot_done = true;
            if self.tmdb.is_some() {
                self.load_trending();
            } else {
                self.show_settings = true;
            }
        }
        self.maybe_check_updates();
        self.header(ctx);
        self.left_panel(ctx);
        self.right_panel(ctx);
        self.central(ctx);
        if self.guide_open {
            let actions = facets::guide_window(ctx, &self.tax, &mut self.guide_open);
            if !actions.is_empty() {
                self.apply_actions(actions);
            }
        }
        if self.show_settings {
            self.settings_window(ctx);
        }
        if self.show_memory {
            self.memory_window(ctx);
        }
        self.toast_overlay(ctx);
        if self.loading
            || self.detail_loading
            || self.summarizing
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
            if self.mode == ViewMode::Chat {
                self.chat_composer_collapsed = false;
            }
            ctx.input_mut(|i| {
                i.consume_key(egui::Modifiers::NONE, egui::Key::Slash);
                i.events
                    .retain(|e| !matches!(e, egui::Event::Text(_)));
            });
        }
        // Se il composer e' minimizzato e si inizia a scrivere, si riapre
        // subito e il primo carattere non va perso.
        if self.mode == ViewMode::Chat
            && self.chat_composer_collapsed
            && !typing
            && !self.show_settings
            && !self.guide_open
        {
            // Con una bozza gia' pronta, Invio riapre e invia in un colpo.
            if !self.chat_busy
                && !self.chat_draft.trim().is_empty()
                && ctx.input(|i| i.key_pressed(egui::Key::Enter) && !i.modifiers.shift)
            {
                self.chat_composer_collapsed = false;
                self.send_chat();
                ctx.input_mut(|i| i.consume_key(egui::Modifiers::NONE, egui::Key::Enter));
                return;
            }
            let typed: String = ctx.input(|i| {
                i.events
                    .iter()
                    .filter_map(|e| match e {
                        egui::Event::Text(t) => Some(t.clone()),
                        _ => None,
                    })
                    .collect()
            });
            if !typed.is_empty() {
                self.chat_composer_collapsed = false;
                self.focus = Some(ViewMode::Chat);
                self.chat_draft.push_str(&typed);
                ctx.input_mut(|i| i.events.retain(|e| !matches!(e, egui::Event::Text(_))));
            }
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
            } else if self.show_memory {
                self.show_memory = false;
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

fn merge_facets(base: &mut Facets, extra: &Facets) {
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
    if extra.min_rating.is_some() {
        base.min_rating = extra.min_rating;
    }
    if extra.max_rating.is_some() {
        base.max_rating = extra.max_rating;
    }
    if extra.min_votes.is_some() {
        base.min_votes = extra.min_votes;
    }
    if extra.runtime.is_some() {
        base.runtime = extra.runtime;
    }
    if extra.sort != Sort::default() {
        base.sort = extra.sort;
    }
}

fn filter_movies(movies: Vec<Movie>, tax: &Taxonomy, f: &Facets) -> Vec<Movie> {
    if f.is_empty() {
        return movies;
    }
    let year = constraints::year_range(tax, f).ok().flatten();
    let langs = constraints::effective_countries(tax, f)
        .iter()
        .filter_map(|c| constraints::country_language(c))
        .collect::<Vec<&'static str>>();
    let forbidden = constraints::forbidden_genres(tax, f);
    movies
        .into_iter()
        .filter(|m| {
            if let Some((a, b)) = year {
                match m.year {
                    Some(y) if y >= a && y <= b => {}
                    _ => return false,
                }
            }
            if !f.genres.is_empty() && !f.genres.iter().any(|g| m.genre_ids.contains(g)) {
                return false;
            }
            if m.genre_ids.iter().any(|g| f.without_genres.contains(g)) {
                return false;
            }
            if m.genre_ids.iter().any(|g| forbidden.contains(g)) {
                return false;
            }
            if !langs.is_empty() && !langs.contains(&m.lang.as_str()) {
                return false;
            }
            if let Some(r) = f.min_rating {
                if m.rating < r {
                    return false;
                }
            }
            // Un voto 0 significa «nessun voto»: non e un giudizio basso.
            if let Some(r) = f.max_rating {
                if m.rating > 0.0 && m.rating > r {
                    return false;
                }
            }
            if let Some(v) = f.min_votes {
                if m.votes < v {
                    return false;
                }
            }
            true
        })
        .collect()
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
            Sort::Popularity => {
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
        let movies = dedupe(rec.results.into_iter().map(Movie::from).collect());
        if movies.is_empty() {
            return Err(format!("Nessun film simile a «{seed}»."));
        }
        return Ok((movies, rec.total_results, format!("Simili a «{seed}»"), true));
    }
    if let Some(t) = title {
        let p1 = tmdb.search(t, 1)?;
        let total = p1.total_results;
        let mut movies: Vec<Movie> = p1.results.into_iter().map(Movie::from).collect();
        if p1.total_pages > 1 && movies.len() < 20 {
            if let Ok(p2) = tmdb.search(t, 2) {
                movies.extend(p2.results.into_iter().map(Movie::from));
            }
        }
        let movies = if eff.is_empty() {
            dedupe(movies)
        } else {
            filter_movies(dedupe(movies), tax, eff)
        };
        if movies.is_empty() && eff.is_empty() && looks_like_question(t) {
            // Solo per una frase che sembra una domanda: una vera ricerca per
            // titolo non viene sostituita dai film del momento.
            notes.push(
                "La frase sembra una domanda, non un titolo: propongo i film del momento.".into(),
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
    if eff.is_empty() && eff.sort == Sort::default() {
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

/// Selettore Chat/Griglia nell'header: due segmenti, quello attivo acceso.
fn mode_switch(ui: &mut egui::Ui, mode: ViewMode) -> Option<ViewMode> {
    let mut clicked = None;
    egui::Frame::none()
        .fill(theme::alpha(theme::PANEL, 190))
        .stroke(egui::Stroke::new(1.0_f32, theme::BORDER))
        .rounding(egui::Rounding::same(12.0))
        .inner_margin(egui::Margin::same(3.0))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            ui.horizontal(|ui| {
                for (m, label, key) in [
                    (ViewMode::Chat, "Chat", "1"),
                    (ViewMode::Griglia, "Griglia", "2"),
                ] {
                    let active = mode == m;
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(66.0, 24.0), egui::Sense::click());
                    let hover = ui.ctx().animate_bool_with_time(
                        resp.id,
                        resp.hovered() && !active,
                        0.12,
                    );
                    let painter = ui.painter();
                    if active {
                        painter.rect_filled(
                            rect,
                            egui::Rounding::same(8.0),
                            theme::alpha(theme::ACCENT_DIM, 225),
                        );
                        painter.rect_stroke(
                            rect,
                            egui::Rounding::same(8.0),
                            egui::Stroke::new(1.0_f32, theme::alpha(theme::ACCENT, 150)),
                        );
                    } else if hover > 0.01 {
                        painter.rect_filled(
                            rect,
                            egui::Rounding::same(8.0),
                            theme::mix(theme::CARD, theme::CARD_HOVER, hover),
                        );
                    }
                    painter.text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        label,
                        egui::FontId::proportional(if active { 12.0 } else { 11.5 }),
                        if active {
                            theme::TEXT
                        } else {
                            theme::mix(theme::DIM, theme::TEXT, hover)
                        },
                    );
                    theme::focus_ring(ui, &resp, 8.0);
                    let response = resp
                        .on_hover_cursor(egui::CursorIcon::PointingHand)
                        .on_hover_text(format!("Vai a {label} (Ctrl+{key})"));
                    if response.clicked() && !active {
                        clicked = Some(m);
                    }
                }
            });
        });
    clicked
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
    let Some(dir) = resize_edge(ctx.screen_rect(), pos) else {
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
            .min_size(egui::vec2(cap, 24.0)),
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    if hover > 0.01 {
        let fill = match icon {
            CaptionIcon::Close => theme::alpha(theme::DANGER, (210.0 * hover) as u8),
            _ => theme::alpha(theme::ACCENT, (28.0 * hover) as u8),
        };
        ui.painter()
            .rect_filled(resp.rect.shrink(1.0), egui::Rounding::same(4.0), fill);
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
                egui::Rounding::same(0.5),
                stroke,
            );
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
            p.rect_stroke(front, egui::Rounding::same(0.5), stroke);
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

/// A finestra stretta il pulsante testuale «Cerca» diventa una lente da 26 px:
/// il campo resta, il comando pure, e i controlli finestra entrano nella riga.
fn compact_search(ui: &mut egui::Ui) -> egui::Response {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let ink = theme::mix(theme::ACCENT_3, theme::TEXT, hover);
    let p = ui.painter();
    p.rect_filled(
        rect,
        egui::Rounding::same(8.0),
        theme::mix(theme::CARD, theme::ACCENT_DIM, hover * 0.7),
    );
    p.rect_stroke(
        rect,
        egui::Rounding::same(8.0),
        egui::Stroke::new(
            1.0_f32,
            theme::mix(theme::BORDER, theme::ACCENT, hover * 0.6),
        ),
    );
    let c = rect.center();
    p.circle_stroke(
        c + egui::vec2(-1.5, -1.5),
        4.6,
        egui::Stroke::new(1.5_f32, ink),
    );
    p.line_segment(
        [c + egui::vec2(1.7, 1.7), c + egui::vec2(5.0, 5.0)],
        egui::Stroke::new(1.5_f32, ink),
    );
    theme::focus_ring(ui, &resp, 8.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text("Cerca")
}

fn allowed_url(url: &str) -> bool {
    let trimmed = url.trim();
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
        allowed_url, describe, has_word, looks_like_question, merge_facets, resize_edge,
        side_panel_widths, Facets,
    };
    use crate::domain::taxonomy::Taxonomy;
    use egui::viewport::ResizeDirection as Dir;

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
        merge_facets(&mut base, &extra);
        assert_eq!(base.countries, ["IT"]);
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
    fn memoria_riconosce_visto_scarto_e_questo_film() {
        let (kind, target) = parse_memory_command("ho già visto Il Padrino").unwrap();
        assert_eq!(kind, MemoryKind::Seen);
        assert_eq!(target.as_deref(), Some("Il Padrino"));

        let (kind, target) = parse_memory_command("scarta questo film").unwrap();
        assert_eq!(kind, MemoryKind::Rejected);
        assert!(target.is_none());
        assert!(is_generic_target("la commedia"));
    }

    #[test]
    fn la_memoria_esclude_id_e_titoli() {
        let mut memory = FilmMemory::default();
        memory.mark(true, Some(1), "Il Padrino");
        let same_id = Movie {
            id: 1,
            title: "Altro titolo".into(),
            ..Default::default()
        };
        let same_title = Movie {
            id: 2,
            title: "Il Padrino".into(),
            ..Default::default()
        };
        let snap = memory.snapshot();
        assert!(snap.contains(&same_id));
        assert!(snap.contains(&same_title));

        memory.mark(false, None, "Il Padrino");
        assert!(memory.seen.is_empty());
        assert_eq!(memory.rejected.len(), 1);
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
        assert_eq!(
            find_card_by_title(&entries, "padrino").map(|c| c.movie.id),
            Some(1)
        );
        let mentioned = cards_mentioned(&entries, "confronta Il Padrino e Quei bravi ragazzi");
        assert_eq!(mentioned.len(), 2);
    }
}
