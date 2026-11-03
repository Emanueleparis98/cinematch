# CineMatch 3.2.0 — codec, accelerazione e reattività

## Decoder e contenitori

- Aggiunto il decoder software **H.266/VVC** (`decode/vvc.wasm`, codec ID 196)
  con riconoscimento del CodecID Matroska e dell’etichetta H.266/VVC. La
  riproduzione VVC è verificata su MP4; il demuxer Matroska incluso in libmedia
  1.3.1 non associa ancora `V_MPEGI/ISO/VVC`, quindi un MKV VVC resta non
  riproducibile. Non viene convertito o estratto il flusso.
- I contenitori che il player nativo non apre in modo affidabile vengono
  instradati direttamente a libmedia: MKV, WebM, AVI, WMV/ASF, FLV, MPEG-TS,
  M2TS/MTS, VOB, MPG/MPEG, M2V e 3GP/3G2. MP4/MOV restano sul percorso nativo
  quando i codec sono supportati.
- Se il decoder hardware/WebCodecs rifiuta il flusso, il player distrugge quel
  motore e riprova una volta con i decoder software. Gli errori di lettura o di
  rete non vengono scambiati per errori del decoder e non attivano la riprova.
- Le tracce TrueHD/MLP senza un’alternativa AC3/E-AC3/AAC restano non
  decodificabili: il pacchetto non contiene un modulo TrueHD. Il video continua
  e lo stato mostra “audio non supportato”; se esiste una traccia compatibile,
  viene selezionata automaticamente.

## Reattività

- `hls.js` usa ora il suo worker per la parte di parsing, fuori dal thread
  principale. Il worker Blob è consentito dalla CSP dell’estensione ed è stato
  verificato in Chromium.
- Il percorso nativo imposta `preload="auto"` solo durante la riproduzione e
  torna a `preload="none"` quando si ferma la sorgente.
- Restano attivi i blocchi Range adattivi della 3.1.6: 8 MiB normali, 1 MiB dopo
  un timeout, un blocco in lettura anticipata, 16 MiB massimi di cache I/O.
- WebCodecs e l’accelerazione hardware restano abilitati quando disponibili.
  WebGPU è disabilitato perché non porta un vantaggio misurato su questo
  percorso e aumenta il rischio di differenze tra GPU/driver.

## Limiti dichiarati

Non vengono aggiunti BitTorrent, DRM, proxy esterni o conversione completa del
film. La fluidità di un remux 4K/HDR dipende dalla GPU, dal supporto HEVC del
sistema e dal bitrate. Nessun decoder può rendere fluido un 4K software su un
computer che non lo decodifica in tempo reale. I formati senza un decoder
incluso, come TrueHD, non vengono presentati come supportati.

## Verifica

- 74 test Node.
- 28 controlli Chromium del player, incluso un MP4 H.266/VVC decodificato dal
  modulo software e un MKV servito dopo 9 secondi.
- 8 controlli Chromium dei decoder: HEVC/AC3, H.264/E-AC3, DTS, MPEG-4/MP3,
  tracce multiple, sottotitoli SRT e H.266/VVC.
- 57 risorse libmedia verificate per SHA-256, incluso `decode/vvc.wasm`.
- Benchmark di lettura anticipata e test del player ancora superati.

I filmati di prova sono sintetici e generati localmente; nessun film
commerciale, account o link privato è incluso.
