# Verifica successiva all'audit Ponytail

Data: 1 novembre 2026. Piattaforma: Windows, Rust 1.99.1, target x86_64-pc-windows-gnu.

## Correzioni

- Stop restituisce un errore definitivo durante la lettura di testo e streaming. Non usa piu `Interrupted`, che i lettori Rust riprovano automaticamente.
- Prossimamente conserva il periodo selezionato. Una combinazione impossibile viene spiegata senza interrogare TMDB e senza eliminare filtri di nascosto.
- Il worker di Sblocca richiede un ridisegno quando consegna i suggerimenti.
- Le verifiche TMDB e AI conservano le impostazioni provate e scartano risposte obsolete. Modificare il modulo cancella il precedente esito visualizzato.

## Controlli automatici

- `cargo test --locked --offline --all-targets -q`: 125 test della libreria e 22 test di integrazione superati.
- `cargo clippy --locked --offline --all-targets -- -D warnings`: superato.
- `cargo check --locked --offline --features wgpu --all-targets`: superato.
- `cargo audit`: nessuna vulnerabilita nota. Tre avvisi di manutenzione per zbus_names, zvariant e zvariant_derive; nessuno dei tre entra nel grafo delle dipendenze Windows di questa build.
- Test di regressione: annullamento durante la lettura, periodo passato con Prossimamente, risposte di verifica obsolete.
- Smoke test dell'interfaccia: 21 stati a 800x600, 1220x660 e 1920x1080, due frame per stato. Include primo avvio, home, errori e caricamento, chat con cinque card e testi lunghi, catalogo, filtri, impostazioni dei provider, dettaglio, guida e modali. Le prove usano dati isolati, senza account reali o potatura della cache personale.

## Servizi reali

Prova temporanea, rimossa dopo l'esecuzione; nessuna credenziale scritta nei log.

- TMDB: credenziale valida, cartellone con 20 film, dettaglio film leggibile.
- OpenRouter configurato: risposta di verifica riuscita.
- Ricerca completa nell'app installata: interpretazione OpenRouter, selezione di cinque film reali e caricamento di metadati e provider riusciti; campo di ricerca fisso e card controllati visivamente.
- SIMKL: lettura di `/sync/activities` riuscita, HTTP 200. Nessuna modifica a liste o voti.

## Installer e controlli visivi

Installer locale generato con Inno Setup 6 ed eseguito per utente, codice di uscita 0. Il log registra `Installation process succeeded`, nessun riavvio Windows richiesto e lancio automatico della copia installata. Il confronto SHA-256 prima/dopo conferma che config.json e rimasto identico. Home, dati SIMKL, catalogo con filtri, impostazioni e scheda con sinossi lunga controllati nell'app installata, anche massimizzata.

## Limiti

Lo smoke test non misura una copertura esaustiva di tutti i rami o di ogni combinazione di dati. WGPU e verificato in compilazione; la prova grafica reale usa il renderer predefinito OpenGL. Le scritture SIMKL sono coperte dai test con server locale, non effettuate sull'account personale. Nessuna nuova release GitHub pubblicata.
