# CineMatch

Catalogo cinematografico desktop per Windows, versione 0.13.0. Ricerca per titolo,
filtri combinabili, tassonomia e correnti. Nessuna chat e nessun provider AI.

## Consultazione online, senza archivio locale

CineMatch consulta TMDB quando apri il catalogo, cerchi, cambi filtri, pagina o
scheda. Non scarica gli export degli identificativi, non acquisisce progressivamente
il database e non salva film o locandine sul disco. Le risposte e le immagini
rimangono in cache limitate nella RAM della sessione e scompaiono alla chiusura.
Non esistono lavori di completamento o indicizzazione in background.

Restano sul PC il programma, le impostazioni e, se collegata, la libreria personale
SIMKL. Le credenziali sono conservate nel Windows Credential Manager. La consultazione
richiede Internet. Il download di un aggiornamento del programma rimane un'azione
esplicita, distinta dai dati del catalogo.

## Uso e limiti della fonte

1. Configura una API Key v3 o un Read Access Token v4 TMDB.
2. Cerca un titolo oppure combina periodi, generi, paesi, temi e correnti.
3. In **Ambito della ricerca** scegli date, anni precisi, adulti, contenuti video
   e AND/OR dei generi. I filtri non aggiungono soglie editoriali di durata o voti.
4. Naviga con le frecce o scrivi il numero di pagina e premi **Vai**.

TMDB consente al massimo **500 pagine per singola query**. Una ricerca molto ampia
può superare il numero di titoli effettivamente navigabili: l'app segnala il limite,
non presenta quel totale come il censimento di tutti i film. Restringi gli anni o
aggiungi criteri per accedere ad altri sottoinsiemi. Non viene promessa la copertura
al 100% dei film esistenti o di ogni combinazione dei loro metadati.

La ricerca per titolo usa l'endpoint TMDB dedicato. Gli ulteriori filtri e gli
ordinamenti si applicano alla pagina corrente; il paginatore segue le pagine della
ricerca per titolo. Anche le verifiche aggiuntive sulle durate e sui decenni separati
possono produrre pagine vuote: la navigazione continua e il limite viene dichiarato.
Una verifica di rete fallita restituisce un errore, senza scartare in silenzio il film.

Le risposte TMDB possono cambiare durante la sessione. **Aggiorna** rinnova le cache
della ricerca. I temi sono keyword TMDB in AND: non c'è interpretazione narrativa,
né eliminazione silenziosa dei criteri che non si riescono a risolvere.

## Correnti e fonti

- **Candidati per ricetta**: periodo, origine ed eventuali keyword. Non certifica
  l'appartenenza storica di ogni risultato.
- **Appartenenza documentata**: solo relazioni con fonte incluse nell'app.
- **Appartenenza e affinità con fonte**: comprende le selezioni editoriali più ampie.

Le classificazioni incluse sono una raccolta parziale: per il Neorealismo,
sei appartenenze documentate e dodici affinità da BFI e Criterion. I dati storici
non vengono inventati per riempire le altre correnti. Le schede mostrano relazione,
nota e collegamento alla fonte. Le classificazioni con fonte non ereditano
automaticamente i limiti cronologici delle ricette.

Le correnti sono alternative, oppure puoi richiederle tutte. Nel primo caso, con
più correnti, ciascuna pagina riunisce la stessa pagina delle singole query e rimuove
i duplicati al suo interno. L'ordine vale per quella pagina; le fonti possono
sovrapporsi fra pagine. L'interfaccia lo dichiara e non mostra un totale unico esatto.

## Sviluppo

```powershell
cargo test --all-targets
cargo clippy --all-targets -- -D warnings
cargo check --features wgpu
cargo run --example catalog_probe
cargo build --release
ISCC.exe /DMyAppVersion=0.13.0 installer\cinematch.iss
```

Il probe reale usa soltanto TMDB, senza AI, e non conserva le risposte sul disco.
Il precedente indice SQLite e i comandi di acquisizione sono stati rimossi.
I documenti della versione 0.12.0 descrivono il comportamento storico, sostituito
in questa versione. Verifica corrente in `docs/catalogo-0.13.0-verifica.md`.
