//! Suggerimenti locali della casa: un piccolo catalogo costruito dalla
//! tassonomia, senza rete e senza AI. Ogni proposta porta con se le facce che
//! descrive; prima di mostrarla la uniamo ai filtri attivi e la facciamo
//! passare dal motore di vincoli. Se il motore dovesse correggerla, la
//! scartiamo: in casa non compaiono mai combinazioni impossibili.
//!
//! La rotazione e' pura selezione: una proposta per categoria, poi il ciclo
//! avanza. In questo modo le quattro card cambiano insieme e non ripetono
//! subito lo stesso testo.

use std::collections::BTreeSet;

use crate::domain::constraints;
use crate::domain::model::{Facets, Sort};
use crate::domain::taxonomy::{Movement, Taxonomy};

/// Ogni quanto cambiare le quattro card della casa, in secondi.
pub const ROTATION_SECS: f64 = 7.0;

/// Ripiego locale per «Sorprendimi»: contiene una faccia riconosciuta dal
/// parser a regole, quindi non viene scambiata per un titolo.
pub const SURPRISE_FALLBACK: &str = "Un film ben votato e poco conosciuto";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Corrente,
    Atmosfera,
    Periodo,
    Scopri,
}

/// Una card della casa. `tag` e `desc` sono solo presentazione; `facets`
/// descrive le facce che la richiesta mette in gioco e serve alla validazione.
#[derive(Clone, Debug)]
pub struct Suggestion {
    pub tag: String,
    pub prompt: String,
    pub desc: String,
    kind: Kind,
    /// Facce usate per la validazione locale; esposte per i test e per i
    /// diagnostici, non servono alla pittura della card.
    pub facets: Facets,
}

impl Suggestion {
    fn new(tag: &str, prompt: String, desc: String, kind: Kind, facets: Facets) -> Self {
        Self {
            tag: tag.to_string(),
            prompt,
            desc,
            kind,
            facets,
        }
    }

    /// Etichetta breve della categoria, usata come kicker nella casa.
    pub fn kind_label(&self) -> &'static str {
        match self.kind {
            Kind::Corrente => "CORRENTE",
            Kind::Atmosfera => "ATMOSFERA",
            Kind::Periodo => "PERIODO",
            Kind::Scopri => "SERATA",
        }
    }
}

/// Costruisce il catalogo valido per lo stato dei filtri della griglia.
/// Nessuna chiamata di rete: tassonomia, anatomia delle facce e motore di
/// vincoli sono gia' in memoria.
pub fn candidates(tax: &Taxonomy, base: &Facets) -> Vec<Suggestion> {
    let mut out = Vec::new();
    let mut seen = BTreeSet::new();
    add_movements(&mut out, &mut seen, tax, base);
    add_moods(&mut out, &mut seen, tax, base);
    add_periods(&mut out, &mut seen, tax, base);
    add_discoveries(&mut out, &mut seen, tax, base);
    out
}

/// Sceglie `count` proposte da mostrare. Prova a tenere una card per
/// categoria, poi riempie con le altre. `avoid` contiene i testi della
/// rotazione precedente: evita di riproporli subito.
pub fn pick(pool: &[Suggestion], cycle: u64, avoid: &[String], count: usize) -> Vec<Suggestion> {
    if count == 0 || pool.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut taken: BTreeSet<String> = BTreeSet::new();
    let order = [
        Kind::Corrente,
        Kind::Corrente,
        Kind::Periodo,
        Kind::Scopri,
    ];

    for kind in order {
        if out.len() >= count {
            break;
        }
        let list: Vec<&Suggestion> = pool.iter().filter(|s| s.kind == kind).collect();
        if list.is_empty() {
            continue;
        }
        let start = cycle as usize % list.len();
        for step in 0..list.len() {
            let candidate = list[(start + step) % list.len()];
            if avoid.iter().any(|p| p == &candidate.prompt)
                || taken.contains(&candidate.prompt)
            {
                continue;
            }
            taken.insert(candidate.prompt.clone());
            out.push(candidate.clone());
            break;
        }
    }

    if out.len() < count {
        let start = (cycle as usize * 3) % pool.len();
        for step in 0..pool.len() {
            if out.len() >= count {
                break;
            }
            let candidate = &pool[(start + step) % pool.len()];
            if avoid.iter().any(|p| p == &candidate.prompt)
                || taken.contains(&candidate.prompt)
            {
                continue;
            }
            taken.insert(candidate.prompt.clone());
            out.push(candidate.clone());
        }
    }

    // Caso limite: i filtri lasciano meno di quattro testi nuovi. Meglio una
    // card ripetuta che una casa con meno di quattro spunti.
    if out.len() < count {
        let start = cycle as usize % pool.len();
        let mut step = 0usize;
        while out.len() < count {
            out.push(pool[(start + step) % pool.len()].clone());
            step += 1;
        }
    }
    out
}

/// Una proposta non visibile, per «Sorprendimi». Resta locale come le card.
pub fn surprise(pool: &[Suggestion], visible: &[String], cycle: u64) -> String {
    let fresh: Vec<&Suggestion> = pool
        .iter()
        .filter(|s| s.kind == Kind::Scopri && !visible.iter().any(|p| p == &s.prompt))
        .collect();
    if let Some(s) = fresh.get(cycle as usize % fresh.len().max(1)) {
        return s.prompt.clone();
    }
    let any: Vec<&Suggestion> = pool
        .iter()
        .filter(|s| !visible.iter().any(|p| p == &s.prompt))
        .collect();
    any.first()
        .map(|s| s.prompt.clone())
        .unwrap_or_else(|| SURPRISE_FALLBACK.to_string())
}

// ----------------------------------------------------------- categorie -----

fn add_movements(
    out: &mut Vec<Suggestion>,
    seen: &mut BTreeSet<String>,
    tax: &Taxonomy,
    base: &Facets,
) {
    // Le prime correnti della casa: un inizio editoriale, non l'ordine grezzo
    // del file di tassonomia. Il resto segue per non perdere il catalogo.
    const PRIORITY: [&str; 6] = [
        "neorealismo",
        "new_hollywood",
        "nouvelle_vague",
        "giallo_italiano",
        "hk_action",
        "italian_political",
    ];
    let mut order: Vec<&Movement> = Vec::new();
    for id in PRIORITY {
        if let Some(m) = tax.movement(id) {
            order.push(m);
        }
    }
    for m in &tax.movements {
        if !PRIORITY.contains(&m.id.as_str()) {
            order.push(m);
        }
    }
    for m in order {
        let prompt = format!("{}: un film da recuperare", m.label);
        let desc = if m.note.trim().is_empty() {
            "Una corrente da attraversare, un film alla volta.".to_string()
        } else {
            m.note.trim().to_string()
        };
        let mut extra = Facets::default();
        extra.movements.push(m.id.clone());
        // Il testo contiene l'etichetta della corrente: il parser a regole
        // ricava anche le facce implicite nel nome. Le aggiungiamo qui, cosi'
        // la validazione vede esattamente cio' che l'utente leggera'.
        match m.id.as_str() {
            "giallo_italiano" => {
                extra.genres.push(9648);
                extra.countries.push("IT".into());
            }
            "slasher_era" => {
                extra.genres.push(27);
                extra.decades.push("1980s".into());
            }
            "j_horror" => extra.genres.push(27),
            "espressionismo" | "german_new_cinema" => extra.countries.push("DE".into()),
            "italian_political" => extra.countries.push("IT".into()),
            "soviet_montage" => extra.countries.push("RU".into()),
            _ => {}
        }
        push(
            out,
            seen,
            tax,
            base,
            Kind::Corrente,
            "corrente",
            prompt,
            desc,
            extra,
        );
    }
}

const MOOD_COPY: [(&str, &str, &str); 8] = [
    (
        "distopia",
        "Un film di distopia, ma con un'idea nuova",
        "Mondi che promettono ordine e consegnano gabbie.",
    ),
    (
        "on_the_road",
        "Un film on the road, ma senza benzina",
        "Strade lunghe, fughe e incontri che cambiano.",
    ),
    (
        "vendetta",
        "Un film di vendetta, ma senza vendetta",
        "Conti in sospeso e conseguenze che pesano.",
    ),
    (
        "slow_burn",
        "Un film slow burn, ma senza annoiarsi",
        "Tensione che sale piano, senza scorciatoie.",
    ),
    (
        "critica_sociale",
        "Un film di critica sociale, ma senza prediche",
        "Lo sguardo politico dentro la storia.",
    ),
    (
        "claustrofobia",
        "Un film claustrofobico? Meglio: claustrofobia con una via d'uscita",
        "Spazi stretti e nessuna via d'uscita.",
    ),
    (
        "horror_psicologico",
        "Un horror psicologico, non solo jumpscare",
        "Paura che nasce dalla testa, non dal buio.",
    ),
    (
        "soprannaturale",
        "Un film soprannaturale, ma con i piedi per terra",
        "Qualcosa che non dovrebbe esserci.",
    ),
];

fn add_moods(
    out: &mut Vec<Suggestion>,
    seen: &mut BTreeSet<String>,
    tax: &Taxonomy,
    base: &Facets,
) {
    for (id, prompt, desc) in MOOD_COPY {
        if tax.mood(id).is_none() {
            continue;
        }
        let mut extra = Facets::default();
        extra.moods.push(id.to_string());
        if id == "horror_psicologico" {
            // «Horror» nel testo e' anche il genere TMDB 27.
            extra.genres.push(27);
        }
        push(
            out,
            seen,
            tax,
            base,
            Kind::Atmosfera,
            "atmosfera",
            prompt.to_string(),
            desc.to_string(),
            extra,
        );
    }
}

const PERIODS: [(&str, u32, &str, &str); 11] = [
    (
        "1920s",
        27,
        "Un horror anni '20",
        "Luci espressioniste e ombre disegnate: il buio diventa linguaggio.",
    ),
    (
        "1930s",
        12,
        "Un film d'avventura anni '30",
        "Avventura di cartapesta, ma con un ritmo che tiene ancora.",
    ),
    (
        "1940s",
        80,
        "Un noir anni '40",
        "Fumo, cappelli a tesa larga e morale ambigua.",
    ),
    (
        "1950s",
        37,
        "Un western anni '50",
        "Frontiera, silenzi e paesaggi che sembrano dipinti.",
    ),
    (
        "1960s",
        53,
        "Un thriller anni '60",
        "La tensione sale, il colore esplode, il finale non perdona.",
    ),
    (
        "1970s",
        27,
        "Un horror anni '70",
        "Il decennio in cui l'horror ha smesso di chiedere permesso.",
    ),
    (
        "1980s",
        35,
        "Una commedia anni '80",
        "Risate, luci al neon e un'idea di cinema che non invecchia.",
    ),
    (
        "1990s",
        28,
        "Un film d'azione anni '90",
        "Azione fisica, montaggio veloce e qualche eccesso bellissimo.",
    ),
    (
        "2000s",
        878,
        "Un film di fantascienza anni 2000",
        "Fantascienza che immagina il futuro con gli occhi di oggi.",
    ),
    (
        "2010s",
        18,
        "Un dramma anni 2010",
        "Drammi intimi e una nuova generazione di autori.",
    ),
    (
        "2020s",
        16,
        "Un film di animazione anni 2020",
        "Animazione contemporanea, tra memoria e mondo che cambia.",
    ),
];

fn add_periods(
    out: &mut Vec<Suggestion>,
    seen: &mut BTreeSet<String>,
    tax: &Taxonomy,
    base: &Facets,
) {
    let mut extra = Facets::default();
    extra.decades.push("1970s".into());
    extra.countries.push("IT".into());
    push(
        out,
        seen,
        tax,
        base,
        Kind::Periodo,
        "periodo",
        "Un film italiano degli anni '70".into(),
        "Poliziottesco, commedia amara e cinema civile da riscoprire.".into(),
        extra,
    );
    for (decade, genre, prompt, desc) in PERIODS {
        if tax.decade(decade).is_none() || tax.genre_name(genre).is_none() {
            continue;
        }
        let mut extra = Facets::default();
        extra.decades.push(decade.to_string());
        extra.genres.push(genre);
        push(
            out,
            seen,
            tax,
            base,
            Kind::Periodo,
            "periodo",
            prompt.to_string(),
            desc.to_string(),
            extra,
        );
    }
}

fn add_discoveries(
    out: &mut Vec<Suggestion>,
    seen: &mut BTreeSet<String>,
    tax: &Taxonomy,
    base: &Facets,
) {
    let mut extra = Facets::default();
    extra.runtime = Some((0, 100));
    extra.min_rating = Some(7.0);
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "serata",
        "Un film sotto i 100 minuti con voto almeno 7".into(),
        "Nessun minuto di troppo.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.min_votes = Some(1000);
    extra.sort = Sort::Rating;
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "gemme",
        "Un film ben votato e poco conosciuto".into(),
        "Voto alto e pubblico ristretto: il tipo da consigliare sottovoce.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.min_rating = Some(7.0);
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "voto",
        "Un film con voto alto".into(),
        "Almeno un 7 di media, senza scendere a compromessi.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.min_rating = Some(8.0);
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "capolavoro",
        "Un film da capolavoro".into(),
        "Otto pieno: il genere di titolo che resta dopo i titoli di coda.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.min_rating = Some(7.0);
    extra.max_rating = Some(8.0);
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "voto",
        "Un film con voto tra 7 e 8".into(),
        "Ben fatto ma non intoccabile: la zona in cui si trovano le sorprese.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.runtime = Some((0, 100));
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "breve",
        "Un film sotto i 100 minuti".into(),
        "Serata piena, tempo stretto: nessun minuto di troppo.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.runtime = Some((0, 90));
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "breve",
        "Un film sotto i 90 minuti".into(),
        "Un'ora e mezza scarsa, per entrare e uscire senza fretta.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.min_rating = Some(7.0);
    extra.sort = Sort::Newest;
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "recente",
        "Un film recente con voto alto".into(),
        "Il catalogo di oggi, visto con un filtro di qualita.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.min_votes = Some(1000);
    extra.sort = Sort::Newest;
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "recente",
        "I film più recenti e ben votati".into(),
        "Partiamo dall'ultimo arrivato, con un pubblico alle spalle.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.min_votes = Some(5000);
    extra.sort = Sort::Rating;
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "voto",
        "I film più votati, con almeno 5000 voti".into(),
        "La media dei voti come bussola, senza perdere il resto.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.countries.push("IT".into());
    extra.min_votes = Some(1000);
    extra.sort = Sort::Rating;
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "italia",
        "Un film italiano ben votato".into(),
        "Cinema italiano con una media solida e un pubblico vero.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.countries.push("JP".into());
    extra.runtime = Some((0, 120));
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "nippon",
        "Un film giapponese sotto i 120 minuti".into(),
        "Due ore al massimo per un altro modo di raccontare.".into(),
        extra,
    );

    for (votes, prompt) in [
        (5000, "Un film con almeno 5000 voti"),
        (10000, "Un film con almeno 10000 voti"),
        (20000, "Un film con almeno 20000 voti"),
        (50000, "Un film con almeno 50000 voti"),
        (100000, "Un film con almeno 100000 voti"),
    ] {
        let mut extra = Facets::default();
        extra.min_votes = Some(votes);
        push(
            out,
            seen,
            tax,
            base,
            Kind::Scopri,
            "voti",
            prompt.into(),
            "Un pubblico ampio: meno rischio di titoli fantasma.".into(),
            extra,
        );
    }

    let mut extra = Facets::default();
    extra.min_votes = Some(1000);
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "voti",
        "Un film con molti voti".into(),
        "La comunita lo conosce: un punto di partenza sicuro.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.genres.push(35);
    extra.min_votes = Some(1000);
    extra.sort = Sort::Rating;
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "commedia",
        "Una commedia ben votata".into(),
        "Ridere bene, senza accontentarsi della prima cosa che passa.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.genres.push(28);
    extra.runtime = Some((0, 110));
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "azione",
        "Un film d'azione sotto i 110 minuti".into(),
        "Adrenalina concentrata, senza tempi morti.".into(),
        extra,
    );

    let mut extra = Facets::default();
    extra.genres.push(878);
    extra.min_rating = Some(7.0);
    push(
        out,
        seen,
        tax,
        base,
        Kind::Scopri,
        "fantascienza",
        "Un film di fantascienza con voto alto".into(),
        "Idee grandi e una media che le tiene in piedi.".into(),
        extra,
    );
}

// ----------------------------------------------------------- validazione -----

fn push(
    out: &mut Vec<Suggestion>,
    seen: &mut BTreeSet<String>,
    tax: &Taxonomy,
    base: &Facets,
    kind: Kind,
    tag: &str,
    prompt: String,
    desc: String,
    facets: Facets,
) {
    if !valid_with_base(tax, base, &facets) {
        return;
    }
    if !seen.insert(prompt.clone()) {
        return;
    }
    out.push(Suggestion::new(tag, prompt, desc, kind, facets));
}

/// Unisce le facce della proposta a quelle gia' attive. I vincoli metrici si
/// intersecano: la proposta non puo' allargare un limite gia' scelto.
fn merged(base: &Facets, extra: &Facets) -> Option<Facets> {
    let mut f = base.clone();
    append_unique(&mut f.decades, &extra.decades);
    append_unique(&mut f.movements, &extra.movements);
    append_unique(&mut f.genres, &extra.genres);
    append_unique(&mut f.without_genres, &extra.without_genres);
    append_unique(&mut f.countries, &extra.countries);
    append_unique(&mut f.moods, &extra.moods);
    append_unique(&mut f.keywords, &extra.keywords);

    f.min_rating = max_opt(base.min_rating, extra.min_rating);
    f.max_rating = min_opt(base.max_rating, extra.max_rating);
    f.min_votes = max_opt(base.min_votes, extra.min_votes);
    f.runtime = match (base.runtime, extra.runtime) {
        (Some(a), Some(b)) => {
            let lo = a.0.max(b.0);
            let hi = a.1.min(b.1);
            if lo > hi {
                return None;
            }
            Some((lo, hi))
        }
        (Some(r), None) | (None, Some(r)) => Some(r),
        (None, None) => None,
    };
    if let (Some(a), Some(b)) = (f.min_rating, f.max_rating) {
        if a > b {
            return None;
        }
    }
    if extra.sort != Sort::default() {
        f.sort = extra.sort;
    }
    Some(f)
}

fn append_unique<T: Clone + PartialEq>(base: &mut Vec<T>, extra: &[T]) {
    for item in extra {
        if !base.contains(item) {
            base.push(item.clone());
        }
    }
}

fn max_opt<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if a >= b { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

fn min_opt<T: PartialOrd>(a: Option<T>, b: Option<T>) -> Option<T> {
    match (a, b) {
        (Some(a), Some(b)) => Some(if a <= b { a } else { b }),
        (Some(a), None) => Some(a),
        (None, Some(b)) => Some(b),
        (None, None) => None,
    }
}

/// La proposta e' valida solo se il motore non deve correggere nulla e le
/// facce scelte restano selezionabili. Cosi' una card non promette mai una
/// combinazione che la griglia stessa rifiuterebbe.
fn valid_with_base(tax: &Taxonomy, base: &Facets, extra: &Facets) -> bool {
    let Some(merged) = merged(base, extra) else {
        return false;
    };
    let (clean, notes) = constraints::sanitize(tax, &merged);
    if !notes.is_empty() || clean != merged {
        return false;
    }
    if constraints::year_range(tax, &clean).is_err() {
        return false;
    }
    let prune = constraints::pruning(tax, &clean);
    for genre in &clean.genres {
        if !prune
            .genres
            .get(genre)
            .map(|a| a.allowed)
            .unwrap_or(false)
        {
            return false;
        }
    }
    for country in &clean.countries {
        if !prune
            .countries
            .get(country)
            .map(|a| a.allowed)
            .unwrap_or(false)
        {
            return false;
        }
    }
    for mood in &clean.moods {
        if !prune.moods.get(mood).map(|a| a.allowed).unwrap_or(false) {
            return false;
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use super::*;
    use crate::ai::{IntentProvider, RuleParser};

    fn tax() -> Taxonomy {
        Taxonomy::load()
    }

    fn prompts(list: &[Suggestion]) -> BTreeSet<String> {
        list.iter().map(|s| s.prompt.clone()).collect()
    }

    #[test]
    fn il_catalogo_e_ricco_e_ogni_proposta_e_valida() {
        let tax = tax();
        let base = Facets::default();
        let pool = candidates(&tax, &base);
        assert!(pool.len() >= 8, "servono abbastanza card per ruotare");
        for s in &pool {
            assert!(!s.tag.is_empty());
            assert!(!s.prompt.is_empty());
            assert!(!s.desc.is_empty());
            assert!(
                valid_with_base(&tax, &base, &s.facets),
                "proposta non valida: {}",
                s.prompt
            );
        }
        let mut kinds: BTreeSet<&'static str> = BTreeSet::new();
        for s in &pool {
            kinds.insert(match s.kind {
                Kind::Corrente => "corrente",
                Kind::Atmosfera => "atmosfera",
                Kind::Periodo => "periodo",
                Kind::Scopri => "scopri",
            });
        }
        assert_eq!(kinds.len(), 4, "il catalogo deve avere tutte le categorie");
    }

    #[test]
    fn la_rotazione_non_ripete_le_card_immediate() {
        let tax = tax();
        let base = Facets::default();
        let pool = candidates(&tax, &base);
        let mut current = pick(&pool, 0, &[], 4);
        assert_eq!(current.len(), 4);
        for cycle in 1..12 {
            let avoid = current.iter().map(|s| s.prompt.clone()).collect::<Vec<_>>();
            let next = pick(&pool, cycle, &avoid, 4);
            assert_eq!(next.len(), 4);
            let next_prompts = prompts(&next);
            assert!(
                avoid.iter().all(|p| !next_prompts.contains(p)),
                "la rotazione {cycle} ripete una card precedente"
            );
            current = next;
        }
    }

    #[test]
    fn i_filtri_attivi_non_vengono_mai_corretti() {
        let tax = tax();
        let base = Facets {
            decades: vec!["1940s".into()],
            movements: vec!["neorealismo".into()],
            countries: vec!["IT".into()],
            min_rating: Some(7.0),
            ..Default::default()
        };
        // Anche con una base stretta il catalogo ha i ripieghi metrici.
        let pool = candidates(&tax, &base);
        assert!(pool.len() >= 4);
        for s in &pool {
            assert!(
                valid_with_base(&tax, &base, &s.facets),
                "proposta incompatibile con i filtri: {}",
                s.prompt
            );
        }
        let four = pick(&pool, 0, &[], 4);
        assert_eq!(four.len(), 4);
        for s in &four {
            assert!(valid_with_base(&tax, &base, &s.facets));
        }
    }

    #[test]
    fn ogni_testo_e_riconoscibile_dal_parser_a_regole() {
        let tax = tax();
        let base = Facets::default();
        let pool = candidates(&tax, &base);
        let parser = RuleParser;
        for s in &pool {
            let intent = parser
                .parse(&s.prompt, &tax)
                .expect("il parser locale non fallisce");
            assert!(
                !intent.facets.is_empty() || intent.facets.sort != Sort::default(),
                "il testo non contiene facce riconoscibili: {}",
                s.prompt
            );
            assert!(
                intent.free_text.is_none(),
                "il testo verrebbe scambiato per un titolo: {}",
                s.prompt
            );
        }
    }

    #[test]
    fn sorprendimi_resta_locale_e_non_ripete_una_card_visibile() {
        let tax = tax();
        let base = Facets::default();
        let pool = candidates(&tax, &base);
        let visible = pick(&pool, 0, &[], 4);
        let visible_prompts = visible.iter().map(|s| s.prompt.clone()).collect::<Vec<_>>();
        let surprise = surprise(&pool, &visible_prompts, 2);
        assert!(!surprise.is_empty());
        assert!(!visible_prompts.contains(&surprise));
        let parser = RuleParser;
        let intent = parser.parse(&surprise, &tax).unwrap();
        assert!(!intent.facets.is_empty() || intent.facets.sort != Sort::default());
    }

    #[test]
    fn la_casa_mostra_quattro_card_anche_con_poche_proposte() {
        let pool = vec![
            Suggestion::new(
                "Corrente",
                "Neorealismo: un film da recuperare".to_string(),
                "Una corrente.".to_string(),
                Kind::Corrente,
                Facets::default(),
            ),
            Suggestion::new(
                "Periodo",
                "Anni '40: un film da riscoprire".to_string(),
                "Un periodo.".to_string(),
                Kind::Periodo,
                Facets::default(),
            ),
        ];
        let picked = pick(&pool, 0, &[], 4);
        assert_eq!(picked.len(), 4, "la casa resta a quattro spunti");
    }
}
