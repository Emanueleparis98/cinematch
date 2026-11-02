//! Controllo mirato del catalogo con TMDB reale; nessun provider AI.
use cinematch::{
    catalog::{browse, CatalogQuery, MovementMatch, ReleaseScope},
    domain::{
        model::{Facets, Sort},
        taxonomy::Taxonomy,
    },
    infra::config::Config,
    tmdb::Tmdb,
};

fn main() -> Result<(), String> {
    let cfg = Config::load();
    if !cfg.has_key() {
        return Err("Configura TMDB per la prova reale.".into());
    }
    let tmdb = Tmdb::new(cfg.tmdb_api_key, cfg.language);
    let tax = Taxonomy::load();
    let cases = [
        (
            "Neorealismo e commedia",
            Facets {
                movements: vec!["neorealismo".into()],
                genres: vec![35],
                ..Default::default()
            },
        ),
        (
            "Horror anni 80 senza commedia",
            Facets {
                decades: vec!["1980s".into()],
                genres: vec![27],
                without_genres: vec![35],
                min_rating: Some(6.5),
                min_votes: Some(1000),
                runtime: Some((70, 130)),
                ..Default::default()
            },
        ),
        (
            "Recenti con voto minimo",
            Facets {
                sort: Sort::Newest,
                min_rating: Some(7.0),
                ..Default::default()
            },
        ),
    ];
    for (label, facets) in cases {
        let page = browse(
            &tmdb,
            &tax,
            &CatalogQuery {
                facets,
                page: 1,
                ..Default::default()
            },
        )?;
        if page.movies.is_empty() {
            return Err(format!("{label}: nessun risultato"));
        }
        println!(
            "{label}: PASS — {} risultati in pagina; {} pagine; esatto {}; saturo {}",
            page.movies.len(),
            page.pages,
            page.exact,
            page.capped
        );
    }
    for (label, request) in [
        (
            "Titolo",
            CatalogQuery {
                text: "Metropolis".into(),
                page: 1,
                ..Default::default()
            },
        ),
        (
            "Neorealismo documentato",
            CatalogQuery {
                facets: Facets {
                    movements: vec!["neorealismo".into()],
                    ..Default::default()
                },
                movements: MovementMatch::Documented,
                page: 1,
                ..Default::default()
            },
        ),
        (
            "Anno preciso e uscite future",
            CatalogQuery {
                years: Some((2027, 2027)),
                release: ReleaseScope::Upcoming,
                page: 1,
                ..Default::default()
            },
        ),
    ] {
        let page = browse(&tmdb, &tax, &request)?;
        if page.movies.is_empty() {
            return Err(format!("{label}: nessun risultato"));
        }
        println!(
            "{label}: PASS — {} risultati; {} pagine",
            page.movies.len(),
            page.pages
        );
    }
    Ok(())
}
