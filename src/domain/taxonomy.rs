use std::collections::HashMap;

use serde::Deserialize;

const TAXONOMY_JSON: &str = include_str!("../../assets/taxonomy.json");

#[derive(Deserialize, Clone, Debug)]
pub struct Decade {
    pub id: String,
    pub label: String,
    pub from: u16,
    pub to: u16,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Genre {
    pub id: u32,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Country {
    pub code: String,
    pub name: String,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Mood {
    pub id: String,
    pub label: String,
    pub keyword: String,
    #[serde(default)]
    pub incompatible_movements: Vec<String>,
}

#[derive(Deserialize, Clone, Debug)]
pub struct Movement {
    pub id: String,
    pub label: String,
    pub period: [u16; 2],
    pub countries: Vec<String>,
    /// Parola chiave TMDB che identifica la corrente (es. «slasher»), quando
    /// la comunita TMDB la cura bene: la ricerca la usa al posto della geometria.
    #[serde(default)]
    pub keyword: String,
    #[serde(default)]
    pub genres_core: Vec<u32>,
    #[serde(default)]
    pub genres_allowed: Vec<u32>,
    #[serde(default)]
    pub genres_forbidden: Vec<u32>,
    #[serde(default)]
    pub note: String,
}

#[derive(Default, Clone, Debug)]
struct Index {
    decades: HashMap<String, usize>,
    movements: HashMap<String, usize>,
    genres: HashMap<u32, String>,
    countries: HashMap<String, String>,
    moods: HashMap<String, usize>,
}

#[derive(Deserialize, Clone, Debug, Default)]
pub struct Taxonomy {
    pub decades: Vec<Decade>,
    pub genres: Vec<Genre>,
    pub countries: Vec<Country>,
    pub moods: Vec<Mood>,
    pub movements: Vec<Movement>,
    #[serde(skip)]
    index: Index,
}

impl Taxonomy {
    pub fn load() -> Self {
        let mut tax: Taxonomy =
            serde_json::from_str(TAXONOMY_JSON).expect("taxonomy.json non valido");
        let countries: Vec<Country> =
            serde_json::from_str(include_str!("../../assets/countries.json"))
                .expect("countries.json non valido");
        for country in countries {
            if !tax.countries.iter().any(|c| c.code == country.code) {
                tax.countries.push(country);
            }
        }
        tax.countries.sort_by(|a, b| a.name.cmp(&b.name));
        for from in (1870..=2090).step_by(10) {
            if !tax.decades.iter().any(|d| d.from == from) {
                tax.decades.push(Decade {
                    id: format!("{from}s"),
                    label: format!("Anni {from}"),
                    from,
                    to: from + 9,
                });
            }
        }
        tax.decades.sort_by_key(|d| d.from);
        tax.build_index();
        tax
    }

    fn build_index(&mut self) {
        let mut ix = Index::default();
        for (i, d) in self.decades.iter().enumerate() {
            ix.decades.insert(d.id.clone(), i);
        }
        for (i, m) in self.movements.iter().enumerate() {
            ix.movements.insert(m.id.clone(), i);
        }
        for g in &self.genres {
            ix.genres.insert(g.id, g.name.clone());
        }
        for c in &self.countries {
            ix.countries.insert(c.code.clone(), c.name.clone());
        }
        for (i, m) in self.moods.iter().enumerate() {
            ix.moods.insert(m.id.clone(), i);
        }
        self.index = ix;
    }

    pub fn decade(&self, id: &str) -> Option<&Decade> {
        self.index.decades.get(id).map(|i| &self.decades[*i])
    }

    pub fn movement(&self, id: &str) -> Option<&Movement> {
        self.index.movements.get(id).map(|i| &self.movements[*i])
    }

    pub fn mood(&self, id: &str) -> Option<&Mood> {
        self.index.moods.get(id).map(|i| &self.moods[*i])
    }

    pub fn genre_name(&self, id: u32) -> Option<String> {
        self.index.genres.get(&id).cloned()
    }

    pub fn country_name(&self, code: &str) -> Option<String> {
        self.index.countries.get(code).cloned()
    }

    pub fn country_label(&self, code: &str) -> String {
        self.country_name(code).unwrap_or_else(|| code.to_string())
    }
}
