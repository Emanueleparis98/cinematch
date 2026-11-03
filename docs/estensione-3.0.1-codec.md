# CineMatch 3.0.1 — avvio e codec

Correzione del 3 novembre 2026 per l’errore generico mostrato all’avvio di una
sorgente Comet. La riga indicata dall’utente dichiara MKV, HEVC/H.265 e AC3 5.1.
In una sessione Chrome locale isolata `canPlayType` ha restituito supporto per
HEVC, H.264 e AAC, ma nessun supporto dichiarato per AC3/E-AC3. Il file remoto
specifico non è stato aperto: codec, disponibilità e accesso al server restano
fattori distinti; non si attribuisce con certezza ogni errore remoto al codec.

## Modifiche

- GET con `Range: bytes=0-65535` al posto della sonda HEAD: un endpoint può rifiutare
  HEAD e funzionare con GET. Per il video vengono seguiti i redirect HTTP; le
  richieste manifest/JSON mantengono la propria regola di rifiuto dei redirect.
- Prefisso letto e mantenuto in memoria fino a 64 KiB, timeout di 8 secondi,
  annullamento del reader e del fetch. La sonda non salva il film. Il server
  potrebbe ignorare Range: il client cancella la risposta al limite, senza
  promettere che ogni server invii esattamente 64 KiB sulla rete.
- HLS riconosciuto dal contenuto anche se il MIME è generico; HTML/JSON di errore
  e DASH vengono distinti dal video. Se CORS/rete impediscono la sonda, il risultato
  resta incerto e il player tenta la riproduzione nativa senza inventare una diagnosi.
- CodecID Matroska individuati nel prefisso e confrontati con il supporto del
  browser. La sonda non è un demuxer completo e può non trovare tracce oltre il
  prefisso. Una traccia AAC alternativa evita di dichiarare incompatibile tutto
  l’audio soltanto perché esiste anche una traccia AC3.
- Nomi, descrizioni e `behaviorHints.filename` producono avvisi indicativi e
  ordinamento preferenziale H.264/AAC. Le etichette non bloccano un flusso che il
  server ha già convertito: il controllo all’avvio usa il contenuto ricevuto.
- Codec effettivamente individuati e non supportati mostrano un messaggio preciso
  e **Scegli un altro flusso**, che porta alle alternative. Errori 401/403/404/429
  e guasti di connessione restano distinti; un errore generico successivo non
  sovrascrive la diagnosi già disponibile.

Non vengono aggiunti decoder, transcodifica, server, API AI, permessi obbligatori
o pacchetti esterni. Un MKV HEVC/AC3 non viene convertito automaticamente. Il
supporto finale dipende anche da profilo del codec, container e decoder del sistema.

## Verifica

47 test Node e 17 controlli Chromium del player riusciti: inclusi vero video
sintetico MKV HEVC/AC3, passaggio a MP4 H.264/AAC, HLS dietro redirect con HEAD 405
e MIME errato, etichette obsolete prima della conversione, 403 senza esposizione
del corpo privato e layout del messaggio a 320/1180 px. Il dialogo dei permessi
resta escluso dall’automazione headless; la copia di test concede solo localhost.

Nessun account esterno o API AI usato, nessun token privato inserito nei sorgenti
o nei test. Il link personale Comet fornito in chat non è stato richiesto, salvato
o aggiunto al repository. Il test dei codec in Chrome usa un profilo isolato.

Riferimenti primari del protocollo e dei limiti:
- https://github.com/g0ldyy/comet/blob/main/docs/advanced/03-streaming-and-debrid-flow.md
- https://developer.chrome.com/docs/extensions/develop/concepts/network-requests
- https://developer.chrome.com/docs/web-platform/best-practices/webcodecs
