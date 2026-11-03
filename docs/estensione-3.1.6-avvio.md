# CineMatch 3.1.6 — avvio dei flussi lenti e dei grandi remux

## Causa riprodotta

Il probe iniziale leggeva solo i primi 64 KiB con un timeout di 8 secondi. Un
MKV grande o poco richiesto può impiegare più tempo del previsto prima di
restituire il primo byte, soprattutto dietro un link Real-Debrid o un CDN che
sta preparando il file.

Quando il probe andava in timeout, il player trattava la sorgente come un video
nativo e assegnava l’URL a `<video>`. Chromium non supporta il contenitore
Matroska: in alcuni casi non emetteva un evento `error`, ma `ended` con
`currentTime` a zero. Il gestore precedente mostrava quindi “Riproduzione
terminata” senza aver mai avviato i decoder integrati. Il difetto è stato
riprodotto in Chromium con un MKV servito dopo 9 secondi, sia con URL `.mkv` sia
senza estensione: la 3.1.5 mostrava “Riproduzione terminata” a schermo vuoto.

## Intervento

- Il probe attende fino a 20 secondi. Se non arriva alcun byte e il contenitore
  è ancora sconosciuto, viene fatto un secondo tentativo da 30 secondi.
- Il contenitore viene ricavato dai byte ricevuti, dall’URL dell’addon, dall’URL
  CDN risolto e, solo come indizio, dal nome/descrizione del flusso. Un URL
  `.mkv`, `.webm` o `.avi` non passa più al player nativo.
- Se il player nativo termina a zero senza errore, viene avviato il motore
  integrato con lo stesso URL. Lo stesso recupero avviene per `error` e per il
  rifiuto di `play()`.
- Il timeout di avvio dei decoder integrati passa da 25 a 90 secondi. I blocchi
  HTTP passano da 20 a 45 secondi; dopo un timeout il blocco viene ritentato una
  volta con una finestra da 1 MiB, che resta attiva per la sessione. Una risposta
  429/500/502/503/504 viene riprovata una volta, rispettando un eventuale
  `Retry-After` fino a 5 secondi.
- Se un remux espone prima una traccia TrueHD non supportata e poi una traccia
  AC3/E-AC3/AAC, il player seleziona automaticamente quella decodificabile.
  TrueHD senza alternative resta senza audio: il pacchetto non include quel
  decoder e il video può comunque proseguire.
- Non vengono scaricati o convertiti interi film. Restano i limiti di 8 MiB per
  blocco, 16 MiB di cache I/O e nessun salvataggio del video.

## Verifica

- 34 test Node, inclusi probe in timeout, scelta del contenitore da URL
  extensionless o CDN risolto, retry di un blocco in timeout con finestra più
  piccola e riuso della finestra ridotta.
- 27 controlli Chromium del player, inclusa una sorgente MKV servita dopo 9
  secondi con URL senza estensione: la 3.1.5 mostrava “Riproduzione terminata”,
  la 3.1.6 decodifica video e audio integrati.
- I test usano filmati sintetici e server locali; nessun account, link privato o
  film commerciale viene incluso. La fluidità di un remux 4K/HDR dipende ancora
  dalla decodifica hardware e dal computer, non è garantita da questa modifica.
