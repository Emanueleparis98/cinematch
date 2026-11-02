use crate::domain::model::{Details, Movie};
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::{
    io::BufRead,
    path::{Path, PathBuf},
    time::Duration,
};

#[derive(Clone, Debug)]
pub struct Catalog {
    pub(crate) path: PathBuf,
    pub(crate) session: Arc<Mutex<super::query::SessionCache>>,
}

#[derive(Clone, Debug, Default)]
pub struct Coverage {
    pub identities: u64,
    pub enriched: u64,
    pub runtime_known: u64,
    pub origin_known: u64,
    pub snapshot: Option<String>,
    pub documented: u64,
}

#[derive(Deserialize, Debug)]
pub struct ExportIdentity {
    pub id: u64,
    pub original_title: String,
    #[serde(default)]
    pub popularity: f64,
    #[serde(default, deserialize_with = "nullable_bool")]
    pub adult: bool,
    #[serde(default, deserialize_with = "nullable_bool")]
    pub video: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct MovementFact {
    pub movie_id: u64,
    pub movement: String,
    pub relation: String,
    pub source: String,
    pub note: String,
}

pub(crate) fn err(e: impl std::fmt::Display) -> String {
    format!("Indice del catalogo: {e}")
}
fn nullable_bool<'de, D: serde::Deserializer<'de>>(d: D) -> Result<bool, D::Error> {
    Ok(Option::<bool>::deserialize(d)?.unwrap_or_default())
}

impl Catalog {
    pub fn open(path: impl AsRef<Path>) -> Result<Self, String> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(err)?;
        }
        let db = Self {
            path,
            session: Arc::new(Mutex::new(super::query::SessionCache::default())),
        };
        let c = db.connect()?;
        c.execute_batch("PRAGMA journal_mode=WAL;
            CREATE TABLE IF NOT EXISTS film(
              id INTEGER PRIMARY KEY, title TEXT NOT NULL, original_title TEXT NOT NULL,
              popularity REAL NOT NULL DEFAULT 0, adult INTEGER NOT NULL DEFAULT 0,
              video INTEGER NOT NULL DEFAULT 0, active INTEGER NOT NULL DEFAULT 1,
              enriched INTEGER NOT NULL DEFAULT 0, release_date TEXT, runtime INTEGER,
              rating REAL, votes INTEGER, movie_json TEXT, updated INTEGER NOT NULL DEFAULT 0,
              retry_after INTEGER NOT NULL DEFAULT 0, data_language TEXT NOT NULL DEFAULT '');
            CREATE INDEX IF NOT EXISTS film_pop ON film(active, adult, video, popularity DESC, id);
            CREATE INDEX IF NOT EXISTS film_release ON film(release_date, id);
            CREATE INDEX IF NOT EXISTS film_enriched_known ON film(id) WHERE active=1 AND enriched=1;
            CREATE INDEX IF NOT EXISTS film_runtime_known ON film(runtime,id) WHERE active=1 AND runtime IS NOT NULL;
            CREATE INDEX IF NOT EXISTS film_rating_known ON film(rating,id) WHERE active=1 AND rating IS NOT NULL;
            CREATE INDEX IF NOT EXISTS film_votes_known ON film(votes,id) WHERE active=1 AND votes IS NOT NULL;
            CREATE INDEX IF NOT EXISTS film_pending ON film(enriched, retry_after, popularity DESC);
            CREATE TABLE IF NOT EXISTS genre(film_id INTEGER, value INTEGER, PRIMARY KEY(film_id,value));
            CREATE INDEX IF NOT EXISTS genre_value ON genre(value,film_id);
            CREATE TABLE IF NOT EXISTS country(film_id INTEGER, value TEXT, PRIMARY KEY(film_id,value));
            CREATE INDEX IF NOT EXISTS country_value ON country(value,film_id);
            CREATE TABLE IF NOT EXISTS keyword(film_id INTEGER, value INTEGER, PRIMARY KEY(film_id,value));
            CREATE INDEX IF NOT EXISTS keyword_value ON keyword(value,film_id);
            CREATE TABLE IF NOT EXISTS movement(film_id INTEGER, value TEXT, relation TEXT,
              source TEXT NOT NULL, note TEXT NOT NULL, PRIMARY KEY(film_id,value,relation,source));
            CREATE INDEX IF NOT EXISTS movement_value ON movement(value,film_id);
            CREATE TABLE IF NOT EXISTS meta(key TEXT PRIMARY KEY,value TEXT NOT NULL);
            CREATE TABLE IF NOT EXISTS alias(id INTEGER PRIMARY KEY,film_id INTEGER,title TEXT,UNIQUE(film_id,title));
            CREATE TABLE IF NOT EXISTS alias_version(film_id INTEGER PRIMARY KEY,version INTEGER);
            CREATE VIRTUAL TABLE IF NOT EXISTS alias_search USING fts5(title,content='alias',content_rowid='id',tokenize='unicode61 remove_diacritics 2');
            CREATE TRIGGER IF NOT EXISTS alias_ai AFTER INSERT ON alias BEGIN
              INSERT INTO alias_search(rowid,title) VALUES(new.id,new.title); END;
            CREATE TRIGGER IF NOT EXISTS alias_ad AFTER DELETE ON alias BEGIN
              INSERT INTO alias_search(alias_search,rowid,title) VALUES('delete',old.id,old.title); END;
            CREATE TABLE IF NOT EXISTS staging(id INTEGER PRIMARY KEY,title TEXT NOT NULL,
              popularity REAL NOT NULL,adult INTEGER NOT NULL,video INTEGER NOT NULL);
            CREATE VIRTUAL TABLE IF NOT EXISTS title_search USING fts5(title,original_title,
              content='film',content_rowid='id',tokenize='unicode61 remove_diacritics 2');
            CREATE TRIGGER IF NOT EXISTS film_ai AFTER INSERT ON film BEGIN
              INSERT INTO title_search(rowid,title,original_title) VALUES(new.id,new.title,new.original_title); END;
            CREATE TRIGGER IF NOT EXISTS film_ad AFTER DELETE ON film BEGIN
              INSERT INTO title_search(title_search,rowid,title,original_title) VALUES('delete',old.id,old.title,old.original_title); END;
            CREATE TRIGGER IF NOT EXISTS film_au AFTER UPDATE OF title,original_title ON film BEGIN
              INSERT INTO title_search(title_search,rowid,title,original_title) VALUES('delete',old.id,old.title,old.original_title);
              INSERT INTO title_search(rowid,title,original_title) VALUES(new.id,new.title,new.original_title); END;")
            .map_err(err)?;
        Ok(db)
    }
    pub(crate) fn connect(&self) -> Result<Connection, String> {
        let c = Connection::open(&self.path).map_err(err)?;
        c.busy_timeout(Duration::from_secs(10)).map_err(err)?;
        c.pragma_update(None, "foreign_keys", true).map_err(err)?;
        Ok(c)
    }
    pub fn coverage(&self) -> Result<Coverage, String> {
        let c = self.connect()?;
        let mut s=c.query_row("SELECT
          (SELECT count(*) FROM film WHERE active=1),
          (SELECT count(*) FROM film INDEXED BY film_enriched_known WHERE active=1 AND enriched=1),
          (SELECT count(*) FROM film INDEXED BY film_runtime_known WHERE active=1 AND runtime IS NOT NULL),
          (SELECT count(DISTINCT country.film_id) FROM country CROSS JOIN film WHERE film.id=country.film_id AND film.active=1)",[],|r|Ok(Coverage{
            identities:r.get::<_,i64>(0)? as u64,enriched:r.get::<_,i64>(1)? as u64,runtime_known:r.get::<_,i64>(2)? as u64,origin_known:r.get::<_,i64>(3)? as u64,..Default::default()
          })).map_err(err)?;
        s.snapshot = c
            .query_row("SELECT value FROM meta WHERE key='snapshot'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(err)?;
        s.documented = c
            .query_row(
                "SELECT count(DISTINCT film_id) FROM movement WHERE relation='documented'",
                [],
                |r| Ok(r.get::<_, i64>(0)? as u64),
            )
            .map_err(err)?;
        Ok(s)
    }
    /// Staging riprendibile e pubblicazione atomica: un file incompleto non diventa uno snapshot.
    pub fn import_export(
        &self,
        reader: impl BufRead,
        snapshot: &str,
        adult_export: bool,
        mut keep_going: impl FnMut(u64) -> bool,
    ) -> Result<u64, String> {
        let mut c = self.connect()?;
        let stage_key = format!("{snapshot}:{}", adult_export as u8);
        let old: Option<String> = c
            .query_row("SELECT value FROM meta WHERE key='staging'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(err)?;
        if old.as_deref() != Some(&stage_key) {
            let tx = c.transaction().map_err(err)?;
            tx.execute("DELETE FROM staging", []).map_err(err)?;
            tx.execute(
                "INSERT OR REPLACE INTO meta VALUES('staging',?1)",
                [&stage_key],
            )
            .map_err(err)?;
            tx.commit().map_err(err)?;
        }
        let mut count = 0u64;
        let mut batch = Vec::with_capacity(2000);
        for line in reader.lines() {
            let line = line.map_err(err)?;
            let mut row: ExportIdentity = serde_json::from_str(&line).map_err(err)?;
            row.adult |= adult_export;
            if row.id == 0
                || row.id > i64::MAX as u64
                || row.original_title.trim().is_empty()
                || !row.popularity.is_finite()
            {
                return Err(
                    "Export TMDB non valido: identificativo, titolo o popolarità assenti.".into(),
                );
            }
            batch.push(row);
            count += 1;
            if batch.len() == 2000 {
                stage_batch(&mut c, &batch)?;
                batch.clear();
                if !keep_going(count) {
                    return Err(
                        "Importazione in pausa; i dati già pubblicati restano disponibili.".into(),
                    );
                }
            }
        }
        stage_batch(&mut c, &batch)?;
        if count == 0 {
            return Err("Export TMDB vuoto: snapshot precedente conservato.".into());
        }
        if !keep_going(count) {
            return Err("Importazione in pausa.".into());
        }
        let unique: u64 = c
            .query_row("SELECT count(*) FROM staging", [], |r| {
                Ok(r.get::<_, i64>(0)? as u64)
            })
            .map_err(err)?;
        if unique != count {
            return Err("Export con identificativi duplicati: snapshot non pubblicato.".into());
        }
        let tx = c.transaction().map_err(err)?;
        tx.execute("UPDATE film SET active=0 WHERE adult=?1", [adult_export])
            .map_err(err)?;
        tx.execute(
            "INSERT INTO film(id,title,original_title,popularity,adult,video,active)
            SELECT id,title,title,popularity,adult,video,1 FROM staging WHERE 1
            ON CONFLICT(id) DO UPDATE SET original_title=excluded.original_title,
              title=CASE WHEN film.enriched=0 THEN excluded.title ELSE film.title END,
              popularity=excluded.popularity,adult=excluded.adult,video=excluded.video,active=1",
            [],
        )
        .map_err(err)?;
        let key = if adult_export {
            "adult_snapshot"
        } else {
            "snapshot"
        };
        tx.execute(
            "INSERT OR REPLACE INTO meta VALUES(?1,?2)",
            params![key, snapshot],
        )
        .map_err(err)?;
        tx.execute(
            "INSERT OR REPLACE INTO meta VALUES(?1,?2)",
            params![format!("{key}_count"), count.to_string()],
        )
        .map_err(err)?;
        tx.execute("DELETE FROM staging", []).map_err(err)?;
        tx.execute("DELETE FROM meta WHERE key='staging'", [])
            .map_err(err)?;
        tx.commit().map_err(err)?;
        Ok(count)
    }
    pub fn save_details(&self, d: &Details, language: &str) -> Result<(), String> {
        self.expire_session();
        let mut c = self.connect()?;
        let tx = c.transaction().map_err(err)?;
        let mut movie = d.movie.clone();
        movie.overview.clear();
        movie.backdrop_path = None;
        movie.track = Default::default();
        let json = serde_json::to_string(&movie).map_err(err)?;
        tx.execute("INSERT INTO film(id,title,original_title,popularity,enriched,release_date,runtime,rating,votes,movie_json,updated,data_language)
            VALUES(?1,?2,?3,?4,1,?5,?6,?7,?8,?9,unixepoch(),?10)
            ON CONFLICT(id) DO UPDATE SET title=excluded.title,original_title=excluded.original_title,
            enriched=1,release_date=excluded.release_date,runtime=excluded.runtime,rating=excluded.rating,
            votes=excluded.votes,movie_json=excluded.movie_json,updated=unixepoch(),retry_after=0,data_language=excluded.data_language",
            params![movie.id as i64,movie.title,movie.original_title,movie.popularity,movie.release_date,
              d.runtime.filter(|v|*v>0), if movie.votes>0 {Some(movie.rating)} else {None}, movie.votes,json,language]).map_err(err)?;
        for table in ["genre", "country", "keyword"] {
            tx.execute(
                &format!("DELETE FROM {table} WHERE film_id=?1"),
                [movie.id as i64],
            )
            .map_err(err)?;
        }
        for g in &movie.genre_ids {
            tx.execute(
                "INSERT OR IGNORE INTO genre VALUES(?1,?2)",
                params![movie.id as i64, g],
            )
            .map_err(err)?;
        }
        for code in &d.origin_country_codes {
            tx.execute(
                "INSERT OR IGNORE INTO country VALUES(?1,?2)",
                params![movie.id as i64, code],
            )
            .map_err(err)?;
        }
        for k in &d.keywords {
            tx.execute(
                "INSERT OR IGNORE INTO keyword VALUES(?1,?2)",
                params![movie.id as i64, *k as i64],
            )
            .map_err(err)?;
        }
        tx.execute("DELETE FROM alias WHERE film_id=?1", [movie.id as i64])
            .map_err(err)?;
        for title in &d.alternative_titles {
            if !title.trim().is_empty() {
                tx.execute(
                    "INSERT OR IGNORE INTO alias(film_id,title) VALUES(?1,?2)",
                    params![movie.id as i64, title],
                )
                .map_err(err)?;
            }
        }
        tx.execute(
            "INSERT OR REPLACE INTO alias_version VALUES(?1,1)",
            [movie.id as i64],
        )
        .map_err(err)?;
        tx.commit().map_err(err)
    }
    pub fn save_movement(&self, f: &MovementFact) -> Result<(), String> {
        self.import_movements(std::slice::from_ref(f)).map(|_| ())
    }
    pub fn import_movements(&self, facts: &[MovementFact]) -> Result<u64, String> {
        if facts.len() > 100_000 {
            return Err("Il file supera 100.000 classificazioni.".into());
        }
        let tax = crate::domain::taxonomy::Taxonomy::load();
        for f in facts {
            let url = reqwest::Url::parse(&f.source).map_err(|_| "URL della fonte non valido.")?;
            if f.movie_id == 0
                || f.movie_id > i64::MAX as u64
                || tax.movement(&f.movement).is_none()
                || !matches!(f.relation.as_str(), "documented" | "affinity")
                || url.scheme() != "https"
                || url.host_str().is_none()
                || !url.username().is_empty()
                || f.note.trim().is_empty()
                || f.note.len() > 4000
            {
                return Err("Una classificazione richiede ID valido, corrente riconosciuta, relazione, fonte HTTPS e nota.".into());
            }
        }
        let mut c = self.connect()?;
        let tx = c.transaction().map_err(err)?;
        for f in facts {
            let exists: bool = tx
                .query_row(
                    "SELECT EXISTS(SELECT 1 FROM film WHERE id=?1)",
                    [f.movie_id as i64],
                    |r| r.get(0),
                )
                .map_err(err)?;
            if !exists {
                return Err(format!(
                    "Il film {} non è nell'indice: acquisisci prima gli identificativi.",
                    f.movie_id
                ));
            }
            tx.execute(
                "INSERT OR REPLACE INTO movement VALUES(?1,?2,?3,?4,?5)",
                params![f.movie_id as i64, f.movement, f.relation, f.source, f.note],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)?;
        Ok(facts.len() as u64)
    }
    pub fn install_bundled_sources(&self) -> Result<u64, String> {
        let facts: Vec<MovementFact> =
            serde_json::from_str(include_str!("../../assets/movement-facts.json")).map_err(err)?;
        let mut present = Vec::new();
        for fact in facts {
            if self.movie(fact.movie_id)?.is_some() {
                present.push(fact);
            }
        }
        self.import_movements(&present)
    }
    pub fn set_paused(&self, paused: bool) -> Result<(), String> {
        self.connect()?
            .execute(
                "INSERT OR REPLACE INTO meta VALUES('paused',?1)",
                [if paused { "1" } else { "0" }],
            )
            .map_err(err)?;
        Ok(())
    }
    pub fn is_paused(&self) -> Result<bool, String> {
        Ok(self
            .connect()?
            .query_row("SELECT value='1' FROM meta WHERE key='paused'", [], |r| {
                r.get(0)
            })
            .optional()
            .map_err(err)?
            .unwrap_or(false))
    }
    pub fn known_keywords(&self) -> Result<Vec<(String, u64)>, String> {
        let c = self.connect()?;
        let mut s = c
            .prepare("SELECT substr(key,9),value FROM meta WHERE key LIKE 'keyword:%'")
            .map_err(err)?;
        let rows = s
            .query_map([], |r| {
                let value: String = r.get(1)?;
                Ok((r.get(0)?, value.parse().unwrap_or_default()))
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }
    pub fn save_keywords(&self, known: &[(String, u64)]) -> Result<(), String> {
        let mut c = self.connect()?;
        let tx = c.transaction().map_err(err)?;
        for (name, id) in known {
            tx.execute(
                "INSERT OR REPLACE INTO meta VALUES(?1,?2)",
                params![format!("keyword:{name}"), id.to_string()],
            )
            .map_err(err)?;
        }
        tx.commit().map_err(err)
    }
    pub fn pending(&self, limit: u32) -> Result<Vec<u64>, String> {
        self.pending_in_scope(limit, false, false)
    }
    pub fn pending_in_scope(
        &self,
        limit: u32,
        adult: bool,
        video: bool,
    ) -> Result<Vec<u64>, String> {
        let c = self.connect()?;
        let scope = format!(
            "{} {}",
            if adult { "" } else { "AND adult=0" },
            if video { "" } else { "AND video=0" }
        );
        let sql=format!("SELECT id FROM film WHERE active=1 {scope} AND (enriched=0 OR updated<unixepoch()-2592000 OR NOT EXISTS(SELECT 1 FROM alias_version a WHERE a.film_id=film.id)) AND retry_after<unixepoch() ORDER BY popularity DESC,id LIMIT ?1");
        let mut statement = c.prepare(&sql).map_err(err)?;
        let rows = statement
            .query_map([limit], |r| Ok(r.get::<_, i64>(0)? as u64))
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }
    pub fn defer(&self, id: u64) -> Result<(), String> {
        self.connect()?
            .execute(
                "UPDATE film SET retry_after=unixepoch()+86400 WHERE id=?1",
                [id as i64],
            )
            .map_err(err)?;
        Ok(())
    }
    pub fn needs_metadata(&self, id: u64) -> Result<bool, String> {
        Ok(self
            .connect()?
            .query_row(
                "SELECT enriched=0 OR updated<unixepoch()-2592000 OR NOT EXISTS(SELECT 1 FROM alias_version a WHERE a.film_id=film.id) FROM film WHERE id=?1",
                [id as i64],
                |r| r.get(0),
            )
            .optional()
            .map_err(err)?
            .unwrap_or(true))
    }
    pub fn movie(&self, id: u64) -> Result<Option<Movie>, String> {
        let c = self.connect()?;
        c.query_row(
            "SELECT id,title,original_title,popularity,movie_json FROM film WHERE id=?1",
            [id as i64],
            read_movie,
        )
        .optional()
        .map_err(err)
    }
    pub fn movement_facts(&self, id: u64) -> Result<Vec<MovementFact>, String> {
        let c = self.connect()?;
        let mut s = c
            .prepare("SELECT film_id,value,relation,source,note FROM movement WHERE film_id=?1")
            .map_err(err)?;
        let rows = s
            .query_map([id as i64], |r| {
                Ok(MovementFact {
                    movie_id: r.get::<_, i64>(0)? as u64,
                    movement: r.get(1)?,
                    relation: r.get(2)?,
                    source: r.get(3)?,
                    note: r.get(4)?,
                })
            })
            .map_err(err)?;
        rows.collect::<Result<Vec<_>, _>>().map_err(err)
    }
    pub fn movement_ids(&self, movements: &[String], documented: bool) -> Result<Vec<u64>, String> {
        let c = self.connect()?;
        let mut ids = std::collections::BTreeSet::new();
        let mut s=c.prepare("SELECT DISTINCT film_id FROM movement WHERE value=?1 AND (?2=0 OR relation='documented')").map_err(err)?;
        for movement in movements {
            let rows = s
                .query_map(params![movement, documented], |r| {
                    Ok(r.get::<_, i64>(0)? as u64)
                })
                .map_err(err)?;
            for row in rows {
                ids.insert(row.map_err(err)?);
            }
        }
        Ok(ids.into_iter().collect())
    }
}

pub(crate) fn read_movie(r: &rusqlite::Row<'_>) -> rusqlite::Result<Movie> {
    let json: Option<String> = r.get(4)?;
    let mut m = if let Some(json) = json {
        serde_json::from_str(&json).map_err(|e| {
            rusqlite::Error::FromSqlConversionFailure(4, rusqlite::types::Type::Text, Box::new(e))
        })?
    } else {
        Movie::default()
    };
    m.id = r.get::<_, i64>(0)? as u64;
    m.title = r.get(1)?;
    m.original_title = r.get(2)?;
    m.popularity = r.get(3)?;
    Ok(m)
}
fn stage_batch(c: &mut Connection, rows: &[ExportIdentity]) -> Result<(), String> {
    let tx = c.transaction().map_err(err)?;
    {
        let mut s = tx
            .prepare_cached("INSERT OR REPLACE INTO staging VALUES(?1,?2,?3,?4,?5)")
            .map_err(err)?;
        for r in rows {
            s.execute(params![
                r.id as i64,
                r.original_title,
                r.popularity,
                r.adult,
                r.video
            ])
            .map_err(err)?;
        }
    }
    tx.commit().map_err(err)
}
