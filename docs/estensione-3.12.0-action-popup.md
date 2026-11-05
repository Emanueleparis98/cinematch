# CineMatch 3.12.0 — Popup dall’icona

> **Superata dalla 3.13.0.** Il popup dell’icona ora contiene solo le
> impostazioni; le correnti restano nel composer DeepSeek. Vedi
> [Popup impostazioni 3.13.0](estensione-3.13.0-settings-popup.md). Questa pagina
> resta come documentazione della 3.12.0.

La 3.12.0 porta lo stesso popover compatto di DeepSeek anche sull’icona
dell’estensione: niente nuova pagina, niente pannello laterale.

## Cosa cambia

- Click sull’icona → si apre un piccolo popup ancorato all’icona, largo 400 px
  e alto 560 px.
- Dentro c’è lo stesso sistema delle correnti:
  - ricerca;
  - elenco delle 74 correnti;
  - fino a quattro selezioni;
  - modalità **Richiesta libera**;
  - opzioni richiudibili;
  - anteprima, copia e inserimento nella chat.
- Il pulsante con l’ingranaggio apre le **Impostazioni** senza lasciare il popup
  in primo piano.
- Se DeepSeek è aperto in una scheda, **Inserisci nella chat** porta il prompt
  nel suo composer e attiva quella scheda.
- Se DeepSeek non è aperto, il prompt si può comunque copiare; l’inserimento
  mostra un messaggio chiaro.
- Il popup segue il tema chiaro/scuro del sistema.

Il pulsante **Correnti** nel composer DeepSeek resta disponibile quando
l’opzione è attiva e apre lo stesso popup.

## Verifica

- **11 controlli DeepSeek**: popup dall’icona, stesso layout, 74 correnti,
  inserimento in DeepSeek, più toggle, ricarica, pulsante nativo, richiesta
  libera, rerender, disattivazione e layout stretto.
- Regressione completa: 86 test Node, 29 browser, 5 TMDB/TV, 23 immagini,
  36 player, 8 codec, 7 CB01, 4 updater e throughput.
