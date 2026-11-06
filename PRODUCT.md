# CineMatch

CineMatch è un’applicazione TUI per Linux, macOS e Windows dedicata a film, serie, anime,
TV e alle correnti del cinema. Riunisce ricerca nei cataloghi degli addon
Stremio/Nuvio, metadati TMDB facoltativi, flussi video, Real-Debrid/TorBox,
preferiti, cronologia e un compositore di prompt locale.

L’utente cerca un titolo, controlla la scheda, sceglie stagione ed episodio quando
serve e apre i flussi dell’addon. La riproduzione avviene in mpv, VLC o ffplay;
CineMatch non avvia un client torrent P2P. I torrent restituiti dagli addon
richiedono un servizio debrid configurato dall’utente. Nessun flusso viene
salvato o convertito interamente prima della visione.

La raccolta editoriale contiene 74 voci: movimenti, tendenze, generi/cicli,
tradizioni e tecniche restano distinti; periodi e attribuzioni sono indicativi.
La consultazione e la generazione dei prompt funzionano offline. Il prompt può
essere copiato o aperto in ChatGPT, DeepSeek, Claude o Gemini: l’invio resta
all’utente e nessuna API AI viene chiamata.

Le chiavi TMDB e debrid e i manifest personali restano nella configurazione locale
dell’utente. CineMatch non importa automaticamente impostazioni di altre versioni,
non acquisisce un database progressivo e non invia la conversazione ai servizi.
La disponibilità di cataloghi, flussi e servizi dipende dai fornitori esterni.
