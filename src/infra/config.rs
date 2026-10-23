use std::path::PathBuf;

use serde::{Deserialize, Serialize};

fn default_provider() -> String {
    "rules".to_string()
}

fn default_language() -> String {
    "it-IT".to_string()
}

fn default_effort() -> String {
    "max".to_string()
}

fn looks_like_openrouter(key: &str) -> bool {
    key.trim().starts_with("sk-or-")
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct AiCfg {
    /// "rules" | "deepseek" | "openrouter" | "ollama"
    #[serde(default = "default_provider")]
    pub provider: String,
    /// Collegamento Ollama (le API cloud hanno un indirizzo fisso).
    #[serde(default)]
    pub base_url: String,
    /// Modello DeepSeek; per gli altri provider si usano i campi dedicati.
    #[serde(default)]
    pub model: String,
    /// Chiave DeepSeek.
    #[serde(default)]
    pub api_key: String,
    #[serde(default = "default_effort")]
    pub effort: String,
    #[serde(default)]
    pub openrouter_api_key: String,
    #[serde(default)]
    pub openrouter_model: String,
    #[serde(default)]
    pub ollama_model: String,
}

impl Default for AiCfg {
    fn default() -> Self {
        Self {
            provider: default_provider(),
            base_url: String::new(),
            model: "deepseek-flash".to_string(),
            api_key: String::new(),
            effort: default_effort(),
            openrouter_api_key: String::new(),
            openrouter_model: String::new(),
            ollama_model: String::new(),
        }
    }
}

impl AiCfg {
    /// Converte le vecchie configurazioni "openai" e "ollama" nei nuovi campi.
    /// Ritorna true se il file va riscritto.
    pub fn migrate(&mut self) -> bool {
        let mut changed = false;
        if self.effort.trim().is_empty() {
            self.effort = default_effort();
            changed = true;
        }
        match self.provider.as_str() {
            "openai" => {
                let base = self.base_url.to_lowercase();
                let model = self.model.to_lowercase();
                let openrouter = base.contains("openrouter") || model.starts_with("deepseek/");
                let deepseek = !openrouter
                    && (model.contains("deepseek") || base.contains("deepseek"));
                if deepseek {
                    self.provider = "deepseek".into();
                    self.model = "deepseek-flash".into();
                } else {
                    self.provider = "openrouter".into();
                    if looks_like_openrouter(&self.api_key) {
                        self.openrouter_api_key = self.api_key.clone();
                    }
                    self.openrouter_model = self.model.clone();
                    self.api_key.clear();
                    self.model = "deepseek-flash".into();
                }
                self.base_url.clear();
                changed = true;
            }
            "ollama" => {
                if self.ollama_model.is_empty() && !self.model.is_empty() {
                    self.ollama_model = self.model.clone();
                }
                if self.base_url.trim().is_empty() {
                    self.base_url = "http://localhost:11434".into();
                    changed = true;
                }
                if self.model != "deepseek-flash" {
                    self.model = "deepseek-flash".into();
                    changed = true;
                }
            }
            "deepseek" => {
                if self.model.trim().is_empty() || self.model == "deepseek-v4.1-flash" {
                    self.model = "deepseek-flash".into();
                    changed = true;
                }
            }
            "openrouter" => {
                if self.openrouter_api_key.trim().is_empty() && looks_like_openrouter(&self.api_key) {
                    self.openrouter_api_key = self.api_key.clone();
                    changed = true;
                }
            }
            _ => {}
        }
        changed
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
            Ok(raw) => {
                let raw = raw.trim_start_matches('\u{feff}');
                let mut cfg: Self = serde_json::from_str(raw).unwrap_or_default();
                if cfg.ai.migrate() {
                    let _ = cfg.save();
                }
                cfg
            }
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

#[cfg(test)]
mod tests {
    use super::AiCfg;

    #[test]
    fn la_vecchia_configurazione_deepseek_passa_alla_api_diretta() {
        let mut ai = AiCfg {
            provider: "openai".into(),
            base_url: "https://proxy.example/v1".into(),
            model: "deepseek-v4.1-flash".into(),
            api_key: "k".into(),
            ..Default::default()
        };
        assert!(ai.migrate());
        assert_eq!(ai.provider, "deepseek");
        assert_eq!(ai.model, "deepseek-flash");
        assert_eq!(ai.api_key, "k");
    }

    #[test]
    fn la_vecchia_configurazione_openai_non_deepseek_passa_a_openrouter() {
        let mut ai = AiCfg {
            provider: "openai".into(),
            base_url: "https://api.openai.com/v1".into(),
            model: "gpt-test".into(),
            api_key: "sk-or-test".into(),
            ..Default::default()
        };
        assert!(ai.migrate());
        assert_eq!(ai.provider, "openrouter");
        assert_eq!(ai.openrouter_model, "gpt-test");
        assert_eq!(ai.openrouter_api_key, "sk-or-test");
    }

    #[test]
    fn un_modello_con_slash_resta_su_openrouter() {
        let mut ai = AiCfg {
            provider: "openai".into(),
            model: "deepseek/deepseek-v4.1-flash-1017".into(),
            api_key: "sk-or-test".into(),
            ..Default::default()
        };
        assert!(ai.migrate());
        assert_eq!(ai.provider, "openrouter");
        assert_eq!(ai.openrouter_model, "deepseek/deepseek-v4.1-flash-1017");
    }

    #[test]
    fn una_chiave_deepseek_non_finisce_su_openrouter() {
        let mut ai = AiCfg {
            provider: "openrouter".into(),
            api_key: "sk-deepseek".into(),
            ..Default::default()
        };
        ai.migrate();
        assert!(ai.openrouter_api_key.is_empty());
    }
}
