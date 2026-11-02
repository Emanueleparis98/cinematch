use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::infra::secrets;

fn default_language() -> String {
    "it-IT".to_string()
}

#[derive(Serialize, Deserialize, Clone, Debug, Default)]
pub struct SimklCfg {
    /// Client ID pubblico dell'app registrata su Simkl.
    #[serde(default)]
    pub client_id: String,
    /// Nasconde i film già visti nel catalogo.
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
    pub simkl: SimklCfg,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            tmdb_api_key: String::new(),
            language: default_language(),
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
                let mut rewritten = recovered
                    || serde_json::from_str::<serde_json::Value>(raw)
                        .ok()
                        .is_some_and(|v| v.get("ai").is_some());
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
            // Riprova una volta, poi usa l'ultima copia buona invece dei
            // default: altrimenti il prossimo salvataggio scriverebbe i
            // default sopra il file vero.
            Err(_) => {
                std::thread::sleep(std::time::Duration::from_millis(100));
                let mut cfg = std::fs::read_to_string(path)
                    .ok()
                    .and_then(|raw| serde_json::from_str(raw.trim_start_matches('\u{feff}')).ok())
                    .or_else(|| Self::recover(path))
                    .unwrap_or_default();
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
            disk.tmdb_api_key.clear();
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
    #[test]
    fn la_configurazione_precedente_perde_i_provider_e_conserva_catalogo_e_simkl() {
        let path = temp_file("catalog_migration");
        std::fs::write(&path,r#"{"tmdb_api_key":"fixture-tmdb","language":"en-US","ai":{"provider":"retired","api_key":"fixture-obsolete"},"simkl":{"client_id":"fixture-simkl","hide_watched":true}}"#).unwrap();
        let cfg = super::Config::load_at(&path, false);
        assert_eq!(cfg.tmdb_api_key, "fixture-tmdb");
        assert_eq!(cfg.language, "en-US");
        assert!(cfg.simkl.hide_watched);
        assert_eq!(cfg.simkl.client_id, "fixture-simkl");
        let saved: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&path).unwrap()).unwrap();
        assert!(saved.get("ai").is_none());
        assert!(!saved.to_string().contains("fixture-obsolete"));
    }

    fn temp_file(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cinematch_cfg_{}_{name}", std::process::id()));
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
