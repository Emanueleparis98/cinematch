# CineMatch 3.14.0 — Solo popup compatto

La 3.14.0 elimina la pagina delle impostazioni complete. Tutto ciò che serve per
configurare CineMatch sta nel **popup compatto dell’icona**; aggiornamenti e
addon restano nel pannello CineMatch, che esisteva già.

## Cosa cambia

- Rimossa `options.html` e la relativa logica: niente più “Impostazioni
  complete”, niente seconda pagina di configurazione.
- L’icona dell’estensione apre sempre e solo il **popup compatto delle
  impostazioni**:
  - **Connessione** per la chiave TMDB, verifica, salvataggio e rimozione;
  - **Funzioni** per Correnti in DeepSeek, Pulsante Flussi su TMDB e Modalità
    CB01.
- Il piè di pagina mostra la versione e il pulsante **Aggiornamenti**, che apre
  il pannello CineMatch direttamente sulla finestra di aggiornamento.
- L’updater è stato spostato nel **pannello CineMatch** (icona freccia in alto).
  La cartella dell’estensione si sceglie lì, in una scheda stabile: il popup
  dell’icona si chiuderebbe durante il selettore di cartelle.
- Il pulsante TMDB nel pannello apre il popup delle impostazioni.
- Il popup delle correnti in DeepSeek non contiene più l’ingranaggio delle
  impostazioni: le correnti restano nel composer, le impostazioni nell’icona.

## Cosa non cambia

- Le correnti e la richiesta libera restano nel pulsante **Correnti** di
  `chat.deepseek.com`.
- La pagina **Addon e player** resta raggiungibile dal pannello e dal player.
- Credenziali, addon e preferenze non vengono toccati dall’aggiornamento.

## Verifica

- 86 test Node, 29 browser, 11 DeepSeek, 5 TMDB/TV, 23 immagini, 36 player,
  8 codec, 7 CB01, 4 updater e throughput.
- L’updater viene provato dal pannello: apertura della finestra, scelta cartella,
  scrittura del pacchetto e conservazione dello storage.
