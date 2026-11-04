# CineMatch 3.8.0 — Flussi da TMDB e serie TV

La 3.8.0 aggiunge un pulsante **Flussi** alle locandine di film e serie TV
mostrate su themoviedb.org e alle immagini che CineMatch inserisce nelle chat.
Lo stesso pulsante riapre il player già esistente, che ora supporta anche le
serie TV con stagione ed episodio.

## Pulsante su TMDB

- Appare sui poster delle card di film e serie TV e sulla locandina principale
  della scheda, senza modificare il link originale.
- Resta fuori dal link: cliccarlo non apre TMDB, ma il selettore delle sorgenti.
- Non viene mostrato su foto di persone, backdrop o immagini senza un titolo.
- Si aggiorna con le liste dinamiche di TMDB tramite un MutationObserver con
  debounce e non crea pulsanti duplicati.
- Un clic apre l’overlay CineMatch nella pagina; se l’overlay non è disponibile,
  apre il player in una scheda dedicata.
- Il pulsante è discreto e sempre utilizzabile, con area di tocco più compatta
  sugli schermi stretti.

## Permesso opzionale

L’accesso a themoviedb.org **non è obbligatorio** e non viene richiesto
durante l’aggiornamento.

1. Collega TMDB, se non lo hai già fatto.
2. Apri **Opzioni → Pulsante flussi su TMDB**.
3. Attiva l’opzione.
4. Concedi l’accesso a `https://www.themoviedb.org/*`.
5. I pulsanti compaiono anche nelle schede TMDB già aperte.

Disattivando l’opzione, CineMatch rimuove i pulsanti dalle schede aperte,
annulla la registrazione del content script e revoca il permesso opzionale.
Il content script legge solo ID, titolo e immagine visibili del poster; non
accede ad account, cookie o conversazioni e non chiama TMDB per trovare i
poster. L’overlay è lo stesso player dell’estensione.

L’accesso a TMDB serve anche a mostrare le locandine nel player; la connessione
TMDB già configurata per le immagini viene riutilizzata.

## Pulsante nelle immagini delle chat

Le locandine e le immagini già inserite sotto le risposte AI mostrano lo stesso
pulsante **Flussi**. Il click sulla locandina continua a funzionare; il pulsante
rende esplicita l’azione e restituisce il focus dopo la chiusura del player.

Le referenze televisive vengono create solo quando la risposta indica
esplicitamente una serie (`Serie TV`, `Stagione`, `Episodio`, `Miniserie`).
In caso di ambiguità non viene aggiunta alcuna immagine, mantenendo la regola
esistente: meglio niente immagine che un abbinamento incerto. Le foto delle
persone restano senza pulsante.

## Serie TV nel player

- TMDB restituisce titolo, stagioni, episodi e identificativo IMDb della serie.
- Il player mostra **Stagione** ed **Episodio**; le sorgenti vengono cercate
  per l’episodio selezionato.
- I manifest degli addon possono dichiarare il tipo `series`, oltre a `movie`.
- Le richieste usano l’identificativo IMDb o TMDB con stagione ed episodio,
  ad esempio `tt0944947:1:2` o `tmdb:1399:1:2`.
- Il resume point è separato per ogni episodio.
- Se un addon non supporta le serie, il player lo spiega senza mostrare errori
  generici.
- I flussi restano soggetti ai limiti dell’addon e del servizio configurato.

## Dati e compatibilità

- Le selezioni, la cronologia e i flussi non vengono salvati su disco.
- La bozza resta nella sessione come prima.
- TMDB riceve solo le richieste già previste per schede e immagini.
- Gli addon ricevono l’identificativo del film o dell’episodio, mai la chat o
  la chiave TMDB.
- I manifest già collegati in versioni precedenti conservano i tipi salvati da
  quella versione: se un addon non espone le serie, ricollegalo per leggere i
  nuovi tipi `series` dal manifest aggiornato.

## Verifica

- **80 test Node**, inclusi parsing TV, dettagli serie, stagioni/episodi e URL
  `stream/series`.
- **29 controlli browser** del pannello e dell’inserimento.
- **10 controlli launcher in-chat**.
- **5 controlli TV/TMDB**: permesso opzionale nel manifest, toggle e injection
  nella scheda aperta, pulsante film/serie, locandina principale della scheda,
  apertura del picker TV, richieste per episodio e rimozione live.
- **23 controlli immagini TMDB**, inclusi pulsante Flussi e referenza serie.
- **36 controlli player/overlay**, incluso il pulsante esplicito nelle chat.
- **8 controlli codec**, **5 controlli updater**, **1 benchmark throughput**.

Il dialogo nativo del permesso opzionale di Chrome non è automatizzabile in
headless; la suite usa una copia dell’estensione con il permesso TMDB
preconcesso, così può collaudare registrazione, injection, toggle e rimozione.
Il prompt nativo resta il primo gesto manuale dell’utente in produzione.

## Installazione e aggiornamento

Da **3.6.0** o successiva usa **Opzioni → Aggiornamenti** per la 3.8.0. Da una
versione precedente alla 3.6.0 sostituisci eccezionalmente i file nella cartella
già caricata e premi **Ricarica**. Il nuovo accesso a TMDB resta opzionale:
l’aggiornamento non lo richiede.

Il primo avvio della 3.8.0 non mostra pulsanti su TMDB finché non attivi
l’opzione e concedi il permesso.
