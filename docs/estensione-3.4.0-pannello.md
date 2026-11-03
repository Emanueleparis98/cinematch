# CineMatch 3.4.0 — Addon nel pannello e tema live

Questa versione rifinisce il player in-chat della 3.3.0 con la gestione degli
addon direttamente nel pannello, l’aggiornamento del tema in tempo reale e il
mantenimento della posizione quando si cambia sorgente.

## Novità

### Addon nel pannello

- Pulsante **Addon** nella barra del pannello, senza aprire nuove schede.
- Collegamento di un manifest Stremio/Nuvio con richiesta del permesso sul sito.
- Elenco degli addon collegati con nome, host, versione, attivazione e rimozione.
- Limite di 20 addon, messaggi di errore leggibili, configuratori utili e guida.
- Dopo ogni modifica il picker aggiorna le sorgenti del film corrente.
- La pagina **Addon e player** resta disponibile per la gestione completa e come
  fallback.

### Tema live

- Il pannello osserva i cambi di tema di `<html>`, `<body>` e `main`, oltre a
  `prefers-color-scheme`.
- Il picker riceve il tema aggiornato mentre la chat è aperta, con transizioni
  brevi di colori e superfici.
- Il player resta sempre scuro; il tema della chat non interferisce con il video.
- Il tema viene ricalcolato anche quando si riapre la locandina.

### Cambio sorgente

- Selezionando un’altra sorgente dal drawer, la riproduzione riparte dalla stessa
  posizione percentuale del flusso precedente (quando la durata è disponibile).
- La ripresa di sessione “Riprendi da…” resta disponibile alla riapertura.

### Rifiniture

- Stato di errore del pannello più leggibile e reset corretto dopo un aggiornamento.
- Messaggi di importazione al singolare/plurale.
- Layout della gestione addon centrato e adattivo, con form impilato su mobile.
- Transizioni del tema disattivate con `prefers-reduced-motion`.

## QA sui siti reali

Smoke test automatico con l’estensione caricata su Chromium:

| Sito | Esito |
|---|---|
| ChatGPT | pagina raggiungibile, content script iniettati, nessun errore dell’estensione |
| Claude | pagina di accesso raggiungibile, content script iniettati, errori solo del provider |
| Gemini | pagina raggiungibile, content script iniettati, errori CSP solo della pagina |
| DeepSeek | verifica umana del provider nella rete di test; content script iniettati |

Il test verifica che `CineMatchCinema` e i riferimenti siano presenti nel mondo
isolato dell’estensione. Le prove con conversazioni reali richiedono un account
autenticato e vanno fatte manualmente.

### Checklist manuale

1. Accedi a ChatGPT, Claude, Gemini e DeepSeek.
2. Chiedi un film e attendi la locandina.
3. Clicca la locandina: il pannello deve aprirsi sopra la chat.
4. Scegli una sorgente, avvia, metti in pausa, cerca e attiva lo schermo intero.
5. Apri **Sorgenti** e cambia flusso: la posizione percentuale deve essere mantenuta.
6. Passa dal tema chiaro a quello scuro nella chat: il picker deve aggiornarsi
   senza chiudere il player.
7. Apri **Addon**, collega un manifest, attivalo/disattivalo, rimuovilo.
8. Chiudi con Esc e riapri: il focus deve tornare alla locandina e la ripresa
   deve essere proposta.

## Collaudo

- 74 test Node.
- 26 controlli browser del pannello e dell’inserimento prompt.
- 21 controlli browser delle immagini TMDB.
- 35 controlli Chromium del player/overlay, inclusi:
  - apertura in-chat, chiusura con Esc, focus e blocco scroll;
  - tema chiaro/scuro live;
  - gestione addon: aggiunta, disattivazione e rimozione dal pannello;
  - MP4, HLS, MKV HEVC/AC3, DASH, VVC e AVI;
  - fullscreen, resume e import JSON.
- 8 controlli dei decoder software libmedia.

## Limiti noti

- I test reali con conversazioni autenticate restano manuali.
- Il tema viene letto da `<html>`, `<body>` e `main`; temi applicati solo a un
  contenitore molto interno potrebbero richiedere una riapertura.
- La gestione addon non modifica i configuratori esterni: il manifest va copiato
  dal sito dell’addon.
- Torrent, DRM e header che richiedono un proxy restano soggetti ai limiti delle
  sorgenti.

## Build

```sh
python scripts/build-extension.py
```

Pacchetto: `dist/CineMatch-Chrome-3.4.0.zip`.
