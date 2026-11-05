# CineMatch 3.13.0 — Popup impostazioni dall’icona

La 3.13.0 cambia il click sull’icona: non apre più le correnti, ma un **popup
compatto con le sole impostazioni**. Le correnti e la richiesta libera restano
nel composer di DeepSeek, dietro il pulsante **Correnti**.

## Cosa cambia

- Click sull’icona → popup ancorato all’icona, 400 × 560 px, senza nuova pagina
  e senza pannello laterale.
- Il popup usa lo stesso sistema visivo del popover DeepSeek: tema chiaro/scuro
  del sistema, intestazione compatta, schede e piè di pagina.
- Scheda **Connessione**:
  - chiave API v3 o API Read Access Token TMDB;
  - mostra/nascondi la chiave;
  - **Verifica e salva** con controllo reale della credenziale;
  - **Rimuovi**;
  - opzione **Ricorda su questo dispositivo**.
- Scheda **Funzioni**:
  - **Correnti in DeepSeek**;
  - **Pulsante Flussi su TMDB**;
  - **Modalità CB01**.
- Piè di pagina con versione installata e pulsante **Impostazioni complete**,
  che apre la pagina estesa con aggiornamenti e addon.
- I permessi TMDB e CB01 vengono richiesti con un click esplicito sul toggle,
  come nella pagina delle impostazioni.

## Cosa non cambia

- Le correnti non sono nel popup dell’icona.
- Il pulsante **Correnti** nel composer di `chat.deepseek.com` continua ad
  aprire il popover con ricerca, **74 correnti**, fino a quattro selezioni e
  **Richiesta libera**.
- La pagina **Impostazioni complete** resta il posto per aggiornamenti, addon,
  player e opzioni avanzate.

## Verifica

- **11 controlli DeepSeek**: popup dell’icona con sole impostazioni, schede,
  versione, toggle DeepSeek senza permessi, popover in chat, inserimento prompt,
  rerender, disattivazione e layout stretto.
- Regressione completa: 86 test Node, 29 browser, 5 TMDB/TV, 23 immagini,
  36 player, 8 codec, 7 CB01, 4 updater e throughput.
