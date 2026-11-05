# CineMatch 3.11.0 — Correnti compatte in DeepSeek

La 3.11.0 ridisegna il pulsante Correnti di DeepSeek: non apre più una grande
schermata modale, ma un **popover compatto ancorato al composer**, pensato per
sembrare parte dell’interfaccia DeepSeek.

## Cosa cambia

- Popover largo circa 420 px (o a tutta larghezza su schermi stretti), alto al
  massimo 560 px.
- Nessun backdrop, nessuna schermata intera: la chat resta visibile.
- Elenco denso delle **74 correnti**, con ricerca immediata.
- Fino a quattro correnti scelte, mostrate come chip.
- Modalità **Correnti** e **Richiesta libera** in due piccole schede.
- Opzioni avanzate chiuse in un pannello `Opzioni` richiudibile.
- Anteprima del prompt compatta, con **Copia** e **Inserisci nella chat**.
- Se il composer contiene già una bozza, il popover chiede se aggiungere in fondo
  o sostituirla.
- `Esc`, click fuori o pulsante di chiusura; focus restituito al pulsante
  Correnti.

## Perché è più leggibile

La vecchia schermata usava titoli grandi, due livelli di schede, una hero e due
colonne. La nuova vista usa una sola colonna:

1. due schede in alto;
2. ricerca;
3. lista compatta;
4. opzioni solo se servono;
5. un solo pulsante principale.

Il tema chiaro/scuro viene ricevuto dalla pagina DeepSeek; il popover si adatta
senza cambiare il resto della chat.

## Inserimento

Il popover è un documento dell’estensione dentro un iframe. Quando premi
**Inserisci**, il background esegue l’inserimento nel composer DeepSeek tramite
lo stesso codice già usato dal pannello. Il messaggio non viene inviato
automaticamente e la bozza esistente non viene sovrascritta senza conferma.

## Verifica

- **10 controlli browser DeepSeek**: toggle, ricarica, pulsante nativo, popover,
  74 correnti, selezione, prompt, inserimento, modalità libera, rerender,
  disattivazione, altre chat e layout stretto.
- Regressione completa: 86 test Node, 29 browser, 5 TMDB/TV, 23 immagini,
  36 player, 8 codec, 7 CB01, 4 updater e throughput.

La verifica manuale va fatta su DeepSeek con account loggato, perché la chat
richiede l’accesso per mostrare il composer.
