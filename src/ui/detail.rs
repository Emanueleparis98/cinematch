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
    OpenNuvio,
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

fn rating_box(ui: &mut egui::Ui, value: u8, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(20.0, 18.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let fill = if selected {
        lumen::EMBER
    } else if hover > 0.01 {
        lumen::RAISED
    } else {
        lumen::SURFACE
    };
    ui.painter()
        .rect_filled(rect, Rounding::same(lumen::RADIUS_SM), fill);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(lumen::RADIUS_SM),
        Stroke::new(
            1.0_f32,
            if selected {
                lumen::EMBER
            } else {
                lumen::BORDER
            },
        ),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        value.to_string(),
        FontId::new(9.0, FontFamily::Monospace),
        if selected {
            lumen::INK
        } else {
            lumen::MUTED
        },
    );
    lumen::focus(ui, &resp, 0.0);
    resp.on_hover_text(if selected {
        "Togli il voto"
    } else {
        "Dai un voto da 1 a 10"
    })
    .clicked()
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

            if d.movie.imdb_id.is_some() {
                ui.add_space(8.0);
                if lumen::button_sized(ui, "Guarda in Nuvio", false, 168.0).clicked() {
                    actions.push(DetailAction::OpenNuvio);
                }
            }

            if track.connected {
                ui.add_space(10.0);
                ui.label(lumen::mono(9.0, "SIMKL").color(lumen::FAINT));
                if track.busy {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(lumen::mono(9.5, "Salvo su SIMKL…").color(lumen::MUTED));
                    });
                } else {
                    let status = track.status.as_deref().unwrap_or("");
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing.x = 8.0;
                        if status == "completed" {
                            if lumen::link(ui, "Rimuovi dai visti", lumen::MUTED)
                                .on_hover_text(
                                    "Rimuove il film dalla cronologia SIMKL e cancella il voto",
                                )
                                .clicked()
                            {
                                actions.push(DetailAction::RemoveTracking);
                            }
                        } else {
                            if lumen::button_sized(ui, "Segna visto", true, 118.0).clicked() {
                                actions.push(DetailAction::MarkWatched);
                            }
                            if status == "plantowatch" {
                                if lumen::link(ui, "Togli da Da vedere", lumen::MUTED)
                                    .on_hover_text("Rimuove il film dalla lista SIMKL")
                                    .clicked()
                                {
                                    actions.push(DetailAction::RemoveTracking);
                                }
                            } else if lumen::button_sized(ui, "Da vedere", false, 98.0).clicked() {
                                actions.push(DetailAction::AddWatchlist);
                            }
                        }
                    });
                    ui.add_space(4.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.label(lumen::mono(9.0, "IL TUO VOTO").color(lumen::FAINT));
                        for value in 1..=10u8 {
                            if rating_box(ui, value, track.rating == Some(value)) {
                                if track.rating == Some(value) {
                                    actions.push(DetailAction::ClearRating);
                                } else {
                                    actions.push(DetailAction::Rate(value));
                                }
                            }
                        }
                    });
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
