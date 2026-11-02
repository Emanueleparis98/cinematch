use std::collections::HashMap;
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

fn decode(ctx: &Context, name: &str, bytes: &[u8]) -> Option<TextureHandle> {
    let img = image::load_from_memory(bytes).ok()?;
    let rgba = img.to_rgba8();
    let size = [rgba.width() as usize, rgba.height() as usize];
    let color = ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
    Some(ctx.load_texture(name.to_string(), color, TextureOptions::LINEAR))
}

/// Poster decoded off-thread, cached only in bounded session memory.
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
            let mut retry = true;
            let tex = match http.get(&uri_owned).send() {
                Ok(resp) if resp.status().is_success() => {
                    resp.bytes().ok().and_then(|b| decode(&ctx, &uri_owned, &b))
                }
                Ok(resp) => {
                    retry = resp.status().as_u16() == 429 || resp.status().is_server_error();
                    None
                }
                Err(_) => None,
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
        // Bounded RAM cache: evicted images are fetched again only when displayed.
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
