//! Verifica i voti IMDb sui dati reali: scarica, costruisce l'indice, legge
//! qualche identificativo noto e controlla l'ordinamento del file (da esso
//! dipende la ricerca binaria).
//!
//! Uso: cargo run --release --example imdb_probe

use cinematch::infra::imdb;

fn main() {
    println!(
        "prima: pronti={} · {:.1} MB · {} voci",
        imdb::is_ready(),
        imdb::data_size_mb(),
        imdb::entries()
    );

    if !imdb::is_ready() {
        let res = imdb::download(&|done| {
            let mb = done as f64 / 1_048_576.0;
            if (mb as u64).is_multiple_of(5) && done % 262_144 < 65_536 {
                println!("  scaricati {mb:.1} MB");
            }
        });
        println!("download: {res:?}");
    }
    println!(
        "dopo:  pronti={} · {:.1} MB · {} voci",
        imdb::is_ready(),
        imdb::data_size_mb(),
        imdb::entries()
    );

    // identici noti: classici + film recente
    for id in [
        "tt0064116", // C'era una volta il West
        "tt0054215", // Psyco
        "tt0111161", // Le ali della liberta
        "tt0137523", // Fight Club
        "tt15239678", // Dune: Parte due (id a 8 cifre)
        "tt99999999", // inesistente: deve dare None
    ] {
        match imdb::lookup(id) {
            Some(r) => println!("  {id}: {:.1} ★ con {} voti", r.rating, r.votes),
            None => println!("  {id}: assente"),
        }
    }

    // Ordinamento reale del file: la ricerca binaria assume un ordine preciso.
    println!("\ncontrollo ordinamento del file:");
    let file = match std::fs::File::open(imdb::data_path()) {
        Ok(f) => f,
        Err(e) => {
            println!("  file non leggibile: {e}");
            return;
        }
    };
    let mut reader = std::io::BufReader::with_capacity(1 << 20, file);
    let mut line = String::new();
    let (mut prev_num, mut prev_str) = (0u64, String::new());
    let (mut num_violations, mut str_violations) = (0usize, 0usize);
    let mut rows = 0usize;
    loop {
        line.clear();
        use std::io::BufRead;
        match reader.read_line(&mut line) {
            Ok(0) => break,
            Ok(_) => {}
            Err(e) => {
                println!("  lettura interrotta: {e}");
                break;
            }
        }
        let id = line.split('\t').next().unwrap_or("").trim().to_string();
        let Some(num) = id.trim_start_matches("tt").parse::<u64>().ok() else {
            continue;
        };
        if rows > 0 {
            if num < prev_num {
                num_violations += 1;
            }
            if id < prev_str {
                str_violations += 1;
            }
        }
        prev_num = num;
        prev_str = id;
        rows += 1;
    }
    println!("  righe: {rows}");
    println!("  violazioni dell'ordine numerico: {num_violations}");
    println!("  violazioni dell'ordine alfabetico: {str_violations}");
}
