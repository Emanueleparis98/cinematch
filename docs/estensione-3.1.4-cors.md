# CineMatch 3.1.4 — accesso ai server Real-Debrid

Il log fornito dall’utente mostra CORS sui server di download dopo il redirect
Comet. La verifica del link diretto con GET/Range e lettura di soli 32 byte per
richiesta ha restituito intervalli corretti a offset 0, 65536 e 8388608. Nel
Chromium isolato, concedendo il permesso per i server di download, il lettore
3.1.3 ha aperto correttamente il prefisso da 64 KiB del medesimo link. Queste
prove riguardano accesso e intestazioni, non la riproduzione integrale del film.
Il link privato e la configurazione dell’account non vengono inclusi nei file.

## Correzioni

- Il clic Riproduci chiede i permessi Chrome prima di attendere la pulizia del
  vecchio player, mantenendo l’attivazione utente richiesta dal browser.
- Le sorgenti etichettate RD/Real-Debrid includono nella richiesta opzionale
  `https://*.download.real-debrid.com/*`. Il consenso copre i server di download
  variabili, senza concedere accesso generale al web o alle API dell’account.
- Anche il recupero di un redirect RD propone il permesso per il servizio e
  il pulsante **Consenti Real-Debrid**. Un rifiuto conserva il pulsante visibile.
  Nessun consenso viene concesso dall’estensione senza il dialogo di Chrome.
- Se un redirect fallisce per CORS, oppure restituisce una risposta 206 senza
  Content-Range leggibile, viene provata una sola richiesta diretta verso la
  destinazione osservata, dopo averne verificato il permesso. URL e query sono
  mantenuti soltanto in memoria. Non si usa un proxy esterno né si modificano
  le intestazioni CORS del server o le protezioni del browser.
- La correlazione dei redirect normalizza la codifica dei parametri (spazio
  `%20`/`+`), senza riscrivere il link effettivamente richiesto.
- Gli errori di permesso hanno priorità sui successivi errori generici. Eventi
  di caricamento/pausa non sovrascrivono più il messaggio di errore attivo.
- Il cambio sorgente mette in pausa i renderer prima di chiudere i canali del
  decoder, evitando richieste pendenti durante la distruzione del player.
- Una risposta 206 priva di Content-Range produce ora un errore specifico,
  distinto da un intervallo realmente errato. Restano i controlli e i limiti
  di lettura delle versioni precedenti.

## Verifica

I test unitari coprono i domini RD, i domini simili non autorizzati, i redirect
falliti o con intestazioni filtrate e l’assenza di richieste prima del consenso.
Il test Chromium verifica che il clic richieda il permesso durante l’attivazione
utente e che un rifiuto mostri il recupero. In un profilo separato con il grant
preconcesso, due hostname RD servono filmati HEVC/AC3 sintetici senza intestazioni
CORS: entrambi vengono decodificati. Le risposte sono intercettate dal test,
non provengono da un film remoto o dall’account personale.

Il dialogo nativo di Chrome non viene accettato automaticamente dal collaudo
headless. Nell’uso reale serve accettare la richiesta di accesso ai server RD.
Il permesso risolve l’accesso; non certifica codec, HDR/TrueHD o prestazioni di
ogni remux 4K. Il lettore non salva o converte preventivamente il film.

Fonti primarie:
- https://developer.chrome.com/docs/extensions/develop/concepts/network-requests
- https://developer.chrome.com/docs/extensions/reference/api/permissions
- https://developer.chrome.com/docs/extensions/reference/api/webRequest
