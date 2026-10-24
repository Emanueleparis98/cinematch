//! Alias del tema LUMEN usati dalle viste 0.2.3 ancora da convertire.

use egui::{Color32, Rounding, Stroke};

use super::lumen;

pub const BG: Color32 = lumen::INK;
pub const BG_TOP: Color32 = lumen::INK;
pub const PANEL: Color32 = lumen::SURFACE;
pub const PANEL_SOFT: Color32 = lumen::SURFACE;
pub const CARD: Color32 = lumen::SURFACE;
pub const CARD_HOVER: Color32 = lumen::RAISED;
pub const ACCENT: Color32 = lumen::EMBER;
pub const ACCENT_2: Color32 = lumen::EMBER_HI;
pub const ACCENT_3: Color32 = lumen::EMBER_HI;
pub const ACCENT_DIM: Color32 = Color32::from_rgb(42, 23, 18);
pub const TEXT: Color32 = lumen::IVORY;
pub const DIM: Color32 = lumen::MUTED;
pub const FAINT: Color32 = lumen::FAINT;
pub const BORDER: Color32 = lumen::BORDER;
pub const SKELETON: Color32 = Color32::from_rgb(26, 30, 39);
pub const SKELETON_HI: Color32 = Color32::from_rgb(38, 44, 56);
pub const POSTER_BG: Color32 = lumen::DEEP;
pub const WARN: Color32 = lumen::EMBER;
pub const DANGER: Color32 = lumen::DANGER;
pub const OK: Color32 = lumen::CELADON;
pub const ROUND: f32 = 2.0;
pub const ROUND_CARD: f32 = 4.0;

pub use super::lumen::{alpha, mix};

pub fn pulse(now: f64, speed: f64) -> f32 {
    (0.5 + 0.5 * (now * speed).sin()) as f32
}

pub fn apply(ctx: &egui::Context) {
    lumen::apply(ctx);
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(ROUND_CARD))
        .inner_margin(egui::Margin::same(14.0))
}

pub fn chip(
    ui: &mut egui::Ui,
    text: &str,
    selected: bool,
    enabled: bool,
    reason: Option<&str>,
) -> egui::Response {
    let mut rich = egui::RichText::new(text).size(11.5);
    let fill = if !enabled {
        lumen::DEEP
    } else if selected {
        ACCENT
    } else {
        lumen::DEEP
    };
    rich = if !enabled {
        rich.color(FAINT)
    } else if selected {
        rich.color(lumen::INK).strong()
    } else {
        rich.color(TEXT)
    };
    let btn = egui::Button::new(rich)
        .fill(fill)
        .rounding(Rounding::same(ROUND))
        .stroke(Stroke::new(
            1.0_f32,
            if selected && enabled { ACCENT } else { BORDER },
        ))
        .min_size(egui::vec2(0.0, 24.0));
    let resp = ui.add_enabled(enabled, btn);
    focus_ring(ui, &resp, ROUND);
    match reason {
        Some(r) if !enabled => resp.on_disabled_hover_text(r),
        Some(r) if selected => {
            resp.on_hover_text(format!("{r}\n(selezionato: clicca per rimuovere)"))
        }
        Some(r) => resp.on_hover_text(r),
        None => resp,
    }
}

pub fn primary_button(ui: &mut egui::Ui, label: &str, size: egui::Vec2) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(size, egui::Sense::click());
    let enabled = ui.is_enabled();
    let hover = if enabled {
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered(), 0.12)
    } else {
        0.0
    };
    let pressed = resp.is_pointer_button_down_on();
    let fill = if !enabled {
        lumen::DEEP
    } else if pressed {
        mix(ACCENT, Color32::BLACK, 0.18)
    } else if hover > 0.01 {
        ACCENT_2
    } else {
        ACCENT
    };
    let ink = if enabled { lumen::INK } else { FAINT };
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(ROUND), fill);
    if resp.has_focus() {
        painter.rect_stroke(
            rect.expand(1.0),
            Rounding::same(ROUND + 1.0),
            Stroke::new(1.0_f32, TEXT),
        );
    }
    painter.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label.to_uppercase(),
        egui::FontId::new(10.0, egui::FontFamily::Name(super::fonts::LUMEN_MONO.into())),
        ink,
    );
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

pub fn focus_ring(ui: &egui::Ui, resp: &egui::Response, rounding: f32) {
    if resp.has_focus() {
        ui.painter().rect_stroke(
            resp.rect.expand(1.0),
            Rounding::same(rounding + 1.0),
            Stroke::new(1.0_f32, ACCENT),
        );
    }
}

pub fn truncate(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let mut out = text.chars().take(max.saturating_sub(1)).collect::<String>();
    out.push('…');
    out
}

pub fn thousands(v: u32) -> String {
    let digits = v.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    let len = digits.len();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (len - i) % 3 == 0 {
            out.push('.');
        }
        out.push(c);
    }
    out
}
