pub mod detail;
pub mod facets;
pub mod fonts;
pub mod fx;
pub mod grid;
pub mod images;
pub mod lumen;
pub mod theme;
pub mod unlock;

#[cfg(test)]
pub(crate) mod layout_test_support {
    pub fn frame(
        ctx: &egui::Context,
        size: egui::Vec2,
        draw: impl FnMut(&mut egui::Ui),
    ) -> egui::FullOutput {
        let mut output = ctx.run_ui(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(egui::Pos2::ZERO, size)),
                ..Default::default()
            },
            draw,
        );
        output.textures_delta.clear();
        output
    }

    pub fn text_rect(output: &egui::FullOutput, value: &str) -> egui::Rect {
        output.shapes.iter().rev().find_map(|shape| {
            if let egui::epaint::Shape::Text(text) = &shape.shape {
                (text.galley.text() == value).then(|| text.galley.rect.translate(text.pos.to_vec2()))
            } else {
                None
            }
        }).unwrap_or_else(|| panic!("Testo non trovato: {value}"))
    }
}
