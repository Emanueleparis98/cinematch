---
target: site/index.html
total_score: 25
max_score: 36
na_heuristics: 7
p0_count: 0
p1_count: 2
target_identity: "file:C:\\Users\\emanu\\Desktop\\test\\cinematch\\site\\index.html"
target_fingerprint: "sha256:eadd0e50825b556875eb8400be908a3a08aa79101ea581d7c379115a32efdfcd"
target_path: "C:\\Users\\emanu\\Desktop\\test\\cinematch\\site\\index.html"
timestamp: 2026-10-29T12-33-20Z
slug: site-index-html
closed: true
---
# Critique — CineMatch site (Persuade)

Target: `site/index.html` — live su https://emanueleparis98.github.io/cinematch/
Method: dual-agent A/B. A: design review con Chrome headless 1440 e 390 (browser in-app disconnesso). B: detector + CDP 320–1440.

## Design Health Score

| # | Heuristic | Score | Key issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Versione e dimensione si aggiornano, ma il download è un salto silenzioso a GitHub e la nav non ha stato attivo. |
| 2 | Match System / Real World | 3 | Hero in italiano semplice; più sotto “corrente dalla guida”, “Sblocca”, “polling”, “SHA-256”, “installer per-utente” senza glossario. |
| 3 | User Control and Freedom | 3 | Pagina lunga su mobile senza back-to-top; i link GitHub sostituiscono la pagina. |
| 4 | Consistency and Standards | 3 | CTA header mobile non allineata; “PRONTO?” senza il punto ember delle altre kicker; release note in mono come una frase. |
| 5 | Error Prevention | 2 | Un visitatore da telefono riceve un `.exe` Windows senza avviso “solo PC”; nessuna preparazione a SmartScreen. |
| 6 | Recognition Rather Than Recall | 3 | Etichette visibili, nessuna icona nuda; la nav sparisce su mobile e i termini di dominio restano da imparare. |
| 7 | Flexibility and Efficiency | n/a | Superficie Persuade statica, una sola azione: niente da accelerare. |
| 8 | Aesthetic and Minimalist Design | 3 | Superficie disciplinata; “Come funziona” e la release note sono densità inutile. |
| 9 | Error Recovery | 3 | I bottoni funzionano senza JS; se il fetch release fallisce restano versione e dimensione vecchie senza “il download non parte?”. |
| 10 | Help and Documentation | 2 | Nessuna FAQ, nessun passo di installazione, nessun requisito di sistema; la chiave TMDB obbligatoria è sepolta nel corpo. |
| **Total** | | **25/36 (69%)** | **Acceptable — appena sotto Good** |

## Design Specificity Verdict

**Verdict: pelle LUMEN specifica, chassis da landing page standard.** Palette, Fraunces, kicker mono 9–10 px, hairline, raggi 4 px e wordmark rendono la pagina inconfondibilmente CineMatch; la copy è autoriale (“fino a cinque film reali con il perché”, “Sblocca”, le esclusioni esplicite). La composizione però è il template: hero split + screenshot → 3 passi → 6 fatti → feature block → screenshot largo → CTA → footer. Il differenziale — il motore che **rifiuta** e spiega il compromesso — è il punto 2 di 6 in “Come funziona” e non si vede mai.

**Detector:** 40 finding (36 warning, 4 advisory): 35 `undersized-ui-text` (9 px ×20, 10 px ×12, 9.5 px ×1, 10.5 px ×2), 2 `overused-font`, 1 `hero-eyebrow-chip`, 1 `kicker-above-heading`, 1 `tiny-text`. Le righe sono tutte `0`: il detector non mappa il sorgente. Falsi positivi: `overused-font` (Fraunces/Instrument Sans sono brand commitment, self-hosted), `hero-eyebrow-chip`/`kicker-above-heading` (kicker LUMEN codificati), `tiny-text` (il body è 16/16.5/13.5 px; il sub-11 è tutto label). Il caso genuino è la **frase** “Novità recenti…” in mono 9 px.

## Overall Impression

Il primo schermo funziona: headline italiana, Fraunces, screenshot reale, CTA chiara. Poi la pagina racconta invece di dimostrare. Il momento che decide — scaricare un `.exe` e aprire l’app per la prima volta — arriva senza la chiave TMDB, senza una nota SmartScreen e senza un esempio leggibile del “perché”. Il picco è la scheda SIMKL; la valle è il muro dei 6 fatti e la release note in mono.

## What's Working

1. **Il sistema regge senza logo.** Hero, fatti e footer usano lo stesso ink/avorio, hairline, label mono e raggi 4 px dello screenshot app. Niente ombre, glow o pill: LUMEN è coerente.
2. **Copy che dichiara i limiti.** “Il motore di vincoli scarta le combinazioni impossibili: nessun titolo inventato” e la descrizione di Sblocca sono credibili e allineate a PRODUCT.md; nessun prezzo, nessun numero utenti o testimonianza inventati.
3. **Immagini reali.** `app-chat.png` e `app-simkl.png` sono UI vera, non mockup; la scheda SIMKL è leggibile. Buona cura tecnica: versione/dimensione live, validazione URL, skip link, dimensioni immagini, preload font.

## Priority Issues

1. **[P1] La chiave TMDB obbligatoria è invisibile nel punto di decisione.**
   *Perché:* l’hero promette “Scrivi cosa vuoi vedere”; l’unica menzione della chiave gratuita obbligatoria è il secondo di sei fatti, 13.5 px muted, circa una schermata e mezza sotto la CTA. Jordan scarica, apre, trova un campo chiave e non ha nulla a cui tornare.
   *Fix:* una riga sotto la CTA: “Serve una chiave TMDB gratuita (2 minuti) al primo avvio · AI e SIMKL opzionali”, più un trio “Cosa ti serve”.
   *Command:* `/impeccable clarify`.

2. **[P1] Nessun ponte di fiducia al momento dell’installazione.**
   *Perché:* un `.exe` Windows da GitHub senza una parola su code signing, SmartScreen, admin o disinstallazione. L’installer 0.10.3 è `NotSigned`: se SmartScreen mostra “Windows protected your PC”, la pagina ha già speso tutta la credibilità sull’onestà.
   *Fix:* blocco “Prima di installare”: installazione per-utente, nessun admin, come superare SmartScreen (“Ulteriori informazioni → Esegui comunque”), verifica SHA-256, disinstallazione. Tradurre “installer per-utente”.
   *Command:* `/impeccable onboard`.

3. **[P2] “Come funziona” è un muro e il differenziale è sepolto.**
   *Perché:* sei fatti espansi + una release note di ~68 parole in mono 9 px: la valle più lunga e il fallimento di progressive disclosure più chiaro. Il motore di vincoli è il punto 2 di 6, descritto in astratto.
   *Fix:* tre fatti (regole offline / TMDB reale / sul tuo PC), il resto in un “Dettagli tecnici” collassato o nel README, e **un esempio concreto di rifiuto**. Togliere “Novità recenti” dalla pagina o linkarla alle note release.
   *Command:* `/impeccable distill`.

4. **[P2] La prova è decorativa, non evidence.**
   *Perché:* lo screenshot hero è ~511 px a 1440 (testo in-app 4–5 px) e ~354 px su mobile; il “perché” e il rifiuto onesto non compaiono mai come UI leggibile. La pagina racconta ciò che dovrebbe mostrare.
   *Fix:* crop stretto di una answer card con titolo + “Perché” + un consiglio, e uno stato “nessun film copre tutti i criteri → 3 compromessi”; link allo screenshot completo.
   *Command:* `/impeccable shape`.

5. **[P2] Su mobile la CTA è un vicolo cieco e il contesto sparisce.**
   *Perché:* `.nav-links` è nascosto e `.hide-sm` toglie “per Windows” dalla label: resta “Scarica” con nome accessibile mutato, su un telefono che non può installare un `.exe`. La CTA header è schiacciata a sinistra (`.nav .btn-small { margin-left: 0 }`) e non c’è back-to-top.
   *Fix:* “Scarica per Windows” sempre, nota “Solo PC Windows 10/11” vicino alla CTA, `margin-left:auto` a ≤760, 2–3 anchor o back-to-top.
   *Command:* `/impeccable adapt`.

## Persona Red Flags

**Jordan (first-timer):** la chiave TMDB è una clausola sepolta, non un passo; “corrente dalla guida”, “Sblocca” e “installer per-utente” non sono definiti; nessuna FAQ e nessun percorso di installazione. Si blocca al primo campo chiave senza aiuto sulla pagina.

**Riley (stress tester):** da telefono ottiene “SCARICA” (qualifica Windows nascosta) → lista asset `.exe` su GitHub. Prova a trascinare l’immagine larga “frecce e trascinamento per scorrere”: è un JPG statico. Bloccando l’API release vede v0.10.3 / ~6 MB restare come se fossero attuali.

**Casey (mobile):** CTA header 34 px schiacciata a sinistra, nav e GitHub spariti, nessuna azione sticky; circa cinque schermate alla seconda CTA; lo screenshot hero è illeggibile; l’unica azione scarica un installer Windows su iOS/Android. “Tutte le versioni” è un tap target da 9 px.

## Minor Observations

- “PRONTO?” non ha il punto ember delle altre kicker (`index.html:173` vs 67/134).
- “Novità recenti” è una frase in mono 9 px: viola la Two-Voice Rule.
- “Scarica per Windows” salta a GitHub senza marcatore esterno, mentre “Vedi su GitHub ↗” ce l’ha.
- A 1440 la colonna “Come si usa” finisce ~200 px prima della destra: buco visibile sotto “Affina”.
- La pagina non dice se l’app è gratuita/open source.
- L’h1 mette l’ember su due righe fino a 84 px e la CTA è ember: l’accento perde scarsità (One Voice Rule).
- `scroll-behavior: smooth` senza `prefers-reduced-motion`; tap target <24 px (nav, “Tutte le versioni”, footer).
- Nessuna FAQ, requisiti o “cosa succede dopo il click”.

## Questions to Consider

1. E se l’hero mostrasse una risposta vera con il suo “perché” **e** un rifiuto onesto, invece della home dell’app?
2. Se sparissero tutti e sei i fatti di “Come funziona”, cosa perderebbe un visitatore prima di cliccare “Scarica”? Se la risposta è “niente”, appartengono al README.
3. Cosa dice questa pagina alla maggioranza che arriva da telefono? Oggi: “Scarica” e un vicolo cieco.

## Coverage / Limitations

Browser in-app disconnesso: A ha usato Chrome headless 1440×900 + 390 via iframe; B ha usato Chrome headless via CDP 320–1440. Nessun test su device reale/Safari, nessun overlay utente, nessun Lighthouse. Il detector è statico: legge ~6 MB mentre la pagina live mostra 5,8 MB; le righe sono tutte 0. Il rischio SmartScreen è reale: l’installer 0.10.3 è `NotSigned`. Non testati: 200% zoom, screen reader, comportamento reale dell’installer.
