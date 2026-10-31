//! Chat LUMEN: home editoriale, conversazione lineare, risultati hero + griglia
//! e composer piatto. Le azioni restano quelle della 0.2.3.

use egui::{Align2, Color32, FontFamily, FontId, Rect, Sense, Stroke};

use crate::domain::model::{Facets, ImdbRating, Movie};
use crate::domain::suggestions::Suggestion;
use crate::domain::taxonomy::Taxonomy;
use crate::tmdb::poster_url;
use crate::ui::fonts;
use crate::ui::images::ImageStore;

use super::{fx, lumen, theme};

const CONTENT_MAX: f32 = 1100.0;
const RESULT_BREAKPOINT: f32 = 940.0;

/// Limite del messaggio: un incolla lungo non deve diventare una chiamata AI
/// da migliaia di token.
pub const MAX_PROMPT_CHARS: usize = 800;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum State {
    Busy,
    #[default]
    Done,
}

#[derive(Clone, Default)]
pub struct CardMeta {
    pub director: Option<String>,
    pub runtime: Option<u32>,
    pub flatrate: Vec<String>,
    pub rent: Vec<String>,
    pub imdb: Option<ImdbRating>,
}

#[derive(Clone)]
pub struct Card {
    pub movie: Movie,
    pub perche: String,
    pub consiglio: String,
    pub meta: Option<CardMeta>,
}

#[derive(Clone, Default)]
pub struct Answer {
    pub state: State,
    pub intro: String,
    pub understood: String,
    pub limit: Option<String>,
    pub context: Option<crate::ai::Intent>,
    pub grid_ready: bool,
    pub notes: Vec<String>,
    pub cards: Vec<Card>,
    pub total: u32,
    pub error: Option<String>,
    pub done_at: f64,
    pub facets: Facets,
    pub keyword_pairs: Vec<(String, u64)>,
    pub intent: Option<(String, crate::ai::Intent)>,
    pub stage: u8,
}

impl Answer {
    pub fn busy() -> Self {
        Self {
            state: State::Busy,
            ..Default::default()
        }
    }
}

#[derive(Clone)]
pub struct Entry {
    pub user: String,
    pub answer: Answer,
}

#[derive(Default)]
pub struct Actions {
    pub open_detail: Option<u64>,
    pub similar_to: Option<String>,
    pub retry: Option<usize>,
    pub stop: bool,
    pub show_all: Option<usize>,
    pub new_chat: bool,
    pub scroll_offset: f32,
    pub at_bottom: bool,
}

#[derive(Default)]
pub struct HomeActions {
    pub prefill: Option<String>,
    pub surprise: bool,
    pub send: bool,
    pub suggestions_hovered: bool,
    /// Film scelto da una riga personale (Riprendi / Dalla tua lista).
    pub open_detail: Option<u64>,
}

/// Una card compatta della home costruita dai dati SIMKL.
#[derive(Clone)]
pub struct PersonalCard {
    pub title: String,
    pub meta: String,
    pub movie_id: u64,
    pub progress: Option<f32>,
    /// URL completo del poster TMDB, se già risolto.
    pub poster: Option<String>,
}

// ------------------------------------------------------------- misure ------

fn content_pad(width: f32) -> f32 {
    if width < 900.0 {
        20.0
    } else {
        34.0
    }
}

fn content_width(avail: f32) -> f32 {
    let pad = content_pad(avail);
    (avail - pad * 2.0).clamp(280.0, CONTENT_MAX)
}

fn result_columns(width: f32) -> usize {
    if width >= RESULT_BREAKPOINT {
        2
    } else {
        1
    }
}

// ------------------------------------------------------------ utilita' -----

fn link(ui: &mut egui::Ui, text: &str, color: Color32) -> egui::Response {
    let font = FontId::new(10.0, FontFamily::Name(fonts::LUMEN_MONO.into()));
    let label = text.to_uppercase();
    let width = ui
        .fonts_mut(|f| f.layout_no_wrap(label.clone(), font.clone(), color).size().x)
        + 2.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 20.0), Sense::click());
    let ink = if resp.hovered() {
        if color == lumen::MUTED {
            lumen::IVORY
        } else {
            lumen::EMBER_HI
        }
    } else {
        color
    };
    ui.painter().text(
        rect.left_center(),
        Align2::LEFT_CENTER,
        label,
        font,
        ink,
    );
    lumen::focus(ui, &resp, 0.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn hairline(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 1.0),
        Sense::hover(),
    );
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, lumen::BORDER);
}

fn hairline_width(ui: &mut egui::Ui, width: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 1.0), Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::ZERO, lumen::BORDER);
}

fn wrapped_font(
    text: &str,
    font: FontId,
    color: Color32,
    width: f32,
    rows: usize,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple(
        text.to_string(),
        font,
        color,
        width.max(40.0),
    );
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    job
}

fn wrapped(
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
    rows: usize,
) -> egui::text::LayoutJob {
    wrapped_font(text, FontId::proportional(size), color, width, rows)
}

fn movie_meta(card: &Card, tax: &Taxonomy) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some(y) = card.movie.year {
        parts.push(y.to_string());
    }
    if let Some(m) = &card.meta {
        if let Some(d) = &m.director {
            parts.push(d.clone());
        }
        if let Some(r) = m.runtime {
            parts.push(format!("{r} min"));
        }
    }
    let genres = card
        .movie
        .genre_ids
        .iter()
        .take(2)
        .filter_map(|g| tax.genre_name(*g))
        .collect::<Vec<_>>();
    if !genres.is_empty() {
        parts.push(genres.join(", "));
    }
    if card.movie.votes > 0 {
        parts.push(format!("TMDB {:.1}", card.movie.rating));
    }
    if let Some(imdb) = card.meta.as_ref().and_then(|m| m.imdb) {
        if imdb.votes > 0 {
            parts.push(format!("IMDb {:.1}", imdb.rating));
        }
    }
    parts.join(" · ")
}

fn availability(card: &Card) -> Option<String> {
    let meta = card.meta.as_ref()?;
    if !meta.flatrate.is_empty() {
        Some(format!(
            "IN ABBONAMENTO · {}",
            meta.flatrate
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ))
    } else if !meta.rent.is_empty() {
        Some(format!(
            "A NOLEGGIO · {}",
            meta.rent
                .iter()
                .take(3)
                .cloned()
                .collect::<Vec<_>>()
                .join(", ")
        ))
    } else {
        None
    }
}

// -------------------------------------------------------------- casa -------

/// `movies` e' il cartellone "In sala" della home, non i risultati della
/// griglia: resta lo stesso mentre si cerca in Sala 02.
#[allow(clippy::too_many_arguments)]
pub fn home(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    draft: &mut String,
    busy: bool,
    ai: bool,
    focus: bool,
    tax: &Taxonomy,
    suggestions: &[Suggestion],
    resume: &[PersonalCard],
    watchlist: &[PersonalCard],
) -> HomeActions {
    let mut actions = HomeActions::default();
    egui::ScrollArea::vertical()
        .id_salt("chat-home-scroll")
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let avail = ui.available_width();
            let pad = content_pad(avail);
            let width = content_width(avail);
            let compact = width < 900.0;
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(pad);
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                    ui.add_space(20.0);
                    if compact {
                        hero_left(
                            ui,
                            width,
                            true,
                            draft,
                            busy,
                            ai,
                            focus,
                            &mut actions,
                        );
                        ui.add_space(20.0);
                        poster_strip(ui, images, movies, width, &mut actions);
                    } else {
                        let left_w = (width * 0.59).floor();
                        let right_w = width - left_w - 30.0;
                        ui.horizontal_top(|ui| {
                            ui.allocate_ui_with_layout(
                                egui::vec2(left_w, 0.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| {
                                    hero_left(
                                        ui,
                                        left_w,
                                        false,
                                        draft,
                                        busy,
                                        ai,
                                        focus,
                                        &mut actions,
                                    )
                                },
                            );
                            ui.add_space(30.0);
                            ui.allocate_ui_with_layout(
                                egui::vec2(right_w, 0.0),
                                egui::Layout::top_down(egui::Align::LEFT),
                                |ui| collage(ui, images, movies, right_w, &mut actions),
                            );
                        });
                    }

                    ui.add_space(16.0);
                    ui.horizontal(|ui| {
                        ui.label(lumen::label("Parti da"));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if lumen::link(ui, "Sorprendimi", lumen::EMBER)
                                    .clicked()
                                {
                                    actions.surprise = true;
                                }
                                ui.add_space(12.0);
                                ui.label(
                                    lumen::mono(9.0, "04 PISTE")
                                        .color(lumen::FAINT),
                                );
                            },
                        );
                    });
                    ui.add_space(8.0);
                    hairline(ui);
                    ui.add_space(12.0);

                    let gap = 12.0;
                    let cols = if width >= 900.0 { 4 } else { 2 };
                    let card_w =
                        ((width - gap * (cols - 1) as f32) / cols as f32).floor();
                    let rows = 4usize.div_ceil(cols);
                    for row in 0..rows {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for col in 0..cols {
                                let i = row * cols + col;
                                if i >= 4 {
                                    break;
                                }
                                let pista = pista(tax, suggestions.get(i), i);
                                let resp = ui
                                    .allocate_ui_with_layout(
                                        egui::vec2(card_w, 0.0),
                                        egui::Layout::top_down(egui::Align::LEFT),
                                        |ui| suggestion_card(ui, card_w, i, &pista),
                                    )
                                    .inner;
                                actions.suggestions_hovered |= resp.hovered() || resp.has_focus();
                                if resp.clicked() {
                                    actions.prefill = Some(pista.prompt.clone());
                                    actions.send = true;
                                }
                            }
                        });
                        ui.add_space(gap);
                    }

                    if !resume.is_empty() {
                        ui.add_space(4.0);
                        personal_strip(
                            ui,
                            images,
                            "Riprendi",
                            "DAL TUO SIMKL",
                            resume,
                            &mut actions,
                        );
                        ui.add_space(20.0);
                    }
                    if !watchlist.is_empty() {
                        personal_strip(
                            ui,
                            images,
                            "Dalla tua lista",
                            "DA VEDERE SU SIMKL",
                            watchlist,
                            &mut actions,
                        );
                    }
                    ui.add_space(20.0);
                });
            });
        });
    actions
}

fn personal_strip(
    ui: &mut egui::Ui,
    images: &ImageStore,
    title: &str,
    kicker: &str,
    cards: &[PersonalCard],
    actions: &mut HomeActions,
) {
    let mut scroll_delta = 0.0f32;
    ui.horizontal(|ui| {
        ui.label(lumen::label(title));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if lumen::icon_button(ui, lumen::Icon::ArrowRight, 24.0, "Scorri a destra")
                .clicked()
            {
                scroll_delta = 1.0;
            }
            if lumen::icon_button(ui, lumen::Icon::ArrowLeft, 24.0, "Scorri a sinistra")
                .clicked()
            {
                scroll_delta = -1.0;
            }
            ui.add_space(6.0);
            ui.label(lumen::mono(9.0, kicker).color(lumen::FAINT));
        });
    });
    ui.add_space(8.0);
    let gap = 10.0;
    let card_w = 132.0;
    let page = (card_w + gap) * 5.0;
    egui::ScrollArea::horizontal()
        .id_salt(("personal-strip", title))
        .scroll_source(egui::scroll_area::ScrollSource { drag: egui::scroll_area::DragScroll::Always, ..Default::default() })
        .auto_shrink([false, true])
        .show(ui, |ui| {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for card in cards {
                    personal_card(ui, images, card_w, card, actions);
                }
            });
            if scroll_delta != 0.0 {
                ui.scroll_with_delta(egui::vec2(scroll_delta * page, 0.0));
            }
        });
    ui.add_space(14.0);
}

fn personal_card(
    ui: &mut egui::Ui,
    images: &ImageStore,
    width: f32,
    card: &PersonalCard,
    actions: &mut HomeActions,
) {
    let poster_h = (width * 1.5).round();
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width, poster_h + 46.0),
        Sense::click(),
    );
    let poster = Rect::from_min_size(rect.min, egui::vec2(width, poster_h));
    let painter = ui.painter();
    painter.rect_filled(poster, egui::CornerRadius::ZERO, lumen::SURFACE);
    if let Some(url) = &card.poster {
        if let Some(tex) = images.texture(ui.ctx(), url) {
            painter.image(
                tex.id(),
                poster,
                Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                Color32::WHITE,
            );
            // Il poster può essere un rettangolo diverso da 2:3: riempi i
            // bordi con il fondo, non con il nero del buffer.
            painter.rect_stroke(
                poster,
                egui::CornerRadius::ZERO,
                Stroke::new(1.0_f32, lumen::BORDER), egui::StrokeKind::Inside);
        } else {
            painter.text(
                poster.center(),
                Align2::CENTER_CENTER,
                "SIMKL",
                FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
                lumen::FAINT,
            );
        }
    } else {
        painter.text(
            poster.center(),
            Align2::CENTER_CENTER,
            "SIMKL",
            FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
            lumen::FAINT,
        );
    }
    if resp.hovered() {
        painter.rect_stroke(
            poster,
            egui::CornerRadius::ZERO,
            Stroke::new(1.0_f32, lumen::BORDER_HI), egui::StrokeKind::Inside);
    }
    if let Some(progress) = card.progress {
        let track = Rect::from_min_size(
            egui::pos2(poster.left(), poster.bottom() - 4.0),
            egui::vec2(poster.width(), 4.0),
        );
        painter.rect_filled(track, egui::CornerRadius::ZERO, lumen::INK);
        painter.rect_filled(
            Rect::from_min_size(
                track.min,
                egui::vec2(track.width() * progress.clamp(0.0, 1.0), 4.0),
            ),
            egui::CornerRadius::ZERO,
            lumen::EMBER,
        );
    }
    painter.text(
        egui::pos2(rect.left(), poster.bottom() + 8.0),
        Align2::LEFT_TOP,
        theme::truncate(&card.title, 22),
        FontId::proportional(13.0),
        lumen::IVORY,
    );
    painter.text(
        egui::pos2(rect.left(), poster.bottom() + 27.0),
        Align2::LEFT_TOP,
        &card.meta,
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        lumen::FAINT,
    );
    if resp.clicked() {
        actions.open_detail = Some(card.movie_id);
    }
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled(),
            card.title.clone(),
        )
    });
    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
}

#[allow(clippy::too_many_arguments)]
fn hero_left(
    ui: &mut egui::Ui,
    width: f32,
    compact: bool,
    draft: &mut String,
    busy: bool,
    ai: bool,
    focus: bool,
    actions: &mut HomeActions,
) {
    let _ = ai;
    ui.label(lumen::mono(10.0, "SALA 01 / RICERCA").color(lumen::EMBER));
    ui.add_space(8.0);
    let size = if compact {
        if width >= 600.0 { 44.0 } else { 38.0 }
    } else if width < 600.0 {
        48.0
    } else {
        56.0
    };
    ui.label(
        lumen::display(size, "Cosa guardiamo\nstasera?")
            .line_height(Some(size * 0.94)),
    );
    ui.add_space(12.0);
    ui.allocate_ui_with_layout(
        egui::vec2(width.min(470.0), 0.0),
        egui::Layout::top_down(egui::Align::LEFT),
        |ui| {
            ui.label(
                lumen::body(
                    if compact { 12.0 } else { 13.0 },
                    "Descrivi un’atmosfera, un regista o un dettaglio. CineMatch cerca film reali su\u{a0}TMDB e ti mostra perché sono lì.",
                )
                .color(lumen::MUTED),
            );
        },
    );
    ui.add_space(20.0);
    let hint = if compact {
        "Un film intenso, ben diretto…"
    } else {
        "Un film intenso, ben diretto, non troppo lungo…"
    };
    if composer_inner(ui, draft, busy, focus, width, hint, true) {
        actions.send = true;
    }
}

fn clickable_poster(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movie: Option<&Movie>,
    width: f32,
    height: f32,
    actions: &mut HomeActions,
) {
    if lumen::poster(ui, images, movie, width, height, false).clicked() {
        if let Some(m) = movie {
            actions.open_detail = Some(m.id);
        }
    }
}

fn collage(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    width: f32,
    actions: &mut HomeActions,
) {
    // Il cartellone resta più basso della colonna di testo: le piste entrano
    // nella prima schermata anche sulle finestre basse.
    let big_w = (width * 0.55).min(180.0);
    let gap = 8.0;
    let mut small_w = ((width - big_w - 12.0 - gap) / 2.0).min(84.0);
    let grid = small_w >= 72.0;
    if !grid {
        small_w = (width - big_w - 12.0).min(84.0);
    }
    let grid_w = if grid {
        small_w * 2.0 + gap
    } else {
        small_w
    };
    let group_w = big_w + 12.0 + grid_w;
    let offset = ((width - group_w) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(offset);
        ui.label(lumen::label("In sala"));
    });
    ui.add_space(8.0);
    let big_h = big_w * 1.5;
    let grid_h = small_w * 3.0 + gap;
    ui.horizontal_top(|ui| {
        ui.add_space(offset);
        clickable_poster(ui, images, movies.first(), big_w, big_h, actions);
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.add_space(((big_h - grid_h) * 0.5).max(0.0));
            if grid {
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    clickable_poster(
                        ui,
                        images,
                        movies.get(1),
                        small_w,
                        small_w * 1.5,
                        actions,
                    );
                    clickable_poster(
                        ui,
                        images,
                        movies.get(2),
                        small_w,
                        small_w * 1.5,
                        actions,
                    );
                });
                ui.add_space(gap);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = gap;
                    clickable_poster(
                        ui,
                        images,
                        movies.get(3),
                        small_w,
                        small_w * 1.5,
                        actions,
                    );
                    clickable_poster(
                        ui,
                        images,
                        movies.get(4),
                        small_w,
                        small_w * 1.5,
                        actions,
                    );
                });
            } else {
                clickable_poster(
                    ui,
                    images,
                    movies.get(1),
                    small_w,
                    small_w * 1.5,
                    actions,
                );
                ui.add_space(gap);
                clickable_poster(
                    ui,
                    images,
                    movies.get(2),
                    small_w,
                    small_w * 1.5,
                    actions,
                );
            }
        });
    });
}

fn poster_strip(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    width: f32,
    actions: &mut HomeActions,
) {
    let gap = 8.0;
    let w = ((width - gap * 5.0) / 6.0).floor();
    ui.label(lumen::label("In sala"));
    ui.add_space(8.0);
    if movies.is_empty() {
        ui.label(
            lumen::body(11.0, "Nessun film in sala disponibile.")
                .color(lumen::MUTED),
        );
        return;
    }
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for i in 0..6 {
            clickable_poster(ui, images, movies.get(i), w, w * 1.5, actions);
        }
    });
}

struct Pista {
    kicker: &'static str,
    title: String,
    combination: String,
    desc: String,
    prompt: String,
}

fn pista_fallback(i: usize) -> Pista {
    match i {
        0 => Pista {
            kicker: "CORRENTE",
            title: "Neorealismo".to_string(),
            combination: "1943–1954 · Italia · Dramma".to_string(),
            desc: "Racconto del reale, attori non professionisti, post-guerra.".to_string(),
            prompt: "Neorealismo: un film da recuperare".to_string(),
        },
        1 => Pista {
            kicker: "CORRENTE",
            title: "New Hollywood".to_string(),
            combination: "1967–1976 · Stati Uniti · Dramma".to_string(),
            desc: "Autori contro il sistema, finali amari, cinema che cambia.".to_string(),
            prompt: "New Hollywood: un film da recuperare".to_string(),
        },
        2 => Pista {
            kicker: "PERIODO",
            title: "Italia anni ’70".to_string(),
            combination: "1970–1979 · Italia".to_string(),
            desc: "Poliziottesco, commedia amara e cinema civile.".to_string(),
            prompt: "Un film italiano degli anni ’70".to_string(),
        },
        _ => Pista {
            kicker: "SERATA",
            title: "Corto e intenso".to_string(),
            combination: "durata < 100 min · voto ≥ 7".to_string(),
            desc: "Nessun minuto di troppo.".to_string(),
            prompt: "Un film sotto i 100 minuti con voto almeno 7".to_string(),
        },
    }
}

fn capitalize_first(text: &str) -> String {
    let mut chars = text.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn shadow_title(prompt: &str) -> String {
    let text = prompt.trim();
    for prefix in ["Un film di ", "Un film d'", "Un film ", "Una ", "Un "] {
        if let Some(rest) = text.strip_prefix(prefix) {
            return capitalize_first(rest);
        }
    }
    capitalize_first(text)
}

fn serata_title(tag: &str, facets: &Facets) -> String {
    match tag {
        "serata" => "Corto e intenso".to_string(),
        "capolavoro" => "Da capolavoro".to_string(),
        "breve" => match facets.runtime {
            Some((_, 90)) => "Sotto i 90 minuti".to_string(),
            _ => "Sotto i 100 minuti".to_string(),
        },
        "recente" => "Recenti e ben votati".to_string(),
        "voti" => "Molti voti".to_string(),
        "commedia" => "Commedia ben votata".to_string(),
        "azione" => "Azione sotto i 110".to_string(),
        "fantascienza" => "Fantascienza con voto alto".to_string(),
        "italia" => "Cinema italiano".to_string(),
        "nippon" => "Giappone sotto i 120".to_string(),
        "gemme" => "Gemme nascoste".to_string(),
        "voto" => {
            if facets.max_rating.is_some() {
                "Tra 7 e 8".to_string()
            } else if facets.min_rating.map(|r| r >= 8.0).unwrap_or(false) {
                "Da capolavoro".to_string()
            } else {
                "Voto alto".to_string()
            }
        }
        _ => "Scoperta".to_string(),
    }
}

fn serata_combination(tax: &Taxonomy, facets: &Facets) -> String {
    let mut parts: Vec<String> = Vec::new();
    if let Some((_, hi)) = facets.runtime {
        parts.push(format!("durata < {hi} min"));
    }
    if let Some(min) = facets.min_rating {
        if let Some(max) = facets.max_rating {
            parts.push(format!("voto {min:.0}–{max:.0}"));
        } else {
            parts.push(format!("voto ≥ {min:.0}"));
        }
    }
    if let Some(votes) = facets.min_votes {
        parts.push(format!("≥ {} voti", theme::thousands(votes)));
    }
    if let Some(code) = facets.countries.first() {
        parts.push(tax.country_label(code));
    }
    if let Some(genre) = facets.genres.first().and_then(|g| tax.genre_name(*g)) {
        parts.push(genre);
    }
    if parts.is_empty() {
        parts.push("scoperta".to_string());
    }
    parts.into_iter().take(3).collect::<Vec<_>>().join(" · ")
}

fn pista(tax: &Taxonomy, suggestion: Option<&Suggestion>, i: usize) -> Pista {
    let Some(s) = suggestion else {
        return pista_fallback(i);
    };
    let kicker = s.kind_label();
    let desc = s.desc.clone();
    let prompt = s.prompt.clone();

    if kicker == "CORRENTE" {
        if let Some(m) = s.facets.movements.first().and_then(|id| tax.movement(id)) {
            let mut parts = vec![format!("{}–{}", m.period[0], m.period[1])];
            if let Some(code) = m.countries.first() {
                parts.push(tax.country_label(code));
            }
            if let Some(genre) = m.genres_core.first().and_then(|g| tax.genre_name(*g)) {
                parts.push(genre);
            }
            return Pista {
                kicker,
                title: m.label.clone(),
                combination: parts.join(" · "),
                desc,
                prompt,
            };
        }
    }

    if kicker == "PERIODO" {
        let decade = s.facets.decades.first().and_then(|d| tax.decade(d));
        let genre = s.facets.genres.first().and_then(|g| tax.genre_name(*g));
        let country = s.facets.countries.first().map(|c| tax.country_label(c));
        let title = match (&country, decade) {
            (Some(c), Some(d)) => format!("{} {}", c, d.label.to_lowercase()),
            _ => shadow_title(&s.prompt),
        };
        let mut parts = Vec::new();
        if let Some(d) = decade {
            parts.push(format!("{}–{}", d.from, d.to));
        }
        if let Some(c) = &country {
            parts.push(c.clone());
        }
        if let Some(g) = &genre {
            parts.push(g.clone());
        }
        if !parts.is_empty() {
            return Pista {
                kicker,
                title,
                combination: parts.join(" · "),
                desc,
                prompt,
            };
        }
    }

    if kicker == "SERATA" {
        return Pista {
            kicker,
            title: serata_title(&s.tag, &s.facets),
            combination: serata_combination(tax, &s.facets),
            desc,
            prompt,
        };
    }

    Pista {
        kicker,
        title: shadow_title(&s.prompt),
        combination: String::new(),
        desc,
        prompt,
    }
}

fn suggestion_card(
    ui: &mut egui::Ui,
    width: f32,
    index: usize,
    pista: &Pista,
) -> egui::Response {
    const CARD_H: f32 = 150.0;
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(width, CARD_H), Sense::click());
    resp.widget_info(|| {
        egui::WidgetInfo::labeled(
            egui::Role::Button,
            ui.is_enabled(),
            pista.title.clone(),
        )
    });
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    p.rect_filled(
        rect,
        lumen::radius(lumen::RADIUS),
        if hover > 0.01 {
            lumen::RAISED
        } else {
            lumen::SURFACE
        },
    );
    p.rect_stroke(
        rect,
        lumen::radius(lumen::RADIUS),
        Stroke::new(
            1.0_f32,
            if hover > 0.01 {
                lumen::BORDER_HI
            } else {
                lumen::BORDER
            },
        ), egui::StrokeKind::Inside);
    p.text(
        rect.min + egui::vec2(14.0, 12.0),
        Align2::LEFT_TOP,
        format!("{:02} / {}", index + 1, pista.kicker),
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        lumen::EMBER,
    );
    let max_title_w = width - 28.0;
    let mut title_size = 20.0_f32;
    while title_size > 16.0 {
        let font = FontId::new(
            title_size,
            FontFamily::Name(fonts::LUMEN_DISPLAY.into()),
        );
        let measured = ui.fonts_mut(|f| {
            f.layout_no_wrap(pista.title.clone(), font, lumen::IVORY)
                .size()
                .x
        });
        if measured <= max_title_w {
            break;
        }
        title_size -= 1.0;
    }
    let title_galley = p.layout_job(wrapped_font(
        &pista.title,
        FontId::new(
            title_size,
            FontFamily::Name(fonts::LUMEN_DISPLAY.into()),
        ),
        lumen::IVORY,
        max_title_w,
        1,
    ));
    p.galley(rect.min + egui::vec2(14.0, 30.0), title_galley, lumen::IVORY);
    let combo_galley = p.layout_job(wrapped_font(
        &pista.combination,
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        lumen::FAINT,
        width - 28.0,
        1,
    ));
    p.galley(
        rect.min + egui::vec2(14.0, 62.0),
        combo_galley,
        lumen::FAINT,
    );
    let desc_galley = p.layout_job(wrapped_font(
        &pista.desc,
        FontId::proportional(11.5),
        lumen::MUTED,
        width - 28.0,
        2,
    ));
    p.galley(rect.min + egui::vec2(14.0, 79.0), desc_galley, lumen::MUTED);
    p.rect_filled(
        Rect::from_min_size(
            egui::pos2(rect.left() + 14.0, rect.bottom() - 34.0),
            egui::vec2(width - 28.0, 1.0),
        ),
        egui::CornerRadius::ZERO,
        lumen::BORDER,
    );
    let footer = if hover > 0.01 {
        lumen::EMBER_HI
    } else {
        lumen::EMBER
    };
    p.text(
        egui::pos2(rect.left() + 14.0, rect.bottom() - 16.0),
        Align2::LEFT_BOTTOM,
        "AVVIA LA RICERCA",
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        footer,
    );
    lumen::icon(
        p,
        Rect::from_center_size(
            egui::pos2(rect.right() - 22.0, rect.bottom() - 20.0),
            egui::vec2(12.0, 12.0),
        ),
        lumen::Icon::ArrowRight,
        footer,
    );
    lumen::focus(ui, &resp, lumen::RADIUS);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// -------------------------------------------------------- conversazione -----

#[allow(clippy::too_many_arguments)]
pub fn view(
    ui: &mut egui::Ui,
    images: &ImageStore,
    tax: &Taxonomy,
    entries: &[Entry],
    busy: bool,
    now: f64,
    jump: &mut bool,
    bottom_pad: f32,
) -> Actions {
    let mut actions = Actions::default();
    let stick = busy
        || entries
            .last()
            .map(|e| e.answer.done_at > 0.0 && now - e.answer.done_at < 1.5)
            .unwrap_or(false);
    if stick {
        ui.ctx().request_repaint();
    }
    let output = egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .stick_to_bottom(stick)
        .show(ui, |ui| {
            let avail = ui.available_width();
            let pad = content_pad(avail);
            let width = content_width(avail);
            ui.add_space(20.0);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.add_space(pad);
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        ui.set_max_width(width);
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if lumen::link(ui, "Nuova chat", lumen::MUTED).clicked() {
                                        actions.new_chat = true;
                                    }
                                },
                            );
                        });
                        ui.add_space(10.0);
                        for (index, entry) in entries.iter().enumerate() {
                            user_message(ui, &entry.user);
                            ui.add_space(18.0);
                            answer_view(ui, images, tax, entry, index, now, &mut actions, width);
                            if index + 1 < entries.len() {
                                ui.add_space(24.0);
                                hairline_width(ui, width);
                                ui.add_space(24.0);
                            }
                        }
                        if *jump {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                            *jump = false;
                        }
                        ui.add_space(bottom_pad.max(16.0));
                    },
                );
            });
        });
    let max_offset = (output.content_size.y - output.inner_rect.height()).max(0.0);
    actions.scroll_offset = output.state.clamped_offset().y;
    actions.at_bottom = output.state.clamped_offset().y >= max_offset - 8.0;
    actions
}

fn user_message(ui: &mut egui::Ui, text: &str) {
    let avail = ui.available_width();
    let galley = ui.painter().layout(
        text.to_string(),
        FontId::proportional(13.0),
        lumen::IVORY,
        (avail * 0.7).min(620.0),
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(avail, galley.size().y + 8.0),
        Sense::hover(),
    );
    let right = rect.right() - 36.0;
    let left = (right - galley.size().x).max(rect.left());
    ui.painter().galley(
        egui::pos2(left, rect.top() + 2.0),
        galley.clone(),
        lumen::IVORY,
    );
    ui.painter().rect_filled(
        Rect::from_min_max(
            egui::pos2(right + 10.0, rect.top() + 2.0),
            egui::pos2(right + 12.0, rect.top() + 2.0 + galley.size().y),
        ),
        egui::CornerRadius::ZERO,
        lumen::EMBER,
    );
    ui.painter().text(
        egui::pos2(rect.right() - 8.0, rect.top() + 3.0),
        Align2::RIGHT_TOP,
        "TU",
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        lumen::FAINT,
    );
}

#[allow(clippy::too_many_arguments)]
fn answer_view(
    ui: &mut egui::Ui,
    images: &ImageStore,
    tax: &Taxonomy,
    entry: &Entry,
    index: usize,
    now: f64,
    actions: &mut Actions,
    width: f32,
) {
    let a = &entry.answer;
    ui.set_max_width(width);
    let meta = match a.state {
        State::Busy => "IN CORSO".to_string(),
        State::Done if a.error.is_some() => "ERRORE".to_string(),
        State::Done if a.cards.is_empty() => "NESSUN RISULTATO".to_string(),
        State::Done => format!("SELEZIONE · {} FILM", a.cards.len()),
    };
    ui.horizontal(|ui| {
        ui.label(lumen::mono(10.0, "CINEMATCH").color(lumen::EMBER));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            ui.label(lumen::mono(9.0, meta).color(lumen::FAINT));
        });
    });
    ui.add_space(8.0);
    hairline_width(ui, width);
    ui.add_space(12.0);

    if a.state == State::Busy {
        busy_view(ui, a, index, now, actions);
        return;
    }

    if let Some(err) = &a.error {
        let interrupted = err == "Risposta interrotta.";
        ui.label(
            lumen::body(
                12.0,
                if interrupted {
                    err.clone()
                } else {
                    format!("Qualcosa non ha funzionato: {err}")
                },
            )
            .color(if interrupted {
                lumen::MUTED
            } else {
                lumen::DANGER
            }),
        );
        ui.add_space(4.0);
        if link(ui, "Riprova", lumen::EMBER).clicked() {
            actions.retry = Some(index);
        }
        if a.cards.is_empty() {
            return;
        }
        ui.add_space(10.0);
    }

    if !a.intro.is_empty() {
        ui.scope(|ui| {
            ui.set_max_width(900.0_f32.min(width));
            ui.label(lumen::body(14.0, &a.intro));
        });
    }
    if !a.understood.is_empty() {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(lumen::label("Criteri"));
            ui.add(
                egui::Label::new(
                    lumen::mono(10.0, theme::truncate(&a.understood, 180))
                        .color(lumen::MUTED),
                )
                .truncate(),
            );
        });
    }
    if let Some(limit) = &a.limit {
        ui.add_space(5.0);
        ui.horizontal_top(|ui| {
            let (bar, _) = ui.allocate_exact_size(egui::vec2(2.0, 16.0), Sense::hover());
            ui.painter().rect_filled(bar, egui::CornerRadius::ZERO, lumen::EMBER);
            ui.add_space(7.0);
            ui.add(
                egui::Label::new(wrapped(
                    limit,
                    10.5,
                    lumen::MUTED,
                    ui.available_width(),
                    3,
                ))
                .wrap(),
            );
        });
    }

    if a.cards.is_empty() {
        if a.intro.is_empty() && a.error.is_none() {
            ui.add_space(4.0);
            ui.label(
                lumen::body(
                    12.0,
                    "Nessun film con questi criteri. Prova ad allargare la richiesta.",
                )
                .color(lumen::MUTED),
            );
        }
        return;
    }

    ui.add_space(14.0);
    hero_card(ui, images, tax, &a.cards[0], actions);
    if a.cards.len() > 1 {
        ui.add_space(10.0);
        let cols = result_columns(width);
        let gap = 10.0;
        let card_w = ((width - gap * (cols - 1) as f32) / cols as f32).floor();
        for row in a.cards[1..].chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for card in row {
                    ui.allocate_ui_with_layout(
                        egui::vec2(card_w, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| compact_card(ui, images, tax, card, actions, card_w),
                    );
                }
            });
            ui.add_space(gap);
        }
    }

    let total = if a.total > 0 {
        a.total
    } else {
        a.cards.len() as u32
    };
    ui.add_space(4.0);
    hairline_width(ui, width);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if a.grid_ready && total > 0 && !a.facets.is_empty()
            && link(
                ui,
                &format!("Vedi tutti i {} nel catalogo →", theme::thousands(total)),
                lumen::EMBER,
            )
            .clicked()
        {
            actions.show_all = Some(index);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.add_space(8.0);
            ui.label(
                lumen::mono(9.0, format!("{} RISULTATI", theme::thousands(total)))
                    .color(lumen::FAINT),
            );
        });
    });
}

fn hero_card(
    ui: &mut egui::Ui,
    images: &ImageStore,
    tax: &Taxonomy,
    card: &Card,
    actions: &mut Actions,
) {
    let frame = egui::Frame::NONE
        .fill(lumen::SURFACE)
        .stroke(Stroke::new(1.0_f32, lumen::BORDER))
        .corner_radius(lumen::radius(lumen::RADIUS))
        .inner_margin(egui::Margin::same(12));
    frame.show(ui, |ui| {
        let available = ui.available_width();
        ui.set_min_width(available);
        ui.set_max_width(available);
        let poster_w = if available < 760.0 { 110.0 } else { 150.0 };
        ui.horizontal_top(|ui| {
            let resp = lumen::poster(
                ui,
                images,
                Some(&card.movie),
                poster_w,
                poster_w * 1.5,
                true,
            );
            if resp.clicked() {
                actions.open_detail = Some(card.movie.id);
            }
            if card.movie.track.watched {
                lumen::watched_overlay(ui, resp.rect, resp.hovered());
            }
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.label(
                    lumen::display(
                        if poster_w < 120.0 { 22.0 } else { 28.0 },
                        &card.movie.title,
                    )
                    .color(if card.movie.track.watched {
                        lumen::MUTED
                    } else {
                        lumen::IVORY
                    }),
                );
                ui.add(
                    egui::Label::new(
                        lumen::mono(10.0, movie_meta(card, tax)).color(lumen::FAINT),
                    )
                    .truncate(),
                );
                ui.add_space(8.0);
                ui.add(egui::Label::new(wrapped(
                    &card.perche,
                    13.0,
                    lumen::MUTED,
                    ui.available_width(),
                    3,
                )));
                if !card.consiglio.is_empty() {
                    ui.add_space(6.0);
                    consiglio(ui, &card.consiglio, 11.0);
                }
                if let Some(av) = availability(card) {
                    ui.add_space(6.0);
                    ui.add(
                        egui::Label::new(
                            lumen::mono(10.0, av).color(lumen::CELADON),
                        )
                        .truncate(),
                    );
                }
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 14.0;
                    if link(ui, "Apri scheda", lumen::EMBER).clicked() {
                        actions.open_detail = Some(card.movie.id);
                    }
                    if link(ui, "Film simili", lumen::MUTED)
                        .on_hover_text("Cerca film simili a questo nel catalogo")
                        .clicked()
                    {
                        actions.similar_to = Some(card.movie.title.clone());
                    }
                });
            });
        });
    });
}

fn compact_card(
    ui: &mut egui::Ui,
    images: &ImageStore,
    tax: &Taxonomy,
    card: &Card,
    actions: &mut Actions,
    width: f32,
) {
    const CARD_H: f32 = 172.0;
    let (rect, _) =
        ui.allocate_exact_size(egui::vec2(width, CARD_H), Sense::hover());
    let id = ui.scope_id().with(("chat-card", card.movie.id));
    let resp = ui.interact(rect, id, Sense::hover());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    ui.painter().rect_filled(
        rect,
        lumen::radius(lumen::RADIUS),
        if hover > 0.01 {
            lumen::RAISED
        } else {
            lumen::SURFACE
        },
    );
    ui.painter().rect_stroke(
        rect,
        lumen::radius(lumen::RADIUS),
        Stroke::new(
            1.0_f32,
            if hover > 0.01 {
                lumen::EMBER
            } else {
                lumen::BORDER
            },
        ), egui::StrokeKind::Inside);

    let poster_rect = Rect::from_min_size(
        rect.min + egui::vec2(10.0, 10.0),
        egui::vec2(64.0, 96.0),
    );
    ui.painter()
        .rect_filled(poster_rect, egui::CornerRadius::ZERO, lumen::RAISED);
    let mut loaded = false;
    if let Some(path) = &card.movie.poster_path {
        let uri = poster_url(path, "w342");
        if let Some(tex) = images.texture(ui.ctx(), &uri) {
            let sized = egui::load::SizedTexture::new(tex.id(), poster_rect.size());
            ui.put(poster_rect, egui::Image::new(sized));
            loaded = true;
        }
    }
    let poster_resp = ui.interact(poster_rect, id.with("poster"), Sense::click());
    if card.movie.track.watched {
        lumen::watched_overlay(ui, poster_rect, poster_resp.hovered());
    }
    if !loaded {
        ui.painter().text(
            poster_rect.center(),
            Align2::CENTER_CENTER,
            theme::truncate(&card.movie.title, 18),
            FontId::new(10.0, FontFamily::Name(fonts::LUMEN_DISPLAY.into())),
            lumen::IVORY,
        );
    }
    ui.painter().rect_stroke(
        poster_rect,
        egui::CornerRadius::ZERO,
        Stroke::new(
            1.0_f32,
            if poster_resp.hovered() {
                lumen::BORDER_HI
            } else {
                lumen::BORDER
            },
        ), egui::StrokeKind::Inside);
    if poster_resp.clicked() {
        actions.open_detail = Some(card.movie.id);
    }

    let text_x = poster_rect.right() + 10.0;
    let text_w = (rect.right() - 10.0 - text_x).max(80.0);
    let p = ui.painter();
    let title = p.layout_job(wrapped_font(
        &card.movie.title,
        FontId::new(15.0, FontFamily::Name(fonts::LUMEN_UI_BOLD.into())),
        lumen::IVORY,
        text_w,
        1,
    ));
    p.galley(
        rect.min + egui::vec2(84.0, 10.0),
        title,
        if card.movie.track.watched {
            lumen::MUTED
        } else {
            lumen::IVORY
        },
    );
    let meta = p.layout_job(wrapped_font(
        &movie_meta(card, tax),
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        lumen::FAINT,
        text_w,
        1,
    ));
    p.galley(rect.min + egui::vec2(84.0, 30.0), meta, lumen::FAINT);
    let desc = p.layout_job(wrapped_font(
        &card.perche,
        FontId::proportional(11.0),
        lumen::MUTED,
        text_w,
        2,
    ));
    p.galley(rect.min + egui::vec2(84.0, 46.0), desc, lumen::MUTED);
    if !card.consiglio.is_empty() {
        p.rect_filled(
            Rect::from_min_size(rect.min + egui::vec2(84.0, 78.0), egui::vec2(2.0, 26.0)),
            egui::CornerRadius::ZERO,
            lumen::EMBER,
        );
        let cons = p.layout_job(wrapped_font(
            &format!("Consiglio · {}", card.consiglio),
            FontId::proportional(10.0),
            lumen::MUTED,
            text_w - 10.0,
            2,
        ));
        p.galley(rect.min + egui::vec2(94.0, 78.0), cons, lumen::MUTED);
    }
    if let Some(av) = availability(card) {
        let av = p.layout_job(wrapped_font(
            &av,
            FontId::new(8.5, FontFamily::Name(fonts::LUMEN_MONO.into())),
            lumen::CELADON,
            text_w,
            1,
        ));
        p.galley(rect.min + egui::vec2(84.0, 110.0), av, lumen::CELADON);
    }
    let (x, opened) = card_link(
        ui,
        id.with("open"),
        egui::pos2(text_x, rect.top() + 134.0),
        "Apri scheda",
        lumen::EMBER,
    );
    if opened {
        actions.open_detail = Some(card.movie.id);
    }
    let (_, similar) = card_link(
        ui,
        id.with("similar"),
        egui::pos2(x + 12.0, rect.top() + 134.0),
        "Film simili",
        lumen::MUTED,
    );
    if similar {
        actions.similar_to = Some(card.movie.title.clone());
    }
}

fn card_link(
    ui: &mut egui::Ui,
    id: egui::Id,
    pos: egui::Pos2,
    text: &str,
    color: Color32,
) -> (f32, bool) {
    let font = FontId::new(10.0, FontFamily::Name(fonts::LUMEN_MONO.into()));
    let label = text.to_uppercase();
    let width = ui.fonts_mut(|f| {
        f.layout_no_wrap(label.clone(), font.clone(), color)
            .size()
            .x
    }) + 2.0;
    let rect = Rect::from_min_size(pos, egui::vec2(width, 20.0));
    let resp = ui.interact(rect, id, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let ink = if hover > 0.01 {
        if color == lumen::MUTED {
            lumen::IVORY
        } else {
            lumen::EMBER_HI
        }
    } else {
        color
    };
    ui.painter()
        .text(rect.left_center(), Align2::LEFT_CENTER, label, font, ink);
    lumen::focus(ui, &resp, 0.0);
    (
        rect.right(),
        resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked(),
    )
}

fn consiglio(ui: &mut egui::Ui, text: &str, size: f32) {
    ui.horizontal_top(|ui| {
        let (bar, _) = ui.allocate_exact_size(egui::vec2(2.0, 26.0), Sense::hover());
        ui.painter().rect_filled(bar, egui::CornerRadius::ZERO, lumen::EMBER);
        ui.add_space(7.0);
        ui.add(
            egui::Label::new(wrapped(
                &format!("Consiglio · {text}"),
                size,
                lumen::MUTED,
                ui.available_width(),
                2,
            ))
            .wrap(),
        );
    });
}

fn busy_view(
    ui: &mut egui::Ui,
    a: &Answer,
    index: usize,
    _now: f64,
    actions: &mut Actions,
) {
    const TAPPE: [&str; 3] = ["INTERPRETO", "CERCO SU TMDB", "SCELGO I FILM"];
    let stage = a.stage.clamp(1, 3);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        for (i, label) in TAPPE.iter().enumerate() {
            let step = i as u8 + 1;
            let (color, text) = if step < stage {
                (lumen::MUTED, format!("✓ {label}"))
            } else if step == stage {
                (lumen::EMBER, label.to_string())
            } else {
                (lumen::FAINT, label.to_string())
            };
            ui.label(lumen::mono(9.0, text).color(color));
            if i + 1 < TAPPE.len() {
                ui.label(lumen::mono(9.0, "·").color(lumen::FAINT));
            }
        }
    });
    ui.add_space(9.0);
    let target = (stage as f32 / 3.0).max(0.08);
    let anim = ui.ctx().animate_value_with_time(
        egui::Id::unique(("chat-progress", index)),
        target,
        0.4,
    );
    fx::progress(ui, 260.0, anim);
    if !a.intro.is_empty() {
        ui.add_space(10.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(lumen::body(13.0, &a.intro));
            ui.label(lumen::body(13.0, "▌").color(lumen::EMBER));
        });
    }
    ui.add_space(9.0);
    if link(ui, "Interrompi", lumen::MUTED).clicked() {
        actions.stop = true;
    }
}

// ------------------------------------------------------------ composer -----

pub fn composer(
    ui: &mut egui::Ui,
    draft: &mut String,
    busy: bool,
    focus: bool,
    width: f32,
) -> bool {
    composer_inner(
        ui,
        draft,
        busy,
        focus,
        width,
        "Scrivi cosa vuoi vedere…",
        false,
    )
}

fn composer_inner(
    ui: &mut egui::Ui,
    draft: &mut String,
    busy: bool,
    focus: bool,
    width: f32,
    hint: &str,
    wide_send: bool,
) -> bool {
    let mut send = false;
    let edit_id = egui::Id::unique("chat-composer");
    let focused_at_start = ui.memory(|m| m.has_focus(edit_id));
    let focused = focused_at_start;
    let button_w = if wide_send { 100.0 } else { 40.0 };
    let gap = 8.0;
    let inner_w = (width - 24.0).max(120.0);
    let text_w = (inner_w - button_w - gap).max(70.0);

    egui::Frame::NONE
        .fill(lumen::DEEP)
        .stroke(Stroke::new(
            1.0_f32,
            if focused {
                lumen::EMBER
            } else {
                lumen::BORDER
            },
        ))
        .corner_radius(lumen::radius(lumen::RADIUS_SM))
        .inner_margin(egui::Margin::symmetric(12, 6))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let resp = ui.add_sized(
                    [text_w, 40.0],
                    egui::TextEdit::multiline(draft)
                        .id(edit_id)
                        .frame(egui::Frame::NONE)
                        .char_limit(MAX_PROMPT_CHARS)
                        .desired_rows(1)
                        .hint_text(hint)
                        .font(egui::TextStyle::Body)
                        .return_key(egui::KeyboardShortcut::new(
                            egui::Modifiers::SHIFT,
                            egui::Key::Enter,
                        )),
                );
                if focus {
                    resp.request_focus();
                }
                let enter = ui.input(|i| {
                    i.key_pressed(egui::Key::Enter) && !i.modifiers.shift
                });
                if enter && (focused_at_start || resp.has_focus()) {
                    send = true;
                }
                if send_button(
                    ui,
                    !busy && !draft.trim().is_empty(),
                    wide_send,
                )
                .clicked()
                {
                    send = true;
                }
            });
        });
    if send && (busy || draft.trim().is_empty()) {
        send = false;
    }
    send
}

fn send_button(ui: &mut egui::Ui, enabled: bool, wide: bool) -> egui::Response {
    let (w, h) = if wide { (100.0, 40.0) } else { (40.0, 40.0) };
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), sense);
    let hover = if enabled {
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered(), 0.12)
    } else {
        0.0
    };
    let fill = if enabled {
        if hover > 0.01 {
            lumen::EMBER_HI
        } else {
            lumen::EMBER
        }
    } else {
        lumen::DEEP
    };
    ui.painter()
        .rect_filled(rect, lumen::radius(lumen::RADIUS_SM), fill);
    let ink = if enabled {
        lumen::INK
    } else {
        lumen::FAINT
    };
    if wide {
        ui.painter().text(
            egui::pos2(rect.left() + 12.0, rect.center().y),
            Align2::LEFT_CENTER,
            "CERCA",
            FontId::new(10.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
            ink,
        );
        lumen::icon(
            ui.painter(),
            Rect::from_center_size(
                egui::pos2(rect.right() - 16.0, rect.center().y),
                egui::vec2(14.0, 14.0),
            ),
            lumen::Icon::ArrowRight,
            ink,
        );
    } else {
        lumen::icon(
            ui.painter(),
            rect.shrink(11.0),
            lumen::Icon::ArrowRight,
            ink,
        );
    }
    lumen::focus(ui, &resp, lumen::RADIUS_SM);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Invia · Shift+Invio va a capo")
    } else {
        resp.on_hover_text("Scrivi un messaggio")
    }
}

#[cfg(test)]
mod layout_tests {
    use super::result_columns;

    #[test]
    fn risultati_una_colonna_nella_finestra_minima() {
        assert_eq!(result_columns(760.0), 1);
        assert_eq!(result_columns(939.0), 1);
    }

    #[test]
    fn risultati_due_colonne_quando_c_e_spazio() {
        assert_eq!(result_columns(940.0), 2);
        assert_eq!(result_columns(1100.0), 2);
    }
}
