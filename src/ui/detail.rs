use egui::{Rounding, Sense, Stroke};

use crate::domain::model::Details;
use crate::ui::images::ImageStore;
use crate::ui::{grid, lumen};

pub enum DetailAction {
    Open(u64),
    Summarize,
    OpenUrl(String),
    Close,
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

pub fn panel(
    ui: &mut egui::Ui,
    images: &ImageStore,
    d: &Details,
    summary: Option<&str>,
    summarizing: bool,
    ai_available: bool,
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
            if d.movie.backdrop_path.is_some() {
                let h = (width * 9.0 / 16.0).clamp(120.0, 210.0);
                lumen::backdrop(ui, images, Some(&d.movie), width, h);
                ui.add_space(12.0);
            }

            let poster_w = 104.0_f32.min(width * 0.34);
            ui.horizontal_top(|ui| {
                let resp = lumen::poster(
                    ui,
                    images,
                    Some(&d.movie),
                    poster_w,
                    poster_w * 1.5,
                    true,
                );
                if resp.clicked() {
                    actions.push(DetailAction::Open(d.movie.id));
                }
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
