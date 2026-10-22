use egui::RichText;

use crate::domain::model::Details;
use crate::tmdb::poster_url;
use crate::ui::images::ImageStore;
use crate::ui::{fx, grid, theme};

pub enum DetailAction {
    Open(u64),
    Summarize,
    OpenUrl(String),
    Close,
}

fn badge(ui: &mut egui::Ui, text: &str, color: egui::Color32) {
    egui::Frame::none()
        .fill(color)
        .rounding(egui::Rounding::same(6.0))
        .inner_margin(egui::Margin::symmetric(6.0, 2.0))
        .show(ui, |ui| {
            ui.label(RichText::new(text).size(10.5).color(egui::Color32::WHITE).strong());
        });
}

fn label_list(items: &[String]) -> String {
    items.join(" · ")
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
        ui.label(RichText::new("SCHEDA FILM").size(11.0).strong().color(theme::DIM));
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if ui
                .add(egui::Button::new(RichText::new("×").size(16.0)).frame(false))
                .on_hover_text("Chiudi (Esc)")
                .clicked()
            {
                actions.push(DetailAction::Close);
            }
        });
    });

    egui::ScrollArea::vertical()
        .id_salt(("detail-scroll", d.movie.id))
        .auto_shrink([false, false])
        .show(ui, |ui| {
            let width = ui.available_width();
            let (path, size, height) = match &d.movie.backdrop_path {
                Some(p) => (p.clone(), "w780", (width * 9.0 / 16.0).min(230.0)),
                None => match &d.movie.poster_path {
                    Some(p) => (p.clone(), "w500", (width * 1.45).min(330.0)),
                    None => (String::new(), "w92", 0.0),
                },
            };
            if height > 0.0 {
                let (rect, _) =
                    ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
                let uri = poster_url(&path, size);
                match images.texture(ui.ctx(), &uri) {
                    Some(tex) => {
                        let sized = egui::load::SizedTexture::new(tex.id(), rect.size());
                        ui.put(
                            rect,
                            egui::Image::new(sized).rounding(egui::Rounding::same(14.0)),
                        );
                    }
                    None => {
                        fx::shimmer(ui, rect, 14.0, ui.ctx().input(|i| i.time));
                    }
                }
                // Sfumatura in basso: l'immagine si fonde con il pannello.
                let fade_h = (rect.height() * 0.45).max(40.0);
                let fade = egui::Rect::from_min_max(
                    egui::pos2(rect.left(), rect.bottom() - fade_h),
                    rect.right_bottom(),
                );
                let mut mesh = egui::Mesh::default();
                mesh.colored_vertex(fade.left_top(), egui::Color32::TRANSPARENT);
                mesh.colored_vertex(fade.right_top(), egui::Color32::TRANSPARENT);
                mesh.colored_vertex(fade.left_bottom(), theme::alpha(theme::PANEL, 190));
                mesh.colored_vertex(fade.right_bottom(), theme::alpha(theme::PANEL, 190));
                mesh.add_triangle(0, 1, 2);
                mesh.add_triangle(1, 2, 3);
                ui.painter().add(mesh);
            }

            ui.add_space(8.0);
            ui.label(RichText::new(&d.movie.title).size(20.0).strong());
            if !d.movie.original_title.is_empty() && d.movie.original_title != d.movie.title {
                ui.label(
                    RichText::new(&d.movie.original_title)
                        .size(12.0)
                        .italics()
                        .color(theme::DIM),
                );
            }

            let mut meta: Vec<String> = Vec::new();
            if let Some(y) = d.movie.year {
                meta.push(y.to_string());
            }
            if let Some(r) = d.runtime {
                meta.push(format!("{r} min"));
            }
            if d.movie.votes > 0 {
                meta.push(format!("TMDB {:.1} ★ · {} voti", d.movie.rating, d.movie.votes));
            }
            if let Some(imdb) = d.imdb {
                if imdb.votes > 0 {
                    meta.push(format!(
                        "IMDb {:.1} ★ · {} voti",
                        imdb.rating,
                        compact_votes(imdb.votes)
                    ));
                }
            }
            if !d.countries.is_empty() {
                meta.push(d.countries.join(", "));
            }
            ui.label(RichText::new(meta.join("   ·   ")).size(12.0).color(theme::DIM));

            if !d.tagline.is_empty() {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(format!("«{}»", d.tagline))
                        .size(12.5)
                        .italics()
                        .color(theme::ACCENT),
                );
            }

            ui.add_space(6.0);
            ui.horizontal_wrapped(|ui| {
                if ui
                    .add(
                        egui::Button::new(RichText::new("Cerca il titolo").size(12.0))
                            .fill(theme::ACCENT)
                            .rounding(egui::Rounding::same(10.0)),
                    )
                    .on_hover_text("Mette il titolo nella barra di ricerca e riparte da capo")
                    .clicked()
                {
                    actions.push(DetailAction::Open(d.movie.id));
                }
                let ai_btn = ui.add_enabled(
                    ai_available && !summarizing && !d.movie.overview.is_empty(),
                    egui::Button::new(RichText::new(if summarizing {
                        "Riassumo…"
                    } else {
                        "Riassumi con AI"
                    })
                    .size(12.0))
                    .rounding(egui::Rounding::same(10.0)),
                );
                if ai_btn.clicked() {
                    actions.push(DetailAction::Summarize);
                }
                if let Some(link) = &d.provider_link {
                    if ui
                        .add(
                            egui::Button::new(RichText::new("Dove vederlo").size(12.0))
                                .rounding(egui::Rounding::same(10.0)),
                        )
                        .clicked()
                    {
                        actions.push(DetailAction::OpenUrl(link.clone()));
                    }
                }
            });

            ui.add_space(8.0);
            match summary {
                Some(s) => {
                    ui.horizontal(|ui| {
                        badge(ui, "AI", theme::ACCENT);
                        ui.label(RichText::new("sintesi").size(10.5).color(theme::DIM));
                    });
                    ui.add_space(2.0);
                    ui.label(RichText::new(s).size(13.0));
                }
                None => {
                    let text = if d.movie.overview.trim().is_empty() {
                        "Nessuna descrizione disponibile in italiano.".to_string()
                    } else {
                        d.movie.overview.clone()
                    };
                    ui.label(RichText::new(text).size(13.0));
                }
            }

            if !d.genres.is_empty() {
                ui.add_space(6.0);
                ui.label(
                    RichText::new(
                        d.genres
                            .iter()
                            .map(|(_, n)| n.clone())
                            .collect::<Vec<_>>()
                            .join("  ·  "),
                    )
                    .size(11.5)
                    .color(theme::DIM),
                );
            }
            if let Some(dir) = &d.director {
                ui.label(
                    RichText::new(format!("Regia: {dir}"))
                        .size(11.5)
                        .color(theme::DIM),
                );
            }
            if !d.cast.is_empty() {
                ui.label(
                    RichText::new(format!("Cast: {}", label_list(&d.cast)))
                        .size(11.5)
                        .color(theme::DIM),
                );
            }

            if !d.flatrate.is_empty() || !d.rent.is_empty() || !d.buy.is_empty() {
                ui.add_space(6.0);
                ui.separator();
                if !d.flatrate.is_empty() {
                    ui.label(
                        RichText::new(format!("In abbonamento: {}", label_list(&d.flatrate)))
                            .size(11.5)
                            .color(theme::OK),
                    );
                }
                if !d.rent.is_empty() {
                    ui.label(
                        RichText::new(format!("A noleggio: {}", label_list(&d.rent)))
                            .size(11.5)
                            .color(theme::DIM),
                    );
                }
                if !d.buy.is_empty() {
                    ui.label(
                        RichText::new(format!("Acquisto: {}", label_list(&d.buy)))
                            .size(11.5)
                            .color(theme::DIM),
                    );
                }
            }

            ui.add_space(4.0);
            let movie_id = d.movie.id;
            if ui
                .hyperlink_to(
                    RichText::new("Scheda TMDB ↗").size(11.5),
                    format!("https://www.themoviedb.org/movie/{movie_id}?language=it-IT"),
                )
                .clicked()
            {
                actions.push(DetailAction::OpenUrl(format!(
                    "https://www.themoviedb.org/movie/{movie_id}?language=it-IT"
                )));
            }

            if !d.recommendations.is_empty() {
                ui.add_space(8.0);
                ui.separator();
                ui.label(RichText::new("SIMILI").size(11.0).strong().color(theme::DIM));
                ui.add_space(2.0);
                for m in d.recommendations.iter().take(8) {
                    if grid::mini_card(ui, images, m) {
                        actions.push(DetailAction::Open(m.id));
                    }
                }
            }
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
