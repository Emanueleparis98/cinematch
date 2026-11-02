# CineMatch

Catalogo cinematografico desktop per Windows, versione 0.12.0. Filtri combinabili,
periodi, generi, paesi, temi e correnti; ricerca deterministica senza provider AI.

## Catalogo e copertura

Il catalogo usa un indice SQLite persistente sul PC. Il primo avvio con TMDB
configurato acquisisce l'export degli identificativi e completa progressivamente
le schede. Puoi esplorare l'indice durante il completamento.

L'export contiene identificativi, titoli originali e alcuni attributi; non è
un export completo dei metadati. Generi, paesi d'origine, durata, keyword,
voti, titoli alternativi e locandine vengono acquisiti separatamente.
La barra distingue titoli nell'indice e schede acquisite. Una scheda acquisita
può comunque avere campi mancanti nella fonte.

Lo snapshot TMDB del 2 novembre 2026 provato in sviluppo contiene 1.268.034
identificativi; l'ambito iniziale, che esclude adulti e contenuti video, conta
1.202.568 titoli. Sono misure di quello snapshot, non il censimento di tutti
i film mai esistiti né una garanzia della completezza delle classificazioni.

## Uso

1. Configura TMDB con una API Key v3 o un Read Access Token v4.
2. Attendi la pubblicazione dell'indice iniziale; il completamento delle schede
   prosegue in background. La barra ne mostra l'avanzamento.
3. Cerca un titolo o combina i filtri. I conteggi filtrati riguardano i metadati
   acquisiti e verificabili; i titoli ancora privi di quei campi restano da completare.
4. Usa **Completa filtri** per dare priorità alle schede della selezione corrente.
5. In **Archivio e copertura** puoi sospendere, riprendere o aggiornare l'indice,
   scegliere l'ambito delle date e includere i contenuti video. L'indice per adulti
   richiede il suo export separato, acquisibile dallo stesso pannello.
6. Nella barra inferiore scrivi la pagina e premi **Vai**: l'indice locale
   consente di superare la pagina 500 e le prime 10.000 schede.

Il completamento conserva il lavoro già fatto. Una pausa esplicita resta tale
anche al riavvio. Durante l'import degli identificativi si continua a leggere
lo snapshot precedente, fino alla pubblicazione atomica di quello nuovo.
Una lettura interrotta dell'export viene ripetuta senza duplicare gli ID;
un download parziale viene riscaricato.

## Significato dei filtri

- L'apertura mostra il catalogo indicizzato, anziché le tendenze settimanali.
- Gli ordinamenti cambiano l'ordine: non impongono 50 voti o 60 minuti.
- L'ambito delle date è esplicito: tutte, già usciti, future oppure data assente
  nelle schede acquisite. I titoli non ancora arricchiti non vengono dichiarati
  verificati come privi di data.
- I generi sono in AND; **Unisci i generi selezionati (OR)** consente le alternative.
- I paesi sono alternative. Il pannello comprende i codici della configurazione
  TMDB, con le voci storiche della tassonomia conservate.
- I temi sono requisiti in AND. Keyword non risolte vengono segnalate, senza
  allargare silenziosamente la ricerca. Non esiste il precedente taglio a tre temi.
- Escludere un genere richiede una classificazione di genere acquisita;
  un titolo non classificato non viene trattato come sicuramente compatibile.
- Voti mancanti e durate mancanti restano sconosciuti. I limiti di voto o durata
  richiedono i dati corrispondenti. Una valutazione senza votanti non è un voto basso.
- I criteri non vengono cancellati per generalizzazioni editoriali sulle correnti.

I titoli originali sono ricercabili già dall'export; titoli localizzati e alternativi
diventano ricercabili quando acquisiti. La ricerca per parole usa prefissi e gestisce
gli accenti. Non è una chat e non interpreta richieste narrative.

## Correnti e fonti

**Candidati per ricetta** usa periodo, origine ed eventuale keyword. Il conteggio
è etichettato come candidati: non certifica l'appartenenza storica.

**Appartenenza documentata** usa le relazioni film–corrente con una fonte acquisita.
**Appartenenza e affinità con fonte** comprende anche le selezioni editoriali più
ampie. La scheda mostra relazione, nota e collegamento alla fonte.

La raccolta iniziale è parziale: per il Neorealismo include sei appartenenze
documentate e dodici affinità da BFI e Criterion. Le altre correnti restano
esplorabili per ricetta; non vengono inventate classificazioni per riempirle.
Più correnti si uniscono, oppure si intersecano scegliendo **Richiedi tutte**.

Puoi importare un file JSON con **Importa classificazioni con fonte…**. Il formato
è quello di `assets/movement-facts.json`: ID TMDB presente nell'indice, ID di
corrente riconosciuto, relazione `documented` o `affinity`, URL HTTPS e nota.
La validazione è atomica: un record non valido impedisce l'intero import.
La presenza di un URL non sostituisce la revisione critica del suo contenuto.

## Pagine e aggiornamenti

Conteggio e schede della navigazione condividono uno snapshot SQLite. I cambiamenti
in background non riordinano silenziosamente le pagine già aperte. **Cerca** o
**Aggiorna** apre una nuova selezione. Una selezione conservata oltre cinque minuti
può richiedere un aggiornamento esplicito per permettere il ricambio dei dati e
limitare la crescita del journal; viene mostrato un messaggio, senza spostare
silenziosamente i film fra le pagine.

Il completamento online dei filtri suddivide le query sature per anni, mesi e
giorni. I casi non enumerabili vengono dichiarati. Le schede senza data continuano
a dipendere dal completamento generale; questo percorso non viene dichiarato
esaustivo per tutti gli ID mentre i metadati sono ancora parziali.

## Dati e integrazioni

L'indice si trova in `%APPDATA%\CineMatch\catalog.sqlite3`; gli export in
`%APPDATA%\CineMatch\catalog-exports`. Il setup non incorpora il database
personale. Credenziale TMDB nel Windows Credential Manager. SIMKL facoltativo
per visti, lista e voti; scritture soltanto sulle azioni dell'utente. Dataset IMDb
non commerciale opzionale per i voti. Immagini e schede complete richiedono rete
quando assenti dalle cache; l'indice acquisito rimane consultabile senza credenziale.

## Sviluppo e verifica

```powershell
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
cargo check --locked --offline --features wgpu
cargo build --locked --offline --release
ISCC.exe /DMyAppVersion=0.12.0 installer\cinematch.iss
```

Gli esempi `catalog_index status`, `page 501` e `verify` leggono l'indice.
`import`, `enrich [numero]` e `selection [corrente]` acquisiscono dati TMDB reali.
I test automatici usano fixture e non interrogano provider AI. Dettagli della
consegna in `docs/catalogo-0.12.0-verifica.md`.

## Fonti

- [TMDB: export degli ID](https://developer.themoviedb.org/docs/daily-id-exports)
- [TMDB: paesi](https://developer.themoviedb.org/reference/configuration-countries)
- [TMDB: limiti delle pagine](https://developer.themoviedb.org/docs/errors)
- [BFI: radici del Neorealismo](https://www.bfi.org.uk/sight-and-sound/features/roots-neorealism)
- [Criterion: Italian Neorealism](https://www.criterion.com/shop/collection/6-italian-neorealism)

This product uses the TMDB API but is not endorsed or certified by TMDB.
Uso non commerciale secondo i termini TMDB. I dataset IMDb sono soggetti alle
condizioni per uso personale e non commerciale.
