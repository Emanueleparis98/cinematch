# CineMatch 3.18.0 — Filtri sorgenti

La 3.18.0 aggiunge una sezione filtri alla schermata **Scegli una sorgente**, per
trovare subito il flusso giusto anche con decine di risultati.

## Cosa cambia

- Nuova barra filtri sotto gli addon, con tre menu:
  - **Qualità** — 4K, 1080p, 720p, SD, Altre;
  - **Tipo** — Diretti, Torrent, Esterni, Altri;
  - **Lingua** — ITA, ENG, FRA, SPA, GER, JPN, KOR, SUB-ITA, MULTI, Non indicata.
- Ogni voce mostra il numero di sorgenti disponibili; le opzioni si aggiornano
  in base agli addon selezionati.
- I filtri si combinano tra loro e con il filtro per addon.
- Il pulsante **Azzera** riporta tutto a “Tutti” ed è disattivato quando non ci
  sono filtri attivi.
- La riga di stato diventa del tipo “12 di 41 sorgenti · 10 riproducibili ·
  filtri attivi”.
- Lingue e qualità vengono lette dal nome, dalla descrizione e dal filename
  della sorgente; nessun dato viene inviato altrove.
- La barra è compatta, si adatta alla larghezza e resta nascosta quando c’è una
  sola sorgente o nella modalità CB01 diretta.

## Cosa non cambia

- Il player, il drawer delle sorgenti e la gestione addon restano invariati.
- Correnti, impostazioni e aggiornamenti restano nel popup dell’icona.
- Nessuna modifica a credenziali, addon o preferenze.

## Verifica

- 38 controlli player/browser, compreso un test che importa sorgenti 4K/1080p/
  720p, ITA/ENG e torrent, e verifica i filtri combinati e **Azzera**.
- Regressione completa: 86 test Node, 29 browser, 12 DeepSeek, 5 TMDB/TV,
  23 immagini, 8 codec, 7 CB01, 4 updater e throughput.
