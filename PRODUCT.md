# CineMatch

CineMatch è un'app Windows dedicata alla consultazione delle correnti del cinema
e alla preparazione di richieste per chat esterne. L'utente è un cinefilo che vuole
capire una corrente e trovare un modo preciso di chiedere consigli o approfondimenti.

La raccolta editoriale contiene 74 voci, con 26 revisioni e 48 aggiunte rispetto
alla versione precedente. Movimenti, tendenze, generi/cicli, tradizioni e tecniche
restano distinti; periodi e attribuzioni sono indicativi. La copertura è ampliabile.

Percorso principale: cerca o filtra una voce, leggi la scheda e le fonti,
seleziona una o più voci, prepara la richiesta, modifica il prompt, copialo e
apri la chat scelta. Incolla e invio sono eseguiti dall'utente nel servizio esterno.

Il generatore è il plugin separato Cinema Currents, utilizzabile come libreria
Rust o eseguibile con protocollo JSON. Generazione e consultazione sono locali.
L'app non richiede chiavi API, non invia messaggi e non usa provider AI.
Non acquisisce database di film, locandine o risposte. Le richieste rimangono in
memoria fino alla chiusura. I file personali delle versioni precedenti sono conservati.

La qualità e gli eventuali limiti della risposta dipendono dal servizio esterno.
Le schede includono note sui casi discussi; nessuna percentuale di infallibilità
o promessa di completezza universale.
