use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};

use egui::{ColorImage, Context, TextureHandle, TextureOptions};

/// Quanto aspettare prima di riprovare un poster fallito per rete o 5xx.
const RETRY_AFTER: Duration = Duration::from_secs(60);

/// Stato di una immagine remota.
enum Slot {
    Loading,
    Ready(TextureHandle),
    /// Errore temporaneo: si puo' riprovare dopo `RETRY_AFTER`.
    Failed(Instant),
    /// 404 o altra risposta definitiva: non ha senso riprovare.
    Missing,
}

fn hash(uri: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in uri.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x1000_0000_01b3);
    }
    h
}

fn cache_dir() -> PathBuf {
    crate::infra::config::Config::dir().join("cache")
}

fn from_disk(uri: &str) -> Option<Vec<u8>> {
    let path = cache_dir().join(format!("{:016x}.img", hash(uri)));
    let bytes = std::fs::read(&path).ok()?;
    // La potatura ordina per mtime: una lettura conta come uso, altrimenti
    // cancella i poster visti spesso ma scaricati tanto tempo fa.
    if let Ok(file) = std::fs::OpenOptions::new().write(true).open(&path) {
        let _ = file.set_modified(std::time::SystemTime::now());
    }
    Some(bytes)
}

fn to_disk(uri: &str, bytes: &[u8]) {
    let dir = cache_dir();
    if std::fs::create_dir_all(&dir).is_err() {
        return;
    }
    let path = dir.join(format!("{:016x}.img", hash(uri)));
    let _ = std::fs::write(path, bytes);
}

/// La cache su disco non deve crescere all'infinito: oltre `max` file si
/// cancellano i poster usati meno di recente, tenendone `keep`.
fn prune_cache_dir(dir: &std::path::Path, max: usize, keep: usize) -> usize {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return 0;
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = entries
        .filter_map(|e| e.ok())
        .filter_map(|e| {
            let meta = e.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some((meta.modified().ok()?, e.path()))
        })
        .collect();
    if files.len() <= max {
        return 0;
    }
    files.sort_by_key(|(t, _)| *t);
    let excess = files.len().saturating_sub(keep);
    let mut removed = 0usize;
    for (_, path) in files.into_iter().take(excess) {
        if std::fs::remove_file(path).is_ok() {
            removed += 1;
        }
    }
    removed
}

/// Potatura della cache reale, in un thread leggero una volta per avvio.
fn prune_disk_cache() {
    const MAX_FILES: usize = 800;
    const KEEP_FILES: usize = 500;
    prune_cache_dir(&cache_dir(), MAX_FILES, KEEP_FILES);
}

fn decode(ctx: &Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(name.to_string(), color, TextureOptions::LINEAR))
}

/// Carica i poster da TMDB: memoria -> disco -> rete, con decodifica in un thread
/// separato e cache su disco in %APPDATA%\CineMatch\cache.
pub struct ImageStore {
    map: Mutex<HashMap<String, Slot>>,
    tx: Sender<(String, Option<TextureHandle>, bool)>,
    rx: Receiver<(String, Option<TextureHandle>, bool)>,
    http: reqwest::blocking::Client,
}

impl ImageStore {
    pub fn new() -> Self {
        let http = reqwest::blocking::Client::builder()
            .user_agent("CineMatch/0.1 (Windows)")
            .timeout(Duration::from_secs(25))
            .build()
            .unwrap_or_else(|_| reqwest::blocking::Client::new());
        let (tx, rx) = channel();
        std::thread::spawn(prune_disk_cache);
        Self {
            map: Mutex::new(HashMap::new()),
            tx,
            rx,
            http,
        }
    }

    /// Ritorna la texture se gia pronta, altrimenti avvia il caricamento.
    pub fn texture(&self, ctx: &Context, uri: &str) -> Option<TextureHandle> {
        {
            let mut map = self.map.lock().ok()?;
            match map.get(uri) {
                Some(Slot::Ready(tex)) => return Some(tex.clone()),
                Some(Slot::Loading) | Some(Slot::Missing) => return None,
                Some(Slot::Failed(at)) if at.elapsed() < RETRY_AFTER => return None,
                Some(Slot::Failed(_)) | None => {}
            }
            map.remove(uri);
            map.insert(uri.to_string(), Slot::Loading);
        }
        let ctx = ctx.clone();
        let tx = self.tx.clone();
        let http = self.http.clone();
        let uri_owned = uri.to_string();
        std::thread::spawn(move || {
            // Prima si prova il disco, poi la rete. Un errore di rete o un 5xx
            // resta riprovabile; un 404 no.
            let mut retry = true;
            let tex = from_disk(&uri_owned).and_then(|b| decode(&ctx, &uri_owned, &b));
            let tex = match tex {
                Some(tex) => Some(tex),
                None => match http.get(&uri_owned).send() {
                    Ok(resp) if resp.status().is_success() => {
                        let bytes = resp.bytes().ok().map(|b| b.to_vec());
                        match bytes {
                            Some(bytes) => {
                                let tex = decode(&ctx, &uri_owned, &bytes);
                                if tex.is_some() {
                                    to_disk(&uri_owned, &bytes);
                                }
                                tex
                            }
                            None => None,
                        }
                    }
                    Ok(resp) => {
                        retry =
                            resp.status().as_u16() == 429 || resp.status().is_server_error();
                        None
                    }
                    Err(_) => None,
                },
            };
            let _ = tx.send((uri_owned, tex, retry));
            ctx.request_repaint();
        });
        None
    }

    /// Raccoglie i caricamenti conclusi. Ritorna true se qualcosa e cambiato.
    pub fn poll(&self) -> bool {
        let mut updated = false;
        while let Ok((uri, tex, retry)) = self.rx.try_recv() {
            if let Ok(mut map) = self.map.lock() {
                match tex {
                    Some(t) => {
                        map.insert(uri, Slot::Ready(t));
                    }
                    None if retry => {
                        map.insert(uri, Slot::Failed(Instant::now()));
                    }
                    None => {
                        map.insert(uri, Slot::Missing);
                    }
                }
            }
            updated = true;
        }
        // Limite di memoria: le texture vengono ricostruite dalla cache su disco.
        if let Ok(mut map) = self.map.lock() {
            if map.len() > 320 {
                map.retain(|_, slot| matches!(slot, Slot::Loading));
                updated = true;
            }
        }
        updated
    }
}

impl Default for ImageStore {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::prune_cache_dir;

    #[test]
    fn la_potatura_tiene_i_poster_piu_recenti() {
        let dir = std::env::temp_dir().join(format!("cinematch_prune_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");

        // 10 file, dal piu vecchio al piu recente
        for i in 0..10 {
            let path = dir.join(format!("{i:02}.img"));
            std::fs::write(&path, b"x").expect("scrittura");
            let when = std::time::SystemTime::now() - std::time::Duration::from_secs(1000 - i * 60);
            let file = std::fs::OpenOptions::new().write(true).open(&path).unwrap();
            file.set_modified(when).expect("mtime");
        }

        // sotto la soglia non tocca nulla
        assert_eq!(prune_cache_dir(&dir, 20, 10), 0);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 10);

        // sopra la soglia scende a `keep` e conserva i piu recenti
        assert_eq!(prune_cache_dir(&dir, 6, 4), 6);
        assert_eq!(std::fs::read_dir(&dir).unwrap().count(), 4);
        assert!(dir.join("09.img").exists(), "il piu recente resta");
        assert!(dir.join("08.img").exists());
        assert!(!dir.join("00.img").exists(), "il piu vecchio viene rimosso");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
