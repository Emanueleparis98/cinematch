# CineMatch 3.15.0 — Aggiornamenti nel popup

> **Superata dalla 3.16.0.** Gli aggiornamenti restano nel popup; la 3.16.0
> compatta il layout della scelta sorgenti. Vedi
> [Layout sorgenti compatto 3.16.0](estensione-3.16.0-layout-sorgenti.md).

La 3.15.0 porta gli aggiornamenti dentro il **popup compatto delle impostazioni**.
Click su **Aggiornamenti** non apre più una nuova scheda: seleziona la scheda
**Aggiorna** nello stesso layout.

## Cosa cambia

- Terza scheda **Aggiorna** accanto a **Connessione** e **Funzioni**.
- Versione installata, stato del canale GitHub, barra di avanzamento e pulsanti
  **Scegli cartella**, **Controlla**, **Aggiorna ora** sono nel popup.
- Il download avviene direttamente dal repository: `latest.json`, ZIP della
  release e verifica SHA-256, senza scaricare nulla a mano.
- Il piè di pagina mostra la versione installata; non ci sono pulsanti che
  aprono pagine.
- La prima volta che serve autorizzare la cartella, CineMatch apre una **piccola
  finestra con lo stesso layout** (tipo popup, non una scheda). Serve perché
  Chrome chiude il popup dell’icona quando si apre il selettore di cartelle.
  Dopo l’autorizzazione, gli aggiornamenti successivi avvengono nel popup.
- La finestra di aggiornamento del pannello CineMatch resta disponibile come
  alternativa, ma non viene più aperta dal popup.

## Cosa non cambia

- Il popup resta la sola superficie delle impostazioni.
- Correnti e richiesta libera restano nel composer DeepSeek.
- Credenziali, addon e preferenze non vengono toccati.

## Verifica

- 12 controlli DeepSeek: popup con tre schede, aggiornamenti contenuti, apertura
  della sola finestra compatta per la cartella, toggle DeepSeek, popover in chat,
  inserimento prompt, rerender, disattivazione e layout stretto.
- Regressione completa: 86 test Node, 29 browser, 5 TMDB/TV, 23 immagini,
  36 player, 8 codec, 7 CB01, 4 updater e throughput.
