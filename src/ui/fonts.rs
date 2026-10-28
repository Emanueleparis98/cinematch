//! Tipografia LUMEN e ripiego CJK.
//!
//! Fraunces per i titoli, Instrument Sans per l'interfaccia, JetBrains Mono
//! per i metadati. I font CJK di Windows entrano una volta sola, solo quando
//! un titolo li richiede, senza perdere i font LUMEN.

use egui::{FontData, FontDefinitions, FontFamily};

const CJK_FONTS: [(&str, &str); 2] = [
    ("yu_gothic", r"C:\Windows\Fonts\YuGothM.ttc"),
    ("malgun_gothic", r"C:\Windows\Fonts\malgun.ttf"),
];

pub const LUMEN_DISPLAY: &str = "lumen-display";
pub const LUMEN_UI_BOLD: &str = "lumen-ui-bold";
pub const LUMEN_MONO: &str = "lumen-mono";

fn lumen_defs() -> FontDefinitions {
    let mut fonts = FontDefinitions::default();
    fonts.font_data.insert(
        "lumen-ui".into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/InstrumentSans-Regular.ttf"
        ))),
    );
    fonts.font_data.insert(
        LUMEN_DISPLAY.into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/Fraunces-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        LUMEN_UI_BOLD.into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/InstrumentSans-SemiBold.ttf"
        ))),
    );
    fonts.font_data.insert(
        LUMEN_MONO.into(),
        std::sync::Arc::new(FontData::from_static(include_bytes!(
            "../../assets/fonts/JetBrainsMono-Regular.ttf"
        ))),
    );
    fonts
        .families
        .entry(FontFamily::Proportional)
        .or_default()
        .insert(0, "lumen-ui".into());
    fonts
        .families
        .entry(FontFamily::Monospace)
        .or_default()
        .insert(0, LUMEN_MONO.into());
    fonts.families.insert(
        FontFamily::Name(LUMEN_DISPLAY.into()),
        vec![LUMEN_DISPLAY.into()],
    );
    fonts.families.insert(
        FontFamily::Name(LUMEN_UI_BOLD.into()),
        vec![LUMEN_UI_BOLD.into()],
    );
    fonts.families.insert(
        FontFamily::Name(LUMEN_MONO.into()),
        vec![LUMEN_MONO.into()],
    );
    fonts
}

pub fn install_lumen(ctx: &egui::Context) {
    ctx.set_fonts(lumen_defs());

    let mut style = (*ctx.global_style()).clone();
    use egui::{FontFamily::Proportional, TextStyle::*};
    style
        .text_styles
        .insert(Heading, egui::FontId::new(24.0, Proportional));
    style
        .text_styles
        .insert(Body, egui::FontId::new(13.0, Proportional));
    style
        .text_styles
        .insert(Button, egui::FontId::new(12.0, Proportional));
    style
        .text_styles
        .insert(Small, egui::FontId::new(11.0, Proportional));
    ctx.set_global_style(style);
}

fn needs_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        let u = c as u32;
        (0x1100..=0x11FF).contains(&u)
            || (0x2E80..=0x9FFF).contains(&u)
            || (0xAC00..=0xD7AF).contains(&u)
            || (0xF900..=0xFAFF).contains(&u)
            || (0x20000..=0x2FA1F).contains(&u)
    })
}

/// Aggiunge i font CJK di sistema alla famiglia LUMEN, una volta sola.
pub fn ensure_cjk(ctx: &egui::Context, loaded: &mut bool, texts: &[&str]) {
    if *loaded || !texts.iter().any(|t| needs_cjk(t)) {
        return;
    }
    let mut fonts = lumen_defs();
    let mut found = false;
    for (name, path) in CJK_FONTS {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        fonts
            .font_data
            .insert(name.to_string(), std::sync::Arc::new(FontData::from_owned(bytes)));
        for family in [
            FontFamily::Proportional,
            FontFamily::Monospace,
            FontFamily::Name(LUMEN_DISPLAY.into()),
            FontFamily::Name(LUMEN_UI_BOLD.into()),
            FontFamily::Name(LUMEN_MONO.into()),
        ] {
            fonts
                .families
                .entry(family)
                .or_default()
                .push(name.to_string());
        }
        found = true;
    }
    if found {
        ctx.set_fonts(fonts);
    }
    *loaded = true;
}
