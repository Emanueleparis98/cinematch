# CineMatch 3.16.0 — Layout sorgenti compatto

> **Aggiornata dalla 3.17.0** per il consenso ai server video nel player.
> Vedi [Consenso server video 3.17.0](estensione-3.17.0-permessi-player.md).

La 3.16.0 sistema la schermata **Scegli una sorgente** con lo stesso criterio
del popup impostazioni: meno spazio sprecato, tutto più contenuto e leggibile.

## Cosa cambia

- Il dialogo del picker non occupa più tutta la finestra: è largo **940 px** e
  alto **720 px**, centrato. Il player resta ampio (1180 × 820) e la gestione
  addon usa 900 × 760.
- La colonna del film passa da 258 a 220 px; la locandina è più compatta
  (195 px), il titolo scende da 36 a 30 px e metadati, azioni e trama sono più
  densi.
- Le schede delle sorgenti sono più basse e compatte: icona da 32 px, padding
  ridotto, testi e badge più piccoli. Le sorgenti non riproducibili mostrano
  finalmente l’icona di avviso (prima il cerchio restava vuoto).
- Filtri, pulsanti e schede di errore seguono la stessa scala compatta.
- La scelta della sorgente mantiene tutte le funzioni: filtri per addon,
  consigliata, apertura sul sito, importazione JSON, cambio episodio e player.

## Cosa non cambia

- Correnti, impostazioni e aggiornamenti restano nel popup dell’icona.
- Il player, il drawer delle sorgenti e la gestione addon non perdono funzioni.
- Nessuna modifica a credenziali, addon o preferenze.

## Verifica

- 36 controlli player/browser, compreso il fatto che il picker resti entro
  980 px e che il player si allarghi quando parte.
- Regressione completa: 86 test Node, 29 browser, 12 DeepSeek, 5 TMDB/TV,
  23 immagini, 8 codec, 7 CB01, 4 updater e throughput.
