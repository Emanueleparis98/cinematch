use std::collections::HashMap;

use serde::Deserialize;

use crate::domain::model::{Details, Movie};

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ApiMovie {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub original_title: String,
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub poster_path: Option<String>,
    #[serde(default)]
    pub backdrop_path: Option<String>,
    #[serde(default)]
    pub vote_average: f32,
    #[serde(default)]
    pub vote_count: u32,
    #[serde(default)]
    pub popularity: f32,
    #[serde(default)]
    pub genre_ids: Vec<u32>,
    #[serde(default)]
    pub original_language: String,
}

pub fn parse_year(date: &str) -> Option<u16> {
    let bytes = date.as_bytes();
    if bytes.len() < 4 || !bytes[..4].iter().all(u8::is_ascii_digit) {
        return None;
    }
    date[..4].parse::<u16>().ok()
}

impl From<ApiMovie> for Movie {
    fn from(a: ApiMovie) -> Self {
        Movie {
            year: parse_year(&a.release_date),
            id: a.id,
            title: a.title,
            original_title: a.original_title,
            overview: a.overview,
            poster_path: a.poster_path,
            backdrop_path: a.backdrop_path,
            rating: a.vote_average,
            votes: a.vote_count,
            popularity: a.popularity,
            genre_ids: a.genre_ids,
            lang: a.original_language,
            imdb_id: None,
        }
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct PagedMovies {
    #[serde(default)]
    pub page: u32,
    #[serde(default)]
    pub results: Vec<ApiMovie>,
    #[serde(default)]
    pub total_pages: u32,
    #[serde(default)]
    pub total_results: u32,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct GenreRef {
    #[serde(default)]
    pub id: u32,
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct CastMember {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub character: String,
    #[serde(default)]
    pub order: u32,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct CrewMember {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub job: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Credits {
    #[serde(default)]
    pub cast: Vec<CastMember>,
    #[serde(default)]
    pub crew: Vec<CrewMember>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Provider {
    #[serde(default)]
    pub provider_name: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct WatchRegion {
    #[serde(default)]
    pub link: Option<String>,
    #[serde(default)]
    pub flatrate: Vec<Provider>,
    #[serde(default)]
    pub rent: Vec<Provider>,
    #[serde(default)]
    pub buy: Vec<Provider>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct WatchProviders {
    #[serde(default)]
    pub results: HashMap<String, WatchRegion>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct CountryRef {
    #[serde(default)]
    pub iso_3166_1: String,
    #[serde(default)]
    pub name: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
struct KeywordItem {
    #[serde(default)]
    id: u64,
    #[serde(default)]
    name: String,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct KeywordResults {
    #[serde(default)]
    results: Vec<KeywordItem>,
}

impl KeywordResults {
    /// Corrispondenza esatta soltanto: il primo risultato fuzzy di TMDB
    /// («cani» -> «cali») produrrebbe film senza alcun rapporto col tema.
    pub fn exact(&self, query: &str) -> Option<u64> {
        let q = query.trim().to_lowercase();
        self.results
            .iter()
            .find(|k| k.name.trim().to_lowercase() == q)
            .map(|k| k.id)
    }
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ExternalIds {
    #[serde(default)]
    pub imdb_id: Option<String>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct ApiDetails {
    #[serde(default)]
    pub id: u64,
    #[serde(default)]
    pub title: String,
    #[serde(default)]
    pub original_title: String,
    #[serde(default)]
    pub overview: String,
    #[serde(default)]
    pub tagline: Option<String>,
    #[serde(default)]
    pub release_date: String,
    #[serde(default)]
    pub poster_path: Option<String>,
    #[serde(default)]
    pub backdrop_path: Option<String>,
    #[serde(default)]
    pub vote_average: f32,
    #[serde(default)]
    pub vote_count: u32,
    #[serde(default)]
    pub runtime: Option<u32>,
    #[serde(default)]
    pub original_language: String,
    #[serde(default)]
    pub genres: Vec<GenreRef>,
    #[serde(default)]
    pub production_countries: Vec<CountryRef>,
    #[serde(default)]
    pub credits: Option<Credits>,
    #[serde(rename = "watch/providers", default)]
    pub watch_providers: Option<WatchProviders>,
    #[serde(default)]
    pub external_ids: Option<ExternalIds>,
    #[serde(default)]
    pub recommendations: Option<PagedMovies>,
    #[serde(default)]
    pub status: String,
}

impl ApiDetails {
    pub fn into_details(self, region: &str) -> Details {        let genre_ids = self.genres.iter().map(|g| g.id).collect::<Vec<u32>>();
        let imdb_id = self
            .external_ids
            .as_ref()
            .and_then(|e| e.imdb_id.clone())
            .filter(|s| !s.trim().is_empty());
        let movie = Movie {
            id: self.id,
            title: self.title.clone(),
            original_title: self.original_title.clone(),
            year: parse_year(&self.release_date),
            overview: self.overview.clone(),
            poster_path: self.poster_path.clone(),
            backdrop_path: self.backdrop_path.clone(),
            rating: self.vote_average,
            votes: self.vote_count,
            popularity: 0.0,
            genre_ids,
            lang: self.original_language.clone(),
            imdb_id: imdb_id.clone(),
        };
        let mut cast = Vec::new();
        let mut director = None;
        if let Some(c) = &self.credits {
            let mut members = c.cast.clone();
            members.sort_by_key(|m| m.order);
            cast = members.into_iter().take(8).map(|m| m.name).collect();
            director = c
                .crew
                .iter()
                .find(|m| m.job.eq_ignore_ascii_case("Director"))
                .map(|m| m.name.clone());
        }
        let mut flatrate = Vec::new();
        let mut rent = Vec::new();
        let mut buy = Vec::new();
        let mut provider_link = None;
        if let Some(wp) = &self.watch_providers {
            if let Some(r) = wp.results.get(region) {
                provider_link = r.link.clone();
                flatrate = r.flatrate.iter().map(|p| p.provider_name.clone()).collect();
                rent = r.rent.iter().map(|p| p.provider_name.clone()).collect();
                buy = r.buy.iter().map(|p| p.provider_name.clone()).collect();
            }
        }
        Details {
            movie,
            tagline: self.tagline.clone().unwrap_or_default(),
            runtime: self.runtime,
            genres: self.genres.iter().map(|g| (g.id, g.name.clone())).collect(),
            cast,
            director,
            flatrate,
            rent,
            buy,
            provider_link,
            countries: self
                .production_countries
                .iter()
                .map(|c| c.name.clone())
                .collect(),
            recommendations: self
                .recommendations
                .map(|p| p.results.into_iter().map(Movie::from).collect())
                .unwrap_or_default(),
            imdb: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{parse_year, KeywordItem, KeywordResults};

    #[test]
    fn la_keyword_deve_corrispondere_esattamente() {
        let res = KeywordResults {
            results: vec![KeywordItem {
                id: 7,
                name: "cali".into(),
            }],
        };
        assert_eq!(res.exact("cani"), None, "niente primo risultato fuzzy");
        assert_eq!(res.exact("Cali"), Some(7));
    }

    #[test]
    fn l_anno_si_legge_solo_da_quattro_cifre_ascii() {
        assert_eq!(parse_year("2024-05-01"), Some(2024));
        assert_eq!(parse_year("1999"), Some(1999));
        assert_eq!(parse_year("20é4-05-01"), None);
        assert_eq!(parse_year("🎬2024"), None);
        assert_eq!(parse_year(""), None);
    }
}
