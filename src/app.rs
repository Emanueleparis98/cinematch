use crate::domain::constraints::{self, Prune};
use crate::domain::model::{Details, Facets, Movie, Sort, TrackState};
use crate::domain::taxonomy::Taxonomy;
use crate::infra::config::Config;
use crate::integrations::simkl;
use crate::tmdb::client::{describe_credential, Tmdb};
use crate::ui::facets::{FacetAction, Section};
use crate::ui::images::ImageStore;
use crate::ui::{detail, facets, fx, grid, lumen, theme, unlock};
use crate::update;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};
const MAX_QUERY_CHARS: usize = 300;
const FILTERS_WIDTH: f32 = 360.0;
const RESIZE_EDGE: f32 = 6.0;

struct Results {
    total_is_exact: bool,
    facets: Facets,
    removed: Vec<String>,
    info: Vec<String>,
    notes: Vec<String>,
    movies: Vec<Movie>,
    total: u32,
    total_pages: u32,
    headline: String,
    error: Option<String>,
    keyword_pairs: Vec<(String, u64)>,
}
#[allow(clippy::large_enum_variant)]
enum Msg {
    Results {
        gen: u64,
        payload: Box<Results>,
    },
    Unlocks {
        gen: u64,
        unlocks: Vec<unlock::Unlock>,
    },
    Details {
        gen: u64,
        res: Result<Details, String>,
    },
    Check {
        key: String,
        res: Result<String, String>,
    },
    OnboardingChecked {
        key: String,
        res: Result<String, String>,
    },
    Imdb {
        res: Result<String, String>,
    },
    ImdbProgress {
        done: u64,
    },
    UpdateChecked(Result<Option<update::Manifest>, String>),
    UpdateProgress {
        done: u64,
        total: u64,
    },
    UpdateDownloaded(Result<PathBuf, String>),
    SimklCode {
        gen: u64,
        res: Result<simkl::DeviceCode, String>,
    },
    SimklAuth {
        gen: u64,
        res: Result<simkl::Tokens, String>,
    },
    SimklSynced {
        gen: u64,
        res: Result<(simkl::Store, simkl::Tokens, simkl::SyncReport), String>,
    },
    SimklWritten {
        gen: u64,
        res: Result<(simkl::Tokens, simkl::WriteOutcome), String>,
    },
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum ViewMode {
    Griglia,
}
#[derive(Clone, PartialEq)]
struct SettingsDraft {
    tmdb_key: String,
    language: String,
    simkl_client_id: String,
    simkl_hide_watched: bool,
}
impl SettingsDraft {
    fn from(cfg: &Config) -> Self {
        Self {
            tmdb_key: cfg.tmdb_api_key.clone(),
            language: cfg.language.clone(),
            simkl_client_id: cfg.simkl.client_id.clone(),
            simkl_hide_watched: cfg.simkl.hide_watched,
        }
    }
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
pub struct App {
    search_cancel: Option<Arc<AtomicBool>>,
    total_is_exact: bool,
    ctx: egui::Context,
    tax: Arc<Taxonomy>,
    tmdb: Option<Arc<Tmdb>>,
    cfg: Config,
    rx: Receiver<Msg>,
    tx: Sender<Msg>,
    gen: u64,
    images: ImageStore,

    mode: ViewMode,
    /// Dopo un invio la vista torna in fondo, anche se si era scrollato su.

    /// Quattro spunti locali della casa, rigenerati dai filtri attivi.
    /// Catalogo gia' validato per lo stato corrente dei filtri.
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
    notes: Vec<String>,

    movies: Vec<Movie>,
    total: u32,
    total_pages: u32,
    page: u32,
    /// La prossima griglia parte dall'alto: nuova ricerca, filtri o pagina.
    grid_reset_scroll: bool,
    loading: bool,
    headline: String,
    /// Ultimo errore della ricerca: distingue TMDB che non risponde da
    /// zero risultati veri.
    results_error: Option<String>,
    /// Ultimo esito di rete: None = non ancora provato, Some(false) = offline.
    net: Option<bool>,
    /// Compromessi «Sblocca» per la ricerca corrente, se la griglia è quasi vuota.
    unlocks: Vec<unlock::Unlock>,

    detail: Option<Details>,
    detail_loading: bool,
    /// oppure dai criteri attivi della griglia.
    detail_why: Option<(String, String)>,
    /// Generazioni delle richieste scheda: una risposta vecchia non
    /// deve sovrascrivere il film aperto dopo.
    detail_gen: u64,

    keyword_map: Vec<(String, u64)>,
    dirty: Option<Instant>,
    focus: Option<ViewMode>,
    show_settings: bool,
    settings: SettingsDraft,
    settings_check: Option<String>,
    /// Chiusura delle Impostazioni con modifiche non salvate.
    settings_close_confirm: bool,
    settings_after_close: Option<ViewMode>,
    /// Primo avvio: chiediamo la chiave TMDB prima delle impostazioni.
    first_run: bool,
    onboarding_busy: bool,
    /// Collegamento SIMKL: token (nel Credential Manager), libreria locale,
    /// codice di autorizzazione in corso e stato dell'ultima sincronizzazione.
    simkl_tokens: Option<simkl::Tokens>,
    simkl_store: simkl::Store,
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
    /// Errore di scrittura legato a un film: la scheda lo mostra accanto
    simkl_write_error: Option<(u64, String)>,
    update: UpdateUi,
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
        Self::with_local_data(
            ctx,
            cfg,
            simkl::load_tokens().ok().flatten(),
            simkl::load_store(),
        )
    }
    fn with_local_data(
        ctx: &egui::Context,
        cfg: Config,
        simkl_tokens: Option<simkl::Tokens>,
        simkl_store: simkl::Store,
    ) -> Self {
        let tax = Arc::new(Taxonomy::load());
        let tmdb = Self::build_tmdb(&cfg);
        let (tx, rx) = channel();
        let facets = Facets::default();
        let prune = constraints::pruning(&tax, &facets);
        let settings = SettingsDraft::from(&cfg);
        let first_run = !cfg.has_key();
        let mut app = Self {
            search_cancel: None,
            total_is_exact: true,
            ctx: ctx.clone(),
            tax,
            tmdb,
            cfg,
            rx,
            tx,
            gen: 0,
            images: ImageStore::new(),
            mode: ViewMode::Griglia,
            grid_query: String::new(),
            facet_query: String::new(),
            guide_open: false,
            filters_open: true,
            facets,
            prune,
            messages: Vec::new(),
            info: Vec::new(),
            notes: Vec::new(),
            movies: Vec::new(),
            total: 0,
            total_pages: 0,
            page: 1,
            grid_reset_scroll: false,
            loading: false,
            headline: String::new(),
            results_error: None,
            net: None,
            unlocks: Vec::new(),
            detail: None,
            detail_loading: false,
            detail_why: None,
            detail_gen: 0,
            keyword_map: Vec::new(),
            dirty: None,
            focus: None,
            show_settings: false,
            settings,
            settings_check: None,
            settings_close_confirm: false,
            settings_after_close: None,
            first_run,
            onboarding_busy: false,
            simkl_tokens,
            simkl_store,
            simkl_auth: None,
            simkl_cancel: Arc::new(AtomicBool::new(false)),
            simkl_epoch: 0,
            simkl_syncing: false,
            simkl_busy: false,
            simkl_pending: None,
            simkl_status: None,
            simkl_error: None,
            simkl_write_error: None,
            update: UpdateUi::default(),
            toast: None,
            boot_done: false,
            imdb_busy: false,
            imdb_progress: None,
            imdb_status: None,
            cjk_loaded: false,
        };
        // La libreria SIMKL si aggiorna all'avvio, ma non più di una volta
        // ogni 15 minuti: niente polling in background.
        if app.simkl_tokens.is_some() && simkl::sync_due(app.simkl_store.last_sync) {
            app.start_simkl_sync(false);
        }
        app
    }
    fn switch_mode(&mut self, mode: ViewMode) {
        let from_settings = self.show_settings;
        self.mode = mode;
        self.show_settings = false;
        if from_settings {
            self.focus = Some(mode);
        }
    }
    fn spawn_search(&mut self) {
        self.dirty = None;
        let Some(tmdb) = self.tmdb.clone() else {
            self.loading = false;
            self.first_run = true;
            return;
        };
        if let Some(cancel) = self.search_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        let (tmdb, cancel) = tmdb.cancellable();
        self.search_cancel = Some(cancel);
        self.gen += 1;
        let gen = self.gen;
        self.loading = true;
        self.results_error = None;
        self.grid_reset_scroll = true;
        self.unlocks.clear();
        let text = self
            .grid_query
            .trim()
            .chars()
            .take(MAX_QUERY_CHARS)
            .collect::<String>();
        let (eff, removed) = constraints::sanitize(&self.tax, &self.facets);
        let mut kw = self.keyword_map.clone();
        let tax = self.tax.clone();
        let page = self.page;
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let mut payload = Results {
                total_is_exact: true,
                facets: eff.clone(),
                removed,
                info: Vec::new(),
                notes: Vec::new(),
                movies: Vec::new(),
                total: 0,
                total_pages: 0,
                headline: String::new(),
                error: None,
                keyword_pairs: Vec::new(),
            };
            resolve_keywords(
                &tmdb,
                &tax,
                &eff,
                &mut kw,
                &mut payload.keyword_pairs,
                &mut payload.info,
            );
            let outcome = (|| {
                require_keywords(&tax, &eff, &kw)?;
                if !text.is_empty() {
                    if eff.is_empty() && eff.sort == Sort::Popularity {
                        let p = tmdb.search(&text, page)?;
                        return Ok((
                            p.results.into_iter().map(Movie::from).collect(),
                            p.total_results,
                            p.total_pages,
                            format!("Risultati per «{text}»"),
                        ));
                    }
                    let upstream = tmdb.search(&text, page)?;
                    let raw = upstream.results.into_iter().map(Movie::from).collect();
                    let details =
                        verified_details(&tmdb, &tax, &eff, &kw, &filter_movies(raw, &tax, &eff))?;
                    let mut movies = details.into_values().map(|d| d.movie).collect::<Vec<_>>();
                    sort_movies(&mut movies, eff.sort);
                    payload.total_is_exact = false;
                    payload.info.push("Ricerca per titolo: filtri e ordinamento applicati alla pagina corrente. Le pagine si riferiscono alla ricerca TMDB completa.".into());
                    return Ok((
                        movies,
                        upstream.total_results,
                        upstream.total_pages,
                        format!("Risultati per «{text}»"),
                    ));
                }
                if wants_trending(&eff) {
                    let p = tmdb.trending(page)?;
                    return Ok((
                        p.results.into_iter().map(Movie::from).collect(),
                        p.total_results,
                        p.total_pages,
                        "Tendenze della settimana".into(),
                    ));
                }
                payload.total_is_exact = !constraints::decade_gap(&tax, &eff)
                    && eff.runtime.is_none()
                    && eff.sort != Sort::Newest;
                discover_strict(&tmdb, &tax, &eff, &kw, page, &mut payload.info)
            })();
            match outcome {
                Ok((movies, total, pages, headline)) => {
                    payload.movies = movies;
                    payload.total = total;
                    payload.total_pages = pages.min(500);
                    payload.headline = headline;
                }
                Err(e) => payload.error = Some(e),
            }
            let unlockable = text.is_empty()
                && page == 1
                && payload.error.is_none()
                && payload.movies.len() <= 3
                && payload.removed.is_empty();
            let seen = payload.movies.iter().map(|m| m.id).collect::<HashSet<_>>();
            let current_total = payload.total;
            let _ = tx.send(Msg::Results {
                gen,
                payload: Box::new(payload),
            });
            ctx.request_repaint();
            if unlockable {
                let unlocks = unlock_options(&tmdb, &tax, &eff, &kw, current_total, &seen);
                if !unlocks.is_empty() {
                    let _ = tx.send(Msg::Unlocks { gen, unlocks });
                    ctx.request_repaint();
                }
            }
        });
        self.ctx.request_repaint();
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
                            p.total_is_exact = false;
                            p.info.push(format!("{hidden} film già visti nascosti."));
                        }
                    }
                    self.facets = p.facets;
                    self.total_is_exact = p.total_is_exact;
                    self.prune = constraints::pruning(&self.tax, &self.facets);
                    self.messages = p.removed;
                    self.info = p.info;
                    self.notes = p.notes;
                    for pair in p.keyword_pairs {
                        if !self.keyword_map.iter().any(|(k, _)| k == &pair.0) {
                            self.keyword_map.push(pair);
                        }
                    }
                    self.headline = p.headline;
                    self.net = Some(net_ok);
                    if let Some(err) = p.error {
                        self.movies.clear();
                        self.total = 0;
                        self.total_pages = 0;
                        self.results_error = Some(err);
                    } else {
                        self.results_error = None;
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
                Msg::Check { key, res } => {
                    let net_ok = res.is_ok();
                    let Some(status) =
                        verified_status(&key.as_str(), &self.settings.tmdb_key.trim(), res)
                    else {
                        continue;
                    };
                    if net_ok {
                        self.net = Some(true);
                    }
                    self.settings_check = Some(status);
                }
                Msg::OnboardingChecked { key, res } => {
                    self.onboarding_busy = false;
                    match res {
                        Ok(msg) => {
                            if self.settings.tmdb_key.trim() != key {
                                self.settings_check =
                                    Some("✕ La chiave è cambiata: verifica di nuovo.".into());
                                continue;
                            }
                            self.settings_check = Some(format!("✔ {msg}"));
                            if self.save_settings() {
                                self.first_run = false;
                                self.show_settings = false;
                                self.mode = ViewMode::Griglia;
                                self.focus = Some(ViewMode::Griglia);
                                self.toast("Chiave salvata. Esplora il catalogo con i filtri.");
                            } else {
                                self.settings_check =
                                    Some("✕ Non sono riuscito a salvare la chiave.".into());
                            }
                        }
                        Err(e) => self.settings_check = Some(format!("✕ {e}")),
                    }
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
                            self.simkl_write_error = None;
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
                            self.simkl_write_error = pending.map(|(id, _)| (id, e.clone()));
                            self.simkl_pending = pending;
                            self.simkl_error = Some(e);
                        }
                    }
                }
            }
        }
    }
    fn tick(&mut self) {
        if let Some(at) = self.dirty {
            if at.elapsed() >= Duration::from_millis(320) {
                // mano), quindi il ritardo resta breve anche con un modello attivo.
                self.dirty = None;
                self.page = 1;
                self.launch_search();
            } else {
                self.ctx.request_repaint_after(Duration::from_millis(70));
            }
        }
    }

    fn schedule_search(&mut self) {
        self.gen = self.gen.wrapping_add(1);
        if let Some(cancel) = self.search_cancel.take() {
            cancel.store(true, Ordering::Relaxed);
        }
        self.loading = true;
        self.results_error = None;
        self.unlocks.clear();
        self.dirty = Some(Instant::now());
    }
    fn toast(&mut self, msg: impl Into<String>) {
        self.toast = Some((msg.into(), Instant::now()));
    }
    fn ensure_fonts_for_results(&mut self) {
        let texts: Vec<&str> = self
            .movies
            .iter()
            .flat_map(|m| [m.title.as_str(), m.original_title.as_str()])
            .collect();
        crate::ui::fonts::ensure_cjk(&self.ctx, &mut self.cjk_loaded, &texts);
    }
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
        self.simkl_write_error = None;
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
        // Scollegare non deve distruggere la cronologia locale: la copia
        // resta e serve a ricollegare senza perdere visti e voti.

        self.simkl_status = Some("SIMKL scollegato: la copia locale resta sul PC.".into());
    }
    fn apply_track_flags(&mut self) {
        let store = &self.simkl_store;
        let flag = |id: u64| track_state(store, id);
        for movie in &mut self.movies {
            movie.track = flag(movie.id);
        }
        if self.cfg.simkl.hide_watched {
            let before = self.movies.len();
            self.movies
                .retain(|m| !store.get(m.id).is_some_and(|s| s.status == "completed"));
            if self.movies.len() != before {
                self.total_is_exact = false;
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
                let entry =
                    self.simkl_store
                        .movies
                        .entry(key)
                        .or_insert_with(|| simkl::TrackedMovie {
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
                        });
                entry.status = "plantowatch".into();
            }
            simkl::WriteOp::Watched => {
                let entry =
                    self.simkl_store
                        .movies
                        .entry(key)
                        .or_insert_with(|| simkl::TrackedMovie {
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
                        });
                entry.status = outcome.status.clone().unwrap_or_else(|| "completed".into());
            }
            simkl::WriteOp::Rate(value) => {
                let entry =
                    self.simkl_store
                        .movies
                        .entry(key)
                        .or_insert_with(|| simkl::TrackedMovie {
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
                error: self
                    .simkl_write_error
                    .as_ref()
                    .filter(|(id, _)| *id == movie.id)
                    .map(|(_, e)| e.clone()),
            },
            None => detail::TrackInfo {
                url: Some(simkl::search_url(&movie.title)),
                connected: self.simkl_tokens.is_some(),
                busy: self.simkl_busy || self.simkl_syncing,
                error: self
                    .simkl_write_error
                    .as_ref()
                    .filter(|(id, _)| *id == movie.id)
                    .map(|(_, e)| e.clone()),
                ..Default::default()
            },
        }
    }
    fn apply_detail_actions(
        &mut self,
        actions: Vec<detail::DetailAction>,
        current_id: u64,
        title: &str,
    ) {
        for action in actions {
            match action {
                detail::DetailAction::Open(id) => {
                    if id == current_id {
                        if self.detail.is_some() {
                            self.page = 1;
                            self.grid_query = title.to_string();
                            self.facets = Facets {
                                sort: self.facets.sort,
                                ..Default::default()
                            };
                            self.facet_query.clear();
                            self.resanitize();
                            self.launch_search();
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
                detail::DetailAction::RetryWrite => {
                    if let Some((id, op)) = self.simkl_pending {
                        if id == current_id {
                            self.simkl_write_current(op);
                        }
                    }
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
    fn launch_search(&mut self) {
        self.spawn_search();
    }
    fn load_trending(&mut self) {
        if self.movies.is_empty() && !self.loading {
            self.page = 1;
            self.launch_search();
        }
    }
    fn close_detail(&mut self) {
        if let Some(d) = &self.detail {
            let open = egui::Id::unique(("rating-open", d.movie.id));
            let draft = egui::Id::unique(("rating-draft", d.movie.id));
            self.ctx.data_mut(|data| {
                data.remove_temp::<bool>(open);
                data.remove_temp::<u8>(draft);
            });
        }
        self.detail = None;
        self.detail_loading = false;
        self.detail_gen = self.detail_gen.wrapping_add(1);
    }
    fn toggle_filters(&mut self) {
        if self.filters_open {
            self.filters_open = false;
        } else {
            self.close_detail();
            self.filters_open = true;
        }
    }
    fn detail_context(&self, _id: u64) -> Option<(String, String)> {
        (!self.facets.is_empty()).then(|| {
            (
                describe(
                    &self.tax,
                    &self.facets,
                    constraints::year_range(&self.tax, &self.facets)
                        .ok()
                        .flatten(),
                ),
                String::new(),
            )
        })
    }
    fn open_detail(&mut self, id: u64) {
        let Some(tmdb) = self.tmdb.clone() else {
            return;
        };
        self.detail_why = self.detail_context(id);

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
            if matches!(action, FacetAction::Clear(Section::All)) {
                self.grid_query.clear();
                self.facet_query.clear();
                self.notes.clear();
            }
            facets::apply_action(&mut self.facets, action);
        }

        self.resanitize();
        if !self.tmdb.is_some() {
            return;
        }
        self.schedule_search();
    }
    fn apply_unlock(&mut self, index: usize) {
        let Some(u) = self.unlocks.get(index).cloned() else {
            return;
        };
        self.facets = u.facets;
        self.unlocks.clear();
        self.page = 1;
        self.resanitize();
        if self.tmdb.is_some() {
            self.launch_search();
        }
    }
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
            let tmdb = Tmdb::new(key.clone(), lang);
            let res = tmdb.ping();
            let _ = tx.send(Msg::Check { key, res });
            ctx.request_repaint();
        });
    }
    fn start_onboarding(&mut self) {
        if self.onboarding_busy {
            return;
        }
        let key = self.settings.tmdb_key.trim().to_string();
        if key.is_empty() {
            self.settings_check = Some("✕ Incolla la chiave TMDB.".to_string());
            return;
        }
        self.onboarding_busy = true;
        self.settings_check = Some("Verifica in corso…".to_string());
        let lang = self.settings.language.clone();
        let tx = self.tx.clone();
        let ctx = self.ctx.clone();
        std::thread::spawn(move || {
            let tmdb = Tmdb::new(key.clone(), lang);
            let res = tmdb.ping();
            let _ = tx.send(Msg::OnboardingChecked { key, res });
            ctx.request_repaint();
        });
    }
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
    fn save_settings(&mut self) -> bool {
        let cfg = Config {
            tmdb_api_key: self.settings.tmdb_key.trim().to_string(),
            language: self.settings.language.clone(),
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
                self.keyword_map.clear();
                self.toast("Impostazioni salvate.");
                if self.tmdb.is_some() {
                    self.page = 1;
                    self.launch_search();
                }
                true
            }
            Err(e) => {
                self.toast(e);
                false
            }
        }
    }
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
                    ui.ctx()
                        .send_viewport_cmd(egui::viewport::ViewportCommand::Maximized(!max));
                }

                ui.allocate_ui_with_layout(
                    r.size(),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.add_space(20.0);
                        ui.label(lumen::display(22.0, "CINE"));
                        ui.label(lumen::display_color(22.0, "MATCH", lumen::EMBER));
                        ui.add_space(18.0);
                        let title = if self.first_run {
                            "PRIMO AVVIO"
                        } else if self.show_settings {
                            "IMPOSTAZIONI"
                        } else {
                            "CATALOGO"
                        };
                        ui.label(lumen::mono(9.0, title).color(lumen::IVORY));

                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            let cap = 38.0;
                            if window_caption(ui, CaptionIcon::Close, "Chiudi (Alt+F4)", cap)
                                .clicked()
                            {
                                ui.ctx()
                                    .send_viewport_cmd(egui::viewport::ViewportCommand::Close);
                            }
                            let max = ui.ctx().input(|i| i.viewport().maximized.unwrap_or(false));
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
                        });
                    },
                );

                ui.painter().line_segment(
                    [r.left_bottom(), r.right_bottom()],
                    egui::Stroke::new(1.0_f32, lumen::BORDER),
                );
            });
    }
    fn rail(&mut self, ui: &mut egui::Ui) {
        let mut switch = None;
        let mut settings = false;
        egui::Panel::left("rail")
            .exact_size(lumen::RAIL_WIDTH)
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
                    lumen::Icon::Grid,
                    "Catalogo",
                    None,
                    self.mode == ViewMode::Griglia && !self.show_settings,
                )
                .on_hover_text("Esplora il catalogo con filtri e correnti")
                .clicked()
                {
                    switch = Some(ViewMode::Griglia);
                }
                let settings_resp = lumen::nav_item(
                    ui,
                    lumen::Icon::Settings,
                    "Impostazioni",
                    None,
                    self.show_settings || self.first_run,
                );
                if self.update.available.is_some() {
                    let dot = settings_resp.rect.right_top() + egui::vec2(-10.0, 10.0);
                    ui.painter().circle_filled(dot, 3.0, lumen::EMBER);
                }
                if settings_resp.clicked() {
                    settings = true;
                }
                ui.with_layout(egui::Layout::bottom_up(egui::Align::LEFT), |ui| {
                    ui.add_space(14.0);
                    ui.horizontal(|ui| {
                        ui.add_space(12.0);
                        ui.label(
                            lumen::mono(9.0, format!("v{}", update::current_version()))
                                .color(lumen::FAINT),
                        );
                    });
                });
            });
        if let Some(mode) = switch {
            if self.show_settings {
                self.request_close_settings(Some(mode));
            } else {
                self.switch_mode(mode);
            }
        }
        if settings {
            if self.show_settings {
                self.request_close_settings(None);
            } else {
                self.show_settings = true;
            }
        }
    }
    fn grid_toolbar(&mut self, ui: &mut egui::Ui) {
        if self.mode != ViewMode::Griglia || self.show_settings || self.first_run {
            return;
        }
        let compact = ui.ctx().content_rect().width() < 1150.0;
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
                    if !compact {
                        ui.add_space(8.0);
                        let count = if self.loading {
                            "CERCO…".to_string()
                        } else {
                            if self.total_is_exact {
                                format!("{} FILM", theme::thousands(self.total))
                            } else {
                                format!("{} IN PAGINA", self.movies.len())
                            }
                        };
                        ui.label(lumen::mono(9.0, count).color(lumen::FAINT));
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.add_space(14.0);
                        if lumen::icon_button(
                            ui,
                            lumen::Icon::Refresh,
                            34.0,
                            "Aggiorna il catalogo",
                        )
                        .clicked()
                        {
                            refresh = true;
                        }
                        egui::ComboBox::from_id_salt("grid-sort")
                            .selected_text(egui::RichText::new(self.facets.sort.label()).size(11.0))
                            .width(if compact { 120.0 } else { 150.0 })
                            .icon(|ui, rect, _visuals, _open| {
                                let c = rect.center();
                                let stroke = egui::Stroke::new(1.2_f32, lumen::MUTED);
                                ui.painter()
                                    .line_segment([c + egui::vec2(-4.0, -2.0), c], stroke);
                                ui.painter()
                                    .line_segment([c + egui::vec2(4.0, -2.0), c], stroke);
                            })
                            .show_ui(ui, |ui| {
                                for s in Sort::ALL {
                                    ui.selectable_value(&mut self.facets.sort, s, s.label());
                                }
                            });
                        let active = self.facets.active_count();
                        if facets::panel_toggle(ui, self.filters_open, active).clicked() {
                            toggle_filters = true;
                        }
                        if lumen::button_sized(ui, "Cerca", true, 80.0).clicked() {
                            do_search = true;
                        }
                        let w = ui.available_width().clamp(140.0, 380.0);
                        let hint = "Cerca un titolo…";
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
                        if resp.changed() {
                            self.schedule_search();
                        }
                        if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                            do_search = true;
                        }
                    });
                });
            });

        if want_focus {
            self.focus = None;
        }
        if self.facets.sort != sort_before {
            self.page = 1;
            self.schedule_search();
        }
        if toggle_filters {
            self.toggle_filters();
        }
        if refresh {
            self.resanitize();
            self.page = 1;
            self.schedule_search();
        }
        if do_search {
            self.page = 1;
            self.launch_search();
        }
    }
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
                ui.allocate_ui_with_layout(
                    r.size(),
                    egui::Layout::left_to_right(egui::Align::Center),
                    |ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        ui.add_space(14.0);
                        let busy = self.loading
                            || self.detail_loading
                            || self.update.checking
                            || self.imdb_busy;
                        let (color, state) = if busy {
                            (lumen::EMBER, "LAVORO")
                        } else {
                            match self.net {
                                Some(false) => (lumen::DANGER, "ATTENZIONE"),
                                Some(true) => (lumen::CELADON, "PRONTO"),
                                None => (lumen::FAINT, "PRONTO"),
                            }
                        };
                        let (dot, _) =
                            ui.allocate_exact_size(egui::vec2(6.0, 6.0), egui::Sense::hover());
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
                        ui.label(lumen::mono(9.0, "FILTRI E CORRENTI").color(lumen::FAINT));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.add_space(14.0);
                            ui.label(
                                lumen::mono(9.0, format!("v{}", update::current_version()))
                                    .color(lumen::MUTED),
                            );
                            if self.update.available.is_some() {
                                ui.label(lumen::mono(9.0, "AGGIORNAMENTO").color(lumen::EMBER));
                            }
                        });
                    },
                );
            });
    }
    fn left_panel(&mut self, ui: &mut egui::Ui) {
        // tutto, ma lo stato (aperto, filtri, larghezza) resta intatto.
        if self.mode != ViewMode::Griglia
            || !self.filters_open
            || self.show_settings
            || self.first_run
        {
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
                            &self.notes,
                        ));
                    });
                if !actions.is_empty() {
                    self.apply_actions(actions);
                }
            });
    }
    fn right_panel(&mut self, ui: &mut egui::Ui) {
        if self.mode != ViewMode::Griglia || self.show_settings || self.first_run {
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
                            let (line, _) = ui.allocate_exact_size(
                                egui::vec2(w * part, 13.0),
                                egui::Sense::hover(),
                            );
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
                let (why, consiglio) = match &self.detail_why {
                    Some((p, c)) => (Some(p.as_str()), (!c.is_empty()).then_some(c.as_str())),
                    None => (None, None),
                };
                let actions =
                    detail::panel(ui, &self.images, &d, false, &track, why, consiglio, false);
                self.apply_detail_actions(actions, d.movie.id, &d.movie.title);
            });
    }
    fn central(&mut self, ui: &mut egui::Ui) {
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(egui::Color32::TRANSPARENT)
                    .inner_margin(egui::Margin::symmetric(14, 10)),
            )
            .show(ui, |ui| {
                if self.first_run {
                    self.onboarding_page(ui);
                    return;
                }
                if self.show_settings {
                    self.settings_page(ui);
                    return;
                }
                if self.tmdb.is_none() {
                    ui.add_space(40.0);
                    ui.vertical_centered(|ui| {
                        ui.label(lumen::display(24.0, "Serve la chiave API di TMDB"));
                        ui.add_space(6.0);
                        ui.label(
                            lumen::body(
                                12.5,
                                "Serve una credenziale gratuita di TMDB: la API Key v3 (32 caratteri)\noppure il Read Access Token v4 (inizia con eyJ). La riconosco da sola.",
                            )
                            .color(lumen::MUTED),
                        );
                        ui.add_space(10.0);
                        if theme::primary_button(
                            ui,
                            "Inserisci la chiave TMDB",
                            egui::vec2(210.0, 36.0),
                        )
                        .clicked()
                        {
                            self.first_run = true;
                            self.show_settings = false;
                        }
                        if lumen::link(ui, "themoviedb.org → API", lumen::EMBER).clicked() {
                            open_url("https://www.themoviedb.org/settings/api");
                        }
                    });
                    return;
                }

                self.grid_central(ui);
            });
    }
    fn grid_central(&mut self, ui: &mut egui::Ui) {
        if !self.loading && self.movies.len() <= 3 && !self.unlocks.is_empty() {
            if let Some(i) = unlock::strip(ui, &self.unlocks) {
                self.apply_unlock(i);
                return;
            }
            ui.add_space(12.0);
        }
        if self.movies.is_empty() && !self.loading {
            let mut retry = false;
            ui.add_space(30.0);
            ui.vertical_centered(|ui| {
                if let Some(err) = &self.results_error {
                    ui.label(lumen::display(20.0, "Ricerca non completata"));
                    ui.add_space(6.0);
                    ui.label(lumen::body(12.0, err).color(lumen::MUTED));
                    ui.add_space(10.0);
                    if lumen::link(ui, "Riprova", lumen::EMBER).clicked() {
                        retry = true;
                    }
                } else {
                    ui.label(
                        egui::RichText::new(if !self.total_is_exact && self.total_pages > 1 {
                            "Nessun risultato compatibile in questa pagina"
                        } else {
                            "Nessun risultato con questi vincoli"
                        })
                        .size(15.0)
                        .color(theme::DIM),
                    );
                    ui.label(
                        egui::RichText::new(if !self.total_is_exact && self.total_pages > 1 {
                            "Prova la pagina successiva oppure modifica i filtri."
                        } else {
                            "Modifica i filtri oppure scegli uno dei suggerimenti disponibili."
                        })
                        .size(12.0)
                        .color(theme::DIM),
                    );
                }
            });
            if retry {
                self.launch_search();
            }
            return;
        }
        let selected = self.detail.as_ref().map(|d| d.movie.id);
        let reset_scroll = std::mem::take(&mut self.grid_reset_scroll);
        if self.loading {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(lumen::body(12.0, "Aggiorno il catalogo…").color(lumen::MUTED));
            });
            ui.add_space(8.0);
        }
        let clicked = ui
            .add_enabled_ui(!self.loading, |ui| {
                grid::grid(ui, &self.images, &self.movies, selected, reset_scroll)
            })
            .inner;
        if let Some(id) = clicked {
            self.open_detail(id);
        }
    }
    fn grid_pager(&mut self, ui: &mut egui::Ui) {
        if self.first_run {
            return;
        }
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
                        lumen::mono(9.0, format!("PAGINA {page} DI {pages}")).color(lumen::FAINT),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(20.0);
                        if pager_button(ui, lumen::Icon::ArrowRight, next, "Pagina successiva")
                            .clicked()
                        {
                            target = Some((page + 1).min(pages));
                        }
                        ui.add_space(6.0);
                        if pager_button(ui, lumen::Icon::ArrowLeft, prev, "Pagina precedente")
                            .clicked()
                        {
                            target = Some(page.saturating_sub(1));
                        }
                    });
                });
            });
        if let Some(page) = target {
            self.page = page;
            self.spawn_search();
        }
    }
    fn onboarding_page(&mut self, ui: &mut egui::Ui) {
        let mut verify = false;
        let mut skip = false;
        let mut open_api = false;
        egui::ScrollArea::vertical()
            .id_salt("onboarding-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                let width = ui.available_width().min(680.0);
                let pad = ((ui.available_width() - width) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(pad);
                    ui.allocate_ui_with_layout(
                        egui::vec2(width, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            ui.add_space(42.0);
                            ui.label(lumen::mono(10.0, "PRIMO AVVIO").color(lumen::EMBER));
                            ui.add_space(8.0);
                            ui.label(lumen::display(38.0, "Iniziamo da TMDB."));
                            ui.add_space(12.0);
                            ui.label(
                                lumen::body(
                                    13.0,
                                    "CineMatch legge i film da TMDB. Inserisci la credenziale per esplorare il catalogo con filtri e correnti. SIMKL è facoltativo.",
                                )
                                .color(lumen::MUTED),
                            );
                            ui.add_space(22.0);
                            ui.label(lumen::mono(9.5, "CHIAVE API TMDB").color(lumen::FAINT));
                            ui.add_space(6.0);
                            let resp = ui.add(
                                egui::TextEdit::singleline(&mut self.settings.tmdb_key)
                                    .password(true)
                                    .hint_text("API Key v3 (32 caratteri) oppure token v4 (eyJ…)")
                                    .desired_width(f32::INFINITY),
                            );
                            if resp.changed() {
                                self.settings_check = None;
                            }
                            theme::focus_ring(ui, &resp, 2.0);
                            let (hint, plausible) = describe_credential(&self.settings.tmdb_key);
                            let color = if self.settings.tmdb_key.trim().is_empty() {
                                theme::DIM
                            } else if plausible {
                                theme::OK
                            } else {
                                theme::WARN
                            };
                            ui.label(egui::RichText::new(hint).size(10.5).color(color));
                            if let Some(check) = &self.settings_check {
                                let color = if check.starts_with('✔') {
                                    theme::OK
                                } else if check.starts_with("Verifica") {
                                    theme::DIM
                                } else {
                                    theme::DANGER
                                };
                                ui.label(egui::RichText::new(check).size(11.0).color(color));
                            }
                            ui.add_space(14.0);
                            ui.horizontal(|ui| {
                                if lumen::button_sized(
                                    ui,
                                    "Verifica e inizia",
                                    !self.onboarding_busy,
                                    196.0,
                                )
                                .clicked()
                                {
                                    verify = true;
                                }
                                if self.onboarding_busy {
                                    fx::dot(ui, lumen::EMBER, 0.0);
                                }
                                if lumen::link(
                                    ui,
                                    "Ottieni una chiave gratuita ↗",
                                    lumen::EMBER,
                                )
                                .clicked()
                                {
                                    open_api = true;
                                }
                            });
                            ui.add_space(12.0);
                            ui.label(
                                lumen::body(
                                    10.5,
                                    "La credenziale viene salvata nel Windows Credential Manager e usata per accedere a TMDB.",
                                )
                                .color(lumen::FAINT),
                            );
                            ui.add_space(10.0);
                            if lumen::link(ui, "Lo faccio dopo", lumen::MUTED).clicked() {
                                skip = true;
                            }
                        },
                    );
                });
            });
        if verify {
            self.start_onboarding();
        }
        if open_api {
            open_url("https://www.themoviedb.org/settings/api");
        }
        if skip {
            self.first_run = false;
            self.show_settings = false;
            self.mode = ViewMode::Griglia;
        }
    }
    fn settings_dirty(&self) -> bool {
        self.settings != SettingsDraft::from(&self.cfg)
    }
    fn request_close_settings(&mut self, after: Option<ViewMode>) {
        if self.settings_dirty() {
            self.settings_close_confirm = true;
            self.settings_after_close = after;
        } else {
            self.show_settings = false;
            if let Some(mode) = after {
                self.switch_mode(mode);
            }
        }
    }
    fn settings_page(&mut self, ui: &mut egui::Ui) {
        let checked_key = self.settings.tmdb_key.clone();
        let mut close = false;
        let mut onboarding = false;
        let mut save = false;
        let mut check = false;
        let mut open_api_page = false;
        let mut update_check = false;
        let mut update_download = false;
        let mut update_install = false;
        let mut simkl_connect = false;
        let mut simkl_sync = false;
        let mut simkl_disconnect = false;
        let mut simkl_cancel = false;
        let dirty = self.settings_dirty();
        // Testata fissa: titolo, stato e comandi restano visibili mentre il
        // form scorre. Il corpo parte subito sotto l'hairline.
        let header_w = ui.available_width().min(760.0);
        let header_pad = ((ui.available_width() - header_w) * 0.5).max(0.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(header_pad);
            ui.allocate_ui_with_layout(
                egui::vec2(header_w, 0.0),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.spacing_mut().item_spacing.x = 8.0;
                    ui.set_width(header_w);
                    ui.add_space(18.0);
                    ui.horizontal(|ui| {
                        ui.vertical(|ui| {
                            ui.label(lumen::mono(10.0, "IMPOSTAZIONI").color(lumen::EMBER));
                            ui.add_space(5.0);
                            ui.label(lumen::display(30.0, "Impostazioni"));
                            if dirty {
                                ui.add_space(4.0);
                                ui.label(
                                    lumen::mono(9.0, "● MODIFICHE NON SALVATE")
                                        .color(lumen::EMBER),
                                );
                            }
                        });
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if lumen::link(ui, "Chiudi ×", lumen::MUTED).clicked() {
                                    close = true;
                                }
                                if dirty
                                    && lumen::button_sized(ui, "Salva", true, 96.0).clicked()
                                {
                                    save = true;
                                }
                            },
                        );
                    });
                    ui.add_space(4.0);
                    ui.label(
                        lumen::body(
                            11.5,
                            "Credenziali e preferenze restano su questo PC. SIMKL gestisce visti, lista e voti dell’account collegato.",
                        )
                        .color(lumen::MUTED),
                    );
                    ui.add_space(12.0);
                    lumen::hairline(ui);
                    ui.add_space(12.0);
                },
            );
        });
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
                            ui.set_width(width);
                            ui.spacing_mut().item_spacing.x = 8.0;
                            // I widget nativi ereditano la pelle LUMEN: fondo,
                            // bordi, raggi, selezione ember.
                            {
                                let v = ui.visuals_mut();
                                v.override_text_color = Some(lumen::IVORY);
                                v.widgets.noninteractive.bg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::BORDER);
                                v.widgets.noninteractive.fg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::MUTED);
                                v.widgets.inactive.bg_fill = lumen::DEEP;
                                v.widgets.inactive.weak_bg_fill = lumen::DEEP;
                                v.widgets.inactive.bg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::BORDER);
                                v.widgets.inactive.fg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::MUTED);
                                v.widgets.inactive.corner_radius =
                                    lumen::radius(lumen::RADIUS_SM);
                                v.widgets.hovered.bg_fill = lumen::RAISED;
                                v.widgets.hovered.weak_bg_fill = lumen::RAISED;
                                v.widgets.hovered.bg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::BORDER_HI);
                                v.widgets.hovered.fg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::IVORY);
                                v.widgets.hovered.corner_radius =
                                    lumen::radius(lumen::RADIUS_SM);
                                v.widgets.active.bg_fill = lumen::SURFACE;
                                v.widgets.active.weak_bg_fill = lumen::SURFACE;
                                v.widgets.active.bg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::EMBER);
                                v.widgets.active.fg_stroke =
                                    egui::Stroke::new(1.0_f32, lumen::IVORY);
                                v.widgets.active.corner_radius =
                                    lumen::radius(lumen::RADIUS_SM);
                                v.widgets.open = v.widgets.active;
                                v.selection.bg_fill = lumen::EMBER;
                                v.selection.stroke =
                                    egui::Stroke::new(0.0_f32, lumen::INK);
                                v.extreme_bg_color = lumen::DEEP;
                                v.faint_bg_color = lumen::SURFACE;
                                v.window_fill = lumen::SURFACE;
                                v.panel_fill = lumen::INK;
                            }
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
                        fx::dot(ui, lumen::EMBER, 0.0);
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
                    egui::RichText::new("Nascondi i film già visti nel catalogo")
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
                                    fx::dot(ui, lumen::EMBER, 0.0);
                                    ui.label(
                                        egui::RichText::new("Controllo…")
                                            .size(10.5)
                                            .color(theme::DIM),
                                    );
                                } else if ui
                                    .add_enabled_ui(self.update.progress.is_none(), |ui| {
                                        lumen::button_sized(ui, "Controlla", false, 104.0)
                                    })
                                    .inner
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
                    lumen::hairline(ui);
                }
                ui.label(lumen::mono(10.0, "TMDB").color(lumen::IVORY));
                ui.add_space(10.0);
                ui.label(
                    egui::RichText::new("Credenziale TMDB: API Key v3 (32 caratteri) oppure Read Access Token v4 (inizia con eyJ)")
                        .size(11.0)
                        .color(theme::DIM),
                );
                let tmdb_field = ui.add(
                    egui::TextEdit::singleline(&mut self.settings.tmdb_key)
                        .password(true)
                        .hint_text("API Key v3 (32 caratteri) o Read Access Token v4 (eyJ…)")
                        .desired_width(f32::INFINITY),
                );
                tmdb_field.widget_info(|| {
                    egui::WidgetInfo::labeled(
                        egui::Role::TextInput,
                        true,
                        "Chiave API TMDB",
                    )
                });
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
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 10.0;
                    if lumen::button_sized(ui, "Verifica credenziale", false, 160.0)
                        .clicked()
                    {
                        check = true;
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
                });
                if lumen::link(ui, "Rivedi il primo avvio", lumen::MUTED).clicked() {
                    onboarding = true;
                }

                ui.add_space(8.0);
                lumen::hairline(ui);
                ui.add_space(4.0);
                language_picker(ui, &mut self.settings.language);

                // ---- dati locali: voti IMDb ----
                ui.add_space(8.0);
                lumen::hairline(ui);
                ui.add_space(4.0);
                ui.label(lumen::mono(10.0, "DATI LOCALI").color(lumen::IVORY));
                ui.add_space(2.0);
                let ready = crate::infra::imdb::is_ready();
                ui.horizontal(|ui| {
                    let label = if ready {
                        "Aggiorna i voti IMDb"
                    } else {
                        "Scarica i voti IMDb"
                    };
                    let imdb_btn = ui
                        .add_enabled_ui(!self.imdb_busy, |ui| {
                            lumen::button_sized(ui, label, false, 200.0)
                        })
                        .inner;
                    if imdb_btn
                        .on_hover_text(
                            "Dataset IMDb per uso personale e non commerciale, salvato su questo PC. Mostra il voto IMDb accanto a quello TMDB.",
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
                        fx::dot(ui, lumen::EMBER, 0.0);
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
                    if ui.add_enabled_ui(dirty, |ui| {
                        lumen::button_sized(ui, "Salva", true, 100.0)
                    }).inner.clicked() {
                        save = true;
                    }
                });

                ui.add_space(8.0);
                lumen::hairline(ui);
                ui.label(
                    lumen::mono(
                        9.0,
                        "This product uses the TMDB API but is not endorsed or certified by TMDB.",
                    )
                    .color(lumen::MUTED),
                );
                ui.add_space(24.0);
                        },
                    );
                });
            });

        if checked_key != self.settings.tmdb_key {
            self.settings_check = None;
        }

        if open_api_page {
            open_url("https://www.themoviedb.org/settings/api");
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
            self.request_close_settings(None);
        }
        if onboarding {
            self.first_run = true;
            self.show_settings = false;
            self.mode = ViewMode::Griglia;
        }
    }
    fn settings_close_dialog(&mut self, ctx: &egui::Context) {
        if !self.settings_close_confirm {
            return;
        }
        let mut cancel = false;
        let mut discard = false;
        let mut save = false;
        let modal = egui::Modal::new(egui::Id::unique("settings-close-confirm"))
            .backdrop_color(lumen::alpha(lumen::INK, 190))
            .frame(
                egui::Frame::NONE
                    .fill(lumen::SURFACE)
                    .stroke(egui::Stroke::new(1.0_f32, lumen::BORDER_HI))
                    .corner_radius(lumen::radius(lumen::RADIUS))
                    .inner_margin(egui::Margin::same(18)),
            )
            .show(ctx, |ui| {
                ui.set_width(440.0);
                ui.label(lumen::display(22.0, "Modifiche non salvate"));
                ui.add_space(6.0);
                ui.label(
                    lumen::body(12.0, "Salva prima di uscire dalle Impostazioni?")
                        .color(lumen::MUTED),
                );
                ui.add_space(14.0);
                ui.horizontal_wrapped(|ui| {
                    if lumen::button_sized(ui, "Annulla", false, 96.0).clicked() {
                        cancel = true;
                    }
                    if lumen::button_sized(ui, "Esci senza salvare", false, 170.0).clicked() {
                        discard = true;
                    }
                    if lumen::button_sized(ui, "Salva", true, 96.0).clicked() {
                        save = true;
                    }
                });
            });
        if modal.should_close() {
            cancel = true;
        }
        if cancel {
            self.settings_close_confirm = false;
            self.settings_after_close = None;
        }
        if discard {
            self.settings_close_confirm = false;
            self.show_settings = false;
            if let Some(mode) = self.settings_after_close.take() {
                self.switch_mode(mode);
            }
        }
        if save {
            if self.save_settings() {
                self.settings_close_confirm = false;
                self.show_settings = false;
                if let Some(mode) = self.settings_after_close.take() {
                    self.switch_mode(mode);
                }
                self.toast("Impostazioni salvate.");
            } else {
                // L'errore è già nel toast: la pagina resta aperta.
                self.settings_close_confirm = false;
                self.settings_after_close = None;
            }
        }
    }
    fn toast_overlay(&mut self, ctx: &egui::Context) {
        let Some((msg, at)) = self.toast.clone() else {
            return;
        };
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
        let lift = 74.0;
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
                    .corner_radius(lumen::radius(lumen::RADIUS))
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
    fn update_ui(&mut self, ui: &mut egui::Ui) {
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
                self.first_run = true;
                self.show_settings = false;
            }
        }
        self.maybe_check_updates();
        self.header(ui);
        self.status_bar(ui);
        self.rail(ui);
        self.grid_toolbar(ui);
        self.left_panel(ui);
        self.right_panel(ui);
        self.grid_pager(ui);
        self.central(ui);
        self.settings_close_dialog(&ctx);
        if self.guide_open {
            let actions = facets::guide_window(&ctx, &self.tax, &mut self.guide_open);
            if !actions.is_empty() {
                self.apply_actions(actions);
            }
        }
        self.toast_overlay(&ctx);
        if self.loading
            || self.detail_loading
            || self.update.checking
            || self.update.progress.is_some()
        {
            ctx.request_repaint_after(Duration::from_millis(120));
        }
    }
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
                i.events.retain(|e| !matches!(e, egui::Event::Text(_)));
            });
        }
        if self.show_settings
            && !self.settings_close_confirm
            && self.settings_dirty()
            && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::S))
            && self.save_settings()
        {
            self.toast("Impostazioni salvate.");
        }
        if !self.show_settings
            && !self.first_run
            && !self.guide_open
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::K))
        {
            self.focus = Some(ViewMode::Griglia);
        }
        // Ctrl+B resta attivo anche con un widget a fuoco: e' il toggle
        if self.mode == ViewMode::Griglia
            && ctx.input(|i| i.modifiers.ctrl && i.key_pressed(egui::Key::B))
        {
            self.toggle_filters();
        }
        if ctx.input(|i| i.key_pressed(egui::Key::Escape)) {
            if self.guide_open {
                self.guide_open = false;
            } else if self.show_settings {
                if self.settings_close_confirm {
                    // Il dialogo gestisce Esc.
                } else if self.settings_dirty() {
                    self.settings_close_confirm = true;
                    self.settings_after_close = None;
                    ctx.input_mut(|i| {
                        i.consume_key(egui::Modifiers::NONE, egui::Key::Escape);
                    });
                } else {
                    self.show_settings = false;
                }
            } else if self.detail.is_some() || self.detail_loading {
                // Con il popover del voto aperto, Esc chiude prima quello.
                let rating_open = self
                    .detail
                    .as_ref()
                    .map(|d| {
                        let id = egui::Id::unique(("rating-open", d.movie.id));
                        ctx.data(|data| data.get_temp::<bool>(id).unwrap_or(false))
                    })
                    .unwrap_or(false);
                if !rating_open {
                    self.close_detail();
                }
            } else if self.filters_open {
                self.filters_open = false;
            } else if let Some(id) = ctx.memory(|m| m.focused()) {
                ctx.memory_mut(|m| m.surrender_focus(id));
            }
        }
    }
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.update_ui(ui);
    }
}
enum CaptionIcon {
    Minimize,
    Maximize,
    Restore,
    Close,
}

fn verified_status<T: PartialEq>(
    tested: &T,
    current: &T,
    res: Result<String, String>,
) -> Option<String> {
    if tested != current {
        return None;
    }
    Some(match res {
        Ok(m) => format!("✔ {m}"),
        Err(e) => format!("✕ {e}"),
    })
}

fn filter_movies(movies: Vec<Movie>, tax: &Taxonomy, f: &Facets) -> Vec<Movie> {
    if f.is_empty() && f.sort == Sort::Popularity {
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
    let min_rating = query.as_ref().and_then(|q| q.min_rating).or(f.min_rating);
    let max_rating = query.as_ref().and_then(|q| q.max_rating).or(f.max_rating);
    let min_votes = query.as_ref().and_then(|q| q.min_votes).or(f.min_votes);

    movies
        .into_iter()
        .filter(|m| {
            if !selected_decade(tax, f, m.year) {
                return false;
            }
            if let Some((a, b)) = year {
                match m.year {
                    Some(y) if y >= a && y <= b => {}
                    _ => return false,
                }
            }
            if let Some(q) = &query {
                let (from, to) =
                    crate::tmdb::client::date_bounds(q, &crate::infra::winclock::today_iso());
                if let Some(date) = m.release_date.as_deref() {
                    if from.as_deref().is_some_and(|v| date < v)
                        || to.as_deref().is_some_and(|v| date > v)
                    {
                        return false;
                    }
                } else if f.sort == Sort::Upcoming {
                    return false;
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

pub fn discover_strict(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    facets: &Facets,
    kw: &[(String, u64)],
    page: u32,
    notes: &mut Vec<String>,
) -> Result<(Vec<Movie>, u32, u32, String), String> {
    require_keywords(tax, facets, kw)?;
    let query = constraints::to_discover(tax, facets, kw, page)?;
    if facets.min_votes.is_none() {
        if let Some(min) = crate::tmdb::client::minimum_votes(&query) {
            notes.push(format!(
                "Per questo ordinamento o filtro sul voto: almeno {min} voti TMDB."
            ));
        }
    }
    if facets.sort == Sort::Newest && facets.runtime.is_none() {
        notes.push("Più recenti: solo film usciti e durata di almeno 60 minuti.".into());
    }
    if !facets.movements.is_empty() {
        notes.push("Le correnti applicano periodo, paesi, generi e parole chiave della guida: sono percorsi di esplorazione, non un censimento certificato delle opere.".into());
    }
    let result = tmdb.discover(&query)?;
    let mut movies: Vec<Movie> = result.results.into_iter().map(Movie::from).collect();
    if facets.runtime.is_some() || facets.sort == Sort::Newest {
        let verified = verified_details(tmdb, tax, facets, kw, &movies)?;
        movies.retain(|m| verified.contains_key(&m.id));
        notes.push("Durata verificata sulla scheda TMDB: le edizioni possono differire. Il totale TMDB precede questo controllo; il catalogo conta le corrispondenze nella pagina.".into());
    }
    if constraints::decade_gap(tax, facets) {
        movies.retain(|m| selected_decade(tax, facets, m.year));
        notes.push("Decenni separati: verificati sui singoli film; il totale TMDB puo includere gli anni intermedi.".into());
    }
    Ok((
        movies,
        result.total_results,
        result.total_pages,
        describe(tax, facets, query.year),
    ))
}

fn selected_decade(tax: &Taxonomy, facets: &Facets, year: Option<u16>) -> bool {
    facets.decades.is_empty()
        || year.is_some_and(|y| {
            facets
                .decades
                .iter()
                .filter_map(|id| tax.decade(id))
                .any(|d| y >= d.from && y <= d.to)
        })
}

fn require_keywords(
    tax: &Taxonomy,
    facets: &Facets,
    known: &[(String, u64)],
) -> Result<(), String> {
    let mut missing = constraints::unresolved_themes(facets, known);
    missing.extend(
        constraints::unresolved_keywords(tax, facets, known)
            .into_iter()
            .map(|(_, name)| name),
    );
    missing.extend(
        constraints::unresolved_movement_keywords(tax, facets, known)
            .into_iter()
            .map(|(_, name)| name),
    );
    if missing.is_empty() {
        Ok(())
    } else {
        Err(format!("Non posso verificare questi temi su TMDB: {}. Modifica o rimuovi il criterio per continuare.", missing.join(", ")))
    }
}

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
                "Il tema «{name}» non ha una parola chiave su TMDB: rimuovilo o modificalo per continuare."
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
                "La corrente richiede la parola chiave TMDB «{word}», che non è disponibile: rimuovi o modifica il criterio per continuare."
            )),
            Err(e) => notes.push(format!(
                "Parola chiave «{word}» della corrente non verificata: {e}"
            )),
        }
    }
}

fn wants_trending(f: &Facets) -> bool {
    f.is_empty() && f.sort == Sort::default()
}

fn sort_movies(movies: &mut [Movie], sort: Sort) {
    movies.sort_by(|a, b| {
        let order = match sort {
            Sort::Rating => b
                .rating
                .total_cmp(&a.rating)
                .then_with(|| b.votes.cmp(&a.votes)),
            Sort::Votes => b.votes.cmp(&a.votes),
            Sort::Newest => b.year.unwrap_or_default().cmp(&a.year.unwrap_or_default()),
            Sort::Oldest => a.year.unwrap_or(u16::MAX).cmp(&b.year.unwrap_or(u16::MAX)),
            Sort::Popularity | Sort::Upcoming => b.popularity.total_cmp(&a.popularity),
        };
        order.then_with(|| a.id.cmp(&b.id))
    });
}

fn verified_match(d: &Details, tax: &Taxonomy, facets: &Facets, known: &[(String, u64)]) -> bool {
    if require_keywords(tax, facets, known).is_err() {
        return false;
    }
    let mut local = facets.clone();
    local.countries.clear();
    if filter_movies(vec![d.movie.clone()], tax, &local).is_empty() {
        return false;
    }
    if (facets.min_rating.is_some() || facets.max_rating.is_some()) && d.movie.votes == 0 {
        return false;
    }
    let Ok(query) = constraints::to_discover(tax, facets, known, 1) else {
        return false;
    };
    if crate::tmdb::client::minimum_votes(&query).is_some_and(|min| d.movie.votes < min) {
        return false;
    }
    if facets.sort == Sort::Newest && query.runtime.is_none() && !d.runtime.is_some_and(|r| r >= 60)
    {
        return false;
    }
    if let Some((min, max)) = query.runtime {
        if !d
            .runtime
            .is_some_and(|minutes| minutes > 0 && minutes >= min && minutes <= max)
        {
            return false;
        }
    }
    if !query.countries.is_empty()
        && !query
            .countries
            .iter()
            .any(|code| d.origin_country_codes.contains(code))
    {
        return false;
    }
    if !query.keywords.is_empty() {
        let matches = |id: &u64| d.keywords.contains(id);
        if query.keywords_or {
            if !query.keywords.iter().any(matches) {
                return false;
            }
        } else if !query.keywords.iter().all(matches) {
            return false;
        }
    }
    true
}

pub fn verified_details(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    facets: &Facets,
    known: &[(String, u64)],
    movies: &[Movie],
) -> Result<HashMap<u64, Details>, String> {
    let mut found = HashMap::new();
    let mut succeeded = 0;
    let mut error = None;
    for batch in movies.chunks(8) {
        let results = std::thread::scope(|scope| {
            let pending: Vec<_> = batch
                .iter()
                .map(|m| scope.spawn(move || tmdb.details(m.id)))
                .collect();
            pending
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err("Scheda non disponibile.".into()))
                })
                .collect::<Vec<_>>()
        });
        for result in results {
            match result {
                Ok(d) => {
                    succeeded += 1;
                    if verified_match(&d, tax, facets, known) {
                        found.insert(d.movie.id, d);
                    }
                }
                Err(e) => {
                    error = Some(e);
                }
            }
        }
    }
    if succeeded == 0 {
        if let Some(error) = error {
            return Err(error);
        }
    }
    Ok(found)
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

fn pager_button(ui: &mut egui::Ui, kind: lumen::Icon, enabled: bool, tip: &str) -> egui::Response {
    let sense = if enabled {
        egui::Sense::click()
    } else {
        egui::Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(30.0, 22.0), sense);
    let hover = if enabled {
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered(), 0.12)
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
        ),
        egui::StrokeKind::Inside,
    );
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

fn language_picker(ui: &mut egui::Ui, language: &mut String) {
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 36.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.spacing_mut().item_spacing.x = 12.0;
            ui.label(lumen::label("Lingua"));
            for (id, label) in [("it-IT", "Italiano"), ("en-US", "Inglese")] {
                ui.selectable_value(
                    language,
                    id.to_string(),
                    egui::RichText::new(label).size(12.0),
                );
            }
        },
    );
}

fn side_panel_widths(screen_w: f32) -> (f32, f32) {
    let left = (screen_w * 0.30).clamp(260.0, 380.0);
    let right = (screen_w * 0.30).clamp(288.0, 420.0);
    (left, right)
}

fn resize_edge(rect: egui::Rect, pos: egui::Pos2) -> Option<egui::viewport::ResizeDirection> {
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

fn window_caption(ui: &mut egui::Ui, icon: CaptionIcon, tip: &str, cap: f32) -> egui::Response {
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
                stroke,
                egui::StrokeKind::Inside,
            );
        }
        CaptionIcon::Restore => {
            let back =
                egui::Rect::from_center_size(c + egui::vec2(1.5, -1.5), egui::vec2(7.0, 7.0));
            let front =
                egui::Rect::from_center_size(c + egui::vec2(-1.5, 1.5), egui::vec2(7.0, 7.0));
            // Del riquadro dietro restano visibili solo alto e lato destro.
            p.line_segment([back.left_top(), back.right_top()], stroke);
            p.line_segment([back.right_top(), back.right_bottom()], stroke);
            p.rect_stroke(front, lumen::radius(0.5), stroke, egui::StrokeKind::Inside);
        }
        CaptionIcon::Close => {
            let d = 4.2;
            p.line_segment([c + egui::vec2(-d, -d), c + egui::vec2(d, d)], stroke);
            p.line_segment([c + egui::vec2(d, -d), c + egui::vec2(-d, d)], stroke);
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

fn ago_label(ts: i64) -> String {
    if ts <= 0 {
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
        let _ = std::process::Command::new("explorer")
            .arg(url.trim())
            .spawn();
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = url;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn una_risposta_obsoleta_non_ripristina_i_filtri_precedenti_durante_il_debounce() {
        let ctx = egui::Context::default();
        let mut app = App::with_local_data(
            &ctx,
            Config {
                tmdb_api_key: "fixture".into(),
                ..Default::default()
            },
            None,
            simkl::Store::default(),
        );
        app.gen = 4;
        app.tx
            .send(Msg::Results {
                gen: 4,
                payload: Box::new(Results {
                    total_is_exact: true,
                    facets: Facets::default(),
                    removed: vec![],
                    info: vec![],
                    notes: vec![],
                    movies: vec![],
                    total: 0,
                    total_pages: 0,
                    headline: String::new(),
                    error: None,
                    keyword_pairs: vec![],
                }),
            })
            .unwrap();
        app.apply_actions(vec![FacetAction::Genre(35)]);
        app.poll();
        assert_eq!(app.facets.genres, vec![35]);
        assert!(app.dirty.is_some());
        assert!(app.gen > 4);
    }
    use egui::ResizeDirection as Dir;

    #[test]
    fn catalogo_impostazioni_e_primo_avvio_si_adattano_alle_dimensioni_supportate() {
        for size in [[800.0, 600.0], [1024.0, 768.0], [1440.0, 900.0]] {
            for state in [
                "catalogo",
                "caricamento",
                "errore",
                "impostazioni",
                "primo_avvio",
            ] {
                let ctx = egui::Context::default();
                crate::ui::fonts::install_lumen(&ctx);
                let mut app = App::with_local_data(
                    &ctx,
                    Config {
                        tmdb_api_key: "fixture".into(),
                        ..Default::default()
                    },
                    None,
                    simkl::Store::default(),
                );
                app.boot_done = true;
                app.update.last_check = Some(Instant::now());
                app.show_settings = state == "impostazioni";
                app.first_run = state == "primo_avvio";
                app.loading = state == "caricamento";
                if state == "errore" {
                    app.results_error = Some("Errore di prova".into());
                }
                if state == "catalogo" {
                    app.movies = (1..=12)
                        .map(|id| Movie {
                            id,
                            title: format!("Film di prova {id}"),
                            year: Some(1951),
                            ..Default::default()
                        })
                        .collect();
                }
                let out = crate::ui::layout_test_support::frame(
                    &ctx,
                    egui::vec2(size[0], size[1]),
                    |ui| app.update_ui(ui),
                );
                assert!(!out.shapes.is_empty(), "{state} {size:?}");
                let text = out
                    .shapes
                    .iter()
                    .filter_map(|s| {
                        if let egui::epaint::Shape::Text(t) = &s.shape {
                            Some(t.galley.text())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                assert!(!text.contains("Sala 01") && !text.contains("INTERPRETE"));
                match state {
                    "catalogo" => assert!(
                        text.contains("Film di prova"),
                        "Catalogo mancante a {size:?}"
                    ),
                    "caricamento" => assert!(text.contains("Aggiorno il catalogo")),
                    "errore" => assert!(text.contains("Ricerca non completata")),
                    "impostazioni" => assert!(text.contains("Impostazioni")),
                    "primo_avvio" => assert!(text.contains("CHIAVE API TMDB")),
                    _ => unreachable!(),
                }
                assert!(!app.loading || state == "caricamento");
            }
        }
    }

    #[test]
    fn prossimamente_per_titolo_esclude_film_gia_usciti() {
        let tax = Taxonomy::load();
        let f = Facets {
            sort: Sort::Upcoming,
            ..Default::default()
        };
        let movies = vec![
            Movie {
                id: 1,
                release_date: Some("1900-01-01".into()),
                ..Default::default()
            },
            Movie {
                id: 2,
                release_date: Some("2999-01-01".into()),
                ..Default::default()
            },
        ];
        assert_eq!(
            filter_movies(movies, &tax, &f)
                .iter()
                .map(|m| m.id)
                .collect::<Vec<_>>(),
            vec![2]
        );
    }

    #[test]
    fn i_voti_mancanti_non_soddisfano_un_massimo() {
        let d = Details {
            movie: Movie {
                id: 1,
                rating: 0.0,
                votes: 0,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(!verified_match(
            &d,
            &Taxonomy::load(),
            &Facets {
                max_rating: Some(5.0),
                ..Default::default()
            },
            &[]
        ));
    }

    #[test]
    fn il_paese_di_origine_non_viene_sostituito_con_quello_di_produzione() {
        let tax = Taxonomy::load();
        let f = Facets {
            countries: vec!["IT".into()],
            ..Default::default()
        };
        let mut d = Details {
            movie: Movie {
                id: 1,
                ..Default::default()
            },
            origin_country_codes: vec!["IT".into()],
            country_codes: vec!["FR".into()],
            ..Default::default()
        };
        assert!(verified_match(&d, &tax, &f, &[]));
        d.origin_country_codes.clear();
        d.country_codes = vec!["IT".into()];
        assert!(!verified_match(&d, &tax, &f, &[]));
    }
    #[test]
    fn periodi_separati_non_includono_il_decennio_intermedio() {
        let tax = Taxonomy::load();
        let facets = Facets {
            decades: vec!["1970s".into(), "1990s".into()],
            ..Default::default()
        };
        let movies = [1975, 1985, 1995]
            .into_iter()
            .map(|year| Movie {
                year: Some(year),
                ..Default::default()
            })
            .collect();
        let result = filter_movies(movies, &tax, &facets);
        assert_eq!(
            result.iter().map(|m| m.year.unwrap()).collect::<Vec<_>>(),
            vec![1975, 1995]
        );
    }
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
    fn i_bordi_ridimensionano_la_finestra() {
        let r = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(800.0, 600.0));
        assert_eq!(resize_edge(r, egui::pos2(2.0, 2.0)), Some(Dir::NorthWest));
        assert_eq!(resize_edge(r, egui::pos2(798.0, 2.0)), Some(Dir::NorthEast));
        assert_eq!(resize_edge(r, egui::pos2(2.0, 598.0)), Some(Dir::SouthWest));
        assert_eq!(
            resize_edge(r, egui::pos2(798.0, 598.0)),
            Some(Dir::SouthEast)
        );
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
    fn si_aprono_solo_link_web() {
        assert!(allowed_url("https://www.themoviedb.org/movie/1"));
        assert!(allowed_url("http://example.com/?a=1&b=2"));
        assert!(!allowed_url("javascript:alert(1)"));
        assert!(!allowed_url("file:///C:/Windows/System32/calc.exe"));
        assert!(!allowed_url("https://example.com/\ncalc"));
        assert!(!allowed_url("https://"));
    }
}
