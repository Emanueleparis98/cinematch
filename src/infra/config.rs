use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn default_provider() -> String {
    "rules".to_string()
}

fn default_language() -> String {
    "it-IT".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiCfg {
    /// "rules" | "openai" | "ollama"
    #[serde(default = "default_provider")]
    pub provider: String,
    #[serde(default)]
    pub base_url: String,
    #[serde(default)]
    pub model: String,
    #[serde(default)]
    pub api_key: String,
}

impl Default for AiCfg {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            base_url: String::new(),
            model: String::new(),
            api_key: String::new(),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub tmdb_api_key: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub ai: AiCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tmdb_api_key: String::new(),
            language: default_language(),
            ai: AiCfg::default(),
        }
    }
}

impl Config {
    pub fn dir() -> PathBuf {
        let base = std::env::var("APPDATA").unwrap_or_else(|_| ".".to_string());
        PathBuf::from(base).join("CineMatch")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("config.json")
    }

    pub fn load() -> Self {
        match std::fs::read_to_string(Self::path()) {
            Ok(raw) => serde_json::from_str(&raw).unwrap_or_default(),
            Err(_) => {
                let cfg = Config::default();
                let _ = cfg.save();
                cfg
            }
        }
    }

    pub fn save(&self) -> Result<(), String> {
        let dir = Self::dir();
        std::fs::create_dir_all(&dir).map_err(|e| format!("Impossibile creare {dir:?}: {e}"))?;
        let raw = serde_json::to_string_pretty(self).map_err(|e| e.to_string())?;
        std::fs::write(Self::path(), raw).map_err(|e| format!("Impossibile salvare: {e}"))
    }

    pub fn has_key(&self) -> bool {
        !self.tmdb_api_key.trim().is_empty()
    }
}
