# CineMatch

App desktop Windows in Rust per esplorare film su TMDB con una **barra di ricerca
combinata a vincoli**: non tutte le combinazioni di tassonomia sono ammesse, e il
programma spiega sempre il perche.

L'interfaccia ha due viste, commutabili dalla rail di sinistra: la **Chat**
per chiedere in liberta e ricevere fino a cinque film pertinenti, la **Griglia**
per esplorare con il pannello dei filtri.

Esempio: `anni '80` + `Neorealismo` + `Horror` e impossibile — il Neorealismo copre
il 1943–1954 in Italia, quindi non puo coesistere con gli anni '80. Se scegli prima
gli anni '80, il Neorealismo **non e selezionabile**; se scegli prima il Neorealismo,
il periodo si stringe automaticamente al 1943–1954.

Viceversa `anni '80` + `Giallo Italiano` + `Horror` e legittimo: il periodo effettivo
diventa 1980–1982, il paese implicato e l'Italia, e il motore esclude d'ufficio i
generi estranei al movimento.

## Installazione (Windows)

La pagina di download e:

<https://emanueleparis98.github.io/cinematch/>

In alternativa, l'installer e il manifesto aggiornamenti sono nelle
[release GitHub](https://github.com/Emanueleparis98/cinematch/releases).
CineMatch e un'app Windows 10/11 a 64 bit e l'installer e **per-utente**:
installa in `%LocalAppData%\Programs\CineMatch` e non chiede i permessi di
amministratore. La configurazione resta in `%APPDATA%\CineMatch`.

### Aggiornamenti

L'app installata controlla la release piu recente all'avvio e ogni sei ore.
Quando c'e una nuova versione compare un puntino sull'ingranaggio delle
Impostazioni; download e installazione partono solo quando premi
**Scarica e installa**. Il setup e completo, viene verificato con SHA-256 e
poi avviato dall'app. Configurazione, chiavi API, cache dei poster e dati
locali non vengono toccati.

La pipeline di release e pronta per la firma Authenticode: se nel repository
sono presenti i secret `SIGN_PFX_BASE64` e `SIGN_PFX_PASSWORD`, CI firma
eseguibile e installer. Senza certificato la release e comunque pubblicata,
ma Windows puo mostrare l'avviso "editore sconosciuto".

## Sviluppo

```powershell
.\run.ps1
```

Il launcher chiude l'istanza aperta, ricompila in release (incrementale) e avvia
il binario fresco. In fondo alle Impostazioni c'e `build gg/mm hh:mm`: se la
data non e quella di adesso, la finestra aperta e una build vecchia. In alternativa:

```powershell
cargo run --release
```

Al primo avvio si apre la finestra **Impostazioni**: serve una chiave API TMDB
(gratuita).

1. Crea un account su [themoviedb.org](https://www.themoviedb.org/signup)
2. Vai su [Impostazioni → API](https://www.themoviedb.org/settings/api)
3. Incolla **una delle due credenziali** nel campo e premi *Salva*:
   - **API Key v3** (32 caratteri esadecimali) → inviata come parametro `api_key`
   - **API Read Access Token v4** (JWT, inizia con `eyJ`) → inviato come header
     `Authorization: Bearer`

CineMatch riconosce da sola il formato (il campo lo dice sotto la casella) e, se
incolli il token v4, ne ricava anche la chiave v3 dal claim `aud` come ripiego.
Il pulsante **Verifica credenziale** fa una lettura di prova e riporta l'esito senza
salvare nulla.

La chiave viene salvata in `%APPDATA%\CineMatch\config.json` (uso personale: resta
sulla tua macchina).

## Chat dei consigli (la casa)

CineMatch si apre sulla **Chat**: scrivi in liberta, anche a modo tuo, e la
risposta e una conversazione con **fino a cinque film reali** presi da TMDB.
Ogni film ha una card con poster, voto TMDB e IMDb, regia, durata, dove si vede,
il **perche** quel film risponde alla richiesta e un **consiglio** pratico (come
guardarlo, a cosa fare attenzione). Se la richiesta e molto specifica e nessun
film copre tutti i criteri, la chat lo dice e mostra soltanto i titoli piu vicini,
con il criterio mancante indicato nella card: non riempie mai la lista con film
generici.

La casa e una colonna centrata: titolo, **quattro spunti vivi** e campo di
scrittura fino a 820 px. Invio invia, Shift+Invio va a capo; gli spunti e
«Sorprendimi» riempiono soltanto il testo, senza inviare e senza chiamare AI o
TMDB. Quando la conversazione parte, il campo si aggancia in basso (sempre 820
px) e il contenuto resta in una colonna da 980 px. Mentre rileggi scorrendo
verso l'alto il campo si riduce a una barra da 44 px; torna intero quando arrivi
in fondo, con un clic, con `/` o appena inizi a scrivere.

- **Quattro spunti locali, sempre legali**: le card nascono dalla tassonomia
  (correnti, atmosfere, periodi, scoperte) e vengono confrontate con i filtri
  attivi tramite lo stesso motore di vincoli della griglia. Se una combinazione
  richiederebbe una correzione, la card non compare. Il catalogo si costruisce
  in memoria, senza rete.
- **Rotazione lenta**: ogni circa 7 secondi arriva un quartetto nuovo, con una
  comparsa sobria; nessuna card ripete subito quella precedente.
  La rotazione si ferma mentre il mouse e sopra una card, cosi il clic non
  scappa. «Sorprendimi» pesca una proposta locale non visibile.

- **Due viste, due stati**: la rail a sinistra cambia vista (Chat, Griglia, Nuova,
  Setup); la striscia in alto porta marchio, contesto e controlli finestra. La
  chat conserva la bozza, la griglia conserva ricerca, filtri e larghezza del
  pannello; i filtri compaiono solo in Griglia. Filtri e scheda film non stanno
  mai aperti insieme.
- **Il contesto segue i seguiti**: l'app conserva il contesto per i seguiti
  espliciti («no, senza commedia», «e uno piu recente?», «e se fossero italiani?»).
  Una richiesta nuova riparte da criteri puliti: non si porta dietro i temi del
  messaggio precedente. «Nuova chat» azzera tutto (niente file: tutto in memoria).
- **Anche le domande fuori tema**: se chiedi qualcosa che non riguarda il cinema
  («qual e la capitale della Francia?»), l'intro risponde in una riga e i film
  sono il ponte verso la sala: il paese, il tema o, se non c'e un legame, i film
  del momento. La domanda non viene mai scambiata per un titolo.
- **Registi e attori**: «film di Vittorio De Sica», «regia di...», «film con...»
  risolvono la persona su TMDB e usano la filmografia. Gli omonimi vengono
  disambiguati per ruolo e popolarita; il film appena visto resta fuori.
- **I film non si inventano**: l'interprete produce i filtri, il motore di
  vincoli li valida, la ricerca interroga TMDB e il **curatore** puo solo
  scegliere e commentare i candidati reali. Gli id fuori elenco vengono scartati;
  se l'AI manca o sbaglia, la scelta e deterministica (voto pesato dai voti) e la
  chat funziona lo stesso, modalita regole compresa.
- **Intro in streaming**: la presentazione arriva parola per parola (SSE), poi
  compaiono le card in sequenza. Se il servizio ignora lo streaming la risposta
  unica viene letta lo stesso; se la curatela non arriva in tempo, la scelta e
  deterministica e la chat non si blocca.
- **Temi liberi e tassonomia**: la chat distingue le facce della tassonomia
  («neorealismo comico romantico») dai temi semantici («cani e gatti amici»,
  «viaggio nello spazio»). I temi vivono solo nella chat: non cambiano i filtri
  della griglia e non diventano keyword TMDB fuzzy. La ricerca usa concetti,
  keyword esatte e titoli di riferimento, poi verifica la trama dei candidati.
- **Onesta**: se un film non copre tutti i concetti, la card dice cosa manca e la
  risposta lo dichiara. Se i criteri esatti non bastano, l'app allarga solo
  quanto necessario e lo dice; se non trova nulla di pertinente, non inventa e
  non ripiega sui film del momento. Con zero risultati la chat spiega il limite.
- **Da chat a griglia**: «Vedi tutti i N in griglia» compare solo quando i criteri
  sono davvero rappresentabili come filtri (niente temi liberi) e porta gli stessi
  filtri nella griglia, con l'intento in cache: nessuna chiamata AI nuova.
  La richiesta naturale «altri» riusa lo stesso contesto, esclude i film appena
  visti e propone altri titoli pertinenti, non necessariamente cinque. Toccare un
  filtro a mano riporta alla griglia. La scheda film resta quella di sempre (clic
  sul poster).

## Aspetto e interazioni

- **LUMEN**: editoriale e cinematografico. Fondo ink (`#08090C`), superfici piatte,
  avorio per il testo, ember (`#FF5C35`) per i comandi e celadon per le
  disponibilita. Niente gradienti, aloni o bolle: solo linee sottili, tipografia e
  poster.
- **Tipografia inclusa**: Fraunces per i titoli, Instrument Sans per il testo e
  JetBrains Mono per etichette, metadati e comandi. I font (licenza OFL) stanno in
  `assets/fonts`; i ripieghi CJK arrivano solo quando servono davvero.
- **Shell**: rail da 72 px (Chat, Griglia, Nuova, Setup), striscia superiore da
  48 px con marchio, contesto e controlli finestra, barra di stato da 28 px con
  versione, sorgente e provider. La finestra si trascina dalla striscia e si
  ridimensiona dai bordi.
- **Chat**: casa con hero, collage di poster reali, composer e quattro spunti
  locali in griglia 2x2; risultati con un film in evidenza e le altre card sotto.
  Il composer resta fisso sopra la barra di stato durante la conversazione. Stop,
  riprova e Â«Vedi tutti in grigliaÂ» restano dove erano.
- **Griglia**: poster 2:3 senza riquadro, titolo e metadati sotto, filo ember sulla
  card selezionata. Colonne responsive: due con i filtri a 800 px, tre senza;
  a 1220 px quattro con i filtri e cinque senza. Il pager sta in basso, fuori
  dallo scroll.
- **Filtri**: pannello a fisarmonica su fondo ink, testata ferma (titolo,
  conteggio, azzera, aiuto, ricerca interna), pill dei filtri attivi, sezioni in
  mono maiuscolo e chip piatte con bordo ember quando attive.
- **Scheda film**: pannello laterale fisso con backdrop e poster, titolo Fraunces,
  fatti, azioni (cerca, riassumi, dove vederlo), disponibilita e simili.
- **Impostazioni**: pagina intera con colonna centrata: TMDB, interprete, lingua,
  dati IMDb, aggiornamenti e salvataggio. Â«ChiudiÂ» o `Esc` tornano alla vista.
- **Ridimensionamento**: minimo 800x600; i testi vanno a capo e i pannelli non si
  sovrappongono. A 1220x660 la griglia passa a cinque colonne senza filtri
  (quattro con i filtri) e la scheda resta leggibile.
- **Da tastiera**: il fuoco e sempre visibile (filo ember) su card, chip e
  pulsanti; Tab naviga, Invio/Spazio attivano, `/` porta al campo di ricerca,
  `Esc` chiude scheda, filtri, guida o impostazioni. I toast entrano scivolando ed
  escono in dissolvenza.
## Barra di ricerca (Griglia)

Nella scheda **Griglia** l'unico campo accetta sia un titolo sia una frase in
linguaggio naturale:

| Input | Interpretazione |
|---|---|
| `Blade Runner` | ricerca per titolo |
| `giallo italiano anni 80, voto alto, senza commedia` | facce: periodo 1980s, movimento Giallo Italiano, genere Mistero, escluso Commedia, paese IT, voto ≥ 7 |
| `film del neorealismo italiano di guerra con voti maggiori di 8` | corrente Neorealismo, paese IT, genere Guerra, voto ≥ 8 |
| `thriller americano con almeno 5000 voti, voto tra 6 e 8` | voto 6–8 e almeno 5000 voti |
| `tipo Parasite` | film simili a Parasite (endpoint recommendations) |
| `"Il Padrino" anni 70` | titolo tra virgolette + periodo |

Puoi scrivere in modo libero e colloquiale: l'interprete estrae quello che riconosce
(periodo, corrente, paese, genere, voto, numero di voti, durata, ordinamento) e ignora
il resto. Gli argomenti che non stanno in tassonomia diventano **temi liberi**
(parole chiave TMDB verificate a runtime) e restano visibili come chip, cosi puoi
toglierli con un clic.

L'interprete predefinito e **a regole, offline e istantaneo**. Nelle impostazioni puoi
attivare **DeepSeek** (V4.1 Flash, sforzo massimo), **OpenRouter** oppure **Ollama**
locale: l'AI propone solo le facce, **la validazione resta al motore di vincoli** -
nessuna allucinazione puo finire nei risultati.

## Pannello dei filtri

Il pannello **Filtri** e una fisarmonica con la testata sempre ferma in
alto. Vive solo nella Griglia: in Chat sparisce, ma stato e larghezza restano
memorizzati e tornano al rientro. Si apre un pannello per volta: aprire i filtri
chiude la scheda film e viceversa.

- **Testata sticky**: titolo «Filtri», numero di filtri accesi, «Azzera», aiuto
  e ricerca interna (non interroga TMDB: filtra le voci del pannello). Sotto, i
  filtri attivi diventano **pill rimovibili** con un clic, comprese le correnti.
- **Minimizza e riapri**: il pulsante «Filtri» nella barra di comando
  (scorciatoia Ctrl+B) chiude il pannello per lasciare piu' spazio ai poster; lo
  stesso pulsante lo riapre alla larghezza di prima, anche dopo un passaggio in
  Chat. Il badge sul pulsante continua a mostrare i filtri accesi, cosi' non si
  perdono di vista.
- **Sezioni**: Periodo, Genere, Paese e lingua, Mood e temi, Metriche. Genere parte
  aperto con i dodici generi piu usati («Tutti i generi» mostra gli altri); Paese
  mostra otto paesi («Tutti i paesi»). Una sezione si apre da sola se ha filtri
  accesi; la ricerca apre e isola le voci che combaciano. Lo stato di apertura dura
  quanto la sessione e non tocca il disco.
- **Periodo, genere, paese, mood/tema**: le chip spiegano sempre perche una faccia e
  bloccata o sconsigliata. Le correnti non hanno una lista di chip: si accendono
  dalla guida e restano nella testata, rimovibili con un clic.
- **Metriche a menu**: Voto minimo, Voto massimo, Voti minimi e Durata sono quattro
  chip che aprono un menu con i valori rapidi (voto alto ≥ 7, capolavori ≥ 8, sotto
  il 6, ecc.) e uno slider per la sfumatura. «Nessuno» spegne la metrica. Il voto e
  l'intervallo TMDB 0-10; i voti minimi usano una scala da 10 a 500.000; la durata e
  in minuti. Se ordini per voto senza soglia, contano solo i titoli con almeno 50
  voti.
- **Temi liberi**: arrivano dall'interprete AI o dalla casella «aggiungi un tema»
  (invio per confermare). Restano pill rimovibili come le altre facce; se TMDB non ha
  quella parola chiave, l'app lo dice nel riquadro *Perche* e prosegue.
- **Guida alle correnti** (pulsante «?» o link in fondo al pannello): si apre la
  finestra **Correnti del cinema**, con ricerca e l'elenco di tutte le correnti. Per
  il Neorealismo: anni 1940-1950, Italia, generi tipici Dramma/Guerra; per lo
  Slasher: anni '70-'80, USA, parola chiave TMDB `slasher`. «Applica» accende quei
  filtri (periodo, paese, parola chiave) e la corrente, che resta visibile nella
  testata: tutto e rimovibile. Cosi il motivo di ogni risultato resta sempre sotto i
  tuoi occhi.

## Collegamento AI e rilevamento dei modelli

In *Impostazioni → Interprete delle richieste* scegli il provider:

- **DeepSeek**: incolli la chiave API, il modello predefinito è `deepseek-flash`
  (V4.1 Flash) e lo sforzo di ragionamento è **Massimo**. L'ID
  `deepseek-v4.1-flash` non è valido sull'API diretta: l'alias corretto è
  `deepseek-flash`.
- **OpenRouter**: incolli la chiave e premi **Rileva modelli**; l'elenco mostra
  tutti i modelli pubblicati, con nome e contesto.
- **Ollama**: indichi il collegamento locale e il modello.

| Pulsante | Cosa fa |
|---|---|
| **Rileva modelli** | interroga `GET /models` (DeepSeek/OpenRouter) o `/api/tags` (Ollama) e riempie il menu con i modelli usabili: provider, contesto e costo relativo quando disponibile |
| **Verifica risposta AI** | manda al modello scelto una frase di prova e controlla che risponda in **JSON conforme**, mostrando le facce estratte |

All'avvio, se il provider è configurato, l'elenco dei modelli viene caricato da solo.
La chiamata AI parte **solo con Invio o «Cerca»** (non mentre digiti) e
l'interpretazione viene messa in cache: cambiare pagina o filtri non costa altre
chiamate. Le risposte AI e i dati TMDB (titoli, trame, note) sono trattati come
**dati non fidati** nei prompt: non possono dettare istruzioni al modello.

Per provare tutto da riga di comando:

```powershell
cargo run --example app_probe -- "film del neorealismo italiano di guerra con voti maggiori di 8"
```

## Motore di vincoli (`src/domain/constraints.rs`)

Le regole applicate, in ordine:

1. **R4 · temporalita**: `year_range()` interseca i decenni scelti con i periodi dei
   movimenti. Se l'intersezione e vuota il movimento viene rimosso con spiegazione.
2. **R3 · geografia**: i paesi del movimento diventano impliciti se non ne scegli
   nessuno; se ne scegli uno incompatibile viene rimosso.
3. **R2 · semantica del genere**: `genres_forbidden` di un movimento blocca la chip e
   finisce in `without_genres` della query TMDB.
4. **R5 · mood**: i mood incompatibili (es. *Soprannaturale* con Neorealismo) vengono
   rimossi.
5. **R6 · backtracking**: qualunque modifica rilancia la propagazione su tutte le
   facce, in entrambe le direzioni.
6. **R7 · traduzione della corrente**: su TMDB i generi di un movimento sono
   *etichette alternative*, non requisiti simultanei. Se il genere chiesto e gia una
   di quelle etichette (Mistero per il *Giallo Italiano*), la query usa l'unione in
   OR — `with_genres=27|53|9648` — invece dell'AND; un genere estraneo al movimento
   resta invece un filtro AND, perche aggiunge una condizione reale.
7. **R5 · validazione sui dati reali**: se i vincoli chiesti producono **zero**
   risultati, il motore allenta un vincolo alla volta e lo dichiara nel riquadro
   *Perche*: prima i filtri sul giudizio (voto minimo, voto massimo, numero di voti),
   poi i generi del movimento in OR, poi le parole chiave tematiche. Nessun
   rilassamento silenzioso.
8. **R8 · identita della corrente**: la geometria (paese + periodo + genere) ha
   molto richiamo ma anche rumore: *giallo italiano anni 80* su TMDB restituisce
   circa 600 titoli dove i gialli veri sono un terzo. Per le correnti che la
   comunita TMDB marca bene esiste una **parola chiave TMDB** (Slasher Era →
   `slasher`): la query la usa come identita della corrente, in alternativa agli
   altri argomenti; le altre correnti restano sulla geometria. La ricerca interroga
   sempre e solo TMDB, in tempo reale.

`pruning()` dice alla UI quali facce sono selezionabili e perche; `sanitize()` corregge
lo stato e produce i messaggi mostrati nel riquadro *Perche*; `to_discover()` traduce
lo stato in query TMDB; `app::discover_with_relaxation()` applica la scala di
rilassamento con le sue spiegazioni.

La tassonomia vive in `assets/taxonomy.json`: decenni, generi (id TMDB), paesi,
mood (keyword TMDB, risolte a runtime) e movimenti con periodo, paesi e compatibilita
di genere. Per aggiungere una corrente basta una voce nel JSON — nessun codice.

## Correnti: dalla guida a TMDB

Ogni corrente e una **combinazione di filtri TMDB**: periodo, paese e, dove la
comunita TMDB la cura bene, una **parola chiave** (Slasher Era → `slasher`; le
altre 16 correnti si riconoscono per periodo e paese). La finestra **Correnti del
cinema** mostra la combinazione di ognuna e con «Applica» la accende: i filtri
compaiono accesi nel pannello, la corrente resta in alto come pill rimovibile e la
ricerca interroga **sempre e solo TMDB, in tempo reale**.

Fino alla versione precedente esisteva in piu un *elenco canonico* per corrente,
costruito offline incrociando Wikidata e Wikipedia e usato per sostituire i
risultati di TMDB con un insieme verificato a mano. E stato rimosso: i risultati
arrivano sempre dalla ricerca live su TMDB, con i filtri che vedi.

## Voto IMDb (facoltativo)

Dai dati pubblici IMDb (`datasets.imdbws.com/title.ratings.tsv.gz`, ~7 MB compressi,
~29 MB scompattati, nessuna chiave e nessun limite) si scarica una volta sola
l'archivio dei voti. Il file e ordinato per identificativo **in ordine alfabetico**
(`tt10000000` viene prima di `tt9999999`, quindi un confronto per numero sbaglierebbe):
CineMatch costruisce un indice di salti ogni 2048 righe con la chiave di testo del
file e legge solo il blocco che serve, quindi **non tiene in memoria** i 29 MB di
dati. L'indice, se manca o e di un formato precedente, si ricostruisce da solo al
primo consulto. Nella scheda film il voto IMDb compare accanto a quello TMDB. Il
pulsante e in Impostazioni → *Dati locali*; senza download la funzione resta spenta
e invisibile.

## Architettura

```
src/
├─ main.rs            bootstrap eframe (glow di default, wgpu con CINEMATCH_RENDERER=wgpu)
├─ app.rs             stato, barra di comando, ricerca asincrona, pannelli
├─ update.rs          manifesto release, download setup, SHA-256, avvio installer
├─ domain/
│  ├─ taxonomy.rs     caricamento tassonomia (decenni, generi, paesi, mood, correnti)
│  ├─ model.rs        Facets, Movie, Details, ordinamento
│  ├─ constraints.rs  motore di vincoli (R1..R8)
│  └─ suggestions.rs  catalogo locale e rotazione degli spunti della casa
├─ tmdb/              client HTTP (throttle 75 ms, retry, cache in memoria)
├─ ai/                trait IntentProvider + parser a regole + LLM pluggable
│                     (chat, curatela dei film, rilevamento modelli, verifica)
├─ ui/                tema, facce, chat, griglia virtualizzata, scheda, immagini
└─ infra/             config, voti IMDb
installer/         Inno Setup per-utente
site/              pagina download statica (GitHub Pages)
```

La CI in `.github/workflows/` testa e compila l'app su Windows, crea il setup,
genera `latest.json` e pubblica release e pagina di download.

- **Griglia virtualizzata** (`ScrollArea::show_rows`): con 20 risultati o con 20.000
  si disegna solo cio che e visibile.
- **Renderer glow (OpenGL) per default**: misurato a riposo ~190 MB di memoria
  privata contro ~420 MB del backend wgpu/DX12 su Windows (il valore assoluto
  dipende dal driver grafico; il rapporto resta di circa due volte). Si cambia con
  `CINEMATCH_RENDERER=wgpu` se un driver preferisce Vulkan/DX12.
- **Poster**: memoria → disco (`%APPDATA%\CineMatch\cache`) → rete, decodifica in
  thread separati. Al riavvio i poster sono gia li.
- **Titoli non latini**: i film giapponesi e coreani arrivano con il titolo originale
  (リング, 呪怨) e i font di base di egui non coprono quei glifi: quando servono
  entrano come ripiego i font di sistema `Yu Gothic` e `Malgun Gothic`, caricati una
  volta sola. Una ricerca italiana normale non carica nulla.
- **Richieste annullabili**: un contatore di generazione scarta le risposte obsolete;
  il testo e debounced a 320 ms.
- **Throttle** sulle chiamate TMDB (75 ms tra richieste, ~13 req/s, sotto il limite di
  ~50 req/s per IP) con retry su 429/5xx.

## Installer e release

Il setup e in `installer/cinematch.iss` (Inno Setup 6); lo script di release
`.github/workflows/release.yml` compila e pubblica tutto su GitHub Releases.
Per una build locale, dopo aver installato Inno Setup 6:

```powershell
cargo build --release
ISCC.exe /DMyAppVersion=0.3.5 installer\cinematch.iss
```

Il risultato e `dist\CineMatch-Setup-0.3.5.exe`. Il manifesto
`dist\latest.json` viene generato dalla CI dopo la firma e contiene versione,
URL, dimensione e SHA-256 del setup.

Per pubblicare una nuova versione: aggiorna `version` in `Cargo.toml`, commit,
poi crea e spingi un tag `vX.Y.Z`. La CI esegue i test, compila l'exe, crea
l'installer, genera `latest.json` e pubblica la release. Il workflow Pages
pubblica `site/` su <https://emanueleparis98.github.io/cinematch/> quando cambia
la pagina. La pagina legge l'ultima release dall'API GitHub e aggiorna da sola
versione, dimensione e link di download.

L'updater e disattivato in debug, quando l'exe gira da `target` e quando e
impostata la variabile `CINEMATCH_NO_UPDATE=1`; `run.ps1` avvia una build di
sviluppo e non deve mai sostituirla.

## Test

```powershell
cargo test
```

I test in `tests/constraints_golden.rs` coprono i casi della discussione: anni '80 +
neorealismo bloccato, anni '80 + giallo ristretto al 1982, movimento scelto dopo il
decennio, Dogme 95 + Fantasy bloccato, mood soprannaturale rimosso, parsing della
frase composta, filtri su voto/voti, temi liberi in AND, ricetta delle combinazioni,
piu le regole delle correnti a parola chiave (keyword dichiarata in tassonomia,
risoluzione una volta sola, keyword in OR e tema in AND). I test in `src/ai/`
verificano le regole dell'interprete, comprese le negazioni («italiano di guerra» non
e un «senza guerra»), e la curatela della chat: id inventati scartati, doppioni,
ordine preservato, parsing tollerante e streaming parziale dell'intro. I test di
`domain::suggestions` verificano che ogni spunto passi il motore di vincoli, resti
riconoscibile dal parser a regole e non si ripeta alla rotazione successiva.

Gli strumenti di sviluppo sono tre: `app_probe` (frase → facce → TMDB, senza
finestra), `chat_probe` (messaggio → candidati → curatela, anche su piu turni)
e `imdb_probe` (controllo dell'indice dei voti IMDb).

## Attribuzione

Questo prodotto usa l'API TMDB ma non e approvato o certificato da TMDB.
Uso non commerciale secondo i termini TMDB.
