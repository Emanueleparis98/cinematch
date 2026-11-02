//! Avvia la versione in sviluppo senza prendere il mutex dell'app installata.
//! Guida e prompt locali; nessun servizio AI o archivio di film.
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "CineMatch · Correnti 0.14.0",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("CineMatch · Correnti 0.14.0")
                .with_position([20.0, 20.0])
                .with_inner_size([1220.0, 660.0])
                .with_min_inner_size([800.0, 600.0])
                .with_decorations(false),
            ..Default::default()
        },
        Box::new(move |cc| {
            cinematch::ui::lumen::install(&cc.egui_ctx);
            Ok(Box::new(cinematch::app::App::new(&cc.egui_ctx)))
        }),
    )
}
