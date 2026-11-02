//! Controlla le fonti reali senza chiamate AI a pagamento.
use cinematch::{
    infra::config::Config,
    search::{Discovery, MovieSource, SearchIntent, TmdbSource},
    tmdb::Tmdb,
};
use std::sync::atomic::AtomicBool;
fn main() -> Result<(), String> {
    let cfg = Config::load();
    let tmdb = Tmdb::new(cfg.tmdb_api_key, cfg.language);
    let source = TmdbSource::new(&tmdb)?;
    let cancel = AtomicBool::new(false);
    let evidence = source.evidence(12233, &cancel)?;
    for doc in evidence.documents {
        println!(
            "{} | {} chars | plot={} | {}",
            doc.title,
            doc.text.chars().count(),
            doc.text.contains("Plot"),
            doc.url
        );
    }
    let films = source.discover(
        &Discovery {
            kind: "narrative".into(),
            query: "dog cat friendship film".into(),
            ..Default::default()
        },
        1,
        &SearchIntent::default(),
        &cancel,
    )?;
    println!("Ricerca documentale: {} identità TMDB risolte", films.len());
    for m in films {
        println!("{} | {} | {:?}", m.id, m.title, m.year);
    }
    Ok(())
}
