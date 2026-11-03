# CineMatch 3.1.1 — accesso al server video

Il messaggio «Connessione al server video non riuscita» poteva nascondere un
reindirizzamento dal sito dell’addon a un CDN privo del permesso host. Il probe
ignorava il problema e il lettore a blocchi riportava soltanto un errore generico.

Le richieste del probe e del lettore Range osservano ora la propria catena di
reindirizzamenti con `webRequest.onBeforeRedirect`. Il listener è temporaneo,
filtra l’initiator dell’estensione e segue esclusivamente l’URL iniziale e i
requestId corrispondenti. Il permesso `webRequest` non concede accesso generale
ai siti: i permessi host rimangono opzionali e specifici per server.

Se manca l’accesso a una destinazione, il player propone **Consenti [server]**.
Il clic richiede il permesso Chrome e riavvia la sorgente originale: non conserva
o riutilizza un link CDN potenzialmente scaduto. Una catena con più destinazioni
può richiedere più permessi. URL firmati, percorsi e query restano in memoria;
messaggi e recupero ricevono soltanto lo schema e l’hostname.

Gli errori di rete osservabili mostrano il codice `ERR_*` del browser e, dove
possibile, una spiegazione per DNS, certificato, timeout o connessione rifiutata.
Non viene attribuito automaticamente un errore di rete ai codec. I listener
sono rimossi quando la richiesta termina e i corpi sono cancellati nei percorsi
di errore. La decodifica resta quella della 3.1.0, senza conversione preventiva.

## Verifica

Il test Chromium riproduce un GET/Range da un host autorizzato che reindirizza
a un altro host, senza CORS né permesso di destinazione. Il fetch originale
fallisce; il player aggiornato identifica il server e mostra il pulsante di
autorizzazione senza esporre il token del link. In un secondo profilo temporaneo
con il permesso di destinazione preconcesso, la stessa catena riproduce realmente
video HEVC e audio AC3 5.1 in MKV. Il dialogo nativo di conferma Chrome non viene
automatizzato in headless. I test unitari coprono isolamento delle richieste,
propagazione del recupero, cancellazione e pulizia dei listener.

Questa prova conferma il difetto e la correzione per tale scenario. Il link
privato e l’account dell’utente non sono stati usati: il flusso remoto segnalato
può avere anche cause indipendenti, come scadenza o indisponibilità del server.

Fonti primarie:
- https://developer.chrome.com/docs/extensions/develop/concepts/network-requests
- https://developer.chrome.com/docs/extensions/reference/api/webRequest
