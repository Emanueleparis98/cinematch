//! Pannello dei filtri: testata sticky (titolo, ricerca, filtri attivi),
//! sezioni a fisarmonica, metriche a menu e guida in una finestra dedicata.
//!
//! La testata resta fuori dallo ScrollArea (vedi app.rs): mentre si scorrono
//! i generi o i paesi, i filtri accesi e la ricerca sono sempre a portata.
//! Lo stato di apertura delle sezioni vive nella memoria di egui, quindi
//! dura quanto la sessione e non tocca il disco.

use egui::{
    Align, Align2, Color32, FontId, Frame, Id, Layout, Margin, Response, RichText, Rounding, Sense,
    Stroke, Ui,
};

use crate::domain::constraints::{movement_recipe, Availability, Prune};
use crate::domain::model::{Facets, Sort};
use crate::domain::taxonomy::{Movement, Taxonomy};
use crate::ui::fonts;
use crate::ui::{lumen, theme};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    All,
    Period,
    Genres,
    Countries,
    Moods,
    Metrics,
}

#[derive(Clone, Debug)]
pub enum FacetAction {
    Decade(String),
    Movement(String),
    Genre(u32),
    Country(String),
    Mood(String),
    /// Tema libero (nome di parola chiave TMDB): proposto dall'interprete AI o
    /// scritto a mano nella sezione temi.
    Keyword(String),
    Rating(Option<f32>),
    MaxRating(Option<f32>),
    Votes(Option<u32>),
    Runtime(Option<(u32, u32)>),
    /// Combinazione suggerita dalla guida per una corrente: decenni, paesi e la
    /// corrente stessa (che porta la sua parola chiave TMDB, dove ne ha una).
    /// Si aggiunge ai filtri gia attivi.
    Combo {
        decades: Vec<String>,
        countries: Vec<String>,
        movements: Vec<String>,
    },
    Clear(Section),
}

// ------------------------------------------------------------- testata -----

/// Titolo, conteggio, azzera, aiuto, ricerca e riepilogo dei filtri attivi.
/// Sta fuori dallo ScrollArea: non scorre mai.
pub fn header(
    ui: &mut Ui,
    tax: &Taxonomy,
    f: &Facets,
    query: &mut String,
    guide_open: &mut bool,
) -> Vec<FacetAction> {
    let mut out = Vec::new();
    let active = f.active_count();

    ui.horizontal(|ui| {
        ui.label(lumen::mono(11.0, "FILTRI").color(lumen::IVORY));
        if active > 0 {
            let (w, _) = badge_size(ui, active);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 15.0), Sense::hover());
            paint_badge(ui.painter(), rect, active);
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if ghost(ui, "Azzera", active > 0).clicked() && active > 0 {
                out.push(FacetAction::Clear(Section::All));
            }
            if help_button(ui).clicked() {
                open_guide(ui, guide_open);
            }
        });
    });

    ui.add_space(6.0);
    search_field(ui, query, "Cerca un filtro…");
    ui.add_space(5.0);

    let pills = active_pills(ui, tax, f);
    out.extend(pills);
    out
}

/// Pulsante della barra di comando: mostra o minimizza il pannello filtri.
/// Il conteggio resta visibile anche a pannello chiuso, cosi' i filtri
/// accesi non spariscono dalla vista mentre si guarda la griglia.
pub fn panel_toggle(ui: &mut Ui, open: bool, active: usize) -> Response {
    let text = ui.painter().layout_no_wrap(
        "Filtri".to_owned(),
        FontId::proportional(11.5),
        Color32::PLACEHOLDER,
    );
    let (badge_w, _) = if active > 0 {
        badge_size(ui, active)
    } else {
        (0.0, 0.0)
    };
    let pad = 9.0;
    let chev_w = 9.0;
    let gap = 4.0;
    let count_gap = if active > 0 { 6.0 } else { 0.0 };
    let w = pad + chev_w + gap + text.size().x + count_gap + badge_w + pad;
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, 26.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    let fill = if open {
        theme::alpha(theme::ACCENT_DIM, (60.0 + 40.0 * hover) as u8)
    } else {
        theme::mix(theme::CARD, theme::CARD_HOVER, hover * 0.7)
    };
    let stroke = theme::mix(
        if open {
            theme::alpha(theme::ACCENT, 130)
        } else {
            theme::BORDER
        },
        theme::ACCENT,
        hover * 0.65,
    );
    p.rect_filled(rect, Rounding::same(8.0), fill);
    p.rect_stroke(rect, Rounding::same(8.0), Stroke::new(1.0_f32, stroke));
    p.text(
        egui::pos2(rect.left() + pad, rect.center().y),
        Align2::LEFT_CENTER,
        if open { "‹" } else { "›" },
        FontId::proportional(15.0),
        if open {
            theme::ACCENT_3
        } else {
            theme::mix(theme::DIM, theme::ACCENT_3, hover)
        },
    );
    let tx = rect.left() + pad + chev_w + gap;
    p.galley(
        egui::pos2(tx, rect.center().y - text.size().y * 0.5),
        text.clone(),
        theme::mix(
            if open { theme::TEXT } else { theme::DIM },
            theme::TEXT,
            hover,
        ),
    );
    if active > 0 {
        let brect = egui::Rect::from_center_size(
            egui::pos2(
                tx + text.size().x + count_gap + badge_w * 0.5,
                rect.center().y,
            ),
            egui::vec2(badge_w, 15.0),
        );
        paint_badge(p, brect, active);
    }
    let tip = if open {
        "Minimizza il pannello dei filtri · Ctrl+B".to_owned()
    } else if active > 0 {
        format!("Mostra il pannello dei filtri · {active} attivi · Ctrl+B")
    } else {
        "Mostra il pannello dei filtri · Ctrl+B".to_owned()
    };
    theme::focus_ring(ui, &resp, 8.0);
    resp.on_hover_text(tip)
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Corpo del pannello: sezioni, guida e note. Scorre sotto la testata.
pub fn sections(
    ui: &mut Ui,
    tax: &Taxonomy,
    f: &Facets,
    prune: &Prune,
    query: &mut String,
    guide_open: &mut bool,
    messages: &[String],
    info: &[String],
    ai_notes: &[String],
) -> Vec<FacetAction> {
    let mut out = Vec::new();
    let q = norm(query);
    let filtering = !q.is_empty();
    let mut drawn = 0usize;

    // ---- periodo ----
    let (mlo, mhi) = movement_window(tax, f);
    let all_decades: Vec<(String, bool, Availability)> = tax
        .decades
        .iter()
        .map(|d| {
            let selected = f.decades.contains(&d.id);
            let av = match (mlo, mhi) {
                (Some(lo), Some(hi)) if d.to < lo || d.from > hi => Availability::blocked(
                    format!("I movimenti scelti coprono il periodo {lo}–{hi}"),
                ),
                _ => Availability::ok(),
            };
            (d.label.clone(), selected, av)
        })
        .collect();
    let title_hit = wanted(&q, "Periodo", PERIOD_ALIASES);
    let (items, show) = filter_items(all_decades, &q, filtering, title_hit);
    if show {
        sep(ui, &mut drawn);
        let acts = section(
            ui,
            "periodo",
            "Periodo",
            f.decades.len(),
            false,
            filtering,
            |ui, out| {
                chip_row(ui, items, out, |label| {
                    let id = tax
                        .decades
                        .iter()
                        .find(|d| d.label == label)
                        .map(|d| d.id.clone())
                        .unwrap_or_default();
                    FacetAction::Decade(id)
                });
            },
        );
        out.extend(acts);
    }

    // ---- generi ----
    let all_genres: Vec<(String, bool, Availability)> = tax
        .genres
        .iter()
        .map(|g| {
            let selected = f.genres.contains(&g.id);
            let excluded = f.without_genres.contains(&g.id);
            let av = if excluded {
                Availability::soft("Escluso: scrivi «senza X» o riattivalo dal pannello".to_string())
            } else {
                prune
                    .genres
                    .get(&g.id)
                    .cloned()
                    .unwrap_or_else(Availability::ok)
            };
            let label = if excluded {
                format!("✕ {}", g.name)
            } else {
                g.name.clone()
            };
            (label, selected, av)
        })
        .collect();
    let title_hit = wanted(&q, "Genere", GENRE_ALIASES);
    let (items, show) = filter_items(all_genres, &q, filtering, title_hit);
    if show {
        sep(ui, &mut drawn);
        let total = items.len();
        let open_id = Id::new("facet-generi-tutti");
        let show_all = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        // I generi principali restano sotto gli occhi; i settoriali arrivano
        // con «Tutti i generi». Le scelte fuori elenco restano sempre visibili.
        let primary: Vec<&str> = tax
            .genres
            .iter()
            .filter(|g| PRIMARY_GENRES.contains(&g.id))
            .map(|g| g.name.as_str())
            .collect();
        let list = if filtering || show_all {
            items
        } else {
            let mut v: Vec<(String, bool, Availability)> = items
                .iter()
                .filter(|(l, _, _)| primary.contains(&l.as_str()) || l.starts_with('✕'))
                .cloned()
                .collect();
            for it in &items {
                if it.1 && !v.iter().any(|x| x.0 == it.0) {
                    v.push(it.clone());
                }
            }
            v
        };
        let acts = section(
            ui,
            "genere",
            "Genere",
            f.genres.len() + f.without_genres.len(),
            true,
            filtering,
            |ui, out| {
                chip_row(ui, list, out, |label| {
                    let name = label.trim_start_matches("✕ ").to_string();
                    let id = tax
                        .genres
                        .iter()
                        .find(|g| g.name == name)
                        .map(|g| g.id)
                        .unwrap_or(0);
                    FacetAction::Genre(id)
                });
                if !filtering && total > primary.len() {
                    ui.add_space(1.0);
                    let label = if show_all {
                        "Meno generi".to_string()
                    } else {
                        format!("Tutti i generi ({total})")
                    };
                    if ghost(ui, &label, true).clicked() {
                        ui.data_mut(|d| d.insert_temp(open_id, !show_all));
                    }
                }
            },
        );
        out.extend(acts);
    }

    // ---- paesi ----
    let all_countries: Vec<(String, bool, Availability)> = tax
        .countries
        .iter()
        .map(|c| {
            let selected = f.countries.contains(&c.code);
            let av = prune
                .countries
                .get(&c.code)
                .cloned()
                .unwrap_or_else(Availability::ok);
            (c.name.clone(), selected, av)
        })
        .collect();
    let title_hit = wanted(&q, "Paese e lingua", COUNTRY_ALIASES);
    let (items, show) = filter_items(all_countries, &q, filtering, title_hit);
    if show {
        sep(ui, &mut drawn);
        let total = items.len();
        let open_id = Id::new("facet-paesi-tutti");
        let show_all = ui.data(|d| d.get_temp::<bool>(open_id)).unwrap_or(false);
        // Al chiuso restano i primi otto, piu' quelli gia' scelti: un filtro
        // attivo non sparisce mai sotto "Tutti i paesi".
        let list = if filtering || show_all {
            items
        } else {
            let mut v: Vec<(String, bool, Availability)> = items.iter().take(8).cloned().collect();
            for it in &items {
                if it.1 && !v.iter().any(|x| x.0 == it.0) {
                    v.push(it.clone());
                }
            }
            v
        };
        let acts = section(
            ui,
            "paesi",
            "Paese e lingua",
            f.countries.len(),
            false,
            filtering,
            |ui, out| {
                chip_row(ui, list, out, |label| {
                    let code = tax
                        .countries
                        .iter()
                        .find(|c| c.name == label)
                        .map(|c| c.code.clone())
                        .unwrap_or_default();
                    FacetAction::Country(code)
                });
                if !filtering && total > 8 {
                    ui.add_space(1.0);
                    let label = if show_all {
                        "Meno paesi".to_string()
                    } else {
                        format!("Tutti i paesi ({total})")
                    };
                    if ghost(ui, &label, true).clicked() {
                        ui.data_mut(|d| d.insert_temp(open_id, !show_all));
                    }
                }
            },
        );
        out.extend(acts);
    }

    // ---- mood e temi ----
    let all_moods: Vec<(String, bool, Availability)> = tax
        .moods
        .iter()
        .map(|m| {
            let selected = f.moods.contains(&m.id);
            let av = prune
                .moods
                .get(&m.id)
                .cloned()
                .unwrap_or_else(Availability::ok);
            (m.label.clone(), selected, av)
        })
        .collect();
    let title_hit = wanted(&q, "Mood e temi", MOOD_ALIASES);
    let (items, show) = filter_items(all_moods, &q, filtering, title_hit);
    if show {
        sep(ui, &mut drawn);
        let acts = section(
            ui,
            "mood",
            "Mood e temi",
            f.moods.len() + f.keywords.len(),
            false,
            filtering,
            |ui, out| {
                chip_row(ui, items, out, |label| {
                    let id = tax
                        .moods
                        .iter()
                        .find(|m| m.label == label)
                        .map(|m| m.id.clone())
                        .unwrap_or_default();
                    FacetAction::Mood(id)
                });
                if !filtering {
                    ui.add_space(4.0);
                    let id = Id::new("cinematch_tema_nuovo");
                    let mut buf = ui.data_mut(|d| d.get_temp::<String>(id).unwrap_or_default());
                    let w = ui.available_width();
                    let resp = text_input(ui, &mut buf, "Aggiungi un tema (invio)…", w);
                    if resp.changed() {
                        ui.data_mut(|d| d.insert_temp(id, buf.clone()));
                    }
                    if resp.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                        let name = buf.trim().to_lowercase();
                        if !name.is_empty() {
                            out.push(FacetAction::Keyword(name));
                        }
                        ui.data_mut(|d| d.insert_temp(id, String::new()));
                    }
                }
            },
        );
        out.extend(acts);
    }

    // ---- metriche ----
    let title_hit = wanted(&q, "Metriche", METRIC_ALIASES);
    if !filtering || title_hit {
        sep(ui, &mut drawn);
        let metriche = f.min_rating.is_some() as usize
            + f.max_rating.is_some() as usize
            + f.min_votes.is_some() as usize
            + f.runtime.is_some() as usize;
        let acts = section(
            ui,
            "metriche",
            "Metriche",
            metriche,
            false,
            filtering,
            |ui, out| metrics(ui, f, out),
        );
        out.extend(acts);
    }

    // ---- nessun risultato ----
    if filtering && drawn == 0 {
        ui.add_space(22.0);
        ui.vertical_centered(|ui| {
            ui.label(
                RichText::new("Nessun filtro trovato")
                    .size(12.5)
                    .strong()
                    .color(theme::DIM),
            );
            ui.add_space(2.0);
            ui.label(
                RichText::new(format!("Nessuna corrispondenza per «{}»", query.trim()))
                    .size(10.5)
                    .color(theme::FAINT),
            );
            ui.add_space(8.0);
            if ghost(ui, "Azzera ricerca", true).clicked() {
                query.clear();
            }
        });
    }

    // ---- guida ----
    if !filtering {
        ui.add_space(6.0);
        let label = format!("Guida alle correnti ({})", tax.movements.len());
        if ghost(ui, &label, true).clicked() {
            open_guide(ui, guide_open);
        }
    }

    // ---- spiegazioni ----
    if !messages.is_empty() || !info.is_empty() || !ai_notes.is_empty() {
        ui.add_space(10.0);
        Frame::none()
            .fill(theme::CARD)
            .stroke(Stroke::new(1.0_f32, theme::BORDER))
            .rounding(Rounding::same(10.0))
            .inner_margin(Margin::same(9.0))
            .show(ui, |ui| {
                ui.label(
                    RichText::new("PERCHÉ")
                        .size(10.0)
                        .strong()
                        .color(theme::ACCENT),
                );
                ui.add_space(2.0);
                for m in messages {
                    ui.label(RichText::new(format!("• {m}")).size(10.5).color(theme::WARN));
                }
                for m in info {
                    ui.label(RichText::new(format!("· {m}")).size(10.5).color(theme::TEXT));
                }
                for n in ai_notes {
                    ui.label(RichText::new(format!("◦ {n}")).size(10.5).color(theme::DIM));
                }
            });
    }

    out
}

// ---------------------------------------------------------------- guida -----

/// Apre la guida da zero: niente ricerca o scroll rimasti dalla volta prima.
fn open_guide(ui: &mut Ui, guide_open: &mut bool) {
    *guide_open = true;
    ui.data_mut(|d| {
        d.insert_temp(Id::new("cinematch_guide_query"), String::new());
        d.insert_temp(Id::new("cinematch_guide_fresh"), true);
    });
}

/// Finestra «Correnti del cinema»: la vecchia guida in fondo al pannello,
/// con piu' spazio, ricerca e un pulsante per applicare la combinazione.
pub fn guide_window(ctx: &egui::Context, tax: &Taxonomy, open: &mut bool) -> Vec<FacetAction> {
    let mut out = Vec::new();
    if !*open {
        return out;
    }
    let mut apply: Option<FacetAction> = None;
    egui::Window::new(
        RichText::new("Correnti del cinema")
            .font(FontId::new(20.0, egui::FontFamily::Name(fonts::LUMEN_DISPLAY.into())))
            .color(lumen::IVORY),
    )
        .open(open)
        .collapsible(false)
        .resizable(true)
        .default_width(560.0)
        .show(ctx, |ui| {
            ui.label(
                lumen::body(
                    11.0,
                    format!(
                        "{} correnti: ognuna è una combinazione di filtri TMDB. Applicala e poi scegli i generi che vuoi stringere.",
                        tax.movements.len()
                    ),
                )
                .color(lumen::MUTED),
            );
            ui.add_space(6.0);
            let qid = Id::new("cinematch_guide_query");
            let mut q = ui.data_mut(|d| d.get_temp::<String>(qid).unwrap_or_default());
            search_field(ui, &mut q, "Cerca una corrente…");
            ui.data_mut(|d| d.insert_temp(qid, q.clone()));
            ui.add_space(6.0);
            hairline(ui);
            ui.add_space(4.0);

            let needle = norm(&q);
            let mut found = 0usize;
            let fresh = ui.data(|d| {
                d.get_temp::<bool>(Id::new("cinematch_guide_fresh"))
                    .unwrap_or(false)
            });
            let mut scroll = egui::ScrollArea::vertical()
                .id_salt("guide-scroll")
                .auto_shrink([false, false])
                .max_height((ctx.screen_rect().height() * 0.62).max(300.0));
            if fresh {
                scroll = scroll.vertical_scroll_offset(0.0);
            }
            scroll.show(ui, |ui| {
                    for m in &tax.movements {
                        let (period, details) = movement_details(tax, m);
                        if !needle.is_empty() {
                            let hay = norm(&format!(
                                "{} {} {} {}",
                                m.label, period, details, m.keyword
                            ));
                            if !hay.contains(&needle) {
                                continue;
                            }
                        }
                        found += 1;
                        ui.horizontal(|ui| {
                            ui.add(
                                egui::Label::new(lumen::display(18.0, &m.label))
                                    .truncate(),
                            );
                            ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                                if lumen::button_sized(ui, "Applica", false, 82.0)
                                    .on_hover_text(
                                        "Accende periodo, paesi e corrente; i generi restano da scegliere",
                                    )
                                    .clicked()
                                {
                                    let (decades, countries, _) = movement_recipe(tax, m);
                                    apply = Some(FacetAction::Combo {
                                        decades,
                                        countries,
                                        movements: vec![m.id.clone()],
                                    });
                                }
                            });
                        });
                        ui.add_space(2.0);
                        ui.label(lumen::mono(9.0, &period).color(lumen::FAINT));
                        ui.label(lumen::body(10.5, &details).color(lumen::MUTED));
                        if !m.note.is_empty() {
                            ui.label(lumen::body(10.0, &m.note).color(lumen::FAINT));
                        }
                        ui.add_space(6.0);
                        hairline(ui);
                        ui.add_space(4.0);
                    }
                    if found == 0 {
                        ui.add_space(14.0);
                        ui.label(
                            RichText::new("Nessuna corrente trovata per questa ricerca.")
                                .size(11.0)
                                .color(theme::DIM),
                        );
                    }
                });
            ui.data_mut(|d| d.insert_temp(Id::new("cinematch_guide_fresh"), false));
        });
    if let Some(action) = apply {
        out.push(action);
        *open = false;
    }
    out
}

// ------------------------------------------------------------ componenti ---

fn badge_size(ui: &Ui, n: usize) -> (f32, f32) {
    let galley = ui
        .painter()
        .layout_no_wrap(n.to_string(), FontId::proportional(10.0), Color32::PLACEHOLDER);
    ((galley.size().x + 10.0).max(18.0), galley.size().y)
}

fn paint_badge(painter: &egui::Painter, rect: egui::Rect, n: usize) {
    let galley = painter.layout_no_wrap(
        n.to_string(),
        FontId::proportional(10.0),
        Color32::PLACEHOLDER,
    );
    painter.rect_filled(rect, Rounding::same(rect.height() * 0.5), theme::ACCENT_DIM);
    painter.galley(
        egui::pos2(
            rect.center().x - galley.size().x * 0.5,
            rect.center().y - galley.size().y * 0.5,
        ),
        galley,
        theme::ACCENT_3,
    );
}

/// Pulsante testuale discreto: «Azzera», «pulisci», «Tutti i paesi».
fn ghost(ui: &mut Ui, text: &str, enabled: bool) -> Response {
    let color = if enabled { theme::ACCENT } else { theme::FAINT };
    let galley = ui.painter().layout_no_wrap(
        text.to_uppercase(),
        FontId::new(9.0, egui::FontFamily::Name(fonts::LUMEN_MONO.into())),
        Color32::PLACEHOLDER,
    );
    let size = galley.size() + egui::vec2(12.0, 6.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = if enabled {
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered(), 0.12)
    } else {
        0.0
    };
    if hover > 0.02 {
        ui.painter().rect_filled(
            rect,
            Rounding::same(7.0),
            theme::alpha(theme::ACCENT, (24.0 * hover) as u8),
        );
    }
    ui.painter().galley(
        rect.min + egui::vec2(6.0, 3.0),
        galley,
        theme::mix(color, Color32::WHITE, hover * 0.25),
    );
    if enabled {
        theme::focus_ring(ui, &resp, 7.0);
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    } else {
        resp
    }
}

fn help_button(ui: &mut Ui) -> Response {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    p.circle_filled(
        rect.center(),
        10.5,
        theme::mix(theme::alpha(theme::CARD, 210), theme::ACCENT_DIM, hover * 0.5),
    );
    p.circle_stroke(
        rect.center(),
        10.5,
        Stroke::new(1.0_f32, theme::mix(theme::BORDER, theme::ACCENT, hover)),
    );
    p.text(
        rect.center(),
        Align2::CENTER_CENTER,
        "?",
        FontId::proportional(11.5),
        theme::mix(theme::DIM, theme::ACCENT_3, hover),
    );
    resp.on_hover_text("Guida alle correnti")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

/// Campo di testo con cornice arrotondata e anello di fuoco blu.
fn text_input(ui: &mut Ui, text: &mut String, hint: &str, width: f32) -> Response {
    let frame = Frame::none()
        .fill(lumen::DEEP)
        .stroke(Stroke::new(1.0_f32, theme::BORDER))
        .rounding(Rounding::same(lumen::RADIUS_SM))
        .inner_margin(Margin::symmetric(8.0, 4.0));
    let inner = frame.show(ui, |ui| {
        ui.add_sized(
            [width - 16.0, 20.0],
            egui::TextEdit::singleline(text)
                .frame(false)
                .font(FontId::proportional(11.5))
                .hint_text(RichText::new(hint).size(11.0).color(theme::FAINT)),
        )
    });
    if inner.inner.has_focus() {
        ui.painter().rect_stroke(
            inner.response.rect,
            Rounding::same(lumen::RADIUS_SM),
            Stroke::new(1.0_f32, theme::ACCENT),
        );
    }
    inner.inner
}

/// Ricerca con la «×» per svuotarla. Lo spazio della × e' sempre riservato,
/// cosi' il campo non cambia larghezza alla prima lettera.
fn search_field(ui: &mut Ui, text: &mut String, hint: &str) -> Response {
    let avail = ui.available_width();
    let field_w = (avail - 28.0).max(80.0);
    ui.horizontal(|ui| {
        let resp = text_input(ui, text, hint, field_w);
        let (rect, click) =
            ui.allocate_exact_size(egui::vec2(20.0, 26.0), Sense::click());
        if !text.is_empty() {
            let hover = ui
                .ctx()
                .animate_bool_with_time(click.id, click.hovered(), 0.12);
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "×",
                FontId::proportional(13.0),
                theme::mix(theme::DIM, theme::ACCENT, hover),
            );
            if click.clicked() {
                text.clear();
            }
            click.on_hover_cursor(egui::CursorIcon::PointingHand);
        }
        resp
    })
    .inner
}

/// Pill removibile di un filtro attivo.
fn pill(ui: &mut Ui, label: &str) -> Response {
    let galley = ui.painter().layout_no_wrap(
        format!("× {label}"),
        FontId::new(10.5, egui::FontFamily::Name(fonts::LUMEN_MONO.into())),
        Color32::PLACEHOLDER,
    );
    let size = galley.size() + egui::vec2(14.0, 8.0);
    let (rect, resp) = ui.allocate_exact_size(size, Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let p = ui.painter();
    p.rect_filled(
        rect,
        Rounding::same(lumen::RADIUS_SM),
        theme::mix(lumen::DEEP, lumen::RAISED, hover * 0.6),
    );
    p.rect_stroke(
        rect,
        Rounding::same(lumen::RADIUS_SM),
        Stroke::new(
            1.0_f32,
            theme::mix(theme::BORDER, theme::ACCENT, hover),
        ),
    );
    p.galley(
        rect.min + egui::vec2(7.0, 4.0),
        galley,
        theme::mix(lumen::MUTED, theme::TEXT, hover),
    );
    resp.on_hover_text(format!("Rimuovi «{label}»"))
        .on_hover_cursor(egui::CursorIcon::PointingHand)
}

fn active_pills(ui: &mut Ui, tax: &Taxonomy, f: &Facets) -> Vec<FacetAction> {
    let mut items: Vec<(String, FacetAction)> = Vec::new();
    for id in &f.movements {
        if let Some(m) = tax.movement(id) {
            items.push((m.label.clone(), FacetAction::Movement(m.id.clone())));
        }
    }
    for id in &f.decades {
        if let Some(d) = tax.decade(id) {
            items.push((d.label.clone(), FacetAction::Decade(d.id.clone())));
        }
    }
    for id in &f.genres {
        if let Some(name) = tax.genre_name(*id) {
            items.push((name, FacetAction::Genre(*id)));
        }
    }
    for id in &f.without_genres {
        if let Some(name) = tax.genre_name(*id) {
            items.push((format!("senza {name}"), FacetAction::Genre(*id)));
        }
    }
    for code in &f.countries {
        items.push((
            tax.country_label(code),
            FacetAction::Country(code.clone()),
        ));
    }
    for id in &f.moods {
        if let Some(m) = tax.mood(id) {
            items.push((m.label.clone(), FacetAction::Mood(m.id.clone())));
        }
    }
    for k in &f.keywords {
        items.push((format!("tema: {k}"), FacetAction::Keyword(k.clone())));
    }
    if let Some(v) = f.min_rating {
        items.push((format!("voto ≥ {v:.1}"), FacetAction::Rating(None)));
    }
    if let Some(v) = f.max_rating {
        items.push((format!("voto ≤ {v:.1}"), FacetAction::MaxRating(None)));
    }
    if let Some(v) = f.min_votes {
        items.push((
            format!("voti ≥ {}", theme::thousands(v)),
            FacetAction::Votes(None),
        ));
    }
    if let Some(r) = f.runtime {
        items.push((
            format!("durata {}", runtime_label(Some(r))),
            FacetAction::Runtime(None),
        ));
    }
    if items.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(5.0, 5.0);
        for (label, action) in items {
            if pill(ui, &label).clicked() {
                out.push(action);
            }
        }
    });
    out
}

/// Testata di sezione: titolo, badge, pulisci, chevron. Il corpo si disegna
/// solo se la sezione e' aperta; lo stato vive nella memoria di egui.
fn section(
    ui: &mut Ui,
    key: &'static str,
    title: &str,
    active: usize,
    default_open: bool,
    forced_open: bool,
    body: impl FnOnce(&mut Ui, &mut Vec<FacetAction>),
) -> Vec<FacetAction> {
    // Id globale: lo stato della sezione non deve dipendere dall'id interno
    // dello ScrollArea, che cambia quando cambia il numero di widget sopra.
    let id = Id::new(("facet-section", key));
    let stored = ui.data(|d| d.get_temp::<bool>(id));
    let open = forced_open || stored.unwrap_or(default_open || active > 0);

    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), Sense::click());
    let resp = if forced_open {
        resp
    } else {
        resp.on_hover_cursor(egui::CursorIcon::PointingHand)
    };
    let hover = if forced_open {
        0.0
    } else {
        ui.ctx()
            .animate_bool_with_time(resp.id, resp.hovered(), 0.12)
    };
    if hover > 0.02 {
        ui.painter().rect_filled(
            rect,
            Rounding::ZERO,
            theme::alpha(lumen::RAISED, (110.0 * hover) as u8),
        );
    }

    let p = ui.painter();
    let title_color = if active > 0 {
        theme::ACCENT
    } else if forced_open || open || hover > 0.3 {
        theme::TEXT
    } else {
        theme::DIM
    };
    let galley = p.layout_no_wrap(
        title.to_uppercase(),
        FontId::new(10.0, egui::FontFamily::Name(fonts::LUMEN_MONO.into())),
        Color32::PLACEHOLDER,
    );
    p.galley(
        egui::pos2(rect.left() + 2.0, rect.center().y - galley.size().y * 0.5),
        galley.clone(),
        title_color,
    );
    if active > 0 {
        let (w, _) = badge_size(ui, active);
        let brect = egui::Rect::from_center_size(
            egui::pos2(rect.left() + 2.0 + galley.size().x + 6.0 + w * 0.5, rect.center().y),
            egui::vec2(w, 15.0),
        );
        paint_badge(p, brect, active);
    }

    let chev = ui
        .ctx()
        .animate_bool_with_time(id.with("chev"), open, 0.14);
    chevron(
        p,
        egui::pos2(rect.right() - 11.0, rect.center().y),
        chev,
        theme::mix(theme::FAINT, theme::ACCENT, hover.max(if forced_open { 1.0 } else { 0.0 })),
    );

    let mut clear_clicked = false;
    if active > 0 {
        let cg = p.layout_no_wrap(
            "PULISCI".to_owned(),
            FontId::new(8.5, egui::FontFamily::Name(fonts::LUMEN_MONO.into())),
            Color32::PLACEHOLDER,
        );
        let cpos = egui::pos2(
            rect.right() - 22.0 - cg.size().x,
            rect.center().y - cg.size().y * 0.5,
        );
        let crect = egui::Rect::from_min_size(cpos, cg.size());
        let cresp = ui.interact(crect.expand(3.0), id.with("clear"), Sense::click());
        let ch = ui
            .ctx()
            .animate_bool_with_time(id.with("clear-hover"), cresp.hovered(), 0.1);
        p.galley(cpos, cg, theme::mix(theme::DIM, theme::ACCENT, ch));
        clear_clicked = cresp.clicked();
        if cresp.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }

    let mut out = Vec::new();
    if clear_clicked {
        out.push(FacetAction::Clear(section_kind(key)));
    }
    if !forced_open && resp.clicked() && !clear_clicked {
        ui.data_mut(|d| d.insert_temp(id, !open));
    }
    if open {
        ui.add_space(2.0);
        body(ui, &mut out);
        ui.add_space(4.0);
    }
    out
}

fn section_kind(key: &str) -> Section {
    match key {
        "periodo" => Section::Period,
        "genere" => Section::Genres,
        "paesi" => Section::Countries,
        "mood" => Section::Moods,
        _ => Section::Metrics,
    }
}

fn chevron(painter: &egui::Painter, center: egui::Pos2, t: f32, color: Color32) {
    let lerp = |a: egui::Pos2, b: egui::Pos2| a + (b - a) * t.clamp(0.0, 1.0);
    let p0 = lerp(
        center + egui::vec2(-2.0, -4.0),
        center + egui::vec2(-4.0, -2.0),
    );
    let p1 = lerp(
        center + egui::vec2(2.0, 0.0),
        center + egui::vec2(0.0, 2.0),
    );
    let p2 = lerp(
        center + egui::vec2(-2.0, 4.0),
        center + egui::vec2(4.0, -2.0),
    );
    painter.add(egui::Shape::line(
        vec![p0, p1, p2],
        Stroke::new(1.6_f32, color),
    ));
}

/// Una riga di chip, gia' filtrata dalla ricerca.
fn chip_row(
    ui: &mut Ui,
    items: Vec<(String, bool, Availability)>,
    out: &mut Vec<FacetAction>,
    make: impl Fn(&str) -> FacetAction,
) {
    ui.horizontal_wrapped(|ui| {
        for (label, selected, av) in items {
            let resp = theme::chip(ui, &label, selected, selected || av.allowed, av.reason.as_deref());
            if resp.clicked() {
                out.push(make(&label));
            }
        }
    });
}

// ------------------------------------------------------------- metriche ----

fn metrics(ui: &mut Ui, f: &Facets, out: &mut Vec<FacetAction>) {
    let gap = 6.0;
    let w = ((ui.available_width() - gap) * 0.5).max(110.0);

    let min_label = match f.min_rating {
        Some(v) => format!("Voto ≥ {v:.1}"),
        None => "Voto minimo".to_string(),
    };
    let max_label = match f.max_rating {
        Some(v) => format!("Voto ≤ {v:.1}"),
        None => "Voto massimo".to_string(),
    };
    let votes_label = match f.min_votes {
        Some(v) => format!("Voti ≥ {}", theme::thousands(v)),
        None => "Voti minimi".to_string(),
    };
    let durata_label = runtime_label(f.runtime);

    egui::Grid::new("metriche-grid")
        .num_columns(2)
        .spacing([gap, 6.0])
        .show(ui, |ui| {
            let (r, id) = metric_chip(ui, "voto-min", w, &min_label, f.min_rating.is_some());
            if let Some(a) = rating_popup(
                ui,
                &r,
                id,
                w,
                f.min_rating,
                "Voto minimo",
                MIN_RATINGS,
                "e più",
                FacetAction::Rating,
            ) {
                out.push(a);
            }
            let (r, id) = metric_chip(ui, "voto-max", w, &max_label, f.max_rating.is_some());
            if let Some(a) = rating_popup(
                ui,
                &r,
                id,
                w,
                f.max_rating,
                "Voto massimo",
                MAX_RATINGS,
                "e meno",
                FacetAction::MaxRating,
            ) {
                out.push(a);
            }
            ui.end_row();

            let (r, id) = metric_chip(ui, "voti", w, &votes_label, f.min_votes.is_some());
            if let Some(a) = votes_popup(ui, &r, id, w, f.min_votes) {
                out.push(a);
            }
            let (r, id) = metric_chip(ui, "durata", w, &durata_label, f.runtime.is_some());
            if let Some(a) = runtime_popup(ui, &r, id, w, f.runtime) {
                out.push(a);
            }
            ui.end_row();
        });

    if f.sort == Sort::Rating && f.min_votes.is_none() && f.min_rating.is_none() {
        ui.add_space(4.0);
        ui.label(
            RichText::new(
                "Per la classifica per voto contano solo i titoli con almeno 50 voti: alza «Voti minimi» per una lista solida.",
            )
            .size(10.0)
            .color(theme::FAINT),
        );
    }
}

const MIN_RATINGS: &[f32] = &[6.0, 6.5, 7.0, 7.5, 8.0, 8.5, 9.0];
const MAX_RATINGS: &[f32] = &[5.0, 5.5, 6.0, 6.5, 7.0, 7.5, 8.0];
const VOTES: &[u32] = &[500, 1_000, 2_000, 5_000, 10_000, 50_000, 100_000];

fn metric_chip(
    ui: &mut Ui,
    key: &'static str,
    width: f32,
    label: &str,
    active: bool,
) -> (Response, Id) {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(width, 28.0), Sense::click());
    let resp = resp.on_hover_cursor(egui::CursorIcon::PointingHand);
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.12);
    let (fill, stroke, text) = if active {
        (
            lumen::DEEP,
            theme::mix(theme::ACCENT, theme::ACCENT_2, hover),
            theme::TEXT,
        )
    } else {
        (
            theme::mix(lumen::DEEP, lumen::RAISED, hover),
            theme::mix(theme::BORDER, theme::ACCENT, hover),
            theme::mix(lumen::MUTED, theme::TEXT, hover),
        )
    };
    let p = ui.painter();
    p.rect_filled(rect, Rounding::same(lumen::RADIUS_SM), fill);
    p.rect_stroke(
        rect,
        Rounding::same(lumen::RADIUS_SM),
        Stroke::new(1.0_f32, stroke),
    );
    let galley = p.layout_no_wrap(
        label.to_owned(),
        FontId::new(10.0, egui::FontFamily::Name(fonts::LUMEN_MONO.into())),
        Color32::PLACEHOLDER,
    );
    p.galley(
        egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5),
        galley,
        text,
    );
    p.text(
        egui::pos2(rect.right() - 8.0, rect.center().y),
        Align2::RIGHT_CENTER,
        "▼",
        FontId::proportional(6.5),
        theme::mix(theme::FAINT, text, 0.55),
    );

    let popup = Id::new(("metric-pop", key));
    if resp.clicked() {
        ui.memory_mut(|m| m.toggle_popup(popup));
    }
    (resp, popup)
}

fn menu_item(ui: &mut Ui, label: &str, selected: bool) -> bool {
    let (rect, resp) =
        ui.allocate_exact_size(egui::vec2(ui.available_width(), 25.0), Sense::click());
    let hover = ui
        .ctx()
        .animate_bool_with_time(resp.id, resp.hovered(), 0.1);
    if selected || hover > 0.02 {
        let fill = if selected {
            theme::alpha(theme::ACCENT, 45)
        } else {
            theme::alpha(theme::CARD_HOVER, (150.0 * hover) as u8)
        };
        ui.painter().rect_filled(rect, Rounding::same(7.0), fill);
    }
    let p = ui.painter();
    let galley = p.layout_no_wrap(
        label.to_owned(),
        FontId::proportional(11.5),
        Color32::PLACEHOLDER,
    );
    p.galley(
        egui::pos2(rect.left() + 8.0, rect.center().y - galley.size().y * 0.5),
        galley,
        if selected { theme::ACCENT_3 } else { theme::TEXT },
    );
    if selected {
        p.text(
            egui::pos2(rect.right() - 8.0, rect.center().y),
            Align2::RIGHT_CENTER,
            "✓",
            FontId::proportional(10.0),
            theme::ACCENT_3,
        );
    }
    resp.clicked()
}

#[allow(clippy::too_many_arguments)]
fn rating_popup(
    ui: &mut Ui,
    resp: &Response,
    id: Id,
    width: f32,
    current: Option<f32>,
    title: &str,
    presets: &[f32],
    verb: &str,
    make: impl Fn(Option<f32>) -> FacetAction,
) -> Option<FacetAction> {
    let mut chosen = None;
    let mut close = false;
    egui::popup::popup_below_widget(
        ui,
        id,
        resp,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(width.max(188.0));
            ui.label(RichText::new(title).size(10.0).color(theme::FAINT));
            ui.add_space(2.0);
            if menu_item(ui, "Nessuno", current.is_none()) {
                chosen = Some(make(None));
                close = true;
            }
            let custom = current.filter(|v| !presets.contains(v));
            if let Some(v) = custom {
                if menu_item(ui, &format!("{v:.1} (attuale)"), true) {
                    chosen = Some(make(Some(v)));
                    close = true;
                }
            }
            for v in presets {
                if menu_item(ui, &format!("{v:.1} {verb}"), current == Some(*v)) {
                    chosen = Some(make(Some(*v)));
                    close = true;
                }
            }
            ui.add_space(4.0);
            ui.separator();
            ui.add_space(2.0);
            ui.label(RichText::new("Valore fine").size(10.0).color(theme::FAINT));
            ui.spacing_mut().slider_width = width.max(150.0) - 28.0;
            let mut value = current.unwrap_or(7.0);
            let slider = ui.add(
                egui::Slider::new(&mut value, 0.0..=10.0)
                    .step_by(0.1)
                    .fixed_decimals(1)
                    .trailing_fill(true),
            );
            if slider.changed() {
                chosen = Some(make(Some(value)));
            }
        },
    );
    if close {
        ui.memory_mut(|m| m.close_popup());
    }
    chosen
}

fn votes_popup(
    ui: &mut Ui,
    resp: &Response,
    id: Id,
    width: f32,
    current: Option<u32>,
) -> Option<FacetAction> {
    let mut chosen = None;
    let mut close = false;
    egui::popup::popup_below_widget(
        ui,
        id,
        resp,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(width.max(188.0));
            ui.label(RichText::new("Voti minimi").size(10.0).color(theme::FAINT));
            ui.add_space(2.0);
            if menu_item(ui, "Nessuno", current.is_none()) {
                chosen = Some(FacetAction::Votes(None));
                close = true;
            }
            if let Some(c) = current {
                if !VOTES.contains(&c) && menu_item(ui, &format!("{} (attuale)", theme::thousands(c)), true)
                {
                    chosen = Some(FacetAction::Votes(Some(c)));
                    close = true;
                }
            }
            for v in VOTES {
                if menu_item(ui, &format!("{} e più", theme::thousands(*v)), current == Some(*v)) {
                    chosen = Some(FacetAction::Votes(Some(*v)));
                    close = true;
                }
            }
        },
    );
    if close {
        ui.memory_mut(|m| m.close_popup());
    }
    chosen
}

const RUNTIMES: &[(&str, Option<(u32, u32)>)] = &[
    ("Qualsiasi", None),
    ("Cortometraggio (< 40')", Some((0, 40))),
    ("Breve (< 90')", Some((0, 90))),
    ("Medio (90–120')", Some((90, 120))),
    ("Lungo (> 120')", Some((120, 600))),
];

fn runtime_popup(
    ui: &mut Ui,
    resp: &Response,
    id: Id,
    width: f32,
    current: Option<(u32, u32)>,
) -> Option<FacetAction> {
    let mut chosen = None;
    let mut close = false;
    egui::popup::popup_below_widget(
        ui,
        id,
        resp,
        egui::popup::PopupCloseBehavior::CloseOnClickOutside,
        |ui| {
            ui.set_min_width(width.max(200.0));
            ui.label(RichText::new("Durata").size(10.0).color(theme::FAINT));
            ui.add_space(2.0);
            for (label, value) in RUNTIMES {
                if menu_item(ui, label, current == *value) {
                    chosen = Some(FacetAction::Runtime(*value));
                    close = true;
                }
            }
        },
    );
    if close {
        ui.memory_mut(|m| m.close_popup());
    }
    chosen
}

fn runtime_label(r: Option<(u32, u32)>) -> String {
    match r {
        None => "Durata".to_string(),
        Some((0, 40)) => "Corto (< 40')".to_string(),
        Some((0, 90)) => "Breve (< 90')".to_string(),
        Some((90, 120)) => "Medio (90–120')".to_string(),
        Some((120, 600)) => "Lungo (> 120')".to_string(),
        Some((a, b)) => format!("{a}–{b} min"),
    }
}

// ------------------------------------------------------------- utilita' ----

/// Generi mostrati nella sezione chiusa: i dodici piu' usati. Gli altri
/// (documentari, guerra, western…) restano a un clic da «Tutti i generi».
const PRIMARY_GENRES: &[u32] = &[28, 12, 16, 35, 80, 18, 14, 27, 9648, 10749, 878, 53];

const PERIOD_ALIASES: &[&str] = &["anni", "anno", "decennio", "decenni", "epoca"];
const GENRE_ALIASES: &[&str] = &["generi", "genere"];
const COUNTRY_ALIASES: &[&str] = &["paesi", "paese", "lingua", "lingue", "nazioni"];
const MOOD_ALIASES: &[&str] = &["mood", "tema", "temi", "atmosfera", "parola chiave"];
const METRIC_ALIASES: &[&str] = &["voto", "voti", "durata", "metriche"];

/// Minuscolo e senza accenti: cosi' «perché» e «perche» si cercano uguali.
fn norm(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ç' => 'c',
            _ => c,
        })
        .collect()
}

fn wanted(q: &str, title: &str, aliases: &[&str]) -> bool {
    let q = norm(q);
    if q.is_empty() {
        return true;
    }
    norm(title).contains(&q) || aliases.iter().any(|a| norm(a).contains(&q))
}

fn filter_items(
    items: Vec<(String, bool, Availability)>,
    q: &str,
    filtering: bool,
    title_hit: bool,
) -> (Vec<(String, bool, Availability)>, bool) {
    if !filtering {
        let show = !items.is_empty();
        return (items, show);
    }
    if title_hit {
        return (items, true);
    }
    let filtered: Vec<_> = items
        .into_iter()
        .filter(|(l, _, _)| norm(l).contains(q))
        .collect();
    let show = !filtered.is_empty();
    (filtered, show)
}

/// Linea sottile disegnata a mano: piu' discreta del separatore di egui.
fn hairline(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(
        egui::vec2(ui.available_width(), 1.0),
        Sense::hover(),
    );
    ui.painter().rect_filled(
        rect,
        Rounding::same(0.0),
        theme::alpha(theme::BORDER, 170),
    );
}

fn sep(ui: &mut Ui, drawn: &mut usize) {
    if *drawn > 0 {
        ui.add_space(3.0);
        hairline(ui);
        ui.add_space(3.0);
    }
    *drawn += 1;
}

fn movement_window(tax: &Taxonomy, f: &Facets) -> (Option<u16>, Option<u16>) {
    let mut lo: Option<u16> = None;
    let mut hi: Option<u16> = None;
    for id in &f.movements {
        if let Some(m) = tax.movement(id) {
            lo = Some(lo.map_or(m.period[0], |v: u16| v.max(m.period[0])));
            hi = Some(hi.map_or(m.period[1], |v: u16| v.min(m.period[1])));
        }
    }
    (lo, hi)
}

fn period_label(from: u16, to: u16) -> String {
    format!("{from}–{to}")
}

/// Periodo e ricetta di una corrente, condivisi dalla guida.
fn movement_details(tax: &Taxonomy, m: &Movement) -> (String, String) {
    let (decades, countries, genres) = movement_recipe(tax, m);
    let period = period_label(m.period[0], m.period[1]);
    let decade_labels = decades
        .iter()
        .filter_map(|d| tax.decade(d))
        .map(|d| d.label.clone())
        .collect::<Vec<_>>()
        .join(" + ");
    let country_labels = countries
        .iter()
        .map(|c| tax.country_label(c))
        .collect::<Vec<_>>()
        .join(", ");
    let genre_labels = genres
        .iter()
        .filter_map(|g| tax.genre_name(*g))
        .collect::<Vec<_>>()
        .join(", ");
    let mut parts: Vec<String> = Vec::new();
    if !decade_labels.is_empty() {
        parts.push(decade_labels);
    }
    if !country_labels.is_empty() {
        parts.push(country_labels);
    }
    if !m.keyword.is_empty() {
        parts.push(format!("parola chiave TMDB: {}", m.keyword));
    }
    if !genre_labels.is_empty() {
        parts.push(format!("generi tipici: {genre_labels}"));
    }
    let details = if parts.is_empty() {
        "Nessuna combinazione stretta: si riconosce dalle descrizioni.".to_string()
    } else {
        parts.join(" · ")
    };
    (period, details)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn norm_ignora_maiuscole_e_accenti() {
        assert_eq!(norm("Paese e Lingua"), "paese e lingua");
        assert_eq!(norm("Perché"), "perche");
    }

    #[test]
    fn gli_alias_trovano_le_sezioni() {
        assert!(wanted("voto", "Metriche", METRIC_ALIASES));
        assert!(wanted("paesi", "Paese e lingua", COUNTRY_ALIASES));
        assert!(wanted("horror", "Genere", GENRE_ALIASES) == false);
        assert!(wanted("generi", "Genere", GENRE_ALIASES));
    }
}
