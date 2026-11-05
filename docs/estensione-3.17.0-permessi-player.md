# CineMatch 3.17.0 — Consenso ai server video

> **Aggiornata dalla 3.18.0** con i filtri per qualità, tipo e lingua nella
> scelta delle sorgenti. Vedi
> [Filtri sorgenti 3.18.0](estensione-3.18.0-filtri-sorgenti.md).

La 3.17.0 corregge il pulsante **Consenti …** nel player dentro la chat.

## Il problema

Quando un flusso usa un server diverso da quello dell’addon, il player mostra:

> Il flusso usa un altro server. Consenti l’accesso a raw.githubusercontent.com
> per leggere il video.

Il pulsante però si limitava a riprovare la stessa sorgente senza chiedere il
permesso. Il video restava fermo e il messaggio tornava.

## La correzione

- **Consenti [server]** ora richiede davvero l’accesso opzionale al dominio
  esatto del flusso, ad esempio `https://raw.githubusercontent.com/*`.
- Se il permesso viene concesso, la sorgente viene ricaricata e la riproduzione
  riparte.
- Se viene negato, il messaggio resta con lo stesso pulsante per riprovare.
- Vale per tutti i server video, non solo raw.githubusercontent.com: CDN degli
  addon, redirect e domini Real-Debrid.

## Verifica

- 37 controlli player/browser, compreso un test dedicato che clicca **Consenti**
  e verifica la richiesta del permesso esatto.
- Regressione completa: 86 test Node, 29 browser, 12 DeepSeek, 5 TMDB/TV,
  23 immagini, 8 codec, 7 CB01, 4 updater e throughput.
