# CineMatch 3.6.0 — Aggiornamenti integrati

La 3.6.0 introduce il self-updater per l’estensione caricata non pacchettizzata.
Non serve più scaricare e selezionare un nuovo ZIP a ogni versione: CineMatch
scarica la release, verifica il pacchetto e aggiorna la **stessa cartella**, poi
si ricarica. TMDB, addon e impostazioni restano.

## Perché serve

Chrome non aggiorna automaticamente le estensioni caricate con **Carica estensione
non pacchettizzata**. L’ID di un’estensione unpacked dipende dalla cartella: se si
carica una cartella nuova per ogni ZIP, Chrome crea un ID nuovo e `chrome.storage`
riparte da zero. Il self-updater risolve entrambi i problemi:

- aggiorna la cartella esistente, quindi l’ID non cambia;
- scrive i file al posto dei vecchi, senza passare da un nuovo ZIP manuale.

## Flusso

1. Apri **Opzioni → Aggiornamenti** (icona con la freccia nella barra del pannello).
2. La prima volta premi **Scegli cartella** e seleziona la cartella da cui hai
   caricato l’estensione: deve contenere `manifest.json`.
3. Premi **Controlla aggiornamenti**; la prima volta Chrome chiede l’accesso a
   GitHub.
4. Se è disponibile una versione nuova appare **Aggiorna ora**.
5. CineMatch scarica `CineMatch-Chrome-<versione>.zip`, verifica lo **SHA-256**,
   estrae i file in memoria, li scrive nella cartella scelta e chiama
   `chrome.runtime.reload()`.

L’aggiornamento richiede un click: Chrome non consente a un’estensione unpacked
di sostituire da sola i propri file senza un gesto dell’utente. Dopo la prima
configurazione non servono più ZIP, estrazioni o nuove cartelle.

## Dati e cartella

- La cartella viene salvata in IndexedDB; l’ID dell’estensione resta lo stesso.
- `chrome.storage.local` (TMDB, addon) resta intatto; il test automatico verifica
  un valore sentinella prima e dopo l’update.
- Se viene scelta una cartella diversa da quella caricata, CineMatch se ne accorge
  al riavvio: mostra un avviso e chiede di selezionare la cartella giusta.
- Le bozze e la ripresa di sessione possono azzerarsi al riavvio del browser,
  come già accade.

## Sicurezza

- Download solo dal repository ufficiale, via HTTPS.
- `latest.json` pubblicato dalla workflow di release con versione, dimensione e
  SHA-256 del pacchetto.
- Hash verificato prima dell’estrazione; ZIP reader locale con controllo CRC,
  limiti di dimensione/numero file e rifiuto dei percorsi non sicuri (`..`,
  path assoluti, backslash).
- La cartella scelta viene validata leggendo `manifest.json` e il nome CineMatch.
- `manifest.json` viene scritto per ultimo; se qualcosa fallisce l’estensione non
  viene ricaricata e l’utente vede l’errore.
- Nessun file sconosciuto viene cancellato.

## Badge e controllo periodico

- La workflow di release pubblica `latest.json` insieme allo ZIP.
- Il service worker controlla `latest.json` all’avvio e ogni 6 ore, ma solo se
  l’accesso a GitHub è già stato concesso; se c’è una versione nuova mostra un
  badge sull’icona.
- Anche la pagina Opzioni controlla automaticamente all’apertura, se il permesso
  è disponibile.
- L’icona di aggiornamento nella barra del pannello apre la sezione dedicata.

## Collaudo

- 74 test Node.
- 26 controlli browser del pannello e dell’inserimento.
- 21 controlli immagini TMDB.
- 35 controlli player/overlay.
- 9 controlli launcher.
- 8 controlli decoder software.
- **5 controlli updater**:
  - 1 test ZIP reader: pacchetto reale, manifest, percorsi sicuri e rifiuto di
    `../escape.txt`;
  - 4 controlli browser: versione installata, canale di aggiornamento, pulsante
    Aggiorna, download + SHA-256 + estrazione + scrittura nella cartella scelta e
    persistenza di `chrome.storage.local`.

La scrittura nella cartella caricata da Chrome viene simulata nei test con un
directory handle OPFS; il reload reale è una singola chiamata
`chrome.runtime.reload()` dopo la scrittura.

## Limiti

- Il primo passaggio richiede ancora lo ZIP **3.6.0**.
- Non è un aggiornamento silenzioso come quello del Chrome Web Store: serve un
  click per autorizzare la scrittura.
- Selezionare la cartella sbagliata non aggiorna l’estensione caricata; l’avviso
  guida a scegliere quella giusta.
- Un aggiornamento che introduce un nuovo permesso può richiedere l’accettazione
  da parte di Chrome.
- Le chat reali autenticate restano un collaudo manuale; DeepSeek presenta una
  verifica umana nella rete dei test automatici.

## Installazione della prima 3.6.0

Scarica `CineMatch-Chrome-3.6.0.zip` e **estrailo**.

- **Se hai già CineMatch caricato (3.5.0 o precedente)**: apri `chrome://extensions`
  → dettagli dell’estensione → “caricata da”, e **sostituisci i file in quella
  cartella** con quelli della 3.6.0. Poi premi **Reload**. Non caricare una
  cartella nuova, altrimenti Chrome cambia ID e perdi TMDB/addon.
- **Prima installazione**: carica la cartella 3.6.0 con **Carica estensione non
  pacchettizzata**.

Chrome non carica lo ZIP direttamente. Dalla 3.6.0 in poi usa
**Opzioni → Aggiornamenti**.

```sh
python scripts/build-extension.py
pnpm test:updater
```
