//! Curatela della chat: sceglie fino a 5 film fra i candidati reali di TMDB e
//! scrive introduction e commenti. Il curatore non inventa mai titoli: e un
//! ordinatore con la penna. Ogni id fuori elenco viene scartato; se l'AI non
//! c'e, la scelta e deterministica (voto x voti) e i testi li compone il
//! motore, cosi la chat funziona anche in modalita regole.

use std::collections::HashSet;

use serde::Deserialize;

/// Un film reale proposto al curatore: pochi campi, quelli che servono per
/// scegliere e per spiegare la scelta.
#[derive(Clone, Debug)]
pub struct Candidate {
    pub id: u64,
    pub title: String,
    pub year: Option<u16>,
    pub rating: f32,
    pub votes: u32,
    pub genres: String,
    pub overview: String,
}

/// Il materiale che il curatore riceve: richiesta, filtri capiti, memoria
/// recente della chat e candidati reali.
#[derive(Clone, Debug)]
pub struct Request {
    pub user_text: String,
    pub understood: String,
    pub history: Vec<(String, String)>,
    pub candidates: Vec<Candidate>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct Pick {
    pub id: u64,
    pub perche: String,
    pub consiglio: String,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Curation {
    pub intro: String,
    pub picks: Vec<Pick>,
}

// ------------------------------------------------------- scelta deterministica

/// Punteggio del motore quando l'AI non c'e: voto pesato dai voti, cosi un
/// 8.5 con 200 voti non batte un 7.9 con 20.000.
pub fn score(c: &Candidate) -> f64 {
    c.rating as f64 * (1.0 + c.votes as f64).ln()
}

fn understood_of(req: &Request) -> String {
    let u = req.understood.trim();
    if u.is_empty() {
        "ricerca libera".to_string()
    } else {
        u.to_string()
    }
}

/// Testo di commento deterministico per un candidato (usato anche per
/// completare le scelte dell'AI quando ne propone meno di cinque).
pub fn pick_texts(c: &Candidate, understood: &str) -> (String, String) {
    let genres = if c.genres.trim().is_empty() {
        String::new()
    } else {
        format!(" Generi: {}.", c.genres)
    };
    let perche = format!(
        "«{}» rientra nei criteri della ricerca: {}.{}",
        c.title, understood, genres
    );
    let consiglio = if c.rating >= 7.5 {
        "Tra i titoli piu solidi del gruppo: buon punto di partenza. Apri la scheda per cast e disponibilita."
    } else {
        "Scelta sicura per la serata: apri la scheda per i dettagli e dove si vede."
    };
    (perche, consiglio.to_string())
}

/// Chiude un testo su un limite di caratteri, senza spezzare le parole in
/// modo brutale.
fn cut(text: &str, max: usize) -> String {
    let compact = text.split_whitespace().collect::<Vec<_>>().join(" ");
    if compact.chars().count() <= max {
        return compact;
    }
    let mut out: String = compact.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

pub fn intro_text(n: usize, understood: &str) -> String {
    if n <= 1 {
        format!("Ecco il titolo che corrisponde di piu: {understood}.")
    } else {
        format!("Ecco {n} titoli in linea con la richiesta: {understood}.")
    }
}

/// Curatela senza AI: i migliori per punteggio, testi composti dal motore.
pub fn deterministic(req: &Request) -> Curation {
    let understood = understood_of(req);
    let n = req.candidates.len().min(5);
    let mut ranked: Vec<&Candidate> = req.candidates.iter().collect();
    ranked.sort_by(|a, b| {
        score(b)
            .partial_cmp(&score(a))
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let picks = ranked
        .into_iter()
        .take(n)
        .map(|c| {
            let (perche, consiglio) = pick_texts(c, &understood);
            Pick {
                id: c.id,
                perche,
                consiglio,
            }
        })
        .collect();
    Curation {
        intro: intro_text(n, &understood),
        picks,
    }
}

// ------------------------------------------------------------- parsing AI ----

#[derive(Deserialize)]
struct RawPick {
    #[serde(default)]
    id: serde_json::Value,
    #[serde(default)]
    perche: String,
    #[serde(default)]
    consiglio: String,
}

#[derive(Deserialize)]
struct RawCuration {
    #[serde(default)]
    intro: String,
    #[serde(default)]
    picks: Vec<RawPick>,
}

fn value_id(v: &serde_json::Value) -> Option<u64> {
    v.as_u64().or_else(|| v.as_str().and_then(|s| s.trim().parse().ok()))
}

fn extract_object(raw: &str) -> Option<&str> {
    let start = raw.find('{')?;
    let end = raw.rfind('}')?;
    if end <= start {
        return None;
    }
    Some(&raw[start..=end])
}

/// Interpreta la risposta del curatore AI: scarta id inventati, doppioni e
/// commenti vuoti. Ordine preservato; massimo cinque scelte.
pub fn parse_curation(raw: &str, valid: &HashSet<u64>) -> Result<Curation, String> {
    let cleaned = extract_object(raw).ok_or("nessun oggetto JSON nella risposta")?;
    let parsed: RawCuration =
        serde_json::from_str(cleaned).map_err(|e| format!("JSON curatela non conforme: {e}"))?;
    let mut picks: Vec<Pick> = Vec::new();
    for p in parsed.picks {
        let Some(id) = value_id(&p.id) else { continue };
        if !valid.contains(&id) {
            continue;
        }
        if picks.iter().any(|q| q.id == id) {
            continue;
        }
        let perche = cut(&p.perche, 320);
        if perche.is_empty() {
            continue;
        }
        picks.push(Pick {
            id,
            perche,
            consiglio: cut(&p.consiglio, 220),
        });
        if picks.len() == 5 {
            break;
        }
    }
    if picks.is_empty() {
        return Err("la curatela non ha scelto film validi".into());
    }
    Ok(Curation {
        intro: cut(&parsed.intro, 300),
        picks,
    })
}

// -------------------------------------------------- streaming dell'intro -----

/// Estrae il valore del campo `intro` da un JSON ancora in costruzione.
/// Ritorna il testo decodificato disponibile finora (anche parziale).
pub fn intro_prefix(raw: &str) -> Option<String> {
    let key = raw.find("\"intro\"")?;
    let rest = &raw[key + 7..];
    let colon = rest.find(':')?;
    let body = rest[colon + 1..].trim_start().strip_prefix('"')?;
    let mut out = String::new();
    let mut it = body.chars();
    while let Some(ch) = it.next() {
        match ch {
            '"' => return Some(out),
            '\\' => match it.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('r') => out.push('\r'),
                Some('b') => out.push('\u{8}'),
                Some('f') => out.push('\u{c}'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('/') => out.push('/'),
                Some('u') => {
                    let hex: String = it.by_ref().take(4).collect();
                    if hex.len() < 4 {
                        return Some(out); // escape incompleto: aspettiamo
                    }
                    if let Ok(code) = u32::from_str_radix(&hex, 16) {
                        if (0xD800..0xDC00).contains(&code) {
                            // surrogato alto: prova la coppia
                            let mut look = it.clone();
                            if look.next() == Some('\\') && look.next() == Some('u') {
                                let hex2: String = look.by_ref().take(4).collect();
                                if hex2.len() == 4 {
                                    if let Ok(lo) = u32::from_str_radix(&hex2, 16) {
                                        if (0xDC00..0xE000).contains(&lo) {
                                            let c = 0x10000
                                                + ((code - 0xD800) << 10)
                                                + (lo - 0xDC00);
                                            if let Some(joined) = char::from_u32(c) {
                                                out.push(joined);
                                            }
                                            it = look;
                                            continue;
                                        }
                                    }
                                }
                            }
                            out.push('\u{FFFD}');
                        } else if let Some(decoded) = char::from_u32(code) {
                            out.push(decoded);
                        }
                    }
                }
                Some(other) => out.push(other),
                None => return Some(out), // escape incompleto: aspettiamo
            },
            _ => out.push(ch),
        }
    }
    Some(out)
}

/// Accumula il JSON in arrivo e restituisce solo la parte nuova dell'intro,
/// pronta per l'effetto macchina da scrivere.
#[derive(Default)]
pub struct IntroStream {
    prev: usize,
}

impl IntroStream {
    pub fn feed(&mut self, raw: &str) -> String {
        let Some(current) = intro_prefix(raw) else {
            return String::new();
        };
        let chars: Vec<char> = current.chars().collect();
        if chars.len() <= self.prev {
            return String::new();
        }
        let delta: String = chars[self.prev..].iter().collect();
        self.prev = chars.len();
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cand(id: u64, title: &str, rating: f32, votes: u32) -> Candidate {
        Candidate {
            id,
            title: title.into(),
            year: Some(1990),
            rating,
            votes,
            genres: "Horror".into(),
            overview: "Una trama.".into(),
        }
    }

    fn req(candidates: Vec<Candidate>) -> Request {
        Request {
            user_text: "slasher anni 80".into(),
            understood: "Anni '80 · Stati Uniti".into(),
            history: Vec::new(),
            candidates,
        }
    }

    #[test]
    fn deterministico_ordina_per_punteggio_e_scrive_testi() {
        let r = req(vec![
            cand(1, "Basso", 5.0, 10),
            cand(2, "Alto", 7.9, 20_000),
            cand(3, "Medio", 8.5, 300),
            cand(4, "Quarto", 7.0, 4_000),
            cand(5, "Quinto", 6.8, 5_000),
            cand(6, "Sesto", 6.0, 900),
        ]);
        let c = deterministic(&r);
        assert_eq!(c.picks.len(), 5);
        assert_eq!(c.picks[0].id, 2);
        assert!(c.intro.contains("5 titoli"));
        assert!(!c.picks[0].perche.is_empty());
        assert!(!c.picks[0].perche.contains("Voto"));
        assert!(!c.picks[0].perche.contains("1990"));
        assert!(!c.picks[0].consiglio.is_empty());
        assert!(c.picks.iter().all(|p| p.id != 1));
    }

    #[test]
    fn gli_id_inventati_e_i_doppioni_vengono_scartati() {
        let valid: HashSet<u64> = [10u64, 20u64, 30u64].into_iter().collect();
        let raw = r#"{"intro":"Ecco a te.","picks":[
            {"id":999,"perche":"inventato","consiglio":"x"},
            {"id":20,"perche":"buono","consiglio":"guardalo"},
            {"id":"20","perche":"doppione","consiglio":"x"},
            {"id":"10","perche":"altro","consiglio":"ok"}
        ]}"#;
        let c = parse_curation(raw, &valid).unwrap();
        assert_eq!(c.intro, "Ecco a te.");
        assert_eq!(
            c.picks.iter().map(|p| p.id).collect::<Vec<_>>(),
            vec![20, 10]
        );
    }

    #[test]
    fn parsing_tollera_recinti_markdown_e_testo_intorno() {
        let valid: HashSet<u64> = [7u64].into_iter().collect();
        let raw = "Certo!\n```json\n{\"intro\":\"Prova\",\"picks\":[{\"id\":7,\"perche\":\"p\",\"consiglio\":\"c\"}]}\n```\nfine";
        let c = parse_curation(raw, &valid).unwrap();
        assert_eq!(c.picks.len(), 1);
        assert_eq!(c.picks[0].id, 7);
    }

    #[test]
    fn senza_scelte_valide_e_un_errore() {
        let valid: HashSet<u64> = [7u64].into_iter().collect();
        let raw = r#"{"intro":"x","picks":[{"id":42,"perche":"p","consiglio":"c"}]}"#;
        assert!(parse_curation(raw, &valid).is_err());
    }

    #[test]
    fn intro_prefix_decodifica_escape_e_frammenti() {
        assert_eq!(intro_prefix(r#"{"intro":"Ciao"#).unwrap(), "Ciao");
        assert_eq!(
            intro_prefix(r#"{"intro":"a \"b\" \n c"#).unwrap(),
            "a \"b\" \n c"
        );
        assert_eq!(intro_prefix(r#"{"intro":"a \u00"#).unwrap(), "a ");
        // escape incompleto in coda: non si emette nulla a meta
        assert_eq!(intro_prefix(r#"{"intro":"ok \u00e"#).unwrap(), "ok ");
    }

    #[test]
    fn intro_stream_emette_solo_il_nuovo() {
        let mut s = IntroStream::default();
        assert_eq!(s.feed(r#"{"intro":"Bel"#), "Bel");
        assert_eq!(s.feed(r#"{"intro":"Bella sce"#), "la sce");
        assert_eq!(s.feed(r#"{"intro":"Bella scelta","picks":[]}"#), "lta");
        assert_eq!(s.feed(r#"{"intro":"Bella scelta","picks":[]}"#), "");
    }

    #[test]
    fn testo_troppo_lungo_viene_tagliato() {
        let long = "parola ".repeat(200);
        let c = cut(&long, 50);
        assert!(c.chars().count() <= 50);
        assert!(c.ends_with('…'));
    }
}
