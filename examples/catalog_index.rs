//! Prove riproducibili dell'indice reale; nessun provider AI.
use cinematch::{
    catalog::{sync, Catalog, CatalogQuery, MovementFact},
    domain::model::Facets,
    domain::taxonomy::Taxonomy,
    infra::config::Config,
    tmdb::Tmdb,
};
use std::sync::atomic::AtomicBool;
fn main() -> Result<(), String> {
    let cfg = Config::load();
    let db = Catalog::open(Config::dir().join("catalog.sqlite3"))?;
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let cancel = AtomicBool::new(false);
    match args.first().map(String::as_str).unwrap_or("status") {
        "import" => {
            let mut last = std::time::Instant::now() - std::time::Duration::from_secs(1);
            sync::download_snapshot(
                &db,
                &Config::dir().join("catalog-exports"),
                &cancel,
                false,
                |p| {
                    if last.elapsed().as_secs() >= 2 || matches!(p, sync::Progress::Ready(_)) {
                        println!("{p:?}");
                        last = std::time::Instant::now();
                    }
                },
            )?;
        }
        "enrich" => {
            let max = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(100);
            let tmdb = Tmdb::new(cfg.tmdb_api_key, cfg.language.clone());
            sync::enrich(
                &db,
                &tmdb,
                &cfg.language,
                &cancel,
                Some(max),
                (false, false),
                |p| {
                    if matches!(p,sync::Progress::Metadata{done,..} if done%25==0) {
                        println!("{p:?}");
                    }
                },
            )?;
        }
        "page" => {
            let page = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let result = db.query(
                &Taxonomy::load(),
                &CatalogQuery {
                    page,
                    ..Default::default()
                },
                &cinematch::infra::winclock::today_iso(),
            )?;
            println!(
                "Pagina {} / {} · {} titoli",
                result.page, result.pages, result.total
            );
            for m in result.movies {
                println!("{} | {} | {:?}", m.id, m.title, m.year);
            }
        }
        "selection" => {
            let id = args.get(1).cloned().unwrap_or("neorealismo".into());
            let request = CatalogQuery {
                facets: Facets {
                    movements: vec![id],
                    ..Default::default()
                },
                ..Default::default()
            };
            let tmdb = Tmdb::new(cfg.tmdb_api_key, cfg.language.clone());
            sync::enrich_selection(
                &db,
                &tmdb,
                &cfg.language,
                &Taxonomy::load(),
                &request,
                &cancel,
                |p| {
                    if matches!(p,sync::Progress::Metadata{done,..} if done%50==0) {
                        println!("{p:?}");
                    }
                },
            )?;
        }
        "seed-sources" => {
            let titles = [
                ("Sciuscià", 1946),
                ("Miracolo a Milano", 1951),
                ("La strada", 1954),
                ("Luci del varietà", 1950),
                ("Paisà", 1946),
                ("Germania anno zero", 1948),
                ("Roma città aperta", 1945),
                ("Ladri di biciclette", 1948),
                ("Riso amaro", 1949),
                ("I bambini ci guardano", 1943),
                ("Viaggio in Italia", 1954),
                ("Europa '51", 1952),
                ("Stromboli (Terra di Dio)", 1950),
                ("Il posto", 1961),
                ("Francesco, giullare di Dio", 1950),
                ("Salvatore Giuliano", 1962),
                ("I vitelloni", 1953),
                ("Umberto D.", 1952),
            ];
            let tmdb = Tmdb::new(cfg.tmdb_api_key, cfg.language.clone());
            let mut facts = Vec::new();
            let normalize = |s: &str| {
                s.chars()
                    .filter(|c| c.is_alphanumeric())
                    .flat_map(char::to_lowercase)
                    .collect::<String>()
            };
            for (title, year) in titles {
                let result = tmdb.search(title, 1)?;
                let movie = result
                    .results
                    .into_iter()
                    .find(|m| {
                        m.release_date.starts_with(&year.to_string())
                            && (normalize(&m.title) == normalize(title)
                                || normalize(&m.original_title) == normalize(title))
                    })
                    .ok_or_else(|| {
                        format!("Identità non risolta con certezza: {title} ({year})")
                    })?;
                let details = tmdb.catalog_details(movie.id)?;
                db.save_details(&details, &cfg.language)?;
                let (relation,source,note)=match title{
                    "Sciuscià"|"Miracolo a Milano"|"Umberto D."=> ("documented","https://www.bfi.org.uk/sight-and-sound/features/roots-neorealism","Classico neorealista secondo la ricostruzione BFI."),
                    "Ladri di biciclette"=>("documented","https://www.bfi.org.uk/film/594f7408-2fdd-55a0-a347-79370e42e0ed/bicycle-thieves","Opera neorealista nella scheda BFI."),
                    "Roma città aperta"=>("documented","https://www.bfi.org.uk/bfi-film-releases/rome-open-city-roma-citta-aperta","Opera neorealista nella presentazione BFI."),
                    "Paisà"=>("documented","https://www.criterion.com/films/2415-paisan","Presentazione Criterion del film di Rossellini."),
                    _=>("affinity","https://www.criterion.com/shop/collection/6-italian-neorealism","Selezione Criterion: Italian Neorealism."),
                };
                let fact = MovementFact {
                    movie_id: movie.id,
                    movement: "neorealismo".into(),
                    relation: relation.into(),
                    source: source.into(),
                    note: note.into(),
                };
                db.save_movement(&fact)?;
                facts.push(fact);
                println!(
                    "Fonte associata: {} ({year}), ID {}",
                    details.movie.title, movie.id
                );
            }
            let data = serde_json::to_string_pretty(&facts).map_err(|e| e.to_string())?;
            std::fs::write("assets/movement-facts.json", data).map_err(|e| e.to_string())?;
        }
        "status" => {}
        "countries" => {
            let tmdb = Tmdb::new(cfg.tmdb_api_key, cfg.language.clone());
            let mut countries = Vec::new();
            for country in tmdb.configuration_countries()? {
                let code = country
                    .get("iso_3166_1")
                    .and_then(|v| v.as_str())
                    .ok_or("Paese senza codice.")?;
                let name = country
                    .get("native_name")
                    .and_then(|v| v.as_str())
                    .filter(|s| !s.trim().is_empty())
                    .or_else(|| country.get("english_name").and_then(|v| v.as_str()))
                    .ok_or("Paese senza nome.")?;
                countries.push(serde_json::json!({"code":code,"name":name}));
            }
            println!(
                "{} paesi dalla configurazione TMDB, lingua {}",
                countries.len(),
                cfg.language
            );
            std::fs::write(
                "assets/countries.json",
                serde_json::to_string_pretty(&countries).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
        }
        "verify" => {
            let tax = Taxonomy::load();
            let queries = [
                (
                    "Pagina 501",
                    CatalogQuery {
                        page: 501,
                        ..Default::default()
                    },
                ),
                (
                    "Candidati Neorealismo",
                    CatalogQuery {
                        facets: Facets {
                            movements: vec!["neorealismo".into()],
                            ..Default::default()
                        },
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
                        movements: cinematch::catalog::MovementMatch::Documented,
                        ..Default::default()
                    },
                ),
                (
                    "Neorealismo con affinità",
                    CatalogQuery {
                        facets: Facets {
                            movements: vec!["neorealismo".into()],
                            ..Default::default()
                        },
                        movements: cinematch::catalog::MovementMatch::Affinity,
                        ..Default::default()
                    },
                ),
            ];
            for (label, q) in queries {
                db.invalidate();
                let start = std::time::Instant::now();
                let p = db.query(&tax, &q, &cinematch::infra::winclock::today_iso())?;
                println!(
                    "{label}: {} risultati, {} pagine, pagina {}, {} schede, {} ms",
                    p.total,
                    p.pages,
                    p.page,
                    p.movies.len(),
                    start.elapsed().as_millis()
                );
                if label == "Pagina 501" && (p.page != 501 || p.movies.len() != 20) {
                    return Err("Navigazione globale non riuscita.".into());
                }
                if label == "Neorealismo documentato" && !p.movies.iter().any(|m| m.id == 43379) {
                    return Err("Miracolo a Milano documentato è stato escluso.".into());
                }
            }
        }
        _ => return Err("Comandi: import | enrich [numero] | page [numero] | status".into()),
    }
    println!("Copertura: {:?}", db.coverage()?);
    Ok(())
}
