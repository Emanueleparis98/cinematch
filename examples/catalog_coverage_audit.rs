//! Valutazione limitata della copertura: solo letture TMDB, nessun import massivo.
use cinematch::{
    domain::{
        constraints::{self, DiscoverQuery},
        model::Facets,
        taxonomy::Taxonomy,
    },
    infra::config::Config,
    tmdb::{models::PagedMovies, Tmdb},
};

fn report(label: &str, page: &PagedMovies, show_titles: bool) {
    println!(
        "{label}: totale={}, pagine_fonte={}, pagine_accessibili={}, schede_pagina={}",
        page.total_results,
        page.total_pages,
        page.total_pages.min(500),
        page.results.len()
    );
    if show_titles {
        for movie in &page.results {
            println!(
                "  {} | {} | generi={:?}",
                movie.title,
                if movie.release_date.is_empty() {
                    "data assente"
                } else {
                    &movie.release_date
                },
                movie.genre_ids
            );
        }
    }
}

fn main() -> Result<(), String> {
    let cfg = Config::load();
    if !cfg.has_key() {
        return Err("Configura TMDB prima della valutazione.".into());
    }
    let tmdb = Tmdb::new(cfg.tmdb_api_key, cfg.language);
    let tax = Taxonomy::load();
    // Sei richieste di lista: nessuna scansione di tutte le pagine.
    report("Apertura attuale: tendenze", &tmdb.trending(1)?, false);
    report(
        "Discover senza filtri espliciti (usciti con data)",
        &tmdb.discover(&DiscoverQuery::default())?,
        false,
    );
    let facets = Facets {
        movements: vec!["neorealismo".into()],
        ..Default::default()
    };
    let query = constraints::to_discover(&tax, &facets, &[], 1)?;
    println!(
        "Ricetta Neorealismo: anni={:?}, paesi={:?}, generi={:?}, generi_OR={}, esclusi={:?}",
        query.year, query.countries, query.genres, query.genres_or, query.without_genres
    );
    report("Neorealismo attuale", &tmdb.discover(&query)?, true);
    let broad = DiscoverQuery {
        year: query.year,
        countries: query.countries,
        ..Default::default()
    };
    report(
        "Stesso periodo e origine senza vincoli di genere",
        &tmdb.discover(&broad)?,
        false,
    );
    for years in [(2000, 2009), (2010, 2019)] {
        let query = DiscoverQuery {
            year: Some(years),
            ..Default::default()
        };
        report(
            &format!("Controllo intervallo separato {}-{}", years.0, years.1),
            &tmdb.discover(&query)?,
            false,
        );
    }
    Ok(())
}
