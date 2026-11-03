# CineMatch 3.1.0 — motore e decoder integrati

Aggiornamento del 3 novembre 2026. La visione resta nell’estensione, senza
componente desktop o server di conversione. Il film non viene scaricato in un
archivio o convertito completamente prima dell’avvio.

## Riproduzione

Il percorso rapido HTML5/HLS viene mantenuto per le sorgenti già supportate.
MKV, codec mancanti, DASH e fallimenti del player nativo attivano libmedia
AVPlayer 1.3.1, incluso nel pacchetto. WebCodecs e l’accelerazione nativa sono
abilitati quando disponibili; i moduli WebAssembly locali aggiungono decoder
software. L’audio AC3 può essere decodificato in PCM per Web Audio mentre il
video HEVC viene riprodotto; non si genera prima un nuovo file AAC/MP4.

Il motore dispone di controlli per pausa, posizione, volume, schermo intero,
lingue audio e sottotitoli incorporati. Spazio e frecce funzionano quando il
player ha il focus; `f` attiva lo schermo intero. I menu seguono il tema scuro.

## Lettura e memoria

Per i file il lettore CineMatch usa GET con HTTP Range, un blocco in memoria fino
a 1 MiB, timeout di 20 secondi, richieste senza cookie/referrer e `no-store`.
Gli offset sono mantenuti senza troncamento a 32 bit: la suite verifica file
virtuali da 8 GiB e seek oltre 4 GiB. Le letture precedenti sono cancellate al
cambio sorgente; heap dei decoder e fotogrammi richiedono memoria aggiuntiva.
Il buffer di I/O non è un limite alla RAM totale del browser.

HLS/DASH leggono manifest e segmenti con il loader del motore. Nessun film o
blocco video viene salvato in storage.local/session, IndexedDB o OPFS da CineMatch.
I buffer temporanei e la normale gestione di cache/memoria del browser restano
distinti da un archivio locale di film. Il pacchetto è circa 6 MB compresso;
le risorse multimediali installate occupano circa 15 MB, indipendenti dalla
quantità di film guardati.

## Compatibilità e limiti

I 21 moduli decoder comprendono H.264, HEVC, AV1, VP8/VP9, MPEG-2/MPEG-4,
AC3, E-AC3, DTS, AAC, MP3, Opus, FLAC, Vorbis, WMA/WMV e altri moduli ausiliari.
La presenza di un modulo non certifica ogni profilo o variante del formato.

Sono verificati direttamente HEVC + AC3 5.1 in MKV, H.264 + E-AC3/DTS in MKV,
MPEG-4 + MP3 in AVI, MP4/HLS, DASH e tracce audio multiple con sottotitoli SRT.
Le prove software forzano i decoder WASM e controllano fotogrammi e audio
decodificato non silenzioso, non soltanto un evento di avvio.

Non è un motore BitTorrent e non aggira DRM. Per le letture casuali il server
deve supportare Range e fornire la dimensione corretta; accesso/CORS, account,
proxy, link scaduti e codec non inclusi restano cause possibili di errore.
TrueHD non ha un modulo dedicato nel pacchetto. Video 4K/HDR o profili complessi
non hanno una garanzia di fluidità: dipendono dalla decodifica hardware e dal
computer. Non vengono introdotti servizi a pagamento o richieste API AI.

## Provenienza e verifica

AVPlayer e moduli decoder provengono dal progetto libmedia, LGPL-3.0-or-later,
con licenze, provenienza e hash nel pacchetto. Sono stati disabilitati i percorsi
automatici thread/AudioWorklet che generano script blob incompatibili con MV3;
si usano i fallback del progetto. WebAssembly è consentito con `wasm-unsafe-eval`,
senza autorizzare JavaScript `unsafe-eval` o codice remoto.

`scripts/vendor-libmedia.py` verifica offline 56 risorse e può ricostruire la
versione fissata con `--restore`. I componenti rimangono sostituibili nell’estensione
non pacchettizzata. Il controllo OSV di AVPlayer/avcodec 1.3.1 non ha restituito
advisory pubblicate il 3 novembre 2026; non sostituisce gli aggiornamenti futuri.

Il foglio di stile dei sottotitoli assjs è estratto dal bundle originale e
collegato come risorsa locale: evita l’iniezione inline bloccata dal CSP. La
verifica dei sottotitoli confronta anche i pixel con traccia attiva/disattivata
sullo stesso fotogramma in pausa, oltre alla visibilità del nodo DOM.

Nessun account personale o film commerciale è stato usato nel collaudo. Le prove
usano video sintetici FFmpeg, endpoint locali e profili Chromium temporanei.
I test del dialogo nativo dei permessi sono esclusi dalla sessione headless;
la copia di test concede soltanto localhost. Edge e film remoti lunghi non sono
stati collaudati direttamente.

Fonti primarie:
- https://github.com/zhaohappy/libmedia/tree/152f629d3021fd8013efa464fcb7b55f9fbe7753
- https://github.com/zhaohappy/libmedia/blob/master/README_en.md
- https://developer.chrome.com/docs/extensions/reference/manifest/content-security-policy
