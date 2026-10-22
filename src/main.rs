#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cinematch::{app, infra, ui};

fn app_icon() -> Option<egui::IconData> {
    let image = image::load_from_memory(include_bytes!("../assets/cinematch.png")).ok()?;
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    Some(egui::IconData {
        rgba: image.into_raw(),
        width,
        height,
    })
}

fn main() -> eframe::Result<()> {
    let cfg = infra::config::Config::load();
    // Renderer: glow (OpenGL) e il default perche su Windows impegna circa meta
    // memoria del backend wgpu/DX12 (misurato: ~190 MB contro ~420 MB a riposo).
    // Per usare wgpu: CINEMATCH_RENDERER=wgpu
    let renderer = match std::env::var("CINEMATCH_RENDERER").unwrap_or_default().as_str() {
        "wgpu" => eframe::Renderer::Wgpu,
        _ => eframe::Renderer::Glow,
    };
    let mut viewport = egui::ViewportBuilder::default()
        // A 150% su 1920x1200 l'area di lavoro e ~1280x752 logici:
        // 660 resta sopra la taskbar. La posizione fissa evita che la
        // cascata di Windows porti la finestra fuori schermo.
        // Il minimo resta sotto la misura iniziale: i layout si adattano
        // fino a 800x600, senza tagliare composer o pannelli.
        .with_inner_size([1220.0, 660.0])
        .with_position([20.0, 20.0])
        .with_min_inner_size([800.0, 600.0])
        // Niente barra di Windows: i controlli vivono nella barra comandi.
        .with_decorations(false)
        .with_title("CineMatch");
    if let Some(icon) = app_icon() {
        viewport = viewport.with_icon(icon);
    }
    let options = eframe::NativeOptions {
        viewport,
        renderer,
        ..Default::default()
    };
    eframe::run_native(
        "CineMatch",
        options,
        Box::new(move |cc| {
            ui::theme::apply(&cc.egui_ctx);
            ui::fonts::install(&cc.egui_ctx);
            Ok(Box::new(app::App::new(&cc.egui_ctx, cfg)))
        }),
    )
}
