//! Apertura di un film in Nuvio. Il deep link ufficiale e' documentato per
//! Android (telefono/tablet/TV); il desktop Windows non registra lo schema,
//! quindi l'app mostra il QR e il ponte "Da vedere" su SIMKL.

use std::path::PathBuf;

/// `tt` + 5-10 cifre. Evita di passare un identificativo non IMDb al sistema.
fn valid_imdb(imdb: &str) -> bool {
    let imdb = imdb.trim();
    let Some(rest) = imdb.strip_prefix("tt") else {
        return false;
    };
    (5..=10).contains(&rest.len()) && rest.chars().all(|c| c.is_ascii_digit())
}

pub fn link(imdb: &str) -> Result<String, String> {
    if !valid_imdb(imdb) {
        return Err("Identificativo IMDb non valido.".into());
    }
    Ok(format!("nuvio://movie/{}", imdb.trim()))
}

pub fn exe_path() -> Option<PathBuf> {
    let mut candidates = Vec::new();
    if let Ok(program_files) = std::env::var("ProgramFiles") {
        candidates.push(PathBuf::from(program_files).join("Nuvio").join("Nuvio.exe"));
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        candidates.push(
            PathBuf::from(&local)
                .join("Programs")
                .join("Nuvio")
                .join("Nuvio.exe"),
        );
    }
    candidates.into_iter().find(|path| path.is_file())
}

pub fn open_link(link: &str) -> Result<(), String> {
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        use std::process::{Command, Stdio};
        use windows_sys::Win32::UI::Shell::ShellExecuteW;

        // Se lo schema non e' registrato, Windows aprirebbe la finestra
        // "Ottieni un'app": meglio il nostro riquadro con QR e Da vedere.
        let registered = [
            r"HKCU\Software\Classes\nuvio",
            r"HKLM\Software\Classes\nuvio",
            r"HKCR\nuvio",
        ]
        .iter()
        .any(|key| {
            Command::new("reg")
                .args(["query", key])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .creation_flags(0x0800_0000)
                .status()
                .map(|status| status.success())
                .unwrap_or(false)
        });
        if !registered {
            return Err("Nuvio non ha registrato il link su questo PC.".into());
        }

        let wide = |value: &str| -> Vec<u16> {
            value.encode_utf16().chain(std::iter::once(0)).collect()
        };
        let operation = wide("open");
        let file = wide(link);
        // 1 = SW_SHOWNORMAL
        let result = unsafe {
            ShellExecuteW(
                std::ptr::null_mut(),
                operation.as_ptr(),
                file.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                1,
            )
        };
        if result as isize <= 32 {
            return Err("Nuvio non ha registrato il link su questo PC.".into());
        }
        Ok(())
    }
    #[cfg(not(target_os = "windows"))]
    {
        let _ = link;
        Err("Apertura di Nuvio non supportata su questa piattaforma.".into())
    }
}

pub fn open_app() -> Result<(), String> {
    let Some(path) = exe_path() else {
        return Err("Nuvio non risulta installato su questo PC.".into());
    };
    std::process::Command::new(path)
        .spawn()
        .map(|_| ())
        .map_err(|e| format!("Nuvio non avviato: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn costruisce_il_link_solo_con_un_imdb_valido() {
        assert_eq!(
            link("tt0060196").unwrap(),
            "nuvio://movie/tt0060196"
        );
        assert!(link("0060196").is_err());
        assert!(link("ttabc").is_err());
        assert!(link("tt0060196/../x").is_err());
        assert!(link("tt0060196\n").is_ok());
    }
}
