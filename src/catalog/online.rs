use super::{CatalogQuery, MovementMatch, ReleaseScope};
use crate::{
    domain::{
        constraints::{self, DiscoverQuery},
        model::{Details, Facets, Movie, Sort},
        taxonomy::Taxonomy,
    },
    tmdb::{client::Tmdb, models::PagedMovies},
};
use std::collections::HashSet;

trait Source: Sync {
    fn discover(&self, q: &DiscoverQuery) -> Result<PagedMovies, String>;
    fn search_scoped(&self, text: &str, page: u32, adult: bool) -> Result<PagedMovies, String>;
    fn catalog_details(&self, id: u64) -> Result<Details, String>;
}
impl Source for Tmdb {
    fn discover(&self, q: &DiscoverQuery) -> Result<PagedMovies, String> {
        Tmdb::discover(self, q)
    }
    fn search_scoped(&self, text: &str, page: u32, adult: bool) -> Result<PagedMovies, String> {
        Tmdb::search_scoped(self, text, page, adult)
    }
    fn catalog_details(&self, id: u64) -> Result<Details, String> {
        Tmdb::catalog_details(self, id)
    }
}

#[derive(Default, Debug)]
pub struct CatalogPage {
    pub movies: Vec<Movie>,
    pub total: u32,
    pub pages: u32,
    pub exact: bool,
    pub capped: bool,
    pub notes: Vec<String>,
}

fn tomorrow(today: &str) -> String {
    let mut parts = today.split('-').map(|s| s.parse::<u16>().unwrap_or(1));
    let (mut y, mut m, mut d) = (
        parts.next().unwrap(),
        parts.next().unwrap(),
        parts.next().unwrap(),
    );
    let days = match m {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400)) {
                29
            } else {
                28
            }
        }
        _ => 31,
    };
    d += 1;
    if d > days {
        d = 1;
        m += 1;
    }
    if m > 12 {
        m = 1;
        y += 1;
    }
    format!("{y:04}-{m:02}-{d:02}")
}

/// OR movements need independent queries; intersecting their historical periods
/// would silently discard valid films. Native source pages remain addressable.
pub fn plans(tax: &Taxonomy, r: &CatalogQuery, today: &str) -> Result<Vec<DiscoverQuery>, String> {
    let mut variants = vec![r.facets.clone()];
    if r.facets.movements.len() > 1 && !r.all_movements {
        variants = r
            .facets
            .movements
            .iter()
            .map(|id| {
                let mut f = r.facets.clone();
                f.movements = vec![id.clone()];
                f
            })
            .collect();
    }
    variants
        .into_iter()
        .map(|f| {
            let mut q = constraints::to_discover(tax, &f, &r.known, r.page.clamp(1, 500))?;
            q.genres_or = r.any_genres;
            if r.all_movements {
                q.keywords_or = false;
            }
            q.include_adult = r.include_adult;
            q.include_video = r.include_video;
            if let Some((a, b)) = r.years {
                if a > b {
                    return Err("L'anno iniziale deve precedere quello finale.".into());
                }
                q.year = Some(match q.year {
                    Some((x, y)) => (a.max(x), b.min(y)),
                    None => (a, b),
                });
                if q.year.is_some_and(|(x, y)| x > y) {
                    return Err(
                        "L'intervallo di anni non si sovrappone ai periodi selezionati.".into(),
                    );
                }
            }
            match r.release {
                ReleaseScope::Released => q.date_to = Some(today.into()),
                ReleaseScope::Upcoming => q.date_from = Some(tomorrow(today)),
                ReleaseScope::All => {}
            }
            Ok(q)
        })
        .collect()
}

fn explicit_match(d: &Details, tax: &Taxonomy, f: &Facets, r: &CatalogQuery, today: &str) -> bool {
    if (!r.include_adult && d.adult) || (!r.include_video && d.video) {
        return false;
    }
    let mut criteria = f.clone();
    if r.any_genres && !criteria.genres.is_empty() {
        if !criteria
            .genres
            .iter()
            .any(|id| d.movie.genre_ids.contains(id))
        {
            return false;
        }
        criteria.genres.clear();
    }
    if !criteria.without_genres.is_empty() && d.movie.genre_ids.is_empty() {
        return false;
    }
    if !crate::app::verified_match(d, tax, &criteria, &r.known) {
        return false;
    }
    if r.movements == MovementMatch::Candidates {
        for id in &criteria.movements {
            if let Some(m) = tax.movement(id) {
                if !m.countries.is_empty()
                    && !m
                        .countries
                        .iter()
                        .any(|code| d.origin_country_codes.contains(code))
                {
                    return false;
                }
            }
        }
    }
    if let Some((a, b)) = r.years {
        if !d.movie.year.is_some_and(|y| y >= a && y <= b) {
            return false;
        }
    }
    let date = d.movie.release_date.as_deref();
    if match r.release {
        ReleaseScope::All => false,
        ReleaseScope::Released => !date.is_some_and(|s| s <= today),
        ReleaseScope::Upcoming => !date.is_some_and(|s| s > today),
    } {
        return false;
    }
    if !r.text.trim().is_empty() {
        let needle = r.text.trim().to_lowercase();
        if !std::iter::once(&d.movie.title)
            .chain(std::iter::once(&d.movie.original_title))
            .chain(d.alternative_titles.iter())
            .any(|s| s.to_lowercase().contains(&needle))
        {
            return false;
        }
    }
    if r.all_movements && criteria.movements.len() > 1 {
        for id in &criteria.movements {
            let mut single = criteria.clone();
            single.movements = vec![id.clone()];
            if !crate::app::verified_match(d, tax, &single, &r.known) {
                return false;
            }
        }
    }
    true
}

fn verified_page(
    tmdb: &impl Source,
    tax: &Taxonomy,
    r: &CatalogQuery,
    movies: Vec<Movie>,
    today: &str,
) -> Result<Vec<Movie>, String> {
    let mut out = Vec::new();
    for batch in movies.chunks(6) {
        let details = std::thread::scope(|scope| {
            let tasks = batch
                .iter()
                .map(|m| scope.spawn(move || tmdb.catalog_details(m.id)))
                .collect::<Vec<_>>();
            tasks
                .into_iter()
                .map(|h| {
                    h.join()
                        .unwrap_or_else(|_| Err("Scheda non disponibile.".into()))
                })
                .collect::<Vec<_>>()
        });
        for d in details {
            // Failed verification is an incomplete search, never a silent nonmatch.
            let d = d?;
            let fits = if r.movements == MovementMatch::Candidates
                && !r.all_movements
                && r.facets.movements.len() > 1
            {
                r.facets.movements.iter().any(|id| {
                    let mut f = r.facets.clone();
                    f.movements = vec![id.clone()];
                    explicit_match(&d, tax, &f, r, today)
                })
            } else {
                explicit_match(&d, tax, &r.facets, r, today)
            };
            if fits {
                out.push(d.movie);
            }
        }
    }
    Ok(out)
}

pub fn browse(tmdb: &Tmdb, tax: &Taxonomy, r: &CatalogQuery) -> Result<CatalogPage, String> {
    browse_source(tmdb, tax, r)
}
fn browse_source(
    tmdb: &impl Source,
    tax: &Taxonomy,
    r: &CatalogQuery,
) -> Result<CatalogPage, String> {
    let today = crate::infra::winclock::today_iso();
    if !r.facets.movements.is_empty() && r.movements != MovementMatch::Candidates {
        let facts = super::bundled_sources();
        let ids: HashSet<_> = facts
            .iter()
            .filter(|fact| {
                r.facets.movements.contains(&fact.movement)
                    && (r.movements == MovementMatch::Affinity || fact.relation == "documented")
            })
            .map(|fact| fact.movie_id)
            .filter(|id| {
                !r.all_movements
                    || r.facets.movements.iter().all(|m| {
                        facts.iter().any(|f| {
                            f.movie_id == *id
                                && f.movement == *m
                                && (r.movements == MovementMatch::Affinity
                                    || f.relation == "documented")
                        })
                    })
            })
            .collect();
        let mut scoped = r.clone();
        scoped.facets.movements.clear();
        let movies = ids
            .into_iter()
            .map(|id| Movie {
                id,
                ..Default::default()
            })
            .collect();
        let mut movies = verified_page(tmdb, tax, &scoped, movies, &today)?;
        crate::app::sort_movies(&mut movies, r.facets.sort);
        let total = movies.len() as u32;
        let start = (r.page.max(1) - 1) as usize * 20;
        return Ok(CatalogPage {movies: movies.into_iter().skip(start).take(20).collect(), total, pages: total.div_ceil(20), exact: true,
            notes: vec!["Classificazioni con fonte incluse nell'app: raccolta parziale, non censimento della corrente.".into()],..Default::default()});
    }
    let mut result = CatalogPage {
        exact: true,
        ..Default::default()
    };
    if !r.text.trim().is_empty() {
        let p = tmdb.search_scoped(r.text.trim(), r.page.clamp(1, 500), r.include_adult)?;
        result.total = p.total_results;
        result.pages = p.total_pages.min(500);
        result.capped = p.total_pages >= 500;
        let before = p.results.len();
        result.movies = p
            .results
            .into_iter()
            .filter(|m| r.include_video || !m.video)
            .map(Movie::from)
            .collect();
        if result.movies.len() != before {
            result.exact = false;
            result
                .notes
                .push("Contenuti video nascosti dalla pagina dei risultati per titolo.".into());
        }
        if !r.facets.is_empty() || r.scope_filters() > usize::from(r.include_adult) {
            let mut scoped = r.clone();
            scoped.text.clear();
            result.movies = verified_page(tmdb, tax, &scoped, result.movies, &today)?;
            result.exact = false;
            result.notes.push("Ricerca per titolo: filtri verificati sulla pagina corrente. Il paginatore segue i risultati per titolo di TMDB.".into());
        }
        if r.facets.sort != Sort::Popularity {
            crate::app::sort_movies(&mut result.movies, r.facets.sort);
            result.exact = false;
            result.notes.push(
                "Ordinamento applicato alla pagina corrente dei risultati per titolo.".into(),
            );
        }
    } else {
        let queries = plans(tax, r, &today)?;
        let multiple = queries.len() > 1;
        let verify = r.facets.runtime.is_some()
            || constraints::decade_gap(tax, &r.facets)
            || (r.all_movements && r.facets.movements.len() > 1)
            || (!r.facets.countries.is_empty() && !r.facets.movements.is_empty());
        for q in queries {
            if r.page > 1 {
                let mut first = q.clone();
                first.page = 1;
                let summary = tmdb.discover(&first)?;
                result.pages = result.pages.max(summary.total_pages.min(500));
                result.total = result.total.saturating_add(summary.total_results);
                result.capped |= summary.total_pages >= 500;
                if r.page > summary.total_pages.min(500) {
                    continue;
                }
            }
            let p = tmdb.discover(&q)?;
            if r.page <= 1 {
                result.pages = result.pages.max(p.total_pages.min(500));
                result.total = result.total.saturating_add(p.total_results);
                result.capped |= p.total_pages >= 500;
            }
            result.movies.extend(p.results.into_iter().map(Movie::from));
        }
        let mut seen = HashSet::new();
        result.movies.retain(|m| seen.insert(m.id));
        if verify {
            result.movies = verified_page(tmdb, tax, r, result.movies, &today)?;
            result.exact = false;
            result.notes.push("I criteri non esposti dalla lista sono verificati sulle schede della pagina corrente; il totale della fonte precede questi controlli.".into());
        }
        if multiple {
            crate::app::sort_movies(&mut result.movies, r.facets.sort);
            result.exact = false;
            result.notes.push("Unione delle correnti: ogni pagina raccoglie la stessa pagina di ciascuna corrente, senza duplicati al suo interno. L'ordine vale per la pagina; le fonti possono sovrapporsi fra pagine.".into());
        }
        if !r.facets.movements.is_empty() {
            result.notes.push("Correnti: periodo, origine ed eventuali keyword individuano candidati, non appartenenze storiche certificate.".into());
        }
    }
    if !r.facets.without_genres.is_empty() {
        let before = result.movies.len();
        result.movies.retain(|m| !m.genre_ids.is_empty());
        if before != result.movies.len() {
            result.exact = false;
            result.notes.push("Titoli senza generi noti esclusi dalla pagina: non è possibile verificare il genere da evitare.".into());
        }
    }
    if result.capped {
        result.exact = false;
        result.notes.push("TMDB consente al massimo 500 pagine per query. Restringi l'intervallo di anni o aggiungi filtri per esplorare altri titoli; questo numero non rappresenta l'intero archivio TMDB.".into());
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tmdb::models::ApiMovie;
    use std::{collections::HashMap, sync::Mutex};

    #[derive(Default)]
    struct Mock {
        pages: HashMap<(String, u32), PagedMovies>,
        details: HashMap<u64, Details>,
        title: PagedMovies,
        calls: Mutex<Vec<DiscoverQuery>>,
        checked: Mutex<Vec<u64>>,
    }
    impl Source for Mock {
        fn discover(&self, q: &DiscoverQuery) -> Result<PagedMovies, String> {
            self.calls.lock().unwrap().push(q.clone());
            self.pages
                .get(&(q.countries.join(","), q.page))
                .cloned()
                .ok_or_else(|| "unexpected query".into())
        }
        fn search_scoped(&self, _: &str, _: u32, _: bool) -> Result<PagedMovies, String> {
            Ok(self.title.clone())
        }
        fn catalog_details(&self, id: u64) -> Result<Details, String> {
            self.checked.lock().unwrap().push(id);
            self.details
                .get(&id)
                .cloned()
                .ok_or_else(|| "verification unavailable".into())
        }
    }
    fn page(ids: &[u64], pages: u32, total: u32) -> PagedMovies {
        PagedMovies {
            page: 1,
            results: ids
                .iter()
                .map(|id| ApiMovie {
                    id: *id,
                    ..Default::default()
                })
                .collect(),
            total_pages: pages,
            total_results: total,
        }
    }
    #[test]
    fn broad_query_fetches_only_one_page_and_never_acquires_details() {
        let mut source = Mock::default();
        source
            .pages
            .insert((String::new(), 1), page(&[1, 2], 63402, 1268034));
        let p = browse_source(
            &source,
            &Taxonomy::load(),
            &CatalogQuery {
                page: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!(p.pages, 500);
        assert!(p.capped && !p.exact);
        assert_eq!(source.calls.lock().unwrap().len(), 1);
        assert!(source.checked.lock().unwrap().is_empty());
        assert!(p.notes.iter().any(|s| s.contains("500 pagine")));
    }
    #[test]
    fn source_already_capped_at_exactly_500_pages_is_not_an_exact_catalog_total() {
        let mut source = Mock::default();
        source
            .pages
            .insert((String::new(), 1), page(&[1], 500, 10000));
        let p = browse_source(
            &source,
            &Taxonomy::load(),
            &CatalogQuery {
                page: 1,
                ..Default::default()
            },
        )
        .unwrap();
        assert!(p.capped && !p.exact);
        assert_eq!(p.pages, 500);
    }
    #[test]
    fn explicit_scope_reaches_the_source_without_hidden_thresholds() {
        let q = CatalogQuery {
            years: Some((2028, 2028)),
            release: ReleaseScope::Upcoming,
            include_adult: true,
            include_video: true,
            any_genres: true,
            facets: Facets {
                genres: vec![18, 35],
                ..Default::default()
            },
            ..Default::default()
        };
        let p = plans(&Taxonomy::load(), &q, "2028-02-28")
            .unwrap()
            .remove(0);
        assert_eq!(p.date_from.as_deref(), Some("2028-02-29"));
        assert_eq!(p.year, Some((2028, 2028)));
        assert!(p.include_adult && p.include_video && p.genres_or);
        assert!(p.runtime.is_none() && p.min_votes.is_none());
        assert_eq!(tomorrow("2028-02-29"), "2028-03-01");
        assert_eq!(tomorrow("2026-12-31"), "2027-01-01");
    }
    #[test]
    fn incompatible_years_fail_without_dropping_user_filters() {
        let q = CatalogQuery {
            years: Some((2020, 2025)),
            facets: Facets {
                movements: vec!["neorealismo".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(plans(&Taxonomy::load(), &q, "2026-11-02").is_err());
        assert_eq!(q.facets.movements, vec!["neorealismo"]);
    }
    fn two_movements(tax: &Taxonomy) -> CatalogQuery {
        let other = tax
            .movements
            .iter()
            .find(|m| m.countries == ["FR"] && m.period[0] > 1955)
            .unwrap();
        CatalogQuery {
            facets: Facets {
                movements: vec!["neorealismo".into(), other.id.clone()],
                ..Default::default()
            },
            page: 1,
            ..Default::default()
        }
    }
    #[test]
    fn alternative_movements_keep_their_own_period_and_origin() {
        let tax = Taxonomy::load();
        let q = two_movements(&tax);
        let p = plans(&tax, &q, "2026-11-02").unwrap();
        assert_eq!(p.len(), 2);
        assert_eq!(p[0].countries, vec!["IT"]);
        assert_eq!(p[1].countries, vec!["FR"]);
        assert_ne!(p[0].year, p[1].year);
    }
    #[test]
    fn union_deduplicates_current_page_and_skips_exhausted_sources() {
        let tax = Taxonomy::load();
        let mut q = two_movements(&tax);
        let mut source = Mock::default();
        source.pages.insert(("IT".into(), 1), page(&[1, 2], 1, 2));
        source.pages.insert(("FR".into(), 1), page(&[2, 3], 2, 22));
        source.pages.insert(("FR".into(), 2), page(&[4], 2, 22));
        let p = browse_source(&source, &tax, &q).unwrap();
        assert_eq!(p.movies.len(), 3);
        assert!(!p.exact);
        assert_eq!(p.pages, 2);
        source.calls.lock().unwrap().clear();
        q.page = 2;
        let p = browse_source(&source, &tax, &q).unwrap();
        assert_eq!(p.movies[0].id, 4);
        assert!(!source
            .calls
            .lock()
            .unwrap()
            .iter()
            .any(|q| q.page == 2 && q.countries == ["IT"]));
    }
    #[test]
    fn title_verification_preserves_next_page_and_surfaces_network_failure() {
        let mut source = Mock {
            title: page(&[1], 3, 45),
            ..Default::default()
        };
        source.details.insert(
            1,
            Details {
                runtime: Some(30),
                ..Default::default()
            },
        );
        let q = CatalogQuery {
            text: "Test".into(),
            facets: Facets {
                runtime: Some((90, 120)),
                ..Default::default()
            },
            page: 1,
            ..Default::default()
        };
        let p = browse_source(&source, &Taxonomy::load(), &q).unwrap();
        assert!(p.movies.is_empty());
        assert_eq!(p.pages, 3);
        assert!(!p.exact);
        source.details.clear();
        assert!(browse_source(&source, &Taxonomy::load(), &q).is_err());
    }
    #[test]
    fn documented_mode_uses_sources_without_imposing_recipe_periods() {
        let tax = Taxonomy::load();
        let mut source = Mock::default();
        for fact in super::super::bundled_sources()
            .into_iter()
            .filter(|f| f.movement == "neorealismo")
        {
            source.details.insert(
                fact.movie_id,
                Details {
                    movie: Movie {
                        id: fact.movie_id,
                        year: Some(1962),
                        ..Default::default()
                    },
                    ..Default::default()
                },
            );
        }
        let mut q = CatalogQuery {
            facets: Facets {
                movements: vec!["neorealismo".into()],
                ..Default::default()
            },
            movements: MovementMatch::Documented,
            page: 1,
            ..Default::default()
        };
        let p = browse_source(&source, &tax, &q).unwrap();
        assert_eq!(p.total, 6);
        assert!(p.exact);
        assert!(source.calls.lock().unwrap().is_empty());
        q.movements = MovementMatch::Affinity;
        assert_eq!(browse_source(&source, &tax, &q).unwrap().total, 18);
    }
    #[test]
    fn explicit_genres_or_and_movement_origin_are_both_enforced() {
        let tax = Taxonomy::load();
        let q = CatalogQuery {
            any_genres: true,
            facets: Facets {
                genres: vec![18, 35],
                movements: vec!["neorealismo".into()],
                countries: vec!["FR".into()],
                ..Default::default()
            },
            ..Default::default()
        };
        let mut d = Details {
            movie: Movie {
                year: Some(1950),
                genre_ids: vec![35],
                ..Default::default()
            },
            origin_country_codes: vec!["FR".into()],
            ..Default::default()
        };
        assert!(!explicit_match(&d, &tax, &q.facets, &q, "2026-11-02"));
        d.origin_country_codes.push("IT".into());
        assert!(explicit_match(&d, &tax, &q.facets, &q, "2026-11-02"));
    }
}
