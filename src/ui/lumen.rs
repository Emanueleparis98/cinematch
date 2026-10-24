//! LUMEN: il linguaggio visivo della style lab.
//! Buio caldo, avorio, ember; Fraunces per i titoli, Instrument Sans per l'UI,
//! JetBrains Mono per i metadati. Solo la lab usa questo modulo.

use egui::{
    Align2, Color32, FontFamily, FontId, Mesh, Rect, Response, RichText, Rounding, Sense,
    Stroke,
};

use crate::domain::model::Movie;
use crate::tmdb::poster_url;
use crate::ui::fonts;
use crate::ui::images::ImageStore;

pub const INK: Color32 = Color32::from_rgb(8, 9, 12);
pub const DEEP: Color32 = Color32::from_rgb(14, 16, 22);
pub const SURFACE: Color32 = Color32::from_rgb(21, 24, 33);
pub const RAISED: Color32 = Color32::from_rgb(28, 32, 43);
pub const BORDER: Color32 = Color32::from_rgb(42, 47, 58);
pub const BORDER_HI: Color32 = Color32::from_rgb(61, 67, 80);
pub const IVORY: Color32 = Color32::from_rgb(244, 238, 226);
pub const MUTED: Color32 = Color32::from_rgb(167, 163, 153);
pub const FAINT: Color32 = Color32::from_rgb(110, 107, 100);
pub const EMBER: Color32 = Color32::from_rgb(255, 92, 53);
pub const EMBER_HI: Color32 = Color32::from_rgb(255, 122, 87);
pub const CELADON: Color32 = Color32::from_rgb(143, 227, 192);
pub const DANGER: Color32 = Color32::from_rgb(255, 77, 94);
pub const RADIUS: f32 = 4.0;
pub const RADIUS_SM: f32 = 2.0;

// ------------------------------------------------------------- stile ------

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = INK;
    visuals.window_fill = SURFACE;
    visuals.faint_bg_color = SURFACE;
    visuals.extreme_bg_color = DEEP;
    visuals.window_rounding = Rounding::same(RADIUS);
    visuals.menu_rounding = Rounding::same(RADIUS);
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 0.0),
        blur: 0.0,
        spread: 0.0,
        color: Color32::TRANSPARENT,
    };
    visuals.popup_shadow = visuals.window_shadow;
    visuals.warn_fg_color = EMBER;
    visuals.error_fg_color = DANGER;
    visuals.code_bg_color = DEEP;
    visuals.text_cursor.stroke = Stroke::new(1.5_f32, EMBER);
    visuals.widgets.noninteractive.bg_fill = SURFACE;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, IVORY);
    visuals.widgets.noninteractive.rounding = Rounding::same(RADIUS);
    visuals.widgets.inactive.bg_fill = DEEP;
    visuals.widgets.inactive.weak_bg_fill = DEEP;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, MUTED);
    visuals.widgets.inactive.rounding = Rounding::same(RADIUS);
    visuals.widgets.hovered.bg_fill = RAISED;
    visuals.widgets.hovered.weak_bg_fill = RAISED;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, BORDER_HI);
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, IVORY);
    visuals.widgets.hovered.rounding = Rounding::same(RADIUS);
    visuals.widgets.active.bg_fill = EMBER;
    visuals.widgets.active.weak_bg_fill = EMBER;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, EMBER);
    // Il testo «strong» di egui usa il colore dei widget attivi: deve restare
    // avorio, altrimenti i titoli delle viste 0.2.3 diventano neri su nero.
    visuals.widgets.active.fg_stroke = Stroke::new(1.0_f32, IVORY);
    visuals.widgets.active.rounding = Rounding::same(RADIUS);
    visuals.widgets.open.weak_bg_fill = RAISED;
    visuals.widgets.open.bg_fill = RAISED;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, IVORY);
    visuals.widgets.open.rounding = Rounding::same(RADIUS);
    visuals.selection.bg_fill = alpha(EMBER, 90);
    visuals.selection.stroke = Stroke::new(1.0_f32, IVORY);
    visuals.hyperlink_color = EMBER;
    visuals.override_text_color = Some(IVORY);

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 8.0);
    style.spacing.button_padding = egui::vec2(12.0, 8.0);
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width: 6.0,
        handle_min_length: 32.0,
        bar_inner_margin: 2.0,
        bar_outer_margin: 2.0,
        ..Default::default()
    };
    ctx.set_style(style);
}

pub fn install(ctx: &egui::Context) {
    fonts::install_lumen(ctx);
    apply(ctx);
}

pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_unmultiplied(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()), 255)
}

pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

// ------------------------------------------------------------- testo ------

pub fn display(size: f32, text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(size)
        .family(FontFamily::Name(fonts::LUMEN_DISPLAY.into()))
        .color(IVORY)
}

pub fn display_color(size: f32, text: impl Into<String>, color: Color32) -> RichText {
    display(size, text).color(color)
}

pub fn bold(size: f32, text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(size)
        .family(FontFamily::Name(fonts::LUMEN_UI_BOLD.into()))
        .color(IVORY)
}

pub fn body(size: f32, text: impl Into<String>) -> RichText {
    RichText::new(text).size(size).color(IVORY)
}

pub fn mono(size: f32, text: impl Into<String>) -> RichText {
    RichText::new(text)
        .size(size)
        .family(FontFamily::Name(fonts::LUMEN_MONO.into()))
        .color(MUTED)
}

pub fn label(text: &str) -> RichText {
    mono(10.0, text.to_uppercase()).color(FAINT)
}

// ---------------------------------------------------------- struttura -----

pub fn hairline(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 1.0),
        Sense::hover(),
    );
    ui.painter().rect_filled(rect, Rounding::ZERO, BORDER);
}

pub fn seam(painter: &egui::Painter, rect: Rect) {
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), EMBER);
    mesh.colored_vertex(rect.right_top(), Color32::TRANSPARENT);
    mesh.colored_vertex(rect.left_bottom(), EMBER);
    mesh.colored_vertex(rect.right_bottom(), Color32::TRANSPARENT);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(mesh);
}

pub fn scrim(painter: &egui::Painter, rect: Rect) {
    let mut mesh = Mesh::default();
    let top = Color32::TRANSPARENT;
    let bottom = alpha(INK, 245);
    mesh.colored_vertex(rect.left_top(), top);
    mesh.colored_vertex(rect.right_top(), top);
    mesh.colored_vertex(rect.left_bottom(), bottom);
    mesh.colored_vertex(rect.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(mesh);
}

pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(RADIUS))
        .inner_margin(egui::Margin::same(14.0))
}

pub fn button_sized(ui: &mut egui::Ui, text: &str, primary: bool, width: f32) -> Response {
    let enabled = ui.is_enabled();
    let sense = if enabled {
        Sense::click()
    } else {
        Sense::hover()
    };
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 36.0), sense);
    let hover = if enabled {
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered(), 0.12)
    } else {
        0.0
    };
    let fill = if !enabled {
        DEEP
    } else if primary {
        if hover > 0.01 {
            EMBER_HI
        } else {
            EMBER
        }
    } else if hover > 0.01 {
        RAISED
    } else {
        DEEP
    };
    let ink = if !enabled {
        FAINT
    } else if primary {
        INK
    } else {
        IVORY
    };
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(RADIUS_SM), fill);
    p.rect_stroke(
        rect,
        Rounding::same(RADIUS_SM),
        Stroke::new(
            1.0_f32,
            if primary && enabled { EMBER } else { BORDER },
        ),
    );
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        text.to_uppercase(),
        FontId::new(10.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        ink,
    );
    focus(ui, &resp, RADIUS_SM);
    if enabled {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

pub fn link(ui: &mut egui::Ui, text: &str, color: Color32) -> Response {
    let resp = ui.add(
        egui::Button::new(
            RichText::new(text.to_uppercase())
                .size(10.0)
                .family(FontFamily::Name(fonts::LUMEN_MONO.into()))
                .color(color),
        )
        .frame(false),
    );
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn focus(ui: &egui::Ui, resp: &Response, rounding: f32) {
    if resp.has_focus() {
        ui.painter().rect_stroke(
            resp.rect.expand(1.0),
            Rounding::same(rounding + 1.0),
            Stroke::new(1.0_f32, EMBER),
        );
    }
}

pub fn field(ui: &mut egui::Ui, label_text: &str, value: &str, mask: bool) {
    ui.add_space(10.0);
    ui.label(label(label_text));
    ui.add_space(5.0);
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 38.0),
        Sense::hover(),
    );
    ui.painter().rect_filled(rect, Rounding::same(RADIUS_SM), DEEP);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(RADIUS_SM),
        Stroke::new(1.0_f32, BORDER),
    );
    let text = if mask {
        "•".repeat(value.chars().count().min(32))
    } else {
        value.to_string()
    };
    ui.painter().text(
        rect.left_center() + egui::vec2(12.0, 0.0),
        Align2::LEFT_CENTER,
        text,
        FontId::proportional(13.0),
        if mask { MUTED } else { IVORY },
    );
}

// -------------------------------------------------------------- icone -----

#[derive(Clone, Copy)]
pub enum Icon {
    Home,
    Results,
    Grid,
    Detail,
    Settings,
    New,
    Refresh,
    ArrowLeft,
    ArrowRight,
}

pub fn icon(painter: &egui::Painter, rect: Rect, icon: Icon, color: Color32) {
    let s = rect.width().min(rect.height()) / 20.0;
    let c = rect.center();
    let p = |x: f32, y: f32| c + egui::vec2(x * s, y * s);
    let stroke = Stroke::new(1.5 * s, color);
    match icon {
        Icon::Home => {
            painter.rect_stroke(
                Rect::from_min_max(p(-7.0, -5.0), p(7.0, 6.0)),
                Rounding::same(1.5 * s),
                stroke,
            );
            painter.line_segment([p(-3.0, 2.0), p(3.0, 2.0)], stroke);
        }
        Icon::Results => {
            for y in [-5.0, 0.0, 5.0] {
                painter.rect_stroke(
                    Rect::from_min_max(p(-7.0, y - 1.8), p(-2.0, y + 1.8)),
                    Rounding::same(1.0 * s),
                    stroke,
                );
                painter.line_segment([p(0.0, y), p(7.0, y)], stroke);
            }
        }
        Icon::Grid => {
            for x in [-6.5, 1.0] {
                for y in [-6.5, 1.0] {
                    painter.rect_stroke(
                        Rect::from_min_size(p(x, y), egui::vec2(5.5, 5.5) * s),
                        Rounding::same(1.0 * s),
                        stroke,
                    );
                }
            }
        }
        Icon::Detail => {
            painter.rect_stroke(
                Rect::from_min_max(p(-7.0, -6.0), p(7.0, 6.0)),
                Rounding::same(1.5 * s),
                stroke,
            );
            painter.line_segment([p(-4.0, 2.0), p(4.0, 2.0)], stroke);
            painter.circle_filled(p(0.0, -1.5), 1.5 * s, color);
        }
        Icon::Settings => {
            for (y, x) in [(-4.5, 3.0), (0.0, -3.0), (4.5, 1.0)] {
                painter.line_segment([p(-7.0, y), p(7.0, y)], stroke);
                painter.circle_filled(p(x, y), 2.0 * s, color);
            }
        }
        Icon::New => {
            painter.line_segment([p(-6.0, 0.0), p(6.0, 0.0)], stroke);
            painter.line_segment([p(0.0, -6.0), p(0.0, 6.0)], stroke);
        }
        Icon::Refresh => {
            let mut points = Vec::with_capacity(24);
            for i in 0..=20 {
                let a = -0.7 + i as f32 / 20.0 * std::f32::consts::TAU * 0.82;
                points.push(egui::pos2(
                    c.x + a.cos() * 5.8 * s,
                    c.y + a.sin() * 5.8 * s,
                ));
            }
            painter.add(egui::Shape::line(points, stroke));
            let tip = p(4.4, -4.4);
            painter.line_segment([tip, tip + egui::vec2(-2.0 * s, -2.4 * s)], stroke);
            painter.line_segment([tip, tip + egui::vec2(5.2 * s, -0.4 * s)], stroke);
        }
        Icon::ArrowLeft => {
            painter.line_segment([p(6.0, 0.0), p(-5.4, 0.0)], stroke);
            painter.line_segment([p(-1.8, -3.8), p(-5.4, 0.0)], stroke);
            painter.line_segment([p(-1.8, 3.8), p(-5.4, 0.0)], stroke);
        }
        Icon::ArrowRight => {
            painter.line_segment([p(-6.0, 0.0), p(5.4, 0.0)], stroke);
            painter.line_segment([p(1.8, -3.8), p(5.4, 0.0)], stroke);
            painter.line_segment([p(1.8, 3.8), p(5.4, 0.0)], stroke);
        }
    }
}

pub fn icon_button(
    ui: &mut egui::Ui,
    icon_kind: Icon,
    size: f32,
    tip: &str,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(size, size), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    if hover > 0.01 {
        ui.painter().rect_filled(
            rect,
            Rounding::same(RADIUS_SM),
            mix(INK, RAISED, hover),
        );
        ui.painter().rect_stroke(
            rect,
            Rounding::same(RADIUS_SM),
            Stroke::new(1.0_f32, BORDER),
        );
    }
    icon(
        ui.painter(),
        rect.shrink(size * 0.28),
        icon_kind,
        mix(FAINT, IVORY, hover),
    );
    focus(ui, &resp, RADIUS_SM);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
        .on_hover_text(tip)
}

pub fn nav_item(ui: &mut egui::Ui, icon_kind: Icon, label_text: &str, active: bool) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(72.0, 62.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    if active || hover > 0.01 {
        p.rect_filled(
            rect,
            Rounding::ZERO,
            if active { SURFACE } else { alpha(SURFACE, 130) },
        );
    }
    if active {
        p.rect_filled(
            Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height())),
            Rounding::ZERO,
            EMBER,
        );
    }
    icon(
        p,
        Rect::from_center_size(
            egui::pos2(rect.center().x, rect.top() + 23.0),
            egui::vec2(20.0, 20.0),
        ),
        icon_kind,
        if active {
            IVORY
        } else {
            mix(FAINT, IVORY, hover)
        },
    );
    p.text(
        egui::pos2(rect.center().x, rect.bottom() - 13.0),
        Align2::CENTER_CENTER,
        label_text.to_uppercase(),
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        if active { IVORY } else { FAINT },
    );
    focus(ui, &resp, 0.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

// ------------------------------------------------------------- poster -----

pub fn poster(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movie: Option<&Movie>,
    width: f32,
    height: f32,
    selected: bool,
) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, height), Sense::click());
    ui.painter().rect_filled(rect, Rounding::ZERO, DEEP);
    let mut loaded = false;
    if let Some(m) = movie {
        if let Some(path) = &m.poster_path {
            let uri = poster_url(path, "w342");
            if let Some(tex) = images.texture(ui.ctx(), &uri) {
                let sized = egui::load::SizedTexture::new(tex.id(), rect.size());
                ui.put(rect, egui::Image::new(sized));
                loaded = true;
            }
        }
    }
    if !loaded {
        ui.painter().rect_filled(rect, Rounding::ZERO, RAISED);
        if let Some(m) = movie {
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                crate::ui::theme::truncate(&m.title, 20),
                FontId::new(
                    (width / 8.0).clamp(11.0, 20.0),
                    FontFamily::Name(fonts::LUMEN_DISPLAY.into()),
                ),
                IVORY,
            );
        }
    }
    let border = if selected || resp.hovered() {
        EMBER
    } else {
        BORDER
    };
    ui.painter().rect_stroke(
        rect,
        Rounding::ZERO,
        Stroke::new(1.0_f32, border),
    );
    if resp.hovered() || selected {
        seam(
            ui.painter(),
            Rect::from_min_size(rect.min, egui::vec2(rect.width(), 1.5)),
        );
    }
    focus(ui, &resp, 0.0);
    resp.on_hover_cursor(egui::CursorIcon::PointingHand)
}

pub fn backdrop(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movie: Option<&Movie>,
    width: f32,
    height: f32,
) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), Sense::hover());
    ui.painter().rect_filled(rect, Rounding::ZERO, DEEP);
    if let Some(m) = movie {
        if let Some(path) = &m.backdrop_path {
            let uri = poster_url(path, "w780");
            if let Some(tex) = images.texture(ui.ctx(), &uri) {
                let sized = egui::load::SizedTexture::new(tex.id(), rect.size());
                ui.put(rect, egui::Image::new(sized));
            }
        }
    }
    scrim(ui.painter(), rect);
}

pub fn warm_spill(painter: &egui::Painter, center: egui::Pos2, radius: f32) {
    for i in 0..4 {
        let r = radius * (1.0 - i as f32 * 0.18);
        let a = (12 - i * 3) as u8;
        painter.circle_filled(center, r, alpha(EMBER, a));
    }
}
