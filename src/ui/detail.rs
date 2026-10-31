use egui::{Align2, FontFamily, FontId, Sense, Stroke};

use crate::domain::model::Details;
use crate::ui::images::ImageStore;
use crate::ui::{grid, lumen};

pub enum DetailAction {
    Open(u64),
    OpenUrl(String),
    Close,
    MarkWatched,
    AddWatchlist,
    RemoveTracking,
    RetryWrite,
    Rate(u8),
    ClearRating,
}

/// Stato SIMKL di un film, calcolato dall'app e passato alla scheda.
#[derive(Clone, Default)]
pub struct TrackInfo {
    pub status: Option<String>,
    pub rating: Option<u8>,
    pub url: Option<String>,
    pub connected: bool,
    pub busy: bool,
    /// Ultimo errore di scrittura per questo film, mostrato nella barra.
    pub error: Option<String>,
}

fn label_list(items: &[String]) -> String {
    items.join(" · ")
}

/// Nomi paese TMDB accorciati, per non mandare a capo la riga dei metadati.
fn short_country(name: &str) -> &str {
    match name {
        "United States of America" => "USA",
        "United Kingdom" => "UK",
        other => other,
    }
}

fn close_button(ui: &mut egui::Ui) -> egui::Response {
    // 36 px come i bottoni dell'app; il riquadro compare solo sotto il
    // puntatore, così la testata resta una riga e non un bottone.
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(36.0, 36.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    if hover > 0.01 {
        ui.painter().rect_filled(
            rect,
            lumen::radius(lumen::RADIUS_SM),
            lumen::mix(lumen::INK, lumen::RAISED, hover),
        );
        ui.painter().rect_stroke(
            rect,
            lumen::radius(lumen::RADIUS_SM),
            Stroke::new(1.0_f32, lumen::BORDER),
            egui::StrokeKind::Inside,
        );
    }
    let c = rect.center();
    let color = lumen::mix(lumen::MUTED, lumen::IVORY, hover);
    ui.painter().line_segment(
        [c + egui::vec2(-5.0, -5.0), c + egui::vec2(5.0, 5.0)],
        Stroke::new(1.2_f32, color),
    );
    ui.painter().line_segment(
        [c + egui::vec2(5.0, -5.0), c + egui::vec2(-5.0, 5.0)],
        Stroke::new(1.2_f32, color),
    );
    lumen::focus(ui, &resp, lumen::RADIUS_SM);
    resp.on_hover_text("Chiudi (Esc)")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn avail_row(ui: &mut egui::Ui, label: &str, items: &[String], color: egui::Color32) {
    if items.is_empty() {
        return;
    }
    let row = ui.horizontal_top(|ui| {
        ui.add_space(9.0);
        ui.add(
            egui::Label::new(
                lumen::mono(
                    9.0,
                    format!(
                        "{label} · {}",
                        label_list(&items.iter().take(3).cloned().collect::<Vec<_>>())
                    ),
                )
                .color(color),
            )
            .wrap(),
        );
    });
    // Il colore dice lo stato (abbonamento/noleggio/acquisto): l'ember
    // resta per i comandi. La barra segue l'altezza del testo.
    let r = row.response.rect;
    ui.painter().rect_filled(
        egui::Rect::from_min_size(
            egui::pos2(r.left(), r.top()),
            egui::vec2(2.0, r.height()),
        ),
        egui::CornerRadius::ZERO,
        color,
    );
}

/// Una riga di scheda: etichetta a colonna, valore che va a capo.
fn fact(ui: &mut egui::Ui, title: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    ui.add_space(2.0);
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        ui.allocate_ui_with_layout(
            egui::vec2(58.0, 0.0),
            egui::Layout::top_down(egui::Align::Min),
            |ui| {
                ui.label(lumen::label(title));
            },
        );
        ui.add(
            egui::Label::new(lumen::body(11.5, value).color(lumen::MUTED))
                .wrap(),
        );
    });
}

fn dial_value(delta: egui::Vec2) -> u8 {
    let mut angle = delta.y.atan2(delta.x) + std::f32::consts::FRAC_PI_2;
    if angle < 0.0 {
        angle += std::f32::consts::TAU;
    }
    let value = (angle / std::f32::consts::TAU * 10.0).floor() as i32 + 1;
    value.clamp(1, 10) as u8
}

fn status_pill(ui: &mut egui::Ui, status: &str) {
    let (label, color) = match status {
        "completed" => ("VISTO", lumen::CELADON),
        "plantowatch" => ("DA VEDERE", lumen::EMBER),
        "watching" => ("IN CORSO", lumen::EMBER),
        "hold" => ("IN PAUSA", lumen::MUTED),
        "dropped" => ("ABBANDONATO", lumen::DANGER),
        _ => ("SU SIMKL", lumen::MUTED),
    };
    let font = FontId::new(9.0, FontFamily::Name(crate::ui::fonts::LUMEN_MONO.into()));
    let galley = ui.painter().layout_no_wrap(label.to_string(), font, color);
    let size = egui::vec2(galley.size().x + 25.0, 18.0);
    let (rect, _) = ui.allocate_exact_size(size, Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, lumen::radius(lumen::RADIUS), lumen::alpha(color, 24));
    painter.rect_stroke(
        rect,
        lumen::radius(lumen::RADIUS),
        Stroke::new(1.0_f32, lumen::alpha(color, 80)), egui::StrokeKind::Inside);
    painter.circle_filled(
        egui::pos2(rect.left() + 8.0, rect.center().y),
        2.2,
        color,
    );
    painter.galley(
        egui::pos2(
            rect.left() + 14.0,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        color,
    );
}

fn rating_button(
    ui: &mut egui::Ui,
    movie_id: u64,
    track: &TrackInfo,
    width: f32,
    actions: &mut Vec<DetailAction>,
) {
    let popup_id = egui::Id::unique(("rating-open", movie_id));
    let draft_id = egui::Id::unique(("rating-draft", movie_id));
    let mut open = ui.data(|d| d.get_temp::<bool>(popup_id).unwrap_or(false));
    let label = match track.rating {
        Some(value) => format!("★ {value}/10"),
        None => "Vota".to_string(),
    };
    let button = lumen::button_sized(ui, &label, false, width);
    if button.clicked() {
        open = !open;
        if open {
            ui.data_mut(|d| {
                d.insert_temp(draft_id, track.rating.unwrap_or(8));
            });
        }
        ui.data_mut(|d| d.insert_temp(popup_id, open));
    }
    if !open {
        return;
    }

    let popup_w = 210.0;
    let popup_h = 246.0;
    let screen = ui.ctx().content_rect();
    // A sinistra della scheda: così il popover non copre il film.
    let left_x = button.rect.left() - popup_w - 8.0;
    let x = if left_x >= screen.left() + 4.0 {
        left_x
    } else {
        (button.rect.left()).min(screen.right() - popup_w - 4.0)
    };
    let y = (button.rect.bottom() - popup_h)
        .max(screen.top() + 4.0)
        .min(screen.bottom() - popup_h - 4.0);
    let mut draft = ui.data(|d| d.get_temp::<u8>(draft_id).unwrap_or(8));
    // Tastiera: 1–9 e 0 (=10). Non ruba i tasti a un campo di testo.
    if !ui.ctx().egui_wants_keyboard_input() {
        for (key, value) in [
            (egui::Key::Num1, 1),
            (egui::Key::Num2, 2),
            (egui::Key::Num3, 3),
            (egui::Key::Num4, 4),
            (egui::Key::Num5, 5),
            (egui::Key::Num6, 6),
            (egui::Key::Num7, 7),
            (egui::Key::Num8, 8),
            (egui::Key::Num9, 9),
            (egui::Key::Num0, 10),
        ] {
            if ui.ctx().input(|i| i.key_pressed(key)) {
                draft = value;
                ui.data_mut(|d| d.insert_temp(draft_id, draft));
            }
        }
    }
    let mut close = false;
    let mut save = None;
    let mut clear = false;
    let mut popup_rect = None;
    egui::Area::new(egui::Id::unique(("rating-popup", movie_id)))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(x, y))
        .show(ui.ctx(), |ui| {
            egui::Frame::NONE
                .fill(lumen::SURFACE)
                .stroke(Stroke::new(1.0_f32, lumen::BORDER_HI))
                .corner_radius(lumen::radius(lumen::RADIUS))
                .inner_margin(egui::Margin::same(12))
                .show(ui, |ui| {
                    ui.set_width(186.0);
                    ui.horizontal(|ui| {
                        ui.label(lumen::mono(9.0, "IL TUO VOTO").color(lumen::FAINT));
                        ui.with_layout(
                            egui::Layout::right_to_left(egui::Align::Center),
                            |ui| {
                                if lumen::link(ui, "×", lumen::FAINT).clicked() {
                                    close = true;
                                }
                            },
                        );
                    });
                    let (rect, resp) =
                        ui.allocate_exact_size(egui::vec2(186.0, 158.0), Sense::click_and_drag());
                    let center = rect.center();
                    let radius = 62.0;
                    let start = -std::f32::consts::FRAC_PI_2;
                    let end = start + draft as f32 / 10.0 * std::f32::consts::TAU;
                    let painter = ui.painter();
                    painter.circle_stroke(center, radius, Stroke::new(3.0_f32, lumen::BORDER));
                    for i in 0..56 {
                        let a0 = start + (end - start) * i as f32 / 56.0;
                        let a1 = start + (end - start) * (i + 1) as f32 / 56.0;
                        painter.line_segment(
                            [
                                center + egui::vec2(a0.cos(), a0.sin()) * radius,
                                center + egui::vec2(a1.cos(), a1.sin()) * radius,
                            ],
                            Stroke::new(3.0_f32, lumen::EMBER),
                        );
                    }
                    let knob = center + egui::vec2(end.cos(), end.sin()) * radius;
                    painter.circle_filled(knob, 6.0, lumen::SURFACE);
                    painter.circle_stroke(knob, 5.0, Stroke::new(2.0_f32, lumen::EMBER));
                    for i in 0..10u8 {
                        let a = start + i as f32 / 10.0 * std::f32::consts::TAU;
                        let dir = egui::vec2(a.cos(), a.sin());
                        painter.line_segment(
                            [
                                center + dir * (radius + 5.0),
                                center + dir * (radius + 9.0),
                            ],
                            Stroke::new(
                                1.0_f32,
                                if i + 1 == draft {
                                    lumen::EMBER
                                } else {
                                    lumen::BORDER_HI
                                },
                            ),
                        );
                    }
                    painter.text(
                        center + egui::vec2(0.0, -8.0),
                        Align2::CENTER_CENTER,
                        draft.to_string(),
                        FontId::new(
                            38.0,
                            FontFamily::Name(crate::ui::fonts::LUMEN_DISPLAY.into()),
                        ),
                        lumen::IVORY,
                    );
                    painter.text(
                        center + egui::vec2(0.0, 20.0),
                        Align2::CENTER_CENTER,
                        "SU 10",
                        FontId::new(9.0, FontFamily::Name(crate::ui::fonts::LUMEN_MONO.into())),
                        lumen::FAINT,
                    );
                    if resp.clicked() || resp.dragged() {
                        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                            let next = dial_value(pointer - center);
                            if next != draft {
                                draft = next;
                                ui.data_mut(|d| d.insert_temp(draft_id, draft));
                            }
                        }
                    }
                    resp.widget_info(|| {
                        egui::WidgetInfo::labeled(
                            egui::Role::Slider,
                            true,
                            format!("Voto: {draft} su 10"),
                        )
                    });
                    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 10.0;
                        if lumen::button_sized(ui, "Salva", true, 92.0).clicked() {
                            save = Some(draft);
                        }
                        if track.rating.is_some()
                            && lumen::link(ui, "Togli voto", lumen::MUTED).clicked()
                        {
                            clear = true;
                        }
                    });
                });
            popup_rect = Some(ui.min_rect());
        });
    if !button.clicked() {
        if let Some(pos) = ui.ctx().pointer_interact_pos() {
            if ui.ctx().input(|i| i.pointer.any_click()) {
                if let Some(rect) = popup_rect {
                    if !rect.contains(pos) {
                        close = true;
                    }
                }
            }
        }
    }
    if save.is_some() || clear || close || ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
        ui.data_mut(|d| d.insert_temp(popup_id, false));
    }
    if let Some(value) = save {
        actions.push(DetailAction::Rate(value));
    } else if clear {
        actions.push(DetailAction::ClearRating);
    }
}

fn simkl_bar(
    ui: &mut egui::Ui,
    movie_id: u64,
    track: &TrackInfo,
    actions: &mut Vec<DetailAction>,
) {
    let status = track.status.as_deref().unwrap_or("");
    // Stato e fonte restano un solo gruppo a sinistra; a destra c'è solo
    // l'uscita verso SIMKL, senza ripetere il nome.
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        ui.label(lumen::label("SIMKL"));
        if !status.is_empty() {
            status_pill(ui, status);
        }
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            // Stesso margine della testata: la barra non copre “Apri”.
            ui.add_space(10.0);
            if let Some(url) = &track.url {
                if lumen::link(ui, "Apri ↗", lumen::FAINT)
                    .on_hover_text("Apri su SIMKL")
                    .clicked()
                {
                    actions.push(DetailAction::OpenUrl(url.clone()));
                }
            }
        });
    });
    if let Some(err) = &track.error {
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            ui.label(
                lumen::body(11.0, format!("Non salvato su SIMKL: {err}")).color(lumen::DANGER),
            );
            if lumen::link(ui, "Riprova", lumen::EMBER).clicked() {
                actions.push(DetailAction::RetryWrite);
            }
        });
    }
    ui.add_space(8.0);
    if track.busy {
        // La testata resta ferma: cambia solo la riga delle azioni.
        ui.allocate_ui_with_layout(
            egui::vec2(ui.available_width(), 36.0),
            egui::Layout::left_to_right(egui::Align::Center),
            |ui| {
                ui.spinner();
                ui.label(lumen::mono(9.5, "SALVO SU SIMKL…").color(lumen::MUTED));
            },
        );
        return;
    }

    // Le azioni occupano tutta la larghezza della scheda: la riga finisce
    // sul filo destro, senza il vuoto che spezzava il blocco.
    let avail = (ui.available_width() - 10.0).max(0.0);
    let gap = 6.0;
    let inner = (avail - gap * 2.0).max(0.0);
    let rating_w = inner * 0.24;
    let track_w = (avail - rating_w - gap * 2.0).max(0.0) * 0.5;
    let confirm = egui::Id::unique(("simkl-remove-confirm", movie_id));
    let armed = status == "completed"
        && ui
            .ctx()
            .memory(|m| m.data.get_temp::<bool>(confirm).unwrap_or(false));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        rating_button(ui, movie_id, track, rating_w, actions);
        match status {
            "completed" => {
                let remove_w = (avail - rating_w - gap).max(0.0);
                let label = if armed {
                    "Conferma rimozione"
                } else {
                    "Rimuovi dai visti"
                };
                let remove = lumen::button_sized(ui, label, false, remove_w);
                let remove = if armed {
                    remove
                } else {
                    remove
                        .on_hover_text("Rimuove il film dalla cronologia SIMKL e cancella il voto")
                };
                if remove.clicked() {
                    if armed {
                        actions.push(DetailAction::RemoveTracking);
                        ui.ctx().memory_mut(|m| m.data.remove::<bool>(confirm));
                    } else {
                        ui.ctx().memory_mut(|m| m.data.insert_temp(confirm, true));
                    }
                }
            }
            "plantowatch" => {
                if lumen::button_sized(ui, "✓ Da vedere", false, track_w)
                    .on_hover_text("Rimuove il film da Da vedere")
                    .clicked()
                {
                    actions.push(DetailAction::RemoveTracking);
                }
                if lumen::button_sized(ui, "Segna visto", true, track_w).clicked() {
                    actions.push(DetailAction::MarkWatched);
                }
            }
            _ => {
                if lumen::button_sized(ui, "+ Da vedere", false, track_w).clicked() {
                    actions.push(DetailAction::AddWatchlist);
                }
                if lumen::button_sized(ui, "Segna visto", true, track_w).clicked() {
                    actions.push(DetailAction::MarkWatched);
                }
            }
        }
    });
    if armed {
        ui.add_space(6.0);
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            ui.label(lumen::body(11.0, "Toglie cronologia e voto.").color(lumen::MUTED));
            if lumen::link(ui, "Annulla", lumen::FAINT).clicked() {
                ui.ctx().memory_mut(|m| m.data.remove::<bool>(confirm));
            }
        });
    }
}

#[allow(clippy::too_many_arguments)] // scheda densa: contesto e focus restano espliciti
pub fn panel(
    ui: &mut egui::Ui,
    images: &ImageStore,
    d: &Details,
    compact: bool,
    track: &TrackInfo,
    why: Option<&str>,
    consiglio: Option<&str>,
    focus_close: bool,
) -> Vec<DetailAction> {
    let mut actions: Vec<DetailAction> = Vec::new();

    // La scheda copre la griglia: i click nello spazio vuoto non devono
    // aprire il poster che sta dietro.
    let _ = ui.interact(
        ui.max_rect(),
        egui::Id::unique(("detail-bg", d.movie.id)),
        egui::Sense::CLICK,
    );

    // Intestazione fissa: la chiusura resta raggiungibile anche dopo lo scroll.
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), 36.0),
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| {
            ui.label(lumen::mono(10.0, "SCHEDA FILM").color(lumen::IVORY));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Lascia il posto alla barra di scorrimento della scheda.
                ui.add_space(10.0);
                let close = close_button(ui);
                if focus_close {
                    close.request_focus();
                }
                if close.clicked() {
                    actions.push(DetailAction::Close);
                }
            });
        },
    );
    ui.add_space(8.0);
    lumen::hairline(ui);
    ui.add_space(12.0);

    egui::ScrollArea::vertical()
        .id_salt(("detail-scroll", d.movie.id))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = ui.available_width();
            if d.movie.backdrop_path.is_some() && !compact {
                let h = (width * 9.0 / 16.0).clamp(120.0, 210.0);
                lumen::backdrop(ui, images, Some(&d.movie), width, h);
                ui.add_space(12.0);
            }

            let poster_w = (if compact { 92.0_f32 } else { 104.0 }).min(width * 0.34);
            ui.horizontal_top(|ui| {
                let _ = lumen::poster(
                    ui,
                    images,
                    Some(&d.movie),
                    poster_w,
                    poster_w * 1.5,
                    true,
                );
                ui.add_space(12.0);
                ui.vertical(|ui| {
                    ui.label(lumen::display(24.0, &d.movie.title));
                    if !d.movie.original_title.is_empty()
                        && d.movie.original_title != d.movie.title
                    {
                        ui.add_space(2.0);
                        ui.label(
                            lumen::mono(9.0, &d.movie.original_title).color(lumen::FAINT),
                        );
                    }
                    if !d.tagline.is_empty() {
                        ui.add_space(6.0);
                        ui.label(
                            lumen::body(12.5, format!("«{}»", d.tagline))
                                .italics()
                                .color(lumen::MUTED),
                        );
                    }
                    let mut meta: Vec<String> = Vec::new();
                    if let Some(y) = d.movie.year {
                        meta.push(y.to_string());
                    }
                    if let Some(r) = d.runtime {
                        meta.push(format!("{r} min"));
                    }
                    if !d.countries.is_empty() {
                        meta.push(
                            d.countries
                                .iter()
                                .map(|c| short_country(c))
                                .collect::<Vec<_>>()
                                .join(", "),
                        );
                    }
                    if !meta.is_empty() {
                        ui.add_space(if d.tagline.is_empty() { 4.0 } else { 8.0 });
                        ui.label(lumen::mono(9.0, meta.join(" · ")).color(lumen::FAINT));
                    }
                    let mut votes: Vec<String> = Vec::new();
                    if d.movie.votes > 0 {
                        votes.push(format!(
                            "TMDB {:.1} ★ {}",
                            d.movie.rating,
                            compact_votes(d.movie.votes)
                        ));
                    }
                    if let Some(imdb) = d.imdb {
                        if imdb.votes > 0 {
                            votes.push(format!(
                                "IMDb {:.1} ★ {}",
                                imdb.rating,
                                compact_votes(imdb.votes)
                            ));
                        }
                    }
                    if !votes.is_empty() {
                        ui.add_space(3.0);
                        ui.label(lumen::mono(9.0, votes.join(" · ")).color(lumen::MUTED));
                    }
                });
            });

            if track.connected {
                ui.add_space(12.0);
                lumen::hairline(ui);
                ui.add_space(8.0);
                simkl_bar(ui, d.movie.id, track, &mut actions);
            }

            if let Some(perche) = why.filter(|p| !p.trim().is_empty()) {
                ui.add_space(12.0);
                lumen::hairline(ui);
                ui.add_space(10.0);
                ui.label(lumen::label("Perché è qui"));
                ui.add_space(4.0);
                ui.label(lumen::body(12.5, perche).color(lumen::IVORY));
                if let Some(c) = consiglio.filter(|c| !c.trim().is_empty()) {
                    ui.add_space(8.0);
                    let row = ui.horizontal_top(|ui| {
                        ui.add_space(9.0);
                        ui.add(
                            egui::Label::new(
                                lumen::body(11.5, format!("Consiglio · {c}"))
                                    .color(lumen::MUTED),
                            )
                            .wrap(),
                        );
                    });
                    // La barra segue l'altezza del testo: resta piena anche
                    // quando il consiglio va a capo.
                    let r = row.response.rect;
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(
                            egui::pos2(r.left(), r.top()),
                            egui::vec2(2.0, r.height()),
                        ),
                        egui::CornerRadius::ZERO,
                        lumen::EMBER,
                    );
                }
            }

            ui.add_space(12.0);
            lumen::hairline(ui);
            ui.add_space(10.0);

            let text = if d.movie.overview.trim().is_empty() {
                "Nessuna descrizione disponibile in italiano.".to_string()
            } else {
                d.movie.overview.clone()
            };
            ui.label(lumen::body(12.5, text).color(lumen::MUTED));

            let genres = d
                .genres
                .iter()
                .map(|(_, n)| n.clone())
                .collect::<Vec<_>>()
                .join(" · ");
            fact(ui, "Generi", &genres);
            if let Some(dir) = &d.director {
                fact(ui, "Regia", dir);
            }
            if !d.cast.is_empty() {
                fact(ui, "Cast", &label_list(&d.cast));
            }

            let provider = d.provider_link.as_deref().filter(|l| !l.trim().is_empty());
            if !d.flatrate.is_empty()
                || !d.rent.is_empty()
                || !d.buy.is_empty()
                || provider.is_some()
            {
                ui.add_space(12.0);
                lumen::hairline(ui);
                ui.add_space(10.0);
                ui.label(lumen::label("Dove vederlo"));
                ui.add_space(4.0);
                avail_row(ui, "IN ABBONAMENTO", &d.flatrate, lumen::CELADON);
                avail_row(ui, "A NOLEGGIO", &d.rent, lumen::MUTED);
                avail_row(ui, "ACQUISTO", &d.buy, lumen::MUTED);
                if let Some(link) = provider {
                    ui.add_space(8.0);
                    if lumen::link(ui, "Apri il link del provider ↗", lumen::EMBER).clicked() {
                        actions.push(DetailAction::OpenUrl(link.to_string()));
                    }
                }
            }

            ui.add_space(10.0);
            let movie_id = d.movie.id;
            let tmdb_url = format!("https://www.themoviedb.org/movie/{movie_id}?language=it-IT");
            if lumen::link(ui, "Scheda TMDB ↗", lumen::MUTED).clicked() {
                actions.push(DetailAction::OpenUrl(tmdb_url));
            }

            if !d.recommendations.is_empty() {
                ui.add_space(12.0);
                lumen::hairline(ui);
                ui.add_space(10.0);
                ui.label(lumen::label("Simili"));
                ui.add_space(6.0);
                for m in d.recommendations.iter().take(8) {
                    if grid::mini_card(ui, images, m) {
                        actions.push(DetailAction::Open(m.id));
                    }
                    ui.add_space(6.0);
                }
            }
            ui.add_space(16.0);
        });

    actions
}

/// Voti in forma breve: 12345 → "12.3k".
fn compact_votes(v: u32) -> String {
    if v >= 1_000_000 {
        format!("{:.1}M", v as f64 / 1_000_000.0)
    } else if v >= 10_000 {
        format!("{:.0}k", v as f64 / 1_000.0)
    } else if v >= 1_000 {
        format!("{:.1}k", v as f64 / 1_000.0)
    } else {
        v.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::dial_value;

    #[test]
    fn la_ghiera_parte_dall_alto_e_gira_in_senso_orario() {
        assert_eq!(dial_value(egui::vec2(0.0, -10.0)), 1);
        assert_eq!(dial_value(egui::vec2(10.0, 0.0)), 3);
        assert_eq!(dial_value(egui::vec2(0.0, 10.0)), 6);
        assert_eq!(dial_value(egui::vec2(-10.0, 0.0)), 8);
    }
}
