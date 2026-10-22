//! Riproduce l'intera pipeline dell'app senza aprire la finestra:
//! frase -> interprete configurato (AI o regole) -> motore di vincoli -> TMDB.
//!
//! Uso: cargo run --example app_probe -- "dogme 95 + fantasy"
//! Legge la configurazione vera da %APPDATA%\CineMatch\config.json.

use cinematch::ai::build_provider;
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

fn main() {
    let phrase = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "giallo italiano anni 80, voto alto".to_string());

    let cfg = Config::load();
    let tax = Taxonomy::load();
    let provider = build_provider(&cfg.ai);

    println!("interprete: {}", provider.label());
    println!("frase: «{phrase}»");

    let intent = match provider.parse(&phrase, &tax) {
        Ok(i) => i,
        Err(e) => {
            println!("interprete non disponibile: {e}");
            return;
        }
    };
    for n in &intent.notes {
        println!("  nota: {n}");
    }
    println!("  facce: decenni={:?} movimenti={:?} generi={:?} esclusi={:?} paesi={:?} mood={:?} temi={:?} voto={:?} voto_max={:?} voti={:?} durata={:?} ordine={:?}",
        intent.facets.decades, intent.facets.movements, intent.facets.genres,
        intent.facets.without_genres, intent.facets.countries, intent.facets.moods,
        intent.facets.keywords, intent.facets.min_rating, intent.facets.max_rating,
        intent.facets.min_votes, intent.facets.runtime, intent.facets.sort);

    let mut eff = Facets::default();
    merge(&mut eff, &intent.facets);
    let (clean, removed) = constraints::sanitize(&tax, &eff);
    for r in &removed {
        println!("  vincolo: {r}");
    }

    let similar = intent.similar_to.clone();
    if let Some(name) = similar {
        println!("  richiesta di film simili a «{name}»");
        return;
    }

    let tmdb = match Tmdb::new(cfg.tmdb_api_key.clone(), cfg.language.clone()).ping() {
        Ok(_) => Tmdb::new(cfg.tmdb_api_key.clone(), cfg.language.clone()),
        Err(e) => {
            println!("TMDB non disponibile: {e}");
            return;
        }
    };

    let mut kw: Vec<(String, u64)> = Vec::new();
    for (id, word) in constraints::unresolved_keywords(&tax, &clean, &kw) {
        if let Ok(Some(v)) = tmdb.keyword_id(&word) {
            println!("  mood «{id}» -> keyword TMDB {v} ({word})");
            kw.push((id, v));
        }
    }
    for name in constraints::unresolved_themes(&clean, &kw) {
        match tmdb.keyword_id(&name) {
            Ok(Some(v)) => {
                println!("  tema «{name}» -> keyword TMDB {v}");
                kw.push((name, v));
            }
            Ok(None) => println!("  tema «{name}»: nessuna parola chiave su TMDB"),
            Err(e) => println!("  tema «{name}»: {e}"),
        }
    }
    for (id, name) in constraints::unresolved_movement_keywords(&tax, &clean, &kw) {
        match tmdb.keyword_id(&name) {
            Ok(Some(v)) => {
                println!("  corrente «{id}» -> keyword TMDB {v} ({name})");
                kw.push((id, v));
            }
            Ok(None) => println!("  corrente «{id}»: parola chiave «{name}» non trovata su TMDB"),
            Err(e) => println!("  corrente «{id}»: {e}"),
        }
    }

    let mut relaxed: Vec<String> = Vec::new();
    if let Ok(q) = constraints::to_discover(&tax, &clean, &kw, 1) {
        println!(
            "  query: generi={:?} (or={}) esclusi={:?} keyword={:?} paesi={:?} anni={:?} durata={:?} voto={:?} voto_max={:?} voti={:?}",
            q.genres, q.genres_or, q.without_genres, q.keywords, q.countries, q.year,
            q.runtime, q.min_rating, q.max_rating, q.min_votes
        );
    }

    match discover_with_relaxation(&tmdb, &tax, &clean, &kw, 1, &mut relaxed) {
        Ok((movies, total, _pages, headline)) => {
            println!("titolo: {headline}");
            for r in &relaxed {
                println!("  rilassato: {r}");
            }
            println!("{total} risultati");
            for m in movies.iter().take(8) {
                let year = m.year.map(|y| y.to_string()).unwrap_or_else(|| "?".into());
                let _: &Movie = m;
                println!("  {:.1} ★  {}  ({year})", m.rating, m.title);
            }
        }
        Err(e) => println!("errore: {e}"),
    }
}
