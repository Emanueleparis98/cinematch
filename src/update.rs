//! Aggiornamenti: legge il manifesto dell'ultima release, scarica il setup
//! completo e lo avvia. Niente patch binarie: l'installer per-utente conserva
//! configurazione e dati locali.

use std::fs::File;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Deserialize;
use sha2::{Digest, Sha256};

const MANIFEST_URL: &str =
    "https://github.com/Emanueleparis98/cinematch/releases/latest/download/latest.json";
const DOWNLOAD_PREFIX: &str =
    "https://github.com/Emanueleparis98/cinematch/releases/download/";
const MAX_INSTALLER: u64 = 200 * 1024 * 1024;

#[derive(Clone, Debug, Deserialize)]
pub struct Installer {
    pub url: String,
    pub sha256: String,
    pub size: u64,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Manifest {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    #[serde(default)]
    pub notes_url: String,
    pub installer: Installer,
}

pub fn current_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

/// Gli aggiornamenti valgono solo per la copia installata. In sviluppo
/// (`cargo run`, `run.ps1`, target) l'updater non deve sostituire il binario
/// che si sta modificando.
pub fn enabled() -> bool {
    if cfg!(debug_assertions) || std::env::var_os("CINEMATCH_NO_UPDATE").is_some() {
        return false;
    }
    let Ok(exe) = std::env::current_exe() else {
        return false;
    };
    let path = exe.to_string_lossy().replace('/', "\\").to_ascii_lowercase();
    !path.contains("\\target\\")
}

/// Chiede il manifesto. `Ok(None)` significa: nessuna release o gia aggiornati.
pub fn check() -> Result<Option<Manifest>, String> {
    let client = client()?;
    let response = client
        .get(MANIFEST_URL)
        .send()
        .map_err(|e| format!("GitHub non raggiungibile: {e}"))?;
    if response.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let response = response
        .error_for_status()
        .map_err(|e| format!("Risposta di GitHub non valida: {e}"))?;
    let raw = response
        .text()
        .map_err(|e| format!("Manifesto non leggibile: {e}"))?;
    let manifest: Manifest =
        serde_json::from_str(&raw).map_err(|e| format!("Manifesto non valido: {e}"))?;
    validate(&manifest)?;
    if is_newer(&manifest.version, current_version()) {
        Ok(Some(manifest))
    } else {
        Ok(None)
    }
}

/// Scarica e verifica il setup completo. Restituisce il percorso dell'exe.
pub fn download_installer(
    manifest: &Manifest,
    progress: &mut dyn FnMut(u64, u64),
) -> Result<PathBuf, String> {
    let dir = updates_dir()?;
    std::fs::create_dir_all(&dir)
        .map_err(|e| format!("Impossibile creare la cartella aggiornamenti: {e}"))?;
    let name = format!("CineMatch-Setup-{}.exe", manifest.version);
    let part = dir.join(format!("{name}.part"));
    let final_path = dir.join(&name);

    let client = client()?;
    let mut response = client
        .get(&manifest.installer.url)
        .send()
        .map_err(|e| format!("Download non riuscito: {e}"))?
        .error_for_status()
        .map_err(|e| format!("Download non riuscito: {e}"))?;
    let total = response.content_length().unwrap_or(manifest.installer.size);
    if total > MAX_INSTALLER {
        return Err("Il setup è troppo grande per essere un aggiornamento CineMatch.".into());
    }

    let mut file = File::create(&part)
        .map_err(|e| format!("Impossibile scrivere il setup: {e}"))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    let mut done = 0u64;
    let result = (|| -> Result<(), String> {
        loop {
            let n = response
                .read(&mut buf)
                .map_err(|e| format!("Download interrotto: {e}"))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])
                .map_err(|e| format!("Impossibile scrivere il setup: {e}"))?;
            hasher.update(&buf[..n]);
            done += n as u64;
            if done > MAX_INSTALLER {
                return Err("Il setup supera il limite di dimensione.".into());
            }
            progress(done, total);
        }
        file.flush()
            .map_err(|e| format!("Impossibile completare il setup: {e}"))?;
        Ok(())
    })();
    if let Err(e) = result {
        let _ = std::fs::remove_file(&part);
        return Err(e);
    }
    if total > 0 && done != total {
        let _ = std::fs::remove_file(&part);
        return Err("Il download è incompleto.".into());
    }
    let digest: String = hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect();
    if !digest.eq_ignore_ascii_case(manifest.installer.sha256.trim()) {
        let _ = std::fs::remove_file(&part);
        return Err("Il setup scaricato non supera la verifica di integrità.".into());
    }

    // Se esiste una copia identica già verificata, riusala.
    let _ = std::fs::remove_file(&final_path);
    std::fs::rename(&part, &final_path)
        .map_err(|e| format!("Impossibile preparare il setup: {e}"))?;
    Ok(final_path)
}

/// Avvia il setup in modalità silenziosa. L'app si chiude subito dopo: Inno
/// attende l'uscita, sostituisce i file e rilancia CineMatch.
pub fn launch_installer(path: &Path) -> Result<(), String> {
    if !path.is_file() {
        return Err("Il setup scaricato non esiste più.".into());
    }
    std::process::Command::new(path)
        .arg("/SILENT")
        .arg("/CLOSEAPPLICATIONS")
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Impossibile avviare il setup: {e}"))
}

fn client() -> Result<reqwest::blocking::Client, String> {
    reqwest::blocking::Client::builder()
        .user_agent(concat!("CineMatch/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(20))
        .build()
        .map_err(|e| format!("Client HTTP non disponibile: {e}"))
}

fn updates_dir() -> Result<PathBuf, String> {
    let base = std::env::var("LOCALAPPDATA")
        .map_err(|_| "LOCALAPPDATA non disponibile.".to_string())?;
    Ok(PathBuf::from(base).join("CineMatch").join("updates"))
}

fn validate(manifest: &Manifest) -> Result<(), String> {
    let version = manifest.version.trim();
    if version.is_empty()
        || version.len() > 32
        || !version
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == '-')
    {
        return Err("Versione del manifesto non valida.".into());
    }
    if !manifest.installer.url.starts_with(DOWNLOAD_PREFIX) {
        return Err("Il manifesto non punta a una release di CineMatch.".into());
    }
    let hash = manifest.installer.sha256.trim();
    if hash.len() != 64 || !hash.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err("Hash del setup non valido.".into());
    }
    if manifest.installer.size == 0 || manifest.installer.size > MAX_INSTALLER {
        return Err("Dimensione del setup non valida.".into());
    }
    if !manifest.notes_url.is_empty()
        && !manifest.notes_url.starts_with("https://github.com/")
    {
        return Err("Link delle note non valido.".into());
    }
    Ok(())
}

/// Confronto numerico tollerante: `0.2.0`, `v0.2.0`, `0.2.0+...`.
/// Le prerelease non vengono installate automaticamente.
pub fn is_newer(remote: &str, current: &str) -> bool {
    if remote.contains('-') {
        return false;
    }
    let Some(a) = numeric_parts(remote) else {
        return false;
    };
    let Some(b) = numeric_parts(current) else {
        return false;
    };
    let len = a.len().max(b.len());
    for i in 0..len {
        let x = a.get(i).copied().unwrap_or(0);
        let y = b.get(i).copied().unwrap_or(0);
        if x != y {
            return x > y;
        }
    }
    false
}

fn numeric_parts(version: &str) -> Option<Vec<u64>> {
    let version = version.trim().trim_start_matches(['v', 'V']);
    let core = version.split(['+', '-']).next()?;
    let mut out = Vec::new();
    for part in core.split('.') {
        if part.is_empty() || !part.chars().all(|c| c.is_ascii_digit()) {
            return None;
        }
        out.push(part.parse().ok()?);
    }
    if out.is_empty() {
        None
    } else {
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn confronta_versioni_numeriche() {
        assert!(is_newer("0.2.0", "0.1.0"));
        assert!(is_newer("v1.0.0", "0.9.9"));
        assert!(is_newer("0.1.10", "0.1.9"));
        assert!(!is_newer("0.1.0", "0.1.0"));
        assert!(!is_newer("0.0.9", "0.1.0"));
        assert!(!is_newer("0.2.0-beta", "0.1.0"));
        assert!(!is_newer("sconosciuta", "0.1.0"));
    }

    #[test]
    fn legge_il_manifesto_generato_dalla_ci() {
        let raw = r#"{
            "version": "0.2.0",
            "published_at": "2026-10-22T20:00:00Z",
            "notes_url": "https://github.com/Emanueleparis98/cinematch/releases/tag/v0.2.0",
            "installer": {
                "url": "https://github.com/Emanueleparis98/cinematch/releases/download/v0.2.0/CineMatch-Setup-0.2.0.exe",
                "sha256": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "size": 6121865
            }
        }"#;
        let m: Manifest = serde_json::from_str(raw).unwrap();
        assert_eq!(m.version, "0.2.0");
        assert!(m.notes.is_empty());
        assert_eq!(m.installer.size, 6_121_865);
        assert!(validate(&m).is_ok());
    }

    #[test]
    fn valida_manifesto() {
        let mut m = Manifest {
            version: "0.2.0".into(),
            notes: String::new(),
            notes_url: "https://github.com/Emanueleparis98/cinematch/releases/tag/v0.2.0".into(),
            installer: Installer {
                url: format!(
                    "{}v0.2.0/CineMatch-Setup-0.2.0.exe",
                    DOWNLOAD_PREFIX
                ),
                sha256: "a".repeat(64),
                size: 1024,
            },
        };
        assert!(validate(&m).is_ok());
        m.installer.url = "https://evil.example/setup.exe".into();
        assert!(validate(&m).is_err());
    }
}
