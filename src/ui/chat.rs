//! Chat LUMEN: home editoriale, conversazione lineare, risultati hero + griglia
//! e composer piatto. Le azioni restano quelle della 0.2.3.

use egui::{Align2, Color32, FontFamily, FontId, Rect, Rounding, Sense, Stroke};

use crate::domain::model::{Facets, ImdbRating, Movie};
use crate::domain::suggestions::Suggestion;
use crate::domain::taxonomy::Taxonomy;
use crate::ui::fonts;
use crate::ui::images::ImageStore;

use super::{fx, lumen, theme};

const CONTENT_MAX: f32 = 1100.0;
const RESULT_BREAKPOINT: f32 = 940.0;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum State {
    Busy,
    Done,
}

impl Default for State {
    fn default() -> Self {
        State::Done
    }
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
        .fonts(|f| f.layout_no_wrap(label.clone(), font.clone(), color).size().x)
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
        .rect_filled(rect, Rounding::ZERO, lumen::BORDER);
}

fn key_hint(ui: &mut egui::Ui, text: &str) {
    egui::Frame::none()
        .stroke(Stroke::new(1.0_f32, lumen::BORDER))
        .inner_margin(egui::Margin::symmetric(5.0, 2.0))
        .show(ui, |ui| {
            ui.label(
                lumen::mono(8.0, text).color(lumen::FAINT),
            );
        });
}

fn wrapped(
    text: &str,
    size: f32,
    color: Color32,
    width: f32,
    rows: usize,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::simple(
        text.to_string(),
        FontId::proportional(size),
        color,
        width.max(40.0),
    );
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    job
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

#[allow(clippy::too_many_arguments)]
pub fn home(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    draft: &mut String,
    busy: bool,
    ai: bool,
    focus: bool,
    suggestions: &[Suggestion],
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
                    ui.add_space(24.0);
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
                        poster_strip(ui, images, movies, width);
                    } else {
                        let left_w = (width * 0.56).floor();
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
                                |ui| collage(ui, images, movies, right_w),
                            );
                        });
                    }

                    ui.add_space(26.0);
                    ui.horizontal(|ui| {
                        ui.label(lumen::label("Parti da"));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if lumen::link(ui, "Sorprendimi ↗", lumen::EMBER)
                                    .clicked()
                                {
                                    actions.surprise = true;
                                }
                                ui.add_space(12.0);
                                ui.label(
                                    lumen::mono(9.0, "04 SPUNTI LOCALI")
                                        .color(lumen::FAINT),
                                );
                            },
                        );
                    });
                    ui.add_space(8.0);
                    hairline(ui);
                    ui.add_space(12.0);

                    let gap = 10.0;
                    let card_w = ((width - gap) / 2.0).floor();
                    for row in 0..2 {
                        ui.horizontal_top(|ui| {
                            ui.spacing_mut().item_spacing.x = gap;
                            for col in 0..2 {
                                let i = row * 2 + col;
                                let (tag, title, desc) = suggestion_text(suggestions, i);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(card_w, 0.0),
                                    egui::Layout::top_down(egui::Align::LEFT),
                                    |ui| {
                                        let resp = suggestion_card(
                                            ui, card_w, i, tag, title, desc,
                                        );
                                        actions.suggestions_hovered |= resp.hovered();
                                        if resp.clicked() {
                                            actions.prefill = Some(title.to_string());
                                        }
                                    },
                                );
                            }
                        });
                        ui.add_space(gap);
                    }
                    ui.add_space(20.0);
                });
            });
        });
    actions
}

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
    let size = if compact { 38.0 } else { 56.0 };
    ui.label(
        lumen::display(size, "Cosa guardiamo\nstasera?")
            .line_height(Some(size * 0.94)),
    );
    ui.add_space(12.0);
    ui.label(
        lumen::body(
            if compact { 12.0 } else { 13.0 },
            "Descrivi un’atmosfera, un regista o un dettaglio. CineMatch cerca film reali su TMDB e ti mostra perché sono lì.",
        )
        .color(lumen::MUTED),
    );
    ui.add_space(20.0);
    let hint = if compact {
        "Un film intenso, ben diretto…"
    } else {
        "Un film intenso, ben diretto, non troppo lungo…"
    };
    if composer_inner(ui, draft, busy, focus, width, hint, true, compact) {
        actions.send = true;
    }
}

fn collage(ui: &mut egui::Ui, images: &ImageStore, movies: &[Movie], width: f32) {
    ui.label(lumen::label("In sala"));
    ui.add_space(8.0);
    let big_w = (width * 0.55).min(220.0);
    let small_w = (width - big_w - 12.0).min(100.0);
    let group_w = big_w + 12.0 + small_w;
    ui.horizontal_top(|ui| {
        ui.add_space(((width - group_w) * 0.5).max(0.0));
        lumen::poster(
            ui,
            images,
            movies.get(0),
            big_w,
            big_w * 1.5,
            true,
        );
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.add_space(18.0);
            lumen::poster(
                ui,
                images,
                movies.get(1),
                small_w,
                small_w * 1.5,
                false,
            );
            ui.add_space(12.0);
            lumen::poster(
                ui,
                images,
                movies.get(2),
                small_w,
                small_w * 1.5,
                false,
            );
        });
    });
}

fn poster_strip(ui: &mut egui::Ui, images: &ImageStore, movies: &[Movie], width: f32) {
    let gap = 8.0;
    let w = ((width - gap * 2.0) / 3.0).floor();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for i in 0..3 {
            lumen::poster(ui, images, movies.get(i), w, w * 1.5, i == 0);
        }
    });
}

fn suggestion_text<'a>(
    suggestions: &'a [Suggestion],
    i: usize,
) -> (&'a str, &'a str, &'a str) {
    const TAGS: [&str; 4] = ["CORRENTE", "ATMOSFERA", "PERIODO", "DURATA"];
    const TITLES: [&str; 4] = [
        "Nouvelle Vague, senza museo",
        "Un noir che non spiega tutto",
        "Italia anni ’70, fuori dai soliti nomi",
        "Un film sotto i 100 minuti",
    ];
    const DESC: [&str; 4] = [
        "La Parigi che si muove, tra strade, jazz e macchina da presa a spalla.",
        "Notte, pioggia, una città che sembra complice.",
        "Poliziottesco, commedia amara e cinema civile da riscoprire.",
        "Serata piena, tempo stretto, nessun minuto di troppo.",
    ];
    match suggestions.get(i) {
        Some(s) => (s.tag.as_str(), s.prompt.as_str(), s.desc.as_str()),
        None => (TAGS[i % 4], TITLES[i % 4], DESC[i % 4]),
    }
}

fn suggestion_card(
    ui: &mut egui::Ui,
    width: f32,
    index: usize,
    tag: &str,
    title: &str,
    desc: &str,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width, 118.0),
        Sense::click(),
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    p.rect_filled(
        rect,
        Rounding::same(lumen::RADIUS),
        if hover > 0.01 {
            lumen::RAISED
        } else {
            lumen::SURFACE
        },
    );
    p.rect_stroke(
        rect,
        Rounding::same(lumen::RADIUS),
        Stroke::new(
            1.0_f32,
            if hover > 0.01 {
                lumen::EMBER
            } else {
                lumen::BORDER
            },
        ),
    );
    p.text(
        rect.min + egui::vec2(14.0, 12.0),
        Align2::LEFT_TOP,
        format!("{:02} / {}", index + 1, tag.to_uppercase()),
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        lumen::EMBER,
    );
    p.text(
        rect.min + egui::vec2(14.0, 32.0),
        Align2::LEFT_TOP,
        theme::truncate(title, 34),
        FontId::new(18.0, FontFamily::Name(fonts::LUMEN_DISPLAY.into())),
        lumen::IVORY,
    );
    p.text(
        rect.min + egui::vec2(14.0, 60.0),
        Align2::LEFT_TOP,
        theme::truncate(desc, 72),
        FontId::proportional(11.0),
        lumen::MUTED,
    );
    p.text(
        rect.right_bottom() + egui::vec2(-14.0, -12.0),
        Align2::RIGHT_BOTTOM,
        "↗",
        FontId::proportional(13.0),
        if hover > 0.01 {
            lumen::EMBER
        } else {
            lumen::FAINT
        },
    );
    lumen::focus(ui, &resp, lumen::RADIUS);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// -------------------------------------------------------- conversazione -----

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
                        for (index, entry) in entries.iter().enumerate() {
                            user_message(ui, &entry.user);
                            ui.add_space(18.0);
                            answer_view(ui, images, tax, entry, index, now, &mut actions);
                            if index + 1 < entries.len() {
                                ui.add_space(24.0);
                                hairline(ui);
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
    actions.scroll_offset = output.state.offset.y;
    actions.at_bottom = output.state.offset.y >= max_offset - 8.0;
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
        Rounding::ZERO,
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

fn answer_view(
    ui: &mut egui::Ui,
    images: &ImageStore,
    tax: &Taxonomy,
    entry: &Entry,
    index: usize,
    now: f64,
    actions: &mut Actions,
) {
    let a = &entry.answer;
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
    hairline(ui);
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
            ui.set_max_width(900.0_f32.min(ui.available_width()));
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
            ui.painter().rect_filled(bar, Rounding::ZERO, lumen::EMBER);
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
        let cols = result_columns(ui.available_width());
        let gap = 10.0;
        let card_w = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32)
            .floor();
        for row in a.cards[1..].chunks(cols) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for card in row {
                    ui.allocate_ui_with_layout(
                        egui::vec2(card_w, 0.0),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| compact_card(ui, images, tax, card, actions),
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
    hairline(ui);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        if a.grid_ready && total > 0 && !a.facets.is_empty() {
            if link(
                ui,
                &format!("Vedi tutti i {} in griglia ↗", theme::thousands(total)),
                lumen::EMBER,
            )
            .clicked()
            {
                actions.show_all = Some(index);
            }
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
    let frame = egui::Frame::none()
        .fill(lumen::SURFACE)
        .stroke(Stroke::new(1.0_f32, lumen::BORDER))
        .rounding(Rounding::same(lumen::RADIUS))
        .inner_margin(egui::Margin::same(12.0));
    frame.show(ui, |ui| {
        let available = ui.available_width();
        ui.set_min_width(available);
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
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.label(lumen::display(
                    if poster_w < 120.0 { 22.0 } else { 28.0 },
                    &card.movie.title,
                ));
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
                        .on_hover_text("Cerca film simili a questo nella griglia")
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
) {
    let frame = egui::Frame::none()
        .fill(lumen::SURFACE)
        .stroke(Stroke::new(1.0_f32, lumen::BORDER))
        .rounding(Rounding::same(lumen::RADIUS))
        .inner_margin(egui::Margin::same(10.0));
    frame.show(ui, |ui| {
        let content_w = ui.available_width();
        ui.set_min_width(content_w);
        ui.horizontal_top(|ui| {
            let resp = lumen::poster(ui, images, Some(&card.movie), 64.0, 96.0, false);
            if resp.clicked() {
                actions.open_detail = Some(card.movie.id);
            }
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.add(
                    egui::Label::new(lumen::bold(15.0, &card.movie.title)).truncate(),
                );
                ui.add(
                    egui::Label::new(
                        lumen::mono(9.0, movie_meta(card, tax)).color(lumen::FAINT),
                    )
                    .truncate(),
                );
                ui.add_space(4.0);
                ui.add(egui::Label::new(wrapped(
                    &card.perche,
                    11.0,
                    lumen::MUTED,
                    ui.available_width(),
                    2,
                )));
                if !card.consiglio.is_empty() {
                    ui.add_space(4.0);
                    consiglio(ui, &card.consiglio, 10.0);
                }
                if let Some(av) = availability(card) {
                    ui.add_space(4.0);
                    ui.add(
                        egui::Label::new(
                            lumen::mono(8.5, av).color(lumen::CELADON),
                        )
                        .truncate(),
                    );
                }
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 12.0;
                    if link(ui, "Apri scheda", lumen::EMBER).clicked() {
                        actions.open_detail = Some(card.movie.id);
                    }
                    if link(ui, "Film simili", lumen::MUTED).clicked() {
                        actions.similar_to = Some(card.movie.title.clone());
                    }
                });
            });
        });
    });
}

fn consiglio(ui: &mut egui::Ui, text: &str, size: f32) {
    ui.horizontal_top(|ui| {
        let (bar, _) = ui.allocate_exact_size(egui::vec2(2.0, 26.0), Sense::hover());
        ui.painter().rect_filled(bar, Rounding::ZERO, lumen::EMBER);
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
    let stage = a.stage.min(3).max(1);
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
        egui::Id::new(("chat-progress", index)),
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
    compact: bool,
) -> bool {
    let mut send = false;
    let edit_id = egui::Id::new("chat-composer");
    let focused_at_start = ui.memory(|m| m.has_focus(edit_id));
    let focused = focused_at_start;
    let button_w = if wide_send { 100.0 } else { 40.0 };
    let gap = 8.0;
    let inner_w = (width - 24.0).max(120.0);
    let extra = if wide_send && !compact { 52.0 } else { 0.0 };
    let text_w = (inner_w - button_w - gap - extra).max(70.0);

    egui::Frame::none()
        .fill(lumen::DEEP)
        .stroke(Stroke::new(
            1.0_f32,
            if focused {
                lumen::EMBER
            } else {
                lumen::BORDER
            },
        ))
        .rounding(Rounding::same(lumen::RADIUS_SM))
        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                let resp = ui.add_sized(
                    [text_w, 22.0],
                    egui::TextEdit::multiline(draft)
                        .id(edit_id)
                        .frame(false)
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
                if wide_send && !compact {
                    key_hint(ui, "INVIO");
                }
                if send_button(
                    ui,
                    !busy && (!draft.trim().is_empty() || wide_send),
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
        .rect_filled(rect, Rounding::same(lumen::RADIUS_SM), fill);
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
