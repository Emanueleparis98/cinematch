# CineMatch browser 1.0.0 — verifica del 2 novembre 2026

L'estensione è il nuovo canale d'uso richiesto. La precedente app Windows non è
stata modificata, ricompilata o disinstallata in questo intervento.

## Risultato

- Pacchetto Manifest V3 autonomo, con pannello laterale e vista ampia facoltativa.
- Le stesse 74 voci della raccolta già revisionata, con categorie, contesto e fonti.
- Generatore JavaScript locale: quantità automatica o esplicita, richieste libere,
  confronti, preferenze, esclusioni e titoli già visti. Nessuna chiamata AI.
- Adattatori ChatGPT, DeepSeek, Claude e Gemini. Script eseguiti solo su azione
  esplicita, limitati al compositore del messaggio. Nessun invio automatico.
- Gestione di bozza presente, aggiunta, sostituzione, annullamento e bozza cambiata
  dopo la prima conferma. Inserimento idempotente: non duplica lo stesso prompt.
- Stato in `chrome.storage.session`; nessun archivio locale progressivo, cookie,
  telemetria, lettura delle conversazioni o API key.

## Verifiche eseguite

**13 test Node superati:** integrità delle 74 voci, quantità automatica/esatta,
validazione di numeri e modalità, confronto fra voci distinte, richiesta libera,
preservazione dei criteri personali, esclusione dei già visti, contesto selettivo,
categorie non storiche dichiarate, ricerca, host esatti, stato superato e permessi.

**22 controlli browser superati** con Playwright 1.62.1 e Chromium dedicato:

- Caricamento dell'estensione reale, service worker, impostazioni sidePanel,
  font locali, ricerca, schede e fonti.
- Generazione da interfaccia con 12 film e invalidazione dopo un nuovo criterio.
- Layout a 320×568, 360×740, 420×850 e 900×850 senza overflow orizzontale.
  Intestazione e controlli di conflitto raggiungibili anche alla dimensione minima.
- Esecuzione reale tramite `chrome.scripting` per tutti e quattro gli adattatori:
  textarea, contenuto modificabile, strutture Tiptap/ProseMirror e Quill simulate.
- Nessun evento di invio o Invio; conferma, append, replace e idempotenza.
- Protezione per sito sbagliato, editor invisibile, disabilitato o ambiguo,
  finestra modale, `beforeinput` annullato e testo ripristinato dal sito.
- Un token di una vecchia bozza non permette di sovrascriverne una nuova.
- Il setter nativo della textarea e l'evento input aggiornano un editor controllato.
- Il background rifiuta pagine attive estranee, anche con una vecchia chat ricordata.
- Riapertura del pannello con bozza ripristinata, `storage.local` vuoto.
- Pulsanti reali del pannello: annullamento senza modifiche e aggiunta a una bozza.
- Assenza di errori JavaScript e CSP durante l'esecuzione.

Impeccable detector eseguito: segnala solo l'uso di Fraunces come font diffuso.
È mantenuto intenzionalmente perché appartiene all'identità LUMEN esistente.
La revisione visuale ha corretto filtri troppo ingombranti e lo scorrimento della
pagina esterna al pannello durante la messa a fuoco del prompt.

## Limiti della verifica

Nessun account reale usato, nessuna conversazione letta e nessuna richiesta AI
inviata. Le pagine dei quattro provider sono fixture controllate, non una prova
dei loro editor attuali autenticati. L'apertura tramite l'icona è configurata e
verificata via API; non è stata installata l'estensione nel profilo personale.
I siti possono cambiare DOM o rifiutare eventi: il pannello segnala il problema
e mantiene disponibile Copia. Chrome/Chromium verificato; Edge non eseguito.
Firefox e Safari non supportati da questo pacchetto.

La qualità della risposta e gli eventuali limiti restano del servizio scelto.
Non sono promessi risultati infallibili, utilizzo AI illimitato o censimento
universale delle correnti. Nessuna pubblicazione sul Chrome Web Store effettuata.

## Riproduzione e distribuzione

`python scripts/build-extension.py` produce la cartella non pacchettizzata e lo ZIP
senza compilare Rust. `pnpm test` e `pnpm test:browser` ripetono le verifiche.
Il workflow `.github/workflows/extension.yml` installa Chromium nel runner e
conserva il pacchetto e gli screenshot come artefatti della CI.

Per l'utente basta caricare `dist/CineMatch-Chrome` o estrarre lo ZIP e selezionare
la cartella con `manifest.json` dalla pagina delle estensioni. Istruzioni e permessi:
`extensions/cinematch/README.md`.
