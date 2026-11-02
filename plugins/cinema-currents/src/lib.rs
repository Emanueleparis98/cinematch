//! Portable offline reference and prompt generator. No network, clipboard or filesystem access.
use serde::{Deserialize, Serialize};
use std::fmt::Write;

pub const SCHEMA_VERSION: u32 = 1;
#[derive(Clone, Copy, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Movement,
    Tendency,
    Cycle,
    Tradition,
    Technique,
}
impl Kind {
    pub const ALL: [Self; 5] = [
        Self::Movement,
        Self::Tendency,
        Self::Cycle,
        Self::Tradition,
        Self::Technique,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Movement => "Movimento storico",
            Self::Tendency => "Tendenza / scena",
            Self::Cycle => "Genere / ciclo",
            Self::Tradition => "Tradizione",
            Self::Technique => "Tecnica / ambito produttivo",
        }
    }
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Source {
    pub title: String,
    pub url: String,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Work {
    pub title: String,
    pub year: u16,
}
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Movement {
    pub id: String,
    pub label: String,
    pub original_name: String,
    pub kind: Kind,
    pub start: u16,
    pub end: Option<u16>,
    pub regions: Vec<String>,
    pub aliases: Vec<String>,
    pub summary: String,
    pub traits: Vec<String>,
    pub authors: Vec<String>,
    pub works: Vec<Work>,
    pub sources: Vec<Source>,
    pub scope_note: String,
    pub original_entry: bool,
}
impl Movement {
    pub fn period(&self) -> String {
        match self.end {
            Some(end) => format!("circa {}–{end}", self.start),
            None => format!("dal {} circa · sviluppi e continuità", self.start),
        }
    }
    pub fn matches(&self, query: &str) -> bool {
        let hay = normalize(&format!(
            "{} {} {} {} {} {} {}",
            self.label,
            self.original_name,
            self.aliases.join(" "),
            self.regions.join(" "),
            self.authors.join(" "),
            self.summary,
            self.works
                .iter()
                .map(|w| w.title.as_str())
                .collect::<Vec<_>>()
                .join(" ")
        ));
        normalize(query)
            .split_whitespace()
            .all(|word| hay.contains(word))
    }
}
pub fn normalize(s: &str) -> String {
    s.to_lowercase()
        .chars()
        .map(|c| match c {
            'à' | 'á' | 'â' | 'ä' | 'ã' | 'å' => 'a',
            'è' | 'é' | 'ê' | 'ë' => 'e',
            'ì' | 'í' | 'î' | 'ï' => 'i',
            'ò' | 'ó' | 'ô' | 'ö' | 'õ' => 'o',
            'ù' | 'ú' | 'û' | 'ü' => 'u',
            'ç' | 'č' => 'c',
            'ñ' | 'ń' => 'n',
            'š' | 'ś' => 's',
            'ž' | 'ź' | 'ż' => 'z',
            _ => c,
        })
        .collect()
}
pub fn catalog() -> Vec<Movement> {
    serde_json::from_str(include_str!("../assets/movements.json"))
        .expect("validated bundled reference")
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Mode {
    #[default]
    Recommendations,
    Introduction,
    Discoveries,
    Compare,
    Free,
}
impl Mode {
    pub const ALL: [Self; 5] = [
        Self::Recommendations,
        Self::Introduction,
        Self::Discoveries,
        Self::Compare,
        Self::Free,
    ];
    pub fn label(self) -> &'static str {
        match self {
            Self::Recommendations => "Consigli di visione",
            Self::Introduction => "Introduzione e percorso",
            Self::Discoveries => "Opere meno conosciute",
            Self::Compare => "Confronta correnti",
            Self::Free => "Richiesta libera",
        }
    }
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Depth {
    #[default]
    Concise,
    Detailed,
}
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Relation {
    #[default]
    Historical,
    Legacy,
}
#[derive(Clone, Debug, Default, Deserialize, Serialize, PartialEq)]
#[serde(default, deny_unknown_fields)]
pub struct PromptRequest {
    pub schema_version: u32,
    pub movements: Vec<String>,
    pub mode: Mode,
    pub count: Option<u32>,
    pub depth: Depth,
    pub relation: Relation,
    pub preferences: String,
    pub exclusions: String,
    pub watched: Vec<String>,
    pub question: String,
}
#[derive(Clone, Debug, Serialize)]
pub struct PromptResponse {
    pub schema_version: u32,
    pub prompt: String,
    pub warnings: Vec<String>,
}

pub fn generate(request: &PromptRequest) -> Result<PromptResponse, String> {
    if request.schema_version != 0 && request.schema_version != SCHEMA_VERSION {
        return Err("Versione della richiesta non supportata.".into());
    }
    if request.movements.is_empty() {
        return Err("Scegli almeno una corrente.".into());
    }
    if request.count == Some(0) {
        return Err("Il numero di film deve essere maggiore di zero oppure automatico.".into());
    }
    if request.mode == Mode::Free && request.question.trim().is_empty() {
        return Err("Scrivi la richiesta libera.".into());
    }
    let reference = catalog();
    let mut selected = Vec::new();
    for id in &request.movements {
        let m = reference
            .iter()
            .find(|m| m.id == *id)
            .ok_or_else(|| format!("Voce non riconosciuta: {id}"))?;
        if !selected.iter().any(|x: &&Movement| x.id == m.id) {
            selected.push(m);
        }
    }
    if request.mode == Mode::Compare && selected.len() < 2 {
        return Err("Per il confronto scegli almeno due correnti.".into());
    }
    let mut text = String::from(
        "Aiutami a esplorare il cinema con attenzione storica. Rispondi in italiano.\n\n",
    );
    writeln!(text, "OBIETTIVO: {}.", request.mode.label()).unwrap();
    if let Some(n) = request.count {
        writeln!(text,"Quantità richiesta: {n} film complessivi, senza duplicati. Se non trovi abbastanza opere pertinenti, spiega il limite invece di aggiungere titoli estranei.").unwrap();
    } else if request.mode == Mode::Free {
        text.push_str("Rispondi alla richiesta libera rispettandone il formato; proponi film soltanto se richiesti.\n");
    } else {
        text.push_str("Quantità: proponi un numero motivato e adeguato alla richiesta, senza un limite predefinito di cinque film.\n");
    }
    text.push_str("\nCONTESTO DELLE VOCI SELEZIONATE\n");
    for m in &selected {
        writeln!(
            text,
            "- {} / {} — {}; {}; {}.",
            m.label,
            m.original_name,
            m.kind.label(),
            m.period(),
            m.regions.join(", ")
        )
        .unwrap();
        writeln!(text, "  {}", m.summary).unwrap();
        writeln!(text, "  Confini: {}", m.scope_note).unwrap();
        if request.depth == Depth::Detailed {
            writeln!(
                text,
                "  Caratteristiche: {}. Figure di riferimento: {}.",
                m.traits.join("; "),
                m.authors.join(", ")
            )
            .unwrap();
        }
        if let Some(source) = m.sources.first() {
            writeln!(
                text,
                "  Riferimento da consultare se puoi navigare: {} — {}",
                source.title, source.url
            )
            .unwrap();
        }
    }
    if request.relation == Relation::Historical {
        text.push_str("\nPrivilegia opere riconosciute come appartenenti alla corrente o al ciclo richiesto. Una coincidenza di anno, paese o genere non basta. Se includi un precursore o un caso discusso, dichiaralo.\n");
    } else {
        text.push_str("\nIncludi anche eredità e influenze successive, distinguendole chiaramente dalle appartenenze storiche. Motiva il collegamento concreto.\n");
    }
    match request.mode {
        Mode::Introduction=>text.push_str("Costruisci un percorso accessibile dal contesto storico alle opere; spiega come guardarle e in quale ordine.\n"),
        Mode::Discoveries=>text.push_str("Privilegia opere meno note rispetto ai titoli canonici; non dichiararle rare o disponibili in streaming senza verifica.\n"),
        Mode::Compare=>text.push_str("Confronta le voci senza fonderle: differenze, punti di contatto ed esempi distinti; ripartisci la quantità complessiva in modo esplicito.\n"),
        _=>{},
    }
    if !request.preferences.trim().is_empty() {
        writeln!(
            text,
            "\nPREFERENZE PERSONALI\n{}",
            request.preferences.trim()
        )
        .unwrap();
    }
    if !request.exclusions.trim().is_empty() {
        writeln!(
            text,
            "\nCRITERI DA ESCLUDERE\n{}",
            request.exclusions.trim()
        )
        .unwrap();
    }
    let watched = request
        .watched
        .iter()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>();
    if !watched.is_empty() {
        writeln!(text,"\nGIÀ VISTI / GIÀ PROPOSTI — non riproporre questi titoli, nemmeno con un titolo alternativo:\n{}",watched.join("; ")).unwrap();
    }
    if !request.question.trim().is_empty() {
        writeln!(text, "\nLA MIA RICHIESTA\n{}", request.question.trim()).unwrap();
    }
    text.push_str("\nCOME RISPONDERE\nPer ogni film che proponi indica titolo originale, titolo italiano se noto, anno e regista; spiega l'attinenza specifica e come rispetta i miei criteri. ");
    if request.depth == Depth::Concise {
        text.push_str("Usa spiegazioni brevi e concrete. ");
    } else {
        text.push_str("Aggiungi un'analisi storica e formale più approfondita. ");
    }
    text.push_str("Distingui fatti verificati, attribuzioni discusse e tue inferenze. Se puoi navigare, controlla i riferimenti e cita pagine pertinenti; altrimenti dichiara che non hai verificato online. Non inventare fonti, URL, film, dati o disponibilità sulle piattaforme. Evita spoiler importanti. Se alcuni criteri sono incompatibili, spiegalo prima di proporre alternative.\n");
    let mut warnings=vec!["Il testo è una richiesta per una chat esterna: la qualità e i limiti della risposta dipendono dal servizio scelto.".into()];
    if selected.iter().any(|m| m.kind != Kind::Movement) {
        warnings.push("La selezione comprende anche tendenze, generi o tradizioni: il prompt li distingue dai movimenti storici.".into());
    }
    Ok(PromptResponse {
        schema_version: SCHEMA_VERSION,
        prompt: text,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn free_requests_do_not_force_film_recommendations() {
        let mut r = request();
        r.mode = Mode::Free;
        r.count = None;
        r.question = "Spiega il rapporto fra montaggio e realismo, senza un elenco di film.".into();
        let prompt = generate(&r).unwrap().prompt;
        assert!(prompt.contains("proponi film soltanto se richiesti"));
        assert!(prompt.contains(&r.question));
        assert!(!prompt.contains("Greek Weird Wave"));
    }
    #[test]
    fn historical_corrections_do_not_restore_false_end_dates() {
        let all = catalog();
        let find = |id: &str| all.iter().find(|m| m.id == id).unwrap();
        assert_eq!(find("german_new_cinema").start, 1962);
        assert!(find("new_hollywood").end.unwrap() >= 1980);
        assert!(find("korean_new_wave").start < 1990);
        assert!(find("dogme95").regions.contains(&"Internazionale".into()));
        for id in ["wuxia", "kaiju", "tokusatsu", "slow_cinema"] {
            assert!(find(id).end.is_none());
        }
    }
    fn request() -> PromptRequest {
        PromptRequest {
            movements: vec!["neorealismo".into()],
            count: Some(1),
            ..Default::default()
        }
    }
    #[test]
    fn counts_are_explicit_and_not_capped_at_five() {
        for n in [1, 5, 12, 100] {
            let mut r = request();
            r.count = Some(n);
            assert!(generate(&r)
                .unwrap()
                .prompt
                .contains(&format!("{n} film complessivi")));
        }
    }
    #[test]
    fn automatic_count_and_personal_criteria_survive() {
        let mut r = request();
        r.count = None;
        r.preferences = "amicizia tra un cane e un gatto".into();
        r.exclusions = "horror".into();
        r.watched = vec!["Ladri di biciclette".into()];
        let p = generate(&r).unwrap().prompt;
        assert!(p.contains("senza un limite predefinito"));
        assert!(
            p.contains(&r.preferences) && p.contains(&r.exclusions) && p.contains(&r.watched[0])
        );
    }
    #[test]
    fn compare_needs_two_distinct_valid_entries() {
        let mut r = request();
        r.mode = Mode::Compare;
        assert!(generate(&r).is_err());
        r.movements.push("neorealismo".into());
        assert!(generate(&r).is_err());
        r.movements.push("nouvelle_vague".into());
        assert!(generate(&r).is_ok());
    }
    #[test]
    fn invalid_requests_are_explicit_errors() {
        let mut r = request();
        r.count = Some(0);
        assert!(generate(&r).is_err());
        r.count = None;
        r.movements = vec!["unknown".into()];
        assert!(generate(&r).is_err());
        r = request();
        r.schema_version = 999;
        assert!(generate(&r).is_err());
        r = request();
        r.mode = Mode::Free;
        assert!(generate(&r).is_err());
    }
    #[test]
    fn detailed_context_is_opt_in() {
        let mut r = request();
        let brief = generate(&r).unwrap().prompt;
        r.depth = Depth::Detailed;
        let detailed = generate(&r).unwrap().prompt;
        assert!(detailed.len() > brief.len());
        assert!(!brief.contains("Figure di riferimento:"));
    }
    #[test]
    fn aliases_authors_and_accents_are_searchable() {
        let all = catalog();
        assert!(all.iter().any(|m| m.matches("nouvELLe")));
        assert!(all.iter().any(|m| m.matches("nová vlna")));
        assert!(all.iter().any(|m| m.matches("De Sica")));
    }
    #[test]
    fn reference_has_valid_unique_ids_sources_and_all_original_entries() {
        let all = catalog();
        assert!(all.len() >= 65);
        assert_eq!(all.iter().filter(|m| m.original_entry).count(), 26);
        let mut ids = std::collections::HashSet::new();
        for m in &all {
            assert!(ids.insert(&m.id));
            assert!(m.end.is_none_or(|e| e >= m.start));
            assert!(!m.sources.is_empty() && !m.works.is_empty() && !m.authors.is_empty());
            for s in &m.sources {
                assert!(s.url.starts_with("https://") && !s.title.is_empty());
            }
        }
    }
}
