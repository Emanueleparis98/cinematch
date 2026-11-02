# CineMatch

CineMatch è ora un'estensione Chrome/Edge dedicata alla consultazione delle correnti del cinema
e alla preparazione di richieste per chat esterne. L'utente è un cinefilo che vuole
capire una corrente e trovare un modo preciso di chiedere consigli o approfondimenti.

La raccolta editoriale contiene 74 voci, con 26 revisioni e 48 aggiunte rispetto
alla versione precedente. Movimenti, tendenze, generi/cicli, tradizioni e tecniche
restano distinti; periodi e attribuzioni sono indicativi. La copertura è ampliabile.

Percorso principale: cerca o filtra una voce, leggi la scheda e le fonti,
seleziona una o più voci, prepara la richiesta, modifica il prompt e inseriscilo
nell'editor di ChatGPT, DeepSeek, Claude o Gemini dal pannello laterale. L'invio
è eseguito dall'utente nel servizio esterno. Copia e incolla servono altre chat.

Il pacchetto browser è autonomo, Manifest V3, senza dipendenze runtime esterne.
Generazione e consultazione sono locali. Non richiede chiavi API e non invia
messaggi. Non acquisisce database di film, locandine o risposte; non legge le
conversazioni. Le bozze restano in storage.session nella memoria del browser.
L'app Windows e il CLI Rust precedenti restano nel repository come versione
precedente e fonte dei dati; non sono necessari per l'estensione.

La qualità e gli eventuali limiti della risposta dipendono dal servizio esterno.
Le schede includono note sui casi discussi; nessuna percentuale di infallibilità
o promessa di completezza universale.
