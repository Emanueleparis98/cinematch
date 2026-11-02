use std::collections::{BTreeMap, BTreeSet};

use crate::domain::model::{Facets, Sort};
use crate::domain::taxonomy::{Movement, Taxonomy};

/// Disponibilita di una faccia rispetto allo stato corrente.
#[derive(Clone, Debug)]
pub struct Availability {
    pub allowed: bool,
    pub reason: Option<String>,
}

impl Availability {
    pub fn ok() -> Self {
        Self {
            allowed: true,
            reason: None,
        }
    }
    pub fn soft(reason: String) -> Self {
        Self {
            allowed: true,
            reason: Some(reason),
        }
    }
    pub fn blocked(reason: String) -> Self {
        Self {
            allowed: false,
            reason: Some(reason),
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Prune {
    pub genres: BTreeMap<u32, Availability>,
    pub countries: BTreeMap<String, Availability>,
    pub moods: BTreeMap<String, Availability>,
}

/// Unione degli intervalli dei decenni selezionati.
pub fn decade_span(tax: &Taxonomy, f: &Facets) -> Option<(u16, u16)> {
    let mut span: Option<(u16, u16)> = None;
    for id in &f.decades {
        if let Some(d) = tax.decade(id) {
            span = Some(match span {
                None => (d.from, d.to),
                Some((a, b)) => (a.min(d.from), b.max(d.to)),
            });
        }
    }
    span
}

/// Vero quando i decenni scelti formano piu' gruppi separati (es. anni '70 e
/// anni '90). La query TMDB usa un solo intervallo continuo, quindi il vuoto in
/// mezzo viene incluso: la UI lo dichiara. Con un movimento attivo il periodo
/// e gia ristretto e il messaggio sarebbe fuorviante.
pub fn decade_gap(tax: &Taxonomy, f: &Facets) -> bool {
    if !f.movements.is_empty() {
        return false;
    }
    let mut ranges: Vec<(u16, u16)> = f
        .decades
        .iter()
        .filter_map(|id| tax.decade(id).map(|d| (d.from, d.to)))
        .collect();
    if ranges.len() < 2 {
        return false;
    }
    ranges.sort();
    let mut groups = 0usize;
    let mut end = 0u16;
    for (a, b) in ranges {
        if groups == 0 || a > end.saturating_add(1) {
            groups += 1;
            end = b;
        } else {
            end = end.max(b);
        }
    }
    groups > 1
}

/// Intervallo anni effettivo della ricerca.
/// Ok(None) = nessun vincolo temporale. Err = vincolo impossibile.
pub fn year_range(tax: &Taxonomy, f: &Facets) -> Result<Option<(u16, u16)>, String> {
    let mut span = decade_span(tax, f);
    for id in &f.movements {
        let Some(m) = tax.movement(id) else { continue };
        let (mf, mt) = (m.period[0], m.period[1]);
        match span {
            None => span = Some((mf, mt)),
            Some((a, b)) => {
                let lo = a.max(mf);
                let hi = b.min(mt);
                if lo > hi {
                    return Err(format!(
                        "«{}» copre il periodo {}–{}, che non si sovrappone a {}–{}.",
                        m.label, mf, mt, a, b
                    ));
                }
                span = Some((lo, hi));
            }
        }
    }
    Ok(span)
}

/// Paesi comuni a tutti i movimenti indicati (None se nessun movimento valido).
pub fn movement_countries(tax: &Taxonomy, ids: &[String]) -> Option<BTreeSet<String>> {
    let mut set: Option<BTreeSet<String>> = None;
    for id in ids {
        let Some(m) = tax.movement(id) else { continue };
        let s: BTreeSet<String> = m.countries.iter().cloned().collect();
        set = Some(match set {
            None => s,
            Some(cur) => cur.union(&s).cloned().collect(),
        });
    }
    set
}

/// Paesi da passare a TMDB: quelli scelti, oppure quelli implicati dai movimenti.
pub fn effective_countries(tax: &Taxonomy, f: &Facets) -> Vec<String> {
    if !f.countries.is_empty() {
        return f.countries.clone();
    }
    movement_countries(tax, &f.movements)
        .map(|s| s.into_iter().collect())
        .unwrap_or_default()
}

/// Generi esclusi d'ufficio dai movimenti scelti (finiscono in without_genres).
pub fn forbidden_genres(_tax: &Taxonomy, _f: &Facets) -> Vec<u32> {
    Vec::new()
}

/// Calcola, per ogni faccia, se e ancora selezionabile e con quale motivazione.
pub fn pruning(tax: &Taxonomy, _f: &Facets) -> Prune {
    // Una generalizzazione editoriale non rende impossibile una combinazione.
    Prune {
        genres: tax
            .genres
            .iter()
            .map(|g| (g.id, Availability::ok()))
            .collect(),
        countries: tax
            .countries
            .iter()
            .map(|c| (c.code.clone(), Availability::ok()))
            .collect(),
        moods: tax
            .moods
            .iter()
            .map(|m| (m.id.clone(), Availability::ok()))
            .collect(),
    }
}

/// Propagazione dei vincoli: corregge le facce impossibili e spiega ogni correzione.
pub fn sanitize(tax: &Taxonomy, input: &Facets) -> (Facets, Vec<String>) {
    let mut msgs: Vec<String> = Vec::new();
    let mut f = input.clone();

    f.decades.retain(|d| {
        let ok = tax.decade(d).is_some();
        if !ok {
            msgs.push(format!("Periodo «{}» non riconosciuto: rimosso.", d));
        }
        ok
    });
    f.genres.retain(|g| tax.genre_name(*g).is_some());
    f.without_genres.retain(|g| tax.genre_name(*g).is_some());
    f.genres.retain(|g| !f.without_genres.contains(g));

    f.movements.retain(|id| tax.movement(id).is_some());
    f.countries.retain(|id| tax.country_name(id).is_some());
    f.moods.retain(|id| tax.mood(id).is_some());

    if let Some(r) = f.min_rating {
        f.min_rating = Some(r.clamp(0.0, 10.0));
    }
    if let Some(r) = f.max_rating {
        f.max_rating = Some(r.clamp(0.0, 10.0));
    }
    if let (Some(a), Some(b)) = (f.min_rating, f.max_rating) {
        if a > b {
            msgs.push(format!(
                "Voto minimo e massimo erano invertiti: li ho rimessi in ordine ({b:.1}–{a:.1})."
            ));
            f.min_rating = Some(b);
            f.max_rating = Some(a);
        }
    }
    // Ogni tema scelto resta un requisito in AND; niente troncamenti nascosti.
    let mut seen: BTreeSet<String> = BTreeSet::new();
    let mut rejected: Vec<String> = Vec::new();
    f.keywords.retain_mut(|k| {
        let clean = k.trim().to_lowercase();
        if clean.is_empty() || clean.chars().count() > 40 {
            rejected.push(k.trim().to_string());
            return false;
        }
        *k = clean.clone();
        seen.insert(clean)
    });
    for k in rejected {
        msgs.push(format!("Tema «{k}» scartato: vuoto o troppo lungo."));
    }
    if let Some((a, b)) = f.runtime {
        if a > b {
            f.runtime = Some((b, a));
        }
    }

    (f, msgs)
}

/// Mood selezionati di cui non conosciamo ancora l'id keyword TMDB.
pub fn unresolved_keywords(
    tax: &Taxonomy,
    f: &Facets,
    known: &[(String, u64)],
) -> Vec<(String, String)> {
    f.moods
        .iter()
        .filter(|id| !known.iter().any(|(k, _)| k == *id))
        .filter_map(|id| tax.mood(id).map(|m| (m.id.clone(), m.keyword.clone())))
        .collect()
}

/// Temi liberi (nomi di parole chiave) di cui non conosciamo ancora l'id TMDB.
pub fn unresolved_themes(f: &Facets, known: &[(String, u64)]) -> Vec<String> {
    f.keywords
        .iter()
        .filter(|k| !known.iter().any(|(name, _)| name == *k))
        .cloned()
        .collect()
}

/// Parole chiave proprie delle correnti scelte (id movimento -> nome keyword),
/// quando l'id TMDB non e ancora noto: l'app le risolve come i mood.
pub fn unresolved_movement_keywords(
    tax: &Taxonomy,
    f: &Facets,
    known: &[(String, u64)],
) -> Vec<(String, String)> {
    f.movements
        .iter()
        .filter(|id| !known.iter().any(|(k, _)| k == *id))
        .filter_map(|id| tax.movement(id))
        .filter(|m| !m.keyword.is_empty())
        .map(|m| (m.id.clone(), m.keyword.clone()))
        .collect()
}

/// Ricetta di una corrente: la combinazione di filtri che la descrive
/// (decenni che toccano il periodo, paesi, generi tipici). La mostra la guida
/// del pannello, e l'utente puo applicarla per vederla negli altri filtri.
pub fn movement_recipe(tax: &Taxonomy, m: &Movement) -> (Vec<String>, Vec<String>, Vec<u32>) {
    let decades = tax
        .decades
        .iter()
        .filter(|d| d.to >= m.period[0] && d.from <= m.period[1])
        .map(|d| d.id.clone())
        .collect();
    (decades, m.countries.clone(), m.genres_core.clone())
}

#[derive(Clone, Debug, Default)]
pub struct DiscoverQuery {
    pub include_adult: bool,
    pub include_video: bool,
    pub genres: Vec<u32>,
    /// Con true i generi vengono passati con l'operatore OR (|) invece di AND (,).
    pub genres_or: bool,
    pub without_genres: Vec<u32>,
    pub keywords: Vec<u64>,
    /// Con true le parole chiave vengono passate in OR (|) invece di AND (,).
    pub keywords_or: bool,
    pub countries: Vec<String>,
    pub year: Option<(u16, u16)>,
    pub date_from: Option<String>,
    pub date_to: Option<String>,
    pub min_rating: Option<f32>,
    pub max_rating: Option<f32>,
    /// Numero minimo di voti TMDB (vote_count.gte).
    pub min_votes: Option<u32>,
    pub runtime: Option<(u32, u32)>,
    pub sort: Sort,
    pub page: u32,
}

/// Traduce le facce (gia sane) nella query TMDB /discover.
/// `known` contiene le parole chiave TMDB gia risolte: per i mood la chiave e
/// l'id del mood, per i temi liberi e il nome del tema.
pub fn to_discover(
    tax: &Taxonomy,
    f: &Facets,
    known: &[(String, u64)],
    page: u32,
) -> Result<DiscoverQuery, String> {
    let year = year_range(tax, f)?;

    // I generi impliciti del movimento sono alternative solo quando
    // l'utente non ha scelto un genere: quelli espliciti restano obbligatori.
    let genres = f.genres.clone();
    let genres_or = false;
    // Parole chiave: quelle dei mood e dei temi liberi sono argomenti (AND fra
    // loro), la parola chiave propria di una corrente (es. «slasher») e
    // l'identita stessa della corrente, quindi va in OR. Se ci sono entrambe si
    // resta prudenti e si usa l'AND.
    let mut keywords = f
        .moods
        .iter()
        .filter_map(|id| known.iter().find(|(k, _)| k == id).map(|(_, v)| *v))
        .collect::<Vec<u64>>();
    for (name, id) in known {
        if f.keywords.iter().any(|k| k == name) && !keywords.contains(id) {
            keywords.push(*id);
        }
    }
    let mut keyword_ids: Vec<u64> = Vec::new();
    for id in &f.movements {
        if let Some(m) = tax.movement(id) {
            if !m.keyword.is_empty() {
                if let Some((_, v)) = known.iter().find(|(k, _)| k == id) {
                    if !keyword_ids.contains(v) {
                        keyword_ids.push(*v);
                    }
                }
            }
        }
    }
    let mut keywords_or = false;
    if !keyword_ids.is_empty() {
        keywords_or = keywords.is_empty();
        for k in keyword_ids {
            if !keywords.contains(&k) {
                keywords.push(k);
            }
        }
    }

    // I generi esclusi dall'utente («senza commedia») e quelli vietati dai
    // movimenti finiscono nella stessa lista: prima i secondi venivano passati
    // a TMDB da soli, quindi l'esclusione esplicita non arrivava al server.
    let mut without = f.without_genres.clone();
    for g in forbidden_genres(tax, f) {
        if !without.contains(&g) {
            without.push(g);
        }
    }

    Ok(DiscoverQuery {
        include_adult: false,
        include_video: false,
        genres,
        genres_or,
        without_genres: without,
        keywords,
        keywords_or,
        countries: effective_countries(tax, f),
        year,
        date_from: None,
        date_to: None,
        min_rating: f.min_rating,
        max_rating: f.max_rating,
        min_votes: f.min_votes,
        runtime: f.runtime,
        sort: f.sort,
        page,
    })
}

/// Traduzione codice paese -> lingua originale, usata per filtrare i risultati
/// quando la ricerca e per titolo (l'endpoint search non espone i paesi).
pub fn country_language(code: &str) -> Option<&'static str> {
    Some(match code {
        "IT" => "it",
        "FR" => "fr",
        "US" | "GB" => "en",
        "DE" => "de",
        "JP" => "ja",
        "KR" => "ko",
        "HK" => "cn",
        "CN" => "zh",
        "TW" => "zh",
        "BR" => "pt",
        "PT" => "pt",
        "MX" | "AR" | "ES" => "es",
        "RU" => "ru",
        "PL" => "pl",
        "DK" => "da",
        "SE" => "sv",
        "TR" => "tr",
        "IR" => "fa",
        "IN" => "hi",
        "TH" => "th",
        _ => return None,
    })
}
