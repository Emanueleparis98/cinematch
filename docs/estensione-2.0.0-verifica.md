# CineMatch 2.0.0 — verifica del 2 novembre 2026

## Richiesta e soluzione

L'utente ha rifiutato l'output con impostazioni e moduli e l'identità precedente.
La sua schermata mostrava la ricerca errata `🌊 2. Flow - Un mondo da salvare`.

Il parser ora elimina decorazioni anche prima della numerazione, preservando
i titoli numerici legittimi come 2001. Genera varianti dei sottotitoli e verifica
i titoli alternativi TMDB nei casi tradotti, senza scegliere risultati simili
arbitrariamente. Flow (2024) è stato verificato realmente: TMDB 823219, locandina
e regista Gints Zilbalodis riconosciuti con il titolo italiano.

L'arricchimento inserisce esclusivamente figure, fotografie, nomi, ruoli e link
TMDB. Nessun form, bottone, select, input, accordion, titolo duplicato, impostazione
o riquadro viene aggiunto alle risposte. Le immagini seguono font e colore della
chat. Se un film è ambiguo/non trovato, la sua anteprima rimane nascosta senza
interrompere il testo. Foto mancanti omesse. Nessun toggle manuale dell'automatismo.

Pannello ridisegnato: fondo chiaro, grafite e cobalto, Manrope locale, wordmark
sans e simbolo a cornici, righe cronologiche, selezione esplicita e dock compatto.
La richiesta personale è il primo campo del compositore. Connessione TMDB
separata dalle risposte, senza nuove chiavi da inserire per chi era già collegato.

## Controlli

- **33 test Node**: generatore, dati, quantità, parser, emoji prima della
  numerazione, nomi internazionali/numerici, traduzioni e titoli alternativi
  verificati, omonimi, credenziali, errori, cache, immagini e concorrenza.
- **22 test browser del pannello**: nuova UI, font locale, ricerca, scheda,
  generazione, stato superato, bozze, inserimento nei quattro editor senza invio,
  storage e comportamento da 320×568 a 900×850.
- **17 test browser dell'output**: funzionamento automatico, isolamento chiave,
  quattro provider e CSP restrittiva, assenza completa di controlli nell'output,
  tema chiaro/scuro, Flow con emoji, messaggi utente esclusi, casi incerti silenziosi,
  streaming senza doppie query, sostituzioni di nodi/testo, vecchi flag ignorati,
  retry transitorio limitato, risposte senza film e rimozione della chiave.
- **9 test browser con TMDB reale**: locandine e fotografie reali di Ladri di
  biciclette, Flow e La terrazza nei quattro adattatori, tema chiaro, assenza di
  controlli, accesso allo storage negato e rimozione. Credenziale passata via stdin
  a un profilo temporaneo; nessuna chiave nel repository o pacchetto.

Totale: **81 controlli superati**. Nessuna API AI o richiesta AI utilizzata.
I provider AI sono fixture DOM controllate, non account autenticati reali.
TMDB, i dati e le immagini del collaudo live sono reali.

## Prestazioni e privacy

Scansioni accorpate a 250 ms sulle sole risposte modificate; riferimenti stabili
650 ms, caricamento entro 250 px dal viewport. Risultati obsoleti non vengono
applicati. Gli ancoraggi di una intestazione già osservata trasferiscono il loro
stato visibile alle nuove anteprime: coperto il cambio titolo tramite characterData.
Rerender del sito riattacca gli elementi mantenendo lo stato. Cache immagini
ephemeral limitata a 64 elementi/sei milioni di caratteri, oltre alla cache
metadata già limitata a 100 richieste/cinque minuti. Nessuna acquisizione su disco.

Host e permessi invariati. Solo query di titoli/anni/nomi arrivano a TMDB, mai la
conversazione completa. Storage credenziale limitato ai contesti fidati. I vecchi
flag tmdbAutomatic vengono ignorati. La credenziale esistente viene conservata.

## Revisione e distribuzione

Screenshot a 320, 420, 900 e 1087 px; temi chiaro/scuro e immagini reali.
Le acquisizioni attendono decode e frame stabili; quelle chiare partono da una
pagina chiara indipendente. Un apparente taglio dei ritratti nel visore è stato
verificato indipendentemente: tutti gli 11 interni delle fotografie risultano
identici fra chiaro e scuro (5332 pixel su 5332 per rettangolo), confermando un
artefatto di presentazione e non delle immagini salvate.

La revisione indipendente ha richiesto due correzioni: aggiornamento del sistema
DESIGN.md/sidecar e maggior spazio per i nomi. Entrambe sono risolte; il verdetto
è **ship**, limitato ai due elementi del fix pass della superficie già esaminata.
Colonne dei nomi104/94px indipendenti dai ritratti76/58px, senza spezzare cognomi.
Il test browser misura con Range ogni parola nei quattro adattatori e nei tre
formati; Maggiorani, Staiola, Saltamrenda e Mastroianni rimangono leggibili.
Il documenter ha validato 20 riferimenti di token e sette anteprime dei componenti.

Il detector ha segnalato soltanto un'etichetta TMDB troppo piccola, portata a 11px;
uniformati anche i metadati e rafforzato il focus. Icona preesistente conservata
senza cambiare pixel e annotata con provenienza; scan raster senza elementi mancanti.
Font e licenza provengono dal repository ufficiale Google Fonts. Le immagini dei
film restano esclusivamente runtime e non vengono distribuite nello ZIP.

Pacchetto: dist/CineMatch-Chrome-2.0.0.zip. Cartella: dist/CineMatch-Chrome.
Nessuna installazione nel profilo personale, pubblicazione sullo store o modifica
alla vecchia app Windows. Restano possibili variazioni del DOM dei provider e
riferimenti non riconoscibili; nessuna garanzia di copertura universale.
