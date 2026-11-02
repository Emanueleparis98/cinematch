use std::io::{self, Read};
fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
fn run() -> Result<(), String> {
    match std::env::args().nth(1).as_deref() {
        Some("catalog") => println!(
            "{}",
            serde_json::json!({"schema_version":cinema_currents::SCHEMA_VERSION,"entries":cinema_currents::catalog()})
        ),
        Some("generate") => {
            let mut raw = String::new();
            io::stdin()
                .take(1_048_577)
                .read_to_string(&mut raw)
                .map_err(|e| e.to_string())?;
            if raw.len() > 1_048_576 {
                return Err("Richiesta troppo grande: massimo 1 MiB.".into());
            }
            let request: cinema_currents::PromptRequest = serde_json::from_str(&raw)
                .map_err(|e| format!("Richiesta JSON non valida: {e}"))?;
            println!(
                "{}",
                serde_json::to_string(&cinema_currents::generate(&request)?)
                    .map_err(|e| e.to_string())?
            );
        }
        Some("--version") => println!("cinema-currents {}", env!("CARGO_PKG_VERSION")),
        _ => {
            return Err(
                "Uso: cinema-currents catalog | generate (JSON da stdin) | --version".into(),
            )
        }
    }
    Ok(())
}
