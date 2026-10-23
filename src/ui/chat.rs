//! La chat di CineMatch: la conversazione (bolle dell'utente e risposte
//! dell'AI) e le card dei film consigliati, con il «perché» e il consiglio.
//! Il modulo espone solo la vista e un pacchetto di azioni: chi la usa (app)
//! decide cosa farne.

use egui::{Align2, Color32, FontId, Rect, RichText, Rounding, Sense, Stroke};

use crate::domain::model::{Facets, ImdbRating, Movie};
use crate::domain::suggestions::Suggestion;
use crate::domain::taxonomy::Taxonomy;
use crate::tmdb::poster_url;
use crate::ui::images::ImageStore;
use crate::ui::{fx, theme};

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

/// Dati di contorno di un film scelto (dalla scheda TMDB).
#[derive(Clone, Default)]
pub struct CardMeta {
    pub director: Option<String>,
    pub runtime: Option<u32>,
    pub flatrate: Vec<String>,
    pub rent: Vec<String>,
    pub imdb: Option<ImdbRating>,
}

/// Un film consigliato con i commenti del curatore.
#[derive(Clone)]
pub struct Card {
    pub movie: Movie,
    pub perche: String,
    pub consiglio: String,
    pub meta: Option<CardMeta>,
}

/// La risposta a un messaggio: in corso oppure finita.
#[derive(Clone, Default)]
pub struct Answer {
    pub state: State,
    pub intro: String,
    pub understood: String,
    /// Limite dichiarato: nessun film copre tutti i criteri, oppure i criteri
    /// esatti sono stati allargati.
    pub limit: Option<String>,
    /// Contesto effettivo della risposta: i seguiti ripartono da qui.
    pub context: Option<crate::ai::Intent>,
    /// La griglia rappresenta davvero questi criteri (niente temi liberi).
    pub grid_ready: bool,
    pub notes: Vec<String>,
    pub cards: Vec<Card>,
    pub total: u32,
    pub error: Option<String>,
    /// Istante (ctx time) in cui la risposta e arrivata: guida la comparsa.
    pub done_at: f64,
    /// Filtri interpretati, per «vedi tutti in griglia».
    pub facets: Facets,
    pub keyword_pairs: Vec<(String, u64)>,
    /// Intento completo, riusato dalla griglia senza una nuova chiamata AI.
    pub intent: Option<(String, crate::ai::Intent)>,
    /// Avanzamento della pipeline: 1 interprete, 2 TMDB, 3 curatela.
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

/// Cosa ha chiesto l'utente alla vista in questo frame.
#[derive(Default)]
pub struct Actions {
    pub open_detail: Option<u64>,
    pub similar_to: Option<String>,
    pub retry: Option<usize>,
    pub stop: bool,
    pub explain: Option<u64>,
    pub show_all: Option<usize>,
    pub new_chat: bool,
    /// Offset dello scroll in questo frame: l'app lo usa per capire se si
    /// sta leggendo verso l'alto e minimizzare il composer.
    pub scroll_offset: f32,
    /// Vero quando la conversazione e' arrivata in fondo.
    pub at_bottom: bool,
}

/// Azioni della casa: il composer invia, gli spunti riempiono soltanto.
#[derive(Default)]
pub struct HomeActions {
    pub prefill: Option<String>,
    /// «Sorprendimi» chiede all'app una proposta locale non visibile.
    pub surprise: bool,
    pub send: bool,
    /// La rotazione si ferma mentre il mouse e sopra una card.
    pub suggestions_hovered: bool,
}

/// Disposizione della casa: colonne, righe e altezza stimata del blocco.
/// La stima serve solo a centrare senza spingere il composer sotto il bordo.
fn home_layout(content_w: f32, count: usize) -> (usize, usize, f32) {
    let cols = if content_w >= 820.0 { 4 } else { 2 };
    let rows = count.min(4).div_ceil(cols);
    let mut height = 252.0 + rows.saturating_sub(1) as f32 * 130.0;
    if content_w < 900.0 {
        height += 15.0;
    }
    (cols, rows, height)
}

fn home_top(avail_y: f32, content_h: f32) -> f32 {
    ((avail_y - content_h) * 0.5).clamp(8.0, 320.0)
}

/// Prima schermata della chat: titolo, quattro spunti e composer. Sulle
/// finestre basse il composer resta ancorato in basso; con più altezza torna
/// subito sotto gli spunti. Gli spunti non inviano: preparano il testo nel
/// composer, pronti da correggere. `focus` arriva da «/» o da un click.
/// `shown_at` e' l'istante dell'ultima rotazione: serve alla comparsa morbida.
#[allow(clippy::too_many_arguments)]
pub fn home(
    ui: &mut egui::Ui,
    draft: &mut String,
    busy: bool,
    ai: bool,
    focus: bool,
    suggestions: &[Suggestion],
    now: f64,
    shown_at: f64,
) -> HomeActions {
    let mut actions = HomeActions::default();
    let avail = ui.available_size();
    // Il margine tiene conto della scrollbar verticale: senza, le card
    // dell'ultima colonna finirebbero sotto la barra.
    let content_w = (avail.x - 16.0).clamp(320.0, 980.0);
    let (cols, rows, body_h) = home_layout(content_w, suggestions.len());

    let body = |ui: &mut egui::Ui| -> HomeActions {
        let mut body_actions = HomeActions::default();
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Cosa guardiamo stasera?")
                    .size(30.0)
                    .strong(),
            );
            ui.add_space(8.0);
            let (line, _) = ui.allocate_exact_size(egui::vec2(178.0, 3.0), Sense::hover());
            fx::gradient_bar(
                ui.painter(),
                line,
                theme::ACCENT,
                theme::ACCENT_2,
                2.0,
            );
            ui.add_space(10.0);
            ui.label(
                RichText::new(if ai {
                    "Scrivi libero, anche a modo tuo: interpreto la richiesta, cerco su TMDB e ti propongo fino a 5 film con il perché e un consiglio."
                } else {
                    "Scrivi libero: le regole interpretano la richiesta, cerco su TMDB e ti propongo fino a 5 film con una spiegazione."
                })
                .size(12.5)
                .color(theme::DIM),
            );
        });

        ui.add_space(22.0);
        let gap = 12.0;
        let card_w = ((content_w - gap * (cols - 1) as f32) / cols as f32).floor();
        let shown = suggestions.iter().take(4).collect::<Vec<_>>();
        let elapsed = (now - shown_at).max(0.0);
        if elapsed < 0.45 {
            // Solo la comparsa: la rotazione successiva viene
            // programmata dall'app, non da un ciclo continuo.
            ui.ctx().request_repaint();
        }
        for row in 0..rows {
            if row > 0 {
                ui.add_space(gap);
            }
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                for col in 0..cols {
                    let index = row * cols + col;
                    let Some(item) = shown.get(index) else {
                        break;
                    };
                    let reveal = card_reveal(elapsed, index);
                    ui.push_id(&item.prompt, |ui| {
                        let response = spunto_card(
                            ui,
                            card_w,
                            &item.tag,
                            &item.prompt,
                            &item.desc,
                            reveal,
                        );
                        body_actions.suggestions_hovered |= response.hovered();
                        if response.clicked() {
                            body_actions.prefill = Some(item.prompt.clone());
                        }
                    });
                }
            });
        }

        ui.add_space(8.0);
        ui.vertical_centered(|ui| {
            if ui
                .add(
                    egui::Button::new(
                        RichText::new("Sorprendimi")
                            .size(11.0)
                            .color(theme::ACCENT_3),
                    )
                    .frame(false),
                )
                .on_hover_text("Un'idea locale: la scrivo nel composer")
                .clicked()
            {
                body_actions.surprise = true;
            }
        });
        body_actions
    };

    let mut composer_row = |ui: &mut egui::Ui| -> bool {
        let composer_w = content_w.min(820.0);
        let pad = ((content_w - composer_w) * 0.5).max(0.0);
        let mut send = false;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.add_space(pad);
            if composer(ui, draft, busy, focus, composer_w) {
                send = true;
            }
        });
        send
    };

    // Sotto i 560 px il composer resta ancorato in basso e il blocco sopra
    // scorre. Con più altezza si torna al blocco unico, con il composer
    // subito sotto gli spunti.
    if avail.y < 560.0 {
        let scroll_h = (avail.y - 96.0).max(120.0);
        let top = home_top(scroll_h, body_h);
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), scroll_h),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                egui::ScrollArea::vertical()
                    .id_salt("chat-home-scroll")
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        ui.add_space(top);
                        let left = ((ui.available_width() - content_w) * 0.5).max(0.0);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = 0.0;
                            ui.add_space(left);
                            ui.vertical(|ui| {
                                ui.set_width(content_w);
                                let body_actions = body(ui);
                                actions.suggestions_hovered |= body_actions.suggestions_hovered;
                                if body_actions.prefill.is_some() {
                                    actions.prefill = body_actions.prefill;
                                }
                                actions.surprise |= body_actions.surprise;
                            });
                        });
                    });
            },
        );
        ui.add_space(8.0);
        if composer_row(ui) {
            actions.send = true;
        }
    } else {
        let top = home_top(avail.y, body_h + 92.0);
        egui::ScrollArea::vertical()
            .id_salt("chat-home-scroll")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.add_space(top);
                let left = ((ui.available_width() - content_w) * 0.5).max(0.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.add_space(left);
                    ui.vertical(|ui| {
                        ui.set_width(content_w);
                        let body_actions = body(ui);
                        actions.suggestions_hovered |= body_actions.suggestions_hovered;
                        if body_actions.prefill.is_some() {
                            actions.prefill = body_actions.prefill;
                        }
                        actions.surprise |= body_actions.surprise;
                        ui.add_space(14.0);
                        if composer_row(ui) {
                            actions.send = true;
                        }
                    });
                });
            });
    }
    actions
}

/// Disegna la conversazione. `busy` = c'e una risposta in corso (la vista
/// resta incollata in fondo); `now` = clock di egui per le animazioni.
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
    // Resta in fondo durante lo streaming e per un attimo dopo: le card
    // arrivano in blocco e altrimenti resterebbero sotto il bordo.
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
            ui.add_space(8.0);
            for (index, entry) in entries.iter().enumerate() {
                user_bubble(ui, &entry.user);
                ui.add_space(7.0);
                answer_view(ui, images, tax, entry, index, now, &mut actions);
                ui.add_space(16.0);
            }
            // Dopo un invio si torna in fondo anche se si era scrollato su:
            // il messaggio appena scritto non deve restare fuori vista.
            if *jump {
                ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                *jump = false;
            }
            // Il composer galleggia sopra la conversazione: lo spazio finale
            // evita che l'ultima card finisca dietro al riquadro di scrittura.
            ui.add_space(bottom_pad);
        });
    let max_offset = (output.content_size.y - output.inner_rect.height()).max(0.0);
    actions.scroll_offset = output.state.offset.y;
    actions.at_bottom = output.state.offset.y >= max_offset - 8.0;
    actions
}

/// Composer condiviso da casa e conversazione. `width` e' la larghezza del
/// riquadro; `focus` richiede il cursore (scorciatoia «/» o spunto cliccato).
/// Restituisce true quando l'utente vuole inviare.
pub fn composer(
    ui: &mut egui::Ui,
    draft: &mut String,
    busy: bool,
    focus: bool,
    width: f32,
) -> bool {
    let mut send = false;
    let edit_id = egui::Id::new("chat-composer");
    // Il fuoco va letto prima del TextEdit: Invio puo' farlo perdere nello
    // stesso frame (egui lo tratta come attivazione del widget a fuoco).
    let focused_at_start = ui.memory(|m| m.has_focus(edit_id));
    let focused = focused_at_start;
    let inner_w = (width - 28.0).max(60.0);

    let mut prepared = egui::Frame::none()
        .fill(if focused {
            theme::mix(theme::CARD, theme::CARD_HOVER, 0.28)
        } else {
            theme::CARD
        })
        .stroke(Stroke::new(
            1.0_f32,
            if focused {
                theme::alpha(theme::ACCENT, 205)
            } else {
                theme::BORDER
            },
        ))
        .rounding(Rounding::same(17.0))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 10.0),
            blur: 28.0,
            spread: 0.0,
            color: Color32::from_black_alpha(120),
        })
        .inner_margin(egui::Margin::symmetric(14.0, 10.0))
        .begin(ui);
    // Il Frame eredita il layout orizzontale del chiamante: qui dentro
    // vogliamo esplicitamente testo sopra e riga dei comandi sotto.
    prepared
        .content_ui
        .with_layout(egui::Layout::top_down(egui::Align::Min), |ui| {
            ui.set_width(inner_w);
            let resp = ui.add(
                egui::TextEdit::multiline(draft)
                    .id(edit_id)
                    .frame(false)
                    .desired_rows(1)
                    .desired_width(inner_w)
                    .hint_text("Scrivi cosa vuoi vedere…")
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
            ui.add_space(4.0);
            ui.horizontal(|ui| {
                // La riga occupa tutto il composer: il suggerimento resta a
                // sinistra, il pulsante di invio a destra.
                ui.set_width(inner_w);
                ui.label(
                    RichText::new("Invio invia · Shift+Invio va a capo")
                        .size(10.0)
                        .color(theme::FAINT),
                );
                ui.with_layout(
                    egui::Layout::right_to_left(egui::Align::Center),
                    |ui| {
                        let enabled = !busy && !draft.trim().is_empty();
                        let button = send_button(ui, enabled, busy);
                        if button.clicked() {
                            send = true;
                        }
                    },
                );
            });
        });
    prepared.end(ui);
    if send && (busy || draft.trim().is_empty()) {
        send = false;
    }
    send
}

/// Barra compatta del composer: mentre si legge la conversazione occupa
/// poco spazio; un click riapre il campo vero.
pub fn composer_mini(ui: &mut egui::Ui, draft: &str, busy: bool, width: f32) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 44.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(17.0),
        theme::mix(theme::CARD, theme::CARD_HOVER, hover),
    );
    painter.rect_stroke(
        rect,
        Rounding::same(17.0),
        Stroke::new(
            1.0_f32,
            if resp.hovered() {
                theme::alpha(theme::ACCENT, 170)
            } else {
                theme::BORDER
            },
        ),
    );
    let text = if busy {
        "Sto preparando i consigli…"
    } else if draft.trim().is_empty() {
        "Scrivi un messaggio…"
    } else {
        draft.trim()
    };
    painter.text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        Align2::LEFT_CENTER,
        theme::truncate(text, 72),
        FontId::proportional(12.5),
        theme::DIM,
    );
    painter.text(
        rect.right_center() - egui::vec2(16.0, 0.0),
        Align2::RIGHT_CENTER,
        "Apri",
        FontId::proportional(10.5),
        theme::FAINT,
    );
    resp.on_hover_text("Riapri la barra di ricerca").clicked()
}

/// Cerchio di invio: gradiente quando c'e testo, puntini mentre l'AI lavora.
fn send_button(ui: &mut egui::Ui, enabled: bool, busy: bool) -> egui::Response {
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), sense);
    let hover = if enabled {
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered(), 0.12)
    } else {
        0.0
    };
    let painter = ui.painter();
    if enabled && hover > 0.02 {
        painter.circle_filled(
            rect.center(),
            18.0 + hover,
            theme::alpha(theme::ACCENT, (38.0 * hover) as u8),
        );
    }
    let fill = if enabled {
        theme::mix(theme::ACCENT, theme::ACCENT_2, 0.24 + 0.16 * hover)
    } else {
        theme::CARD
    };
    painter.circle_filled(rect.center(), 15.0, fill);
    painter.circle_stroke(
        rect.center(),
        15.0,
        Stroke::new(
            1.0_f32,
            if enabled {
                theme::alpha(theme::ACCENT_3, 95)
            } else {
                theme::BORDER
            },
        ),
    );
    if busy {
        let now = ui.input(|i| i.time);
        for i in 0..3 {
            let a = 0.30 + 0.70 * theme::pulse(now - i as f64 * 0.12, 5.0);
            painter.circle_filled(
                rect.center() + egui::vec2((i as f32 - 1.0) * 5.0, 0.0),
                1.7,
                theme::alpha(theme::ACCENT_3, (210.0 * a) as u8),
            );
        }
        ui.ctx().request_repaint();
    } else {
        painter.text(
            rect.center() + egui::vec2(0.0, -0.5),
            Align2::CENTER_CENTER,
            "↑",
            FontId::proportional(16.0),
            if enabled {
                Color32::WHITE
            } else {
                theme::FAINT
            },
        );
    }
    if enabled {
        if resp.has_focus() {
            painter.circle_stroke(
                rect.center(),
                18.0,
                Stroke::new(1.5_f32, theme::alpha(theme::ACCENT_3, 190)),
            );
        }
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
            .on_hover_text("Invia")
    } else if busy {
        resp.on_hover_text("Sto preparando i consigli…")
    } else {
        resp.on_hover_text("Scrivi un messaggio")
    }
}

/// Comparsa scaglionata delle card dopo una rotazione. Curva ease-out
/// cubica: veloce all'inizio, morbida sull'arrivo.
fn card_reveal(elapsed: f64, index: usize) -> f32 {
    let local = ((elapsed - index as f64 * 0.035) / 0.26).clamp(0.0, 1.0);
    (1.0 - (1.0 - local).powi(3)) as f32
}

/// Schedina di spunto: etichetta, richiesta e una riga di contesto.
/// `reveal` va da 0 (appena arrivata) a 1 (ferma). Il rect del hit-test resta
/// fermo: si muove solo la pittura, cosi' il clic non balla durante la
/// comparsa.
fn spunto_card(
    ui: &mut egui::Ui,
    w: f32,
    tag: &str,
    prompt: &str,
    desc: &str,
    reveal: f32,
) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 118.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let ease = reveal.clamp(0.0, 1.0);
    let rect = rect.translate(egui::vec2(0.0, (1.0 - ease) * 7.0));
    let fade = |c: Color32| theme::alpha(c, (c.a() as f32 * ease) as u8);
    let painter = ui.painter();
    painter.rect_filled(
        rect,
        Rounding::same(14.0),
        fade(theme::mix(theme::CARD, theme::CARD_HOVER, hover)),
    );
    if ease > 0.01 && ease < 0.99 {
        // Filo di luce blu/ciano: accompagna il cambio senza un lampo pieno.
        let edge = egui::Rect::from_min_size(rect.left_top(), egui::vec2(rect.width(), 1.6));
        let glow = (1.0 - ease) * 150.0;
        fx::gradient_bar(
            painter,
            edge,
            theme::alpha(theme::ACCENT, glow as u8),
            theme::alpha(theme::ACCENT_2, (glow * 0.8) as u8),
            1.0,
        );
    }
    painter.rect_stroke(
        rect,
        Rounding::same(14.0),
        Stroke::new(
            1.0_f32,
            fade(theme::mix(
                theme::BORDER,
                theme::alpha(theme::ACCENT, 200),
                hover,
            )),
        ),
    );
    theme::focus_ring(ui, &resp, 14.0);
    painter.text(
        rect.min + egui::vec2(14.0, 12.0),
        Align2::LEFT_TOP,
        tag.to_uppercase(),
        FontId::proportional(10.0),
        fade(theme::alpha(theme::ACCENT_3, 220)),
    );
    let mut title = egui::text::LayoutJob::simple(
        prompt.to_string(),
        FontId::proportional(12.5),
        fade(theme::TEXT),
        (w - 28.0).max(40.0),
    );
    title.wrap.max_rows = 2;
    title.wrap.overflow_character = Some('…');
    let title = painter.layout_job(title);
    painter.galley(rect.min + egui::vec2(14.0, 30.0), title, fade(theme::TEXT));

    let mut subtitle = egui::text::LayoutJob::simple(
        desc.to_string(),
        FontId::proportional(11.0),
        fade(theme::DIM),
        (w - 28.0).max(40.0),
    );
    subtitle.wrap.max_rows = 2;
    subtitle.wrap.overflow_character = Some('…');
    let subtitle = painter.layout_job(subtitle);
    painter.galley(rect.min + egui::vec2(14.0, 68.0), subtitle, fade(theme::DIM));
    painter.text(
        rect.right_bottom() + egui::vec2(-14.0, -12.0),
        Align2::RIGHT_BOTTOM,
        "→",
        FontId::proportional(12.0),
        fade(theme::mix(theme::FAINT, theme::ACCENT_3, hover)),
    );
    if resp.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
    }
    resp
}

fn user_bubble(ui: &mut egui::Ui, text: &str) {
    let max = (ui.available_width() * 0.72).min(620.0);
    ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
        egui::Frame::none()
            .fill(theme::mix(theme::ACCENT_DIM, theme::ACCENT, 0.16))
            .stroke(Stroke::new(1.0_f32, theme::alpha(theme::ACCENT, 90)))
            .rounding(Rounding {
                nw: 14.0,
                ne: 5.0,
                sw: 14.0,
                se: 14.0,
            })
            .shadow(egui::epaint::Shadow {
                offset: egui::vec2(0.0, 8.0),
                blur: 20.0,
                spread: 0.0,
                color: Color32::from_black_alpha(80),
            })
            .inner_margin(egui::Margin::symmetric(13.0, 9.0))
            .show(ui, |ui| {
                ui.set_max_width(max);
                ui.label(RichText::new(text).size(13.0));
            });
    });
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
    match a.state {
        State::Busy => {
            busy_view(ui, a, index, now, actions);
            return;
        }
        State::Done => {}
    }

    if let Some(err) = &a.error {
        let interrupted = err == "Risposta interrotta.";
        ui.label(
            RichText::new(if interrupted {
                err.clone()
            } else {
                format!("Qualcosa non ha funzionato: {err}")
            })
            .size(12.5)
            .color(if interrupted { theme::DIM } else { theme::DANGER }),
        );
        ui.add_space(4.0);
        if theme::chip(
            ui,
            "↻  Riprova",
            false,
            true,
            Some("Ripete questa richiesta"),
        )
        .clicked()
        {
            actions.retry = Some(index);
        }
    }
    if !a.intro.is_empty() {
        ui.label(RichText::new(&a.intro).size(13.5));
    }
    if !a.understood.is_empty() {
        ui.add_space(2.0);
        ui.label(
            RichText::new(format!("Ho capito così · {}", a.understood))
                .size(11.0)
                .color(theme::DIM),
        );
    }
    if let Some(limit) = &a.limit {
        ui.add_space(2.0);
        ui.label(RichText::new(limit).size(11.5).color(theme::ACCENT_3));
    }
    if a.cards.is_empty() {
        if a.intro.is_empty() && a.error.is_none() {
            ui.label(
                RichText::new("Non ho trovato film da proporti con questi criteri.")
                    .size(12.5)
                    .color(theme::DIM),
            );
        }
        return;
    }

    ui.add_space(8.0);
    for (i, card) in a.cards.iter().enumerate() {
        let alpha = if a.done_at <= 0.0 {
            1.0
        } else {
            ((now - a.done_at - 0.05 * i as f64) / 0.22).clamp(0.0, 1.0) as f32
        };
        if alpha < 1.0 {
            ui.ctx().request_repaint();
        }
        film_card(ui, images, tax, card, i, alpha, actions);
        ui.add_space(10.0);
    }

    if a.grid_ready && a.total > 0 && !a.facets.is_empty() {
        ui.add_space(2.0);
        ui.horizontal_wrapped(|ui| {
            if theme::chip(
                ui,
                &format!("Vedi tutti i {} in griglia", theme::thousands(a.total)),
                false,
                true,
                Some("Apre la griglia con gli stessi criteri"),
            )
            .clicked()
            {
                actions.show_all = Some(index);
            }
        });
    }
}

/// Avanzamento della pipeline: tre tappe, quella attiva pulsa; sotto, la
/// barra che avanza. Durante lo streaming il testo cresce col cursore.
fn busy_view(ui: &mut egui::Ui, a: &Answer, index: usize, now: f64, actions: &mut Actions) {
    const TAPPE: [&str; 3] = [
        "Interpreto la richiesta",
        "Cerco su TMDB",
        "Scelgo e scrivo i film",
    ];
    let stage = a.stage.min(3);
    ui.horizontal_wrapped(|ui| {
        for (i, label) in TAPPE.iter().enumerate() {
            let done = (i as u8) + 1 < stage;
            let active = (i as u8) + 1 == stage.max(1);
            if i > 0 {
                ui.add_space(10.0);
            }
            ui.horizontal(|ui| {
                if done {
                    fx::dot(ui, theme::OK, 0.0);
                    ui.label(
                        RichText::new(format!("✓ {label}"))
                            .size(11.0)
                            .color(theme::mix(theme::OK, theme::DIM, 0.35)),
                    );
                } else if active {
                    let p = theme::pulse(now, 6.0);
                    fx::dot(ui, theme::mix(theme::ACCENT, theme::ACCENT_2, p), p);
                    ui.label(RichText::new(*label).size(11.0).strong());
                } else {
                    fx::dot(ui, theme::FAINT, 0.0);
                    ui.label(RichText::new(*label).size(11.0).color(theme::FAINT));
                }
            });
        }
    });
    ui.add_space(7.0);
    let target = (stage as f32 / 3.0).max(0.08);
    let anim = ui.ctx().animate_value_with_time(
        egui::Id::new(("chat-progress", index)),
        target,
        0.4,
    );
    fx::progress(ui, 260.0, anim);
    if !a.intro.is_empty() {
        ui.add_space(9.0);
        let caret = if theme::pulse(now, 4.0) > 0.5 { "▌" } else { "" };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            ui.label(RichText::new(&a.intro).size(13.5));
            ui.label(RichText::new(caret).size(13.5).color(theme::ACCENT));
        });
    }
    ui.add_space(7.0);
    if theme::chip(ui, "■  Interrompi", false, true, Some("Ferma la risposta in corso"))
        .clicked()
    {
        actions.stop = true;
    }
}

fn badge(ui: &mut egui::Ui, text: &str) {
    let galley = ui.painter().layout(
        text.to_string(),
        FontId::proportional(10.0),
        Color32::WHITE,
        f32::INFINITY,
    );
    let size = galley.size() + egui::vec2(12.0, 4.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    fx::gradient_bar(
        ui.painter(),
        rect,
        theme::ACCENT,
        theme::mix(theme::ACCENT_2, theme::ACCENT, 0.45),
        6.0,
    );
    ui.painter()
        .galley(rect.min + egui::vec2(6.0, 2.0), galley, Color32::WHITE);
}

fn film_card(
    ui: &mut egui::Ui,
    images: &ImageStore,
    tax: &Taxonomy,
    card: &Card,
    index: usize,
    alpha: f32,
    actions: &mut Actions,
) {
    ui.scope(|ui| {
        ui.set_opacity(alpha);
        let outer = theme::card_frame()
            .show(ui, |ui| {
                // Tutte le card occupano la stessa colonna: i bordi laterali
                // restano allineati anche con titoli e commenti di lunghezza
                // diversa.
                ui.set_width(ui.available_width());
                ui.horizontal_top(|ui| {
                    // Poster cliccabile, con shimmer finché l'immagine arriva.
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(116.0, 174.0), Sense::click());
                    ui.painter().rect_filled(
                        rect,
                        Rounding::same(12.0),
                        theme::POSTER_BG,
                    );
                    let mut loaded = false;
                    if let Some(path) = &card.movie.poster_path {
                        let uri = poster_url(path, "w342");
                        if let Some(tex) = images.texture(ui.ctx(), &uri) {
                            let sized = egui::load::SizedTexture::new(tex.id(), rect.size());
                            ui.put(rect, egui::Image::new(sized).rounding(Rounding::same(12.0)));
                            loaded = true;
                        }
                    }
                    let now = ui.ctx().input(|i| i.time);
                    if !loaded {
                        fx::shimmer(ui, rect, 12.0, now);
                    }
                    let hover = ui
                        .ctx()
                        .animate_bool_with_time(resp.id, resp.hovered(), 0.16);
                    fx::ring(ui, rect, 12.0, theme::ACCENT, hover);
                    theme::focus_ring(ui, &resp, 12.0);
                    if resp.on_hover_text("Apri la scheda").clicked() {
                        actions.open_detail = Some(card.movie.id);
                    }
                    // Voto in sovraimpressione, in basso sul poster.
                    if card.movie.votes > 0 {
                        let g = ui.painter().layout(
                            format!("★ {:.1}", card.movie.rating),
                            FontId::proportional(10.0),
                            Color32::WHITE,
                            f32::INFINITY,
                        );
                        let size = g.size() + egui::vec2(10.0, 4.0);
                        let chip = Rect::from_min_size(
                            egui::pos2(rect.left() + 6.0, rect.bottom() - size.y - 6.0),
                            size,
                        );
                        let p = ui.painter();
                        p.rect_filled(chip, Rounding::same(7.0), Color32::from_black_alpha(160));
                        p.galley(chip.min + egui::vec2(5.0, 2.0), g, Color32::WHITE);
                    }

                    ui.add_space(4.0);
                    ui.vertical(|ui| {
                        ui.horizontal_wrapped(|ui| {
                            // Il titolo e i metadati hanno bisogno del loro
                            // respiro: lo spacing zero dell'intro non si eredita.
                            ui.spacing_mut().item_spacing.x = 6.0;
                            ui.label(
                                RichText::new(format!("{}.", index + 1))
                                    .size(15.0)
                                    .strong()
                                    .color(theme::ACCENT),
                            );
                            ui.label(RichText::new(&card.movie.title).size(15.5).strong());
                            if let Some(y) = card.movie.year {
                                ui.label(
                                    RichText::new(y.to_string()).size(12.0).color(theme::DIM),
                                );
                            }
                            if index == 0 {
                                badge(ui, "SCELTA MIGLIORE");
                            }
                        });

                        let mut meta: Vec<String> = Vec::new();
                        if let Some(m) = &card.meta {
                            if let Some(dir) = &m.director {
                                meta.push(format!("di {dir}"));
                            }
                            if let Some(rt) = m.runtime {
                                meta.push(format!("{rt} min"));
                            }
                        }
                        if card.movie.votes > 0 {
                            meta.push(format!(
                                "TMDB {:.1} ★ · {} voti",
                                card.movie.rating,
                                theme::thousands(card.movie.votes)
                            ));
                        }
                        if let Some(imdb) = card.meta.as_ref().and_then(|m| m.imdb) {
                            if imdb.votes > 0 {
                                meta.push(format!("IMDb {:.1} ★", imdb.rating));
                            }
                        }
                        if !meta.is_empty() {
                            ui.label(
                                RichText::new(meta.join("   ·   "))
                                    .size(11.5)
                                    .color(theme::DIM),
                            );
                        }

                        let genres: Vec<String> = card
                            .movie
                            .genre_ids
                            .iter()
                            .take(3)
                            .filter_map(|g| tax.genre_name(*g))
                            .collect();
                        if !genres.is_empty() {
                            ui.label(
                                RichText::new(genres.join(" · "))
                                    .size(11.0)
                                    .color(theme::DIM),
                            );
                        }
                        if let Some(m) = &card.meta {
                            let dove = if !m.flatrate.is_empty() {
                                Some(("In abbonamento", &m.flatrate))
                            } else if !m.rent.is_empty() {
                                Some(("A noleggio", &m.rent))
                            } else {
                                None
                            };
                            if let Some((label, list)) = dove {
                                ui.label(
                                    RichText::new(format!(
                                        "{label}: {}",
                                        list.iter().take(3).cloned().collect::<Vec<_>>().join(", ")
                                    ))
                                    .size(11.0)
                                    .color(theme::OK),
                                );
                            }
                        }

                        ui.add_space(6.0);
                        ui.label(
                            RichText::new("PERCHÉ QUESTO FILM")
                                .size(10.5)
                                .strong()
                                .color(theme::ACCENT),
                        );
                        ui.add_space(1.0);
                        ui.label(RichText::new(&card.perche).size(12.5));

                        ui.add_space(6.0);
                        egui::Frame::none()
                            .fill(theme::mix(theme::ACCENT_DIM, theme::CARD, 0.45))
                            .rounding(Rounding::same(12.0))
                            .inner_margin(egui::Margin::symmetric(11.0, 8.0))
                            .show(ui, |ui| {
                                ui.set_max_width(ui.available_width());
                                ui.horizontal_wrapped(|ui| {
                                    ui.spacing_mut().item_spacing.x = 6.0;
                                    ui.label(
                                        RichText::new("Consiglio")
                                            .size(11.0)
                                            .strong()
                                            .color(theme::ACCENT),
                                    );
                                    ui.label(RichText::new(&card.consiglio).size(12.0));
                                });
                            });

                        ui.add_space(8.0);
                        ui.horizontal_wrapped(|ui| {
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Apri scheda").size(12.0))
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::new(1.0_f32, theme::BORDER))
                                        .rounding(Rounding::same(10.0)),
                                )
                                .clicked()
                            {
                                actions.open_detail = Some(card.movie.id);
                            }
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Film simili").size(12.0))
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::new(1.0_f32, theme::BORDER))
                                        .rounding(Rounding::same(10.0)),
                                )
                                .on_hover_text("Cerca film simili a questo nella griglia")
                                .clicked()
                            {
                                actions.similar_to = Some(card.movie.title.clone());
                            }
                            if ui
                                .add(
                                    egui::Button::new(RichText::new("Spiega").size(12.0))
                                        .fill(Color32::TRANSPARENT)
                                        .stroke(Stroke::new(1.0_f32, theme::BORDER))
                                        .rounding(Rounding::same(10.0)),
                                )
                                .on_hover_text("Spiega perché questo film è nella lista")
                                .clicked()
                            {
                                actions.explain = Some(card.movie.id);
                            }
                        });
                    });
                });
            });
        // Anello di luce sull'intera card quando il mouse ci passa sopra.
        let card_hover = ui.ctx().animate_bool_with_time(
            ui.id().with(("card", card.movie.id)),
            outer.response.hovered(),
            0.15,
        );
        fx::ring(
            ui,
            outer.response.rect,
            theme::ROUND_CARD,
            theme::ACCENT,
            card_hover,
        );
    });
}

#[cfg(test)]
mod layout_tests {
    use super::{home_layout, home_top};

    #[test]
    fn home_entra_nella_finestra_minima() {
        // 800x600: sopra il composer restano circa 426 px di scroll.
        let (cols, rows, height) = home_layout(756.0, 4);
        assert_eq!((cols, rows), (2, 2));
        assert!(height + home_top(426.0, height) <= 426.0);
    }

    #[test]
    fn home_resta_a_quattro_colonne_quando_c_e_spazio() {
        let (cols, rows, height) = home_layout(969.0, 4);
        assert_eq!((cols, rows), (4, 1));
        assert!(height + home_top(484.0, height) <= 484.0);
    }
}
