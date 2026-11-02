use cinematch::{
    catalog::{Catalog, CatalogQuery, MovementFact, MovementMatch, ReleaseScope},
    domain::{
        model::{Details, Facets, Movie, Sort},
        taxonomy::Taxonomy,
    },
};
use std::{
    io::Cursor,
    path::PathBuf,
    sync::atomic::{AtomicU64, Ordering},
};
static SEQUENCE: AtomicU64 = AtomicU64::new(0);
struct Fixture {
    db: Catalog,
    path: PathBuf,
}
impl Fixture {
    fn new() -> Self {
        let path = std::env::temp_dir().join(format!(
            "cinematch-index-test-{}-{}.sqlite3",
            std::process::id(),
            SEQUENCE.fetch_add(1, Ordering::Relaxed)
        ));
        let db = Catalog::open(&path).unwrap();
        Self { db, path }
    }
    fn query(&self, q: CatalogQuery) -> cinematch::catalog::CatalogPage {
        self.db.query(&Taxonomy::load(), &q, "2026-11-02").unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        self.db.invalidate();
        for suffix in ["", "-wal", "-shm"] {
            let _ = std::fs::remove_file(format!("{}{suffix}", self.path.display()));
        }
    }
}
fn export(n: u64) -> String {
    (1..=n)
        .map(|id| format!("{{\"id\":{id},\"original_title\":\"Film {id}\",\"popularity\":1}}\n"))
        .collect()
}
fn details(id: u64, date: Option<&str>) -> Details {
    Details {
        movie: Movie {
            id,
            title: format!("Film {id}"),
            original_title: format!("Film {id}"),
            release_date: date.map(str::to_string),
            genre_ids: vec![18],
            rating: 8.0,
            votes: 100,
            ..Default::default()
        },
        runtime: Some(90),
        origin_country_codes: vec!["IT".into()],
        keywords: vec![123],
        ..Default::default()
    }
}

#[test]
fn oltre_venticinquemila_titoli_tutte_le_pagine_senza_perdite() {
    let f = Fixture::new();
    let data = export(25_017);
    assert_eq!(
        f.db.import_export(Cursor::new(&data), "2026-11-01", false, |_| true)
            .unwrap(),
        25_017
    );
    let page = f.query(CatalogQuery {
        page: 501,
        ..Default::default()
    });
    assert_eq!(page.total, 25_017);
    assert_eq!(page.pages, 1251);
    assert_eq!(page.movies[0].id, 10_001);
    let last = f.query(CatalogQuery {
        page: 1251,
        ..Default::default()
    });
    assert_eq!(last.movies.len(), 17);
    assert_eq!(last.movies.last().unwrap().id, 25_017);
    // Replay dello stesso snapshot è idempotente.
    f.db.import_export(Cursor::new(data), "2026-11-01", false, |_| true)
        .unwrap();
    assert_eq!(f.db.coverage().unwrap().identities, 25_017);
}
#[test]
fn import_interrotto_o_corrotto_non_sostituisce_lo_snapshot() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(3)), "a", false, |_| true)
        .unwrap();
    assert!(f
        .db
        .import_export(Cursor::new(export(2500)), "b", false, |_| false)
        .is_err());
    assert_eq!(f.db.coverage().unwrap().identities, 3);
    f.db.import_export(Cursor::new(export(2500)), "b", false, |_| true)
        .unwrap();
    assert_eq!(f.db.coverage().unwrap().identities, 2500);
    assert!(f
        .db
        .import_export(Cursor::new("not json"), "c", false, |_| true)
        .is_err());
    assert_eq!(f.db.coverage().unwrap().snapshot.as_deref(), Some("b"));
    assert!(f
        .db
        .import_export(Cursor::new(""), "d", false, |_| true)
        .is_err());
    assert_eq!(f.db.coverage().unwrap().identities, 2500);
}
#[test]
fn snapshot_successivo_ritira_solo_le_identita_assenti() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(3)), "a", false, |_| true)
        .unwrap();
    f.db.save_details(&details(1, Some("1951-01-01")), "it-IT")
        .unwrap();
    f.db.import_export(Cursor::new(export(2)), "b", false, |_| true)
        .unwrap();
    let s = f.db.coverage().unwrap();
    assert_eq!(s.identities, 2);
    assert_eq!(s.enriched, 1);
    assert_eq!(f.query(Default::default()).total, 2);
}
#[test]
fn ordinare_non_elimina_corti_pochi_voti_o_date_future() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(4)), "a", false, |_| true)
        .unwrap();
    let mut d = details(1, Some("2027-01-01"));
    d.runtime = Some(5);
    d.movie.votes = 1;
    f.db.save_details(&d, "it-IT").unwrap();
    for sort in [
        Sort::Popularity,
        Sort::Rating,
        Sort::Newest,
        Sort::Oldest,
        Sort::Votes,
    ] {
        let p = f.query(CatalogQuery {
            facets: Facets {
                sort,
                ..Default::default()
            },
            ..Default::default()
        });
        assert_eq!(p.total, 4);
    }
    let mut long = details(2, Some("1951-01-01"));
    long.runtime = Some(1200);
    f.db.save_details(&long, "it-IT").unwrap();
    let p = f.query(CatalogQuery {
        facets: Facets {
            runtime: Some((121, u32::MAX)),
            ..Default::default()
        },
        ..Default::default()
    });
    assert_eq!(p.total, 1);
    assert_eq!(p.movies[0].id, 2);
    let p = f.query(CatalogQuery {
        release: ReleaseScope::Upcoming,
        ..Default::default()
    });
    assert_eq!(p.total, 1);
    assert_eq!(p.movies[0].id, 1);
}
#[test]
fn filtri_contano_soltanto_metadati_verificabili_e_non_sconosciuti() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(3)), "a", false, |_| true)
        .unwrap();
    f.db.save_details(&details(1, Some("1951-01-01")), "it-IT")
        .unwrap();
    let mut missing = details(2, None);
    missing.runtime = None;
    missing.movie.votes = 0;
    missing.origin_country_codes.clear();
    f.db.save_details(&missing, "it-IT").unwrap();
    let p = f.query(CatalogQuery {
        facets: Facets {
            runtime: Some((70, 100)),
            countries: vec!["IT".into()],
            max_rating: Some(9.0),
            ..Default::default()
        },
        ..Default::default()
    });
    assert_eq!(p.total, 1);
    assert_eq!(p.unverified, 1);
    assert_eq!(
        f.query(CatalogQuery {
            release: ReleaseScope::Undated,
            ..Default::default()
        })
        .total,
        1
    );
    assert_eq!(
        f.query(CatalogQuery {
            facets: Facets {
                without_genres: vec![27],
                ..Default::default()
            },
            ..Default::default()
        })
        .total,
        2
    );
}
#[test]
fn correnti_documentate_prescindono_dalla_ricetta_e_hanno_fonti() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(3)), "a", false, |_| true)
        .unwrap();
    for id in 1..=3 {
        let mut d = details(id, Some("1951-01-01"));
        if id == 1 {
            d.movie.genre_ids = vec![14];
        }
        f.db.save_details(&d, "it-IT").unwrap();
    }
    let fact = MovementFact {
        movie_id: 1,
        movement: "neorealismo".into(),
        relation: "documented".into(),
        source: "https://www.bfi.org.uk/example".into(),
        note: "Fonte di prova".into(),
    };
    f.db.save_movement(&fact).unwrap();
    let q = CatalogQuery {
        facets: Facets {
            movements: vec!["neorealismo".into()],
            ..Default::default()
        },
        movements: MovementMatch::Documented,
        ..Default::default()
    };
    let p = f.query(q);
    assert_eq!(p.total, 1);
    assert_eq!(p.movies[0].id, 1);
    assert_eq!(f.db.movement_facts(1).unwrap().len(), 1);
    let p = f.query(CatalogQuery {
        facets: Facets {
            movements: vec!["neorealismo".into()],
            ..Default::default()
        },
        ..Default::default()
    });
    assert_eq!(p.total, 3);
}
#[test]
fn piu_correnti_si_uniscono_o_si_intersecano_esplicitamente() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(2)), "a", false, |_| true)
        .unwrap();
    f.db.save_details(&details(1, Some("1951-01-01")), "it-IT")
        .unwrap();
    let mut d = details(2, Some("1963-01-01"));
    d.origin_country_codes = vec!["FR".into()];
    f.db.save_details(&d, "it-IT").unwrap();
    let mut q = CatalogQuery {
        facets: Facets {
            movements: vec!["neorealismo".into(), "nouvelle_vague".into()],
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(f.query(q.clone()).total, 2);
    q.all_movements = true;
    assert_eq!(f.query(q).total, 0);
}
#[test]
fn ricerca_titolo_globale_accenti_e_metacaratteri_sicuri() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(3)), "a", false, |_| true)
        .unwrap();
    let mut d = details(1, Some("1951-01-01"));
    d.movie.title = "Amélie d'Italia".into();
    f.db.save_details(&d, "it-IT").unwrap();
    assert_eq!(
        f.query(CatalogQuery {
            text: "amelie".into(),
            ..Default::default()
        })
        .total,
        1
    );
    assert_eq!(
        f.query(CatalogQuery {
            text: "\" OR 1=1 --".into(),
            ..Default::default()
        })
        .total,
        0
    );
    assert_eq!(
        f.query(CatalogQuery {
            hidden: vec![1, 2],
            ..Default::default()
        })
        .total,
        1
    );
}

#[test]
fn export_reale_ammette_flag_null_e_limport_delle_fonti_e_atomico() {
    let f = Fixture::new();
    f.db.import_export(
        Cursor::new("{\"id\":1,\"original_title\":\"Film\",\"video\":null,\"adult\":null}\n"),
        "a",
        false,
        |_| true,
    )
    .unwrap();
    let valid = MovementFact {
        movie_id: 1,
        movement: "neorealismo".into(),
        relation: "documented".into(),
        source: "https://www.bfi.org.uk/example".into(),
        note: "Fonte di prova".into(),
    };
    let mut invalid = valid.clone();
    invalid.movie_id = 2;
    assert!(f.db.import_movements(&[valid, invalid]).is_err());
    assert!(f.db.movement_facts(1).unwrap().is_empty());
    f.db.set_paused(true).unwrap();
    assert!(Catalog::open(&f.path).unwrap().is_paused().unwrap());
    f.db.save_keywords(&[("critica_sociale".into(), 123)])
        .unwrap();
    assert_eq!(
        f.db.known_keywords().unwrap(),
        [("critica_sociale".into(), 123)]
    );
}

#[test]
fn la_navigazione_conserva_conti_e_schede_durante_gli_aggiornamenti() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(42)), "a", false, |_| true)
        .unwrap();
    f.db.save_details(&details(21, Some("1951-01-01")), "it-IT")
        .unwrap();
    assert_eq!(f.query(Default::default()).total, 42);
    let mut changed = details(21, Some("2027-01-01"));
    changed.movie.title = "Titolo aggiornato".into();
    f.db.save_details(&changed, "it-IT").unwrap();
    f.db.import_export(Cursor::new(export(43)), "b", false, |_| true)
        .unwrap();
    let p = f.query(CatalogQuery {
        page: 2,
        ..Default::default()
    });
    assert_eq!(p.total, 42);
    assert_eq!(p.movies[0].title, "Film 21");
    f.db.invalidate();
    let p = f.query(CatalogQuery {
        page: 2,
        ..Default::default()
    });
    assert_eq!(p.total, 43);
    assert_eq!(p.movies[0].title, "Titolo aggiornato");
}

#[test]
fn generi_or_e_ambito_esplicito_hanno_conteggi_coerenti() {
    let f = Fixture::new();
    let raw="{\"id\":1,\"original_title\":\"Uno\"}\n{\"id\":2,\"original_title\":\"Due\"}\n{\"id\":3,\"original_title\":\"Tre\",\"video\":true}\n{\"id\":4,\"original_title\":\"Quattro\",\"adult\":true}\n";
    f.db.import_export(Cursor::new(raw), "a", false, |_| true)
        .unwrap();
    f.db.save_details(&details(1, None), "it-IT").unwrap();
    let mut d = details(2, None);
    d.movie.genre_ids = vec![35];
    f.db.save_details(&d, "it-IT").unwrap();
    assert_eq!(f.query(Default::default()).total, 2);
    assert_eq!(
        f.query(CatalogQuery {
            include_adult: true,
            include_video: true,
            ..Default::default()
        })
        .total,
        4
    );
    let mut q = CatalogQuery {
        facets: Facets {
            genres: vec![18, 35],
            ..Default::default()
        },
        ..Default::default()
    };
    assert_eq!(f.query(q.clone()).total, 0);
    q.any_genres = true;
    assert_eq!(f.query(q).total, 2);
    assert_eq!(f.db.pending_in_scope(20, true, true).unwrap().len(), 2);
}

#[test]
fn anche_i_titoli_alternativi_acquisiti_sono_ricercabili() {
    let f = Fixture::new();
    f.db.import_export(Cursor::new(export(1)), "a", false, |_| true)
        .unwrap();
    let mut d = details(1, Some("1951-01-01"));
    d.alternative_titles = vec!["The Alternate Title".into()];
    f.db.save_details(&d, "it-IT").unwrap();
    assert_eq!(
        f.query(CatalogQuery {
            text: "alternate".into(),
            ..Default::default()
        })
        .total,
        1
    );
}
