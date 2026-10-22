//! Prova end-to-end della chat senza aprire la finestra:
//! messaggio -> interprete (memoria breve) -> candidati reali TMDB ->
//! curatela (streaming) -> 5 film con «perche» e consiglio.
//!
//! Uso: cargo run --release --example chat_probe -- "5 slasher anni 80" "e uno piu recente?"
//! Legge la configurazione vera da %APPDATA%\CineMatch\config.json.

use cinematch::ai::{build_provider, curator, IntentProvider};
use cinematch::app::discover_with_relaxation;
use cinematch::domain::constraints;
use cinematch::domain::model::{Facets, Movie, Sort};
use cinematch::domain::taxonomy::Taxonomy;
use cinematch::infra::config::Config;
use cinematch::tmdb::Tmdb;

fn merge(base: &mut Facets, extra: &Facets) {
    for d in &extra.decades {
        if !base.decades.contains(d) {
            base.decades.push(d.clone());
        }
    }
    for m in &extra.movements {
        if !base.movements.contains(m) {
            base.movements.push(m.clone());
        }
    }
    for g in &extra.genres {
        if !base.genres.contains(g) {
            base.genres.push(*g);
        }
    }
    for g in &extra.without_genres {
        if !base.without_genres.contains(g) {
            base.without_genres.push(*g);
        }
    }
    for c in &extra.countries {
        if !base.countries.contains(c) {
            base.countries.push(c.clone());
        }
    }
    for m in &extra.moods {
        if !base.moods.contains(m) {
            base.moods.push(m.clone());
        }
    }
    for k in &extra.keywords {
        if !base.keywords.contains(k) {
            base.keywords.push(k.clone());
        }
    }
    if extra.min_rating.is_some() {
        base.min_rating = extra.min_rating;
    }
    if extra.max_rating.is_some() {
        base.max_rating = extra.max_rating;
    }
    if extra.min_votes.is_some() {
        base.min_votes = extra.min_votes;
    }
    if extra.runtime.is_some() {
        base.runtime = extra.runtime;
    }
    if extra.sort != Sort::Popularity {
        base.sort = extra.sort;
    }
}

fn understood(tax: &Taxonomy, f: &Facets) -> String {
    let mut parts: Vec<String> = Vec::new();
    for m in &f.movements {
        if let Some(mv) = tax.movement(m) {
            parts.push(mv.label.clone());
        }
    }
    if !f.decades.is_empty() {
        parts.push(
            f.decades
                .iter()
                .filter_map(|d| tax.decade(d))
                .map(|d| d.label.clone())
                .collect::<Vec<_>>()
                .join(" + "),
        );
    }
    if !f.genres.is_empty() {
        parts.push(
            f.genres
                .iter()
                .filter_map(|g| tax.genre_name(*g))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    if !f.countries.is_empty() {
        parts.push(
            f.countries
                .iter()
                .map(|c| tax.country_label(c))
                .collect::<Vec<_>>()
                .join(", "),
        );
    }
    if !f.moods.is_empty() {
        let moods = f
            .moods
            .iter()
            .filter_map(|m| tax.mood(m))
            .map(|m| m.label.clone())
            .collect::<Vec<_>>()
            .join(" + ");
        if !moods.is_empty() {
            parts.push(moods);
        }
    }
    if let Some(r) = f.min_rating {
        parts.push(format!("voto >= {r}"));
    }
    if !f.keywords.is_empty() {
        parts.push(format!("temi: {}", f.keywords.join(", ")));
    }
    if parts.is_empty() {
        "ricerca libera".into()
    } else {
        parts.join(" · ")
    }
}

fn candidate_of(tax: &Taxonomy, m: &Movie) -> curator::Candidate {
    curator::Candidate {
        id: m.id,
        title: m.title.clone(),
        year: m.year,
        rating: m.rating,
        votes: m.votes,
        genres: m
            .genre_ids
            .iter()
            .filter_map(|g| tax.genre_name(*g))
            .collect::<Vec<_>>()
            .join(", "),
        overview: m.overview.chars().take(240).collect(),
    }
}

fn dedupe(movies: Vec<Movie>) -> Vec<Movie> {
    let mut seen: Vec<u64> = Vec::new();
    movies
        .into_iter()
        .filter(|m| {
            if seen.contains(&m.id) {
                false
            } else {
                seen.push(m.id);
                true
            }
        })
        .collect()
}

/// Un turno di chat: interpreta, cerca, cura. Ritorna i filtri capiti e i
/// titoli proposti (per la memoria del turno successivo).
fn turn(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    provider: &std::sync::Arc<dyn IntentProvider>,
    text: &str,
    active: &Facets,
    history: &[(String, String)],
) -> Option<(Facets, Vec<String>)> {
    println!("\n=== messaggio: «{text}» ===");
    let intent = match provider.parse_chat(text, tax, history, active) {
        Ok(i) => i,
        Err(e) => {
            println!("interprete non disponibile: {e}");
            return None;
        }
    };
    for n in &intent.notes {
        println!("  nota: {n}");
    }
    println!(
        "  facce: decenni={:?} movimenti={:?} generi={:?} esclusi={:?} paesi={:?} mood={:?} temi={:?} voto={:?} voti={:?} modalita={:?}",
        intent.facets.decades,
        intent.facets.movements,
        intent.facets.genres,
        intent.facets.without_genres,
        intent.facets.countries,
        intent.facets.moods,
        intent.facets.keywords,
        intent.facets.min_rating,
        intent.facets.min_votes,
        intent.mode
    );

    let mut eff = active.clone();
    if provider.is_llm() {
        // Stato completo: il modello parte dai filtri attivi che riceve.
        eff = intent.facets.clone();
    } else {
        merge(&mut eff, &intent.facets);
    }
    let (eff, removed) = constraints::sanitize(tax, &eff);
    for r in &removed {
        println!("  vincolo: {r}");
    }
    let off_topic = intent.mode.eq_ignore_ascii_case("off_topic");
    let descr = if off_topic && eff.is_empty() {
        "film del momento".to_string()
    } else {
        understood(tax, &eff)
    };
    println!("  capito: {descr}");

    // Parole chiave (mood, temi, correnti).
    let mut kw: Vec<(String, u64)> = Vec::new();
    for (id, word) in constraints::unresolved_keywords(tax, &eff, &kw) {
        if let Ok(Some(v)) = tmdb.keyword_id(&word) {
            kw.push((id, v));
        }
    }
    for name in constraints::unresolved_themes(&eff, &kw) {
        if let Ok(Some(v)) = tmdb.keyword_id(&name) {
            kw.push((name, v));
        }
    }
    for (id, word) in constraints::unresolved_movement_keywords(tax, &eff, &kw) {
        if let Ok(Some(v)) = tmdb.keyword_id(&word) {
            kw.push((id, v));
        }
    }
    if !kw.is_empty() {
        println!("  keywords TMDB: {kw:?}");
    }

    let title = intent.free_text.clone();
    let similar = intent.similar_to.clone();
    // Se l'interprete non ha riconosciuto un titolo, non cerco l'intera frase:
    // una domanda o un «sorprendimi» non sono titoli (stessa regola dell'app).

    let mut relaxed: Vec<String> = Vec::new();
    let movies: Vec<Movie> = if let Some(name) = &similar {
        match tmdb.search(name, 1) {
            Ok(p) => match p.results.first() {
                Some(first) => {
                    println!("  simili a «{}»", first.title);
                    match tmdb.similar(first.id, 1) {
                        Ok(rec) => dedupe(rec.results.into_iter().map(Movie::from).collect()),
                        Err(e) => {
                            println!("errore: {e}");
                            return None;
                        }
                    }
                }
                None => {
                    println!("nessun film per «{name}»");
                    return None;
                }
            },
            Err(e) => {
                println!("errore: {e}");
                return None;
            }
        }
    } else if let Some(t) = &title {
        println!("  ricerca per titolo: «{t}»");
        match tmdb.search(t, 1) {
            Ok(p) => dedupe(p.results.into_iter().map(Movie::from).collect()),
            Err(e) => {
                println!("errore: {e}");
                return None;
            }
        }
    } else if eff.is_empty() {
        match tmdb.trending(1) {
            Ok(p) => dedupe(p.results.into_iter().map(Movie::from).collect()),
            Err(e) => {
                println!("errore: {e}");
                return None;
            }
        }
    } else {
        match discover_with_relaxation(tmdb, tax, &eff, &kw, 1, &mut relaxed) {
            Ok((m1, total, pages, headline)) => {
                println!("  {headline} ({total} risultati, {pages} pagine)");
                let mut movies = m1;
                if pages > 1 && movies.len() < 20 {
                    if let Ok((m2, _, _, _)) =
                        discover_with_relaxation(tmdb, tax, &eff, &kw, 2, &mut relaxed)
                    {
                        movies.extend(m2);
                    }
                }
                dedupe(movies)
            }
            Err(e) => {
                println!("errore: {e}");
                return None;
            }
        }
    };
    for r in &relaxed {
        println!("  rilassato: {r}");
    }
    let movies: Vec<Movie> = movies.into_iter().take(20).collect();
    println!("  candidati reali: {}", movies.len());

    let req = curator::Request {
        user_text: text.to_string(),
        understood: descr,
        history: history.to_vec(),
        candidates: movies.iter().map(|m| candidate_of(tax, m)).collect(),
    };
    let mut chunks = 0usize;
    let cur = match provider.curate(&req, &mut |_chunk| {
        chunks += 1;
    }) {
        Ok(c) => c,
        Err(e) => {
            println!("  curatela AI non disponibile ({e}): uso la scelta deterministica");
            curator::deterministic(&req)
        }
    };
    println!("  streaming: {chunks} frammenti");
    println!("\nintro: {}", cur.intro);
    let mut titles: Vec<String> = Vec::new();
    for (i, p) in cur.picks.iter().enumerate() {
        let Some(m) = movies.iter().find(|m| m.id == p.id) else {
            continue;
        };
        titles.push(m.title.clone());
        let year = m.year.map(|y| y.to_string()).unwrap_or_else(|| "?".into());
        println!("\n{}. {} ({year})  id={}", i + 1, m.title, m.id);
        println!("   perche: {}", p.perche);
        println!("   consiglio: {}", p.consiglio);
    }

    // Verifica del percorso «scheda del film scelto»: regia, durata, dove si vede.
    if let Some(first) = cur.picks.first() {
        match tmdb.details(first.id) {
            Ok(d) => println!(
                "\nscheda {}: regia={:?} durata={:?} abbonamento={:?} imdb={:?}",
                d.movie.title,
                d.director,
                d.runtime,
                d.flatrate,
                d.imdb.map(|i| i.rating)
            ),
            Err(e) => println!("\nscheda non disponibile: {e}"),
        }
    }

    println!("\nscelte: {}", titles.len());
    Some((eff, titles))
}

fn main() {
    let phrases: Vec<String> = std::env::args().skip(1).collect();
    let phrases = if phrases.is_empty() {
        vec!["5 slasher anni 80 poco conosciuti".to_string()]
    } else {
        phrases
    };

    let cfg = Config::load();
    let tax = Taxonomy::load();
    let provider = build_provider(&cfg.ai);
    println!("interprete: {}", provider.label());

    let tmdb = match Tmdb::new(cfg.tmdb_api_key.clone(), cfg.language.clone()).ping() {
        Ok(_) => Tmdb::new(cfg.tmdb_api_key.clone(), cfg.language.clone()),
        Err(e) => {
            println!("TMDB non disponibile: {e}");
            return;
        }
    };

    let mut active = Facets::default();
    let mut history: Vec<(String, String)> = Vec::new();
    for phrase in &phrases {
        match turn(&tmdb, &tax, &provider, phrase, &active, &history) {
            Some((eff, titles)) => {
                active = eff;
                history.push(("utente".into(), phrase.clone()));
                if !titles.is_empty() {
                    history.push(("assistente".into(), format!("Ho proposto: {}", titles.join(", "))));
                }
            }
            None => break,
        }
    }
}
