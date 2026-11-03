# CineMatch 3.1.2 — lettura anticipata dei flussi pesanti

Il lettore precedente completava una richiesta da 1 MiB, consumava il blocco e
solo allora ne richiedeva un altro. Ogni richiesta partiva dal link dell’addon
anche quando questo reindirizzava a un CDN. La combinazione di richieste piccole,
latenza e reindirizzamenti ripetuti poteva limitare la banda utile e causare
buffering su file ad alto bitrate, indipendentemente dalla capacità del decoder.

## Intervento

- Il primo prefisso rimane 64 KiB. I successivi blocchi arrivano fino a 8 MiB.
- Mentre il blocco attivo viene consumato, viene richiesto il successivo. La
  cache I/O mantiene fino a due blocchi, 16 MiB in RAM; heap, pacchetti e frame
  dei decoder richiedono memoria aggiuntiva. Non è un limite alla RAM totale.
- Il CDN restituito dal fetch autorizzato viene riusato soltanto nella sessione.
  Se risponde 401/403/404/410, viene risolto di nuovo il link iniziale una volta.
  Rate limit e permessi mancanti non avviano riprove in loop.
- I salti fuori dalla finestra annullano la lettura anticipata obsoleta. Errori
  speculativi vengono mostrati solo quando il relativo blocco serve realmente.
- Si verificano intervalli, dimensione e corpi incompleti per evitare letture
  errate dopo seek, scadenza o risposte Range ridotte dal server.
- Il precaricamento dei pacchetti nel motore passa da 3 a 6 secondi. WebCodecs e
  accelerazione hardware mantengono la selezione esistente.

Non viene scaricato o convertito l’intero film prima della visione. Nessun
blocco video o link CDN viene scritto su storage.local, IndexedDB o file da
CineMatch. La normale gestione temporanea del browser è distinta da un archivio.

## Verifica riproducibile

`pnpm test:throughput` serve in RAM un file virtuale da 50 GiB, senza materiale
cinematografico o account. Consuma 32 MiB attraverso un addon simulato con 60 ms
di latenza e un CDN con altri 60 ms. Il confronto riproduce il modello precedente
(1 MiB seriale, risoluzione dell’addon a ogni blocco) e il lettore aggiornato:

| Misura locale | Modello precedente | Lettore aggiornato |
| --- | ---: | ---: |
| Richieste HTTP del lettore | 33 | 6 |
| Risoluzioni tramite addon | 33 | 1 |
| Tempo misurato | 4,651 s | 0,503 s |
| Banda utile del test | 57,7 Mbit/s | 533,6 Mbit/s |

Questi tempi descrivono solo il benchmark locale con latenza controllata e non
misurano la sorgente privata dell’utente. Le asserzioni CI verificano richieste,
reindirizzamenti, integrità e memoria; non impongono una soglia temporale fragile.
I test unitari coprono anche file oltre 4 GiB, cancellazione, blocchi anticipati,
scadenza del CDN, risposte Range accorciate e corpi troncati. Le suite Chromium
verificano riproduzione, seek, cambio sorgente e decoder audio/video reali su
filmati sintetici.

Un server che eroga meno del bitrate del film può continuare a causare pause.
Questa modifica rimuove un limite del lettore; non aumenta la banda della rete,
non cambia la qualità della sorgente e non certifica la fluidità di ogni remux 4K.

API del motore:
https://github.com/zhaohappy/libmedia/blob/152f629d3021fd8013efa464fcb7b55f9fbe7753/packages/avplayer/src/AVPlayer.ts
