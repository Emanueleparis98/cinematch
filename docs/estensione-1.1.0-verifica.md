# CineMatch browser 1.1.0 — verifica del 2 novembre 2026

## Risultato

Le risposte di ChatGPT, DeepSeek, Claude e Gemini ricevono il controllo **Mostra
locandine e cast**. Il testo della sola risposta scelta viene letto al clic e
analizzato localmente. Le richieste TMDB contengono titoli, anni o nomi; nessun
testo integrale della conversazione, messaggio utente o prompt viene inviato.

Le schede sotto la risposta mostrano locandina, titolo, anno e regista. Regia e
cast sono espandibili con foto e link TMDB. Disponibili ricerca manuale di film
o persone, scelta fra candidati, correzione, aggiornamento e compressione.
I controlli e le schede usano Shadow DOM e un foglio CSS isolato.

Solo un titolo esatto, anno coincidente e risultato unico permettono la selezione
automatica. Un regista esplicito deve coincidere; altrimenti è richiesta la scelta.
I titoli alternativi vengono tentati prima di dichiarare l'abbinamento incerto.
I riferimenti senza anno o con formati non riconosciuti possono essere cercati
manualmente. Nessuna promessa di interpretazione universale delle risposte AI.

## Privacy e credenziali

Credenziale verificata da una pagina di opzioni fidata, conservata su scelta
in storage.local oppure solo in storage.session. Entrambi hanno accesso
TRUSTED_CONTEXTS: il content script non può leggerli. Nessuna chiave è inclusa
in sorgenti, ZIP, URL delle chat, screenshot o repository. Nessun segreto viene
restituito nei messaggi di stato. Token Bearer nell'header Authorization;
le chiavi v3 usano l'autenticazione prevista da TMDB, senza log degli URL.

Sono aggiunti due host, api.themoviedb.org e image.tmdb.org. Nessun permesso
generale ai siti, cookie o cronologia. Gli script automatici aggiungono soltanto
controlli ai contenitori delle risposte. I testi vengono letti al clic, mai
durante il rilevamento iniziale o l'arrivo di nuove risposte.

Immagini raster da path validati e host fisso, MIME e dimensione controllati;
trasferimento via background e data URL per funzionare anche quando la pagina
blocca immagini esterne. Niente HTML proveniente da TMDB inserito nel DOM.
Richieste concorrenti al massimo tre, timeout, errori recuperabili e riprova
manuale. Nessun ciclo automatico di retry o richiesta AI a pagamento.
Paginazione di 12 riferimenti/persone, senza limite totale applicativo di film.
Cache delle schede solo in memoria, cinque minuti, massimo 100 voci. Nessun
database progressivo o download persistente di locandine; richieste no-store.

## Prove eseguite

- **28 test Node**: generatore e 74 voci, permessi, parser di intestazioni/liste/
  tabelle, titoli alternativi, metadati del regista senza duplicati, riconoscimento
  di persone, credenziali, validazione input e immagini, corrispondenza esatta,
  casi ambigui e regista in conflitto, errori, cache, autenticazione e concorrenza.
- **22 controlli browser precedenti**: prompt, inserimento nei quattro editor,
  bozze, assenza di invii, responsive e storage della sessione. Superati con 1.1.0.
- **12 controlli browser TMDB con fixture**: nessuna rete prima del clic, verifica
  e rimozione della chiave, storage inaccessibile al content script, schede e
  immagini per tutti e quattro i siti, scelta manuale, persone, errori e nuove
  risposte. Un submit sintetico non avvia richieste. CSP della pagina di prova:
  niente script, stili, connessioni o immagini esterne; consentiti solo img data.
- **8 controlli browser con TMDB reale**: credenziale dell'utente passata al
  processo di test via stdin, senza file o variabili nel repository. Verifica,
  salvataggio nel solo profilo temporaneo, ricerca di Ladri di biciclette (1948),
  locandina e foto reali di Vittorio De Sica e del cast nei quattro adattatori,
  isolamento dello storage e rimozione della credenziale. Nessuna richiesta AI.

Screenshot verificati a 320, 420 e 900 px per le schede e 320 px per le impostazioni.
Corrette leggibilità dei pulsanti su pagine chiare e allineamento del campo anno.
Le immagini reali sono state caricate e controllate anche con CSP restrittiva.
Font e identità LUMEN conservati; logo TMDB ufficiale non modificato e dichiarazione
di attribuzione nei crediti delle opzioni e sotto le schede.

## Limiti e distribuzione

I siti AI sono pagine di prova con contenitori rappresentativi; non sono stati
usati account AI autenticati, letti dialoghi personali o inviati messaggi. TMDB
è stato realmente chiamato, compreso il caricamento delle immagini nel browser.
Gli editor e i contenitori dei siti possono cambiare: restano aggiornabili gli
adattatori. Chrome/Chromium verificato; Edge non eseguito. Firefox/Safari esclusi.

Pacchetto `dist/CineMatch-Chrome-1.1.0.zip`, cartella pronta `dist/CineMatch-Chrome`.
Aggiornare dalla pagina delle estensioni, configurare la propria credenziale una
volta e ricaricare le chat. Nessuna installazione nel profilo personale o
pubblicazione sullo store effettuata. Il browser/profilo usato per i test è isolato.
