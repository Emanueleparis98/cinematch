# CineMatch 3.10.1 — Correnti DeepSeek persistenti

La 3.10.1 corregge un bug della 3.10.0: dopo aver attivato **Correnti in
DeepSeek**, ricaricando `chat.deepseek.com` il pulsante non compariva finché non
si riattivava l’opzione nelle Impostazioni.

## Causa

All’avvio della pagina DeepSeek il content script chiedeva lo stato al
background. Il listener del messaggio restituiva `true` ma non eseguiva la
funzione asincrona che avrebbe dovuto inviare la risposta, quindi la richiesta
rimaneva senza risposta e il pulsante restava nascosto. Il toggle manuale
funzionava perché inviava un messaggio separato alle schede aperte.

## Correzione

- Il listener `deepseek-currents-status` esegue ora correttamente la funzione
  asincrona che restituisce lo stato salvato.
- Il pulsante viene ricostruito automaticamente a ogni ricarica di DeepSeek,
  senza dover riattivare l’opzione.
- Aggiunto un test browser che ricarica DeepSeek dopo l’attivazione e verifica
  la presenza del pulsante e dello stato del toggle.

## Verifica

- **10 controlli browser DeepSeek**, inclusa la ricarica della pagina.
- Regressione completa: 86 test Node, 29 controlli browser, 5 TMDB/TV, 23
  immagini, 36 player, 8 codec, 7 CB01, 4 updater e benchmark throughput.
