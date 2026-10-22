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
        Self { allowed: true, reason: None }
    }
    pub fn soft(reason: String) -> Self {
        Self { allowed: true, reason: Some(reason) }
    }
    pub fn blocked(reason: String) -> Self {
        Self { allowed: false, reason: Some(reason) }
    }
}

impl Default for Availability {
    fn default() -> Self {
        Availability::ok()
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
            Some(cur) => cur.intersection(&s).cloned().collect(),
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
pub fn forbidden_genres(tax: &Taxonomy, f: &Facets) -> Vec<u32> {
    let mut set = BTreeSet::new();
    for id in &f.movements {
        if let Some(m) = tax.movement(id) {
            for g in &m.genres_forbidden {
                set.insert(*g);
            }
        }
    }
    set.into_iter().collect()
}

/// Calcola, per ogni faccia, se e ancora selezionabile e con quale motivazione.
pub fn pruning(tax: &Taxonomy, f: &Facets) -> Prune {
    let mut p = Prune::default();

    let active: Vec<&Movement> = f.movements.iter().filter_map(|id| tax.movement(id)).collect();

    for g in &tax.genres {
        let mut av = Availability::ok();
        for m in &active {
            if m.genres_forbidden.contains(&g.id) {
                av = Availability::blocked(format!(
                    "«{}» non appartiene a «{}»",
                    g.name, m.label
                ));
                break;
            }
            if !m.genres_core.contains(&g.id) && !m.genres_allowed.contains(&g.id) {
                av = Availability::soft(format!(
                    "«{}» non e centrale per «{}»: potrebbero uscire pochi risultati",
                    g.name, m.label
                ));
            }
        }
        p.genres.insert(g.id, av);
    }

    let allowed_countries = movement_countries(tax, &f.movements);
    for c in &tax.countries {
        let av = match &allowed_countries {
            Some(set) if !set.contains(&c.code) => Availability::blocked(format!(
                "I movimenti scelti sono legati a {}",
                set.iter()
                    .map(|x| tax.country_label(x))
                    .collect::<Vec<_>>()
                    .join(", ")
            )),
            _ => Availability::ok(),
        };
        p.countries.insert(c.code.clone(), av);
    }

    for mo in &tax.moods {
        let mut av = Availability::ok();
        for m in &active {
            if mo.incompatible_movements.contains(&m.id) {
                av = Availability::blocked(format!(
                    "«{}» e incompatibile con «{}»",
                    mo.label, m.label
                ));
                break;
            }
        }
        p.moods.insert(mo.id.clone(), av);
    }

    p
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

    let span = decade_span(tax, &f);
    let mut kept: Vec<String> = Vec::new();
    let mut kept_span = span;
    let mut kept_countries: Option<BTreeSet<String>> = None;

    for id in &f.movements {
        let Some(m) = tax.movement(id) else {
            msgs.push(format!("Movimento «{}» non riconosciuto: rimosso.", id));
            continue;
        };
        let next_span = if let Some((a, b)) = kept_span {
            let lo = a.max(m.period[0]);
            let hi = b.min(m.period[1]);
            if lo > hi {
                msgs.push(format!(
                    "Rimosso «{}»: copre {}–{}, fuori dall'intervallo {}–{}.",
                    m.label, m.period[0], m.period[1], a, b
                ));
                continue;
            }
            Some((lo, hi))
        } else {
            Some((m.period[0], m.period[1]))
        };
        let s: BTreeSet<String> = m.countries.iter().cloned().collect();
        let next_countries = match &kept_countries {
            None => Some(s),
            Some(cur) => {
                let inter: BTreeSet<String> = cur.intersection(&s).cloned().collect();
                if inter.is_empty() {
                    let names = kept
                        .iter()
                        .filter_map(|k| tax.movement(k))
                        .map(|x| x.label.clone())
                        .collect::<Vec<_>>()
                        .join(", ");
                    msgs.push(format!(
                        "Rimosso «{}»: nessun paese in comune con {}.",
                        m.label,
                        if names.is_empty() { "i movimenti scelti".to_string() } else { names }
                    ));
                    continue;
                }
                Some(inter)
            }
        };
        kept_span = next_span;
        kept_countries = next_countries;
        kept.push(id.clone());
    }
    if kept.len() != f.movements.len() {
        f.movements = kept;
    }

    if !f.movements.is_empty() {
        let forbidden = forbidden_genres(tax, &f);
        f.genres.retain(|g| {
            let ok = !forbidden.contains(g);
            if !ok {
                msgs.push(format!(
                    "Rimosso il genere «{}»: estraneo ai movimenti scelti.",
                    tax.genre_name(*g).unwrap_or_default()
                ));
            }
            ok
        });

        if !f.countries.is_empty() {
            if let Some(set) = movement_countries(tax, &f.movements) {
                f.countries.retain(|c| {
                    let ok = set.contains(c);
                    if !ok {
                        msgs.push(format!(
                            "Rimosso il paese «{}»: i movimenti scelti sono legati a {}.",
                            tax.country_label(c),
                            set.iter()
                                .map(|x| tax.country_label(x))
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    ok
                });
            }
        }

        let active: Vec<&Movement> = f.movements.iter().filter_map(|id| tax.movement(id)).collect();
        f.moods.retain(|mid| {
            let bad = match tax.mood(mid) {
                None => true,
                Some(mo) => active.iter().any(|m| mo.incompatible_movements.contains(&m.id)),
            };
            if bad {
                let label = tax
                    .mood(mid)
                    .map(|x| x.label.clone())
                    .unwrap_or_else(|| mid.clone());
                msgs.push(format!(
                    "Rimosso il mood «{}»: incompatibile con i movimenti scelti.",
                    label
                ));
            }
            !bad
        });
    }

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
    if let Some(v) = f.min_votes {
        if v > 1_000_000 {
            msgs.push("Numero minimo di voti troppo alto: l'ho ridotto.".into());
            f.min_votes = Some(1_000_000);
        }
    }
    // temi liberi: brevi, senza doppioni, al massimo tre (ognuno e un
    // requisito in AND sulla ricerca).
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
    if f.keywords.len() > 3 {
        for k in f.keywords.drain(3..) {
            msgs.push(format!("Tema «{k}» rimosso: con piu di tre temi non esce nulla."));
        }
    }
    if let Some((a, b)) = f.runtime {
        if a > b {
            f.runtime = Some((b, a));
        }
    }

    (f, msgs)
}

/// Mood selezionati di cui non conosciamo ancora l'id keyword TMDB.
pub fn unresolved_keywords(tax: &Taxonomy, f: &Facets, known: &[(String, u64)]) -> Vec<(String, String)> {
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
    pub genres: Vec<u32>,
    /// Con true i generi vengono passati con l'operatore OR (|) invece di AND (,).
    pub genres_or: bool,
    pub without_genres: Vec<u32>,
    pub keywords: Vec<u64>,
    /// Con true le parole chiave vengono passate in OR (|) invece di AND (,).
    pub keywords_or: bool,
    pub countries: Vec<String>,
    pub year: Option<(u16, u16)>,
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

    // Un movimento si traduce su TMDB in piu etichette alternative (il giallo e
    // Horror *oppure* Thriller *oppure* Mistero, non tutti insieme). Se il genere
    // chiesto e gia una di quelle etichette, non va imposto in AND: era l'indizio
    // che ha portato al movimento. In quel caso si usa l'unione in OR; un genere
    // estraneo al movimento resta invece un filtro AND, perche aggiunge davvero
    // una condizione.
    let mut genres = f.genres.clone();
    let mut genres_or = false;
    let mut core: Vec<u32> = Vec::new();
    for id in &f.movements {
        if let Some(m) = tax.movement(id) {
            for g in &m.genres_core {
                if !core.contains(g) {
                    core.push(*g);
                }
            }
        }
    }
    if !core.is_empty() && !f.genres.is_empty() && f.genres.iter().all(|g| core.contains(g)) {
        for g in &core {
            if !genres.contains(g) {
                genres.push(*g);
            }
        }
        genres_or = true;
    }

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

    Ok(DiscoverQuery {
        genres,
        genres_or,
        without_genres: forbidden_genres(tax, f),
        keywords,
        keywords_or,
        countries: effective_countries(tax, f),
        year,
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
