# CineMatch — estensione browser 3.4.0

**La versione corrente è un'estensione per Chrome e Edge**, con 74 voci sulle
correnti del cinema e un generatore locale di prompt per ChatGPT, DeepSeek,
Claude e Gemini. Si apre accanto alla chat e inserisce il testo nell'editor:
l'invio rimane all'utente. Aggiunge automaticamente locandine, cast e registi
sotto i titoli nelle risposte. Non richiede app Windows o API AI a pagamento; le immagini usano una
credenziale personale TMDB, configurabile nelle opzioni dell'estensione.

## Installare l'estensione

Estrai `CineMatch-Chrome-3.4.0.zip`, apri `chrome://extensions`, attiva
**Modalità sviluppatore**, premi **Carica estensione non pacchettizzata** e scegli
la cartella che contiene `manifest.json`. Fissa CineMatch alla barra dal menu
Estensioni e premi l'icona per aprire il pannello. In Edge: `edge://extensions`.

[Istruzioni complete, permessi e privacy](extensions/cinematch/README.md).
Chrome 116+; Edge con supporto `sidePanel`. Pacchetto manuale, non pubblicato sullo store.

## Flusso nel browser

1. Cerca per corrente, autore, opera o sinonimo e consulta contesto e fonti.
2. Seleziona una o più voci e passa a **La tua richiesta**.
3. Scegli obiettivo, quantità libera o precisa, preferenze, esclusioni e titoli già visti.
4. Genera e modifica la richiesta. Apri la chat, poi premi **Inserisci nella chat**.
5. Rileggi e invia dal sito. Una bozza già presente richiede la tua scelta prima
   di essere modificata. **Copia** permette di usare qualsiasi altra chat.

Per le immagini collega TMDB una volta dal pulsante nell'intestazione del pannello.
Le immagini appaiono da sole sotto ciascun titolo mentre la chat risponde:
locandina, regista e primi interpreti. Seguono colori e caratteri della chat,
senza pannelli, titoli duplicati, pulsanti, impostazioni o moduli di ricerca.
Gli abbinamenti incerti vengono omessi, lasciando leggibile il testo originale.
Emoji, numerazioni e sottotitoli tradotti vengono gestiti automaticamente.

Vengono analizzate localmente le risposte dell'AI, non i messaggi dell'utente.
TMDB riceve titoli, anni o nomi, non la conversazione. Nessun invio automatico
di messaggi e nessuna API AI. Quattro host AI e i due host ufficiali TMDB autorizzati;
nessuna cronologia o accesso generale iniziale ai siti. Addon e sorgenti video
richiedono permessi opzionali per singolo host. La chiave non viene esposta alle chat.
La bozza è conservata in memoria per la sessione del browser, senza archivio
progressivo di film o richieste. I limiti della chat rimangono quelli del servizio.

## Addon e player integrato

Clicca una locandina nella chat: si apre un pannello CineMatch **sopra la
conversazione**, senza cambiare scheda. Il picker mostra poster, regia, cast e
sorgenti raggruppate per addon; scegli un flusso e il player parte nello stesso
pannello. Il tema del picker segue la chat in tempo reale, il player resta scuro,
e la posizione di visione viene ricordata nella sessione. Dal pulsante **Addon**
puoi collegare, attivare e rimuovere i manifest compatibili con Stremio/Nuvio
senza uscire dalla chat; la pagina **Addon e player** resta disponibile per la
gestione completa e come fallback.

Sono supportati video HTTP, HLS e DASH, con pausa, seek, tracce audio, sottotitoli
e schermo intero. Il motore integrato libmedia aggiunge decoder locali per MKV/AVI,
HEVC, H.266/VVC in MP4 e audio AC3/E-AC3/DTS. La riproduzione avviene a blocchi:
il film non viene convertito o salvato interamente prima dell’avvio. Puoi importare
anche liste JSON. Torrent, DRM e header che richiedono un proxy restano soggetti
ai limiti delle sorgenti. Se un sito non consente l’iframe, il pannello offre il
fallback nella scheda dedicata.

[Uso, compatibilità, privacy e collaudo del player](docs/estensione-3.0.0-player.md).
[Correzione avvio e riconoscimento dei codec, 3.0.1](docs/estensione-3.0.1-codec.md).
[Motore e decoder integrati, 3.1.0](docs/estensione-3.1.0-decoder.md).
[Recupero dei permessi dei server video, 3.1.1](docs/estensione-3.1.1-redirect.md).
[Lettura anticipata per flussi pesanti, 3.1.2](docs/estensione-3.1.2-buffer.md).
[Correzione delle risposte Range estese, 3.1.3](docs/estensione-3.1.3-range.md).
[Accesso ai server Real-Debrid, 3.1.4](docs/estensione-3.1.4-cors.md).
[Correzione degli intervalli vicino a EOF, 3.1.5](docs/estensione-3.1.5-eof.md).
[Avvio dei flussi lenti e dei grandi remux, 3.1.6](docs/estensione-3.1.6-avvio.md).
[Codec e accelerazione, 3.2.0](docs/estensione-3.2.0-codec.md).
[Player in-chat, 3.3.0](docs/estensione-3.3.0-in-chat.md).
[Addon nel pannello e tema live, 3.4.0](docs/estensione-3.4.0-pannello.md).

## Sviluppo dell'estensione

```sh
python scripts/build-extension.py
pnpm install --frozen-lockfile
pnpm test
pnpm exec playwright install chromium
pnpm test:browser
pnpm test:images
pnpm test:player
pnpm test:codecs
pnpm test:throughput
```

Python usa solo la libreria standard per assemblare `dist/CineMatch-Chrome` e lo
ZIP. Node 24 e pnpm 11.25.0 servono soltanto ai test. La distribuzione non contiene
Node, Python o dipendenze remote. Il workflow **Browser extension** verifica e
pubblica lo ZIP come artefatto CI. I test browser usano Chromium isolato e pagine
simulate: nessun account reale o richiesta AI. I siti possono cambiare editor;
in quel caso rimane il copia/incolla.

Il pannello laterale e la connessione TMDB usano un tema fisso scuro; il picker in-chat segue il tema della chat e il player resta scuro. Spunte disegnate e
menu personalizzati sostituiscono i menu del sistema, con navigazione da tastiera.
Le immagini arrivano subito sotto la riga del titolo, prima della descrizione,
anche quando il provider mette entrambi nello stesso paragrafo.

Verifica immagini e controlli: [collaudo 2.1.0](docs/estensione-2.1.0-verifica.md).

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
