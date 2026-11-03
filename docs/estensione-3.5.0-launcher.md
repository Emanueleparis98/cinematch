# CineMatch 3.5.0 — Launcher correnti e prompt in-chat

La 3.5.0 aggiunge un pulsante flottante accanto alla chat che apre il pannello
reale delle correnti e dei prompt, con un layout largo pensato per il desktop e
un flusso completo fino all’inserimento nel composer.

## Esperienza

- **Pulsante flottante CineMatch** in basso a destra, icona + etichetta su
  desktop, solo icona su mobile.
- **Apertura modale** con backdrop sfocato e pannello centrato; la chat resta
  dietro.
- **Esplora**: elenco delle 74 correnti a sinistra, dettaglio affiancato a destra
  con caratteristiche, figure, opere, fonti e azione “Prepara per questa voce”.
- **La tua richiesta**: form a sinistra, anteprima del prompt a destra; i pulsanti
  **Genera**, **Copia** e **Inserisci nella chat** sono sempre raggiungibili.
- **Inserisci nella chat**: il prompt finisce nel composer della chat corrente,
  senza invio automatico; l’overlay si chiude e il focus torna al pulsante.
- **Bozza e selezioni** restano in `storage.session`, condivise con il pannello
  laterale.
- **Esc / X / backdrop** chiudono e restituiscono il focus al launcher.

## Layout

- Overlay 1080×820 px su desktop, full-screen sotto i 900 px.
- Esplora a due colonne (elenco + dettaglio) sopra i 980 px, a una colonna sotto.
- Richiesta a due colonne (form + anteprima) sopra i 980 px; form e anteprima
  impilati su mobile.
- Pannello scuro coerente con il pannello laterale; header, tab e action bar
  restano leggibili anche sopra una chat chiara.

## Architettura

```
FAB (launcher.js, Shadow DOM)
  → <dialog> + iframe panel.html?view=embed
      → panel.js: syncActive() sulla scheda chat corrente
      → panel-ready → lo skeleton sparisce
      → selezioni + prompt
      → Inserisci → background insert-prompt → composer
      → prompt-close → overlay chiuso, focus al FAB
```

### File

| File | Ruolo |
|---|---|
| `launcher.js` | FAB, dialog, iframe del pannello, focus, scroll lock, chiusura |
| `panel.html` / `panel.js` | modalità `?view=embed`, placeholder dettaglio/anteprima, auto-chiusura dopo inserimento |
| `panel.css` | layout largo a due colonne, action bar, responsive |
| `manifest.json` | `launcher.js` nei content script; `panel.html` in `web_accessible_resources` |
| `background.js` | apertura del pannello completo in una scheda come fallback |
| `tests/extension-launcher-browser.mjs` | 9 controlli del launcher e del flusso di inserimento |

### Sicurezza e dati

- L’iframe è una pagina dell’estensione dichiarata in `web_accessible_resources`
  solo per i quattro host chat.
- I messaggi genitore ⇄ iframe sono validati con `event.source` e origin
  `chrome-extension://`.
- Il pannello non riceve la conversazione; l’inserimento passa dal background e
  dalla scheda attiva già autorizzata.
- La bozza resta nella sessione del browser; nessuna cronologia persistente.

## Collaudo

- 74 test Node.
- 26 controlli browser del pannello laterale e dell’inserimento.
- 21 controlli immagini TMDB.
- 35 controlli player/overlay.
- **9 controlli launcher**:
  - FAB presente una sola volta e assente sulle altre pagine;
  - apertura del pannello reale con 74 correnti;
  - rilevamento della chat corrente;
  - elenco + dettaglio affiancati;
  - form + anteprima a due colonne;
  - inserimento nel composer senza invio;
  - chiusura automatica, focus e riapertura con Esc;
  - responsive a 430 px.
- 8 controlli decoder software libmedia.

## Limiti noti

- Il pulsante ha posizione fissa; trascinamento e opzione “Nascondi” sono
  rifiniture successive.
- Il pannello del launcher è scuro; una variante chiara richiede un passaggio
  completo sui token.
- Le chat reali autenticate vanno provate manualmente; DeepSeek presenta una
  verifica umana nella rete dei test automatici.
- Il fallback “Apri in una scheda” usa `panel.html?view=full` se l’iframe non è
  disponibile.

## Installazione

Scarica `CineMatch-Chrome-3.5.0.zip`, **estrailo** e carica la cartella che
contiene `manifest.json` da `chrome://extensions` con la modalità sviluppatore.
Chrome non carica lo ZIP direttamente.

```sh
python scripts/build-extension.py
pnpm test:launcher
```
