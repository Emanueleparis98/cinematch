# CineMatch 3.0 — addon e player nel browser

## Uso

1. Ricarica l’estensione aggiornata e le schede delle chat.
2. Dal pannello CineMatch apri **Addon e player** (icona video in alto).
3. Incolla un link `https://…/manifest.json` oppure `stremio://…/manifest.json`.
   Accetta l’autorizzazione di Chrome per quel sito. I configuratori indicati
   dall’utente sono raggiungibili nella sezione espandibile dedicata; va copiato
   il manifest generato dopo la configurazione, non l’indirizzo del configuratore.
4. Clicca la locandina di un film nella risposta di DeepSeek, ChatGPT, Claude o
   Gemini: si apre una scheda CineMatch con titolo, anno, regista e sorgenti.
5. Scegli **Riproduci**. Se il server video è diverso, Chrome richiede accesso
   anche a quel sito. I controlli del video includono pausa, posizione, volume,
   schermo intero e le funzioni offerte dal browser.

Gli addon possono essere disattivati o rimossi. Rimuoverli cancella il manifest
salvato; per revocare l’accesso a un host usa le impostazioni dell’estensione in Chrome.
La modifica è disponibile nel pacchetto aggiornato; non modifica il profilo Chrome
personale o le estensioni già caricate senza ricaricamento dell’utente.

## Liste JSON

**Importa JSON** accetta file fino a 2 MB e 500 elementi; la sezione sotto le
sorgenti accetta anche un URL. Sono liste per il film/sessione corrente, non
cataloghi associati automaticamente a ogni film. Esempio:

```json
{"streams":[{"name":"La mia sorgente","title":"1080p · Italiano","url":"https://video.example/film.mp4"}]}
```

Le liste possono essere anche array di oggetti. I nomi sono trattati come testo,
mai come HTML; URL eseguibili e schemi non HTTP sono esclusi. Nessun torrent viene
scaricato, nessuno script dell’addon viene eseguito.

## Compatibilità effettiva

Il protocollo pubblico è quello Stremio usato anche da Nuvio: manifest, risorse
stream, tipi e prefissi. I film sono risolti usando l’IMDb ID verificato nella
scheda TMDB; `tmdb:` viene usato solo quando l’addon lo dichiara. I parametri e i
percorsi personali del manifest sono conservati. Massimo tre ricerche concorrenti;
timeout di 20 secondi, cancellazione delle ricerche superate e aggiornamento manuale.

Video diretti HTTP/HTTPS, MP4/WebM con codec supportati dal browser e HLS tramite
HLS.js 1.7.4 locale (Apache-2.0) sono integrati. Gli URL HLS senza estensione
sono riconosciuti con una richiesta HEAD limitata a quattro secondi. Sottotitoli
WebVTT inclusi nel flusso sono passati ai controlli nativi; il server deve consentirli.
I flussi `externalUrl` aprono il sito esterno soltanto su clic esplicito.

Il browser non sostituisce MPV o un motore BitTorrent: `infoHash`/magnet, archivi,
Usenet, DASH e sorgenti DRM non sono convertiti. Gli header personalizzati non
vengono falsificati nel browser: serve l’URL di un proxy configurato nell’addon.
La lista spiega questi casi e permette di scegliere un altro flusso. Redirect
dei manifest/JSON sono rifiutati: usare l’endpoint finale. I server HLS/CDN devono
consentire CORS o ricevere il permesso host richiesto dal player se falliscono.
Non viene promesso il funzionamento di ogni addon, codec, server o account.

## Dati e licenze

I soli dati persistenti nuovi sono manifest e preferenze di attivazione, massimo
20 addon. Gli URL personali possono contenere token e vanno mantenuti privati.
Le ricerche inviano agli addon l’ID del film, mai chat, credenziali TMDB o cookie.
Richieste JSON senza credenziali, referrer o cache persistente. Il player usa il
buffer temporaneo del browser: nessun archivio di film, download intenzionale del
file completo o copia persistente in storage dell’estensione. Il browser può
gestire autonomamente cache e buffer video: non si promette zero uso del disco.

NuvioDesktop è GPL-3.0. Non sono stati copiati codice, asset o player nativi:
implementazione indipendente del protocollo JSON documentato da Stremio.
HLS.js è distribuito con licenza e provenienza nel pacchetto.

Fonti primarie:
- https://github.com/NuvioMedia/NuvioDesktop
- https://github.com/Stremio/stremio-addon-sdk/blob/master/docs/protocol.md
- https://github.com/Stremio/stremio-addon-sdk/blob/master/docs/api/responses/manifest.md
- https://github.com/Stremio/stremio-addon-sdk/blob/master/docs/api/responses/stream.md
- https://github.com/video-dev/hls.js/tree/v1.7.4
- https://developer.chrome.com/docs/extensions/develop/concepts/network-requests

## Collaudo

101 verifiche riuscite: 40 test Node, 26 controlli del pannello/inserimento,
21 controlli delle immagini nelle chat controllate e 14 del player nel browser.
Revisione visiva indipendente: **ship**, limitata all’aggiunta di player/addon;
DESIGN.md e il suo sidecar restano quelli dell’identità scura già approvata.
Il controllo OSV del 2 novembre 2026 non ha restituito advisory pubblicate
associate a HLS.js 1.7.4; non sostituisce gli aggiornamenti futuri.

Suite dedicata con un server HTTP locale e video sintetico FFmpeg H.264/AAC:
clic reale su una locandina di una risposta DeepSeek controllata, associazione
TMDB/IMDb, installazione manifest, MP4 con decodifica e avanzamento del tempo,
pausa/seek, HLS con segmenti e URL HLS senza estensione. Controlli anche di JSON
invalido, markup/URL ostili, limiti, errori server, import non persistente, sorgenti
incompatibili, disattivazione/rimozione e layout a 320/600/1180 px.

Il dialogo nativo dei permessi opzionali non è automatizzabile in Chromium headless:
la suite usa una copia temporanea del pacchetto con permesso solo al server localhost;
il manifest di produzione rimane invariato e viene controllato separatamente.
Non sono stati usati account personali, token AI o flussi commerciali. I sette
configuratori sono collegamenti forniti dall’utente, non addon preinstallati o
certificati. Il collaudo dei loro flussi specifici richiede i manifest personali
e gli eventuali account/proxy previsti dai rispettivi addon. Edge non eseguito.
