# CineMatch 3.9.0 — Modalità CB01

La 3.9.0 aggiunge una modalità opzionale per `cb01one.org`: blocca la
pubblicità, aggiunge un pulsante **Guarda con CineMatch** sui poster di film e
serie TV e riproduce la sorgente disponibile nel player CineMatch, senza cambiare
pagina e senza caricare il player pubblicitario del sito.

## Attivazione

1. Apri **Opzioni → Modalità CB01**.
2. Attiva l’opzione.
3. Concedi una volta l’accesso a CB01 e ai suoi player.
4. Apri o ricarica una scheda di `cb01one.org`.

L’accesso è **opzionale**: non viene richiesto durante l’aggiornamento e non
serve per le immagini nelle chat o per il pulsante su TMDB. Disattivando la
modalità, CineMatch rimuove i pulsanti, deregistra i content script, disattiva
le regole di rete e revoca i permessi CB01.

## Cosa succede sulla pagina

### Homepage, categorie e ricerca

- Ogni locandina di film o serie riceve un discreto badge **Play**.
- Un click sull’immagine apre subito l’overlay CineMatch e avvia la ricerca della
  sorgente; la pagina CB01 resta dietro.
- Ctrl/Cmd/Shift-click e click centrale continuano ad aprire la scheda CB01.
- I poster delle persone, i backdrop e le immagini non collegate a un titolo non
  ricevono il pulsante.

### Scheda film

- La locandina principale diventa cliccabile e mostra **Guarda con CineMatch**.
- Il blocco con gli iframe del player originale viene nascosto e sostituito da
  una card pulita con **Guarda ora** e **Player originale**.
- Gli iframe originali vengono portati a `about:blank` finché non vengono
  ripristinati manualmente, così non caricano annunci in sottofondo.
- I link “Streaming” e “Streaming HD” vengono intercettati: un click apre il
  player CineMatch invece di una scheda pubblicitaria.
- I link “Download” restano esclusi.

### Scheda serie TV

- La locandina apre il selettore **stagione + episodio**.
- Ogni riga episodio riceve un pulsante Play; anche i link Maxstream/Mixdrop
  della riga vengono intercettati.
- La stagione predefinita è la più recente disponibile; si può cambiare dal
  selettore e tornare alla puntata scelta con **← Episodi**.
- Ogni episodio conserva un punto di ripresa separato nella sessione.

## Come viene scelta la sorgente

CineMatch legge i collegamenti già presenti nella scheda e prova i provider in
ordine di qualità e affidabilità:

1. Mixdrop HD (StayOnline → Mixdrop → MP4 diretto).
2. Mixdrop SD.
3. Maxstream HD/SD, solo se non richiede una verifica.

Il primo flusso valido parte automaticamente. Se il provider richiede un
CAPTCHA a immagini, CineMatch **non lo aggira**: passa alla sorgente successiva
e, se nessuna è automatica, spiega il motivo e offre il fallback manuale.
I link Mixdrop contengono un token temporaneo: vengono risolti di nuovo a ogni
click e non vengono salvati su disco.

## Pubblicità

La modalità usa `declarativeNetRequest` per bloccare le reti pubblicitarie e i
tracker osservati su CB01 e sui suoi player, più filtri cosmetici per gli spazi
residui. Una guardia nel mondo MAIN impedisce i popunder e i click verso domini
pubblicitari.

Sono bloccati, tra gli altri: `adsco.re`, `cdn4ads.com`, `rapidadexchange.com`,
`acscdn.com`, ShareThis, Histats, Polldaddy/Crowdsignal, DoubleClick,
`usrpubtrck.com`, `vidwestxyz.com`, `soakysecrets.com`, `mdstats.info`.
Cloudflare Turnstile, gli asset del sito, i font, i trailer YouTube e i
collegamenti sociali legittimi restano disponibili.

Il player CineMatch riproduce il file diretto: non ci sono annunci, popup o
“clicca per continuare”. Il trailer YouTube può comunque mostrare gli annunci
propri di YouTube, fuori dal controllo di CB01.

Nessun adblock è garantito al 100%. Le regole vengono aggiornate con le release
quando cambiano domini o provider.

## Permessi e privacy

Origini richieste solo all’attivazione:

- `https://cb01one.org/*` e `https://www.cb01one.org/*`
- `https://stayonline.pro/*`
- `https://uprot.net/*`
- `https://*.mxdrop.click/*`
- `https://*.m1xdrop.net/*`
- `https://*.mixdrop.co/*`
- `https://*.mixdrop.ag/*`
- `https://*.mxdrop.to/*`
- `https://*.mixdrop.to/*`
- `https://*.mxcontent.net/*`

Il permesso tecnico `declarativeNetRequest` serve solo per le regole di rete e
non concede accesso ai siti. Le richieste usano HTTPS, `credentials: “omit”`,
niente cookie o account, `referrerPolicy: “no-referrer”`, timeout e limiti di
dimensione. CineMatch non esegue il JavaScript remoto: il pacchetto Mixdrop
viene decodificato con una funzione pura. I token di riproduzione restano nella
sessione del browser e scadono.

I link ai video non vengono inseriti nella cronologia, nei prompt o nelle chat.
CineMatch non ospita né distribuisce video: automatizza soltanto l’accesso del
browser a un sito di terzi. Usa la modalità solo per contenuti che hai il
diritto di guardare e rispetta i termini del sito.

## Limiti noti

- Maxstream/UpRot può richiedere un CAPTCHA: non viene aggirato.
- Mixdrop può cambiare dominio, token o formato della pagina; la risoluzione
  viene aggiornata con le release.
- Il sito può cambiare selettori o introdurre nuovi anti-adblock.
- Alcuni film usano solo provider non automatici; in quel caso resta il player
  originale o la scheda CB01.
- I video non vengono scaricati: la riproduzione avviene in streaming a blocchi.

## Verifica

- **86 test Node**, inclusi parsing dei link film/serie, ordinamento HD/SD,
  normalizzazione HTTPS, risposta StayOnline, decoder del packer Mixdrop e
  rifiuto degli host non autorizzati.
- **7 controlli browser CB01**: toggle opzionale, registrazione script e regole,
  `testMatchOutcome`, click su card film, scheda film, player in scheda, picker
  serie con cambio episodio e disattivazione.
- **Regressione completa**: 29 controlli browser del pannello, 10 launcher,
  5 TMDB/TV, 23 immagini, 36 player, 8 codec, 4 updater e benchmark throughput.

La suite usa pagine fixture e un server video locale. La verifica manuale su
CB01 reale controlla poster, blocco annunci e riproduzione Mixdrop. Il test
automatico usa una copia dell’estensione con i permessi CB01 pre-concessi, perché
il prompt nativo di Chrome non è automatizzabile in headless.
