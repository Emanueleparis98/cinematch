//! Effetti visivi disegnati a mano: sfondo aurora, gradienti, shimmer,
//! anelli di luce. Sono mesh e cerchi dipinti col Painter: nessuna
//! dipendenza nuova, tutto resta in un singolo eseguibile.
//!
//! Nota sulle prestazioni: la deriva dell'aurora chiede un ridisegno ogni
//! ~125 ms (8 fps) e lo shimmer solo mentre e' visibile.

use std::time::Duration;

use egui::{Color32, Context, Mesh, Pos2, Rect, Rounding, Stroke, Ui};

use super::theme;

// ------------------------------------------------------------- sfondo ------

/// Dipinge lo sfondo dell'app: gradiente verticale + tre aloni colorati
/// che si muovono lentamente. Va chiamata all'inizio del frame, prima dei
/// pannelli: cosi' i pannelli translucidi la lasciano trasparire.
pub fn background(ctx: &Context, now: f64) {
    let rect = ctx.screen_rect();
    if rect.width() < 4.0 {
        return;
    }
    let painter = ctx.layer_painter(egui::LayerId::background());
    let mut sky = Mesh::default();
    sky.colored_vertex(rect.left_top(), theme::BG_TOP);
    sky.colored_vertex(rect.right_top(), theme::BG_TOP);
    sky.colored_vertex(rect.left_bottom(), theme::BG);
    sky.colored_vertex(rect.right_bottom(), theme::BG);
    sky.add_triangle(0, 1, 2);
    sky.add_triangle(1, 2, 3);
    painter.add(sky);

    let orbs: [(f32, f32, f32, Color32, u8, f64); 3] = [
        (0.10, -0.04, 560.0, theme::ACCENT, 30, 0.0),
        (
            1.02,
            0.18,
            420.0,
            theme::mix(theme::ACCENT_2, theme::ACCENT, 0.50),
            13,
            2.1,
        ),
        (0.44, 1.14, 660.0, theme::ACCENT_3, 9, 4.2),
    ];
    for (fx, fy, radius, color, peak, phase) in orbs {
        let dx = 36.0 * ((now * 0.05 + phase).sin() as f32);
        let dy = 26.0 * ((now * 0.04 + phase * 1.7).cos() as f32);
        let center = egui::pos2(
            rect.left() + rect.width() * fx + dx,
            rect.top() + rect.height() * fy + dy,
        );
        soft_circle(&painter, center, radius, color, peak);
    }
    // Deriva lenta: pochi ridisegni al secondo, costo trascurabile.
    ctx.request_repaint_after(Duration::from_millis(125));
}

/// Alone morbido: ventaglio di triangoli con gradiente radiale, dal centro
/// pieno al bordo trasparente. Due anelli per una caduta più naturale.
fn soft_circle(painter: &egui::Painter, center: Pos2, radius: f32, color: Color32, peak: u8) {
    const STEPS: usize = 64;
    const INNER: f32 = 0.42;
    let inner_r = radius * INNER;
    let mid = theme::alpha(color, (peak as f32 * 0.42) as u8);
    let edge = Color32::TRANSPARENT;
    let ring = |r: f32| -> Vec<Pos2> {
        (0..=STEPS)
            .map(|i| {
                let a = i as f32 / STEPS as f32 * std::f32::consts::TAU;
                center + egui::vec2(a.cos() * r, a.sin() * r)
            })
            .collect()
    };
    // Nucleo: centro -> anello interno.
    let mut core = Mesh::default();
    core.colored_vertex(center, theme::alpha(color, peak));
    for p in ring(inner_r) {
        core.colored_vertex(p, mid);
    }
    for i in 0..STEPS as u32 {
        core.add_triangle(0, 1 + i, 2 + i);
    }
    painter.add(core);
    // Alone: anello interno -> bordo.
    let mut halo = Mesh::default();
    for p in ring(inner_r) {
        halo.colored_vertex(p, mid);
    }
    for p in ring(radius) {
        halo.colored_vertex(p, edge);
    }
    let n = STEPS as u32 + 1;
    for i in 0..STEPS as u32 {
        halo.add_triangle(i, i + 1, n + i);
        halo.add_triangle(i + 1, n + i, n + i + 1);
    }
    painter.add(halo);
}

// ----------------------------------------------------------- gradienti -----

/// Rettangolo con gradiente orizzontale. Il clip del painter lo ritaglia.
pub fn gradient_bar(painter: &egui::Painter, rect: Rect, from: Color32, to: Color32, _rounding: f32) {
    if !rect.intersects(painter.clip_rect()) || rect.width() < 1.0 || rect.height() < 1.0 {
        return;
    }
    let mut mesh = Mesh::default();
    mesh.colored_vertex(rect.left_top(), from);
    mesh.colored_vertex(rect.right_top(), to);
    mesh.colored_vertex(rect.left_bottom(), from);
    mesh.colored_vertex(rect.right_bottom(), to);
    mesh.add_triangle(0, 1, 2);
    mesh.add_triangle(1, 2, 3);
    painter.add(mesh);
}

// ------------------------------------------------------------ shimmer ------

/// Segnaposto animato per i poster che stanno arrivando: una banda di luce
/// attraversa il rettangolo. Chiede un ridisegno solo finche' e' visibile.
pub fn shimmer(ui: &Ui, rect: Rect, rounding: f32, now: f64) {
    if rect.width() < 3.0 || rect.height() < 3.0 {
        return;
    }
    let painter = ui.painter().with_clip_rect(rect);
    painter.rect_filled(rect, Rounding::same(rounding), theme::SKELETON);
    let t = (now * 0.42).fract() as f32;
    let band = rect.width() * 0.6;
    let x = rect.left() - band + t * (rect.width() + 2.0 * band);
    let b = Rect::from_min_size(egui::pos2(x, rect.top()), egui::vec2(band, rect.height()));
    if b.intersects(rect) {
        let hi = theme::alpha(theme::SKELETON_HI, 165);
        let lo = Color32::TRANSPARENT;
        let mut mesh = Mesh::default();
        mesh.colored_vertex(b.left_top(), lo);
        mesh.colored_vertex(b.center_top(), hi);
        mesh.colored_vertex(b.right_top(), lo);
        mesh.colored_vertex(b.left_bottom(), lo);
        mesh.colored_vertex(b.center_bottom(), hi);
        mesh.colored_vertex(b.right_bottom(), lo);
        mesh.add_triangle(0, 1, 3);
        mesh.add_triangle(1, 3, 4);
        mesh.add_triangle(1, 2, 4);
        mesh.add_triangle(2, 4, 5);
        painter.add(mesh);
    }
    ui.ctx().request_repaint_after(Duration::from_millis(40));
}

// --------------------------------------------------------------- varie -----

/// Anello di luce attorno a una scheda (hover/attenzione).
pub fn ring(ui: &Ui, rect: Rect, rounding: f32, color: Color32, t: f32) {
    if t <= 0.02 {
        return;
    }
    ui.painter().rect_stroke(
        rect,
        Rounding::same(rounding),
        Stroke::new(1.4_f32, theme::alpha(color, (170.0 * t) as u8)),
    );
}

/// Barra di avanzamento sottile con gradiente.
pub fn progress(ui: &mut Ui, width: f32, t: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 4.0), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, Rounding::same(2.0), theme::alpha(theme::TEXT, 16));
    let w = (rect.width() * t.clamp(0.0, 1.0)).max(6.0);
    let fill = Rect::from_min_size(rect.min, egui::vec2(w, rect.height()));
    gradient_bar(painter, fill, theme::ACCENT, theme::ACCENT_2, 2.0);
}

/// Pallino di stato: acceso e pulsante quando c'e' qualcosa in corso.
pub fn dot(ui: &mut Ui, color: Color32, pulse: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    let painter = ui.painter();
    if pulse > 0.01 {
        let halo = 5.5 + 2.5 * pulse;
        painter.circle_filled(rect.center(), halo, theme::alpha(color, (60.0 * pulse) as u8));
    }
    painter.circle_filled(rect.center(), 3.2, color);
}
