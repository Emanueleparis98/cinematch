//! Download di uno snapshot e completamento riprendibile dei metadati.
use super::Catalog;
use super::{CatalogQuery, MovementMatch, ReleaseScope};
use crate::domain::{
    constraints::{self, DiscoverQuery},
    taxonomy::Taxonomy,
};
use crate::tmdb::Tmdb;
use flate2::read::GzDecoder;
use std::{
    fs::File,
    io::{BufReader, Read, Write},
    path::Path,
    sync::atomic::{AtomicBool, Ordering},
    time::Duration,
};

#[derive(Clone, Debug)]
pub enum Progress {
    Download(u64),
    Import(u64),
    Metadata { done: u64, failed: u64 },
    Ready(String),
}

/// Usa un file .part fino a download terminato; lo snapshot viene pubblicato solo dopo il parsing completo.
pub fn download_snapshot(
    db: &Catalog,
    dir: &Path,
    cancel: &AtomicBool,
    adult: bool,
    mut progress: impl FnMut(Progress),
) -> Result<u64, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let http = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(180))
        .user_agent("CineMatch catalog index/0.12")
        .build()
        .map_err(|e| e.to_string())?;
    let today = crate::infra::winclock::today_iso();
    let dates = previous_dates(&today, 4)?;
    let mut selected = None;
    for date in dates {
        if cancel.load(Ordering::Relaxed) {
            return Err("Sincronizzazione in pausa.".into());
        }
        let name = format!(
            "{}movie_ids_{}_{}_{}.json.gz",
            if adult { "adult_" } else { "" },
            &date[5..7],
            &date[8..10],
            &date[..4]
        );
        let path = dir.join(&name);
        if path.exists() {
            selected = Some((date, path));
            break;
        }
        let response = http
            .get(format!("https://files.tmdb.org/p/exports/{name}"))
            .send()
            .map_err(|e| e.without_url().to_string())?;
        if response.status().as_u16() == 404 {
            continue;
        }
        if !response.status().is_success() {
            return Err(format!("Export TMDB: HTTP {}", response.status().as_u16()));
        }
        let mut response = response;
        let partial = path.with_extension("part");
        let mut file = File::create(&partial).map_err(|e| e.to_string())?;
        let mut buffer = [0; 65536];
        let mut bytes = 0;
        loop {
            if cancel.load(Ordering::Relaxed) {
                return Err("Download in pausa; lo snapshot precedente resta disponibile.".into());
            }
            let n = response.read(&mut buffer).map_err(|e| e.to_string())?;
            if n == 0 {
                break;
            }
            file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
            bytes += n as u64;
            progress(Progress::Download(bytes));
        }
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        std::fs::rename(&partial, &path).map_err(|e| e.to_string())?;
        selected = Some((date, path));
        break;
    }
    let (date, path) =
        selected.ok_or("Export giornaliero non disponibile negli ultimi quattro giorni.")?;
    let reader = BufReader::new(GzDecoder::new(File::open(path).map_err(|e| e.to_string())?));
    let count = db.import_export(reader, &date, adult, |n| {
        progress(Progress::Import(n));
        !cancel.load(Ordering::Relaxed)
    })?;
    progress(Progress::Ready(format!(
        "Snapshot {date}: {count} identificativi importati."
    )));
    Ok(count)
}

pub fn enrich(
    db: &Catalog,
    tmdb: &Tmdb,
    language: &str,
    cancel: &AtomicBool,
    max: Option<u64>,
    scope: (bool, bool),
    mut progress: impl FnMut(Progress),
) -> Result<u64, String> {
    let mut done = 0;
    let mut failed = 0;
    let mut consecutive = 0;
    loop {
        if cancel.load(Ordering::Relaxed) || max.is_some_and(|max| done + failed >= max) {
            break;
        }
        let ids = db.pending_in_scope(20, scope.0, scope.1)?;
        if ids.is_empty() {
            break;
        }
        for id in ids {
            if cancel.load(Ordering::Relaxed) || max.is_some_and(|max| done + failed >= max) {
                break;
            }
            match tmdb.catalog_details(id) {
                Ok(d) => {
                    db.save_details(&d, language)?;
                    done += 1;
                    consecutive = 0;
                }
                Err(error) => {
                    if cancel.load(Ordering::Relaxed) {
                        return Ok(done);
                    }
                    failed += 1;
                    consecutive += 1;
                    db.defer(id)?;
                    if error.contains("401") || consecutive >= 5 {
                        return Err(format!(
                            "Completamento sospeso dopo {consecutive} errori consecutivi: {error}"
                        ));
                    }
                }
            }
            progress(Progress::Metadata { done, failed });
            std::thread::sleep(Duration::from_millis(120));
        }
    }
    Ok(done)
}

/// Completa la selezione senza fermarsi a 500 pagine: le query sature vengono suddivise.
pub fn enrich_selection(
    db: &Catalog,
    tmdb: &Tmdb,
    language: &str,
    tax: &Taxonomy,
    request: &CatalogQuery,
    cancel: &AtomicBool,
    mut progress: impl FnMut(Progress),
) -> Result<u64, String> {
    let mut seen = std::collections::HashSet::new();
    let mut done = 0;
    let mut failed = 0;
    let mut ingest = |movies: Vec<crate::tmdb::models::ApiMovie>| -> Result<(), String> {
        let mut pending = Vec::new();
        for movie in movies {
            if seen.insert(movie.id) && db.needs_metadata(movie.id)? {
                pending.push(movie.id);
            }
        }
        for batch in pending.chunks(6) {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            let results = std::thread::scope(|scope| {
                let jobs = batch
                    .iter()
                    .map(|id| scope.spawn(move || (*id, tmdb.catalog_details(*id))))
                    .collect::<Vec<_>>();
                jobs.into_iter()
                    .map(|job| {
                        job.join()
                            .map_err(|_| "Lavoro scheda interrotto.".to_string())
                    })
                    .collect::<Result<Vec<_>, _>>()
            })?;
            for (id, result) in results {
                match result {
                    Ok(details) => {
                        db.save_details(&details, language)?;
                        done += 1;
                    }
                    Err(error) => {
                        if cancel.load(Ordering::Relaxed) {
                            return Ok(());
                        }
                        failed += 1;
                        db.defer(id)?;
                        if failed >= 5 {
                            return Err(format!("Completamento della selezione sospeso: {error}"));
                        }
                    }
                }
                progress(Progress::Metadata { done, failed });
            }
        }
        Ok(())
    };
    if !request.text.trim().is_empty() {
        let first = tmdb.search(&request.text, 1)?;
        if first.total_pages > 500 {
            return Err("La ricerca per titolo è troppo ampia per il completamento online. Specifica il titolo; l'indice locale resta consultabile senza quel limite.".into());
        }
        let pages = first.total_pages;
        ingest(first.results)?;
        for page in 2..=pages {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            ingest(tmdb.search(&request.text, page)?.results)?;
        }
        return Ok(done);
    }
    // Termina il prestito del contatore nella closure prima del ramo delle fonti.
    if request.movements != MovementMatch::Candidates && !request.facets.movements.is_empty() {
        for id in db.movement_ids(
            &request.facets.movements,
            request.movements == MovementMatch::Documented,
        )? {
            if cancel.load(Ordering::Relaxed) {
                break;
            }
            if db.needs_metadata(id)? {
                db.save_details(&tmdb.catalog_details(id)?, language)?;
                done += 1;
                progress(Progress::Metadata { done, failed });
            }
        }
        return Ok(done);
    }
    if request.release == ReleaseScope::Undated {
        return Err("Per trovare le schede senza data prosegui il completamento generale: TMDB non offre questo filtro Discover.".into());
    }
    let variants = if request.facets.movements.is_empty() {
        vec![request.facets.clone()]
    } else {
        request
            .facets
            .movements
            .iter()
            .map(|id| {
                let mut f = request.facets.clone();
                f.movements = vec![id.clone()];
                f
            })
            .collect()
    };
    let today = crate::infra::winclock::today_iso();
    for facets in variants {
        let mut q = constraints::to_discover(tax, &facets, &request.known, 1)?;
        q.include_adult = request.include_adult;
        q.include_video = request.include_video;
        q.genres_or = request.any_genres;
        q.year = Some(q.year.unwrap_or((1, 9999)));
        match request.release {
            ReleaseScope::Released => q.date_to = Some(today.clone()),
            ReleaseScope::Upcoming => q.date_from = Some(today.clone()),
            _ => {}
        }
        let mut work = vec![q];
        while let Some(mut q) = work.pop() {
            if cancel.load(Ordering::Relaxed) {
                return Ok(done);
            }
            let (a, b) = q.year.unwrap();
            if q.date_from
                .as_ref()
                .is_some_and(|from| format!("{b:04}-12-31") < *from)
                || q.date_to
                    .as_ref()
                    .is_some_and(|to| format!("{a:04}-01-01") > *to)
            {
                continue;
            }
            let result = tmdb.discover(&q)?;
            if result.total_pages > 500 {
                let (left, right) = split_query(&q)?;
                work.push(right);
                work.push(left);
                continue;
            }
            let pages = result.total_pages;
            ingest(result.results)?;
            for page in 2..=pages {
                if cancel.load(Ordering::Relaxed) {
                    return Ok(done);
                }
                q.page = page;
                ingest(tmdb.discover(&q)?.results)?;
            }
        }
    }
    Ok(done)
}

fn split_query(q: &DiscoverQuery) -> Result<(DiscoverQuery, DiscoverQuery), String> {
    let (a, b) = q.year.unwrap_or((1, 9999));
    let mut left = q.clone();
    let mut right = q.clone();
    if a < b {
        let middle = a + (b - a) / 2;
        left.year = Some((a, middle));
        right.year = Some((middle + 1, b));
        return Ok((left, right));
    }
    let from = q
        .date_from
        .as_deref()
        .filter(|s| s.starts_with(&format!("{a:04}")))
        .map(|s| s[5..].to_string())
        .unwrap_or("01-01".into());
    let to = q
        .date_to
        .as_deref()
        .filter(|s| s.starts_with(&format!("{a:04}")))
        .map(|s| s[5..].to_string())
        .unwrap_or("12-31".into());
    let month_a: u32 = from[..2].parse().map_err(|_| "Data non valida.")?;
    let month_b: u32 = to[..2].parse().map_err(|_| "Data non valida.")?;
    if month_a < month_b {
        let mid = (month_a + month_b) / 2;
        left.date_to = Some(format!("{a:04}-{mid:02}-{}", days_in_month(a, mid)));
        right.date_from = Some(format!("{a:04}-{:02}-01", mid + 1));
    } else {
        let day_a: u32 = from[3..].parse().map_err(|_| "Data non valida.")?;
        let day_b: u32 = to[3..].parse().map_err(|_| "Data non valida.")?;
        if day_a >= day_b {
            return Err("Una singola data supera 500 pagine: questa acquisizione non può essere dichiarata completa. Prosegui il completamento generale dell'indice.".into());
        }
        let mid = (day_a + day_b) / 2;
        left.date_to = Some(format!("{a:04}-{month_a:02}-{mid:02}"));
        right.date_from = Some(format!("{a:04}-{month_a:02}-{:02}", mid + 1));
    }
    Ok((left, right))
}
fn days_in_month(y: u16, m: u32) -> u32 {
    match m {
        4 | 6 | 9 | 11 => 30,
        2 => {
            if y.is_multiple_of(4) && (!y.is_multiple_of(100) || y.is_multiple_of(400)) {
                29
            } else {
                28
            }
        }
        _ => 31,
    }
}

fn previous_dates(today: &str, count: usize) -> Result<Vec<String>, String> {
    let parts = today
        .split('-')
        .map(str::parse::<u32>)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?;
    if parts.len() != 3 {
        return Err("Data non valida.".into());
    }
    let (mut y, mut m, mut d) = (parts[0], parts[1], parts[2]);
    let mut out = Vec::new();
    for _ in 0..count {
        out.push(format!("{y:04}-{m:02}-{d:02}"));
        if d > 1 {
            d -= 1;
        } else {
            if m > 1 {
                m -= 1;
            } else {
                y -= 1;
                m = 12;
            }
            d = match m {
                4 | 6 | 9 | 11 => 30,
                2 => {
                    if y % 4 == 0 && (y % 100 != 0 || y % 400 == 0) {
                        29
                    } else {
                        28
                    }
                }
                _ => 31,
            };
        }
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    #[test]
    fn query_sature_si_dividono_senza_sovrapporre_intervalli() {
        let mut q = crate::domain::constraints::DiscoverQuery {
            year: Some((1940, 1959)),
            ..Default::default()
        };
        let (a, b) = super::split_query(&q).unwrap();
        assert_eq!(a.year, Some((1940, 1949)));
        assert_eq!(b.year, Some((1950, 1959)));
        q.year = Some((2024, 2024));
        q.date_from = Some("2024-02-01".into());
        q.date_to = Some("2024-02-29".into());
        let (a, b) = super::split_query(&q).unwrap();
        assert_eq!(a.date_to.as_deref(), Some("2024-02-15"));
        assert_eq!(b.date_from.as_deref(), Some("2024-02-16"));
        q.date_from = Some("2024-02-29".into());
        assert!(super::split_query(&q).is_err());
    }
    #[test]
    fn date_export_attraversano_mesi_e_anni() {
        assert_eq!(
            super::previous_dates("2024-03-01", 3).unwrap(),
            ["2024-03-01", "2024-02-29", "2024-02-28"]
        );
        assert_eq!(
            super::previous_dates("2026-01-01", 2).unwrap(),
            ["2026-01-01", "2025-12-31"]
        );
    }
}
