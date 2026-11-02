//! Controllo mirato del catalogo con TMDB reale; nessun provider AI.
use cinematch::{
    app::{discover_strict, verified_details},
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
        let mut notes = Vec::new();
        let (movies, total, pages, _) = discover_strict(&tmdb, &tax, &facets, &[], 1, &mut notes)?;
        if movies.is_empty() {
            return Err(format!("{label}: nessuna scheda da verificare"));
        }
        let sample = &movies[..movies.len().min(4)];
        let details = verified_details(&tmdb, &tax, &facets, &[], sample)?;
        if details.len() != sample.len() {
            for movie in sample {
                let d = tmdb.details(movie.id)?;
                println!("{}: anno {:?}, data {:?}, generi {:?}, paesi {:?}, durata {:?}, voto {} / {} voti, conforme {}",d.movie.title,d.movie.year,d.movie.release_date,d.movie.genre_ids,d.origin_country_codes,d.runtime,d.movie.rating,d.movie.votes,details.contains_key(&movie.id));
            }
            return Err(format!(
                "{label}: una scheda non rispetta i filtri o non è disponibile"
            ));
        }
        println!(
            "{label}: PASS — {} schede controllate; totale TMDB {total}, {pages} pagine",
            details.len()
        );
    }
    Ok(())
}
