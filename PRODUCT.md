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
titoli, anni e persone; aggiunge immagini sotto ciascun titolo durante lo streaming.
Locandina, regista e primi interpreti sono visibili senza click, con colori e
caratteri ereditati dalla chat. Nessun pannello, titolo duplicato, modulo, pulsante
o impostazione viene inserito nella risposta. Il testo originale non è riscritto.
Una credenziale TMDB personale è configurata nelle opzioni, isolata dai siti.
Abbinamenti incerti e immagini non disponibili vengono omessi senza interfacce
di errore. Il riconoscimento gestisce emoji, numerazioni e sottotitoli tradotti.
Non acquisisce un database progressivo. Le bozze restano in storage.session;
la credenziale può essere ricordata in storage.local su scelta dell'utente.
Le immagini sono sempre automatiche; le vecchie preferenze di disattivazione
non vengono usate. Il pannello adotta un'identità solo scura con accento cobalto,
spunte disegnate e menu personalizzati accessibili. Le immagini vengono ancorate
alla riga del titolo, prima della descrizione anche nei paragrafi con titolo inline.
L'app Windows e il CLI Rust precedenti restano nel repository come versione
precedente e fonte dei dati; non sono necessari per l'estensione.

Dal clic sulla locandina si apre ora un pannello CineMatch sopra la chat, senza
cambiare scheda: il film viene risolto tramite TMDB/IMDb, il picker mostra le
sorgenti degli addon scelti dall’utente e il player parte nello stesso pannello.
Il tema del picker segue la chat, mentre il player resta scuro; la posizione di
visione viene ricordata nella sessione. Se l’iframe non può essere incorporato,
resta disponibile la scheda dedicata come fallback.
Il pannello Addon collega, attiva e rimuove manifest compatibili con Stremio/Nuvio
senza lasciare la chat; la pagina dedicata resta per la gestione completa e il
fallback. Nessun codice GPL del player nativo Nuvio viene copiato. HTTP, HLS e DASH sono riprodotti nel browser;
torrent, DRM e header senza proxy conservano i limiti delle rispettive sorgenti.
Liste JSON possono essere importate solo per la sessione. I soli dati persistenti
nuovi sono i manifest e le preferenze di attivazione, non i film o i flussi.
I permessi opzionali vengono richiesti per gli host scelti. Nessun cookie,
testo della chat o credenziale TMDB viene inviato agli addon. Il player resta
scuro; il picker segue il tema della chat. Manrope, i token e i controlli del
sistema esistente restano invariati.

Il player distingue ora gli errori HTTP dai formati non supportati. Le etichette
dei codec guidano l’ordine e gli avvisi; all’avvio viene letto un prefisso limitato
del flusso per riconoscere HLS e i CodecID Matroska disponibili. Le capacità sono
interrogate nel browser in uso, senza dichiarare HEVC sempre supportato o sempre
incompatibile. Il motore libmedia e i decoder locali vengono selezionati quando
servono per MKV/AVI, HEVC, AC3/E-AC3/DTS e altri formati inclusi. I dati vengono
letti a blocchi e decodificati durante la visione, senza convertire/salvare prima
l’intero film. Per i file pesanti il lettore anticipa un blocco e riusa il CDN
risolto, con due finestre fino a 8 MiB in memoria; i buffer del motore sono
aggiuntivi. Una scadenza del CDN fa risolvere di nuovo il link dell’addon.
Restano le differenze di hardware, server, profili e codec inclusi;
non si promette che ogni URL HTTP sia riproducibile.

La qualità e gli eventuali limiti della risposta dipendono dal servizio esterno.
Le schede includono note sui casi discussi; nessuna percentuale di infallibilità
o promessa di completezza universale.
