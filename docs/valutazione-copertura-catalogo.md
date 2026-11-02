# Valutazione della copertura del Catalogo

Valutazione del 2 novembre 2026 sulla versione 0.11.0, commit applicativo `528accf`.

## Esito

Il catalogo esegue le query implementate, ma non soddisfa il requisito «intero database accessibile e filtrabile». I test della precedente consegna verificavano comportamenti e regressioni mirati; non dimostravano completezza della fonte, enumerazione globale o accuratezza storica delle correnti.

I problemi coinvolgono selezione della sorgente, paginazione, significato dei conteggi, filtri impliciti e classificazione. Un cambiamento del solo numero nell'interfaccia non li risolve.

## Prova riproducibile

```powershell
cargo run --locked --offline --example catalog_coverage_audit
```

Lo strumento esegue sei letture di liste TMDB, senza scansionare tutte le pagine, importare archivi o interrogare provider AI. Nessuna credenziale viene stampata. I risultati dipendono dall'aggiornamento della fonte; i conteggi seguenti sono quelli osservati durante la prova.

| Query | Totale restituito | Pagine restituite | Pagine raggiungibili nell'app |
| --- | ---: | ---: | ---: |
| Apertura attuale: tendenze della settimana | 10.000 | 500 | 500 |
| Discover senza filtri espliciti, con l'attuale limite ai film usciti e datati | 20.001 | 1.001 | 500 |
| Ricetta attuale Neorealismo | 475 | 24 | 24 |
| Italia 1943–1954 senza vincoli di genere | 1.060 | 53 | 53 |
| Periodo 2000–2009, query separata | 20.001 | 1.001 | 500 |
| Periodo 2010–2019, query separata | 20.001 | 1.001 | 500 |

Ogni pagina osservata contiene 20 schede. La query generale non fornisce un censimento: lo stesso valore 20.001 ricorre per due intervalli disgiunti. Va trattato come conteggio potenzialmente saturato, non come numero esatto dei film presenti su TMDB. Non viene ricavato da questa prova il numero complessivo del database.

## Riscontri nel codice

### 1. Apertura su tendenze, non sull'universo del catalogo

In `src/app.rs`, `wants_trending` attiva `tmdb.trending` quando non ci sono filtri ed è selezionata Popolarità. L'endpoint è `/trending/movie/week`. Il totale della lista viene mostrato come «FILM» senza qualificare la sorgente. I 10.000 osservati appartengono a quella risposta.

La documentazione ufficiale definisce l'endpoint come elenco delle tendenze: [Trending Movies](https://developer.themoviedb.org/reference/trending-movies).

### 2. Limite effettivo di navigazione

`spawn_search` conserva `pages.min(500)`. Il limite non si risolve cancellando `min(500)`: TMDB documenta un errore per pagine superiori a 500. Con le 20 schede per pagina osservate, una singola ricerca permette di attraversare al massimo 10.000 schede, anche quando il totale restituito è maggiore.

Fonte: [TMDB Errors, codice 22](https://developer.themoviedb.org/docs/errors).

### 3. Il Neorealismo è una ricetta euristica

`assets/taxonomy.json` e `constraints::to_discover` producono:

- uscita primaria dal 1943 al 1954;
- origine Italia;
- presenza del genere Dramma quando non si sceglie un genere esplicito;
- esclusione di Fantasy, Animazione, Horror, Western, Fantascienza, Musica e film TV;
- nessun collegamento verificato fra l'identificativo del film e il movimento.

475 è il totale della combinazione. Fra le prime venti schede compaiono, ad esempio, Ladri di biciclette, Sciuscià, Roma città aperta, Capitan Fantasma, La primadonna e Maria Malibran. Questa prova descrive la composizione della lista e non assegna automaticamente a tutte queste opere una valutazione storico-critica.

Il confronto con 1.060 misura l'effetto dei vincoli di genere nello stesso periodo e paese. Non dimostra che 1.060 siano film neorealisti o che 475 siano tutti quelli appartenenti alla corrente. Metadati mancanti o classificazioni diverse possono produrre omissioni; il rispetto di periodo, origine e genere non prova da solo l'appartenenza.

### 4. Altre riduzioni dell'universo

- `date_bounds` applica fino a oggi agli ordinamenti normali; Prossimamente usa un intervallo futuro separato. La query Discover generale attuale non include insieme passato, futuro e date mancanti.
- `minimum_votes` introduce almeno 50 voti per Voto/Più recenti, oppure 25 con voto minimo. Più recenti introduce anche 60 minuti se non si sceglie una durata.
- `include_adult=false`; i contenuti video sono esclusi dal valore predefinito di Discover. Sono scelte di perimetro da rendere esplicite rispetto alla parola «tutto».
- La ricerca per titolo con filtri verifica solo la pagina corrente della ricerca TMDB: la prima pagina vuota non dimostra assenza globale.
- Durata, decenni discontinui e visti nascosti possono ridurre i risultati localmente, senza disporre di un totale globale filtrato.
- Le incompatibilità della tassonomia possono rimuovere criteri e bloccare combinazioni sulla base di regole editoriali. Una tassonomia rigida non equivale a una classificazione completa delle opere.

## Requisito corretto per la prossima implementazione

Il riferimento verificabile deve essere «tutti gli identificativi della fonte scelta alla data X», accompagnato dalla copertura dei metadati e delle classificazioni. «Tutti i film mai esistiti con ogni campo completo e ogni corrente certa» non è una garanzia ottenibile da queste API.

TMDB pubblica export giornalieri degli identificativi. La documentazione precisa che non sono export completi dei metadati: importare tutti gli ID non rende automaticamente filtrabili tutte le schede per durata, paesi, keyword o correnti.

Fonte: [Daily ID Exports](https://developer.themoviedb.org/docs/daily-id-exports).

## Architettura raccomandata

Un indice catalografico locale persistente, con TMDB come fonte iniziale e aggiornamenti incrementali, si adatta al requisito di consultare l'insieme con conteggi e ordinamenti globali. La ricerca resta deterministica, senza modelli AI. Dimensioni e tempi dell'import devono essere misurati su un campione prima di fissare il pacchetto.

1. **Identità e copertura.** Import riprendibile degli ID dello snapshot scelto; manifesto con data, numero atteso e numero importato. Separare identità, metadati acquisiti e classificazioni verificate. Trattare adulti, film TV, cortometraggi, video, opere future e date sconosciute come perimetri espliciti.
2. **Metadati indicizzati.** Titoli, date, generi, paesi d'origine, durata, keyword e voti in SQLite, con provenienza e data. Arricchimento a lotti, pausa/ripresa e gestione degli errori; immagini su richiesta. Il popolamento completo dei campi richiesti è un lavoro distinto dall'import degli ID.
3. **Ricerca globale.** Conteggio, filtri e ordinamento sullo stesso insieme indicizzato, paginazione stabile e deduplicazione per ID. L'interfaccia deve dichiarare la copertura parziale durante il popolamento. I dati mancanti rimangono sconosciuti e consultabili, senza essere presentati come incompatibilità verificate.
4. **Percorso online durante il popolamento.** Query disgiunte per periodi, suddivise ulteriormente quando sature, possono ampliare la navigazione oltre la singola query da 500 pagine. Servono unione ordinata, cursori e deduplicazione. Questo percorso da solo non dimostra copertura di schede senza data né dei filtri locali su tutto l'universo.
5. **Correnti documentate.** Relazione film–corrente con fonte, nota e stato: appartenenza documentata, influenza/affinità oppure candidato per ricetta. Periodo, paese e genere diventano strumenti di scoperta. Le modalità devono restare riconoscibili nell'interfaccia e nei conteggi.
6. **Filtri controllabili.** Ordinare cambia l'ordine; eventuali soglie di qualità devono essere scelte e visibili. Filtri attivi espliciti, combinazioni AND/OR leggibili e distinzione fra contraddizione dei dati e suggerimento editoriale. Più correnti devono poter essere unite oppure intersecate con una scelta comprensibile.

L'export degli ID e i pacchetti di metadati richiedono acquisizioni diverse. Non è necessario avviare un'importazione massiva per completare questa valutazione: non è stata avviata.

## Ordine di lavoro e criteri di accettazione

| Passo | Consegna | Evidenza richiesta |
| --- | --- | --- |
| 1 | Apertura su catalogo, conteggi qualificati, ambito e soglie espliciti | Le tendenze non vengono etichettate come universo; nessun totale saturo viene spacciato per esatto |
| 2 | Indice e sincronizzazione riprendibile | Confronto ID per ID con uno snapshot di riferimento, duplicati e mancanti misurati |
| 3 | Filtri e paginazione globali | Conteggio uguale all'insieme enumerato, ordinamento coerente fra pagine, nessuna perdita oltre 10.000 risultati |
| 4 | Classificazione delle correnti | Campione revisionato, fonti per appartenenza e affinità, esempi di falsi positivi/negativi misurati |
| 5 | Collaudo dell'intero percorso | Dataset con oltre 25.000 risultati, date mancanti/future, punteggi assenti, combinazioni multiple, ripartenza e aggiornamento durante la consultazione |

La correttezza delle query è verificabile con test; la copertura si verifica confrontando gli insiemi; la pertinenza delle correnti richiede anche revisione dei dati. Queste misure devono restare distinte.

## Limite della consegna di valutazione

Aggiunti questo rapporto e lo strumento riproducibile. Il comportamento dell'applicazione 0.11.0 non è stato modificato da questa valutazione. I numeri osservati non sono stati gonfiati, né è stato dichiarato completato l'indice proposto.
