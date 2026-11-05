# CineMatch - estensione browser 3.11.0

74 voci sulle correnti del cinema, richiesta libera senza corrente, generatore
locale di richieste, immagini automatiche nelle risposte di ChatGPT, DeepSeek,
Claude e Gemini, pulsante **Flussi** su TMDB, pulsante **Correnti** in DeepSeek
e modalità **CB01** senza pubblicità per film e serie TV.

## Installazione

1. Estrai completamente `CineMatch-Chrome-3.11.0.zip` in una cartella stabile.
2. Apri `chrome://extensions`, attiva **Modalità sviluppatore** e premi
   **Carica estensione non pacchettizzata**.
3. Seleziona la cartella con `manifest.json`. Fissa CineMatch alla barra dal menu
   Estensioni: l’icona apre le **Impostazioni**, da cui puoi attivare il pulsante
   Correnti in DeepSeek.

Da una versione **3.6.0 o successiva** aggiorna da **Opzioni → Aggiornamenti**
nella stessa cartella, senza cambiare ID né perdere TMDB e addon. Per una
versione precedente alla 3.6.0 sostituisci invece i file della cartella già
caricata e premi **Ricarica**.

Nel repository la cartella già pronta è `dist/CineMatch-Chrome` dopo la build.
Il sorgente `extensions/cinematch` non include i dati assemblati. Mantieni la
cartella caricata nella stessa posizione. Chrome 116+; in Edge con Chromium
recente usa `edge://extensions`. Firefox e Safari non sono supportati da questo
pacchetto.
Distribuzione manuale, non pubblicata sullo store.
Guida Chrome: https://developer.chrome.com/docs/extensions/get-started/tutorial/hello-world.

## Esplorare e preparare una richiesta

L'estensione usa solo il tema scuro: pannello, connessione TMDB, campi e menu.
Le spunte sono disegnate; i menu di scelta sono liste personalizzate con ricerca
digitando, frecce, Home/End, Invio, Esc e Tab. Si aprono sopra o sotto il campo
in base allo spazio, senza essere tagliati dal bordo del pannello.

In **Esplora**, cerca per corrente, sinonimo, autore o opera. I filtri per
categoria, periodo e area sono espandibili. Apri una voce per caratteristiche,
contesto, esempi, limiti interpretativi e fonti. Le caselle permettono fino a
quattro selezioni; **Prepara per questa voce** prepara una sola selezione.
Il collegamento **Richiesta libera** porta direttamente al composer senza
obbligare a scegliere una corrente.

In **Componi** scegli tra **Con una corrente** e **Richiesta libera**. La modalità
libera è pronta senza selezioni: scrivi la richiesta, aggiungi gli esempi o i
dettagli e genera. Nella modalità con corrente, le voci scelte restano salvate
anche passando alla modalità libera, ma vengono ignorate finché non torni a
**Con una corrente**. Le impostazioni di approfondimento, relazione, preferenze,
esclusioni e già visti sono raccolte sotto **Dettagli**.
Genera e rileggi il prompt, poi usa **Inserisci nella chat** o **Copia**. L'invio
resta all'utente. Una bozza esistente richiede la scelta di aggiungere o
sostituire; non viene sovrascritta in silenzio. Ctrl+F cerca nelle correnti;
Ctrl+Invio genera la richiesta. L'icona in alto apre una vista ampia in una
scheda del browser.

## Immagini come parte della risposta

Collega TMDB una sola volta dal pulsante **TMDB** nell'intestazione. Sono accettate
una chiave API v3 oppure un API Read Access Token. La connessione già salvata
nelle versioni precedenti viene mantenuta: non serve reinserire la credenziale.

Locandine e foto di registi/interpreti vengono aggiunte automaticamente subito
dopo la riga del titolo, prima della descrizione. Questo vale anche per titoli
in grassetto e descrizioni nello stesso paragrafo, separati da un ritorno a capo.
Le immagini seguono tipografia e colori della pagina della chat. Il pannello e
le opzioni usano il tema scuro dell’estensione; il player in overlay segue il
tema della pagina ospite (chat o TMDB).
Non vengono inseriti pannelli, titoli duplicati, impostazioni, moduli, selettori,
riquadri di errore o richieste di correzione. Ogni locandina mostra un discreto
pulsante **Flussi**; anche la locandina apre il player CineMatch. Foto delle
persone e attribuzione aprono TMDB. Il testo dell’AI rimane invariato.

Le risposte che indicano esplicitamente una **serie TV** ricevono la locandina
della serie e lo stesso pulsante. Il player chiede stagione ed episodio e cerca
le sorgenti con il protocollo `series`. In caso di ambiguità non viene inserita
alcuna immagine.

## Pulsante Flussi su TMDB

Dalla 3.8.0, da **Opzioni → Pulsante flussi su TMDB**, puoi mostrare il pulsante
sui poster di themoviedb.org. L’accesso al sito è un permesso opzionale:
CineMatch legge solo titolo, ID e immagine del poster per aprire il player; non
legge account, cookie o conversazioni. Disattivando l’opzione i pulsanti vengono
rimossi e il permesso revocato. I poster di film e serie TV, compresa la locandina
principale, aprono lo stesso selettore sorgenti dell’estensione.

## Correnti in DeepSeek

Dalla 3.11.0 non esiste più il pannello laterale e il vecchio pulsante
flottante. L’icona dell’estensione apre le **Impostazioni**. Attiva
**Correnti in DeepSeek** per far comparire un pulsante **Correnti** nel composer
di `chat.deepseek.com`, accanto a **Pensiero Profondo** e **Ricerca**.

Il pulsante usa lo stile nativo di DeepSeek e apre un piccolo popover ancorato al
composer: ricerca, elenco compatto delle **74 correnti**, fino a quattro
selezioni, **Richiesta libera**, opzioni richiudibili e anteprima del prompt.
Puoi copiare il prompt o inserirlo direttamente nella chat; l’invio resta
all’utente. Non è un pannello laterale e non si apre da solo. Le altre chat non
mostrano il pulsante Correnti e non hanno più il pulsante flottante; conservano
immagini TMDB, **Flussi** e player.

## Modalità CB01

Dalla 3.9.0, da **Opzioni → Modalità CB01**, puoi aprire `cb01one.org` con la
pubblicità bloccata e un pulsante **Guarda con CineMatch** sui poster di film e
serie TV. Un click sulla locandina apre l’overlay CineMatch, risolve la sorgente
Mixdrop disponibile e la riproduce senza cambiare pagina. Nella scheda film il
blocco player originale viene sostituito da una card pulita; per le serie puoi
scegliere stagione ed episodio. Le sorgenti Maxstream che richiedono un CAPTCHA
non vengono aggirate: CineMatch prova la sorgente successiva e, se necessario,
mostra un messaggio chiaro.

La modalità blocca le reti pubblicitarie e i tracker osservati con regole di
rete e filtri cosmetici, impedisce i popunder e non carica gli iframe player
originali nella pagina. Il player CineMatch riproduce il file diretto, quindi
non mostra annunci. L’accesso a CB01, StayOnline, UpRot, Mixdrop e al CDN video
è un permesso opzionale richiesto una sola volta; l’aggiornamento non lo impone.
CineMatch legge solo i collegamenti visibili della scheda, non account, cookie o
conversazioni. I token di riproduzione restano nella sessione e scadono.

Emoji, numerazioni e sottotitoli vengono ripuliti per la ricerca: ad esempio
`🌊 2. Flow - Un mondo da salvare (2024)`. Quando necessario vengono consultati
i titoli alternativi verificati di TMDB. Un semplice risultato simile non basta:
titolo e anno devono corrispondere senza ambiguità; un regista esplicito deve
coincidere. Se non c'è sufficiente certezza, la risposta rimane senza immagini.
Le foto assenti vengono omesse, senza icone o box che interrompano la lettura.

La modalità è sempre automatica; i vecchi flag di disattivazione sono ignorati.
Nessun limite di cinque film o di dodici anteprime. Le immagini si caricano
vicino all'area visibile e le altre arrivano scorrendo. I ritratti su schermi
stretti scorrono orizzontalmente. Per un film vengono mostrati regia e fino a
quattro interpreti con foto, senza un pannello di gestione del cast.

## Addon e player

L’icona video **Addon e player** nell’intestazione apre la nuova sezione.
Collega un manifest compatibile con Stremio/Nuvio, poi clicca la locandina di un
film nella risposta AI. Scegli il flusso nella scheda CineMatch per riprodurlo.
Puoi importare file o indirizzi JSON anche senza addon. Video HTTP, HLS e DASH
sono integrati. libmedia aggiunge decoder locali per MKV/AVI, HEVC, H.266/VVC
in MP4 e audio AC3/E-AC3/DTS, usati automaticamente quando servono. Torrent,
DRM e header senza proxy conservano i limiti dei rispettivi servizi.

I sette configuratori indicati sono disponibili in una sezione espandibile:
completa la configurazione sui loro siti e incolla il manifest personale.
Nessun addon è preinstallato. [Guida e collaudo](../../docs/estensione-3.0.0-player.md).

La versione 3.1.0 legge e decodifica i blocchi durante la visione: non converte
e salva prima il film completo. Per il motore integrato sono disponibili pausa,
avanzamento, volume, schermo intero e selezione delle tracce audio/sottotitoli
incorporate. I codec e il motore sono nel pacchetto, senza CDN a runtime.
[Compatibilità e verifica](../../docs/estensione-3.1.0-decoder.md).
Dalla 3.1.2 i file vengono letti in blocchi fino a 8 MiB con un blocco successivo
in lettura anticipata: fino a 16 MiB di cache I/O in RAM, oltre ai buffer del
motore e dei decoder. Il CDN risolto viene riusato nella sessione e il link
dell’addon viene risolto di nuovo se quello del CDN scade. Il precaricamento
del motore passa da 3 a 6 secondi. Nessun file video viene archiviato.
[Collaudo delle prestazioni](../../docs/estensione-3.1.2-buffer.md).
La 3.1.3 corregge il rifiuto delle risposte CDN più ampie della finestra richiesta:
legge soltanto il blocco necessario e cancella i dati residui.
[Correzione Range](../../docs/estensione-3.1.3-range.md).
La 3.1.4 richiede dal clic di riproduzione il permesso opzionale per i server
di download Real-Debrid delle sorgenti RD. Accetta il dialogo Chrome; se lo
rifiuti, il player mostra **Consenti Real-Debrid** per riprovare. Il consenso
copre i server variabili del servizio, senza richiederlo per ogni hostname.
[Accesso e recupero CORS](../../docs/estensione-3.1.4-cors.md).
La 3.1.5 limita anche le richieste dopo un seek alla fine fisica del file e
gestisce i server che annunciano intervalli oltre EOF: corregge l’errore
«intervallo video non valido» durante l’apertura di MKV grandi.
[Prova prima/dopo su server reale](../../docs/estensione-3.1.5-eof.md).
La 3.1.6 corregge l’avvio dei flussi pesanti e lenti: il riconoscimento del
contenitore attende più a lungo, un MKV senza estensione non viene più passato
al player nativo, i blocchi in timeout vengono ritentati con una finestra più
piccola e una risposta 429/5xx viene riprovata una volta. Il limite di avvio dei
decoder integrati passa da 25 a 90 secondi; se un remux contiene una traccia
TrueHD non decodificabile ma anche AC3/E-AC3, viene scelta quella compatibile.
[Correzione avvio, 3.1.6](../../docs/estensione-3.1.6-avvio.md).
La 3.2.0 aggiunge il decoder H.266/VVC per i flussi MP4, instrada TS/M2TS/FLV/
WMV/ASF/VOB/MPG/3GP ai decoder integrati, attiva il worker di hls.js e
`preload="auto"` per il percorso nativo. Se il decoder hardware rifiuta un
flusso, il player riprova una volta con i decoder software; gli errori di rete
non attivano questa riprova. I remux con TrueHD senza una traccia AC3/E-AC3
compatibile restano senza audio e lo indicano nello stato.
[Codec e accelerazione, 3.2.0](../../docs/estensione-3.2.0-codec.md).

## Costi, dati e permessi

La consultazione della guida e la generazione sono locali. Nessuna API AI,
telemetria, invio automatico o archivio progressivo di film. Servono Internet
per immagini/fonti/chat e gli account eventualmente richiesti dai siti.
Le condizioni e i limiti delle chat restano quelli dei rispettivi servizi.

TMDB è gratuito per uso non commerciale con attribuzione, soggetto ai suoi
limiti: https://developer.themoviedb.org/docs/faq. Logo ufficiale e dichiarazione
sono nei crediti della pagina di connessione. Le immagini sono caricate al momento,
non incluse nello ZIP. Font Manrope e relativa licenza sono locali.

Accesso iniziale a quattro siti AI (`chatgpt.com`, `chat.deepseek.com`,
`claude.ai`, `gemini.google.com`) e ai due host TMDB (`api.themoviedb.org`,
`image.tmdb.org`). I siti di addon e sorgenti video richiedono autorizzazioni
opzionali per singolo host, soltanto quando li colleghi o avvii un flusso.
Anche il pulsante su themoviedb.org e la modalità CB01 sono opzionali e vengono
concessi solo quando li attivi; non sono richiesti dall’aggiornamento. La
modalità CB01 richiede un permesso tecnico `declarativeNetRequest` per applicare
i filtri di rete, senza accesso generale ai siti. Nessuna autorizzazione generale
viene concessa in fase di installazione.
Non richiede accesso a cookie o cronologia.
Le risposte AI vengono analizzate localmente; TMDB riceve titoli, anni o nomi,
richieste di schede e immagini, non il testo completo della conversazione.
I messaggi dell'utente restano esclusi dai selettori delle risposte.

`scripting` inserisce il prompt nei quattro editor, `clipboardWrite` copia il
testo, `storage` conserva le bozze nella sessione. L’icona dell’estensione apre
le Impostazioni; il pannello Correnti è un overlay caricato solo su DeepSeek.
`webRequest` osserva temporaneamente soltanto i reindirizzamenti e gli errori
della richiesta video corrente, sui siti già autorizzati. Se un flusso passa
a un altro server, **Consenti [server]** richiede il relativo permesso opzionale
e riparte dalla sorgente iniziale. I link firmati restano in memoria e non
vengono mostrati o salvati. Non intercetta le conversazioni o la navigazione.
`declarativeNetRequest` applica le regole pubblicitarie della modalità CB01 solo
quando la modalità è attiva; le regole sono limitate a CB01 e ai suoi player.
La credenziale può essere ricordata in `storage.local` scegliendo **Ricorda
su questo dispositivo**, oppure in `storage.session` fino al riavvio.
Entrambi gli storage hanno accesso TRUSTED_CONTEXTS: gli script inseriti nelle
pagine non possono leggere la chiave. Nessun uso di `storage.sync`. La chiave
personale non viene inclusa nei sorgenti, nel pacchetto o negli URL delle chat.
Anche i manifest personali degli addon restano in storage.local, accessibile
soltanto alle pagine dell’estensione. Le liste importate e i flussi non vengono salvati.

Richieste concorrenti limitate a tre, timeout e al massimo una riprova automatica
per guasti transitori. Nessun retry per ambiguità o chiave non valida. Le schede
hanno una cache in memoria di cinque minuti, massimo 100 richieste. Le immagini
condivise nella scheda hanno una cache di massimo 64 elementi e sei milioni di
caratteri, mai su disco. Immagini e API usano no-store. Un riferimento deve
stabilizzarsi per circa 650 ms; lo streaming non ripete le ricerche già concluse.

## Verifica e aggiornamento

Test con Chromium isolato, veri script/API dell'estensione e pagine controllate
dei quattro provider; inclusi anche TMDB e immagini reali, con Flow. Nessun account
AI personale o invio di messaggi è stato usato. I siti possono cambiare DOM e non
ogni formato di risposta è riconoscibile; in questi casi il testo resta leggibile.
Edge non eseguito direttamente. L'inserimento diretto del prompt accetta fino a
200.000 caratteri; restano i limiti propri della chat.

Dalla 3.6.0 usa **Opzioni → Aggiornamenti** per scaricare, verificare con SHA-256
e applicare la nuova versione nella stessa cartella. Per una versione precedente
alla 3.6.0 sostituisci i file della cartella già caricata e premi **Ricarica** in
chrome://extensions. La credenziale e gli addon già salvati vengono mantenuti.
Per rimuovere usa **Rimuovi** nella pagina delle estensioni; la credenziale può
essere rimossa separatamente dalla pagina di connessione. L'app Windows precedente
non viene modificata o richiesta.
