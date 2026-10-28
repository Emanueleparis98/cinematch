use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::infra::secrets;

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
            "openrouter"
                if self.openrouter_api_key.trim().is_empty()
                    && looks_like_openrouter(&self.api_key) =>
            {
                self.openrouter_api_key = self.api_key.clone();
                changed = true;
            }
            _ => {}
        }
        changed
    }
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SimklCfg {
    /// Client ID pubblico dell'app registrata su Simkl.
    #[serde(default)]
    pub client_id: String,
    /// Nasconde i film già visti in griglia e chat.
    #[serde(default)]
    pub hide_watched: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Config {
    #[serde(default)]
    pub tmdb_api_key: String,
    #[serde(default = "default_language")]
    pub language: String,
    #[serde(default)]
    pub ai: AiCfg,
    #[serde(default)]
    pub simkl: SimklCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tmdb_api_key: String::new(),
            language: default_language(),
            ai: AiCfg::default(),
            simkl: SimklCfg::default(),
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
        Self::load_at(&Self::path(), true)
    }

    fn load_at(path: &Path, use_keyring: bool) -> Self {
        match std::fs::read_to_string(path) {
            Ok(raw) => {
                let raw = raw.trim_start_matches('\u{feff}');
                let mut recovered = false;
                let mut cfg: Self = match serde_json::from_str(raw) {
                    Ok(cfg) => cfg,
                    Err(_) => match Self::recover(path) {
                        Some(cfg) => {
                            recovered = true;
                            cfg
                        }
                        // Keep the unreadable file: do not write defaults over it.
                        None => {
                            let mut cfg = Self::default();
                            if use_keyring && secrets::available() {
                                cfg.absorb_secrets();
                            }
                            return cfg;
                        }
                    },
                };
                let mut rewritten = recovered | cfg.ai.migrate();
                if use_keyring && secrets::available() {
                    let secrets_migrated = cfg.absorb_secrets();
                    rewritten |= secrets_migrated;
                    if rewritten {
                        let saved = cfg.save_at(path, true).is_ok();
                        // Old versions kept secrets in this file, and the
                        // backup would keep a copy: drop it only after the
                        // rewrite succeeded.
                        if secrets_migrated && saved {
                            let _ = std::fs::remove_file(backup_path(path));
                        }
                    }
                } else if rewritten {
                    let _ = cfg.save_at(path, false);
                }
                cfg
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let mut cfg = Config::default();
                if use_keyring && secrets::available() {
                    cfg.absorb_secrets();
                }
                let _ = cfg.save_at(path, use_keyring);
                cfg
            }
            // Lock, permission error or transient read failure: keep the file.
            Err(_) => {
                let mut cfg = Config::default();
                if use_keyring && secrets::available() {
                    cfg.absorb_secrets();
                }
                cfg
            }
        }
    }

    /// Previous good copy, used when the primary file is not valid JSON.
    fn recover(path: &Path) -> Option<Self> {
        let raw = std::fs::read_to_string(backup_path(path)).ok()?;
        serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok()
    }

    /// Moves plaintext keys from `config.json` into the credential store.
    /// Returns true when the file must be rewritten without them.
    fn absorb_secrets(&mut self) -> bool {
        let mut migrated = false;
        let mut move_key = |name: &str, value: &mut String| {
            let had_plaintext = !value.trim().is_empty();
            if let Some(stored) = secrets::load(name) {
                *value = stored;
            }
            if had_plaintext {
                let _ = secrets::save(name, value);
                migrated = true;
            }
        };
        move_key(secrets::TMDB, &mut self.tmdb_api_key);
        move_key(secrets::DEEPSEEK, &mut self.ai.api_key);
        move_key(secrets::OPENROUTER, &mut self.ai.openrouter_api_key);
        migrated
    }

    pub fn save(&self) -> Result<(), String> {
        self.save_at(&Self::path(), true)
    }

    fn save_at(&self, path: &Path, use_keyring: bool) -> Result<(), String> {
        let dir = path.parent().unwrap_or_else(|| Path::new("."));
        std::fs::create_dir_all(dir).map_err(|e| format!("Impossibile creare {dir:?}: {e}"))?;
        let mut disk = self.clone();
        if use_keyring && secrets::available() {
            secrets::save(secrets::TMDB, &self.tmdb_api_key)?;
            secrets::save(secrets::DEEPSEEK, &self.ai.api_key)?;
            secrets::save(secrets::OPENROUTER, &self.ai.openrouter_api_key)?;
            disk.tmdb_api_key.clear();
            disk.ai.api_key.clear();
            disk.ai.openrouter_api_key.clear();
        }
        let raw = serde_json::to_string_pretty(&disk).map_err(|e| e.to_string())?;
        let tmp = tmp_path(path);
        std::fs::write(&tmp, raw).map_err(|e| format!("Impossibile salvare: {e}"))?;
        // Only copy a file that is still valid JSON: a corrupt primary must
        // not replace the last good backup.
        if path.exists()
            && std::fs::read(path)
                .ok()
                .and_then(|b| serde_json::from_slice::<Self>(&b).ok())
                .is_some()
        {
            let _ = std::fs::copy(path, backup_path(path));
        }
        std::fs::rename(&tmp, path).map_err(|e| format!("Impossibile salvare: {e}"))
    }

    pub fn has_key(&self) -> bool {
        !self.tmdb_api_key.trim().is_empty()
    }
}

fn tmp_path(path: &Path) -> PathBuf {
    path.with_extension("json.tmp")
}

fn backup_path(path: &Path) -> PathBuf {
    path.with_extension("json.bak")
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

    fn temp_file(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "cinematch_cfg_{}_{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        dir.join("config.json")
    }

    #[test]
    fn un_file_corrotto_non_viene_sovrascritto() {
        let path = temp_file("corrotto");
        std::fs::write(&path, "{ rotto").expect("scrittura");
        let cfg = super::Config::load_at(&path, false);
        assert!(cfg.tmdb_api_key.is_empty());
        assert_eq!(std::fs::read_to_string(&path).unwrap(), "{ rotto");
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn il_backup_recupera_il_file_corrotto() {
        let path = temp_file("backup");
        let mut cfg = super::Config {
            tmdb_api_key: "chiave".into(),
            ..Default::default()
        };
        cfg.save_at(&path, false).expect("salvataggio");
        // A second save creates the backup of the first good file.
        cfg.tmdb_api_key = "seconda".into();
        cfg.save_at(&path, false).expect("secondo salvataggio");
        std::fs::write(&path, "{ rotto").expect("corruzione");
        let recovered = super::Config::load_at(&path, false);
        assert_eq!(recovered.tmdb_api_key, "chiave");
        // The primary file is restored from the backup.
        let raw = std::fs::read_to_string(&path).unwrap();
        assert!(serde_json::from_str::<super::Config>(&raw).is_ok());
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }

    #[test]
    fn il_salvataggio_e_atomico_e_lascia_il_backup() {
        let path = temp_file("atomico");
        let mut cfg = super::Config {
            tmdb_api_key: "prima".into(),
            ..Default::default()
        };
        cfg.save_at(&path, false).expect("primo salvataggio");
        cfg.tmdb_api_key = "seconda".into();
        cfg.save_at(&path, false).expect("secondo salvataggio");
        let tmp = super::tmp_path(&path);
        assert!(!tmp.exists(), "nessun file temporaneo rimasto");
        let bak = std::fs::read_to_string(super::backup_path(&path)).expect("backup");
        assert!(bak.contains("prima"));
        let current = std::fs::read_to_string(&path).expect("corrente");
        assert!(current.contains("seconda"));
        let _ = std::fs::remove_dir_all(path.parent().unwrap());
    }
}
