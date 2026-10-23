//! Prova end-to-end della chat senza aprire la finestra:
//! messaggio -> interprete (contesto della chat) -> candidati reali TMDB ->
//! curatela (streaming) -> film pertinenti con «perche» e consiglio.
//!
//! Uso: cargo run --release --example chat_probe -- "5 slasher anni 80" "e uno piu recente?"
//! Legge la configurazione vera da %APPDATA%\CineMatch\config.json.

use std::collections::HashMap;

use cinematch::ai::{
    build_provider, curator, is_follow_up, Intent, IntentProvider, Theme,
};
use cinematch::app::chat_pool;
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

fn merge_themes(base: &mut Vec<Theme>, extra: &[Theme]) {
    for theme in extra {
        if let Some(existing) = base
            .iter_mut()
            .find(|t| t.label.eq_ignore_ascii_case(&theme.label))
        {
            for concept in &theme.concepts {
                if !existing
                    .concepts
                    .iter()
                    .any(|c| c.label.eq_ignore_ascii_case(&concept.label))
                {
                    existing.concepts.push(concept.clone());
                }
            }
            for title in &theme.seed_titles {
                if !existing.seed_titles.contains(title) {
                    existing.seed_titles.push(title.clone());
                }
            }
        } else {
            base.push(theme.clone());
        }
    }
    base.truncate(2);
}

fn understood(tax: &Taxonomy, f: &Facets, themes: &[Theme]) -> String {
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
    if !themes.is_empty() {
        parts.push(format!(
            "temi: {}",
            themes
                .iter()
                .map(|t| t.label.as_str())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if parts.is_empty() {
        "ricerca libera".into()
    } else {
        parts.join(" · ")
    }
}

fn candidate_of(
    tax: &Taxonomy,
    m: &Movie,
    reasons: &HashMap<u64, String>,
    relevance: &HashMap<u64, f32>,
) -> curator::Candidate {
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
        reason: reasons.get(&m.id).cloned(),
        relevance: relevance.get(&m.id).copied().unwrap_or(1.0),
    }
}

/// Un turno di chat: interpreta, cerca, cura. Ritorna il contesto effettivo e i
/// titoli proposti (per la memoria del turno successivo).
fn turn(
    tmdb: &Tmdb,
    tax: &Taxonomy,
    provider: &std::sync::Arc<dyn IntentProvider>,
    text: &str,
    active: &Intent,
    history: &[(String, String)],
) -> Option<(Intent, Vec<String>)> {
    println!("\n=== messaggio: «{text}» ===");
    let follow = is_follow_up(text);
    let empty = Intent::default();
    let parse_active = if follow { active } else { &empty };
    let parse_history: &[(String, String)] = if follow { history } else { &[] };
    let intent = match provider.parse_chat(text, tax, parse_history, parse_active) {
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
        "  facce: decenni={:?} movimenti={:?} generi={:?} esclusi={:?} paesi={:?} mood={:?} voto={:?} voti={:?} modalita={:?}",
        intent.facets.decades,
        intent.facets.movements,
        intent.facets.genres,
        intent.facets.without_genres,
        intent.facets.countries,
        intent.facets.moods,
        intent.facets.min_rating,
        intent.facets.min_votes,
        intent.mode
    );
    for theme in &intent.themes {
        println!(
            "  tema: {} [{}] seed={:?}",
            theme.label,
            theme
                .concepts
                .iter()
                .map(|c| c.label.as_str())
                .collect::<Vec<_>>()
                .join(" + "),
            theme.seed_titles
        );
    }

    let mut eff = if follow {
        active.facets.clone()
    } else {
        Facets::default()
    };
    let mut themes = if follow {
        active.themes.clone()
    } else {
        Vec::new()
    };
    if intent.source == "AI" {
        eff = intent.facets.clone();
        themes = intent.themes.clone();
    } else if follow {
        merge(&mut eff, &intent.facets);
        merge_themes(&mut themes, &intent.themes);
    } else {
        eff = intent.facets.clone();
        themes = intent.themes.clone();
    }
    if intent.mode.eq_ignore_ascii_case("reset") {
        eff = Facets::default();
        themes.clear();
    }
    let (eff, removed) = constraints::sanitize(tax, &eff);
    for r in &removed {
        println!("  vincolo: {r}");
    }
    let title = intent.free_text.clone();
    let similar = intent.similar_to.clone();
    let context = Intent {
        facets: eff.clone(),
        themes: themes.clone(),
        free_text: title.clone(),
        similar_to: similar.clone(),
        ..Default::default()
    };
    let off_topic = intent.mode.eq_ignore_ascii_case("off_topic");
    let mut descr = understood(tax, &eff, &themes);
    if themes.is_empty() {
        if let Some(s) = similar.as_deref() {
            descr = format!("simili a «{s}»");
        } else if let Some(t) = title.as_deref() {
            descr = if descr.eq_ignore_ascii_case("ricerca libera") {
                format!("titolo «{t}»")
            } else {
                format!("titolo «{t}» · {descr}")
            };
        }
    }
    if off_topic && eff.is_empty() && themes.is_empty() {
        descr = "film del momento".to_string();
    }
    println!("  capito: {descr}");

    let mut pool = match chat_pool(
        tmdb,
        tax,
        &eff,
        &themes,
        &[],
        title.as_deref(),
        similar.as_deref(),
        &mut Vec::new(),
    ) {
        Ok(p) => p,
        Err(e) => {
            println!("errore ricerca: {e}");
            return None;
        }
    };
    println!(
        "  candidati reali: {} · totale TMDB: {} · griglia: {}",
        pool.movies.len(),
        pool.total,
        pool.grid_ready
    );
    if let Some(limit) = &pool.limit {
        println!("  limite: {limit}");
    }
    for (id, reason) in &pool.reasons {
        println!("  match {id}: {reason}");
    }
    pool.movies.truncate(20);
    let movies = pool.movies;

    let req = curator::Request {
        user_text: text.to_string(),
        understood: descr,
        limit: pool.limit,
        history: if follow { history.to_vec() } else { Vec::new() },
        candidates: movies
            .iter()
            .map(|m| candidate_of(tax, m, &pool.reasons, &pool.relevance))
            .collect(),
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
    Some((context, titles))
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

    let mut active = Intent::default();
    let mut history: Vec<(String, String)> = Vec::new();
    for phrase in &phrases {
        match turn(&tmdb, &tax, &provider, phrase, &active, &history) {
            Some((context, titles)) => {
                active = context;
                history.push(("utente".into(), phrase.clone()));
                if !titles.is_empty() {
                    history.push((
                        "assistente".into(),
                        format!("Ho proposto: {}", titles.join(", ")),
                    ));
                }
            }
            None => break,
        }
    }
}
