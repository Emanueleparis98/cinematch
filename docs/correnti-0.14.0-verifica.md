# Verifica CineMatch 0.14.0 / Cinema Currents 1.0

Consegna della nuova guida alle correnti e del plugin locale, richiesta e approvata
il 2 novembre 2026. Il catalogo generale e le integrazioni API delle versioni
precedenti sono stati ritirati dall'app. I file personali preesistenti sono conservati.

## Implementazione

- 74 voci tipizzate: 44 movimenti, 14 tendenze, 12 generi/cicli, 3 tradizioni,
  1 tecnica/ambito produttivo. Tutte le 26 voci precedenti sono riconciliate per ID;
  48 aggiunte coprono origini, avanguardie, documentario, nuove onde e percorsi globali.
- Periodi indicativi, nomi originali o internazionali, sinonimi, figure, opere e
  fonti. Le date finali artificiali di Wuxia, Kaiju e Tokusatsu sono rimosse.
- Rimosse le liste di generi vietati e i filtri geografici usati come sostituti
  dell'appartenenza storica. Due riferimenti secondari di Wikipedia sono dichiarati.
- Guida con ricerca per nomi, sinonimi, figure e opere, categorie, epoche, aree e
  ordinamento cronologico o alfabetico. Scheda e compositore condividono la selezione.
- Sotto 1000 pixel logici la navigazione passa a viste a larghezza piena.
- Cinque obiettivi di prompt, quantità automatica o positiva esplicita, livello
  di dettaglio, appartenenza storica o influenze, criteri personali e titoli da evitare.
- Modifica manuale dell'output; copia disabilitata se i criteri cambiano senza
  rigenerare. Gli errori richiedono correzioni esplicite, senza rimuovere criteri.
- Scelta della chat: collegamenti HTTPS senza prompt, credenziali o parametri.
  Copia, apertura del sito e invio sono passaggi distinti controllati dall'utente.
- Nessun client HTTP, provider AI, accesso agli account o acquisizione di film.
  La raccolta testuale pesa circa 86 KB nei sorgenti; richieste e prompt restano in RAM.

## Plugin riutilizzabile

`plugins/cinema-currents` è una libreria Rust autonoma con dipendenze serde e
serde_json. Il binario `cinema-currents` espone `catalog`, `generate` e `--version`.
Il protocollo JSON versione 1 è documentato con esempio. Non richiede CineMatch.
Il pacchetto ZIP include eseguibile Windows, documentazione, esempio e sorgenti.
L'installer comprende anche il plugin nella cartella `plugins` del programma.

## Verifiche automatiche e native

Test del generatore su quantità 1, 5, 12 e 100, quantità automatica, criteri,
titoli da evitare, confronto, richieste libere, contesto sintetico e richieste errate.
Controlli su unicità, completezza strutturale, fonti HTTPS, continuità delle voci
e correzioni storiche. Il protocollo è provato avviando il binario da un altro processo.

Rendering egui a 800×600, 1024×768 e 1440×900 per scheda, elenco, compositore,
prompt generato e ricerca vuota. Verificati gli stati di copia superata e gli URL.
La prova nativa della guida è stata eseguita a 1220×660 con font e contenuti reali.

Il controllo Impeccable sui file UI modificati non ha restituito rilievi.
La generazione non è un test della qualità delle risposte delle AI esterne:
non vengono eseguite chiamate API o inviati messaggi per questo collaudo.

## Limiti dichiarati

Il repertorio resta editoriale e ampliabile. Una fonte di approfondimento non
certifica automaticamente ogni autore o esempio citato; i casi controversi sono
segnalati e le attribuzioni richiedono contesto. La ricerca testuale usa corrispondenze
normalizzate e sinonimi curati, senza interpretazione semantica AI.

Il plugin non installa estensioni all'interno di ChatGPT o DeepSeek: produce testo
riutilizzabile in qualsiasi chat. Account, disponibilità e limiti dei siti esterni
restano quelli dei relativi servizi. Il protocollo CLI limita l'input a 1 MiB.

## Esito della consegna

- 18 test superati: 9 del generatore e della raccolta, 3 del protocollo CLI,
  6 dell'app e dei servizi locali, compreso il rendering dei cinque stati alle tre dimensioni.
- Clippy su workspace e tutti i target senza warning; backend wgpu compilato.
- Build release di app e plugin riuscite. L'installer 0.14.0 è installato sul PC;
  binari installati uguali alla build verificata e file personali preservati con SHA-256.
- ZIP provato dopo estrazione separata: 74 voci, raccolta aggiornata, generazione
  riuscita e nessun nuovo file di dati creato dal plugin.
- Installer circa 4,62 MiB; ZIP del plugin circa 0,28 MiB. Ripuliti 3,7 GiB di
  artefatti di compilazione dopo il collaudo. La cartella temporanea dell'estrazione
  del test ZIP resta in TEMP: il controllo automatico ne ha bloccato la cancellazione.

SHA-256 installer: `DCCA5ED536E969845134F59B724C1C0B32D25956D55762E6889FE55EDD64D0C8`.
SHA-256 ZIP: `8337C19BB1469E73253B630CD8B08DD46B00E5BE74D44372FAB32E67186B0923`.
