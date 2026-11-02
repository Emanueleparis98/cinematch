# CineMatch 0.12.0: indice globale e copertura verificabile

Consegna del 2 novembre 2026. Attuazione della valutazione del catalogo 0.11.0 e della richiesta di procedere con indice, filtri globali, tassonomia e correnti con fonti. La chat e i provider AI restano assenti.

## Risultato e perimetro

Importato sul PC lo snapshot standard TMDB del 2 novembre: **1.268.034 identificativi**, con validazione del numero di righe e degli ID unici prima della pubblicazione. L'ambito iniziale omette i record contrassegnati come adulti o video e conta **1.202.568 titoli, distribuiti su 60.129 pagine**. L'export per adulti è un'acquisizione separata e facoltativa.

Questa è copertura degli identificativi dello snapshot scelto. Non significa che tutti i metadati siano già acquisiti o che tutte le opere siano classificate storicamente. Il completamento è progressivo, interrompibile e riprendibile; durante il collaudo grafico ha superato le 3.000 schede acquisite. Il numero continua a cambiare mentre l'app è aperta e il lavoro è attivo.

La barra distingue identità nell'indice e schede acquisite. I risultati filtrati contano i metadati verificabili acquisiti; le schede non ancora arricchite non vengono presentate come corrispondenze certe. Anche una scheda acquisita può avere campi assenti nella fonte.

## Implementazione

- Indice SQLite con FTS5, titoli originali/localizzati e alias acquisiti, generi, paesi d'origine, keyword, durate e voti.
- Import degli ID in staging, controlli su duplicati e dati illeggibili, pubblicazione atomica e conservazione dello snapshot precedente in caso di interruzione o errore. Replay idempotente dello stesso export.
- Un flag `null` nell'export reale ha fatto emergere un caso non coperto dal modello iniziale: parser reso compatibile e regressione aggiunta.
- All'avvio si consulta l'indice; il catalogo non viene più sostituito dalle tendenze settimanali. L'indice acquisito si consulta anche senza credenziale configurata.
- Conteggio, ordinamento e pagine sullo stesso snapshot SQLite. Salto diretto alla pagina con Vai; nessun tetto locale di 500 pagine o 10.000 titoli.
- Snapshot di lettura conservato durante la navigazione, per evitare che un aggiornamento cambi appartenenza, valori o ordine fra pagine. Cerca/Aggiorna riapre la selezione. Dopo cinque minuti una selezione può richiedere un rinnovo esplicito per liberare il journal; non viene riordinata silenziosamente.
- Completamento generale in background e completamento prioritario dei filtri scelti. Usa un endpoint di metadati compatto, senza filmografie e suggerimenti inutili all'indice.
- Query online sature suddivise per anni, mesi e giorni; deduplicazione e messaggio esplicito nei casi non enumerabili. Le date assenti restano affidate al completamento generale.
- Pausa persistente al riavvio; progressi dei metadati salvati per singolo film. Retry differiti, arresto dopo errori consecutivi e annullamento delle richieste superate.
- Nessuna soglia implicita di 50 voti o 60 minuti introdotta dall'ordinamento. Eliminato anche il taglio del minimo voti a un milione.
- Intervalli di durata con etichette coerenti: il preset Lungo oltre 120 minuti non nasconde più un tetto a 600 minuti. Verifica su una scheda di prova da 1.200 minuti.
- Generi in AND o OR esplicito, paesi in alternativa, temi in AND, correnti unite o intersecate a scelta. Le incompatibilità editoriali non cancellano più criteri dell'utente.
- Ambiti delle date espliciti, record senza data distinti dai titoli ancora da completare, dati senza valutazioni distinti dai voti bassi.
- 251 codici di paese dalla configurazione TMDB in italiano, integrati con le voci storiche preesistenti. Decenni estesi al periodo 1870–2099.
- Segnaposto statici e descrittivi per le schede ancora da completare, senza animare indefinitamente locandine inesistenti.

## Correnti e classificazioni

Tre modalità dichiarate: candidati per ricetta, appartenenza documentata e appartenenza/affinità con fonte. La prima usa periodo, origine ed eventuale keyword; non è una certificazione dell'appartenenza.

La raccolta iniziale del Neorealismo contiene sei relazioni documentate e dodici affinità editoriali. Gli ID sono stati risolti mediante titolo originale e anno e verificati con TMDB. Le note e gli URL sono inclusi in `assets/movement-facts.json` e consultabili dalla scheda.

Le fonti iniziali sono BFI e Criterion. Le relazioni di affinità tratte dalla raccolta editoriale Criterion vengono conservate come affinità, senza trasformarle tutte in appartenenza stretta. Le altre correnti mantengono le ricette; la loro classificazione documentale resta da popolare.

Il caso Miracolo a Milano verifica una differenza sostanziale: un'appartenenza documentata può essere trovata anche se il film ha un genere che la vecchia tassonomia vietava automaticamente.

Importazione di ulteriori classificazioni da file JSON attraverso un selettore nativo. Validazione di ID, corrente, relazione, fonte HTTPS e nota, con transazione atomica. La verifica della struttura del file non certifica il contenuto critico delle fonti importate.

## Prove reali e prestazioni

Comando riproducibile: `cargo run --locked --offline --example catalog_index -- verify`.

| Caso sull'indice reale | Risultato | Ultimo tempo osservato prima della consegna |
| --- | --- | ---: |
| Pagina 501, ambito iniziale | 20 schede; 1.202.568 titoli, 60.129 pagine | 115 ms |
| Neorealismo, candidati per ricetta | 1.059 corrispondenze nei metadati acquisiti, 53 pagine | 149 ms |
| Neorealismo documentato | 6 schede, incluso Miracolo a Milano | 260 ms |
| Neorealismo con affinità | 18 schede | 278 ms |

Sono misure locali di un campione, non un obiettivo garantito su qualsiasi PC o query. Le prime prove avevano rilevato circa 22 secondi per i candidati e 10–15 secondi per le fonti: risolte le scansioni estese mediante insiemi di ID indicizzati, indici parziali e conteggi di copertura dedicati. Il worker dei metadati utilizza anch'esso query indicizzate.

Il conteggio dei candidati usa data e origine effettivamente conservate nelle schede e l'ambito scelto; non viene forzato a coincidere con un totale online né gonfiato per sembrare più completo. I risultati possono cambiare con la fonte e il completamento.

## Verifica automatica

**105 test riusciti:** 70 di libreria, 2 contratti del catalogo, 12 dell'indice e 21 sui vincoli.

- `cargo test --locked --offline --all-targets`.
- `cargo clippy --locked --offline --all-targets -- -D warnings`.
- `cargo check --locked --offline --features wgpu`.
- `cargo build --locked --offline --release`.
- `git diff --check`.

Tra i casi: 25.017 titoli senza perdita oltre la pagina 500, ultima pagina e replay, snapshot interrotto/corrotto, ritiro degli ID assenti dal nuovo export, coerenza di conti e schede durante aggiornamenti, query di titoli con accenti e metacaratteri, alias, generi AND/OR, ambiti, dati mancanti, origini, correnti multiple, fonti atomiche, durata oltre 600 minuti, keyword e pausa persistenti.

Rendering egui su catalogo, caricamento, errori, impostazioni e primo avvio a 800×600, 1024×768 e 1440×900; percorso dell'indice offline e nuovi controlli anche a 1220×660 e 1920×1080. Sono prove automatiche di contenuto e layout, non una certificazione di ogni pixel.

## Verifica grafica

Usata Impeccable mantenendo il sistema Lumen. Un passaggio del detector sui file UI modificati non ha prodotto segnalazioni; questo non sostituisce il controllo nativo.

Osservata la finestra finale a 1220×660 con dati reali: conteggi, schede, progressi, apertura persistente di Archivio e copertura, controlli di ambito e classificazione, immissione del valore 501, comando Vai e caricamento della pagina 501 di 60.129. Verificati i segnaposto dei titoli non ancora arricchiti.

Il controllo nativo ha individuato e corretto la richiesta di apertura del pannello che si esauriva in un solo frame; aggiunta una regressione sul suo stato persistente. Il tentativo di ridimensionamento tramite il gesto dell'automazione non ha cambiato la dimensione della finestra e non viene contato come una verifica nativa del minimo. Il minimo resta coperto dai test egui sopra citati.

Non sono stati collaudati manualmente tutti i rami, l'intero selettore di importazione e ogni combinazione della tassonomia. Il popolamento completo di tutti i metadati e la revisione documentale di tutte le correnti restano lavori progressivi sui dati.

## Installazione

Setup locale `dist/CineMatch-Setup-0.12.0.exe`, circa 6,36 MiB; eseguibile release circa 13,04 MiB. Installazione eseguita con codice 0, versione installata 0.12.0, SHA-256 identico fra release e installato e configurazione invariata. Indice conservato nel profilo utente.

SHA-256 dell'eseguibile installato: `D69D262B56D9153D6DE301DFF2CED9F974C7D2BDE2FE0C942AB62B30F1EF0620`.

L'indice misurato durante il collaudo occupava circa 263,6 MiB, più il journal temporaneo; export compresso circa 27,1 MiB. Il file può crescere durante il completamento dei metadati. Non è incorporato nel setup né caricato sul repository.

Non sono state effettuate chiamate a provider AI. La consegna del codice non pubblica automaticamente una release GitHub.

## Riferimenti

- [TMDB: Daily ID Exports](https://developer.themoviedb.org/docs/daily-id-exports).
- [TMDB: Countries](https://developer.themoviedb.org/reference/configuration-countries).
- [TMDB: Errors e limite delle pagine](https://developer.themoviedb.org/docs/errors).
- [BFI: The roots of neorealism](https://www.bfi.org.uk/sight-and-sound/features/roots-neorealism).
- [BFI: Bicycle Thieves](https://www.bfi.org.uk/film/594f7408-2fdd-55a0-a347-79370e42e0ed/bicycle-thieves).
- [BFI: Rome, Open City](https://www.bfi.org.uk/bfi-film-releases/rome-open-city-roma-citta-aperta).
- [Criterion: Paisan](https://www.criterion.com/films/2415-paisan).
- [Criterion: Italian Neorealism](https://www.criterion.com/shop/collection/6-italian-neorealism).
