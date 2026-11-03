# CineMatch · Fase 0 — prototipo UI in-chat

Prototipo navigabile per validare l’esperienza **prima** di modificare l’estensione.

> La versione reale del player in-chat è ora in `extensions/cinematch/cinema.html`,
> `cinema-app.js` e `overlay.js`; è documentata in
> [`docs/estensione-3.3.0-in-chat.md`](../estensione-3.3.0-in-chat.md). Questo
> prototipo resta come riferimento visivo e per provare gli stati senza installare
> l’estensione.

## File

| File | Scopo |
|---|---|
| `cinema-ui.html` | Prototipo completo: chat simulata, poster, picker sorgenti, player, drawer, temi e stati |
| `demo.mp4` | Video demo locale (30 s) usato dal player |
| `demo-poster.jpg` | Primo frame usato come poster del player |
| `README.md` | Questo documento |

Nessun dato esce dal computer. Addon, film e flussi sono dati finti.

## Come aprirlo

Dalla radice del repository:

```sh
python3 -m http.server 8123 --bind 127.0.0.1
```

Poi apri:

```
http://127.0.0.1:8123/docs/prototype/cinema-ui.html
```

In alternativa apri direttamente `docs/prototype/cinema-ui.html` nel browser
(il font e il video sono relativi alla cartella).

## Cosa provare

1. Clicca la locandina di **Ultima Luce** nella chat.
2. Nel picker:
   - cambia filtro addon (Tutti / Comet / MediaFusion / EasyStreams);
   - clicca **Riproduci** su una sorgente;
   - apri **Sorgenti** nel player per cambiare flusso;
   - usa i controlli custom, il fullscreen e il menu velocità.
3. Chiudi con X o **Esc**: la posizione viene ricordata e, riaprendo, compare
   **Riprendi da …**.
4. Usa il gruppo **Demo** nella barra in alto per cambiare stato:
   - Pronto
   - Caricamento (skeleton)
   - Errore addon (con “Ricollega”)
   - Nessun addon (onboarding)
5. Clicca **◐** per passare dal tema scuro a quello chiaro: il picker si adatta
   alla chat, il player resta scuro.
6. Prova la finestra stretta (800×600 o 420×850): il picker si impila e il
   drawer diventa un bottom sheet.

### Scorciatoie da tastiera (player)

| Tasto | Azione |
|---|---|
| `Spazio` | Play / pausa |
| `←` / `→` | Indietro / avanti di 5 s |
| `M` | Muto |
| `C` | Sottotitoli demo |
| `F` | Schermo intero |
| `Esc` | Chiude drawer, menu o overlay |

## Deep link per la revisione

| URL | Cosa mostra |
|---|---|
| `?demo=picker` | Picker aperto |
| `?demo=player` | Player avviato |
| `?demo=player&drawer=1` | Player con drawer sorgenti |
| `?demo=picker&state=empty` | Stato “Nessun addon” |
| `?demo=picker&state=loading` | Skeleton di caricamento |
| `?demo=picker&state=error` | Errore addon |
| `?demo=picker&theme=light` | Tema chiaro |
| `?static=1` | Disattiva animazioni (per screenshot) |
| `?chrome=1` | Mantiene i controlli del player visibili |
| `?paused=1` | Mette in pausa il video dopo il caricamento |
| `?poster=1` | Renderizza solo la locandina 800×1200 |

Esempio:

```
http://127.0.0.1:8123/docs/prototype/cinema-ui.html?demo=player&chrome=1&paused=1
```

## POC tecnica — iframe, CSP, fullscreen, permessi

La POC vive fuori dal repository, in `/tmp/opencode/poc` (non fa parte del
pacchetto). Verifica su una pagina con:

```html
<meta http-equiv="Content-Security-Policy"
      content="default-src 'self'; frame-src 'none'; ...">
```

Risultato:

| Verifica | Esito |
|---|---|
| iframe `chrome-extension://` caricato con `frame-src 'none'` | ✅ |
| `postMessage` genitore ⇄ iframe | ✅ |
| `chrome.permissions` disponibile nell’iframe | ✅ |
| `chrome.permissions.contains` sul host permesso | ✅ |
| fullscreen richiesto dall’iframe | ✅ |
| violazione CSP solo per l’iframe di test `https://example.com` | ✅ atteso |

Comando:

```sh
cd /tmp/opencode/poc
python3 -m http.server 8124 --bind 127.0.0.1   # in un terminale
node poc.mjs                                    # in un altro
```

Nota: la richiesta del permesso opzionale reale mostra il prompt di Chrome e va
provata manualmente; la POC verifica che l’API sia disponibile e che il permesso
già concesso sia leggibile dall’iframe.

## Limiti voluti di questa fase

- Nessuna modifica a `extensions/cinematch`.
- Flussi e addon sono mock; il player usa un video locale.
- Il tema è simulato con un toggle: nella versione reale i colori verranno letti
  dal DOM della chat.
- La POC non è ancora collegata al vero `watch.html`.

## Prossimo passo (Fase 1)

- Guscio overlay in Shadow DOM (`<dialog>`, focus, scroll lock, fallback).
- Iframe `cinema.html` con `web_accessible_resources`.
- Click sulla locandina → apertura del prototipo con dati reali TMDB/addon.
- Primi test Playwright sul guscio.
