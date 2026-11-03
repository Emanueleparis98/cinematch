# CineMatch 3.7.0 — Composer libero e correnti

La 3.7.0 rende la **richiesta libera** un flusso di primo livello. Il pannello
si apre su **Componi** senza una corrente preselezionata: puoi scrivere subito
cosa cerchi e ottenere un prompt completo. Il catalogo resta disponibile nella
modalità **Con una corrente**, ma non è più un passaggio obbligatorio.

## Cosa cambia

- Il tab **La tua richiesta** diventa **Componi**.
- Il composer si apre in **Richiesta libera**, con esempi pronti e nessuna
  corrente obbligatoria.
- Il selettore **Con una corrente / Richiesta libera** è sempre visibile in cima
  al composer. Supporta clic e frecce da tastiera.
- Le correnti scelte restano salvate quando passi alla modalità libera. In quella
  modalità vengono ignorate e il composer lo spiega; tornando a **Con una
  corrente** tornano subito disponibili.
- La selezione è limitata a **quattro correnti**. La casella della quinta voce e
  il pulsante **Aggiungi** vengono disattivati, con un messaggio esplicito.
- Il tab **Esplora** include il collegamento **Richiesta libera** per aprire il
  composer senza selezioni.
- Il composer mostra un'anteprima del prompt, lo stato di validità e il motivo
  per cui **Genera** è disattivato.
- **Dettagli** raccoglie approfondimento, relazione con la corrente, preferenze,
  esclusioni e titoli già visti. La quantità resta visibile sopra i dettagli.
- La modalità libera conserva le selezioni ma il prompt non include il contesto
  delle correnti; la modalità con corrente continua a costruirlo come prima.
- Il prompt libero adatta il formato della risposta: se la richiesta riguarda un
  tema, un autore o un periodo non forza un elenco di film.

## Coerenza e sessione

La bozza e le selezioni continuano a vivere nella sessione del browser, senza
`storage.local` e senza cronologia. I vecchi draft restano leggibili. Se un draft
precedente contiene più di quattro correnti, vengono mantenute le prime quattro
e il prompt viene marcato come da rigenerare. La modalità **Richiesta libera** è
il default per le nuove sessioni; un draft già in modalità con corrente mantiene
il proprio obiettivo.

Il tasto **Ctrl+Invio** genera il prompt, **Ctrl+F** porta alla ricerca delle
correnti. I menu personalizzati restano invariati. Il passaggio di modalità non
cancella il testo o le selezioni e l'anteprima viene marcata come non aggiornata
quando i criteri cambiano.

## Prompt

`generatePrompt` accetta ora una richiesta libera anche con `movements: []`.
Per la modalità libera:

- non richiede correnti e ignora eventuali selezioni residue;
- non inserisce `CONTESTO DELLE VOCI SELEZIONATE`;
- mantiene quantità, preferenze, esclusioni e titoli già visti;
- usa istruzioni di risposta adattabili a film, temi, autori e periodi.

La modalità con corrente richiede ancora almeno una corrente e, per il confronto,
almeno due. Il limite di quattro è validato anche nel core, non soltanto
nell'interfaccia.

## Verifica

- **77 test Node**.
- **29 controlli browser** del pannello e dell'inserimento, inclusi composer
  libero, cambio modalità, limite di quattro correnti, prompt senza contesto,
  blocco del quinto elemento e layout a 320/360/420/900 px.
- **10 controlli launcher in-chat**, inclusa l'apertura sul composer libero e la
  generazione con corrente nel layout a due colonne.
- **21 controlli immagini TMDB**.
- **35 controlli player/overlay**.
- **8 controlli decoder software**.
- **5 controlli updater**: ZIP reader più versione, download, SHA-256, estrazione
  e conservazione di `chrome.storage.local`.
- **1 benchmark throughput**.

## Installazione e aggiornamento

Per una nuova installazione usa `CineMatch-Chrome-3.7.0.zip`. Da una versione
**3.6.0 o successiva** apri **Opzioni → Aggiornamenti**: CineMatch scarica il
pacchetto, verifica lo SHA-256 e aggiorna la stessa cartella senza cambiare ID,
TMDB o addon. Da versioni precedenti alla 3.6.0 sostituisci eccezionalmente i
file nella cartella già caricata e premi **Ricarica**.

Il prototipo di questa fase è conservato in
`docs/prototype/composer-ui.html` e non fa parte del pacchetto dell'estensione.

## Limiti

- In modalità libera le correnti selezionate sono ignorate per scelta: servono
  solo nella modalità **Con una corrente**.
- Il limite di quattro correnti è editoriale, non un limite tecnico del modello.
- La generazione resta locale; l'invio alla chat resta manuale.
- La disponibilità dei film e la verifica delle fonti dipendono dalla chat scelta.
