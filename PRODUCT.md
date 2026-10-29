# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Users

Spettatori italofoni che devono decidere cosa guardare per la serata: dal pubblico occasionale che vuole una risposta rapida al cinefilo che vuole affinare la scelta. Il sito in `site/` è la pagina di download dell'app desktop.

## Product Purpose

Trasformare una richiesta libera in fino a cinque film reali da TMDB, ciascuno con il perché e un consiglio pratico; in alternativa, esplorare il catalogo con filtri a vincoli. Il successo è una scelta fatta senza perdere la serata, con motivazioni oneste.

## Positioning

Il motore di vincoli: le combinazioni della tassonomia vengono validate, quelle impossibili rifiutate o spiegate, e l'AI può solo proporre criteri. I titoli restano reali (TMDB), mai inventati. Funziona anche senza AI, con regole offline.

## Operating Context

- App desktop Windows 10/11 a 64 bit (Rust/egui); installer per-utente in `%LocalAppData%\Programs\CineMatch`, configurazione in `%APPDATA%\CineMatch`.
- Chiavi API e token SIMKL nel Windows Credential Manager, non in `config.json`.
- Prima richiesta: chiave TMDB gratuita; AI opzionale (DeepSeek, OpenRouter, Ollama); SIMKL opzionale.
- Aggiornamenti: controllo all'avvio e ogni sei ore; download e installazione solo su conferma, verifica SHA-256.
- Sito statico italiano in `site/`, pubblicato su GitHub Pages dal workflow `pages.yml`.

## Capabilities and Constraints

- Due viste: **Chat** (richiesta libera, fino a cinque film con perché e consiglio) e **Griglia** (filtri per periodo, genere, paese/lingua, mood/temi, metriche e correnti).
- "Sblocca" propone i compromessi più piccoli quando i risultati sono pochi; "Più recenti" e "Prossimamente" hanno regole proprie.
- SIMKL: visti, "Da vedere", voto 1–10, Riprendi e "Dalla tua lista"; sincronizzazione incrementale, nessun polling in background.
- Voti IMDb dai dati pubblici, accanto al voto TMDB.
- Interprete a regole offline predefinito; l'AI propone solo i criteri, la validazione resta al motore di vincoli.
- Solo Windows; interfaccia e sito in italiano. Nessun titolo inventato, nessun riempimento con film generici.

## Brand Commitments

- Nome: **CineMatch**. Voce italiana, diretta e onesta: dice cosa manca invece di riempire.
- Identità **LUMEN** già in uso: fondo ink `#08090C`, testo avorio, ember `#FF5C35` per i comandi, celadon per le disponibilità; niente gradienti decorativi, aloni o bolle; sono ammessi solo lo scrim del backdrop e la seam funzionale sui poster selezionati.
- Tipografia inclusa (licenza OFL): Fraunces per i titoli, Instrument Sans per il testo, JetBrains Mono per etichette e metadati.

## Evidence on Hand

- `README.md`: documentazione dettagliata di flussi, filtri, chat, AI e aggiornamenti.
- Screenshot reali dell'app in `site/assets/`: `app-chat.png`, `app-simkl.png`, `app-simkl-shelf-hd.jpg`; logo `cinematch.png`; font in `site/assets/fonts/`.
- Copy reale in `site/index.html`; cronologia release e repository GitHub pubblici.
- Nessuna testimonianza, press, numero di utenti o prezzo: il lavoro futuro non deve inventarli.

## Product Principles

1. Onestà prima di tutto: se un film non copre i criteri la card lo dice, e la chat non riempie la lista.
2. Film reali, mai inventati: l'AI propone, il motore di vincoli valida, TMDB conferma.
3. Ogni risultato spiega il perché e offre un consiglio pratico.
4. Rispetto del tempo: risposta rapida, approfondimento su richiesta.
5. Locale e privato: chiavi, configurazione e cache sul PC dell'utente; nessun polling in background.

## Accessibility & Inclusion

- Percorso da tastiera completo nell'app (Tab, Invio/Spazio, `/`, Esc) con focus sempre visibile; il sito ha uno skip link. Nessuno standard formale (es. WCAG) dichiarato.
