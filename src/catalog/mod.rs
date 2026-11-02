//! Online catalog: no movie database or disk cache.
mod online;
use crate::domain::model::Facets;
pub use online::{browse, plans, CatalogPage};
use serde::{Deserialize, Serialize};
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ReleaseScope {
    #[default]
    All,
    Released,
    Upcoming,
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
    pub years: Option<(u16, u16)>,
    pub page: u32,
}
impl CatalogQuery {
    pub fn scope_filters(&self) -> usize {
        usize::from(self.release != ReleaseScope::All)
            + usize::from(self.include_adult)
            + usize::from(self.include_video)
            + usize::from(self.years.is_some())
    }
}
#[derive(Deserialize, Serialize, Clone, Debug, PartialEq, Eq)]
pub struct MovementFact {
    pub movie_id: u64,
    pub movement: String,
    pub relation: String,
    pub source: String,
    pub note: String,
}
pub fn bundled_sources() -> Vec<MovementFact> {
    serde_json::from_str(include_str!("../../assets/movement-facts.json"))
        .expect("validated bundled classifications")
}
