#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sort {
    Popularity,
    Rating,
    Votes,
    Newest,
    Oldest,
}

impl Sort {
    pub const ALL: [Sort; 5] = [
        Sort::Popularity,
        Sort::Rating,
        Sort::Votes,
        Sort::Newest,
        Sort::Oldest,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Sort::Popularity => "Popolarita",
            Sort::Rating => "Voto",
            Sort::Votes => "Numero di voti",
            Sort::Newest => "Piu recenti",
            Sort::Oldest => "Piu vecchi",
        }
    }

    pub fn as_tmdb(self) -> &'static str {
        match self {
            Sort::Popularity => "popularity.desc",
            Sort::Rating => "vote_average.desc",
            Sort::Votes => "vote_count.desc",
            Sort::Newest => "primary_release_date.desc",
            Sort::Oldest => "primary_release_date.asc",
        }
    }
}

impl Default for Sort {
    fn default() -> Self {
        Sort::Popularity
    }
}

/// Stato delle facce selezionate dall'utente.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Facets {
    pub decades: Vec<String>,
    pub movements: Vec<String>,
    pub genres: Vec<u32>,
    pub without_genres: Vec<u32>,
    pub countries: Vec<String>,
    pub moods: Vec<String>,
    /// Temi liberi (nomi di parole chiave TMDB) proposti dall'interprete AI
    /// quando la richiesta parla di un argomento che non e un mood della
    /// tassonomia. Restano visibili e rimovibili come i mood.
    pub keywords: Vec<String>,
    pub min_rating: Option<f32>,
    pub max_rating: Option<f32>,
    /// Numero minimo di voti TMDB: un filtro di sostanza (evita i titoli
    /// con due voti) e il modo naturale per dire «ben votato».
    pub min_votes: Option<u32>,
    pub runtime: Option<(u32, u32)>,
    pub sort: Sort,
}

impl Facets {
    pub fn is_empty(&self) -> bool {
        self.decades.is_empty()
            && self.movements.is_empty()
            && self.genres.is_empty()
            && self.without_genres.is_empty()
            && self.countries.is_empty()
            && self.moods.is_empty()
            && self.keywords.is_empty()
            && self.min_rating.is_none()
            && self.max_rating.is_none()
            && self.min_votes.is_none()
            && self.runtime.is_none()
    }

    pub fn active_count(&self) -> usize {
        self.decades.len()
            + self.movements.len()
            + self.genres.len()
            + self.without_genres.len()
            + self.countries.len()
            + self.moods.len()
            + self.keywords.len()
            + usize::from(self.min_rating.is_some())
            + usize::from(self.max_rating.is_some())
            + usize::from(self.min_votes.is_some())
            + usize::from(self.runtime.is_some())
    }
}

pub fn toggle<T: PartialEq>(list: &mut Vec<T>, item: T) {
    if let Some(pos) = list.iter().position(|x| x == &item) {
        list.remove(pos);
    } else {
        list.push(item);
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrackState {
    pub watched: bool,
    pub watchlist: bool,
    pub rating: Option<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct Movie {
    pub id: u64,
    pub title: String,
    pub original_title: String,
    pub year: Option<u16>,
    pub overview: String,
    pub poster_path: Option<String>,
    pub backdrop_path: Option<String>,
    pub rating: f32,
    pub votes: u32,
    pub popularity: f32,
    pub genre_ids: Vec<u32>,
    pub lang: String,
    /// Identificativo IMDb (tconst), per il voto IMDb dai dati pubblici.
    pub imdb_id: Option<String>,
    /// Stato locale SIMKL per la resa delle card.
    pub track: TrackState,
}

/// Voto IMDb letto dai dati pubblici (title.ratings.tsv di datasets.imdbws.com).
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ImdbRating {
    pub rating: f32,
    pub votes: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Details {
    pub movie: Movie,
    pub tagline: String,
    pub runtime: Option<u32>,
    pub genres: Vec<(u32, String)>,
    pub cast: Vec<String>,
    pub director: Option<String>,
    pub flatrate: Vec<String>,
    pub rent: Vec<String>,
    pub buy: Vec<String>,
    pub provider_link: Option<String>,
    pub countries: Vec<String>,
    pub recommendations: Vec<Movie>,
    pub imdb: Option<ImdbRating>,
}
