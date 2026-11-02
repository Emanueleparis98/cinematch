# CineMatch — estensione browser 3.0.0

74 voci sulle correnti del cinema, generatore locale di richieste e immagini
automatiche nelle risposte di ChatGPT, DeepSeek, Claude e Gemini.

## Installazione

1. Estrai completamente `CineMatch-Chrome-3.0.0.zip` in una cartella stabile.
2. Apri `chrome://extensions`, attiva **Modalità sviluppatore** e premi
   **Carica estensione non pacchettizzata**.
3. Seleziona la cartella con `manifest.json`. Fissa CineMatch alla barra dal menu
   Estensioni, poi premi l'icona per aprire il pannello.

Nel repository la cartella già pronta è `dist/CineMatch-Chrome` dopo la build.
Il sorgente `extensions/cinematch` non include i dati assemblati. Mantieni la
cartella caricata nella stessa posizione. Chrome 116+; in Edge con sidePanel usa
`edge://extensions`. Firefox e Safari non sono supportati da questo pacchetto.
Distribuzione manuale, non pubblicata sullo store e senza aggiornamento automatico.
Guida Chrome: https://developer.chrome.com/docs/extensions/get-started/tutorial/hello-world.

## Esplorare e preparare una richiesta

L'estensione usa solo il tema scuro: pannello, connessione TMDB, campi e menu.
Le spunte sono disegnate; i menu di scelta sono liste personalizzate con ricerca
digitando, frecce, Home/End, Invio, Esc e Tab. Si aprono sopra o sotto il campo
in base allo spazio, senza essere tagliati dal bordo del pannello.

In **Esplora**, cerca per corrente, sinonimo, autore o opera. I filtri per
categoria, periodo e area sono espandibili. Apri una voce per caratteristiche,
contesto, esempi, limiti interpretativi e fonti. Le caselle permettono selezioni
multiple; **Prepara per questa voce** prepara una sola selezione.

In **La tua richiesta**, scrivi cosa desideri e scegli obiettivo, quantità libera
o esatta, preferenze, esclusioni e già visti. Genera e rileggi il prompt, poi usa
**Inserisci nella chat** o **Copia**. L'invio resta all'utente. Una bozza esistente
richiede la scelta di aggiungere o sostituire; non viene sovrascritta in silenzio.
Ctrl+F cerca nelle correnti; Ctrl+Invio genera la richiesta. L'icona in alto apre
una vista ampia in una scheda del browser.

## Immagini come parte della risposta

Collega TMDB una sola volta dal pulsante **TMDB** nell'intestazione. Sono accettate
una chiave API v3 oppure un API Read Access Token. La connessione già salvata
nelle versioni precedenti viene mantenuta: non serve reinserire la credenziale.

Locandine e foto di registi/interpreti vengono aggiunte automaticamente subito
dopo la riga del titolo, prima della descrizione. Questo vale anche per titoli
in grassetto e descrizioni nello stesso paragrafo, separati da un ritorno a capo.
Le immagini seguono tipografia e colori della pagina della chat; il tema delle
schermate proprie dell'estensione rimane sempre scuro.
Non vengono inseriti pannelli, titoli duplicati, pulsanti, impostazioni, moduli,
selettori, riquadri di errore o richieste di correzione. La locandina apre il player
CineMatch; foto delle persone e attribuzione aprono TMDB. Il testo dell’AI rimane invariato.

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
Puoi importare file o indirizzi JSON anche senza addon. Video HTTP e HLS sono
integrati; torrent, DASH, DRM e header senza proxy non vengono convertiti.

I sette configuratori indicati sono disponibili in una sezione espandibile:
completa la configurazione sui loro siti e incolla il manifest personale.
Nessun addon è preinstallato. [Guida e collaudo](../../docs/estensione-3.0.0-player.md).

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
Nessuna autorizzazione generale viene concessa in fase di installazione.
Non richiede accesso a cookie o cronologia.
Le risposte AI vengono analizzate localmente; TMDB riceve titoli, anni o nomi,
richieste di schede e immagini, non il testo completo della conversazione.
I messaggi dell'utente restano esclusi dai selettori delle risposte.

`sidePanel` apre il pannello, `scripting` inserisce il prompt nei quattro editor,
`clipboardWrite` copia il testo, `storage` conserva le bozze nella sessione.
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

Per aggiornare usa la stessa cartella e premi **Ricarica** in chrome://extensions,
poi ricarica le schede delle chat. La credenziale già salvata viene mantenuta.
Per rimuovere usa **Rimuovi** nella pagina delle estensioni; la credenziale può
essere rimossa separatamente dalla pagina di connessione. L'app Windows precedente
non viene modificata o richiesta.
