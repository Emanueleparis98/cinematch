# CineMatch — estensione browser 1.0.0

74 voci sulle correnti del cinema e un generatore locale di richieste per le chat AI.
Non richiede l'app Windows, un eseguibile, un server o chiavi API.

## Installazione in Chrome

1. Estrai completamente `CineMatch-Chrome-1.0.0.zip` in una cartella stabile.
2. Apri `chrome://extensions`, attiva **Modalità sviluppatore** e premi
   **Carica estensione non pacchettizzata**.
3. Seleziona la cartella estratta che contiene `manifest.json`. Dal menu Estensioni
   di Chrome fissa CineMatch alla barra e premi la sua icona per aprire il pannello.

Nel repository la cartella già pronta è `dist/CineMatch-Chrome` dopo la build.
Non selezionare `extensions/cinematch`: è il sorgente, senza dati e font assemblati.
Non cancellare/spostare la cartella caricata finché usi l'estensione.

Chrome 116 o successivo. In Edge usa `edge://extensions` e **Carica decompressa**.
Altri browser Chromium devono supportare l'API `sidePanel`; Firefox e Safari
non sono supportati da questo pacchetto. Questa distribuzione si carica manualmente:
non è una pubblicazione sul Chrome Web Store e non si aggiorna automaticamente.

Guida ufficiale: https://developer.chrome.com/docs/extensions/get-started/tutorial/hello-world

## Uso

1. Cerca una corrente, un autore o un'opera. I filtri sono sotto **Filtri e ordinamento**.
2. Apri la scheda per contesto, caratteristiche, esempi e fonti. Premi
   **Prepara per questa voce**; per selezioni multiple usa le caselle dell'elenco.
3. In **Il tuo prompt** scegli obiettivo, quantità automatica o esatta, criteri,
   esclusioni e titoli già visti. Il generatore non impone cinque film.
4. Premi **Genera il prompt**. Puoi leggere e modificare tutto il testo.
5. Apri la chat scelta e attendi che l'editor sia disponibile. Premi
   **Inserisci nella chat**, poi rileggi e invia tu il messaggio.

Inserimento previsto per **ChatGPT, DeepSeek, Claude e Gemini** nei rispettivi siti.
Se c'è già una bozza, scegli se aggiungere il prompt in fondo, sostituirla o
mantenerla. Se la bozza cambia dopo l'avviso, la scelta va confermata di nuovo.
L'estensione non preme Invio e non invia messaggi automaticamente.

**Copia** funziona anche per altre chat. Non servono collegamenti all'account:
accedi normalmente sul sito. Se vuoi più spazio, usa l'icona in alto per aprire
CineMatch in una scheda; l'inserimento riporta in primo piano la chat selezionata.
Ctrl+F cerca nelle correnti; Ctrl+Invio genera il prompt nella sezione dedicata.

Se cambi i criteri, rigenera prima di copiare/inserire per evitare richieste superate.
Le risposte dipendono dal servizio AI: il plugin non garantisce risposte esatte,
accesso illimitato o assenza dei limiti previsti dai siti.

## Privacy, permessi e spazio

- Consultazione e generazione funzionano offline; nessuna richiesta a servizi AI,
  telemetria o acquisizione progressiva di film. Dati, font e icona sono nel pacchetto.
- L'accesso ai siti riguarda solo `https://chatgpt.com/*`,
  `https://chat.deepseek.com/*`, `https://claude.ai/*`, `https://gemini.google.com/*`.
  Chrome mostrerà il relativo permesso di accesso. Il codice viene eseguito solo
  quando premi **Inserisci nella chat** e legge/modifica esclusivamente l'editor.
  Non legge la conversazione, cookie, credenziali o cronologia di navigazione.
- `sidePanel` apre il pannello; `scripting` permette l'inserimento nei quattro siti;
  `clipboardWrite` copia il prompt; `storage` conserva la bozza in
  `chrome.storage.session`, in memoria per la sessione del browser.
  Non viene usato `storage.local`, `storage.sync` o un database di film.
- La bozza sopravvive alla chiusura del pannello nella stessa finestra. Il riavvio
  del browser, la disattivazione o il ricaricamento dell'estensione la cancellano.
  Non è un archivio di richieste. Una scheda CineMatch già aperta tiene il proprio
  stato fino alla chiusura: evita di modificare la stessa bozza in più viste insieme.
- **Apri chat** non mette il prompt nell'URL. Soltanto il testo inserito/copiato e
  successivamente inviato da te è trattato dal sito secondo le sue condizioni.
  Le fonti si aprono su tua azione. Il pacchetto occupa meno di 400 KB estratto.

## Compatibilità e limiti verificati

I quattro adattatori sono provati con Chromium isolato e pagine di test che
riproducono textarea, ProseMirror/Tiptap e Quill, tramite le vere API dell'estensione.
Non sono stati usati account reali né inviate richieste AI nei test. I siti possono
cambiare editor: in caso di mancato riconoscimento usa **Copia** e incolla.
L'inserimento diretto accetta fino a 200.000 caratteri; copia e modifica non hanno
un limite applicativo di quantità di film. Restano i limiti propri della chat.

Il codice sorgente si trova in `extensions/cinematch` nel repository CineMatch.
La raccolta canonica resta `plugins/cinema-currents/assets/movements.json`:
74 voci, categorie distinte e copertura editoriale ampliabile, non universale.

## Aggiornamento e rimozione

Per aggiornare, estrai la nuova versione nella stessa cartella e premi **Ricarica**
nella pagina delle estensioni. Per rimuovere usa **Rimuovi** nella stessa pagina;
poi puoi cancellare la cartella. L'estensione non modifica l'app Windows già installata.
