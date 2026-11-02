# CineMatch

Catalogo cinematografico desktop per Windows: filtri combinabili, tassonomia,
correnti e guida alla scoperta dei film. Versione 0.11.0.

Il catalogo è la schermata iniziale e il centro del prodotto. Ricerca chat,
provider AI, rilevamento dei modelli e relative impostazioni sono stati rimossi.
Non sono richiesti modelli locali o credenziali per servizi AI.

## Uso

1. Inserisci una credenziale TMDB: API Key v3 oppure Read Access Token v4.
2. Esplora i film con i filtri oppure cerca un titolo nella barra del catalogo.
3. Usa la guida alle correnti per leggere periodo, paesi e ricetta dei filtri.
4. Apri una scheda per trama, persone, durata, voti e disponibilità riportata.

La credenziale TMDB è conservata nel Windows Credential Manager. SIMKL è
facoltativo: consente di gestire visti, lista e voti del proprio account.

## Filtri e correnti

- Decenni, correnti, generi, paesi, mood e parole chiave TMDB.
- Inclusione dei generi ed esclusione tramite «Escludi generi…»; entrambe
  rimangono visibili nel riepilogo dei criteri attivi.
- Voto minimo e massimo TMDB, numero minimo di voti, durata e ordinamento.
- Ricerca nel pannello, criteri rimovibili e combinazioni incompatibili spiegate.
- «Sblocca» propone rinunce da scegliere, senza allargamenti silenziosi.

Le correnti sono percorsi di esplorazione definiti dalla tassonomia: periodo,
paesi, generi ed eventuale keyword. Non sono un censimento certificato
dell'appartenenza di ogni opera a un movimento. La guida mostra la combinazione.

I generi espliciti sono in AND; i paesi sono alternative. I temi restano vincoli,
senza troncamento arbitrario a tre. Una keyword non risolta viene segnalata e
impedisce di presentare una ricerca più larga come equivalente a quella chiesta.

## Risultati e pagine

La ricerca testuale cerca titoli originali, tradotti e alternativi. Con filtri
attivi, le schede della pagina corrente vengono verificate e ordinate localmente.
Le pagine si riferiscono alla ricerca TMDB originaria: una pagina vuota può
essere seguita da una pagina con corrispondenze.

Quando il totale non rappresenta i film filtrati, la barra indica il numero
**in pagina**. Questo vale anche per decenni non contigui e visti nascosti.
La navigazione rispetta il limite operativo delle prime 500 pagine.

Una nuova ricerca annulla quella precedente; risultati obsoleti non
sovrascrivono la selezione attuale. Il caricamento ha uno stato esplicito.

Le soglie applicate dagli ordinamenti sono indicate nei filtri. «Più recenti»
usa normalmente almeno 50 voti e 60 minuti quando non sono stati scelti valori
espliciti. «Prossimamente» seleziona date future e ordina per popolarità.
Il voto massimo esclude i titoli privi di valutazioni.

## Dati e integrazioni

- **TMDB:** catalogo, immagini e schede; richiede connessione. L'API è gratuita
  per uso non commerciale secondo le condizioni e attribuzioni della fonte.
- **SIMKL:** collegamento tramite codice, sincronizzazione e gestione di visti,
  lista e voti. Le scritture avvengono su azione dell'utente.
- **IMDb:** dataset non commerciale opzionale scaricato localmente per mostrare
  i voti; non è una lettura in tempo reale del sito IMDb.
- **Aggiornamenti:** manifesto GitHub e verifica del setup scaricato.

Senza rete non sono garantiti dati o immagini assenti dalle cache. Il catalogo
non richiede l'archivio narrativo ipotizzato nei piani precedenti, ora storici.

## Scorciatoie

| Comando | Azione |
| --- | --- |
| `/` oppure `Ctrl+K` | Fuoco nella ricerca del catalogo |
| `Ctrl+B` | Mostra o nasconde i filtri |
| `Ctrl+S` nelle impostazioni | Salva le modifiche |
| `Esc` | Chiude prima guida, impostazioni o scheda attiva |

Dimensione minima: 800×600. Filtri e dettagli usano pannelli alternativi per
lasciare spazio alla griglia.

## Sviluppo

```powershell
cargo test --locked --offline --all-targets
cargo clippy --locked --offline --all-targets -- -D warnings
cargo build --locked --offline --release
cargo check --locked --offline --features wgpu
```

La build predefinita usa OpenGL; wgpu è opzionale. Installer con Inno Setup 6:

```powershell
ISCC.exe /DMyAppVersion=0.11.0 installer\cinematch.iss
```

`/NOLAUNCH=1` installa senza avviare automaticamente l'app.
I test automatici usano dati controllati e non chiamano provider AI.

## Attribuzione

This product uses the TMDB API but is not endorsed or certified by TMDB.
Uso non commerciale secondo i termini TMDB. I dataset IMDb sono soggetti alle
condizioni per uso personale e non commerciale.
