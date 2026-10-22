//! Tipografia e glifi non latini.
//!
//! All'avvio si usano i font di Windows (Segoe UI Variable quando c'e'):
//! nessun download, nessun byte in piu' nell'eseguibile. I font di sistema
//! CJK (Yu Gothic, Malgun Gothic) entrano come ripiego **una volta sola e
//! solo quando servono davvero**: una ricerca italiana normale non carica
//! nulla e l'avvio resta leggero.

const UI_FONTS: [(&str, &str); 2] = [
    ("SegUIVar.ttf", "Segoe UI Variable"),
    ("segoeui.ttf", "Segoe UI"),
];
const MONO_FONT: &str = "consola.ttf";

const CJK_FONTS: [(&str, &str); 2] = [
    ("yu_gothic", r"C:\Windows\Fonts\YuGothM.ttc"),
    ("malgun_gothic", r"C:\Windows\Fonts\malgun.ttf"),
];

fn read_font(file: &str) -> Option<Vec<u8>> {
    let path = std::path::Path::new("C:/Windows/Fonts").join(file);
    if !path.is_file() {
        return None;
    }
    std::fs::read(path).ok()
}

/// Definizioni base: font di sistema per il testo e per il monospaziato,
/// con i font interni di egui come rete di sicurezza.
fn base() -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    if let Some((file, _name)) = UI_FONTS.iter().find(|(f, _)| read_font(f).is_some()) {
        if let Some(bytes) = read_font(file) {
            fonts
                .font_data
                .insert("cine-ui".into(), egui::FontData::from_owned(bytes));
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "cine-ui".into());
        }
    }
    if let Some(bytes) = read_font(MONO_FONT) {
        fonts
            .font_data
            .insert("cine-mono".into(), egui::FontData::from_owned(bytes));
        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "cine-mono".into());
    }
    fonts
}

/// Installa la tipografia di base e le taglie dei testi.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(base());

    let mut style = (*ctx.style()).clone();
    use egui::{FontFamily::Proportional, TextStyle::*};
    style
        .text_styles
        .insert(Heading, egui::FontId::new(20.0, Proportional));
    style
        .text_styles
        .insert(Body, egui::FontId::new(13.5, Proportional));
    style
        .text_styles
        .insert(Button, egui::FontId::new(13.0, Proportional));
    style
        .text_styles
        .insert(Small, egui::FontId::new(11.0, Proportional));
    ctx.set_style(style);
}

/// Vero se il testo contiene scritture che i font di base non coprono.
fn needs_cjk(text: &str) -> bool {
    text.chars().any(|c| {
        let u = c as u32;
        (0x1100..=0x11FF).contains(&u)   // jamo coreani
            || (0x2E80..=0x9FFF).contains(&u) // radicali, kana, ideogrammi
            || (0xAC00..=0xD7AF).contains(&u) // sillabe coreane
            || (0xF900..=0xFAFF).contains(&u) // ideogrammi compatibili
            || (0x20000..=0x2FA1F).contains(&u) // estensione B e successivi
    })
}

/// Carica i font di sistema come ripiego quando i testi li richiedono.
/// `loaded` evita di rifarlo (anche nel caso i font non esistano).
pub fn ensure_cjk(ctx: &egui::Context, loaded: &mut bool, texts: &[&str]) {
    if *loaded || !texts.iter().any(|t| needs_cjk(t)) {
        return;
    }
    // Si riparte dalla base tipografica: i font UI non vanno persi.
    let mut fonts = base();
    let mut found = false;
    for (name, path) in CJK_FONTS {
        let Ok(bytes) = std::fs::read(path) else {
            continue;
        };
        fonts
            .font_data
            .insert(name.to_string(), egui::FontData::from_owned(bytes));
        for family in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
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
    // segnato comunque: senza font di sistema non si ritenta a ogni ricerca
    *loaded = true;
}
