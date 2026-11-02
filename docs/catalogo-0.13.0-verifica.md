# CineMatch 0.13.0: catalogo online senza acquisizione locale

La richiesta di non conservare progressivamente i dati sostituisce l'architettura
della versione 0.12.0. Il precedente documento sull'indice descrive una versione
storica, non il funzionamento attuale.

## Modifiche

- Rimossi SQLite, export degli identificativi, arricchimento generale/prioritario,
  scrittura delle schede aperte e i comandi CLI di acquisizione.
- Locandine scaricate soltanto quando visualizzate, decodificate fuori dal thread
  dell'interfaccia e conservate in una cache RAM limitata. Nessuna cache su disco.
- Cache delle liste, delle schede complete/compatte e delle keyword limitate alla
  memoria della sessione. Le schede compatte non sostituiscono quelle complete.
- Rimossi il download opzionale e la lettura del dataset IMDb locale.
- Eliminati i controlli di avanzamento dell'acquisizione. Nuovo ambito online con
  anni precisi, date, adulti, video, combinazione dei generi e correnti.
- Query con almeno 500 pagine segnalate come limitate, anche quando la risposta
  della fonte si ferma esattamente a 500 pagine e 10.000 titoli.
- Correnti alternative interrogate separatamente, senza intersecare per errore
  i loro periodi; unione delle pagine native con deduplicazione nella pagina.
- Classificazioni con fonte incluse nel programma, distinte dalle ricette.
  La raccolta resta esplicitamente parziale. Nessun censimento storico inventato.
- Rete fallita durante una verifica: errore esplicito, senza risultati scartati
  in silenzio. Filtri ulteriori sulla ricerca per titolo: ambito della pagina dichiarato.
- Conservati impostazioni, credenziali e libreria personale SIMKL. Nessuna chiamata AI.

## Collaudo

95 test automatici superati (72 unitari, 2 contratti del catalogo, 21 vincoli),
Clippy su tutti i target senza warning e compilazione opzionale wgpu riuscita.
I test comprendono query ampie senza acquisizione di metadati, limite
esatto di 500 pagine, anni incompatibili, anni bisestili, AND/OR, origini delle
correnti, deduplicazione, sorgenti esaurite, pagine vuote con navigazione,
fallimento della verifica e classificazioni documentate fuori dal periodo della ricetta.

Rendering egui verificato a 800×600, 1024×768 e 1440×900 per catalogo, ambito
della ricerca aperto, caricamento, errore, impostazioni e primo avvio.
Il detector Impeccable non ha restituito rilievi sui file UI modificati.

Prove reali TMDB, attraverso lo stesso motore usato dall'app: Neorealismo con
commedia, horror anni Ottanta senza commedia con durata/voti espliciti,
recenti con voto minimo, titolo Metropolis, sei appartenenze documentate del
Neorealismo, anno preciso con uscite future. Tutte riuscite. Nessun provider AI.

## Limiti espliciti

Internet necessario. TMDB impone 500 pagine per query e non garantisce il censimento
di ogni film o la completezza di ciascun metadato. Gli anni precisi e gli altri
filtri restringono la selezione senza scaricare un archivio. Un anno molto ampio
può comunque richiedere altri criteri.

Con più correnti alternative l'ordine vale per la pagina riunita, non per una
fusione globale; possono esistere sovrapposizioni fra pagine delle fonti.
I controlli aggiuntivi sulla ricerca per titolo o sulle durate possono ridurre
la pagina corrente senza ridurre il numero di pagine della fonte. La UI lo dichiara.

La RAM temporanea non è un archivio persistente. Programma, impostazioni e dati
personali SIMKL continuano naturalmente a occupare spazio sul disco.

## Installazione e recupero dello spazio

Installer 0.13.0 eseguito sul PC. Hash del binario installato uguale alla build
release; versione verificata. Configurazione e libreria SIMKL identiche, verificate
con SHA-256 prima e dopo installazione/pulizia. Nessuna classificazione personale
aggiunta al precedente database: le 18 relazioni presenti coincidono con le fonti incluse.

Eliminati database, journal, export, cache di locandine, dataset IMDb e vecchie
cache narrative: **362.768.641 byte, circa 345,96 MiB**. Conservati impostazioni,
credenziali, libreria personale e file di cronologia/diagnostica preesistenti.
Rimossi anche **23,10 GiB di artefatti di compilazione** dalla cartella `target`,
dopo test, build e installazione. Il setup aggiornato rimane in `dist`.

Verifica nativa dell'app installata a 1220×660: catalogo con locandine caricate,
versione 0.13.0, limite TMDB visibile, nuovo ambito espandibile e campi degli anni
utilizzabili senza sovrapposizioni. Dopo l'avvio e le richieste online non sono
ricomparsi database, export o cartelle di cache dei film.

SHA-256 del setup: `FC555330DA86D7747AAB71A5B32F94F2F80EECAB756B4933FA954D1FC875AB40`.
