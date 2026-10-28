#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use cinematch::{app, infra, ui};

// Glow (OpenGL) e il default: su Windows impegna circa meta memoria del
// backend wgpu/DX12 (~190 MB contro ~420 MB a riposo). Il backend wgpu entra
// solo in una build con `--features wgpu`; in quella build `CINEMATCH_RENDERER=wgpu` lo sceglie.
fn renderer() -> eframe::Renderer {
    #[cfg(feature = "wgpu")]
    {
        match std::env::var("CINEMATCH_RENDERER").unwrap_or_default().as_str() {
            "wgpu" => eframe::Renderer::Wgpu,
            _ => eframe::Renderer::Glow,
        }
    }
    #[cfg(not(feature = "wgpu"))]
    {
        eframe::Renderer::Glow
    }
}

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
    let Some(_instance) = infra::single_instance::acquire() else {
        return Ok(());
    };
    let cfg = infra::config::Config::load();
    let renderer = renderer();
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
            ui::lumen::install(&cc.egui_ctx);
            Ok(Box::new(app::App::new(&cc.egui_ctx, cfg)))
        }),
    )
}
