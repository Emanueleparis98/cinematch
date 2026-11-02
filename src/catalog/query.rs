use super::store::{err, read_movie, Catalog};
use crate::domain::{
    model::{Facets, Movie, Sort},
    taxonomy::Taxonomy,
};
use rusqlite::{params_from_iter, types::Value};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReleaseScope {
    #[default]
    All,
    Released,
    Upcoming,
    Undated,
}
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MovementMatch {
    #[default]
    Candidates,
    Documented,
    Affinity,
}
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CatalogQuery {
    pub facets: Facets,
    pub text: String,
    pub known: Vec<(String, u64)>,
    pub release: ReleaseScope,
    pub include_adult: bool,
    pub include_video: bool,
    pub movements: MovementMatch,
    pub all_movements: bool,
    pub any_genres: bool,
    pub hidden: Vec<u64>,
    pub page: u32,
}
#[derive(Debug, Default)]
pub struct CatalogPage {
    pub movies: Vec<Movie>,
    pub total: u64,
    pub pages: u32,
    pub page: u32,
    pub unverified: u64,
}
impl CatalogQuery {
    pub fn scope_filters(&self) -> usize {
        usize::from(self.release != ReleaseScope::All)
            + usize::from(self.include_adult)
            + usize::from(self.include_video)
    }
    pub fn is_filtered(&self) -> bool {
        !self.facets.is_empty() || !self.text.trim().is_empty() || self.scope_filters() > 0
    }
}

#[derive(Default)]
struct Where {
    clauses: Vec<String>,
    values: Vec<Value>,
}
impl Where {
    fn bind(&mut self, value: impl Into<Value>) -> String {
        self.values.push(value.into());
        format!("?{}", self.values.len())
    }
    fn push(&mut self, sql: impl Into<String>) {
        self.clauses.push(sql.into());
    }
    fn sql(&self) -> String {
        self.clauses.join(" AND ")
    }
}

#[derive(Debug, Default)]
pub(crate) struct SessionCache {
    current: Option<std::sync::Arc<QuerySession>>,
    expired: Option<CatalogQuery>,
}
#[derive(Debug)]
struct QuerySession {
    key: CatalogQuery,
    today: String,
    created: std::time::Instant,
    connection: std::sync::Mutex<rusqlite::Connection>,
}
impl Catalog {
    /// La navigazione usa uno snapshot SQLite; Cerca/Aggiorna apre una nuova selezione.
    pub fn invalidate(&self) {
        if let Ok(mut state) = self.session.lock() {
            *state = SessionCache::default();
        }
    }
    pub(crate) fn expire_session(&self) {
        if let Ok(mut state) = self.session.lock() {
            if state
                .current
                .as_ref()
                .is_some_and(|s| s.created.elapsed() > std::time::Duration::from_secs(300))
            {
                state.expired = state.current.take().map(|s| s.key.clone());
            }
        }
    }
    pub fn query(
        &self,
        tax: &Taxonomy,
        q: &CatalogQuery,
        today: &str,
    ) -> Result<CatalogPage, String> {
        self.expire_session();
        let mut key = q.clone();
        key.page = 0;
        key.hidden.sort_unstable();
        key.hidden.dedup();
        let cached = {
            let state = self.session.lock().map_err(err)?;
            if q.page > 1 && state.expired.as_ref() == Some(&key) {
                return Err("La selezione è scaduta per consentire l'aggiornamento dell'archivio. Premi Cerca o Riprova per aprire una nuova selezione.".into());
            }
            state
                .current
                .as_ref()
                .filter(|s| s.key == key && s.today == today)
                .cloned()
        };
        let session = if let Some(session) = cached {
            session
        } else {
            let mut c = self.connect()?;
            c.execute_batch("CREATE TEMP TABLE hidden(id INTEGER PRIMARY KEY)")
                .map_err(err)?;
            {
                let tx = c.transaction().map_err(err)?;
                {
                    let mut insert = tx
                        .prepare_cached("INSERT OR IGNORE INTO hidden VALUES(?1)")
                        .map_err(err)?;
                    for id in &q.hidden {
                        insert.execute([*id as i64]).map_err(err)?;
                    }
                }
                tx.commit().map_err(err)?;
            }
            c.execute_batch("BEGIN DEFERRED").map_err(err)?;
            let session = std::sync::Arc::new(QuerySession {
                key,
                today: today.into(),
                created: std::time::Instant::now(),
                connection: std::sync::Mutex::new(c),
            });
            let mut state = self.session.lock().map_err(err)?;
            state.current = Some(session.clone());
            state.expired = None;
            session
        };
        let c = session.connection.lock().map_err(err)?;
        let w = build(tax, q, today)?;
        let total: u64 = c
            .query_row(
                &format!("SELECT count(*) FROM film f WHERE {}", w.sql()),
                params_from_iter(w.values.iter()),
                |r| Ok(r.get::<_, i64>(0)? as u64),
            )
            .map_err(err)?;
        let pages = total.div_ceil(20).min(u32::MAX as u64) as u32;
        let page = q.page.max(1).min(pages.max(1));
        let order = match q.facets.sort {
            Sort::Popularity => "f.popularity DESC,f.id ASC",
            Sort::Rating => "f.rating IS NULL,f.rating DESC,f.votes DESC,f.id ASC",
            Sort::Votes => "f.votes IS NULL,f.votes DESC,f.id ASC",
            Sort::Newest => "f.release_date IS NULL,f.release_date DESC,f.id ASC",
            Sort::Oldest => "f.release_date IS NULL,f.release_date ASC,f.id ASC",
            Sort::Upcoming => "f.release_date ASC,f.id ASC",
        };
        let sql=format!("SELECT f.id,f.title,f.original_title,f.popularity,f.movie_json FROM film f WHERE {} ORDER BY {order} LIMIT 20 OFFSET {}",w.sql(),u64::from(page-1)*20);
        let movies = {
            let mut s = c.prepare(&sql).map_err(err)?;
            let rows = s
                .query_map(params_from_iter(w.values.iter()), read_movie)
                .map_err(err)?;
            rows.collect::<Result<Vec<_>, _>>().map_err(err)?
        };
        let needs_details = !q.facets.is_empty() || q.release != ReleaseScope::All;
        let unverified = if needs_details {
            let scope = format!(
                "active=1 {} {}",
                if q.include_adult { "" } else { "AND adult=0" },
                if q.include_video { "" } else { "AND video=0" }
            );
            c.query_row(&format!("SELECT (SELECT count(*) FROM film WHERE {scope})-(SELECT count(*) FROM film INDEXED BY film_enriched_known WHERE {scope} AND enriched=1)"),[],|r|Ok(r.get::<_,i64>(0)? as u64)).map_err(err)?
        } else {
            0
        };
        Ok(CatalogPage {
            movies,
            total,
            pages,
            page,
            unverified,
        })
    }
}

fn build(tax: &Taxonomy, q: &CatalogQuery, today: &str) -> Result<Where, String> {
    let f = &q.facets;
    let mut w = Where::default();
    w.push("f.active=1");
    if !q.include_adult {
        w.push("f.adult=0");
    }
    if !q.include_video {
        w.push("f.video=0");
    }
    w.push("NOT EXISTS(SELECT 1 FROM hidden h WHERE h.id=f.id)");
    if !q.text.trim().is_empty() {
        let tokens = q
            .text
            .split(|c: char| !c.is_alphanumeric())
            .filter(|s| !s.is_empty())
            .map(|s| format!("\"{s}\"*"))
            .collect::<Vec<_>>();
        if tokens.is_empty() {
            return Err("Inserisci almeno una lettera o un numero nel titolo.".into());
        }
        let p = w.bind(tokens.join(" AND "));
        w.push(format!(
            "f.id IN(SELECT rowid FROM title_search WHERE title_search MATCH {p} UNION SELECT a.film_id FROM alias a JOIN alias_search s ON a.id=s.rowid WHERE alias_search MATCH {p})"
        ));
    }
    let release = if f.sort == Sort::Upcoming {
        ReleaseScope::Upcoming
    } else {
        q.release
    };
    match release {
        ReleaseScope::All => {}
        ReleaseScope::Released => {
            let p = w.bind(today.to_string());
            w.push(format!("f.release_date<={p}"));
        }
        ReleaseScope::Upcoming => {
            let p = w.bind(today.to_string());
            w.push(format!("f.release_date>{p}"));
        }
        ReleaseScope::Undated => w.push("f.enriched=1 AND f.release_date IS NULL"),
    }
    if !f.decades.is_empty() {
        let mut parts = Vec::new();
        for id in &f.decades {
            let d = tax.decade(id).ok_or("Decennio non riconosciuto.")?;
            let a = w.bind(format!("{}-01-01", d.from));
            let b = w.bind(format!("{}-12-31", d.to));
            parts.push(format!(
                "f.id IN(SELECT id FROM film WHERE release_date BETWEEN {a} AND {b})"
            ));
        }
        w.push(format!("({})", parts.join(" OR ")));
    }
    if q.any_genres && !f.genres.is_empty() {
        let list = f
            .genres
            .iter()
            .map(|id| w.bind(*id as i64))
            .collect::<Vec<_>>()
            .join(",");
        w.push(format!(
            "f.id IN(SELECT film_id FROM genre WHERE value IN({list}))"
        ));
    }
    for id in f.genres.iter().filter(|_| !q.any_genres) {
        let p = w.bind(*id as i64);
        w.push(format!(
            "f.id IN(SELECT film_id FROM genre WHERE value={p})"
        ));
    }
    if !f.without_genres.is_empty() {
        w.push("f.id IN(SELECT film_id FROM genre)");
    }
    for id in &f.without_genres {
        let p = w.bind(*id as i64);
        w.push(format!(
            "f.id NOT IN(SELECT film_id FROM genre WHERE value={p})"
        ));
    }
    if !f.countries.is_empty() {
        let list = f
            .countries
            .iter()
            .map(|v| w.bind(v.clone()))
            .collect::<Vec<_>>()
            .join(",");
        w.push(format!(
            "f.id IN(SELECT film_id FROM country WHERE value IN({list}))"
        ));
    }
    for name in f.moods.iter().chain(&f.keywords) {
        let id = q
            .known
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, id)| *id)
            .ok_or_else(|| format!("Tema «{name}» non risolto su TMDB."))?;
        let p = w.bind(id as i64);
        w.push(format!(
            "f.id IN(SELECT film_id FROM keyword WHERE value={p})"
        ));
    }
    if let Some(v) = f.min_rating {
        let p = w.bind(v as f64);
        w.push(format!(
            "f.id IN(SELECT id FROM film WHERE active=1 AND rating>={p})"
        ));
    }
    if let Some(v) = f.max_rating {
        let p = w.bind(v as f64);
        w.push(format!(
            "f.id IN(SELECT id FROM film WHERE active=1 AND rating<={p})"
        ));
    }
    if let Some(v) = f.min_votes {
        let p = w.bind(v as i64);
        w.push(format!(
            "f.id IN(SELECT id FROM film WHERE active=1 AND votes>={p})"
        ));
    }
    if let Some((a, b)) = f.runtime {
        let a = w.bind(a as i64);
        let b = w.bind(b as i64);
        w.push(format!(
            "f.id IN(SELECT id FROM film WHERE active=1 AND runtime BETWEEN {a} AND {b})"
        ));
    }
    if !f.movements.is_empty() {
        let mut parts = Vec::new();
        for id in &f.movements {
            let m = tax.movement(id).ok_or("Corrente non riconosciuta.")?;
            let p = w.bind(id.clone());
            let clause=match q.movements{
                MovementMatch::Documented=>format!("f.id IN(SELECT film_id FROM movement WHERE value={p} AND relation='documented')"),
                MovementMatch::Affinity=>format!("f.id IN(SELECT film_id FROM movement WHERE value={p})"),
                MovementMatch::Candidates=>{
                    // Candidatura dichiarata, senza generi vietati o potature editoriali.
                    let a=w.bind(format!("{}-01-01",m.period[0]));let b=w.bind(format!("{}-12-31",m.period[1]));
                    let countries=m.countries.iter().map(|c|w.bind(c.clone())).collect::<Vec<_>>().join(",");
                    let mut parts=vec![format!("f.id IN(SELECT id FROM film WHERE release_date BETWEEN {a} AND {b})")];
                    if !countries.is_empty(){parts.push(format!("f.id IN(SELECT film_id FROM country WHERE value IN({countries}))"));}
                    if !m.keyword.is_empty(){
                        let id=q.known.iter().find(|(n,_)|n==id).map(|(_,id)|*id).ok_or_else(||format!("Parola chiave della corrente «{}» non risolta.",m.label))?;
                        let k=w.bind(id as i64);parts.push(format!("f.id IN(SELECT film_id FROM keyword WHERE value={k})"));
                    }
                    format!("({})",parts.join(" AND "))
                }
            };
            parts.push(clause);
        }
        w.push(format!(
            "({})",
            parts.join(if q.all_movements { " AND " } else { " OR " })
        ));
    }
    Ok(w)
}
