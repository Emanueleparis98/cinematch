use egui::{Align2, Color32, FontId, Rect, Rounding, Sense};

use crate::domain::model::Movie;
use crate::tmdb::poster_url;
use crate::ui::images::ImageStore;
use crate::ui::{fx, theme};

pub const CARD_W: f32 = 152.0;
pub const CARD_H: f32 = 288.0;
const POSTER_H: f32 = 224.0;
const GAP: f32 = 12.0;

/// Dipinge una texture arrotondata dentro `rect`, se disponibile.
pub fn paint_image(
    ui: &mut egui::Ui,
    images: &ImageStore,
    uri: &str,
    rect: Rect,
    rounding: f32,
) {
    if let Some(tex) = images.texture(ui.ctx(), uri) {
        let sized = egui::load::SizedTexture::new(tex.id(), rect.size());
        ui.put(rect, egui::Image::new(sized).rounding(Rounding::same(rounding)));
    }
}

fn card(ui: &mut egui::Ui, images: &ImageStore, m: &Movie, selected: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(CARD_W, CARD_H), Sense::click());
    let hovered = resp.hovered();
    let hover = ui.ctx().animate_bool_with_time(resp.id, hovered, 0.14);
    let bg = if selected {
        theme::mix(theme::ACCENT_DIM, theme::ACCENT, 0.10)
    } else {
        theme::mix(theme::CARD, theme::CARD_HOVER, hover)
    };
    let poster_rect = Rect::from_min_size(
        rect.min + egui::vec2(4.0, 4.0),
        egui::vec2(CARD_W - 8.0, POSTER_H),
    );
    ui.painter().rect_filled(rect, Rounding::same(12.0), bg);
    // I poster arrivano a rate: finché l'immagine non c'è, shimmer.
    let mut loaded = false;
    if let Some(path) = &m.poster_path {
        let uri = poster_url(path, "w342");
        if let Some(tex) = images.texture(ui.ctx(), &uri) {
            let sized = egui::load::SizedTexture::new(tex.id(), poster_rect.size());
            ui.put(
                poster_rect,
                egui::Image::new(sized).rounding(Rounding::same(9.0)),
            );
            loaded = true;
        }
    }
    if !loaded {
        let now = ui.ctx().input(|i| i.time);
        fx::shimmer(ui, poster_rect, 9.0, now);
    }
    // Voto sul poster: compare al passaggio del mouse.
    if hover > 0.02 && m.votes > 0 {
        let g = ui.painter().layout(
            format!("★ {:.1}", m.rating),
            FontId::proportional(10.0),
            Color32::WHITE,
            f32::INFINITY,
        );
        let size = g.size() + egui::vec2(10.0, 4.0);
        let chip = Rect::from_min_size(
            egui::pos2(poster_rect.right() - size.x - 5.0, poster_rect.top() + 5.0),
            size,
        );
        let p = ui.painter();
        p.rect_filled(
            chip,
            Rounding::same(7.0),
            Color32::from_black_alpha((165.0 * hover) as u8),
        );
        p.galley(chip.min + egui::vec2(5.0, 2.0), g, Color32::WHITE);
    }
    fx::ring(
        ui,
        poster_rect,
        9.0,
        theme::ACCENT,
        if selected { 1.0 } else { hover },
    );
    theme::focus_ring(ui, &resp, 12.0);
    let painter = ui.painter();
    let text_x = rect.min.x + 8.0;
    let title_y = poster_rect.max.y + 8.0;
    painter.text(
        egui::pos2(text_x, title_y),
        Align2::LEFT_TOP,
        theme::truncate(&m.title, 21),
        FontId::proportional(13.0),
        if hovered { Color32::WHITE } else { theme::TEXT },
    );
    let year = m
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "—".to_string());
    let sub = if m.votes > 0 {
        format!("{year}  ·  {:.1} ★", m.rating)
    } else {
        year
    };
    painter.text(
        egui::pos2(text_x, title_y + 19.0),
        Align2::LEFT_TOP,
        sub,
        FontId::proportional(11.5),
        theme::DIM,
    );
    resp.clicked()
}

/// Griglia virtualizzata di poster. Ritorna l'id del film cliccato.
pub fn grid(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    selected: Option<u64>,
    reset_scroll: bool,
) -> Option<u64> {
    let mut clicked = None;
    let avail = ui.available_width();
    let cols = (((avail + GAP) / (CARD_W + GAP)).floor() as usize).max(1);
    let row_h = CARD_H + GAP;
    let rows = movies.len().div_ceil(cols);
    let mut area = egui::ScrollArea::vertical()
        .id_salt("grid-scroll")
        .auto_shrink([false, false]);
    if reset_scroll {
        area = area.vertical_scroll_offset(0.0);
    }
    area.show_rows(ui, row_h, rows, |ui, range| {
        for row in range {
            ui.horizontal(|ui| {
                // La formula delle colonne conta GAP: qui lo spacing deve
                // essere lo stesso, altrimenti l'ultima card esce a destra.
                ui.spacing_mut().item_spacing.x = GAP;
                for col in 0..cols {
                    let idx = row * cols + col;
                    match movies.get(idx) {
                        Some(m) => {
                            if card(ui, images, m, selected == Some(m.id)) {
                                clicked = Some(m.id);
                            }
                        }
                        None => {
                            ui.allocate_exact_size(
                                egui::vec2(CARD_W, CARD_H),
                                Sense::hover(),
                            );
                        }
                    }
                }
            });
        }
    });
    clicked
}

/// Riga compatta, usata per i film consigliati nel pannello dettaglio.
pub fn mini_card(ui: &mut egui::Ui, images: &ImageStore, m: &Movie) -> bool {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 62.0), Sense::click());
    let hovered = resp.hovered();
    let hover = ui.ctx().animate_bool_with_time(resp.id, hovered, 0.14);
    ui.painter().rect_filled(
        rect,
        Rounding::same(10.0),
        theme::mix(theme::CARD, theme::CARD_HOVER, hover),
    );
    fx::ring(ui, rect, 10.0, theme::ACCENT, hover * 0.8);
    theme::focus_ring(ui, &resp, 10.0);
    let thumb = Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(38.0, 54.0));
    if let Some(path) = &m.poster_path {
        let uri = poster_url(path, "w92");
        paint_image(ui, images, &uri, thumb, 6.0);
    }
    let painter = ui.painter();
    painter.text(
        egui::pos2(thumb.max.x + 8.0, rect.min.y + 12.0),
        Align2::LEFT_TOP,
        theme::truncate(&m.title, 26),
        FontId::proportional(12.5),
        if hovered { Color32::WHITE } else { theme::TEXT },
    );
    let year = m
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "—".to_string());
    painter.text(
        egui::pos2(thumb.max.x + 8.0, rect.min.y + 32.0),
        Align2::LEFT_TOP,
        format!("{year}  ·  {:.1} ★", m.rating),
        FontId::proportional(11.0),
        theme::DIM,
    );
    resp.clicked()
}
