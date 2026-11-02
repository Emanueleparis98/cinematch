//! Anteprima grafica OFFLINE. Nessuna credenziale, ricerca o account reale.
use cinematch::{
    domain::{model::Movie, taxonomy::Taxonomy},
    search::{Requirement, SearchIntent},
    ui::{chat, images::ImageStore, lumen},
};

struct Preview {
    entries: Vec<chat::Entry>,
    images: ImageStore,
    tax: Taxonomy,
    draft: String,
    jump: bool,
    mode: usize,
}
impl Preview {
    fn new(ctx: &egui::Context) -> Self {
        lumen::install(ctx);
        let intent=SearchIntent {request:"12 film su un’amicizia significativa tra un cane e un gatto, con un tono piacevole e senza horror".into(),count:Some(12),
            requirements:vec![Requirement{text:"Amicizia fra le due specie, centrale nella storia".into(),origin:"amicizia tra un cane e un gatto".into(),essential:true}],..Default::default()};
        let cards=(1..=12).map(|id|chat::Card {movie:Movie{id,title:if id==1 {"Oliver & Company — esempio grafico".into()} else {format!("Titolo di prova {id} con un nome piuttosto lungo")},year:Some(1988),rating:7.1,..Default::default()},
            perche:"Il legame tra il gattino e i cani guida la storia: si aiutano durante le difficoltà e costruiscono una famiglia scelta. Questa motivazione è un esempio per controllare la lettura alle diverse dimensioni della finestra.".into(),consiglio:if id==2 {"Alcune scene di pericolo possono risultare intense per i più piccoli.".into()} else {String::new()},
            meta:Some(chat::CardMeta{runtime:Some(90),sources:vec![chat::CardSource{title:"TMDB — esempio di fonte".into(),url:"https://www.themoviedb.org/movie/12233".into(),license:"TMDB API terms".into()},chat::CardSource{title:"Wikipedia contributors".into(),url:"https://en.wikipedia.org/wiki/Oliver_%26_Company".into(),license:"CC BY-SA 4.0".into()}],..Default::default()}) }).collect();
        let entries=vec![chat::Entry{user:"Vorrei 12 film, con motivazioni utili e fonti consultabili.".into(),answer:chat::Answer{state:chat::State::Done,research:Some(intent.clone()),understood:intent.request,intro:"Anteprima grafica con dati di prova. Serve esclusivamente per verificare layout, lettura e controlli.".into(),cards,..Default::default()}}];
        Self {
            entries,
            images: ImageStore::new(),
            tax: Taxonomy::load(),
            draft: String::new(),
            jump: false,
            mode: 0,
        }
    }
}
impl eframe::App for Preview {
    fn ui(&mut self, ui: &mut egui::Ui, _: &mut eframe::Frame) {
        ui.horizontal(|ui| {
            ui.label("ANTEPRIMA OFFLINE · DATI DI PROVA");
            for (i, label) in [
                "12 schede",
                "Risposta testuale",
                "Fonti insufficienti",
                "In corso",
            ]
            .iter()
            .enumerate()
            {
                if ui.selectable_label(self.mode == i, *label).clicked() {
                    self.mode = i;
                }
            }
        });
        let mut entries = self.entries.clone();
        match self.mode {
            1 => {
                entries[0].answer.cards.clear();
                entries[0].answer.text_only = true;
                entries[0].answer.intro="Il montaggio alternato intreccia azioni che avvengono nello stesso momento; quello parallelo accosta situazioni per creare un confronto di significato.\n\nPuoi usarli entrambi nello stesso film: descrivono il rapporto fra le sequenze, non un genere cinematografico.".into();
            }
            2 => {
                entries[0].answer.cards.clear();
                entries[0].answer.limit=Some("Non ho verificato abbastanza corrispondenze. Puoi continuare la ricerca conservando i criteri.".into());
            }
            3 => entries[0].answer.state = chat::State::Busy,
            _ => {}
        }
        egui::Panel::bottom("preview-composer").show(ui, |ui| {
            let width = ui.available_width();
            chat::composer(ui, &mut self.draft, false, false, width);
        });
        egui::CentralPanel::default().show(ui, |ui| {
            let now = ui.input(|i| i.time);
            chat::view(
                ui,
                &self.images,
                &self.tax,
                &entries,
                self.mode == 3,
                now,
                &mut self.jump,
                20.0,
            );
        });
    }
}
fn main() -> eframe::Result<()> {
    eframe::run_native(
        "CineMatch — anteprima ricerca",
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1220.0, 660.0])
                .with_min_inner_size([800.0, 600.0]),
            renderer: eframe::Renderer::Glow,
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(Preview::new(&cc.egui_ctx)))),
    )
}
