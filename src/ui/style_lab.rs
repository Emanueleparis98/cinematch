//! Style lab nativa di LUMEN: cinque schermate statiche con poster reali.
//! Non tocca la logica dell'app: serve solo a giudicare il linguaggio visivo.

use egui::{Align, Align2, FontFamily, FontId, Rect, Rounding, Sense, Stroke};

use crate::domain::model::Movie;
use crate::domain::suggestions::Suggestion;
use crate::domain::taxonomy::Taxonomy;
use crate::ui::fonts;
use crate::ui::images::ImageStore;
use crate::ui::lumen::{self, *};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum LabScreen {
    Home,
    Results,
    Grid,
    Detail,
    Settings,
}

pub struct LabState {
    pub screen: LabScreen,
    pub wide: bool,
    pub started: bool,
    pub fonts_ready: bool,
}

impl Default for LabState {
    fn default() -> Self {
        Self {
            screen: LabScreen::Home,
            wide: false,
            started: false,
            fonts_ready: false,
        }
    }
}

pub fn show(
    ctx: &egui::Context,
    images: &ImageStore,
    movies: &[Movie],
    suggestions: &[Suggestion],
    tax: &Taxonomy,
    state: &mut LabState,
) -> bool {
    if !state.fonts_ready {
        state.fonts_ready = true;
        ctx.request_repaint();
        return false;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Num1)) {
        state.screen = LabScreen::Home;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Num2)) {
        state.screen = LabScreen::Results;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Num3)) {
        state.screen = LabScreen::Grid;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Num4)) {
        state.screen = LabScreen::Detail;
    }
    if ctx.input(|i| i.key_pressed(egui::Key::Num5)) {
        state.screen = LabScreen::Settings;
    }
    let target_zoom = if state.wide { 0.656 } else { 1.0 };
    if (ctx.zoom_factor() - target_zoom).abs() > 0.002 {
        ctx.set_zoom_factor(target_zoom);
        ctx.request_repaint();
    }
    if !state.started {
        state.started = true;
        ctx.send_viewport_cmd(egui::viewport::ViewportCommand::InnerSize(
            egui::vec2(800.0, 600.0),
        ));
        ctx.request_repaint();
    }

    let fallback = if movies.is_empty() {
        Some(fallback_movies())
    } else {
        None
    };
    let list = fallback.as_deref().unwrap_or(movies);
    let mut close = ctx.input(|i| i.key_pressed(egui::Key::Escape));

    egui::CentralPanel::default()
        .frame(egui::Frame::none().fill(INK))
        .show(ctx, |ui| {
            let full = ui.max_rect();
            let design_w = if state.wide { 1220.0 } else { 800.0 };
            let design_h = if state.wide { 660.0 } else { 600.0 };
            let design = Rect::from_min_size(full.min, egui::vec2(design_w, design_h));
            ui.painter()
                .rect_filled(design, Rounding::ZERO, INK);
            ui.painter().rect_stroke(
                design,
                Rounding::ZERO,
                Stroke::new(1.0_f32, BORDER),
            );
            let top_h = 48.0;
            let rail_w = 72.0;
            let status_h = 28.0;
            let content_rect = Rect::from_min_max(
                egui::pos2(design.left() + rail_w, design.top() + top_h),
                egui::pos2(design.right() - 12.0, design.bottom() - status_h),
            );
            let compact = content_rect.width() < 900.0;

            // Alone caldo dietro la home, ritagliato nel contenuto.
            if state.screen == LabScreen::Home {
                let painter = ui.painter().with_clip_rect(content_rect);
                warm_spill(
                    &painter,
                    content_rect.left_top() + egui::vec2(240.0, 110.0),
                    340.0,
                );
            }

            // Top strip
            let top_rect = Rect::from_min_size(design.min, egui::vec2(design.width(), top_h));
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(top_rect), |ui| {
                top_strip(ui, top_rect, state, &mut close);
            });

            // Rail
            let rail_rect = Rect::from_min_max(
                egui::pos2(design.left(), design.top() + top_h),
                egui::pos2(design.left() + rail_w, design.bottom() - status_h),
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rail_rect), |ui| {
                rail(ui, rail_rect, state);
            });

            // Contenuto
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(content_rect), |ui| {
                ui.painter()
                    .rect_filled(content_rect, Rounding::ZERO, INK);
                egui::ScrollArea::vertical()
                    .id_salt("lumen-lab-scroll")
                    .max_height(content_rect.height())
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let avail = ui.available_width();
                        let content_w = (avail - if compact { 48.0 } else { 68.0 })
                            .clamp(280.0, 1080.0);
                        let pad = ((avail - content_w) * 0.5).max(24.0);
                        ui.horizontal(|ui| {
                            ui.add_space(pad);
                            ui.vertical(|ui| {
                                ui.set_width(content_w);
                                ui.add_space(26.0);
                                match state.screen {
                                    LabScreen::Home => {
                                        home(ui, images, list, suggestions, compact)
                                    }
                                    LabScreen::Results => {
                                        results(ui, images, list, tax, compact)
                                    }
                                    LabScreen::Grid => {
                                        grid_screen(ui, images, list, compact)
                                    }
                                    LabScreen::Detail => {
                                        detail_screen(ui, images, list, tax, compact)
                                    }
                                    LabScreen::Settings => {
                                        settings_screen(ui, compact)
                                    }
                                }
                                ui.add_space(30.0);
                            });
                        });
                    });
            });

            // Status
            let status_rect = Rect::from_min_max(
                egui::pos2(design.left(), design.bottom() - status_h),
                design.max,
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(status_rect), |ui| {
                status_bar(ui, status_rect, state);
            });

            if full.height() > design.height() + 24.0 {
                ui.painter().text(
                    design.left_bottom() + egui::vec2(0.0, 16.0),
                    Align2::LEFT_TOP,
                    format!(
                        "ANTEPRIMA {} × {} · SCALA {:.2}",
                        design_w as u32,
                        design_h as u32,
                        target_zoom
                    ),
                    FontId::new(
                        11.0,
                        FontFamily::Name(fonts::LUMEN_MONO.into()),
                    ),
                    FAINT,
                );
            }
        });

    close
}

// -------------------------------------------------------------- chrome -----

fn top_strip(ui: &mut egui::Ui, rect: Rect, state: &mut LabState, close: &mut bool) {
    let drag = ui.interact(rect, egui::Id::new("lumen-drag"), Sense::click_and_drag());
    if drag.drag_started() {
        ui.ctx()
            .send_viewport_cmd(egui::viewport::ViewportCommand::StartDrag);
    }
    ui.painter().rect_filled(rect, Rounding::ZERO, INK);
    ui.painter().line_segment(
        [rect.left_bottom(), rect.right_bottom()],
        Stroke::new(1.0_f32, BORDER),
    );
    ui.horizontal(|ui| {
        ui.add_space(20.0);
        ui.label(display(22.0, "CINE"));
        ui.label(display_color(22.0, "MATCH", EMBER));
        ui.add_space(14.0);
        ui.label(mono(9.0, "LUMEN / STYLE LAB").color(FAINT));
        ui.add_space(14.0);
        ui.label(
            mono(9.0, screen_name(state.screen).to_uppercase()).color(MUTED),
        );
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            if size_tab(ui, "CHIUDI", false) {
                *close = true;
            }
            ui.add_space(10.0);
            if size_tab(ui, "1220", state.wide) {
                state.wide = true;
            }
            if size_tab(ui, "800", !state.wide) {
                state.wide = false;
            }
            ui.add_space(8.0);
            ui.label(mono(9.0, "FORMATO").color(FAINT));
        });
    });
}

fn size_tab(ui: &mut egui::Ui, text: &str, active: bool) -> bool {
    let font = FontId::new(10.0, FontFamily::Name(fonts::LUMEN_MONO.into()));
    let width = ui
        .fonts(|f| f.layout_no_wrap(text.to_string(), font.clone(), MUTED).size().x)
        + 18.0;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 30.0), Sense::click());
    let color = if active || resp.hovered() { IVORY } else { FAINT };
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        font,
        color,
    );
    if active {
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 6.0, rect.bottom() - 1.0),
                egui::pos2(rect.right() - 6.0, rect.bottom() - 1.0),
            ],
            Stroke::new(2.0_f32, EMBER),
        );
    }
    resp.on_hover_cursor(egui::CursorIcon::PointingHand).clicked()
}

fn rail(ui: &mut egui::Ui, rect: Rect, state: &mut LabState) {
    ui.painter().rect_filled(rect, Rounding::ZERO, DEEP);
    ui.painter().line_segment(
        [rect.right_top(), rect.right_bottom()],
        Stroke::new(1.0_f32, BORDER),
    );
    ui.add_space(12.0);
    for (screen, icon, label) in [
        (LabScreen::Home, Icon::Home, "Home"),
        (LabScreen::Results, Icon::Results, "Risultati"),
        (LabScreen::Grid, Icon::Grid, "Griglia"),
        (LabScreen::Detail, Icon::Detail, "Scheda"),
        (LabScreen::Settings, Icon::Settings, "Setup"),
    ] {
        if lumen::nav_item(ui, icon, label, state.screen == screen).clicked() {
            state.screen = screen;
        }
    }
    ui.with_layout(egui::Layout::bottom_up(Align::LEFT), |ui| {
        ui.add_space(14.0);
        ui.horizontal(|ui| {
            ui.add_space(12.0);
            ui.label(mono(9.0, "0.3 LAB").color(FAINT));
        });
        ui.add_space(6.0);
    });
}

fn status_bar(ui: &mut egui::Ui, rect: Rect, state: &LabState) {
    ui.painter().rect_filled(rect, Rounding::ZERO, INK);
    ui.painter().line_segment(
        [rect.left_top(), rect.right_top()],
        Stroke::new(1.0_f32, BORDER),
    );
    ui.horizontal(|ui| {
        ui.add_space(16.0);
        ui.label(mono(9.0, "LUMEN / STYLE LAB").color(FAINT));
        ui.add_space(10.0);
        ui.label(mono(9.0, "PROTOTIPO STATICO · NESSUNA FUNZIONE REALE").color(FAINT));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.add_space(16.0);
            ui.label(mono(9.0, "v0.3.0-lab").color(MUTED));
            ui.add_space(14.0);
            ui.label(
                mono(
                    9.0,
                    if state.wide {
                        "1220 × 660 · SCALA 0.66"
                    } else {
                        "800 × 600"
                    },
                )
                .color(FAINT),
            );
        });
    });
}

fn screen_name(screen: LabScreen) -> &'static str {
    match screen {
        LabScreen::Home => "Home",
        LabScreen::Results => "Risultati",
        LabScreen::Grid => "Griglia",
        LabScreen::Detail => "Scheda",
        LabScreen::Settings => "Impostazioni",
    }
}

// --------------------------------------------------------------- home -----

fn home(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    suggestions: &[Suggestion],
    compact: bool,
) {
    let width = ui.available_width();
    if compact {
        hero_left(ui, width, true);
        ui.add_space(20.0);
        poster_strip(ui, images, movies, width);
    } else {
        let left_w = (width * 0.56).floor();
        let right_w = width - left_w - 30.0;
        ui.horizontal_top(|ui| {
            ui.allocate_ui_with_layout(
                egui::vec2(left_w, 0.0),
                egui::Layout::top_down(Align::LEFT),
                |ui| hero_left(ui, left_w, false),
            );
            ui.add_space(30.0);
            ui.allocate_ui_with_layout(
                egui::vec2(right_w, 0.0),
                egui::Layout::top_down(Align::LEFT),
                |ui| collage(ui, images, movies, right_w),
            );
        });
    }

    ui.add_space(26.0);
    ui.horizontal(|ui| {
        ui.label(label("Parti da"));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            lumen::link(ui, "Sorprendimi ↗", EMBER);
            ui.add_space(12.0);
            ui.label(mono(9.0, "04 SPUNTI LOCALI").color(FAINT));
        });
    });
    ui.add_space(8.0);
    lumen::hairline(ui);
    ui.add_space(12.0);

    let gap = 10.0;
    let card_w = ((width - gap) / 2.0).floor();
    for row in 0..2 {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for col in 0..2 {
                let i = row * 2 + col;
                let (tag, title, desc) = suggestion_text(suggestions, i);
                ui.allocate_ui_with_layout(
                    egui::vec2(card_w, 0.0),
                    egui::Layout::top_down(Align::LEFT),
                    |ui| suggestion_card(ui, card_w, i, tag, title, desc),
                );
            }
        });
        ui.add_space(gap);
    }
}

fn hero_left(ui: &mut egui::Ui, width: f32, compact: bool) {
    ui.label(mono(10.0, "SALA 01 / RICERCA").color(EMBER));
    ui.add_space(8.0);
    let size = if compact { 38.0 } else { 56.0 };
    ui.label(
        display(size, "Cosa guardiamo\nstasera?")
            .line_height(Some(size * 0.94)),
    );
    ui.add_space(12.0);
    ui.label(
        body(
            if compact { 12.0 } else { 13.0 },
            "Descrivi un’atmosfera, un regista o un dettaglio. CineMatch cerca film reali su TMDB e ti mostra perché sono lì.",
        )
        .color(MUTED),
    );
    ui.add_space(20.0);
    composer(ui, width, compact);
}

fn composer(ui: &mut egui::Ui, width: f32, compact: bool) {
    let h = 56.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, h), Sense::hover());
    ui.painter().rect_filled(rect, Rounding::same(RADIUS), DEEP);
    ui.painter().rect_stroke(
        rect,
        Rounding::same(RADIUS),
        Stroke::new(1.0_f32, BORDER),
    );
    let btn_w = if compact { 82.0 } else { 100.0 };
    let btn = Rect::from_min_size(
        egui::pos2(rect.right() - btn_w - 8.0, rect.top() + 8.0),
        egui::vec2(btn_w, h - 16.0),
    );
    ui.painter()
        .rect_filled(btn, Rounding::same(RADIUS_SM), EMBER);
    ui.painter().text(
        btn.center(),
        Align2::CENTER_CENTER,
        "CERCA",
        FontId::new(10.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        INK,
    );
    ui.painter().text(
        rect.left_center() + egui::vec2(16.0, 0.0),
        Align2::LEFT_CENTER,
        if compact {
            "Un film intenso, ben diretto…"
        } else {
            "Un film intenso, ben diretto, non troppo lungo…"
        },
        FontId::proportional(13.0),
        MUTED,
    );
    if !compact {
        let kbd = Rect::from_min_size(
            egui::pos2(btn.left() - 58.0, rect.center().y - 11.0),
            egui::vec2(46.0, 22.0),
        );
        ui.painter().rect_stroke(
            kbd,
            Rounding::same(RADIUS_SM),
            Stroke::new(1.0_f32, BORDER),
        );
        ui.painter().text(
            kbd.center(),
            Align2::CENTER_CENTER,
            "INVIO",
            FontId::new(8.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
            FAINT,
        );
    }
}

fn collage(ui: &mut egui::Ui, images: &ImageStore, movies: &[Movie], width: f32) {
    ui.label(label("In sala"));
    ui.add_space(8.0);
    let big_w = (width * 0.55).min(220.0);
    let small_w = (width - big_w - 12.0).min(100.0);
    let group_w = big_w + 12.0 + small_w;
    ui.horizontal_top(|ui| {
        ui.add_space(((width - group_w) * 0.5).max(0.0));
        lumen::poster(
            ui,
            images,
            movies.get(0),
            big_w,
            big_w * 1.5,
            true,
        );
        ui.add_space(12.0);
        ui.vertical(|ui| {
            ui.add_space(18.0);
            lumen::poster(
                ui,
                images,
                movies.get(1),
                small_w,
                small_w * 1.5,
                false,
            );
            ui.add_space(12.0);
            lumen::poster(
                ui,
                images,
                movies.get(2),
                small_w,
                small_w * 1.5,
                false,
            );
        });
    });
}

fn poster_strip(ui: &mut egui::Ui, images: &ImageStore, movies: &[Movie], width: f32) {
    let gap = 8.0;
    let w = ((width - gap * 2.0) / 3.0).floor();
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = gap;
        for i in 0..3 {
            lumen::poster(ui, images, movies.get(i), w, w * 1.5, i == 0);
        }
    });
}

fn suggestion_text<'a>(
    suggestions: &'a [Suggestion],
    i: usize,
) -> (&'a str, &'a str, &'a str) {
    const TAGS: [&str; 4] = ["CORRENTE", "ATMOSFERA", "PERIODO", "DURATA"];
    const TITLES: [&str; 4] = [
        "Nouvelle Vague, senza museo",
        "Un noir che non spiega tutto",
        "Italia anni ’70, fuori dai soliti nomi",
        "Un film sotto i 100 minuti",
    ];
    const DESC: [&str; 4] = [
        "La Parigi che si muove, tra strade, jazz e macchina da presa a spalla.",
        "Notte, pioggia, una città che sembra complice.",
        "Poliziottesco, commedia amara e cinema civile da riscoprire.",
        "Serata piena, tempo stretto, nessun minuto di troppo.",
    ];
    match suggestions.get(i) {
        Some(s) => (s.tag.as_str(), s.prompt.as_str(), s.desc.as_str()),
        None => (TAGS[i % 4], TITLES[i % 4], DESC[i % 4]),
    }
}

fn suggestion_card(
    ui: &mut egui::Ui,
    width: f32,
    index: usize,
    tag: &str,
    title: &str,
    desc: &str,
) {
    let (rect, resp) = ui.allocate_exact_size(
        egui::vec2(width, 118.0),
        Sense::click(),
    );
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    p.rect_filled(
        rect,
        Rounding::same(RADIUS),
        if hover > 0.01 { RAISED } else { SURFACE },
    );
    p.rect_stroke(
        rect,
        Rounding::same(RADIUS),
        Stroke::new(
            1.0_f32,
            if hover > 0.01 { EMBER } else { BORDER },
        ),
    );
    p.text(
        rect.min + egui::vec2(14.0, 12.0),
        Align2::LEFT_TOP,
        format!("{:02} / {}", index + 1, tag.to_uppercase()),
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        EMBER,
    );
    p.text(
        rect.min + egui::vec2(14.0, 32.0),
        Align2::LEFT_TOP,
        crate::ui::theme::truncate(title, 34),
        FontId::new(18.0, FontFamily::Name(fonts::LUMEN_DISPLAY.into())),
        IVORY,
    );
    p.text(
        rect.min + egui::vec2(14.0, 60.0),
        Align2::LEFT_TOP,
        crate::ui::theme::truncate(desc, 72),
        FontId::proportional(11.0),
        MUTED,
    );
    p.text(
        rect.right_bottom() + egui::vec2(-14.0, -12.0),
        Align2::RIGHT_BOTTOM,
        "↗",
        FontId::proportional(13.0),
        if hover > 0.01 { EMBER } else { FAINT },
    );
}

// ------------------------------------------------------------ risultati -----

fn results(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    tax: &Taxonomy,
    compact: bool,
) {
    user_message(ui, "Un film intenso, ben diretto, non troppo lungo. Ho già visto i titoli più ovvi.");
    ui.add_space(18.0);
    ui.horizontal(|ui| {
        ui.label(mono(10.0, "CINEMATCH").color(EMBER));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(mono(9.0, "SELEZIONE · 5 FILM").color(FAINT));
        });
    });
    ui.add_space(8.0);
    lumen::hairline(ui);
    ui.add_space(12.0);
    ui.label(
        body(
            if compact { 13.0 } else { 14.0 },
            "Ho scelto film dalla cura formale forte: tensione, immagini nette e un finale che resta.",
        ),
    );
    ui.add_space(6.0);
    ui.horizontal(|ui| {
        ui.label(label("Criteri"));
        ui.label(mono(10.0, "Voto ≥ 7 · Non troppo lungo · Da scoprire").color(MUTED));
    });
    ui.add_space(16.0);

    let hero = movies.get(0);
    hero_card(ui, images, hero, tax, compact);
    ui.add_space(12.0);

    let cols = if compact { 1 } else { 2 };
    let gap = 10.0;
    let card_w = ((ui.available_width() - gap * (cols - 1) as f32) / cols as f32).floor();
    for row in 0..2 {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            for col in 0..cols {
                let i = 1 + row * cols + col;
                ui.allocate_ui_with_layout(
                    egui::vec2(card_w, 0.0),
                    egui::Layout::top_down(Align::LEFT),
                    |ui| compact_card(ui, images, movies.get(i), tax, card_w),
                );
            }
        });
        ui.add_space(gap);
    }

    ui.add_space(4.0);
    lumen::hairline(ui);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        lumen::link(ui, "Vedi tutti i 2.294 in griglia ↗", EMBER);
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            ui.label(mono(9.0, "2.294 RISULTATI · TMDB").color(FAINT));
        });
    });
}

fn user_message(ui: &mut egui::Ui, text: &str) {
    let avail = ui.available_width();
    let galley = ui.painter().layout(
        text.to_string(),
        FontId::proportional(13.0),
        IVORY,
        (avail * 0.7).min(620.0),
    );
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(avail, galley.size().y + 8.0),
        Sense::hover(),
    );
    let right = rect.right() - 28.0;
    let left = (right - galley.size().x).max(rect.left());
    ui.painter().galley(
        egui::pos2(left, rect.top() + 2.0),
        galley.clone(),
        IVORY,
    );
    ui.painter().rect_filled(
        Rect::from_min_max(
            egui::pos2(right + 10.0, rect.top() + 2.0),
            egui::pos2(right + 12.0, rect.top() + 2.0 + galley.size().y),
        ),
        Rounding::ZERO,
        EMBER,
    );
    ui.painter().text(
        egui::pos2(rect.right(), rect.top() + 3.0),
        Align2::RIGHT_TOP,
        "TU",
        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
        FAINT,
    );
}

fn hero_card(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movie: Option<&Movie>,
    tax: &Taxonomy,
    compact: bool,
) {
    let poster_w = if compact { 110.0 } else { 150.0 };
    let frame = egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(RADIUS))
        .inner_margin(egui::Margin::same(12.0));
    frame.show(ui, |ui| {
        ui.horizontal_top(|ui| {
            lumen::poster(ui, images, movie, poster_w, poster_w * 1.5, true);
            ui.add_space(14.0);
            ui.vertical(|ui| {
                ui.label(
                    display(
                        if compact { 22.0 } else { 28.0 },
                        movie.map(|m| m.title.as_str()).unwrap_or("Cleo dalle 5 alle 7"),
                    ),
                );
                ui.label(
                    mono(
                        10.0,
                        movie_meta(movie, tax).as_str(),
                    )
                    .color(FAINT),
                );
                ui.add_space(8.0);
                ui.label(
                    body(13.0, movie_why(movie)).color(MUTED),
                );
                ui.add_space(8.0);
                ui.label(mono(10.0, "IN ABBONAMENTO · THE FILM CLUB").color(CELADON));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    lumen::link(ui, "Apri scheda", EMBER);
                    ui.add_space(12.0);
                    lumen::link(ui, "Film simili", MUTED);
                });
            });
        });
    });
}

fn compact_card(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movie: Option<&Movie>,
    tax: &Taxonomy,
    width: f32,
) {
    let frame = egui::Frame::none()
        .fill(SURFACE)
        .stroke(Stroke::new(1.0_f32, BORDER))
        .rounding(Rounding::same(RADIUS))
        .inner_margin(egui::Margin::same(10.0));
    frame.show(ui, |ui| {
        ui.set_width(width - 22.0);
        ui.horizontal_top(|ui| {
            lumen::poster(ui, images, movie, 64.0, 96.0, false);
            ui.add_space(10.0);
            ui.vertical(|ui| {
                ui.label(
                    bold(15.0, movie.map(|m| m.title.as_str()).unwrap_or("Play Time")),
                );
                ui.label(mono(9.0, movie_meta(movie, tax).as_str()).color(FAINT));
                ui.add_space(4.0);
                ui.label(
                    body(11.0, movie_why(movie)).color(MUTED),
                );
                ui.add_space(6.0);
                lumen::link(ui, "Apri scheda", EMBER);
            });
        });
    });
}

fn movie_meta(movie: Option<&Movie>, tax: &Taxonomy) -> String {
    match movie {
        Some(m) => {
            let genres = m
                .genre_ids
                .iter()
                .take(2)
                .filter_map(|g| tax.genre_name(*g))
                .collect::<Vec<_>>()
                .join(", ");
            format!(
                "{} · {} · TMDB {:.1}",
                m.year.map(|y| y.to_string()).unwrap_or_else(|| "—".into()),
                genres,
                m.rating
            )
        }
        None => "1962 · Dramma · TMDB 7.7".to_string(),
    }
}

fn movie_why(movie: Option<&Movie>) -> String {
    match movie {
        Some(m) if !m.overview.trim().is_empty() => {
            crate::ui::theme::truncate(&m.overview, 190)
        }
        _ => "La giornata di una cantante che aspetta un responso medico: Parigi vista da vicino, un passo alla volta. Un ritratto intimo e luminoso, dove la grazia visiva accompagna paura e scoperta di sé.".to_string(),
    }
}

// ------------------------------------------------------------- griglia -----

fn grid_screen(ui: &mut egui::Ui, images: &ImageStore, movies: &[Movie], compact: bool) {
    ui.horizontal(|ui| {
        ui.label(mono(11.0, "CATALOGO").color(IVORY));
        ui.add_space(8.0);
        ui.label(mono(10.0, "10.000 FILM").color(FAINT));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            lumen::button_sized(ui, "Filtri 3", false, 92.0);
            ui.add_space(8.0);
            if compact {
                search_box(ui, 200.0, "Cerca…");
            } else {
                ui.label(mono(10.0, "ORDINA · POPOLARITÀ").color(MUTED));
                ui.add_space(8.0);
                search_box(ui, 300.0, "Cerca o descrivi…");
            }
        });
    });
    ui.add_space(12.0);
    lumen::hairline(ui);
    ui.add_space(14.0);

    let show_filters = !compact;
    let filter_w = 240.0;
    ui.horizontal_top(|ui| {
        if show_filters {
            ui.allocate_ui_with_layout(
                egui::vec2(filter_w, 0.0),
                egui::Layout::top_down(Align::LEFT),
                |ui| filters(ui, filter_w),
            );
            ui.add_space(18.0);
        }
        let grid_w = ui.available_width();
        let cols = if compact { 3 } else { 4 };
        let gap = 10.0;
        let card_w = ((grid_w - gap * (cols - 1) as f32) / cols as f32).floor();
        let poster_h = card_w * 1.5;
        ui.allocate_ui_with_layout(
            egui::vec2(grid_w, 0.0),
            egui::Layout::top_down(Align::LEFT),
            |ui| {
                for row in 0..3 {
                    ui.horizontal_top(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for col in 0..cols {
                            let i = row * cols + col;
                            let m = movies.get(i);
                            ui.allocate_ui_with_layout(
                                egui::vec2(card_w, 0.0),
                                egui::Layout::top_down(Align::LEFT),
                                |ui| {
                                    lumen::poster(
                                        ui,
                                        images,
                                        m,
                                        card_w,
                                        poster_h,
                                        false,
                                    );
                                    ui.add_space(5.0);
                                    ui.label(
                                        bold(
                                            12.0,
                                            m.map(|m| m.title.as_str())
                                                .unwrap_or("Titolo"),
                                        ),
                                    );
                                    ui.label(
                                        mono(
                                            9.0,
                                            &m.map(|m| {
                                                format!(
                                                    "{} · {:.1}",
                                                    m.year.unwrap_or(0),
                                                    m.rating
                                                )
                                            })
                                            .unwrap_or_else(|| "1962 · 7.7".into()),
                                        )
                                        .color(FAINT),
                                    );
                                },
                            );
                        }
                    });
                    ui.add_space(14.0);
                }
            },
        );
    });
    ui.add_space(4.0);
    lumen::hairline(ui);
    ui.add_space(8.0);
    ui.horizontal(|ui| {
        ui.label(mono(9.0, "PAGINA 1 DI 500").color(FAINT));
        ui.with_layout(egui::Layout::right_to_left(Align::Center), |ui| {
            lumen::button_sized(ui, "›", false, 28.0);
            ui.add_space(6.0);
            lumen::button_sized(ui, "‹", false, 28.0);
        });
    });
}

fn search_box(ui: &mut egui::Ui, width: f32, placeholder: &str) {
    let (r, _) = ui.allocate_exact_size(egui::vec2(width, 36.0), Sense::hover());
    ui.painter().rect_filled(r, Rounding::same(RADIUS_SM), DEEP);
    ui.painter().rect_stroke(
        r,
        Rounding::same(RADIUS_SM),
        Stroke::new(1.0_f32, BORDER),
    );
    ui.painter().text(
        r.left_center() + egui::vec2(12.0, 0.0),
        Align2::LEFT_CENTER,
        placeholder,
        FontId::proportional(12.0),
        FAINT,
    );
}

fn filters(ui: &mut egui::Ui, width: f32) {
    ui.set_width(width);
    ui.label(mono(10.0, "FILTRI").color(IVORY));
    ui.add_space(10.0);
    for (title, rows) in [
        ("PERIODO", ["Anni 2020", "Anni 2010", "Anni ’90"]),
        ("GENERE", ["Thriller", "Dramma", "Commedia"]),
        ("PAESE", ["Stati Uniti", "Italia", "Francia"]),
        ("METRICHE", ["Voto ≥ 7", "Durata < 120 min", "Voti ≥ 1.000"]),
    ] {
        ui.label(label(title));
        ui.add_space(6.0);
        for (i, row) in rows.iter().enumerate() {
            let active = i == 0;
            let (rect, resp) = ui.allocate_exact_size(
                egui::vec2(width, 26.0),
                Sense::click(),
            );
            if resp.hovered() {
                ui.painter()
                    .rect_filled(rect, Rounding::ZERO, alpha(SURFACE, 150));
            }
            let box_rect = Rect::from_min_size(
                egui::pos2(rect.left() + 2.0, rect.center().y - 6.0),
                egui::vec2(12.0, 12.0),
            );
            ui.painter().rect_stroke(
                box_rect,
                Rounding::ZERO,
                Stroke::new(1.0_f32, if active { EMBER } else { BORDER }),
            );
            if active {
                ui.painter()
                    .rect_filled(box_rect.shrink(3.0), Rounding::ZERO, EMBER);
            }
            ui.painter().text(
                egui::pos2(rect.left() + 22.0, rect.center().y),
                Align2::LEFT_CENTER,
                *row,
                FontId::proportional(11.0),
                if active { IVORY } else { MUTED },
            );
        }
        ui.add_space(12.0);
        lumen::hairline(ui);
        ui.add_space(10.0);
    }
    ui.label(mono(9.0, "GUIDA ALLE CORRENTI (17)").color(EMBER));
}

// -------------------------------------------------------------- scheda -----

fn detail_screen(
    ui: &mut egui::Ui,
    images: &ImageStore,
    movies: &[Movie],
    tax: &Taxonomy,
    compact: bool,
) {
    let movie = movies.get(0);
    let width = ui.available_width().min(1000.0);
    let pad = ((ui.available_width() - width) * 0.5).max(0.0);
    ui.horizontal(|ui| {
        ui.add_space(pad);
        let frame = egui::Frame::none()
            .fill(SURFACE)
            .stroke(Stroke::new(1.0_f32, BORDER_HI))
            .rounding(Rounding::same(RADIUS))
            .inner_margin(egui::Margin::same(if compact { 14.0 } else { 20.0 }));
        let outer = frame.show(ui, |ui| {
            ui.with_layout(egui::Layout::top_down(egui::Align::LEFT), |ui| {
                ui.set_width(width - if compact { 28.0 } else { 40.0 });
                lumen::backdrop(
                    ui,
                    images,
                    movie,
                    width - if compact { 28.0 } else { 40.0 },
                    if compact { 140.0 } else { 220.0 },
                );
                ui.add_space(12.0);
                ui.horizontal_top(|ui| {
                    let poster_w = if compact { 100.0 } else { 150.0 };
                    lumen::poster(ui, images, movie, poster_w, poster_w * 1.5, true);
                    ui.add_space(14.0);
                    ui.vertical(|ui| {
                        ui.label(display(
                            if compact { 26.0 } else { 36.0 },
                            movie.map(|m| m.title.as_str()).unwrap_or("Play Time"),
                        ));
                        ui.label(
                            mono(10.0, movie_meta(movie, tax).as_str()).color(FAINT),
                        );
                        ui.add_space(10.0);
                        ui.label(body(13.0, movie_why(movie)).color(MUTED));
                        ui.add_space(12.0);
                        lumen::hairline(ui);
                        ui.add_space(9.0);
                        let tmdb = movie
                            .map(|m| {
                                format!(
                                    "{:.1} · {}",
                                    m.rating,
                                    crate::ui::theme::thousands(m.votes)
                                )
                            })
                            .unwrap_or_else(|| "—".into());
                        ui.columns(3, |cols| {
                            fact(&mut cols[0], "REGIA", "—");
                            fact(&mut cols[1], "VOTO TMDB", &tmdb);
                            fact(&mut cols[2], "VOTO IMDb", "—");
                        });
                        ui.add_space(9.0);
                        lumen::hairline(ui);
                        ui.add_space(10.0);
                        ui.horizontal(|ui| {
                            lumen::button_sized(ui, "Cerca il titolo", true, 140.0);
                            ui.add_space(8.0);
                            lumen::button_sized(ui, "Riassumi", false, 100.0);
                            ui.add_space(8.0);
                            lumen::link(ui, "Scheda TMDB ↗", EMBER);
                        });
                    });
                });
                ui.add_space(14.0);
                lumen::hairline(ui);
                ui.add_space(8.0);
                ui.label(label("Simili"));
                ui.add_space(6.0);
                ui.horizontal(|ui| {
                    for i in 1..5 {
                        lumen::poster(ui, images, movies.get(i), 82.0, 123.0, false);
                        ui.add_space(8.0);
                    }
                });
            });
        });
        let r = outer.response.rect;
        lumen::seam(
            ui.painter(),
            Rect::from_min_size(r.min, egui::vec2(r.width(), 1.5)),
        );
        let c = r.right_top() + egui::vec2(-22.0, 22.0);
        ui.painter().rect_stroke(
            Rect::from_center_size(c, egui::vec2(26.0, 26.0)),
            Rounding::same(RADIUS_SM),
            Stroke::new(1.0_f32, BORDER),
        );
        ui.painter().line_segment(
            [c + egui::vec2(-4.0, -4.0), c + egui::vec2(4.0, 4.0)],
            Stroke::new(1.4_f32, IVORY),
        );
        ui.painter().line_segment(
            [c + egui::vec2(4.0, -4.0), c + egui::vec2(-4.0, 4.0)],
            Stroke::new(1.4_f32, IVORY),
        );
    });
}

fn fact(ui: &mut egui::Ui, label_text: &str, value: &str) {
    ui.label(label(label_text));
    ui.add_space(3.0);
    ui.label(body(11.0, value));
}

// ---------------------------------------------------------- impostazioni -----

fn settings_screen(ui: &mut egui::Ui, compact: bool) {
    let width = ui.available_width();
    let nav_w = if compact { 140.0 } else { 190.0 };
    ui.horizontal_top(|ui| {
        ui.allocate_ui_with_layout(
            egui::vec2(nav_w, 0.0),
            egui::Layout::top_down(Align::LEFT),
            |ui| {
                ui.set_width(nav_w);
                ui.add_space(4.0);
                ui.label(label("Impostazioni"));
                ui.add_space(10.0);
                for (i, item) in ["Interprete", "Aggiornamenti", "Dati locali", "Informazioni"]
                    .iter()
                    .enumerate()
                {
                    let (rect, resp) = ui.allocate_exact_size(
                        egui::vec2(nav_w, 36.0),
                        Sense::click(),
                    );
                    let active = i == 0;
                    if active || resp.hovered() {
                        ui.painter()
                            .rect_filled(rect, Rounding::ZERO, SURFACE);
                    }
                    if active {
                        ui.painter().rect_filled(
                            Rect::from_min_size(rect.min, egui::vec2(2.0, rect.height())),
                            Rounding::ZERO,
                            EMBER,
                        );
                    }
                    ui.painter().text(
                        egui::pos2(rect.left() + 16.0, rect.center().y),
                        Align2::LEFT_CENTER,
                        item.to_uppercase(),
                        FontId::new(9.0, FontFamily::Name(fonts::LUMEN_MONO.into())),
                        if active { IVORY } else { MUTED },
                    );
                }
            },
        );
        ui.add_space(24.0);
        ui.vertical(|ui| {
            ui.set_width((width - nav_w - 24.0).max(260.0));
            ui.label(mono(10.0, "IMPOSTAZIONI").color(EMBER));
            ui.add_space(6.0);
            ui.label(display(30.0, "Interprete delle richieste"));
            ui.add_space(8.0);
            ui.label(
                body(11.0, "Le regole funzionano offline. Un modello è opzionale: i film arrivano sempre da TMDB.")
                    .color(MUTED),
            );
            ui.add_space(12.0);
            lumen::hairline(ui);
            ui.add_space(8.0);
            ui.label(mono(10.0, "TMDB").color(IVORY));
            lumen::field(ui, "Chiave API TMDB", "••••••••••••••••", true);
            ui.label(
                mono(9.0, "Read Access Token v4 · la chiave resta su questo PC")
                    .color(CELADON),
            );
            ui.add_space(14.0);
            lumen::hairline(ui);
            ui.add_space(10.0);
            ui.label(mono(10.0, "INTERPRETE").color(IVORY));
            ui.add_space(8.0);
            let gap = 8.0;
            let card_w = ((ui.available_width() - gap) / 2.0).floor();
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                provider(ui, card_w, "Regole offline", "Predefinito, nessuna chiave", false);
                provider(ui, card_w, "DeepSeek", "V4.1 Flash · sforzo massimo", true);
            });
            ui.add_space(gap);
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = gap;
                provider(ui, card_w, "OpenRouter", "Tutti i modelli pubblici", false);
                provider(ui, card_w, "Ollama", "Modello locale sul tuo PC", false);
            });
            lumen::field(ui, "Chiave API DeepSeek", "••••••••••••••", true);
            ui.add_space(10.0);
            ui.label(label("Sforzo di ragionamento"));
            ui.add_space(5.0);
            ui.horizontal(|ui| {
                for (i, text) in ["Massimo", "Alto", "Medio", "Basso"].iter().enumerate() {
                    choice(ui, text, i == 0);
                    ui.add_space(6.0);
                }
            });
            ui.add_space(14.0);
            ui.horizontal(|ui| {
                lumen::button_sized(ui, "Salva", true, 100.0);
                ui.add_space(8.0);
                ui.label(mono(9.0, "La chiave resta in %APPDATA%\\CineMatch").color(FAINT));
            });
        });
    });
}

fn provider(ui: &mut egui::Ui, width: f32, title: &str, desc: &str, active: bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 56.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    ui.painter().rect_filled(
        rect,
        Rounding::same(RADIUS),
        if hover > 0.01 { RAISED } else { SURFACE },
    );
    ui.painter().rect_stroke(
        rect,
        Rounding::same(RADIUS),
        Stroke::new(1.0_f32, if active { EMBER } else { BORDER }),
    );
    ui.painter().text(
        rect.min + egui::vec2(10.0, 9.0),
        Align2::LEFT_TOP,
        title,
        FontId::proportional(12.0),
        IVORY,
    );
    ui.painter().text(
        rect.min + egui::vec2(10.0, 29.0),
        Align2::LEFT_TOP,
        desc,
        FontId::proportional(9.5),
        MUTED,
    );
    if active {
        ui.painter()
            .circle_filled(rect.right_top() + egui::vec2(-10.0, 11.0), 2.5, EMBER);
    }
}

fn choice(ui: &mut egui::Ui, text: &str, active: bool) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(64.0, 26.0), Sense::click());
    ui.painter().rect_filled(
        rect,
        Rounding::same(RADIUS_SM),
        if active { EMBER } else { DEEP },
    );
    ui.painter().rect_stroke(
        rect,
        Rounding::same(RADIUS_SM),
        Stroke::new(1.0_f32, if active { EMBER } else { BORDER }),
    );
    ui.painter().text(
        rect.center(),
        Align2::CENTER_CENTER,
        text,
        FontId::proportional(10.5),
        if active { INK } else { MUTED },
    );
    let _ = resp;
}

// ------------------------------------------------------------ fallback -----

fn fallback_movies() -> Vec<Movie> {
    const TITLES: [&str; 8] = [
        "Cleo dalle 5 alle 7",
        "Play Time",
        "Il buco",
        "Bella di giorno",
        "Fino all’ultimo respiro",
        "Lola",
        "Les parapluies de Cherbourg",
        "Pierrot le fou",
    ];
    TITLES
        .iter()
        .enumerate()
        .map(|(i, title)| Movie {
            id: i as u64 + 1,
            title: title.to_string(),
            original_title: title.to_string(),
            year: Some(1960 + i as u16),
            overview: "Un classico del cinema europeo: eleganza formale, sguardo moderno e una città che diventa scenografia.".to_string(),
            poster_path: None,
            backdrop_path: None,
            rating: 7.4 + (i as f32) * 0.1,
            votes: 1000,
            popularity: 0.0,
            genre_ids: vec![],
            lang: "it".to_string(),
            imdb_id: None,
        })
        .collect()
}
