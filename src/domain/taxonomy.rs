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

    /// Riassunto compatto della tassonomia, usato nel prompt per il modello AI.
    pub fn prompt_summary(&self) -> String {
        let mut s = String::new();
        s.push_str("MOVIMENTI (id | etichetta | periodo | paesi):\n");
        for m in &self.movements {
            s.push_str(&format!(
                "- {} | {} | {}-{} | {}\n",
                m.id,
                m.label,
                m.period[0],
                m.period[1],
                m.countries.join(",")
            ));
        }
        s.push_str("\nDECENNI: ");
        s.push_str(
            &self
                .decades
                .iter()
                .map(|d| d.id.clone())
                .collect::<Vec<_>>()
                .join(", "),
        );
        s.push_str("\n\nGENERI (id=nome): ");
        s.push_str(
            &self
                .genres
                .iter()
                .map(|g| format!("{}={}", g.id, g.name))
                .collect::<Vec<_>>()
                .join(", "),
        );
        s.push_str("\n\nPAESI (codice): ");
        s.push_str(
            &self
                .countries
                .iter()
                .map(|c| c.code.clone())
                .collect::<Vec<_>>()
                .join(", "),
        );
        s.push_str("\n\nMOOD (id): ");
        s.push_str(
            &self
                .moods
                .iter()
                .map(|m| m.id.clone())
                .collect::<Vec<_>>()
                .join(", "),
        );
        s
    }
}
