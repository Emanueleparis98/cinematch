//! Data/ora dell'ultima compilazione del binario in esecuzione (solo Windows).
//!
//! In fondo alle Impostazioni appare «build gg/mm hh:mm» letta dalla data del file eseguibile:
//! cosi si riconosce subito se la finestra aperta e la build appena fatta o una
//! vecchia rimasta li da prima.
//!
//! Usa kernel32 direttamente (FileTimeToLocalFileTime + FileTimeToSystemTime)
//! per non aggiungere dipendenze solo per formattare una data.

use std::time::{SystemTime, UNIX_EPOCH};

#[repr(C)]
struct FileTime {
    low: u32,
    high: u32,
}

#[repr(C)]
struct SystemTimeParts {
    year: u16,
    month: u16,
    day_of_week: u16,
    day: u16,
    hour: u16,
    minute: u16,
    second: u16,
    milliseconds: u16,
}

#[link(name = "kernel32")]
unsafe extern "system" {
    fn FileTimeToLocalFileTime(in_time: *const FileTime, out_time: *mut FileTime) -> i32;
    fn FileTimeToSystemTime(file_time: *const FileTime, out_time: *mut SystemTimeParts) -> i32;
    fn GetLocalTime(lpSystemTime: *mut SystemTimeParts);
}

/// Data locale di oggi in formato `YYYY-MM-DD`, per delimitare le uscite.
pub fn today_iso() -> String {
    let mut st = SystemTimeParts {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    unsafe { GetLocalTime(&mut st) };
    format!("{:04}-{:02}-{:02}", st.year, st.month, st.day)
}

/// «gg/mm hh:mm» in ora locale dell'ultima modifica del binario in esecuzione;
/// «--/-- --:--» se la data non e leggibile.
pub fn build_stamp() -> String {
    let mtime = std::env::current_exe()
        .ok()
        .and_then(|p| std::fs::metadata(p).ok())
        .and_then(|m| m.modified().ok())
        .unwrap_or(SystemTime::now());
    stamp_of(mtime)
}

fn stamp_of(t: SystemTime) -> String {
    // Secondi UNIX -> FILETIME (passi da 100 ns dal 1601-01-01 UTC).
    let secs = t.duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs());
    let ticks = secs * 10_000_000 + 116_444_736_000_000_000;
    let utc = FileTime {
        low: ticks as u32,
        high: (ticks >> 32) as u32,
    };
    let mut local = FileTime { low: 0, high: 0 };
    let mut st = SystemTimeParts {
        year: 0,
        month: 0,
        day_of_week: 0,
        day: 0,
        hour: 0,
        minute: 0,
        second: 0,
        milliseconds: 0,
    };
    unsafe {
        if FileTimeToLocalFileTime(&utc, &mut local) == 0
            || FileTimeToSystemTime(&local, &mut st) == 0
        {
            return "--/-- --:--".into();
        }
    }
    format!("{:02}/{:02} {:02}:{:02}", st.day, st.month, st.hour, st.minute)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formato_gg_mm_hh_mm() {
        let s = build_stamp();
        // Deve essere una data «gg/mm hh:mm» leggibile (non il fallback).
        assert_ne!(s, "--/-- --:--", "conversione FILETIME non riuscita");
        let b = s.as_bytes();
        assert_eq!(b.len(), 11, "formato inatteso: {s}");
        assert_eq!(b[2], b'/');
        assert_eq!(b[5], b' ');
        assert_eq!(b[8], b':');
        assert!(b
            .iter()
            .enumerate()
            .all(|(i, c)| matches!(i, 2 | 5 | 8) || c.is_ascii_digit()));
    }

    #[test]
    fn oggi_e_una_data_iso() {
        let s = today_iso();
        assert_eq!(s.len(), 10, "formato inatteso: {s}");
        assert_eq!(&s[4..5], "-");
        assert_eq!(&s[7..8], "-");
        assert!(s[..4].chars().all(|c| c.is_ascii_digit()));
        assert!(s[5..7].chars().all(|c| c.is_ascii_digit()));
        assert!(s[8..].chars().all(|c| c.is_ascii_digit()));
    }
}
