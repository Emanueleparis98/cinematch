# CineMatch 3.19.2 — Aggiornamenti e affidabilità

La revisione riguarda updater, persistenza della cartella, pacchetti ZIP,
controllo degli aggiornamenti e ciclo di vita del player. Mantiene le funzioni
della 3.18 e non reintroduce il modulo Live ritirato.

## Aggiornare senza selezionare sempre la cartella

La cartella scelta viene conservata in IndexedDB. **Aggiorna ora** apre una
finestra persistente e riprende l’operazione automaticamente se l’accesso è già
autorizzato. Se il browser ha sospeso il permesso, **Conferma accesso e aggiorna**
richiede l’accesso alla cartella salvata: non apre nuovamente il selettore.
La prima installazione, una cartella spostata o la cancellazione dei dati del
browser possono comunque richiedere di selezionarla.

Un riavvio non confermato non elimina più il riferimento alla cartella.
Viene conservata la versione in attesa e offerto **Riavvia estensione**.
Le connessioni IndexedDB vengono chiuse al termine delle operazioni.

## Problemi individuati e corretti

| Problema | Comportamento nuovo | Verifica |
| --- | --- | --- |
| Cartella dimenticata dopo un aggiornamento non confermato | Riferimento conservato e riavvio riprovabile | Riapertura del popup dopo scrittura con versione ancora precedente |
| Stati di aggiornamento poco leggibili nel tema chiaro | Colori di successo ed errore con maggiore contrasto | Controllo visivo del popup |
| Riconferma del permesso confusa con nuova selezione | Richiesta sul riferimento esistente, avviata dal click | Permesso scaduto, selettore impostato per fallire se richiamato |
| Possibilità di aggiornare una copia diversa di CineMatch | Prova temporanea nella cartella, letta tramite l’origine dell’estensione caricata | Cartella servita corretta e copia diversa rifiutata |
| Finestre concorrenti potevano scrivere insieme | Lock condiviso fra le finestre durante l’aggiornamento | Seconda finestra bloccata prima delle scritture |
| Scrittura interrotta lasciava un aggiornamento parziale | Snapshot dei file sostituiti e tentativo automatico di ripristino | Errore di scrittura simulato, recupero degli originali e rimozione dei nuovi file |
| Tutti i file venivano riscritti a ogni aggiornamento | I file identici vengono confrontati e saltati | Decoder e file invariati esclusi dalle scritture |
| Dimensioni e tempi di download non erano limitati durante la lettura | Limiti sui byte ricevuti e timeout; SHA-256 prima delle modifiche | Corpo sovradimensionato annullato e download troncato rifiutato |
| ZIP espanso prima del controllo del limite totale | Limiti prima e durante l’estrazione, coerenza degli header e percorsi unici | ZIP con dimensioni false, percorsi duplicati, cifrati e header incoerenti rifiutati |
| Decoder o probe di una sorgente precedente potevano aggiornare il nuovo player | Callback vincolate alla sorgente attiva e superfici dei decoder separate | Due avvii sovrapposti e decoder precedente completato in ritardo |
| Tornare al selettore non annullava un caricamento in corso | Probe e decoder fermati quando si esce dal player | Risposta video ritardata: nessuna riproduzione nascosta dopo il ritorno |

La scrittura di un’estensione non pacchettizzata non è un’operazione atomica
offerta dal browser. Se disco o permessi diventano indisponibili anche durante
il ripristino, CineMatch segnala il ripristino incompleto e non ricarica
automaticamente l’estensione. I file personali non presenti nel pacchetto non
vengono sovrascritti né eliminati.

## Verifica

Le prove automatiche includono filesystem simulati con guasti, ZIP ostili,
limiti di download, decoder concorrenti e sessioni Chromium con estensione MV3.
Il test browser verifica anche che un nuovo file della cartella caricata sia
leggibile tramite l’origine dell’estensione, prima di simulare il filesystem
scelto dall’utente. I test del player riproducono media sintetici reali.

Comandi: `npm test`, `npm run test:updater`, `npm run test:browser`,
`npm run test:player`, oltre alle regressioni su immagini, serie, CB01 e codec
eseguite dalla pipeline di release.
