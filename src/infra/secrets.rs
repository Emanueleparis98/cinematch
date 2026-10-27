//! Chiavi API fuori dal file di configurazione: Windows Credential Manager.
//! Su altre piattaforme l'app non e supportata; `available()` dice al chiamante
//! di non cancellare le chiavi dal file.

pub const TMDB: &str = "tmdb-api-key";
pub const DEEPSEEK: &str = "deepseek-api-key";
pub const OPENROUTER: &str = "openrouter-api-key";

const SERVICE: &str = "CineMatch";

pub fn available() -> bool {
    cfg!(target_os = "windows")
}

#[cfg(target_os = "windows")]
fn entry(name: &str) -> Result<keyring::Entry, String> {
    keyring::Entry::new(SERVICE, name).map_err(|e| format!("Credenziale non accessibile: {e}"))
}

#[cfg(target_os = "windows")]
pub fn load(name: &str) -> Option<String> {
    match entry(name).ok()?.get_password() {
        Ok(raw) if !raw.trim().is_empty() => Some(raw),
        _ => None,
    }
}

#[cfg(not(target_os = "windows"))]
pub fn load(_name: &str) -> Option<String> {
    None
}

#[cfg(target_os = "windows")]
pub fn save(name: &str, value: &str) -> Result<(), String> {
    let entry = entry(name)?;
    if value.trim().is_empty() {
        match entry.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
            Err(e) => Err(format!("Credenziale non rimossa: {e}")),
        }
    } else {
        entry
            .set_password(value)
            .map_err(|e| format!("Credenziale non salvata: {e}"))
    }
}

#[cfg(not(target_os = "windows"))]
pub fn save(_name: &str, _value: &str) -> Result<(), String> {
    Ok(())
}
