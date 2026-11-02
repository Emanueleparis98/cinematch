# CineMatch browser 1.2.0 — verifica del 2 novembre 2026

## Comportamento richiesto

Le immagini compaiono automaticamente nell'output visualizzato della chat,
direttamente dopo l'intestazione di ogni film. Il pulsante di attivazione per
singola risposta della 1.1.0 è rimosso. Locandina, ritratto del regista e primi
tre interpreti sono già visibili. Solo cast completo, correzione e ricerca manuale
rimangono espandibili. Il testo originale dell'AI non viene sostituito o spostato.

La preferenza è attiva per impostazione predefinita, anche con una credenziale
già salvata nella 1.1.0. Le opzioni permettono di sospendere e riattivare le
immagini senza reinserire la chiave. Cambio di preferenza, salvataggio e rimozione
della credenziale notificano le chat aperte, senza riavvio della pagina.
L'aggiornamento del codice dell'estensione richiede invece ricaricamento delle
schede delle chat, come per gli altri aggiornamenti di content script.

## Streaming e caricamento

- Rilevamento di nuove risposte e modifiche sia alla struttura sia ai nodi di testo.
- Scansione accorpata ogni 250 ms; vengono rianalizzate solo le risposte modificate.
- Ogni riferimento deve rimanere stabile per circa 700 ms prima della ricerca:
  il corpo della risposta può continuare a crescere senza ritardare le immagini.
- Schede caricate automaticamente entro 250 px dall'area visibile; le altre
  vengono caricate scorrendo. Nessun pulsante e nessun limite di 5 o 12 film.
- Stato e identità per titolo/anno: il testo aggiunto non ripete ricerche concluse.
  Un titolo modificato invalida la vecchia richiesta e rimuove la scheda superata.
- Se il sito rimuove i nodi aggiunti dall'estensione durante un aggiornamento,
  vengono riattaccati conservando lo stato, senza una nuova ricerca.
- Rimozione dei riferimenti e navigazione liberano timer, osservatori e nodi.
- Richieste TMDB concorrenti limitate a tre, cache solo in memoria già esistente,
  timeout ed errori recuperabili. Nessuna API AI, invio di messaggi o token AI.

Il caso della schermata dell'utente è coperto esplicitamente:
`8. La terrazza (1980) — Regia: Ettore Scola`. Il parser elimina sia il prefisso
dell'intestazione sia la numerazione; non cerca erroneamente "8. La terrazza".
I nomi di persone esatti e univoci possono essere abbinati automaticamente;
le omonimie restano da scegliere. Un regista dichiarato deve ancora coincidere.

## Privacy

La modalità automatica analizza localmente le risposte dell'AI, mentre arrivano.
I messaggi dell'utente restano esclusi. TMDB riceve soltanto titoli, anni o nomi,
richieste di schede e immagini; non la risposta integrale o la conversazione.
Le descrizioni privacy, impostazioni e README sono state aggiornate da "al clic"
a questo comportamento. Nessun nuovo host o permesso rispetto alla 1.1.0.

La chiave salvata resta nei contesti fidati dell'estensione. Il content script
non può leggerla; i test verificano il diniego di storage.local. Nello storage
locale è aggiunta soltanto la preferenza booleana tmdbAutomatic. Non vengono
salvate risposte, fotografie o archivi progressivi di film.

## Verifiche eseguite

**30 test Node superati**, inclusi i due nuovi casi: intestazione numerata della
schermata e persone esatte/univoche contro omonimie. Restano coperti generatore,
dati, input, autenticazione, corrispondenze, errori, immagini e concorrenza.

**22 controlli browser del pannello e inserimento prompt superati** con la 1.2.0.

**15 controlli browser automatici con fixture superati:**

1. Nessuna richiesta TMDB o scheda senza credenziale configurata.
2. Salvataggio della chiave avvia le immagini nelle chat già aperte, senza click.
3. Storage della chiave inaccessibile al content script.
4–7. Quattro adattatori con immagini automatiche, CSP restrittiva e schede
     immediatamente dopo gli h3 originali, anche numerati.
8. Numerazione e messaggi utente non arrivano nelle query TMDB.
9. Film caricato prima della fine dello streaming; una sola ricerca per titolo.
10. Modifiche ai soli nodi di testo sostituiscono la scheda, senza duplicati.
11. Rimozione dei nodi da parte del sito ripristina le schede senza nuove ricerche.
12. Sospensione e riattivazione senza reinserire la credenziale.
13. Correzione manuale, titoli ambigui e ricerca di persone ancora funzionanti.
14. Risposta senza riferimenti a film: nessuna ricerca o immagine automatica.
15. Rimozione della credenziale rimuove le schede e blocca ricerche successive.

**8 controlli con TMDB reale superati:** la credenziale è stata passata al solo
processo isolato via stdin. Ladri di biciclette (1948) e La terrazza (1980) sono
stati riconosciuti e arricchiti automaticamente nei quattro adattatori: locandine,
foto di Vittorio De Sica, Ettore Scola e interpreti reali. Nessun pulsante di avvio,
apertura del cast o richiesta AI necessaria per mostrare i ritratti iniziali.
Credential storage, configurazione e rimozione verificati anche nella prova reale.

Totale: **75 controlli superati**. Screenshot verificati a 320, 420 e 900 px,
compresi i casi con immagini reali e foto mancanti. Corretti contrasto del piede
su pagine chiare e sovrapposizioni del testo nei piccoli segnaposto dei ritratti.

## Limiti e consegna

TMDB e le immagini sono reali; le quattro chat sono pagine di prova isolate con
contenitori rappresentativi. Nessun accesso ad account AI personali o invio di
messaggi. Resta possibile un cambiamento del DOM dei provider. Firefox/Safari
non supportati; Edge non eseguito direttamente.

L'arricchimento riguarda la visualizzazione locale nel browser con estensione.
Titoli/anni non riconoscibili restano ricercabili nel modulo sotto la risposta;
abbinamenti incerti richiedono la scelta dell'utente, per evitare fotografie errate.

Cartella pronta: dist/CineMatch-Chrome. ZIP: dist/CineMatch-Chrome-1.2.0.zip.
Nessuna credenziale personale inclusa nel pacchetto o nei sorgenti. Per applicare
la versione, ricaricare l'estensione e la pagina della chat: la chiave già salvata
rimane valida e l'automatismo parte senza una nuova configurazione.
