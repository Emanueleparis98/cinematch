---
name: CineMatch — LUMEN
description: La cabina di proiezione — ink caldo, avorio, un solo ember per i comandi.
colors:
  ink: "#08090c"
  deep: "#0e1016"
  surface: "#151821"
  raised: "#1c202b"
  border: "#2a2f3a"
  border-strong: "#3d4350"
  ivory: "#f4eee2"
  muted: "#a7a399"
  faint: "#918d84"
  faint-deep: "#6e6b64"
  ember: "#ff5c35"
  ember-hi: "#ff7a57"
  celadon: "#8fe3c0"
  danger: "#ff4d5e"
typography:
  display:
    fontFamily: "Fraunces, Georgia, Times New Roman, serif"
    fontSize: "clamp(44px, 6.2vw, 84px)"
    fontWeight: 600
    lineHeight: 0.94
    letterSpacing: "-0.015em"
  headline:
    fontFamily: "Fraunces, Georgia, Times New Roman, serif"
    fontSize: "30px"
    fontWeight: 600
    lineHeight: 1.05
    letterSpacing: "-0.01em"
  title:
    fontFamily: "Instrument Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "15.5px"
    fontWeight: 600
    lineHeight: 1.3
  body:
    fontFamily: "Instrument Sans, Segoe UI, system-ui, sans-serif"
    fontSize: "16px"
    fontWeight: 400
    lineHeight: 1.55
  label:
    fontFamily: "JetBrains Mono, Consolas, monospace"
    fontSize: "10px"
    fontWeight: 400
    lineHeight: 1
    letterSpacing: "0.12em"
rounded:
  none: "0"
  sm: "2px"
  md: "4px"
spacing:
  xs: "4px"
  sm: "8px"
  md: "12px"
  lg: "16px"
  xl: "24px"
  2xl: "32px"
  huge: "56px"
components:
  button-primary:
    backgroundColor: "{colors.ember}"
    textColor: "{colors.ink}"
    rounded: "{rounded.md}"
    padding: "0 20px"
    height: "44px"
  button-primary-hover:
    backgroundColor: "{colors.ember-hi}"
    textColor: "{colors.ink}"
  button-secondary:
    backgroundColor: "{colors.deep}"
    textColor: "{colors.ivory}"
    rounded: "{rounded.md}"
    padding: "0 20px"
    height: "44px"
  button-secondary-hover:
    backgroundColor: "{colors.raised}"
    textColor: "{colors.ivory}"
  chip:
    backgroundColor: "{colors.deep}"
    textColor: "{colors.ivory}"
    rounded: "{rounded.sm}"
    padding: "0 10px"
    height: "24px"
  composer:
    backgroundColor: "{colors.deep}"
    textColor: "{colors.ivory}"
    rounded: "{rounded.sm}"
    padding: "6px 12px"
    height: "52px"
  poster-card:
    backgroundColor: "{colors.deep}"
    rounded: "{rounded.none}"
  nav-item-active:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.ivory}"
    rounded: "{rounded.none}"
    width: "72px"
    height: "62px"
---

# Design System: CineMatch

## Overview

**Creative North Star: "The Projection Booth"**

La cabina di proiezione: una stanza buia, una lampada calda, il film che conta. LUMEN non illumina se stessa. Il fondo è ink, le superfici salgono di un tono appena, il testo è avorio. L'ember è la lampada: compare solo dove si agisce o si è attivi, e proprio per questo conta. Chi guarda non deve vedere l'impianto, deve vedere i film.

Il sistema è editoriale, non cinematografico di superficie: niente popcorn, niente bagliori, niente bolle. Il cinema entra dai poster 2:3, dal titolo in Fraunces e dai metadati in mono, come in una pagina di catalogo. La densità è da strumento: rail da 72 px, etichette da 9–10 px, hairline da 1 px. Il testo respira, i controlli stanno in fila.

La stessa lingua copre due superfici: l'app desktop egui e il sito statico. Cambiano solo le misure di servizio — 2 px di raggio e controlli da 36 px nell'app, 4 px e 44 px sul sito. Colori, font e regola della luce restano identici: una schermata nuova deve riconoscersi come LUMEN anche senza logo.

**Key Characteristics:**
- Ink caldo, mai nero puro; avorio per leggere, mai bianco puro.
- Un solo accento: ember per comandi, selezione, focus e kicker.
- Celadon riservato a disponibilità e stati positivi; danger solo a errore.
- Superfici piatte, separate da hairline da 1 px e salti di tono; nessuna ombra.
- Tre voci tipografiche: Fraunces per i titoli, Instrument Sans per il testo, JetBrains Mono per le etichette.
- Poster 2:3 a filo; tutto il resto quasi squadrato, tra 0 e 4 px.

## Colors

Un fondo notturno caldo, una sola fiamma, un verde di servizio. I neutri non sono grigi puri: hanno una punta di calore, come l'ink sotto una lampada.

### Primary
- **Ember** (#ff5c35): comandi primari, stato attivo, focus, kicker, link in hover, selezione. Riempie superfici piccole — un bottone, un badge, un filo — mai aree grandi.
- **Ember High** (#ff7a57): solo hover dell'ember. Non è un secondo accento.

### Secondary
- **Celadon** (#8fe3c0): disponibilità in streaming, «VISTO», «PRONTO», stato SIMKL completato. Dice che qualcosa c'è, non che si può cliccare.

### Tertiary
- **Danger** (#ff4d5e): errore, offline, rimozione. Mai decorativo.

### Neutral
- **Ink** (#08090c): fondo di app e sito. Il nero non è mai #000.
- **Deep** (#0e1016): campi, chip, bottoni secondari, fondo del poster.
- **Surface** (#151821): pannelli, rail attiva, card rialzate di un tono.
- **Raised** (#1c202b): hover di superfici e controlli.
- **Border** (#2a2f3a): hairline a riposo. **Border Strong** (#3d4350): hover.
- **Ivory** (#f4eee2): testo principale e titoli.
- **Muted** (#a7a399): testo secondario, lede, metadati leggibili.
- **Faint** (#918d84): etichette e note sul sito e nell'app. **Faint Deep** (#6e6b64): riservato a superfici dense, mai a testo sotto i 12 px.

### Named Rules
**The One Voice Rule.** L'ember occupa al massimo il 10% di una schermata. Se due elementi competono per lo stesso sguardo, uno dei due non è un comando.

**The Flat Light Rule.** La luce è un filo o un pieno, mai un alone. Nessun gradiente decorativo, glow, bolla o ombra colorata. Restano due soli usi funzionali già in codice: lo scrim verticale trasparente→ink sopra i backdrop, per rendere leggibile il testo, e il filo ember da 1,5 px che sfuma a destra sulla card selezionata.

## Typography

**Display Font:** Fraunces (fallback Georgia, Times New Roman, serif)
**Body Font:** Instrument Sans (fallback Segoe UI, system-ui, sans-serif)
**Label/Mono Font:** JetBrains Mono (fallback Consolas, monospace)

**Character:** Una testata di cinema e una strumentazione da cabina. Fraunces SemiBold dà peso e curva ai titoli; Instrument Sans tiene l'interfaccia neutra; JetBrains Mono, maiuscolo nelle etichette e spaziato nei metadati, dice dove siamo e cosa stiamo guardando.

### Hierarchy
- **Display** (600, clamp 44–84 px sul sito, 38–56 px nell'app, line-height 0.94, tracking −0.015em): solo il titolo della hero e i titoli di vista. Massimo due righe.
- **Headline** (600, 30 px / 1.05, tracking −0.01em; 22–30 px nell'app): titoli di sezione e di pannello.
- **Title** (600, 15.5 px / 1.3): sottotitoli dei passi e titoli di riga; nell'app 11–13 px.
- **Body** (400, 16 px / 1.55): testo corrente; il lede sale a 16,5 px e resta entro 50ch. Nell'app 12–13 px.
- **Label** (400, 9–10,5 px mono, maiuscolo, tracking 0,08–0,14em): kicker, etichette di campo, metadati, comandi. Il pavimento è 9 px; l'8 px esiste solo nel badge «VISTO».

### Named Rules
**The Two-Voice Rule.** Fraunces e Instrument Sans leggono; JetBrains Mono non scrive mai una frase. Se una riga è più lunga di un'etichetta, non è mono.

**The Small Label Rule.** Le etichette mono sono maiuscole, tra 9 e 10,5 px, con tracking tra 0,08 e 0,14em. Sotto i 9 px si sconfina nel badge, non nel testo.

## Layout

Sito e app condividono lo stesso ritmo: colonna centrale, hairline orizzontali, blocchi da 46–56 px di respiro verticale.

**Sito:** contenitore 1120 px, gutter 56 px (36 px sotto 760), nav 68 px. Hero a due colonne 1.04/0.96 con gap 56; sezioni informative 1fr/1fr con gap 64; blocco SIMKL a tre colonne (1fr / 260–360 / 1fr) con gap 40. Breakpoint a 1080, 980, 760 e 430 px: prima si impila la terza colonna, poi la hero, poi si nasconde la nav. La CTA si impila sotto 760.

**App:** rail 72 px, barra superiore 48 px, barra di stato 28 px, finestra minima 800×600. Contenuto massimo 1100 px, padding 34 px (20 sotto 900). La griglia passa a 2/3/4/5 colonne a 560/740/1040 px con gap 14; la scheda film è un pannello laterale da 260–420 px, i filtri 360 px. A 1220 px la griglia arriva a cinque colonne senza filtri, quattro con.

**Spacing:** base 8, passi 4/8/12/16/24/32/56. I controlli non scendono sotto 24 px di altezza (chip) e 36 px nell'app (bottoni); sul sito il bottone è 44 px.

### Named Rules
**The Hairline Rule.** Le sezioni si separano con una linea da 1 px, non con un'ombra o un riquadro. Se serve più stacco, si aggiunge spazio.

**The Poster Rule.** Il poster è sempre 2:3, a filo, senza raggio. È l'unica immagine che può essere grande.

## Elevation & Depth

Il sistema è piatto: nessuna ombra, in nessuna superficie. L'app azzera esplicitamente `window_shadow` e `popup_shadow`; il sito non dichiara un solo `box-shadow`. La profondità è tonale e lineare — ink → deep → surface → raised — e ogni salto è marcato da un bordo da 1 px. Un pannello che si apre non galleggia: sale di tono e prende una hairline. Sopra le immagini, l'unica profondità è lo scrim funzionale che porta al testo.

### Named Rules
**The No-Shadow Rule.** Se un elemento deve staccarsi, sale di tono o prende un bordo. Mai un'ombra, mai un blur.

## Shapes

Quasi squadrato. Poster a 0 px; chip, campi e bottoni dell'app a 2 px; card, figure e bottoni del sito a 4 px. Nessun raggio oltre 4 px, nessuna pillola: i «pill» dei filtri sono chip da 2 px. I bordi sono sempre interni, da 1 px. Le geometrie ricorrenti sono il poster 2:3, la hairline, la barra ember da 2 px sulla rail attiva e il filo ember da 1,5 px sulla card selezionata. Le uniche curve piene sono il punto da 6 px del kicker e le icone da 20 px con tratto 1,5.

## Components

### Buttons
Strumenti, non pastiglie: piatti, etichettati, con un bordo da 1 px.
- **Shape:** app 2 px, sito 4 px; altezza 36 px nell'app, 44 px sul sito.
- **Primary:** riempimento ember, testo ink, mono maiuscolo 10–10,5 px con tracking 0,08em, padding orizzontale 12 px (app) o 20 px (sito). Hover ember-high; premuto mescola l'ember col 18% di nero.
- **Focus:** anello da 1 px. Ember sul fondo scuro, ivory quando il bottone è già ember; offset 3 px sul sito, interno all'app.
- **Secondary:** fondo deep, bordo 1 px, testo avorio; hover su raised con bordo border-strong.
- **Disabled:** fondo deep, testo faint, nessun hover.
- **Ghost / link:** solo testo mono maiuscolo 10 px, muted che vira a ivory (o ember-high se già colorato). Nessun bordo.

### Chips
- **Style:** fondo deep, bordo da 1 px, raggio 2 px, altezza minima 24 px, testo Instrument Sans 11,5 px, padding 0 10 px.
- **Selected:** bordo ember e testo avorio forte; la superficie resta deep.
- **Disabled:** fondo ink, testo faint, con il motivo nel tooltip.

### Cards / Containers
- **Poster card:** 2:3, raggio 0, bordo da 1 px che passa da border a border-strong in hover e a ember in selezione; filo ember da 1,5 px sul bordo superiore quando è attiva. Titolo a 12 px Instrument Sans SemiBold, 6 px sotto il poster; metadati a 9 px mono faint.
- **Visto:** velo grigio al 67% sul poster (20% in hover) e badge ink con bordo border-strong, testo celadon 8 px.
- **Mini card:** fondo surface, raggio 2 px, hover raised; miniatura 43×64 a filo, titolo 12 px, metadati 9 px mono.
- **Figure del sito:** fondo deep, bordo 1 px, raggio 4 px, didascalia mono 9 px faint.

### Inputs / Fields
- **Composer:** frame deep, bordo 1 px, raggio 2 px, padding 6/12; il bordo diventa ember al focus, senza glow. Testo avorio 13 px, hint muted.
- **Send:** quadrato ember 40×40, raggio 2 px, icona freccia ink; nella hero diventa 100×40 con l'etichetta «CERCA». Hover ember-high.
- **Select / search:** stessa ricetta del composer; nessuna ombra, nessun riempimento chiaro.

### Navigation
- **Rail app:** 72 px, voce 72×62 con icona 20 px a tratto 1,5 e label mono 9 px maiuscolo. Attiva: fondo surface, barra ember da 2 px a sinistra, icona e testo avorio. Hover: surface al 50%.
- **Header sito:** 68 px, hairline in basso; link mono 10 px maiuscolo, tracking 0,08em, muted → avorio. Brand Fraunces 20 px, con «MATCH» in ember. Il bottone di download è la variante small da 34 px.

### Film Sheet
Pannello laterale fisso da 260–420 px, fondo surface con hairline. Backdrop con scrim, poster a filo, titolo Fraunces 24 px, titolo originale 9 px mono faint, fatti uniti da «·», disponibilità in celadon, stato SIMKL celadon o danger. Chiude con Esc. Nessuna ombra: è una superficie, non un foglio sospeso.

### Status Bar
28 px, fondo ink, etichette mono 9 px. «PRONTO» celadon solo dopo una richiesta di rete riuscita, «LAVORO» ember durante le operazioni, «OFFLINE» danger quando l'ultima richiesta fallisce. Versione e sorgente in faint.

## Do's and Don'ts

### Do:
- **Do** usare l'ember solo su comandi, selezione, focus e kicker: un solo elemento ember per blocco d'azione.
- **Do** separare con hairline da 1 px e salti di tono; il fondo resta ink.
- **Do** scrivere le etichette in mono maiuscolo 9–10,5 px con tracking 0,08–0,14em.
- **Do** tenere i poster 2:3, a filo, con bordo da 1 px che diventa border-strong in hover ed ember in selezione.
- **Do** usare celadon per disponibilità e stati positivi, danger per errore e offline.
- **Do** mantenere il focus visibile: anello ember da 1 px, offset 3 px sul sito, interno all'app; su un riempimento ember l'anello passa a ivory.
- **Do** rispettare le altezze di controllo della superficie: 44 px sul sito, 36 px nell'app, 24 px per le chip.

### Don't:
- **Don't** aggiungere ombre, glow, gradienti decorativi, aloni o bolle: restano solo lo scrim e il filo ember funzionali già in codice.
- **Don't** usare l'ember come colore del testo corrente o per riempire aree grandi.
- **Don't** usare celadon o danger come decorazione.
- **Don't** arrotondare oltre 4 px né trasformare chip e bottoni in pillole.
- **Don't** usare Fraunces sotto i 18 px o per il corpo del testo.
- **Don't** usare JetBrains Mono per frasi intere.
- **Don't** introdurre nuove tinte: la palette è chiusa.
- **Don't** mettere un poster dentro un riquadro arrotondato o con ombra.
