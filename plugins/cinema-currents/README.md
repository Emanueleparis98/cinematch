# Cinema Currents 1.0

Plugin locale e indipendente per consultare le correnti cinematografiche e generare
prompt utilizzabili in qualsiasi chat. CineMatch Terminal usa il catalogo incluso
in questa cartella. Il plugin non chiama servizi AI, non invia messaggi, non legge account e non salva
richieste o risposte. La raccolta di riferimento è incorporata nel pacchetto.

## Uso autonomo

Il pacchetto Windows contiene `cinema-currents.exe`. Puoi copiarlo in un'altra
cartella: non richiede CineMatch, chiavi API o un'installazione. Per altri sistemi
compila i sorgenti Rust di questa cartella con `cargo build --release`.

```powershell
.\cinema-currents.exe catalog
Get-Content request.example.json -Raw | .\cinema-currents.exe generate
.\cinema-currents.exe --version
```

`catalog` restituisce il catalogo JSON con ID, nomi, sinonimi, categorie, periodi,
aree, caratteristiche, figure, opere, note e fonti. `generate` legge un singolo
oggetto JSON da stdin e restituisce un oggetto JSON su stdout. Gli errori sono
su stderr e producono un codice di uscita diverso da zero. Limite dell'input CLI:
1 MiB, per proteggere la lettura del processo; nessun limite fisso di cinque film.

## Contratto della richiesta, versione 1

```json
{
  "schema_version": 1,
  "movements": ["neorealismo"],
  "mode": "recommendations",
  "count": 5,
  "depth": "concise",
  "relation": "historical",
  "preferences": "Un percorso accessibile da guardare in coppia",
  "exclusions": "Niente spoiler importanti",
  "watched": ["Ladri di biciclette"],
  "question": "Alterna opere note e scoperte meno conosciute."
}
```

- `movements`: almeno un ID di `catalog`. Nel confronto servono due ID distinti.
- `mode`: `recommendations`, `introduction`, `discoveries`, `compare`, `free`.
- `count`: intero positivo oppure `null` per quantità automatica. Riguarda il
  totale dei film, anche in un confronto fra più correnti.
- `depth`: `concise` oppure `detailed`.
- `relation`: `historical` oppure `legacy`, per includere influenze successive.
- `question`: obbligatoria nella modalità `free`; negli altri casi facoltativa.
- `preferences`, `exclusions`, `watched`: criteri personali, incorporati nel testo
  senza risoluzioni di rete o eliminazioni silenziose.

I campi non riconosciuti vengono rifiutati. La risposta contiene `schema_version`,
`prompt` e `warnings`. La generazione è deterministica; il testo può essere
modificato prima di incollarlo. I modelli dei cinque obiettivi sono nel sorgente
`src/lib.rs`; i contenuti storici sono in `assets/movements.json`.

## Uso come libreria

Aggiungi `cinema-currents = { path = ".../cinema-currents" }` alle dipendenze Rust.
Le funzioni pubbliche sono `catalog()` e `generate(&PromptRequest)`. Altre
applicazioni possono usare il protocollo CLI senza dipendere da Rust.

La raccolta comprende movimenti, tendenze, generi/cicli, tradizioni e tecniche:
queste categorie restano distinte. Date e attribuzioni sono indicative; le opere
sono esempi orientativi, non un censimento di tutte le appartenenze. Le fonti
sono consultabili, con istituzioni cinematografiche e riferimenti secondari
identificati come tali. La correttezza della risposta finale dipende dalla chat
esterna: il plugin prepara la richiesta e non certifica l'output dell'AI.
