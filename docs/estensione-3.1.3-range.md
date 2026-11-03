# CineMatch 3.1.3 — correzione delle risposte Range estese

La 3.1.2 ha introdotto una regressione: rifiutava risposte HTTP 206 il cui byte
finale superava quello richiesto, anche se la posizione iniziale era corretta.
Un CDN che restituisce un intervallo più grande o fino alla fine del file
provocava quindi «Il server ha restituito un intervallo video non valido».

La 3.1.3 accetta questi intervalli e limita autonomamente allocazione e lettura
alla finestra richiesta, poi cancella il corpo residuo. Il primo prefisso resta
64 KiB e ciascun blocco successivo non supera 8 MiB. Non viene allocato o letto
l’intero intervallo annunciato dal server. La lettura anticipata della 3.1.2 e
il rinnovo dei link CDN rimangono attivi.

Il parser tollera spazi nell’intestazione Content-Range. Restano rifiutati offset
iniziali diversi da quelli richiesti, intervalli invertiti o oltre la dimensione
dichiarata, numeri non rappresentabili e corpi più corti della finestra necessaria.
La tolleranza per un CDN non viene usata per assegnare byte alla posizione sbagliata.

## Verifica

Due test riproducevano l’errore prima della correzione e passano dopo. Una sorgente
virtuale da 50 GiB annuncia risposte fino a EOF: il test controlla prefisso da
64 KiB, blocchi da 8 MiB, seek oltre 4 GiB e cancellazione del corpo residuo.
Non crea un file da 50 GiB e non usa un account esterno.

Il test Chromium serve un vero filmato sintetico HEVC/AC3 in MKV ignorando la
fine dell’intervallo richiesto. Verifica che la risposta sia effettivamente più
grande, che audio e video vengano decodificati e che il seek funzioni. Questo
scenario integra i test con server che rispettano esattamente Range.

Il flusso privato dell’utente non è stato interrogato. La correzione riguarda
una regressione riprodotta con lo stesso messaggio; altre anomalie dei server
restano distinguibili dai controlli sugli intervalli e dagli errori HTTP/rete.
