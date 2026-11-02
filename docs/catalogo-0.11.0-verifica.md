# CineMatch 0.11.0: catalogo come centro del prodotto

Consegna del 2 novembre 2026, in seguito alla scelta di eliminare la Sala 1 e concentrare il progetto su catalogo, filtri, tassonomia e correnti. I precedenti piani di chat e archivio narrativo sono superati.

## Rimozione della ricerca AI

- Eliminati interfaccia chat, home conversazionale, composer, stato e memoria della conversazione, curatore, interpreti e adattatori dei provider.
- Rimossi i moduli `ai` e `search`, gli strumenti di prova relativi e il rilevamento dei modelli all'avvio.
- Rimosse le impostazioni dei provider e la lettura delle loro credenziali dal codice dell'app.
- La configurazione precedente viene migrata conservando TMDB, lingua e SIMKL ed eliminando la sezione dei provider dal JSON attivo. Il codice non legge più le precedenti credenziali AI eventualmente rimaste nel gestore credenziali Windows; questa consegna non revoca chiavi presso servizi esterni.
- Catalogo come schermata iniziale; navigazione con Catalogo e Impostazioni. Nessun reindirizzamento automatico a un servizio AI.

TMDB, immagini, dettagli, disponibilità, IMDb locale, SIMKL e aggiornamento dell'app restano le integrazioni del catalogo. Non viene implementato né scaricato il grande archivio narrativo discusso in precedenza.

## Correzioni del catalogo

1. Esclusione dei generi dal pannello, con criteri attivi rimovibili e mutua esclusione fra inclusione ed esclusione dello stesso genere.
2. Conservati tutti i temi validi scelti, senza il precedente taglio a tre.
3. Ricerca per titolo applicata anche con filtri attivi; navigazione delle pagine originarie al posto della ricerca limitata alle prime tre pagine.
4. Totali distinti dai risultati verificati nella pagina: conteggio «in pagina» quando esistono filtri locali, decenni separati, verifiche della durata o visti nascosti. Non viene simulato un totale globale già verificato.
5. Eliminato il doppio parametro `vote_count.gte` nella combinazione Più recenti + voto minimo, causa potenziale di errore TMDB.
6. Un voto massimo richiede almeno una valutazione; un dato mancante non equivale a un voto basso.
7. Date di uscita conservate e controllate nelle ricerche per titolo con Prossimamente.
8. Filtro geografico coerente con `with_origin_country`: origine distinta dalla produzione. Una prova reale ha individuato una scheda con origine italiana e produzione non compilata; il precedente controllo usava il campo diverso.
9. Durate ricontrollate sulle schede quando richieste, compreso il minimo implicito di Più recenti. La prova reale ha rilevato Shining nel risultato server di un intervallo 70–130 minuti, con durata di 142 minuti nella scheda letta dall'app: il catalogo ora lo esclude per quel dato, senza presentarlo come compatibile.
10. Aggiunti Unione Sovietica e Filippine, già referenziate dalle correnti ma assenti dall'elenco dei paesi. Russia separata da URSS.
11. Una nuova modifica invalida subito i risultati precedenti, anche durante il debounce; le richieste obsolete sono annullate. I risultati vecchi non sono interattivi durante l'aggiornamento.
12. Stato di caricamento esplicito, errori e pagine senza corrispondenze distinti, guida e soglie implicite rese consultabili. Primo avvio senza pannelli del catalogo sovrapposti. Corretto il caso in cui la griglia vuota spingeva il messaggio fuori dalla vista.

Le correnti rimangono ricette di esplorazione basate sulla tassonomia. Non viene affermata una certificazione storica dell'appartenenza di ogni risultato a una corrente. Differenze fra edizioni e informazioni mancanti delle fonti restano possibili.

## Verifica automatica

- 89 test riusciti: 66 di libreria, 2 del contratto del catalogo, 21 sui vincoli.
- `cargo test --locked --offline --all-targets`.
- Clippy su tutti i target con `-D warnings`.
- Verifica della feature opzionale wgpu.
- `git diff --check`.

Tra le regressioni: migrazione della configurazione precedente, assenza della vecchia navigazione, inclusione/esclusione dei generi, conservazione dei temi, riferimenti della tassonomia, query TMDB, dati mancanti, paese d'origine, Prossimamente, invalidazione delle risposte obsolete e rendering dei principali stati a 800×600, 1024×768 e 1440×900.

Il rendering automatico verifica presenza dei contenuti previsti nei diversi stati; non è una certificazione di ogni pixel o ramo d'interazione.

## Prove reali con TMDB

Comando riproducibile: `cargo run --locked --offline --example catalog_probe`.

| Caso | Esito |
| --- | --- |
| Neorealismo + Commedia | PASS, 4 schede controllate |
| Horror anni 80, senza Commedia, voto minimo 6,5, almeno 1.000 voti, durata 70–130 | PASS, 4 schede controllate dopo la verifica locale della durata |
| Più recenti + voto minimo 7 | PASS, 4 schede controllate |

Sono prove mirate su 12 schede e tre combinazioni, non un censimento della correttezza di tutto TMDB. Lo strumento usa soltanto TMDB e la credenziale già configurata; nessun provider AI è presente nella nuova pipeline.

## Verifica grafica

Usata la skill Impeccable mantenendo il sistema visivo esistente. Controllo meccanico sui file modificati: nessuna segnalazione. Il risultato del detector non sostituisce la verifica visiva.

La finestra nativa 0.11.0 è stata osservata a 1220×660 e 1280×752, con dati TMDB reali. Verificati ingresso nel catalogo, navigazione priva della Sala 1, griglia, pannello, menu di esclusione e applicazione di «senza Horror». Le verifiche manuali ulteriori di ridimensionamento e impostazioni sono state lasciate al controllo dell'utente quando il sistema ha rilevato suoi input. La dimensione minima e gli stati delle impostazioni restano coperti dai test di rendering citati, non dichiarati provati manualmente in questa sessione.

L'esempio `catalog_preview` permette di controllare la build in sviluppo senza il mutex dell'app installata. Usa configurazione e dati reali del catalogo e non va confuso con una nuova installazione o con dati dimostrativi.

## Consegna

Versione applicativa e installer portati a 0.11.0. README riscritto per il catalogo; piani precedenti marcati come storici. Build release e installer verificati prima della consegna. Questa modifica del repository non pubblica automaticamente una release GitHub.

Installer locale `dist/CineMatch-Setup-0.11.0.exe`, circa 5,48 MiB. Installazione completata con codice 0; versione installata 0.11.0. Verificata l'uguaglianza SHA-256 fra eseguibile release e installato, e l'uguaglianza dell'hash della configurazione prima e dopo l'installer. L'app non è stata avviata automaticamente dall'installer.
