//! Voti IMDb dai dati pubblici gratuiti, senza chiave API e senza limiti di
//! richieste: il file `title.ratings.tsv` di datasets.imdbws.com (~8 MB
//! compresso) viene scaricato una volta e consultato sul posto.
//!
//! Il file e ordinato per identificativo in ordine **alfabetico** (quindi
//! `tt10000000` viene prima di `tt9999999`), non serve tenerlo in memoria: si
//! costruisce un indice dei salti ogni 2048 righe e si cerca l'identificativo
//! con una ricerca binaria sull'indice, leggendo poi solo il blocco di righe
//! che interessa. L'indice si ricostruisce da solo se manca o e di un formato
//! precedente.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, OnceLock, RwLock};

use crate::domain::model::ImdbRating;

const URL: &str = "https://datasets.imdbws.com/title.ratings.tsv.gz";
/// Righe per blocco dell'indice.
const BLOCK: usize = 2048;
/// Il file vero e' molto piu' grande: sotto questa soglia e' un errore.
const MIN_DATA_BYTES: u64 = 1_000_000;
/// Intestazione del file indice: distingue il formato corrente da quelli vecchi.
const MAGIC: &[u8; 8] = b"CMIDX03\n";
/// Byte per voce dell'indice: offset (8) + identificativo grezzo (16).
const ENTRY: usize = 24;

/// Chiave di ordinamento identica a quella del file IMDb, che e ordinato per
/// identificativo **alfabeticamente** (non per numero): gli id a 8 cifre stanno
/// dove li mette il confronto carattere per carattere. Il riempimento con zeri
/// a destra non cambia l'ordine, perche lo zero e minore di ogni cifra.
type Key = [u8; 16];

fn key(tconst: &str) -> Key {
    let mut k = [0u8; 16];
    let bytes = tconst.as_bytes();
    let n = bytes.len().min(16);
    k[..n].copy_from_slice(&bytes[..n]);
    k
}

fn dir() -> PathBuf {
    crate::infra::config::Config::dir().join("imdb")
}

/// Il file tsv non compresso, con il suo indice a fianco.
pub fn data_path() -> PathBuf {
    dir().join("title.ratings.tsv")
}

fn index_path() -> PathBuf {
    dir().join("title.ratings.idx")
}

/// Vero se i dati sono gia stati scaricati. L'indice si ricostruisce da solo:
/// se manca (o e di un formato vecchio) viene rifatto al primo consulto.
pub fn is_ready() -> bool {
    data_path().exists()
}

/// Dimensione dei dati scaricati, per informazione nelle Impostazioni.
pub fn data_size_mb() -> f64 {
    std::fs::metadata(data_path())
        .map(|m| m.len() as f64 / 1_048_576.0)
        .unwrap_or(0.0)
}

/// Quante voci contiene l'indice, contando anche l'ultimo blocco parziale.
pub fn entries() -> usize {
    entries_from(&index_path(), &data_path())
}

fn entries_from(index: &Path, data: &Path) -> usize {
    let raw = std::fs::read(index).unwrap_or_default();
    if raw.len() <= MAGIC.len() || &raw[..MAGIC.len()] != MAGIC {
        return 0;
    }
    let body = &raw[MAGIC.len()..];
    if !body.len().is_multiple_of(ENTRY) {
        return 0;
    }
    let blocks = body.len() / ENTRY;
    if blocks == 0 {
        return 0;
    }
    let last = &body[(blocks - 1) * ENTRY..];
    let offset = u64::from_le_bytes(last[0..8].try_into().unwrap_or([0; 8]));
    let rows = std::fs::File::open(data)
        .ok()
        .and_then(|file| {
            let mut reader = BufReader::new(file);
            reader.seek(SeekFrom::Start(offset)).ok()?;
            let mut line = String::new();
            let mut n = 0usize;
            loop {
                line.clear();
                if reader.read_line(&mut line).ok()? == 0 || n == BLOCK {
                    break;
                }
                n += 1;
            }
            Some(n)
        })
        .unwrap_or(0);
    (blocks - 1) * BLOCK + rows
}

/// Scarica e scompatta i voti IMDb, poi costruisce l'indice.
/// `progress` riceve i byte scompattati scritti finora.
pub fn download(progress: &dyn Fn(u64)) -> Result<String, String> {
    std::fs::create_dir_all(dir()).map_err(|e| format!("Cartella IMDb non creabile: {e}"))?;
    let client = reqwest::blocking::Client::builder()
        .user_agent("CineMatch/0.1 (Windows)")
        .timeout(std::time::Duration::from_secs(600))
        .build()
        .map_err(|e| format!("Client HTTP non disponibile: {e}"))?;
    let mut resp = client
        .get(URL)
        .send()
        .map_err(|e| format!("Dati IMDb non raggiungibili: {e}"))?;
    if !resp.status().is_success() {
        return Err(format!("IMDb ha risposto {}", resp.status().as_u16()));
    }

    let tmp = dir().join("title.ratings.tsv.part");
    {
        let mut file =
            std::fs::File::create(&tmp).map_err(|e| format!("File temporaneo non creabile: {e}"))?;
        let mut decoder = flate2::read::GzDecoder::new(resp.by_ref());
        let mut buf = [0u8; 256 * 1024];
        let mut written = 0u64;
        loop {
            let n = decoder
                .read(&mut buf)
                .map_err(|e| format!("Dati compressi illeggibili: {e}"))?;
            if n == 0 {
                break;
            }
            file.write_all(&buf[..n])
                .map_err(|e| format!("Scrittura non riuscita: {e}"))?;
            written += n as u64;
            progress(written);
        }
        file.flush().ok();
    }
    // Un corpo valido ma non TSV (portale captive, pagina di errore gzip)
    // non deve sostituire i voti buoni.
    if let Err(e) = validate_data(&tmp, MIN_DATA_BYTES) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    // Dati e indice si sostituiscono insieme: una lookup non deve leggere
    // offset vecchi nel file nuovo.
    let _guard = DATA_LOCK
        .write()
        .unwrap_or_else(|e| e.into_inner());
    std::fs::rename(&tmp, data_path()).map_err(|e| format!("Rinomina non riuscita: {e}"))?;

    // Il file appena scritto invalida sia l'indice su disco sia quello in
    // memoria: senza questo, i voti letti dopo l'aggiornamento userebbero
    // offset del file precedente.
    invalidate_index_cache();
    let _ = std::fs::remove_file(index_path());
    build_index()?;
    let mb = data_size_mb();
    Ok(format!(
        "voti IMDb pronti: {:.0} MB in {:?}",
        mb,
        dir().display()
    ))
}

/// Il file deve iniziare con l'intestazione TSV e avere una dimensione
/// plausibile. La soglia e' zero nei test.
fn validate_data(path: &Path, min_bytes: u64) -> Result<(), String> {
    let len = std::fs::metadata(path)
        .map_err(|e| format!("Dati IMDb non leggibili: {e}"))?
        .len();
    if len < min_bytes {
        return Err(format!("Dati IMDb incompleti: {len} byte."));
    }
    let mut file = std::fs::File::open(path).map_err(|e| format!("Dati IMDb non leggibili: {e}"))?;
    let mut head = [0u8; 7];
    file.read_exact(&mut head)
        .map_err(|e| format!("Dati IMDb non leggibili: {e}"))?;
    if &head != b"tconst\t" {
        return Err("Il file IMDb non contiene la tabella dei voti.".into());
    }
    Ok(())
}

/// Costruisce l'indice dei blocchi: per ogni blocco salva (offset, primo id).
fn build_index() -> Result<(), String> {
    let tmp = index_path().with_extension("idx.part");
    if let Err(e) = build_index_from(&data_path(), &tmp) {
        let _ = std::fs::remove_file(&tmp);
        return Err(e);
    }
    std::fs::rename(&tmp, index_path()).map_err(|e| format!("Indice non rinominabile: {e}"))
}

fn build_index_from(data: &Path, out_path: &Path) -> Result<(), String> {
    let file = std::fs::File::open(data).map_err(|e| format!("Dati assenti: {e}"))?;
    let mut reader = BufReader::with_capacity(256 * 1024, file);
    let mut out = std::fs::File::create(out_path).map_err(|e| e.to_string())?;
    out.write_all(MAGIC).map_err(|e| e.to_string())?;
    let mut offset: u64 = 0;
    let mut line = String::new();
    let mut line_no = 0usize;
    let mut data_no = 0usize;
    loop {
        line.clear();
        let read = reader
            .read_line(&mut line)
            .map_err(|e| format!("Lettura non riuscita: {e}"))?;
        if read == 0 {
            break;
        }
        // La prima riga e l'intestazione: non e un film e non deve consumare
        // il primo blocco, altrimenti gli id piu bassi non sarebbero indicizzati.
        let header = line_no == 0 && line.starts_with("tconst");
        if !header {
            if data_no.is_multiple_of(BLOCK) {
                let id = line.split('\t').next().unwrap_or("").trim();
                if id.starts_with("tt") {
                    // offset a 8 byte + identificativo grezzo a 16 byte
                    out.write_all(&offset.to_le_bytes())
                        .and_then(|_| out.write_all(&key(id)))
                        .map_err(|e| format!("Indice non scrivibile: {e}"))?;
                }
            }
            data_no += 1;
        }
        offset += read as u64;
        line_no += 1;
    }
    out.flush().map_err(|e| format!("Indice non scrivibile: {e}"))?;
    Ok(())
}

/// Legge l'indice una sola volta per esecuzione riuscita: se i dati non c'erano
/// ancora (prima del download) il tentativo successivo li ritrova. Un indice
/// mancante o di formato vecchio viene ricostruito dal file dei voti.
type IndexSlot = Mutex<Option<Arc<Vec<(u64, Key)>>>>;
static INDEX: OnceLock<IndexSlot> = OnceLock::new();

/// Serializza lo scambio dati/indice con le letture dei voti.
static DATA_LOCK: RwLock<()> = RwLock::new(());

fn index_slot() -> &'static IndexSlot {
    INDEX.get_or_init(|| Mutex::new(None))
}

/// Dimentica l'indice in memoria dopo un aggiornamento dei dati.
fn invalidate_index_cache() {
    if let Some(slot) = INDEX.get() {
        if let Ok(mut guard) = slot.lock() {
            *guard = None;
        }
    }
}

fn index() -> Option<Arc<Vec<(u64, Key)>>> {
    let slot = index_slot();
    let mut guard = slot.lock().ok()?;
    if guard.is_none() {
        if let Some(v) = load_index() {
            *guard = Some(Arc::new(v));
        } else if data_path().exists() {
            // indice assente o scritto da una versione precedente: si rifa qui
            if build_index().is_ok() {
                *guard = load_index().map(Arc::new);
            }
        }
    }
    guard.clone()
}

/// Costruisce l'indice in memoria dai salti salvati su disco.
fn load_index() -> Option<Vec<(u64, Key)>> {
    let raw = std::fs::read(index_path()).ok()?;
    if raw.len() <= MAGIC.len() || &raw[..MAGIC.len()] != MAGIC {
        return None;
    }
    let body = &raw[MAGIC.len()..];
    if !body.len().is_multiple_of(ENTRY) {
        return None;
    }
    let mut out = Vec::with_capacity(body.len() / ENTRY);
    for chunk in body.as_chunks::<ENTRY>().0 {
        let offset = u64::from_le_bytes(chunk[0..8].try_into().ok()?);
        let mut id = [0u8; 16];
        id.copy_from_slice(&chunk[8..24]);
        out.push((offset, id));
    }
    Some(out)
}

/// Blocco che puo contenere l'identificativo cercato.
fn block_for(tconst: &str) -> Option<(usize, u64)> {
    let idx = index()?;
    let wanted = key(tconst);
    // ultimo blocco con primo id <= cercato (ordine alfabetico, come il file)
    let mut lo = 0usize;
    let mut hi = idx.len();
    while lo < hi {
        let mid = (lo + hi) / 2;
        if idx[mid].1 <= wanted {
            lo = mid + 1;
        } else {
            hi = mid;
        }
    }
    if lo == 0 {
        return None;
    }
    let (offset, _) = idx[lo - 1];
    Some((lo - 1, offset))
}

/// Voto e numero di voti IMDb per un identificativo `tt…`.
pub fn lookup(tconst: &str) -> Option<ImdbRating> {
    if !tconst.starts_with("tt") {
        return None;
    }
    let _guard = DATA_LOCK.read().unwrap_or_else(|e| e.into_inner());
    let (block, offset) = block_for(tconst)?;
    let file = std::fs::File::open(data_path()).ok()?;
    let mut reader = BufReader::with_capacity(64 * 1024, file);
    reader.seek(SeekFrom::Start(offset)).ok()?;
    let mut line = String::new();
    for i in 0..=BLOCK {
        line.clear();
        let read = reader.read_line(&mut line).ok()?;
        if read == 0 {
            break;
        }
        let mut fields = line.trim_end().split('\t');
        let id = fields.next()?;
        // nel blocco giusto l'id cercato non puo essere prima del primo del blocco
        if id == tconst {
            let rating: f32 = fields.next()?.parse().ok()?;
            let votes: u32 = fields.next()?.parse().ok()?;
            return Some(ImdbRating { rating, votes });
        }
        if i == BLOCK {
            break;
        }
    }
    let _ = block;
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    fn blocco(idx: &[(u64, Key)], cercato: &str) -> usize {
        let wanted = key(cercato);
        let mut lo = 0usize;
        let mut hi = idx.len();
        while lo < hi {
            let mid = (lo + hi) / 2;
            if idx[mid].1 <= wanted {
                lo = mid + 1;
            } else {
                hi = mid;
            }
        }
        assert!(lo > 0, "il primo blocco copre gli identificativi piu bassi");
        lo - 1
    }

    #[test]
    fn l_indice_parte_dalla_prima_riga_dati() {
        let dir = std::env::temp_dir().join(format!("cinematch_idx_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let data = dir.join("ratings.tsv");
        let idx = dir.join("ratings.idx");
        let header = "tconst\taverageRating\tnumVotes\n";
        std::fs::write(
            &data,
            format!("{header}tt0000001\t8.1\t100\ntt0000002\t7.2\t200\n"),
        )
        .expect("dati di prova");
        build_index_from(&data, &idx).expect("indice di prova");
        assert_eq!(entries_from(&idx, &data), 2, "conta anche il blocco parziale");
        let raw = std::fs::read(&idx).expect("lettura indice");
        assert_eq!(&raw[..MAGIC.len()], MAGIC);
        let offset = u64::from_le_bytes(raw[MAGIC.len()..MAGIC.len() + 8].try_into().unwrap());
        assert_eq!(offset, header.len() as u64, "l'intestazione non e un blocco");
        assert_eq!(&raw[MAGIC.len() + 8..MAGIC.len() + 17], b"tt0000001");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn la_ricerca_binaria_segue_l_ordine_alfabetico_del_file() {        // Indice finto in memoria: non serve il file reale. L'ordine e quello
        // del file IMDb, alfabetico: «tt9999999» viene *dopo* «tt10000000»,
        // quindi un confronto per numero sbaglierebbe blocco.
        let idx: Vec<(u64, Key)> = vec![
            (0, key("tt0000001")),
            (10_000, key("tt10000000")),
            (20_000, key("tt9999999")),
        ];
        assert_eq!(
            idx[blocco(&idx, "tt15239678")].1,
            key("tt10000000"),
            "gli identificativi a 8 cifre stanno nel blocco giusto"
        );
        assert_eq!(
            idx[blocco(&idx, "tt0000002")].1,
            key("tt0000001"),
            "gli identificativi bassi restano nel primo blocco"
        );
        assert_eq!(
            idx[blocco(&idx, "tt9999999")].1,
            key("tt9999999"),
            "un identificativo di 7 cifre alte trova l'ultimo blocco"
        );
    }

    #[test]
    fn i_dati_scaricati_devono_essere_la_tabella_dei_voti() {
        let dir = std::env::temp_dir().join(format!("cinematch_val_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).expect("cartella di prova");
        let path = dir.join("ratings.tsv");

        std::fs::write(&path, b"tconst\taverageRating\tnumVotes\n").expect("file valido");
        assert!(validate_data(&path, 0).is_ok());

        std::fs::write(&path, b"<!DOCTYPE html><html>errore</html>").expect("file html");
        assert!(validate_data(&path, 0).is_err());

        std::fs::write(&path, b"tconst\t").expect("file corto");
        assert!(validate_data(&path, 10).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
