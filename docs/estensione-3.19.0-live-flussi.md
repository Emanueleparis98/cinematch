# CineMatch 3.19.0 — Flussi Live TV / Sport

Il pulsante **Flussi** è disponibile sui servizi delle sezioni Live TV e Live
Sports di FMHY. Apre le sorgenti HTTP, HLS e DASH rilevate nella pagina nel
player CineMatch esistente. La funzione è opzionale e non richiede TMDB.

## Utilizzo

1. Popup dell’estensione → **Funzioni → Flussi su Live TV / Sport (FMHY)**.
2. Consenti l’accesso ai siti del catalogo.
3. Apri il canale o l’evento scelto, poi premi **Flussi** in basso a destra.
4. Il primo flusso disponibile viene avviato. **Sorgenti** permette di cambiarlo.
5. Se necessario, **Consenti e cerca** autorizza il dominio del player esterno.
   Il consenso ai server video viene richiesto dal player quando serve.

In caso di errore vengono tentate al massimo tre sorgenti per apertura.
**Torna in diretta** torna al termine della finestra disponibile nei player
nativi/HLS; i decoder integrati possono non esporre un punto live utilizzabile.
Non viene salvato un punto di ripresa per le dirette.

## Copertura effettiva

Il catalogo importato il 5 novembre 2026 contiene **124 host di servizi e alias**,
accorpando le ripetizioni tra le due sezioni. Il JSON contiene nomi, URL,
sezioni, percorso sulle piattaforme condivise e hash della fonte importata.
Link social, pagine di stato e indici di mirror non autorizzano script su quei
domini. Le nuove destinazioni dei mirror richiedono un aggiornamento del catalogo.

La funzione utilizza un rilevatore comune, **non 124 estrattori specifici già
certificati**. Cerca sorgenti nel DOM e negli script inline, URL letterali base64,
configurazioni packer standard, manifest osservati nelle risposte di rete
(anche senza estensione nel percorso) e iframe autorizzati. Per gli iframe
la lettura statica è limitata a due livelli, otto pagine, 2 MB per pagina e
un budget di tempo di circa 12 secondi più la richiesta in corso.

Un sito può caricare il flusso solo dopo aver selezionato un canale o avviato il
suo player. In quel caso occorre effettuare quella scelta e riaprire Flussi.
Le API private, i player cifrati o offuscati in modi diversi, le sorgenti che
richiedono header speciali e le pagine che impediscono il recupero statico
possono richiedere adattatori dedicati. Non vengono aggirati DRM, CAPTCHA,
login, abbonamenti o restrizioni territoriali. Un catalogo, una risposta HTTP
e un URL rilevato non costituiscono una certificazione della diretta.

## Pubblicità e permessi

Il player CineMatch non incorpora il player HTML del fornitore e non aggiunge
annunci. Un filtro di rete per domini pubblicitari noti è limitato alle schede
del catalogo supportate, inclusi gli iframe in quelle schede. Un piccolo filtro
dei popup blocca aperture vuote e domini pubblicitari noti e viene ripristinato
alla disattivazione. Non viene rimossa la pubblicità inserita nel video.

I permessi dei siti sono opzionali. I domini aggiuntivi dei player vengono
presentati prima dell’autorizzazione. URL firmati e token restano in
`chrome.storage.session`, accessibile soltanto ai contesti fidati dell’estensione;
le sessioni player scadono dopo 20 minuti e sono legate alla scheda e alla pagina
di origine. I messaggi provenienti dalle pagine non possono leggere i token.
La disattivazione elimina token, sorgenti, script e regole Live. I permessi
già concessi possono essere revocati dalle impostazioni del browser; non vengono
rimossi automaticamente perché un dominio può essere usato anche da un addon.

## Verifica

`npm test` include le prove del catalogo, dei confini tra host e percorsi,
degli URL non sicuri, dei segmenti da escludere, del MIME, della deduplicazione
e delle configurazioni letterali. `npm run test:live` controlla l’iniezione del
pulsante su tutti i 124 host usando **pagine simulate**, MP4 e HLS con media
locali, fallback, iframe autorizzati e non autorizzati, URL senza estensione,
accesso ai token, blocco pubblicitario, disattivazione e riattivazione.

Esito locale: **95 test Node**, **14 controlli Live/browser**, **29 controlli
chat/browser**, **5 TMDB/serie**, **7 CB01** e **38 player/browser** superati.

Le prove non usano account esterni e non attestano la disponibilità delle
dirette reali dei 124 host. Le regressioni browser coprono inoltre chat,
TMDB/serie, CB01 e il player esistente.

## Aggiornare l’elenco

Scaricare e revisionare `docs/video.md` da `fmhy/edit`, poi eseguire:

```sh
python scripts/import-live-sites.py path/to/video.md
```

Revisionare il JSON generato e aggiornare i match di `cinema.html` nel manifest
per i nuovi host prima di costruire il pacchetto. Nessun codice remoto viene
scaricato o eseguito dall’estensione per aggiornare gli estrattori.
