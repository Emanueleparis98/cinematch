use egui::{Align2, FontFamily, FontId, Rounding, Sense, Stroke};

use crate::domain::model::Details;
use crate::ui::images::ImageStore;
use crate::ui::{grid, lumen};

pub enum DetailAction {
    Open(u64),
    Summarize,
    OpenUrl(String),
    Close,
    MarkWatched,
    AddWatchlist,
    RemoveTracking,
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
}

fn label_list(items: &[String]) -> String {
    items.join(" · ")
}

fn close_button(ui: &mut egui::Ui) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(lumen::RADIUS_SM),
        Stroke::new(
            1.0_f32,
            if hover > 0.01 {
                lumen::EMBER
            } else {
                lumen::BORDER
            },
        ),
    );
    let c = rect.center();
    let color = lumen::mix(lumen::MUTED, lumen::IVORY, hover);
    ui.painter().line_segment(
        [c + egui::vec2(-4.0, -4.0), c + egui::vec2(4.0, 4.0)],
        Stroke::new(1.2_f32, color),
    );
    ui.painter().line_segment(
        [c + egui::vec2(4.0, -4.0), c + egui::vec2(-4.0, 4.0)],
        Stroke::new(1.2_f32, color),
    );
    lumen::focus(ui, &resp, lumen::RADIUS_SM);
    resp.on_hover_text("Chiudi (Esc)")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
}

fn avail_row(ui: &mut egui::Ui, label: &str, items: &[String], color: egui::Color32) {
    if items.is_empty() {
        return;
    }
    ui.horizontal_top(|ui| {
        let (bar, _) = ui.allocate_exact_size(egui::vec2(2.0, 26.0), Sense::hover());
        ui.painter().rect_filled(bar, Rounding::ZERO, lumen::EMBER);
        ui.add_space(7.0);
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
}

fn field(ui: &mut egui::Ui, title: &str, value: &str) {
    if value.is_empty() {
        return;
    }
    ui.add_space(8.0);
    ui.label(lumen::label(title));
    ui.add_space(2.0);
    ui.label(lumen::body(11.5, value).color(lumen::MUTED));
}

fn dial_value(delta: egui::Vec2) -> u8 {
    let mut angle = delta.y.atan2(delta.x) + std::f32::consts::FRAC_PI_2;
    if angle < 0.0 {
        angle += std::f32::consts::TAU;
    }
    let value = (angle / std::f32::consts::TAU * 10.0).floor() as i32 + 1;
    value.clamp(1, 10) as u8
}

fn rating_button(
    ui: &mut egui::Ui,
    movie_id: u64,
    track: &TrackInfo,
    actions: &mut Vec<DetailAction>,
) {
    let popup_id = egui::Id::new(("rating-open", movie_id));
    let draft_id = egui::Id::new(("rating-draft", movie_id));
    let mut open = ui.data(|d| d.get_temp::<bool>(popup_id).unwrap_or(false));
    let label = match track.rating {
        Some(value) => format!("★ {value}/10"),
        None => "Vota".to_string(),
    };
    let button = lumen::button_sized(ui, &label, false, 96.0);
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

    let screen = ui.ctx().screen_rect();
    let mut x = button.rect.left();
    if x + 216.0 > screen.right() {
        x = screen.right() - 216.0;
    }
    let mut y = button.rect.bottom() + 6.0;
    if y + 262.0 > screen.bottom() {
        y = (button.rect.top() - 262.0).max(screen.top() + 4.0);
    }
    let mut draft = ui.data(|d| d.get_temp::<u8>(draft_id).unwrap_or(8));
    let mut close = false;
    let mut save = None;
    let mut clear = false;
    egui::Area::new(egui::Id::new(("rating-popup", movie_id)))
        .order(egui::Order::Foreground)
        .fixed_pos(egui::pos2(x, y))
        .show(ui.ctx(), |ui| {
            egui::Frame::none()
                .fill(lumen::SURFACE)
                .stroke(Stroke::new(1.0_f32, lumen::BORDER_HI))
                .rounding(Rounding::same(lumen::RADIUS))
                .inner_margin(egui::Margin::same(12.0))
                .show(ui, |ui| {
                    ui.set_width(184.0);
                    ui.label(lumen::mono(9.0, "IL TUO VOTO").color(lumen::FAINT));
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(184.0, 168.0),
                        Sense::click_and_drag(),
                    );
                    let center = rect.center();
                    let radius = 58.0;
                    let painter = ui.painter();
                    painter.circle_stroke(
                        center,
                        radius,
                        Stroke::new(1.0_f32, lumen::BORDER),
                    );
                    for value in 1..=10u8 {
                        let angle = -std::f32::consts::FRAC_PI_2
                            + (value - 1) as f32 * std::f32::consts::TAU / 10.0;
                        let dir = egui::vec2(angle.cos(), angle.sin());
                        let selected = value == draft;
                        painter.circle_filled(
                            center + dir * radius,
                            if selected { 5.0 } else { 3.0 },
                            if selected { lumen::EMBER } else { lumen::BORDER_HI },
                        );
                        painter.text(
                            center + dir * (radius + 15.0),
                            Align2::CENTER_CENTER,
                            value.to_string(),
                            FontId::new(8.0, FontFamily::Name(crate::ui::fonts::LUMEN_MONO.into())),
                            if selected { lumen::EMBER } else { lumen::FAINT },
                        );
                    }
                    if resp.clicked() || resp.dragged() {
                        if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                            let next = dial_value(pointer - center);
                            if next != draft {
                                draft = next;
                                ui.data_mut(|d| d.insert_temp(draft_id, draft));
                            }
                        }
                    }
                    ui.painter().text(
                        center,
                        Align2::CENTER_CENTER,
                        draft.to_string(),
                        FontId::new(
                            30.0,
                            FontFamily::Name(crate::ui::fonts::LUMEN_DISPLAY.into()),
                        ),
                        lumen::IVORY,
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 6.0;
                        if lumen::link(ui, "−", lumen::MUTED).clicked() {
                            draft = draft.saturating_sub(1).max(1);
                            ui.data_mut(|d| d.insert_temp(draft_id, draft));
                        }
                        ui.label(lumen::mono(10.0, format!("{draft}/10")).color(lumen::MUTED));
                        if lumen::link(ui, "+", lumen::MUTED).clicked() {
                            draft = (draft + 1).min(10);
                            ui.data_mut(|d| d.insert_temp(draft_id, draft));
                        }
                    });
                    ui.add_space(6.0);
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if lumen::button_sized(ui, "Salva", true, 86.0).clicked() {
                            save = Some(draft);
                        }
                        if track.rating.is_some()
                            && lumen::link(ui, "Togli voto", lumen::MUTED).clicked()
                        {
                            clear = true;
                        }
                    });
                    if lumen::link(ui, "Chiudi", lumen::MUTED).clicked() {
                        close = true;
                    }
                });
        });
    if save.is_some() || clear || close || ui.ctx().input(|i| i.key_pressed(egui::Key::Escape)) {
        ui.data_mut(|d| d.insert_temp(popup_id, false));
    }
    if let Some(value) = save {
        actions.push(DetailAction::Rate(value));
    } else if clear {
        actions.push(DetailAction::ClearRating);
    }
}

fn simkl_actions(
    ui: &mut egui::Ui,
    movie_id: u64,
    track: &TrackInfo,
    actions: &mut Vec<DetailAction>,
) {
    let status = track.status.as_deref().unwrap_or("");
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 8.0;
        if status == "completed" {
            if lumen::link(ui, "Rimuovi dai visti", lumen::MUTED)
                .on_hover_text("Rimuove il film dalla cronologia SIMKL e cancella il voto")
                .clicked()
            {
                actions.push(DetailAction::RemoveTracking);
            }
        } else if status == "plantowatch" {
            if lumen::button_sized(ui, "✓ Da vedere", false, 118.0)
                .on_hover_text("Rimuove il film da Da vedere")
                .clicked()
            {
                actions.push(DetailAction::RemoveTracking);
            }
            if lumen::button_sized(ui, "Segna visto", true, 112.0).clicked() {
                actions.push(DetailAction::MarkWatched);
            }
        } else {
            if lumen::button_sized(ui, "+ Da vedere", false, 118.0).clicked() {
                actions.push(DetailAction::AddWatchlist);
            }
            if lumen::button_sized(ui, "Segna visto", true, 112.0).clicked() {
                actions.push(DetailAction::MarkWatched);
            }
        }
        rating_button(ui, movie_id, track, actions);
    });
}

pub fn panel(
    ui: &mut egui::Ui,
    images: &ImageStore,
    d: &Details,
    summary: Option<&str>,
    summarizing: bool,
    ai_available: bool,
    compact: bool,
    track: &TrackInfo,
) -> Vec<DetailAction> {
    let mut actions: Vec<DetailAction> = Vec::new();

    // Intestazione fissa: la chiusura resta raggiungibile anche dopo lo scroll.
    ui.horizontal(|ui| {
        ui.label(lumen::mono(10.0, "SCHEDA FILM").color(lumen::IVORY));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if close_button(ui) {
                actions.push(DetailAction::Close);
            }
        });
    });
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
                    let mut meta: Vec<String> = Vec::new();
                    if let Some(y) = d.movie.year {
                        meta.push(y.to_string());
                    }
                    if let Some(r) = d.runtime {
                        meta.push(format!("{r} min"));
                    }
                    if !d.countries.is_empty() {
                        meta.push(d.countries.join(", "));
                    }
                    if !meta.is_empty() {
                        ui.add_space(4.0);
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
                        ui.add_space(2.0);
                        ui.label(
                            lumen::mono(9.0, votes.join(" · ")).color(lumen::MUTED),
                        );
                    }
                    if let Some(status) = track.status.as_deref().filter(|s| !s.is_empty()) {
                        ui.add_space(4.0);
                        let (label, color) = match status {
                            "completed" => ("VISTO SU SIMKL", lumen::CELADON),
                            "plantowatch" => ("DA VEDERE SU SIMKL", lumen::EMBER),
                            _ => ("SU SIMKL", lumen::MUTED),
                        };
                        ui.horizontal(|ui| {
                            ui.label(lumen::mono(9.0, label).color(color));
                        });
                        if let Some(url) = &track.url {
                            if lumen::link(ui, "Apri su SIMKL", lumen::MUTED).clicked() {
                                actions.push(DetailAction::OpenUrl(url.clone()));
                            }
                        }
                    }
                    if track.connected {
                        ui.add_space(6.0);
                        if track.busy {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(
                                    lumen::mono(9.5, "Salvo su SIMKL…").color(lumen::MUTED),
                                );
                            });
                        } else {
                            simkl_actions(ui, d.movie.id, track, &mut actions);
                        }
                    }
                });
            });

            if !d.tagline.is_empty() {
                ui.add_space(10.0);
                ui.label(
                    lumen::body(12.5, format!("«{}»", d.tagline))
                        .italics()
                        .color(lumen::EMBER),
                );
            }

            ui.add_space(12.0);
            let ai_w = (ui.available_width() - 148.0).clamp(96.0, 140.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                ui.spacing_mut().item_spacing.y = 8.0;
                if lumen::button_sized(ui, "Cerca il titolo", true, 140.0).clicked() {
                    actions.push(DetailAction::Open(d.movie.id));
                }
                let ai_btn = ui
                    .add_enabled_ui(
                        ai_available && !summarizing && !d.movie.overview.is_empty(),
                        |ui| {
                            lumen::button_sized(
                                ui,
                                if summarizing {
                                    "Riassumo…"
                                } else {
                                    "Riassumi"
                                },
                                false,
                                ai_w,
                            )
                        },
                    )
                    .inner;
                if ai_btn.clicked() {
                    actions.push(DetailAction::Summarize);
                }
            });
            if let Some(link) = &d.provider_link {
                ui.add_space(6.0);
                if lumen::link(ui, "Dove vederlo ↗", lumen::EMBER).clicked() {
                    actions.push(DetailAction::OpenUrl(link.clone()));
                }
            }

            ui.add_space(12.0);
            lumen::hairline(ui);
            ui.add_space(10.0);

            match summary {
                Some(s) => {
                    ui.label(lumen::mono(9.0, "SINTESI AI").color(lumen::EMBER));
                    ui.add_space(4.0);
                    ui.label(lumen::body(13.0, s));
                }
                None => {
                    let text = if d.movie.overview.trim().is_empty() {
                        "Nessuna descrizione disponibile in italiano.".to_string()
                    } else {
                        d.movie.overview.clone()
                    };
                    ui.label(lumen::body(12.5, text).color(lumen::MUTED));
                }
            }

            let genres = d
                .genres
                .iter()
                .map(|(_, n)| n.clone())
                .collect::<Vec<_>>()
                .join(" · ");
            field(ui, "Generi", &genres);
            if let Some(dir) = &d.director {
                field(ui, "Regia", dir);
            }
            if !d.cast.is_empty() {
                field(ui, "Cast", &label_list(&d.cast));
            }

            if !d.flatrate.is_empty() || !d.rent.is_empty() || !d.buy.is_empty() {
                ui.add_space(12.0);
                lumen::hairline(ui);
                ui.add_space(10.0);
                ui.label(lumen::label("Dove vederlo"));
                ui.add_space(4.0);
                avail_row(ui, "IN ABBONAMENTO", &d.flatrate, lumen::CELADON);
                avail_row(ui, "A NOLEGGIO", &d.rent, lumen::MUTED);
                avail_row(ui, "ACQUISTO", &d.buy, lumen::MUTED);
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