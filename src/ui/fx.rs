//! Disegno piatto LUMEN: sfondo pieno, skeleton statici, barre ember.

use egui::{Color32, Context, Rect, Stroke, Ui};

use super::{lumen, theme};

pub fn background(ctx: &Context, _now: f64) {
    let rect = ctx.viewport_rect();
    if rect.width() < 4.0 {
        return;
    }
    ctx.layer_painter(egui::LayerId::background())
        .rect_filled(rect, egui::CornerRadius::ZERO, theme::BG);
}

pub fn shimmer(ui: &Ui, rect: Rect, rounding: f32, _now: f64) {
    if rect.width() < 3.0 || rect.height() < 3.0 {
        return;
    }
    ui.painter()
        .rect_filled(rect, lumen::radius(rounding), theme::SKELETON);
}

pub fn ring(ui: &Ui, rect: Rect, rounding: f32, color: Color32, t: f32) {
    if t <= 0.02 {
        return;
    }
    ui.painter().rect_stroke(
        rect,
        lumen::radius(rounding),
        Stroke::new(1.0_f32, theme::alpha(color, (200.0 * t) as u8)), egui::StrokeKind::Inside);
}

pub fn progress(ui: &mut Ui, width: f32, t: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 3.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, egui::CornerRadius::ZERO, theme::alpha(theme::TEXT, 18));
    let w = (rect.width() * t.clamp(0.0, 1.0)).max(3.0);
    let fill = Rect::from_min_size(rect.min, egui::vec2(w, rect.height()));
    painter.rect_filled(fill, egui::CornerRadius::ZERO, theme::ACCENT);
}

pub fn dot(ui: &mut Ui, color: Color32, _pulse: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(8.0, 8.0), egui::Sense::hover());
    ui.painter().circle_filled(rect.center(), 3.0, color);
}
