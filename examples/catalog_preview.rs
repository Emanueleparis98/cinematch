//! Avvia la versione in sviluppo senza prendere il mutex dell'app installata.
//! Dati TMDB reali e configurazione locale; nessun provider AI.
fn main() -> eframe::Result<()> {
    let cfg = cinematch::infra::config::Config::load();
    eframe::run_native(
        "CineMatch · Catalogo 0.12.0",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_title("CineMatch · Catalogo 0.12.0")
                .with_position([20.0, 20.0])
                .with_inner_size([1220.0, 660.0])
                .with_min_inner_size([800.0, 600.0])
                .with_decorations(false),
            ..Default::default()
        },
        Box::new(move |cc| {
            cinematch::ui::lumen::install(&cc.egui_ctx);
            Ok(Box::new(cinematch::app::App::new(&cc.egui_ctx, cfg)))
        }),
    )
}
