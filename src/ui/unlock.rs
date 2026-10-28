//! Striscia «Sblocca»: i compromessi minimi quando la griglia è quasi vuota.

use egui::text::LayoutJob;
use egui::{Align2, Color32, FontFamily, FontId, Sense, Stroke};

use crate::domain::model::Facets;
use crate::ui::{fonts, lumen, theme};

/// Una sola rinuncia ai filtri: quanto sblocca e cosa si vedrebbe.
#[derive(Clone)]
pub struct Unlock {
    pub label: String,
    pub facets: Facets,
    pub total: u32,
    pub preview: Vec<String>,
}

fn trunc(text: &str, font: FontId, width: f32) -> LayoutJob {
    let mut job = LayoutJob::simple(
        text.to_string(),
        font,
        Color32::PLACEHOLDER,
        width.max(40.0),
    );
    job.wrap.max_rows = 1;
    job.wrap.overflow_character = Some('…');
    job
}

/// Mostra i compromessi e restituisce l'indice di quello cliccato.
pub fn strip(ui: &mut egui::Ui, unlocks: &[Unlock]) -> Option<usize> {
    if unlocks.is_empty() {
        return None;
    }
    let mut clicked = None;
    egui::Frame::none()
        .fill(lumen::DEEP)
        .stroke(Stroke::new(1.0_f32, lumen::BORDER))
        .rounding(egui::Rounding::same(lumen::RADIUS))
        .inner_margin(egui::Margin::symmetric(12.0, 9.0))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                ui.label(lumen::mono(9.0, "SBLOCCA").color(lumen::EMBER));
                ui.add_space(6.0);
                ui.label(
                    lumen::mono(9.0, "UN VINCOLO IN MENO · STIME TMDB")
                        .color(lumen::FAINT),
                );
            });
            ui.add_space(4.0);
            for (i, u) in unlocks.iter().enumerate() {
                if row(ui, u) {
                    clicked = Some(i);
                }
                if i + 1 < unlocks.len() {
                    ui.add_space(3.0);
                }
            }
        });
    clicked
}

fn row(ui: &mut egui::Ui, u: &Unlock) -> bool {
    let width = ui.available_width();
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 42.0), Sense::click());
    let hover = ui.ctx().animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(
        rect,
        egui::Rounding::same(lumen::RADIUS_SM),
        theme::mix(lumen::SURFACE, lumen::RAISED, hover),
    );
    painter.rect_stroke(
        rect,
        egui::Rounding::same(lumen::RADIUS_SM),
        Stroke::new(
            1.0_f32,
            if hover > 0.05 {
                lumen::BORDER_HI
            } else {
                lumen::BORDER
            },
        ),
    );

    let left = rect.left() + 10.0;
    let text_width = (rect.width() - 36.0).max(40.0);
    let label = format!(
        "{}  ·  {} film",
        u.label,
        theme::thousands(u.total)
    );
    let galley = painter.layout_job(trunc(
        &label,
        FontId::new(11.0, FontFamily::Name(fonts::LUMEN_UI_BOLD.into())),
        text_width,
    ));
    painter.galley(egui::pos2(left, rect.top() + 7.0), galley, lumen::IVORY);

    if !u.preview.is_empty() {
        let preview = u.preview.join(" · ");
        let galley = painter.layout_job(trunc(
            &preview,
            FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
            text_width,
        ));
        painter.galley(
            egui::pos2(left, rect.top() + 23.0),
            galley,
            lumen::MUTED,
        );
    }

    painter.text(
        egui::pos2(rect.right() - 10.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "→",
        FontId::new(13.0, FontFamily::Proportional),
        if hover > 0.05 {
            lumen::EMBER
        } else {
            lumen::FAINT
        },
    );
    lumen::focus(ui, &resp, lumen::RADIUS_SM);
    resp.on_hover_text("Applica e cerca subito").clicked()
}
