use crate::ui::{
    lumen,
    window::{window_caption, window_resize_edges, CaptionIcon},
};
use cinema_currents::{Depth, Kind, Mode, Movement, PromptRequest, Relation};

#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Era {
    #[default]
    All,
    Origins,
    Interwar,
    Postwar,
    Recent,
    Contemporary,
}
impl Era {
    const ALL: [Self; 6] = [
        Self::All,
        Self::Origins,
        Self::Interwar,
        Self::Postwar,
        Self::Recent,
        Self::Contemporary,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::All => "Tutte le epoche",
            Self::Origins => "Origini · fino al 1919",
            Self::Interwar => "1920–1945",
            Self::Postwar => "1946–1979",
            Self::Recent => "1980–1999",
            Self::Contemporary => "Dal 2000",
        }
    }
    fn includes(self, m: &Movement) -> bool {
        let (a, b) = match self {
            Self::All => return true,
            Self::Origins => (0, 1919),
            Self::Interwar => (1920, 1945),
            Self::Postwar => (1946, 1979),
            Self::Recent => (1980, 1999),
            Self::Contemporary => (2000, u16::MAX),
        };
        m.start <= b && m.end.unwrap_or(u16::MAX) >= a
    }
}
#[derive(Clone, Copy, PartialEq, Eq, Default)]
enum Provider {
    #[default]
    ChatGpt,
    DeepSeek,
    Claude,
    Gemini,
    Custom,
}
impl Provider {
    const ALL: [Self; 5] = [
        Self::ChatGpt,
        Self::DeepSeek,
        Self::Claude,
        Self::Gemini,
        Self::Custom,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::ChatGpt => "ChatGPT",
            Self::DeepSeek => "DeepSeek",
            Self::Claude => "Claude",
            Self::Gemini => "Gemini",
            Self::Custom => "Altra chat",
        }
    }
    fn url(self, custom: &str) -> Result<String, String> {
        let raw = match self {
            Self::ChatGpt => "https://chatgpt.com/",
            Self::DeepSeek => "https://chat.deepseek.com/",
            Self::Claude => "https://claude.ai/",
            Self::Gemini => "https://gemini.google.com/",
            Self::Custom => custom.trim(),
        };
        let url = url::Url::parse(raw)
            .map_err(|_| "Inserisci un indirizzo HTTPS valido della chat.".to_string())?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err("Usa l'indirizzo HTTPS della chat, senza credenziali o parametri.".into());
        }
        Ok(url.to_string())
    }
}
pub struct App {
    entries: Vec<Movement>,
    query: String,
    kind: Option<Kind>,
    era: Era,
    region: String,
    alphabetic: bool,
    active: usize,
    request: PromptRequest,
    watched: String,
    composer: bool,
    prompt: String,
    generated_for: Option<PromptRequest>,
    warnings: Vec<String>,
    error: Option<String>,
    copied: bool,
    provider: Provider,
    custom_url: String,
    search_focus: bool,
    compact_list: bool,
}
impl App {
    pub fn new(_ctx: &egui::Context) -> Self {
        let entries = cinema_currents::catalog();
        let active = entries
            .iter()
            .position(|m| m.id == "neorealismo")
            .unwrap_or(0);
        let request = PromptRequest {
            schema_version: 1,
            movements: vec![entries[active].id.clone()],
            ..Default::default()
        };
        Self {
            entries,
            query: String::new(),
            kind: None,
            era: Era::All,
            region: String::new(),
            alphabetic: false,
            active,
            request,
            watched: String::new(),
            composer: false,
            prompt: String::new(),
            generated_for: None,
            warnings: Vec::new(),
            error: None,
            copied: false,
            provider: Provider::default(),
            custom_url: String::new(),
            search_focus: false,
            compact_list: false,
        }
    }
    fn filtered(&self) -> Vec<usize> {
        let mut ids = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, m)| {
                self.kind.is_none_or(|k| k == m.kind)
                    && self.era.includes(m)
                    && (self.region.is_empty() || m.regions.contains(&self.region))
                    && m.matches(&self.query)
            })
            .map(|(i, _)| i)
            .collect::<Vec<_>>();
        if self.alphabetic {
            ids.sort_by_key(|i| cinema_currents::normalize(&self.entries[*i].label));
        }
        ids
    }
    fn choose(&mut self, index: usize) {
        self.active = index;
        self.composer = false;
        self.compact_list = false;
        if self.request.movements.len() <= 1 {
            self.request.movements = vec![self.entries[index].id.clone()];
        }
    }
    fn sync_watched(&mut self) {
        self.request.watched = self
            .watched
            .lines()
            .flat_map(|s| s.split(';'))
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(str::to_string)
            .collect();
    }
    fn generate(&mut self) {
        self.sync_watched();
        match cinema_currents::generate(&self.request) {
            Ok(r) => {
                self.prompt = r.prompt;
                self.warnings = r.warnings;
                self.generated_for = Some(self.request.clone());
                self.error = None;
                self.copied = false;
            }
            Err(e) => self.error = Some(e),
        }
    }
    fn ready(&self) -> bool {
        self.generated_for.as_ref() == Some(&self.request) && !self.prompt.trim().is_empty()
    }
    fn header(&mut self, ui: &mut egui::Ui) {
        egui::Panel::top("titlebar")
            .exact_size(48.0)
            .frame(egui::Frame::NONE.fill(lumen::INK))
            .show(ui, |ui| {
                let rect = ui.max_rect();
                let drag = ui.interact(
                    rect,
                    egui::Id::unique("window-drag"),
                    egui::Sense::click_and_drag(),
                );
                if drag.drag_started() {
                    ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                if drag.double_clicked() {
                    let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                    ui.ctx()
                        .send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                }
                ui.horizontal(|ui| {
                    ui.add_space(18.0);
                    ui.label(lumen::display(22.0, "CINE"));
                    ui.label(lumen::display_color(22.0, "MATCH", lumen::EMBER));
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if window_caption(ui, CaptionIcon::Close, "Chiudi (Alt+F4)", 38.0).clicked()
                        {
                            ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
                        }
                        let max = ui.input(|i| i.viewport().maximized.unwrap_or(false));
                        if window_caption(
                            ui,
                            if max {
                                CaptionIcon::Restore
                            } else {
                                CaptionIcon::Maximize
                            },
                            "Ingrandisci / ripristina",
                            38.0,
                        )
                        .clicked()
                        {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Maximized(!max));
                        }
                        if window_caption(ui, CaptionIcon::Minimize, "Riduci a icona", 38.0)
                            .clicked()
                        {
                            ui.ctx()
                                .send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                        }
                    });
                });
            });
        egui::Panel::bottom("status")
            .exact_size(28.0)
            .frame(
                egui::Frame::NONE
                    .fill(lumen::DEEP)
                    .inner_margin(egui::Margin::symmetric(16, 5)),
            )
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.label(
                        lumen::body(
                            11.0,
                            "Guida e prompt locali · invio alla chat sotto il tuo controllo",
                        )
                        .color(lumen::MUTED),
                    );
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(
                            lumen::mono(10.0, format!("v{}", env!("CARGO_PKG_VERSION")))
                                .color(lumen::FAINT),
                        );
                    });
                });
            });
    }
    fn navigator(&mut self, ui: &mut egui::Ui) {
        ui.heading("Correnti del cinema");
        ui.label(
            lumen::body(
                12.0,
                format!("{} voci · dalle origini a oggi", self.entries.len()),
            )
            .color(lumen::MUTED),
        );
        ui.add_space(8.0);
        let search = ui.add(
            egui::TextEdit::singleline(&mut self.query)
                .id(egui::Id::unique("movement-search"))
                .hint_text("Cerca corrente, autore o film…")
                .desired_width(f32::INFINITY),
        );
        if self.search_focus {
            search.request_focus();
            self.search_focus = false;
        }
        ui.add_space(4.0);
        egui::ComboBox::from_id_salt("kind")
            .width(ui.available_width() - 12.0)
            .selected_text(self.kind.map(Kind::label).unwrap_or("Tutte le categorie"))
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.kind, None, "Tutte le categorie");
                for k in Kind::ALL {
                    ui.selectable_value(&mut self.kind, Some(k), k.label());
                }
            });
        egui::ComboBox::from_id_salt("era")
            .width(ui.available_width() - 12.0)
            .selected_text(self.era.label())
            .show_ui(ui, |ui| {
                for era in Era::ALL {
                    ui.selectable_value(&mut self.era, era, era.label());
                }
            });
        let regions = std::collections::BTreeSet::from_iter(
            self.entries.iter().flat_map(|m| m.regions.iter().cloned()),
        );
        egui::ComboBox::from_id_salt("region")
            .width(ui.available_width() - 12.0)
            .selected_text(if self.region.is_empty() {
                "Tutte le aree"
            } else {
                &self.region
            })
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut self.region, String::new(), "Tutte le aree");
                for region in regions {
                    ui.selectable_value(&mut self.region, region.clone(), region);
                }
            });
        ui.horizontal_wrapped(|ui| {
            ui.checkbox(&mut self.alphabetic, "Ordine alfabetico");
            if ui.small_button("Azzera ricerca").clicked() {
                self.query.clear();
                self.kind = None;
                self.era = Era::All;
                self.region.clear();
            }
        });
        let filtered = self.filtered();
        ui.label(
            lumen::body(
                11.0,
                format!(
                    "{} voci trovate · {} nel prompt",
                    filtered.len(),
                    self.request.movements.len()
                ),
            )
            .color(lumen::MUTED),
        );
        lumen::hairline(ui);
        egui::ScrollArea::vertical()
            .id_salt("movement-list")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                if filtered.is_empty() {
                    ui.add_space(10.0);
                    ui.label("Nessuna voce con questi criteri.");
                    ui.label("Prova un sinonimo oppure azzera i filtri.");
                }
                for index in filtered {
                    let m = &self.entries[index];
                    let id = m.id.clone();
                    let label = m.label.clone();
                    let period = m.period();
                    let kind = m.kind.label();
                    ui.push_id(&id, |ui| {
                        let mut selected = self.request.movements.contains(&id);
                        ui.horizontal_top(|ui| {
                            let check = ui
                                .checkbox(&mut selected, "")
                                .on_hover_text(format!("Includi {label} nel prompt"));
                            check.widget_info(|| {
                                egui::WidgetInfo::selected(
                                    egui::Role::CheckBox,
                                    ui.is_enabled(),
                                    selected,
                                    format!("Includi {label} nel prompt"),
                                )
                            });
                            if check.changed() {
                                if selected {
                                    self.request.movements.push(id.clone());
                                } else {
                                    self.request.movements.retain(|x| x != &id);
                                }
                            }
                            if ui
                                .add_sized(
                                    [ui.available_width(), 0.0],
                                    egui::Button::selectable(
                                        self.active == index,
                                        egui::RichText::new(&label).size(14.0),
                                    )
                                    .wrap(),
                                )
                                .clicked()
                            {
                                self.choose(index);
                            }
                        });
                        ui.label(
                            lumen::body(11.0, format!("{period} · {kind}")).color(lumen::MUTED),
                        );
                        ui.add_space(6.0);
                        lumen::hairline(ui);
                        ui.add_space(4.0);
                    });
                }
            });
    }
    fn detail(&mut self, ui: &mut egui::Ui) {
        let m = self.entries[self.active].clone();
        egui::ScrollArea::vertical().id_salt("movement-detail").auto_shrink([false,false]).show(ui,|ui|{
            ui.add_space(12.0);ui.label(lumen::display(32.0,&m.label));if m.original_name!=m.label{ui.label(lumen::body(14.0,&m.original_name).color(lumen::MUTED));}
            ui.label(lumen::body(12.0,format!("{} · {} · {}",m.kind.label(),m.period(),m.regions.join(", "))).color(lumen::MUTED));ui.add_space(14.0);ui.label(lumen::body(16.0,&m.summary));ui.add_space(10.0);
            if lumen::button_sized(ui,"Prepara una ricerca",true,190.0).clicked(){if !self.request.movements.contains(&m.id){self.request.movements=vec![m.id.clone()];}self.composer=true;self.compact_list=false;}
            ui.add_space(20.0);ui.heading("Come riconoscerla");for item in &m.traits{ui.label(lumen::body(14.0,item));}
            ui.add_space(16.0);ui.heading("Figure di riferimento");ui.label(lumen::body(14.0,m.authors.join(", ")));
            ui.add_space(16.0);ui.heading("Opere per orientarsi");for work in &m.works{ui.horizontal_wrapped(|ui|{ui.label(lumen::bold(14.0,&work.title));ui.label(lumen::body(13.0,work.year.to_string()).color(lumen::MUTED));});}
            ui.add_space(6.0);ui.label(lumen::body(12.0,"Esempi orientativi: appartenenze, precursori e casi di confine vanno letti nel loro contesto.").color(lumen::MUTED));
            ui.add_space(16.0);ui.heading("Confini e interpretazioni");ui.label(lumen::body(14.0,&m.scope_note));
            ui.add_space(16.0);ui.heading("Fonti e approfondimenti");for source in &m.sources{ui.add(egui::Hyperlink::from_label_and_url(&source.title,&source.url));}
            ui.add_space(16.0);ui.label(lumen::body(12.0,"Periodi indicativi, categorie distinte e copertura ampliabile: nessuna raccolta esaurisce ogni interpretazione della storia del cinema. Revisione: 2 novembre 2026.").color(lumen::MUTED));ui.add_space(18.0);
        });
    }
    fn composer(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        ui.label(lumen::display(28.0, "Prepara la tua ricerca"));
        ui.label(
            lumen::body(
                13.0,
                "Il testo viene preparato qui. Copialo nella chat che preferisci.",
            )
            .color(lumen::MUTED),
        );
        ui.horizontal_wrapped(|ui| {
            if lumen::button_sized(ui, "Genera il prompt", true, 160.0)
                .on_hover_text("Generazione locale · Ctrl+Invio")
                .clicked()
            {
                self.generate();
            }
            if ui
                .add_enabled(
                    self.ready(),
                    egui::Button::new(if self.copied {
                        "Prompt copiato"
                    } else {
                        "Copia prompt"
                    }),
                )
                .clicked()
            {
                ui.ctx().copy_text(self.prompt.clone());
                self.copied = true;
            }
            egui::ComboBox::from_id_salt("provider")
                .selected_text(self.provider.label())
                .show_ui(ui, |ui| {
                    for p in Provider::ALL {
                        ui.selectable_value(&mut self.provider, p, p.label());
                    }
                });
            if ui.button("Apri chat").clicked() {
                match self.provider.url(&self.custom_url) {
                    Ok(url) => {
                        ui.ctx().open_url(egui::OpenUrl::new_tab(url));
                        self.error = None;
                    }
                    Err(e) => self.error = Some(e),
                }
            }
        });
        if self.provider == Provider::Custom {
            ui.add(
                egui::TextEdit::singleline(&mut self.custom_url)
                    .hint_text("https://indirizzo-della-chat/")
                    .desired_width(f32::INFINITY),
            );
        }
        if let Some(e) = &self.error {
            ui.label(lumen::body(13.0, e).color(lumen::DANGER));
        }
        if !self.prompt.is_empty() && !self.ready() {
            ui.label(
                lumen::body(
                    12.0,
                    "Criteri modificati: rigenera il prompt prima di copiarlo.",
                )
                .color(lumen::EMBER),
            );
        }
        ui.add_space(8.0);
        lumen::hairline(ui);
        egui::ScrollArea::vertical().id_salt("prompt-composer").auto_shrink([false,false]).show(ui,|ui|{
            ui.horizontal_wrapped(|ui|{for id in self.request.movements.clone(){if let Some(m)=self.entries.iter().find(|m|m.id==id){if ui.button(format!("{} ×",m.label)).on_hover_text("Rimuovi dal prompt").clicked(){self.request.movements.retain(|x|x!=&id);}}}});
            if self.request.movements.is_empty(){ui.label(lumen::body(13.0,"Seleziona almeno una voce nell'elenco delle correnti.").color(lumen::EMBER));}
            ui.add_space(8.0);ui.horizontal_wrapped(|ui|{
                egui::ComboBox::from_id_salt("mode").selected_text(self.request.mode.label()).show_ui(ui,|ui|{for mode in Mode::ALL{ui.selectable_value(&mut self.request.mode,mode,mode.label());}});
                let mut specific=self.request.count.is_some();if ui.checkbox(&mut specific,"Numero di film preciso").changed(){self.request.count=if specific{Some(5)}else{None};}
                if let Some(n)=&mut self.request.count{ui.add(egui::DragValue::new(n).range(1..=u32::MAX).suffix(" film"));}
            });ui.horizontal_wrapped(|ui|{
                egui::ComboBox::from_id_salt("depth").selected_text(if self.request.depth==Depth::Concise{"Risposta sintetica"}else{"Risposta approfondita"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.request.depth,Depth::Concise,"Risposta sintetica");ui.selectable_value(&mut self.request.depth,Depth::Detailed,"Risposta approfondita");});
                egui::ComboBox::from_id_salt("relation").selected_text(if self.request.relation==Relation::Historical{"Appartenenza storica"}else{"Anche eredità e influenze"}).show_ui(ui,|ui|{ui.selectable_value(&mut self.request.relation,Relation::Historical,"Appartenenza storica");ui.selectable_value(&mut self.request.relation,Relation::Legacy,"Anche eredità e influenze");});
            });ui.add_space(8.0);
            field(ui,"La tua richiesta",&mut self.request.question,"Esempio: un percorso accessibile da guardare in coppia, senza scene troppo violente.",3);
            egui::CollapsingHeader::new("Preferenze, esclusioni e titoli già visti").show(ui,|ui|{
                field(ui,"Preferenze",&mut self.request.preferences,"Atmosfera, temi, durata, periodo o altre preferenze…",2);
                field(ui,"Da escludere",&mut self.request.exclusions,"Contenuti o caratteristiche da evitare…",2);
                field(ui,"Già visti o già proposti",&mut self.watched,"Un titolo per riga, oppure separati da punto e virgola.",2);
            });self.sync_watched();
            if !self.prompt.is_empty(){ui.add_space(14.0);ui.heading("Il prompt da inviare");let response=ui.add(egui::TextEdit::multiline(&mut self.prompt).desired_width(f32::INFINITY).desired_rows(14).font(egui::TextStyle::Body));if response.changed(){self.copied=false;}
                ui.label(lumen::body(12.0,"Dopo aver aperto la chat, incolla e invia tu il testo. Puoi usarlo anche in una conversazione già aperta.").color(lumen::MUTED));for w in &self.warnings{ui.label(lumen::body(11.0,w).color(lumen::MUTED));}}
            ui.add_space(20.0);
        });
    }
    pub(crate) fn update_ui(&mut self, ui: &mut egui::Ui) {
        let ctx = ui.ctx().clone();
        window_resize_edges(&ctx);
        if ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::F)) {
            self.search_focus = true;
            self.compact_list = true;
        }
        if self.composer
            && ctx.input_mut(|i| i.consume_key(egui::Modifiers::CTRL, egui::Key::Enter))
        {
            self.generate();
        }
        self.header(ui);
        let narrow = ctx.content_rect().width() < 1000.0;
        if narrow {
            egui::Panel::top("compact-navigation")
                .frame(
                    egui::Frame::NONE
                        .fill(lumen::DEEP)
                        .inner_margin(egui::Margin::symmetric(16, 6)),
                )
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        if ui
                            .selectable_label(self.compact_list, "Esplora le correnti")
                            .clicked()
                        {
                            self.compact_list = true;
                        }
                        if ui
                            .selectable_label(!self.compact_list && !self.composer, "Scheda")
                            .clicked()
                        {
                            self.compact_list = false;
                            self.composer = false;
                        }
                        if ui
                            .selectable_label(
                                !self.compact_list && self.composer,
                                "Prepara una ricerca",
                            )
                            .clicked()
                        {
                            self.compact_list = false;
                            self.composer = true;
                        }
                    });
                });
        } else {
            egui::Panel::left("navigator")
                .exact_size(330.0)
                .resizable(false)
                .frame(egui::Frame::NONE.fill(lumen::DEEP).inner_margin(16))
                .show(ui, |ui| self.navigator(ui));
        }
        egui::CentralPanel::default()
            .frame(
                egui::Frame::NONE
                    .fill(lumen::INK)
                    .inner_margin(egui::Margin::symmetric(24, 12)),
            )
            .show(ui, |ui| {
                if narrow && self.compact_list {
                    self.navigator(ui);
                } else {
                    ui.horizontal_wrapped(|ui| {
                        if ui.selectable_label(!self.composer, "La corrente").clicked() {
                            self.composer = false;
                        }
                        if ui
                            .selectable_label(
                                self.composer,
                                format!("Prepara una ricerca ({})", self.request.movements.len()),
                            )
                            .clicked()
                        {
                            self.composer = true;
                        }
                    });
                    lumen::hairline(ui);
                    if self.composer {
                        self.composer(ui);
                    } else {
                        self.detail(ui);
                    }
                }
            });
    }
}
fn field(ui: &mut egui::Ui, label: &str, value: &mut String, hint: &str, rows: usize) {
    ui.label(lumen::bold(13.0, label));
    ui.add(
        egui::TextEdit::multiline(value)
            .hint_text(hint)
            .desired_width(f32::INFINITY)
            .desired_rows(rows),
    );
    ui.add_space(4.0);
}
impl eframe::App for App {
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.update_ui(ui);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn provider_links_never_embed_the_prompt_or_credentials() {
        for p in [
            Provider::ChatGpt,
            Provider::DeepSeek,
            Provider::Claude,
            Provider::Gemini,
        ] {
            let u = url::Url::parse(&p.url("").unwrap()).unwrap();
            assert_eq!(u.scheme(), "https");
            assert!(u.query().is_none());
        }
        for s in [
            "javascript:alert(1)",
            "file:///private",
            "http://example.com",
            "https://name:secret@example.com",
            "https://example.com/?prompt=private",
        ] {
            assert!(Provider::Custom.url(s).is_err());
        }
    }
    #[test]
    fn changed_criteria_disable_stale_copy() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.generate();
        assert!(app.ready());
        app.request.preferences = "niente horror".into();
        assert!(!app.ready());
        app.generate();
        assert!(app.ready());
        assert!(app.prompt.contains("niente horror"));
    }
    #[test]
    fn search_filters_and_eras_work_together() {
        let ctx = egui::Context::default();
        let mut app = App::new(&ctx);
        app.query = "nová vlna".into();
        assert_eq!(app.filtered().len(), 1);
        app.era = Era::Origins;
        assert!(app.filtered().is_empty());
        app.query.clear();
        app.kind = Some(Kind::Movement);
        for i in app.filtered() {
            assert_eq!(app.entries[i].kind, Kind::Movement);
        }
    }
    #[test]
    fn layouts_render_at_supported_sizes() {
        for size in [[800.0, 600.0], [1024.0, 768.0], [1440.0, 900.0]] {
            for state in ["detail", "list", "prompt", "generated", "empty"] {
                let ctx = egui::Context::default();
                lumen::install(&ctx);
                let mut app = App::new(&ctx);
                app.compact_list = state == "list" || state == "empty";
                app.composer = state == "prompt" || state == "generated";
                if state == "empty" {
                    app.query = "nessunarispostaxyz".into();
                }
                if state == "generated" {
                    app.generate();
                }
                let mut output = ctx.run_ui(
                    egui::RawInput {
                        screen_rect: Some(egui::Rect::from_min_size(
                            egui::Pos2::ZERO,
                            egui::vec2(size[0], size[1]),
                        )),
                        ..Default::default()
                    },
                    |ui| app.update_ui(ui),
                );
                output.textures_delta.clear();
                assert!(!output.shapes.is_empty());
                let text = output
                    .shapes
                    .iter()
                    .filter_map(|s| {
                        if let egui::epaint::Shape::Text(t) = &s.shape {
                            Some(t.galley.text())
                        } else {
                            None
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                assert!(text.contains("CINE") && !text.contains("CHIAVE API"));
                if state == "prompt" || state == "generated" {
                    assert!(
                        text.contains("Genera il prompt")
                            && text.contains("Copia prompt")
                            && text.contains("Apri chat")
                    );
                }
            }
        }
    }
}
