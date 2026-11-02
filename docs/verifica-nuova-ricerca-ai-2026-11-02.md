# Verifica della nuova ricerca AI

Data: 2 novembre 2026. Provider reale configurato nell'app: DeepSeek, modello `deepseek-flash`, ragionamento `none`. Le credenziali restano nel gestore credenziali Windows; report completi e diagnostica locale non sono inclusi nel repository.

## Comportamento consegnato

La chat AI usa il coordinatore dedicato in `src/search`, anziché ridurre la conversazione ai filtri del catalogo. Il percorso a regole resta disponibile quando scelto esplicitamente.

- Intenzione completa: richiesta originale, quantità, requisiti narrativi, negazioni, contesto di visione, vincoli numerici e ordine richiesto.
- Memoria di sessione, separazione degli argomenti, correzioni dei criteri e alternative con esclusione degli identificativi già mostrati. Un'etichetta AI errata `new` non azzera gli esclusi quando l'obiettivo restituito è identico.
- Quantità variabile, comprese 1, 5 e 12; consegna progressiva oltre 24 risultati per passaggio, mantenendo il totale richiesto. Nessun limite globale di cinque film.
- Domande e confronti ricevono risposte testuali senza raccomandazioni automatiche. I fatti specifici sono controllati sulle fonti disponibili.
- Scoperta attraverso titoli, persone, generi, keyword, film simili e ricerca documentale Wikimedia. I documenti narrativi sono collegati mediante identificativi Wikidata/TMDB, evitando il primo omonimo trovato.
- Metadati controllati dal programma, con distinzione fra IMDb e TMDB, paese e lingua, comparatori stretti e gruppi di alternative OR.
- Prima verifica dei requisiti con citazioni letterali controllate; secondo controllo di pertinenza sull'intento completo e sulle fonti, senza mostrare al revisore il precedente giudizio favorevole. Entrambi i passaggi usano il provider configurato: non sono una valutazione umana indipendente.
- Solo corrispondenze ammesse; nessun riempimento con film popolari quando il modello fallisce. Le corrispondenze incerte non completano artificialmente la quantità.
- Ordinamento per qualità fra i risultati ammessi, salvo ordine esplicito diverso; varietà di autori, saghe ed epoche compatibilmente con la richiesta. Cache delle alternative già verificate ma non ancora mostrate.
- Fonti consultabili nelle schede, criteri correggibili e indicazioni esplicite di ricerca incompleta. Progressi per fasi effettive, senza percentuali simulate.

## Verifiche automatiche locali

Tutti i seguenti controlli sono riusciti sulla versione finale del codice:

| Controllo | Risultato |
| --- | --- |
| `cargo test --locked --offline --all-targets` | 189 test: 138 di libreria, 23 di integrazione esistenti, 28 del nuovo motore |
| `cargo clippy --locked --offline --all-targets -- -D warnings` | Nessun errore o avviso |
| `cargo check --locked --offline --features wgpu` | Riuscito |
| `git diff --check` | Nessun errore di whitespace |

I test del motore coprono quantità e consegna progressiva, seguiti e memoria, correzioni, cambi di argomento, recupero esplicito, numeri e fonti, gruppi OR, persone nei crediti, identità e citazioni, errori del provider, annullamento, riuso delle alternative e veto finale delle corrispondenze parziali. Sono presenti regressioni per il titolo ripetuto dopo una classificazione errata del seguito e per l'ordine per qualità.

## Comprensione con DeepSeek reale

Corpus versionato in `tests/fixtures/search-corpus.json`: 200 richieste e 50 conversazioni di quattro turni, 400 messaggi complessivi. Le richieste combinano 40 situazioni narrative con quantità diverse; non equivalgono a 200 temi indipendenti. I temi riservati alla valutazione finale sono distinti da quelli di sviluppo; i test verificano anche l'assenza di conversazioni duplicate.

| Suddivisione | Scenari | Messaggi | Scenari riusciti |
| --- | ---: | ---: | ---: |
| Sviluppo | 190 | 310 | 190 |
| Valutazione finale | 60 | 90 | 60 |
| Totale | 250 | 400 | 250 |

Questa misura verifica **azione conversazionale e quantità**, non l'accuratezza narrativa dei film, né ogni vincolo estratto. Non dimostra un'accuratezza universale del 99,9%.

Riproduzione con il provider già configurato, comporta chiamate reali a pagamento:

```powershell
cargo run --locked --example search_benchmark -- --workers=4
cargo run --locked --example chat_probe -- --critical
```

## Ricerca completa con servizi reali

La suite critica comprende 12 scenari e 20 messaggi (`tests/fixtures/search-critical.json`). Sono stati controllati quantità, titoli, mantenimento dei criteri e motivazioni; non è stato svolto uno studio indipendente con valutatori umani.

- Halloween piacevole, voti alti, senza Horror: sei richieste consecutive hanno restituito sei identificativi distinti. Titoli: Ghostbusters, Beetlejuice, La famiglia Addams, Halloweentown High, Hocus Pocus, Shaun - Operazione Z.U.C.C.A. Nessun duplicato nella sequenza finale.
- Amicizia cane-gatto: Bolt e Marnie's World, con evidenze narrative e motivazione del rapporto fra i personaggi.
- Fantascienza e viaggi nel tempo: ottenuti rispettivamente 1, 5 e 12 film nei tre casi con quantità esplicita.
- Spiegazione dei generi e confronto Alien/Arrival: risposte testuali, zero schede aggiunte.
- Correzione della durata e sua rimozione: mantenuti argomento, esclusione Horror e quantità desiderata. Nel primo turno di questa sequenza sono stati consegnati 4 film su 5 con avviso esplicito; i due seguiti hanno restituito 5 film. Il sistema può trovare meno corrispondenze di quelle realmente esistenti.
- Passaggio da Halloween a dramma storico coreano: nuovo contesto, senza ereditare i vecchi requisiti narrativi.
- Anni 1993–1997 e IMDb strettamente sopra 8: cinque risultati, con fonte e intervallo verificati.
- Film del 1800 su viaggi interstellari: nessuna raccomandazione inventata; richiesta di chiarimento sull'anno.
- Caso bibliotecaria/polpo/Marte: ripetizione riuscita, zero film e spiegazione esplicita della mancanza di corrispondenze verificate; nessuna trama inventata.

La prima esecuzione finale si è interrotta sull'ultimo scenario per una risposta HTTP 502 del provider, dopo 19 messaggi completati. Il caso è stato ripetuto separatamente; l'esecuzione interrotta non viene presentata come suite conclusa con codice di uscita zero.

Durante le prove precedenti erano emersi: deriva del tono verso fantasy generico, confusione fra compagnia di visione e trama romantica, perdita degli esclusi in un seguito, risposte JSON complete seguite da frammenti malformati e testo Wikipedia limitato all'introduzione. Le correzioni includono contesto separato, controllo finale cieco, protezione della memoria, parsing del primo oggetto completo senza inventare chiusure e recupero del testo documentale con limite locale.

## Interfaccia e limiti

L'anteprima nativa con dati dichiaratamente dimostrativi ha consentito di controllare dodici schede, due colonne, fonti e risposte testuali. Una sovrapposizione delle schede compatte è stata corretta e coperta da regressione. Esistono controlli automatici della geometria a dimensioni ridotte; la sessione finale di interazione nativa è stata interrotta dall'utente con Escape. Non si dichiara verificato ogni ramo grafico o ogni dimensione della finestra.

La pertinenza resta dipendente dal modello e dalle fonti. Le valutazioni di tono sono soggettive; prove mancanti possono produrre falsi negativi. Le cache hanno scadenza e i voti IMDb possono provenire dal dataset locale. Esistono limiti operativi dichiarati per singolo passaggio, necessari per tempo e consumo API; non dimostrano l'esaurimento del catalogo. Errori e indisponibilità esterni restano possibili e non attivano un ripiego silenzioso.

## Build e installazione locale

Build release compilata e installer Inno Setup eseguito con successo (codice 0). Installazione effettuata senza avviare automaticamente l'app e senza processi CineMatch attivi. L'hash del file `config.json` prima e dopo l'installazione è identico. L'eseguibile installato in `%LOCALAPPDATA%\Programs\CineMatch` corrisponde alla build release:

```text
SHA256 B43E1386CB2DAEAE101CD10FD885587C31B70B57ACE0C6AB24343DECB2DCAE86
```

Versione applicativa mantenuta a 0.10.20; questa consegna non pubblica una nuova release o un tag GitHub. Riaprire CineMatch per usare la nuova build.
