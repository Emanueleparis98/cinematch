# Ricerca AI e filtri: comportamento aggiornato

## Cosa cambia

1. I filtri espliciti non vengono allargati automaticamente. Con zero risultati restano attivi; l'utente sceglie una rinuncia da Sblocca o modifica i criteri della chat. Anche un genere esplicito resta obbligatorio quando e attivo un movimento.
2. La memoria delle preferenze contiene richieste dell'utente. Le proposte dell'assistente non diventano gusti approvati. Un'espressione come «ben diretto» orienta la curatela senza inventare una soglia di voto.
3. Il comando fisso Criteri porta alle chip dell'ultima risposta e al suo editor espandibile. Nell'editor puoi cambiare piu valori e premere Applica criteri, oppure annullare la bozza. L'elenco dei campi scorre in un'area limitata; Applica resta disabilitato finche non cambia un valore. La modifica crea una nuova risposta e la porta in vista; i filtri del catalogo restano indipendenti.
4. Le schede si caricano prima della scelta finale: durata, codici dei paesi di produzione e keyword vengono verificati anche per titolo, simili e filmografie. I candidati senza dati sufficienti per una raccomandazione sono esclusi. I consigli pratici usano durata e abbonamenti ricevuti da TMDB, senza suggerimenti inventati su restauri o doppiaggi.
5. Altri esclude tutti i titoli gia proposti e passa alle pagine successive. I candidati gia verificati e incompatibili vengono saltati usando la cache. Limite di lavoro: 20 pagine per fonte.

## Verifiche ripetibili

`cargo test --locked --offline --all-targets -q`: 134 test della libreria e 23 di integrazione superati.

I nuovi test coprono paginazione oltre i primi venti film, pagine senza corrispondenze, durata sconosciuta o eccessiva, paese distinto dalla lingua, keyword mancanti, decenni separati, concetti tutti obbligatori, memoria senza preferenze inventate e rimozione di una chip tramite clic reale nel motore UI.

`cargo clippy --locked --offline --all-targets -- -D warnings`: superato.

`cargo check --locked --offline --features wgpu --all-targets`: superato.

Impeccable detect sui componenti dei criteri: nessun rilievo meccanico.

## Ricerche reali con OpenRouter e TMDB

- Italia, anni 80, voto minimo 9,9 e almeno un milione di voti: zero risultati, soglie conservate.
- Thriller italiano anni 80 senza horror, seguito da «sotto i 100 minuti»: criteri conservati, cinque proposte con durata verificata entro il limite.
- Altri dopo quella selezione: nuovi titoli senza ripetizioni, ancora entro i 100 minuti.
- Film italiano anni 80 ben diretto: preferenza di curatela, nessun voto minimo aggiunto.
- Nuova richiesta su viaggio nello spazio entro 60 minuti: vecchi filtri italiani e anni 80 azzerati; candidati con riscontro di spazio e viaggio, cinque proposte entro il limite.

Per ripetere le prove: `cargo run --example chat_probe -- "richiesta" "seguito" "altri"`. Il probe usa la stessa verifica delle schede dell'app e stampa anche il tempo del turno; richiede gli account configurati. I tempi osservati dipendono dai servizi e dalla rete.

## Prova nell'app installata

Installer eseguito con codice di uscita 0 e configurazione conservata. Il confronto degli hash conferma che l'eseguibile installato coincide con la build finale. Controllati home, ricerca, risultati a colonna singola e criteri nella finestra nativa 800x600. Verificati il comando Criteri, l'apertura dell'editor, il cambio da 100 a 90 minuti come bozza e la creazione della nuova risposta dopo Applica criteri, conservando periodo, paese, genere ed esclusione dell'horror. I messaggi di errore sui criteri non verificabili indicano direttamente cosa correggere.

## Limiti

Le motivazioni dell'AI restano interpretazioni: il controllo rigido riguarda i metadati e i criteri rappresentabili. La ricerca tematica dipende da trame e keyword presenti in TMDB. I dati mancanti possono ridurre il numero di proposte; non vengono sostituiti con corrispondenze parziali. WGPU e verificato in compilazione, mentre la prova grafica locale usa OpenGL. Non viene pubblicata una nuova release GitHub.
