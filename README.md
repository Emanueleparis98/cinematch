# CineMatch — estensione browser 1.1.0

**La versione corrente è un'estensione per Chrome e Edge**, con 74 voci sulle
correnti del cinema e un generatore locale di prompt per ChatGPT, DeepSeek,
Claude e Gemini. Si apre accanto alla chat e inserisce il testo nell'editor:
l'invio rimane all'utente. Aggiunge locandine, cast e registi alle risposte su
richiesta. Non richiede app Windows o API AI a pagamento; le immagini usano una
credenziale personale TMDB, configurabile nelle opzioni dell'estensione.

## Installare l'estensione

Estrai `CineMatch-Chrome-1.1.0.zip`, apri `chrome://extensions`, attiva
**Modalità sviluppatore**, premi **Carica estensione non pacchettizzata** e scegli
la cartella che contiene `manifest.json`. Fissa CineMatch alla barra dal menu
Estensioni e premi l'icona per aprire il pannello. In Edge: `edge://extensions`.

[Istruzioni complete, permessi e privacy](extensions/cinematch/README.md).
Chrome 116+; Edge con supporto `sidePanel`. Pacchetto manuale, non pubblicato sullo store.

## Flusso nel browser

1. Cerca per corrente, autore, opera o sinonimo e consulta contesto e fonti.
2. Seleziona una o più voci e passa a **Il tuo prompt**.
3. Scegli obiettivo, quantità libera o precisa, preferenze, esclusioni e titoli già visti.
4. Genera e modifica la richiesta. Apri la chat, poi premi **Inserisci nella chat**.
5. Rileggi e invia dal sito. Una bozza già presente richiede la tua scelta prima
   di essere modificata. **Copia** permette di usare qualsiasi altra chat.

Per le immagini apri **Immagini TMDB**, salva la credenziale e ricarica la chat.
Sulla risposta premi **Mostra locandine e cast**; apri **Regia** o **Cast** per i
ritratti. Sono disponibili scelta fra omonimi, correzione e ricerca manuale.

Viene letta solo la risposta selezionata al clic, non i messaggi dell'utente.
TMDB riceve titoli, anni o nomi, non la conversazione. Nessun invio automatico
di messaggi e nessuna API AI. Quattro host AI e i due host ufficiali TMDB autorizzati;
nessuna cronologia o accesso generale ai siti. La chiave non viene esposta alle chat.
La bozza è conservata in memoria per la sessione del browser, senza archivio
progressivo di film o richieste. I limiti della chat rimangono quelli del servizio.

## Sviluppo dell'estensione

```sh
python scripts/build-extension.py
pnpm install --frozen-lockfile
pnpm test
pnpm exec playwright install chromium
pnpm test:browser
pnpm test:images
```

Python usa solo la libreria standard per assemblare `dist/CineMatch-Chrome` e lo
ZIP. Node 24 e pnpm 11.25.0 servono soltanto ai test. La distribuzione non contiene
Node, Python o dipendenze remote. Il workflow **Browser extension** verifica e
pubblica lo ZIP come artefatto CI. I test browser usano Chromium isolato e pagine
simulate: nessun account reale o richiesta AI. I siti possono cambiare editor;
in quel caso rimane il copia/incolla.

Verifica immagini e privacy: [collaudo 1.1.0](docs/estensione-1.1.0-verifica.md).

La raccolta conserva 44 movimenti, 14 tendenze/scenari, 12 generi/cicli,
3 tradizioni e 1 tecnica. [Audit storico](docs/correnti-0.14.0-audit.md),
[dati canonici](plugins/cinema-currents/assets/movements.json).

---

## Versione desktop precedente — 0.14.0

Guida Windows dedicata alle correnti del cinema e generatore locale di prompt
per chat esterne. I paragrafi seguenti documentano la versione precedente;
non sono necessari per usare l'estensione browser.

## Uso

1. Cerca una corrente, un sinonimo, una figura o un'opera rappresentativa.
2. Filtra per categoria, periodo e area; consulta caratteristiche, contesto e fonti.
3. Premi **Prepara una ricerca**. Per più voci usa le caselle nell'elenco.
4. Scegli obiettivo, quantità automatica o precisa, approfondimento e tipo di relazione.
5. Scrivi la tua richiesta ed eventualmente preferenze, esclusioni e titoli già visti.
6. Premi **Genera il prompt**, modifica il testo se vuoi e usa **Copia prompt**.
7. Con **Apri chat** apri ChatGPT, DeepSeek, Claude, Gemini o un indirizzo HTTPS
   personale. Incolla e invia tu il testo; puoi usarlo anche in una chat già aperta.

**Ctrl+F** porta alla ricerca delle correnti; **Ctrl+Invio** genera il prompt
nel compositore. Le azioni principali rimangono sopra l'area scorrevole.
Sotto 1000 pixel logici, i passaggi fra elenco, scheda e compositore usano
l'intera larghezza. Dimensione minima supportata: 800×600.

## Contenuti e precisione storica

La raccolta comprende **74 voci**: **44 movimenti storici**, **14 tendenze/scenari**,
**12 generi o cicli**, **3 tradizioni**, **1 tecnica/ambito produttivo**.
Le 26 voci precedenti sono state riviste e sono state aggiunte 48 voci.
Non tutte le voci sono movimenti storici: le categorie sono dichiarate.

Ogni scheda comprende nome originale o internazionale, sinonimi, periodo indicativo,
aree, caratteristiche, figure di riferimento, esempi di opere, limiti interpretativi
ed approfondimenti. Le date non certificano appartenenze e le aree non sono filtri
che escludono automaticamente opere. La raccolta è editoriale e ampliabile;
non viene presentata come il censimento universale della storia del cinema.

Fonti e confronto delle 26 voci precedenti: [audit della raccolta](docs/correnti-0.14.0-audit.md).
La raccolta canonica è [movements.json](plugins/cinema-currents/assets/movements.json).

## Plugin separato

**Cinema Currents 1.0** è un pacchetto autonomo, utilizzabile anche senza CineMatch:
una libreria Rust e un eseguibile con protocollo JSON. Il generatore dell'app usa
la stessa libreria, senza processi o servizi in background.

```powershell
.\cinema-currents.exe catalog
Get-Content request.example.json -Raw | .\cinema-currents.exe generate
```

[Contratto, sorgenti e istruzioni del plugin](plugins/cinema-currents/README.md).
Il pacchetto ZIP Windows contiene eseguibile, esempio e sorgenti riutilizzabili.
Il plugin è incluso anche nella cartella `plugins` dell'installazione.

## Rete, costi e dati

La consultazione della guida e la generazione del prompt funzionano offline.
L'app non richiede API key, non chiama servizi AI, non invia messaggi, non legge
account e non scarica schede, locandine o database. Nessun prompt o risposta
viene salvato automaticamente; le richieste rimangono nella RAM della sessione.
La raccolta testuale è inclusa nel programma e occupa circa 86 KB nei sorgenti.

Le fonti e la chat si aprono nel browser soltanto su azione dell'utente.
Gli indirizzi delle chat non contengono il prompt o altri dati personali.
Servono Internet e gli eventuali account richiesti dal sito. Qualità, disponibilità,
prezzi e limiti della chat dipendono dal servizio esterno.

L'aggiornamento conserva i file personali e le credenziali delle versioni precedenti;
la nuova app non li usa. Per futuri aggiornamenti è disponibile l'installer completo
nella sezione Releases del repository.

## Sviluppo e distribuzione

```powershell
cargo test --workspace --all-targets
cargo clippy --workspace --all-targets -- -D warnings
cargo check --features wgpu
cargo build --release --workspace
.\scripts\package-plugin.ps1
ISCC.exe /DMyAppVersion=0.14.0 installer\cinematch.iss
```

I documenti delle versioni precedenti descrivono comportamenti storici.
La verifica corrente è in [correnti-0.14.0-verifica.md](docs/correnti-0.14.0-verifica.md).
