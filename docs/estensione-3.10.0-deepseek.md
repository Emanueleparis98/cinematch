# CineMatch 3.10.0 — Correnti in DeepSeek

La 3.10.0 cambia il modo di aprire le correnti: niente più pannello laterale e
niente più pulsante flottante. L’icona dell’estensione apre le **Impostazioni**;
da lì si attiva un pulsante **Correnti** che compare direttamente nel composer
di DeepSeek, accanto ai pulsanti nativi **Pensiero Profondo** e **Ricerca**.

## Cosa cambia

- **Rimosso il pannello laterale** (`side_panel` e permesso `sidePanel`).
- **Rimosso il pulsante flottante CineMatch** da tutte le chat.
- Click sull’icona dell’estensione → pagina **Impostazioni** in una scheda.
- Nuova opzione **Correnti in DeepSeek**, disattivata di default.
- Il pulsante Correnti esiste solo su `chat.deepseek.com` e solo se l’opzione è
  attiva.
- Il pannello CineMatch completo si apre in **overlay** solo dopo un click
  esplicito sul pulsante. Non si apre da solo, non è laterale e non viene
  caricato finché non serve.

## Pulsante nella chat

Il pulsante viene inserito nella stessa riga dei due toggle nativi di DeepSeek:

- stesse classi native `ds-toggle-button` e `ds-toggle-button--m`;
- icona discreta e testo **Correnti**;
- stessa altezza, raggio, font, spaziatura e stato hover dei pulsanti vicini;
- accessibile da tastiera con `Tab`, `Invio`, `Spazio` ed `Esc`;
- `aria-haspopup="dialog"` e `aria-expanded` durante l’apertura;
- si adatta al tema chiaro/scuro e alle larghezze strette.

Il pulsante viene reinserito automaticamente dopo i re-render React del composer,
senza duplicati. Se il composer non è presente (login o caricamento), non viene
mostrato nulla; compare quando DeepSeek rende disponibile la chat.

## Pannello in overlay

Il click su Correnti apre il pannello CineMatch esistente in un `<dialog>` sopra
la chat, con:

- Esplora, ricerca e filtri;
- dettaglio della corrente;
- fino a quattro correnti;
- Richiesta libera e Con una corrente;
- generazione, copia e inserimento del prompt nel composer DeepSeek.

L’invio del messaggio resta all’utente. `Esc`, il pulsante × e il click sul
backdrop chiudono il pannello e riportano il focus al pulsante Correnti.

## Impostazioni

In **Opzioni → Correnti in DeepSeek**:

- checkbox **Mostra il pulsante Correnti nella chat DeepSeek**;
- default: disattivo;
- attivazione e disattivazione hanno effetto immediato sulle schede DeepSeek
  aperte;
- disattivando l’opzione il pulsante viene rimosso e l’overlay viene chiuso;
- le altre chat non ricevono il pulsante.

Il vecchio pannello laterale non è più disponibile. La pagina completa del
pannello resta raggiungibile dall’overlay tramite il suo pulsante di espansione,
in una scheda dedicata.

## Privacy e permessi

- Nessun nuovo permesso host.
- Rimosso il permesso `sidePanel`.
- Il content script del pulsante viene caricato solo su `chat.deepseek.com`.
- Il pannello è caricato solo dopo il click e resta un documento
  dell’estensione; la chat non riceve accesso ai dati interni.
- Le bozze restano nella sessione del browser.

## Verifica

- **9 controlli browser DeepSeek**: toggle disattivo/attivo, un solo pulsante
  dopo i due nativi, classi e misure, apertura overlay, inserimento prompt,
  focus, rerender, disattivazione e layout stretto.
- **Test browser del pannello**: 29 controlli aggiornati per l’assenza del
  pannello laterale e per l’apertura delle impostazioni dall’icona.
- **Regressione completa**: 86 test Node, CB01, TMDB/TV, immagini, player,
  codec, updater e throughput.

La verifica manuale va fatta su DeepSeek con account loggato, perché la chat
richiede l’accesso per mostrare il composer.
