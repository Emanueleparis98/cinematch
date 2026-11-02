# CineMatch browser 2.1.0 — verifica del 2 novembre 2026

## Modifiche richieste

- Immagini subito dopo la riga del titolo, prima della descrizione.
- Pannello, connessione TMDB, campi e menu esclusivamente scuri.
- Spunte disegnate e liste personalizzate al posto dei menu del sistema.

L'identità, la tipografia Manrope e la disposizione 2.0 sono mantenute; cambia
la palette in grafite/navy con azioni cobalto e link azzurri. Il tema della pagina
del provider non viene modificato: le immagini continuano a ereditarne i caratteri.

## Ancoraggio delle immagini

Il vecchio algoritmo cercava soltanto blocchi completi. Quando il provider metteva
titolo e descrizione nello stesso paragrafo, la foto arrivava dopo entrambi.
La selezione dell'ancoraggio ora privilegia, nell'ordine:

1. Intestazioni semantiche h1–h6 o role=heading.
2. Il ritorno a capo che conclude una riga contenente titolo e anno.
3. Titoli completi in strong/bold.
4. Il blocco di testo, come ripiego.

Un titolo ripetuto nella descrizione non sposta l'ancoraggio. Nel paragrafo viene
usato un host span con Shadow DOM e layout block: non viene aggiunto un div
invalido dentro p. Anno e regista fuori dal grassetto rimangono prima delle
immagini. Il testo originale viene conservato.

## Controlli moderni

controls.css disegna checkbox arrotondate con tick, hover, focus e stato disabilitato.
controls.js mantiene il select originale come fonte del dato, ma lo esclude dalla
vista e dall'albero accessibile; un combobox espone il valore e una listbox
personalizzata in popover contiene le opzioni. Nessun menu nativo del sistema
si apre nell'uso normale. Le liste restano nel viewport, sopra il dock se necessario.

Supportati mouse, frecce, Home/End, Invio/Spazio, ricerca digitando, Esc, Tab e
chiusura esterna/fuori fuoco. Gli eventi input/change conservano tutti i filtri,
la generazione e il salvataggio già esistenti. Il reset e il riconoscimento della
chat attiva aggiornano anche la vista del controllo. Il menu usa il top layer,
evitando il taglio da parte dell'area scorrevole del pannello.

## Verifiche eseguite

- **33 test Node superati**: dati, generatore, parser, credenziali, matching,
  immagini, errori, cache e concorrenza.
- **26 controlli browser del pannello superati**: i 22 precedenti, più tema scuro
  e checkbox custom; scelta/reset delle liste; tastiera/cancellazione/typeahead;
  menu sopra il dock e interamente visibile a 320px. Verificati anche font locale,
  copie/inserimenti, bozze protette, responsive e assenza di invio automatico.
- **21 controlli browser delle immagini con fixture superati**: i 17 precedenti,
  più quattro forme di titolo/paragrafo. Immagini prima della descrizione nel
  paragrafo bold+br, anno/regista fuori dal bold, titolo bold senza br e titolo
  ripetuto nella descrizione. Ordine DOM e geometria verticale verificati.
- **13 controlli con TMDB reale superati**: i nove precedenti più gli stessi
  quattro casi, usando The Texas Chain Saw Massacre (1974) con locandina e cast
  reali. Credenziale passata via stdin al solo processo di prova, non ai sorgenti.

Totale: **93 controlli superati**. Nessuna richiesta AI o consumo di token API AI.
Le chat sono pagine DOM controllate; gli account AI personali non sono stati usati.
TMDB e le fotografie del collaudo live sono reali.

## Evidenze e limiti

Screenshot a 320, 420, 900 e 1140px. Le acquisizioni del menu usano reduced motion
per evitare fotogrammi intermedi dell'apertura; la UI normale mantiene transizioni
brevi rispettando la preferenza di sistema. Nessun permesso o host aggiunto.
La credenziale salvata e le bozze non vengono migrate o cancellate.

DESIGN.md e sidecar schemaVersion2 aggiornati dal documenter: 29 colori verificati,
16 varianti, 10 esempi di componenti e 35 riferimenti di token risolti.
Il detector, eseguito prima del refresh, ha segnalato solo discrepanze intenzionali
rispetto alla documentazione chiara precedente, ora sostituita dalla palette scura.
Revisione indipendente Impeccable: **ship** nel perimetro di questa rifinitura.
Otto acquisizioni valide e codice dei controlli/ancoraggi esaminato; nessuna
correzione materiale richiesta. Rimane escluso il collaudo con account AI autenticati.

Restano i limiti di riconoscimento dei formati non previsti e le possibili variazioni
del DOM dei provider. Gli abbinamenti incerti rimangono silenziosi, senza moduli
o impostazioni nella risposta. Firefox/Safari esclusi; Edge non eseguito direttamente.

Distribuzione: dist/CineMatch-Chrome-2.1.0.zip e dist/CineMatch-Chrome.
Per applicare, ricaricare l'estensione e la pagina della chat; nessuna nuova chiave
da inserire se la connessione TMDB era già salvata.
