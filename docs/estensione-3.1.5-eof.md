# CineMatch 3.1.5 — intervalli vicino alla fine del file

## Causa riprodotta sul server reale

La richiesta di un blocco da 8 MiB vicino a EOF poteva superare la dimensione
del file. Il server RD esaminato rispondeva HTTP 206 copiando la fine richiesta
in Content-Range e Content-Length, anche quando eccedeva i byte disponibili.
Il controllo del lettore rifiutava l’intervallo prima di leggere il corpo.

Sul file reale da 80.138.595.881 byte, il lettore 3.1.4 chiedeva, dopo un seek:

```
Richiesta: bytes=80138530345-80146918952
Risposta:  bytes 80138530345-80146918952/80138595881
```

La prova Chromium con la 3.1.4 ha prodotto esattamente «Il server ha restituito
un intervallo video non valido». Non era una limitazione di risoluzione o GPU:
il lettore si fermava prima della decodifica. I file MKV possono cercare indici
e metadati verso EOF durante l’apertura, rendendo visibile questo errore anche
senza un salto manuale. Il percorso MP4 nativo non usava questo lettore.

## Correzione

Quando la dimensione è nota, ogni richiesta termina al massimo all’ultimo byte
del file, anche per le letture su richiesta dopo un seek; il solo prefetch era
già limitato in questo modo. La richiesta corretta dello stesso test è:

```
bytes=80138530345-80138595880
```

Se una risposta annuncia comunque una fine oltre EOF, per esempio durante il
primo prefisso quando la dimensione non è ancora nota, il lettore limita
allocazione e lettura ai byte realmente disponibili. Non attende i byte
inesistenti annunciati dal server e non li inserisce nel flusso. Rimangono
invalidi offset iniziali diversi, intervalli invertiti, numeri non sicuri e
dimensioni cambiate. I blocchi restano limitati a 8 MiB e non sono archiviati.

## Verifica

- Prima/dopo in Chromium sullo stesso link reale: errore con 3.1.4, lettura
  corretta con la modifica; prefisso e coda, 131.072 byte complessivi letti.
- Apertura dei metadati del medesimo MKV tramite libmedia: video HEVC 3840×2160
  e tracce individuati, 3 richieste e circa 10,5 MB letti in memoria. Il test non
  avvia la riproduzione integrale né scrive il film su disco.
- Test unitari con dimensione di 80 GB, seek nella coda, ultimo byte, EOF e primo
  prefisso più grande della risorsa. Il server simulato ripete la stessa anomalia
  di Content-Range/Content-Length, anziché correggerla artificialmente.
- Test Chromium locale su file virtuale della stessa dimensione, senza creare
  un file da 80 GB o usare un account nei test CI, più le regressioni del player.

Il link privato non è incluso nei sorgenti o nel pacchetto. Questa verifica
riguarda l’errore degli intervalli e l’apertura del contenitore; non certifica
la fluidità di ogni remux 4K o la decodifica di ogni traccia HDR/TrueHD.
