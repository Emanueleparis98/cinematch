//! Tema e stile globale: palette, spaziature, font di sistema, widget di base.
//! Le superfici sono stratificate su uno sfondo «aurora» (vedi ui::fx):
//! i pannelli sono appena translucidi e lasciano passare la luce.

use egui::{Color32, Rounding, Stroke};

// ------------------------------------------------------------- colori ------

/// Cima dello sfondo: notte blu; il fondo e piu profondo.
pub const BG_TOP: Color32 = Color32::from_rgb(11, 18, 34);
pub const BG: Color32 = Color32::from_rgb(5, 8, 15);
pub const PANEL: Color32 = Color32::from_rgb(12, 20, 34);
/// Pannelli in primo piano: un velo translucido sull'aurora.
pub const PANEL_SOFT: Color32 = Color32::from_rgba_premultiplied(10, 16, 28, 235);
pub const CARD: Color32 = Color32::from_rgb(15, 25, 41);
pub const CARD_HOVER: Color32 = Color32::from_rgb(21, 35, 56);
/// Blu cinematografico: il colore guida di tutta l'interfaccia.
pub const ACCENT: Color32 = Color32::from_rgb(59, 130, 246);
/// Ciano da proiettore: seconda luce, solo nei gradienti e nei dettagli.
pub const ACCENT_2: Color32 = Color32::from_rgb(34, 211, 238);
pub const ACCENT_3: Color32 = Color32::from_rgb(147, 197, 253);
pub const ACCENT_DIM: Color32 = Color32::from_rgb(22, 48, 90);
pub const TEXT: Color32 = Color32::from_rgb(233, 239, 251);
pub const DIM: Color32 = Color32::from_rgb(147, 163, 189);
pub const FAINT: Color32 = Color32::from_rgb(112, 129, 156);
pub const BORDER: Color32 = Color32::from_rgb(30, 45, 70);
pub const SKELETON: Color32 = Color32::from_rgb(20, 33, 54);
pub const SKELETON_HI: Color32 = Color32::from_rgb(36, 57, 90);
/// Fondo dei poster mentre l'immagine non c'e ancora.
pub const POSTER_BG: Color32 = Color32::from_rgb(6, 11, 20);
pub const WARN: Color32 = Color32::from_rgb(242, 193, 78);
pub const DANGER: Color32 = Color32::from_rgb(240, 113, 120);
pub const OK: Color32 = Color32::from_rgb(52, 211, 153);

/// Arrotondamenti: piccolo per i widget, grande per schede e pannelli.
pub const ROUND: f32 = 10.0;
pub const ROUND_CARD: f32 = 16.0;

// ------------------------------------------------------------ utilita' -----

/// Mescola due colori (t = 0 -> a, t = 1 -> b).
pub fn mix(a: Color32, b: Color32, t: f32) -> Color32 {
    let t = t.clamp(0.0, 1.0);
    let ch = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t) as u8;
    Color32::from_rgba_unmultiplied(ch(a.r(), b.r()), ch(a.g(), b.g()), ch(a.b(), b.b()), 255)
}

/// Stesso colore con un'altra trasparenza.
pub fn alpha(c: Color32, a: u8) -> Color32 {
    Color32::from_rgba_unmultiplied(c.r(), c.g(), c.b(), a)
}

/// Pulsazione lenta in [0,1] per i puntini di stato.
pub fn pulse(now: f64, speed: f64) -> f32 {
    (0.5 + 0.5 * (now * speed).sin()) as f32
}

// -------------------------------------------------------------- stile ------

pub fn apply(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    visuals.panel_fill = BG;
    visuals.window_fill = PANEL;
    visuals.faint_bg_color = PANEL;
    visuals.extreme_bg_color = Color32::from_rgb(4, 7, 13);
    visuals.window_rounding = Rounding::same(16.0);
    visuals.menu_rounding = Rounding::same(12.0);
    visuals.window_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 14.0),
        blur: 40.0,
        spread: 0.0,
        color: Color32::from_black_alpha(140),
    };
    visuals.popup_shadow = egui::epaint::Shadow {
        offset: egui::vec2(0.0, 8.0),
        blur: 24.0,
        spread: 0.0,
        color: Color32::from_black_alpha(120),
    };
    visuals.window_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.warn_fg_color = WARN;
    visuals.error_fg_color = DANGER;
    visuals.code_bg_color = CARD;
    visuals.text_cursor.stroke = Stroke::new(2.0_f32, ACCENT_3);
    visuals.widgets.noninteractive.bg_fill = PANEL;
    visuals.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.noninteractive.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.widgets.inactive.bg_fill = CARD;
    visuals.widgets.inactive.weak_bg_fill = CARD;
    visuals.widgets.inactive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    // Il colore «fg» dei widget inattivi dipinge anche la barra di scorrimento
    // e le spunte: un azzurro spento resta coerente con il tema.
    visuals.widgets.inactive.fg_stroke = Stroke::new(1.0_f32, mix(ACCENT_3, DIM, 0.25));
    visuals.widgets.hovered.bg_fill = CARD_HOVER;
    visuals.widgets.hovered.weak_bg_fill = CARD_HOVER;
    visuals.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, alpha(ACCENT, 160));
    visuals.widgets.hovered.fg_stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.widgets.active.bg_fill = ACCENT_DIM;
    visuals.widgets.active.weak_bg_fill = ACCENT_DIM;
    visuals.widgets.active.bg_stroke = Stroke::new(1.0_f32, ACCENT);
    // Stato «aperto»: testata delle finestre e voci di menu gia' scelte.
    visuals.widgets.open.weak_bg_fill = CARD;
    visuals.widgets.open.bg_fill = ACCENT_DIM;
    visuals.widgets.open.bg_stroke = Stroke::new(1.0_f32, BORDER);
    visuals.widgets.open.fg_stroke = Stroke::new(1.0_f32, TEXT);
    visuals.selection.bg_fill = alpha(ACCENT, 110);
    visuals.selection.stroke = Stroke::new(1.0_f32, Color32::WHITE);
    visuals.hyperlink_color = ACCENT_3;
    visuals.override_text_color = Some(TEXT);

    let mut style = (*ctx.style()).clone();
    style.visuals = visuals;
    style.spacing.item_spacing = egui::vec2(8.0, 7.0);
    style.spacing.button_padding = egui::vec2(10.0, 5.0);
    style.spacing.scroll = egui::style::ScrollStyle {
        bar_width: 7.0,
        handle_min_length: 44.0,
        bar_inner_margin: 2.0,
        bar_outer_margin: 2.0,
        ..Default::default()
    };
    ctx.set_style(style);
}

/// Cornice standard delle schede: vetro scuro, bordo sottile, ombra morbida.
pub fn card_frame() -> egui::Frame {
    egui::Frame::none()
        .fill(CARD)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(ROUND_CARD))
        .shadow(egui::epaint::Shadow {
            offset: egui::vec2(0.0, 10.0),
            blur: 26.0,
            spread: 0.0,
            color: Color32::from_black_alpha(110),
        })
        .inner_margin(egui::Margin::same(14.0))
}

// ------------------------------------------------------------- widget ------

/// Chip selezionabile. Quando `enabled` e false resta visibile ma inerte,
/// con il motivo mostrato nel tooltip: e la parte visibile del motore di vincoli.
pub fn chip(
    ui: &mut egui::Ui,
    text: &str,
    selected: bool,
    enabled: bool,
    reason: Option<&str>,
) -> egui::Response {
    let mut rich = egui::RichText::new(text).size(12.5);
    let fill = if !enabled {
        PANEL
    } else if selected {
        ACCENT
    } else {
        CARD
    };
    rich = if !enabled {
        rich.color(FAINT)
    } else if selected {
        rich.color(Color32::WHITE).strong()
    } else {
        rich.color(TEXT)
    };
    let stroke_color = if selected && enabled { ACCENT } else { BORDER };
    let btn = egui::Button::new(rich)
        .fill(fill)
        .rounding(Rounding::same(13.0))
        .stroke(Stroke::new(1.0_f32, stroke_color))
        .min_size(egui::vec2(0.0, 26.0));
    let resp = ui.add_enabled(enabled, btn);
    // Anello di luce leggero al passaggio del mouse.
    if enabled {
        let hover = ui
            .ctx()
            .animate_bool_with_time(resp.id.with("chip-hover"), resp.hovered(), 0.12);
        if hover > 0.02 {
            ui.painter().rect_stroke(
                resp.rect,
                Rounding::same(13.0),
                Stroke::new(1.0_f32, alpha(ACCENT, (110.0 * hover) as u8)),
            );
        }
    }
    focus_ring(ui, &resp, 13.0);
    match reason {
        Some(r) if !enabled => resp.on_disabled_hover_text(r),
        Some(r) if selected => resp.on_hover_text(format!("{r}\n(selezionato: clicca per rimuovere)")),
        Some(r) => resp.on_hover_text(r),
        None => resp,
    }
}

/// Pulsante principale disegnato a mano: gradiente blu -> ciano,
/// alone al passaggio del mouse, affondo al clic.
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
    let (top, bottom, text) = if !enabled {
        (CARD, CARD, FAINT)
    } else {
        (
            mix(ACCENT, Color32::WHITE, 0.10 * hover),
            mix(ACCENT_2, ACCENT, 0.40),
            Color32::WHITE,
        )
    };
    let painter = ui.painter();
    if enabled && hover > 0.02 {
        for i in 0..3 {
            let grow = 3.0 + i as f32 * 4.0;
            let a = (26.0 * hover / (1.0 + i as f32)) as u8;
            painter.rect_filled(
                rect.expand(grow),
                Rounding::same(ROUND + grow * 0.6),
                alpha(ACCENT, a),
            );
        }
    }
    let y = if pressed { 1.0 } else { 0.0 };
    let r = rect.translate(egui::vec2(0.0, y));
    let mut mesh = egui::Mesh::default();
    mesh.colored_vertex(r.left_top(), top);
    mesh.colored_vertex(r.right_top(), top);
    mesh.colored_vertex(r.left_bottom(), bottom);
    mesh.colored_vertex(r.right_bottom(), bottom);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(mesh);
    // Etichetta bianca con una leggera ombra: resta leggibile anche sul ciano.
    painter.text(
        r.center() + egui::vec2(0.0, 1.0),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        Color32::from_black_alpha(80),
    );
    painter.text(
        r.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(13.0),
        text,
    );
    if enabled {
        focus_ring(ui, &resp, ROUND);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

/// Anello visibile quando un widget ha il fuoco da tastiera.
pub fn focus_ring(ui: &egui::Ui, resp: &egui::Response, rounding: f32) {
    if resp.has_focus() {
        ui.painter().rect_stroke(
            resp.rect.expand(2.0),
            Rounding::same(rounding + 2.0),
            Stroke::new(1.5_f32, alpha(ACCENT_3, 190)),
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

/// «12.500»: i numeri grandi si leggono a colpo d'occhio.
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
