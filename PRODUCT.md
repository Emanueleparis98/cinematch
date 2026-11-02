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
Generazione e consultazione sono locali. Non richiede API AI e non invia messaggi.
L'arricchimento automatico analizza localmente le risposte dell'AI e cerca su TMDB
titoli, anni e persone; aggiunge schede sotto ciascun titolo durante lo streaming.
Locandina, regista e primi interpreti sono visibili senza click; il cast completo
è espandibile. Nessuna riscrittura del testo o analisi dei messaggi dell'utente.
Una credenziale TMDB personale è configurata nelle opzioni, isolata dai siti.
Abbinamenti incerti richiedono una scelta; ricerca manuale e correzione disponibili.
Non acquisisce un database progressivo. Le bozze restano in storage.session;
la credenziale può essere ricordata in storage.local su scelta dell'utente, insieme
alla preferenza per le immagini automatiche (attive per impostazione predefinita).
L'app Windows e il CLI Rust precedenti restano nel repository come versione
precedente e fonte dei dati; non sono necessari per l'estensione.

La qualità e gli eventuali limiti della risposta dipendono dal servizio esterno.
Le schede includono note sui casi discussi; nessuna percentuale di infallibilità
o promessa di completezza universale.
