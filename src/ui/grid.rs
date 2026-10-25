use egui::text::LayoutJob;
use egui::{Align2, Color32, FontFamily, FontId, Rect, Rounding, Sense, Stroke};

use crate::domain::model::Movie;
use crate::tmdb::poster_url;
use crate::ui::fonts;
use crate::ui::images::ImageStore;
use crate::ui::{fx, lumen, theme};

const GAP: f32 = 14.0;

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

fn trunc_job(text: &str, font: FontId, width: f32, rows: usize) -> LayoutJob {
    let mut job = LayoutJob::simple(
        text.to_string(),
        font,
        Color32::PLACEHOLDER,
        width.max(40.0),
    );
    job.wrap.max_rows = rows;
    job.wrap.overflow_character = Some('…');
    job
}

/// Card editoriale: poster 2:3, titolo e metadati su fondo pagina, senza
/// riquadro: la selezione è un filo ember attorno al poster.
fn card(
    ui: &mut egui::Ui,
    images: &ImageStore,
    m: &Movie,
    selected: bool,
    width: f32,
    height: f32,
) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    let hovered = resp.hovered();
    let hover = ui.ctx().animate_bool_with_time(resp.id, hovered, 0.14);
    let poster_h = (width * 1.5).floor();
    let poster_rect = Rect::from_min_size(rect.min, egui::vec2(width, poster_h));
    ui.painter().rect_filled(poster_rect, Rounding::ZERO, lumen::DEEP);
    // I poster arrivano a rate: finché l'immagine non c'è, shimmer.
    let mut loaded = false;
    if let Some(path) = &m.poster_path {
        let uri = poster_url(path, "w342");
        if let Some(tex) = images.texture(ui.ctx(), &uri) {
            let sized = egui::load::SizedTexture::new(tex.id(), poster_rect.size());
            ui.put(poster_rect, egui::Image::new(sized));
            loaded = true;
        }
    }
    if !loaded {
        let now = ui.ctx().input(|i| i.time);
        fx::shimmer(ui, poster_rect, 0.0, now);
    }
    let border = if selected {
        lumen::EMBER
    } else if hover > 0.05 {
        lumen::BORDER_HI
    } else {
        lumen::BORDER
    };
    ui.painter().rect_stroke(
        poster_rect,
        Rounding::ZERO,
        Stroke::new(1.0_f32, border),
    );
    lumen::focus(ui, &resp, 0.0);

    let title = ui.painter().layout_job(trunc_job(
        &m.title,
        FontId::new(12.0, FontFamily::Name(fonts::LUMEN_UI_BOLD.into())),
        width,
        1,
    ));
    ui.painter().galley(
        egui::pos2(rect.left(), poster_rect.bottom() + 6.0),
        title,
        lumen::IVORY,
    );
    let year = m
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "—".to_string());
    let sub = if m.votes > 0 {
        format!("{year} · {:.1} ★", m.rating)
    } else {
        year
    };
    ui.painter().text(
        egui::pos2(rect.left(), poster_rect.bottom() + 23.0),
        Align2::LEFT_TOP,
        sub,
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        if hovered { lumen::MUTED } else { lumen::FAINT },
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
    let cols = if avail >= 1040.0 {
        5
    } else if avail >= 740.0 {
        4
    } else if avail >= 560.0 {
        3
    } else {
        2
    };
    let card_w = ((avail - GAP * (cols - 1) as f32) / cols as f32).floor();
    let card_h = (card_w * 1.5).floor() + 38.0;
    let rows = movies.len().div_ceil(cols);
    ui.spacing_mut().item_spacing.y = GAP;
    let mut area = egui::ScrollArea::vertical()
        .id_salt("grid-scroll")
        .auto_shrink([false, false]);
    if reset_scroll {
        area = area.vertical_scroll_offset(0.0);
    }
    area.show_rows(ui, card_h, rows, |ui, range| {
        for row in range {
            ui.horizontal(|ui| {
                // La formula delle colonne conta GAP: qui lo spacing deve
                // essere lo stesso, altrimenti l'ultima card esce a destra.
                ui.spacing_mut().item_spacing.x = GAP;
                for col in 0..cols {
                    let idx = row * cols + col;
                    match movies.get(idx) {
                        Some(m) => {
                            if card(ui, images, m, selected == Some(m.id), card_w, card_h) {
                                clicked = Some(m.id);
                            }
                        }
                        None => {
                            ui.allocate_exact_size(
                                egui::vec2(card_w, card_h),
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
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 72.0), Sense::click());
    let hovered = resp.hovered();
    let hover = ui.ctx().animate_bool_with_time(resp.id, hovered, 0.14);
    ui.painter().rect_filled(
        rect,
        Rounding::same(lumen::RADIUS_SM),
        theme::mix(lumen::SURFACE, lumen::RAISED, hover),
    );
    fx::ring(ui, rect, lumen::RADIUS_SM, lumen::EMBER, hover * 0.8);
    lumen::focus(ui, &resp, lumen::RADIUS_SM);
    let thumb = Rect::from_min_size(rect.min + egui::vec2(4.0, 4.0), egui::vec2(43.0, 64.0));
    if let Some(path) = &m.poster_path {
        let uri = poster_url(path, "w92");
        paint_image(ui, images, &uri, thumb, 0.0);
    }
    let painter = ui.painter();
    let title = painter.layout_job(trunc_job(
        &m.title,
        FontId::new(12.0, FontFamily::Name(fonts::LUMEN_UI_BOLD.into())),
        width - 61.0,
        2,
    ));
    let title_h = title.size().y;
    let block_h = title_h + 3.0 + 11.0;
    let text_y = (rect.center().y - block_h * 0.5).max(rect.top() + 6.0);
    let title_pos = egui::pos2(thumb.max.x + 8.0, text_y);
    painter.galley(title_pos, title, lumen::IVORY);
    let year = m
        .year
        .map(|y| y.to_string())
        .unwrap_or_else(|| "—".to_string());
    painter.text(
        title_pos + egui::vec2(0.0, title_h + 3.0),
        Align2::LEFT_TOP,
        format!("{year} · {:.1} ★", m.rating),
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        lumen::FAINT,
    );
    resp.clicked()
}
