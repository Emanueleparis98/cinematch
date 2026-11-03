# CineMatch 3.3.0 — player in-chat

Questa versione sposta l’esperienza del player **dentro la conversazione**.
Cliccando una locandina non si apre più una nuova scheda: un pannello CineMatch
compare sopra la chat, mostra il picker delle sorgenti e avvia il film nello
stesso pannello.

## Cosa cambia

- Pannello in-chat con backdrop sfocato, `<dialog>` nativo e focus trap.
- Picker con poster, titolo, anno, durata, generi, voto, regia, cast e trama.
- Sorgenti raggruppate per addon, con badge di qualità, codec, audio, lingua e
  dimensione; la prima sorgente riproducibile è marcata **Consigliata**.
- Player con controlli custom, timeline, volume, tracce audio/sottotitoli,
  schermo intero, Picture-in-Picture (video nativo) e drawer per cambiare
  sorgente senza chiudere il film.
- Tema del picker adattivo alla chat (chiaro/scuro) con accento del servizio;
  il player resta sempre scuro.
- Ripresa della posizione nella sessione: riaprendo la locandina compare
  **Riprendi da …**.
- Fallback automatico alla scheda `watch.html` se l’iframe non può essere
  incorporato o non risponde.
- La pagina **Addon e player** resta disponibile per collegare, attivare e
  rimuovere i manifest Stremio/Nuvio.

## Architettura

```
locandina (enrich.js)
  -> overlay.js (content script)
       -> <dialog> in Shadow DOM
            -> iframe chrome-extension://…/cinema.html?movie=ID
                 -> cinema-app.js
                      -> TMDB + addon Stremio/Nuvio
                      -> player-engine.js (nativo / HLS / libmedia)
```

- `overlay.js` crea il guscio, gestisce apertura/chiusura, blocco dello scroll,
  ripristino del focus, tema della chat, timeout e fallback.
- `cinema.html` è una pagina dell’estensione caricata in un iframe: la CSP e le
  API dell’estensione restano nel contesto corretto, isolato dalla pagina.
- `cinema-app.js` gestisce picker, filtri, import JSON, drawer, resume e messaggi.
- `player-engine.js` incapsula la scelta tra `<video>` nativo, HLS.js e i
  decoder libmedia, con le stesse strategie del player 3.2.0.
- `watch.html` resta la pagina standalone per la gestione addon e il fallback.
- `tmdb.js` espone ora durata, generi, trama, voto e numero di voti.
- `background.js` autorizza `cinema.html` per i dettagli film e le immagini TMDB.

### Sicurezza

- L’iframe è `chrome-extension://` e viene dichiarato in
  `web_accessible_resources` solo per i quattro host chat supportati.
- I messaggi genitore ⇄ iframe sono validati con `event.source` e origin
  `chrome-extension://`.
- La pagina della chat non riceve la credenziale TMDB, i flussi o i dati degli
  addon: tutto avviene nel contesto dell’estensione.
- I permessi opzionali per i server video vengono richiesti solo al click di
  riproduzione.

## POC tecnica

Prima dell’implementazione è stata verificata una pagina con CSP
`frame-src 'none'`:

| Verifica | Esito |
|---|---|
| iframe `chrome-extension://` caricato nonostante la CSP | ✅ |
| `postMessage` genitore ⇄ iframe | ✅ |
| fullscreen richiesto dall’iframe | ✅ |
| API `chrome.permissions` disponibile nell’iframe | ✅ |
| permesso host già concesso leggibile | ✅ |

La POC completa è documentata in `docs/prototype/README.md`.

## Collaudo

- 74 test Node su riferimenti, TMDB, stream, playback e range.
- 26 controlli browser del pannello, della ricerca e dell’inserimento prompt.
- 21 controlli browser sulle immagini TMDB e sulla decorazione delle risposte.
- 34 controlli Chromium del player, inclusi:
  - apertura del pannello in-chat e ripristino del focus;
  - chiusura con Esc dall’interno dell’iframe;
  - player MP4, HLS, MKV HEVC/AC3, DASH, VVC e AVI;
  - cambio tema chiaro/scuro;
  - import JSON;
  - resume di sessione;
  - fullscreen dentro l’iframe;
  - fallback e pagina standalone.
- 8 controlli dei decoder software libmedia.

## Limiti noti

- DeepSeek non è raggiungibile dalla rete usata per i test automatici (WAF);
  il QA visivo su DeepSeek va fatto manualmente. Gli altri host e le fixture
  locali sono coperti dai test.
- La gestione addon resta nella pagina dedicata: il pannello in-chat offre il
  collegamento, non un editor completo dei manifest.
- Torrent, DRM e header che richiedono un proxy restano soggetti ai limiti delle
  sorgenti.
- Il tema viene letto all’apertura del pannello; un cambio tema della chat
  mentre il pannello è aperto viene recepito alla riapertura.
- La ripresa è locale alla sessione del browser e non viene sincronizzata.

## Installazione e build

```sh
python scripts/build-extension.py
```

Il pacchetto è `dist/CineMatch-Chrome-3.3.0.zip`. Per provarlo: estrai lo ZIP,
apri `chrome://extensions`, attiva la modalità sviluppatore, carica la cartella
non pacchettizzata e apri una chat supportata.
