---
target: "src/app.rs#settings_page"
total_score: 25
max_score: 40
na_heuristics: 
p0_count: 0
p1_count: 3
target_identity: "file:C:\\Users\\emanu\\Desktop\\test\\cinematch\\src\\app.rs#settings_page"
timestamp: 2026-10-29T18-19-17Z
slug: src-app-rs-settings-page
---
# Critique — Impostazioni (CineMatch, Operate)

Target: `src/app.rs` → `settings_page()` (v0.10.10) and the rendered 2000×1032 screenshot of the same build. Native egui surface: `impeccable detect` and the browser overlay read web markup, so the detector/browser assessment is not applicable. The surface is a Windows desktop window, not a page.

## Design Health Score

| # | Heuristic | Score | Key Issue |
|---|-----------|-------|-----------|
| 1 | Visibility of System Status | 3 | Sync counts, update states, busy dots, and live credential parsing are clear; there is no dirty/unsaved indicator, and the status bar keeps showing the saved provider while the form shows another. |
| 2 | Match System / Real World | 3 | The Italian voice is plain and specific; "Read Access Token v4 (Bearer)", "Client ID", "JWT" and "interprete" still require prior knowledge, and the lede misstates what stays on the PC. |
| 3 | User Control and Freedom | 2 | Esc/Chiudi/rail all leave the page, and SIMKL auth can be cancelled; but closing neither applies nor saves the draft, and there is no undo for disconnect. |
| 4 | Consistency and Standards | 2 | Three header systems for peer sections ("TRACCIAMENTO" ivory mono, "TMDB" ember mono, "Interprete" Fraunces 24), one off-system update button, and a page mixing immediate actions with draft-only fields. |
| 5 | Error Prevention | 2 | The credential field recognises formats before any request — excellent; but disconnect has no confirmation, close has no dirty guard, and provider selection looks applied before Salva. |
| 6 | Recognition Rather Than Recall | 3 | Labels and hints are visible and the provider list is short; but the verification action sits several sections below the key, Save is under the fold, and the stored masked key cannot be identified. |
| 7 | Flexibility and Efficiency | 2 | No Ctrl+S, no section jump, no quick path to TMDB or the provider; a power user must Tab and scroll through the whole form to commit a one-field edit. |
| 8 | Aesthetic and Minimalist Design | 3 | Calm, flat, and typographically confident at the top; the page becomes a long undifferentiated form with repeated version/build/config noise at the bottom. |
| 9 | Error Recovery | 3 | Credential errors are specific and actionable, and update/SIMKL failures are shown in context; long API errors are truncated, with no expansion or retry affordance. |
| 10 | Help and Documentation | 2 | Inline credential help and the IMDb tooltip are good; nothing explains what the four providers are, what they cost, or what "Interprete delle richieste" means for the film results. |
| **Total** | | **25/40 (62.5%)** | **Acceptable** |

## Design Specificity Verdict

**Autoriale nella pelle, generica nella struttura.** The shell is unmistakably LUMEN: ink, ivory, one ember command, hairlines, mono metadata. The settings body is a 760px single-column form that could be lifted into any dark desktop utility. The two moments that could only be CineMatch are the credential diagnosis (`describe_credential`) and the SIMKL ledger line "81 VISTI · 22 DA VEDERE · 3 min fa". The composition does not use the product's "projection booth console" idea: no fixed status/source strip, no relationship between a field and the action that validates it, no sense of a machine being configured.

**Deterministic scan:** not applicable (native egui surface). **Browser overlays:** not applicable; there is no page to inject into.

## Overall Impression

Calm, legible, and honest in the credential field. Then it becomes a long form whose commit action is invisible, whose destructive action is disguised as a footnote, and whose first ember command is "Sincronizza ora" rather than anything that saves the user's work. The single biggest opportunity is to make the page state its model plainly: what is live now, what is a draft, and what leaves this PC.

## What's Working

1. **The credential parser** (`src/tmdb/client.rs:87-123`): it names the format, catches a truncated JWT, and explains the v3 fallback derived from a bearer token, in one line under the field. This prevents the most common setup error before a network call and sounds like the product: precise, honest, no jargon dump.
2. **The SIMKL ledger** (`app.rs:4754-4766`): counts of watched/watchlist plus a relative timestamp, set in mono before the destructive link. It answers "is my library current?" at a glance and uses the same ledger voice as the rest of LUMEN.
3. **The LUMEN page header**: ember kicker, Fraunces title, one muted lede, a quiet mono close, one hairline. It proves the system can carry an operational form without cards, shadows, or decoration; the restraint is right for Operate.

## Priority Issues

1. **[P1] The page does not say what is already applied and what is only a draft.**
   - **What:** SIMKL sync/connect/disconnect, update check/download/install, model detection, the AI probe, and the IMDb download act immediately. The TMDB key, provider, language, and "Nascondi i film già visti" apply only on "Salva", which lives at the bottom after every section. Selecting another provider starts background model detection while the status bar and chat still use the old provider. Closing with "Chiudi ×", Esc, or the rail hides the page without saving or applying; reopening shows the edited draft, so it looks saved. The draft survives only until the app quits.
   - **Why:** users believe a key or provider is active when the app is still using the old one, and they can lose the edit on quit. The status bar actively reinforces the wrong belief.
   - **Fix:** pick one model. Either apply valid changes immediately with an undo, or mark the form "Modifiche non salvate", add a sticky footer with Salva/Annulla, and confirm on close. Make the status bar show the draft provider or label it "salvato". Label instant actions as actions. Move "Verifica credenziale" under the TMDB field or into the sticky bar.
   - **Suggested command:** `/impeccable harden`

2. **[P1] Custom controls and password fields have no accessible name.**
   - **What:** `lumen::button_sized` ("Sincronizza ora", "Connetti SIMKL", "Rileva modelli", "Verifica risposta AI", "Scarica i voti IMDb", "Salva", "Verifica credenziale") and `theme::primary_button` ("Scarica e installa") paint their own text and never call `widget_info`. The TMDB/DeepSeek/OpenRouter text fields have no `hint_text` or label association; the visible label above them is a separate text node.
   - **Why:** a screen-reader user meets unnamed buttons and unnamed password fields. Keyboard focus exists, so this is specifically an AccessKit gap, and it blocks the whole screen for that user.
   - **Fix:** add `WidgetInfo::labeled(Role::Button, …)` to every custom button, including the disabled state; associate each field with its visible label (or a real hint) so it is announced. Verify the page once with AccessKit/NVDA, not only by Tab.
   - **Suggested command:** `/impeccable audit`

3. **[P1] "Scollega SIMKL" wipes the local library with one click.**
   - **What:** the 10px mono text link immediately clears the token and `simkl_store` (visti, watchlist, votes), saves the empty store, and reports "la copia locale è stata rimossa" (`app.rs:4768`, `disconnect_simkl`). No confirmation, no undo.
   - **Why:** a misclick on a footnote-sized link destroys the local tracking history. The data can be restored only by reconnecting and resyncing, which the UI never says. It undermines the "onesto" promise exactly when data is at risk.
   - **Fix:** confirm before disconnecting with the two consequences ("Scollega e conserva la copia locale" / "Scollega e rimuovi anche la copia locale"), keep the local copy by default, or provide an undo toast. Move the link out of the status row into a small danger section with a 36px control.
   - **Suggested command:** `/impeccable harden`

4. **[P2] Peer sections use three different header systems, and one button breaks the control recipe.**
   - **What:** "TRACCIAMENTO" and "AGGIORNAMENTI" are ivory mono 10; "TMDB" is ember mono 10; "Interprete delle richieste" is Fraunces 24; field labels are sans 11. "Controlla" is a 27px sans sentence-case button with 9px corners (`app.rs:4823-4827`), while every other button is 36px mono uppercase at 2px. The screenshot makes the mismatch obvious next to the ember sync button. The hairline above TMDB also has no space after it (`app.rs:4930-4932`).
   - **Why:** the eye cannot map which command belongs to which block, and the page reads as a form with three competing levels rather than one console. The 9px radius is outside LUMEN's allowed shape range and is the first place the system looks broken.
   - **Fix:** one section header recipe for all peers (mono 10 uppercase ivory; reserve ember for the page kicker, selection, and focus); render "Controlla" with the 36px LUMEN secondary button; align field labels to the mono label style; add the missing section spacing.
   - **Suggested command:** `/impeccable layout`

5. **[P2] The privacy sentence contradicts the SIMKL behavior.**
   - **What:** the lede says "Chiavi, interprete, tracciamento e aggiornamenti restano su questo PC", while the first section syncs watch history with SIMKL and disconnect removes the local copy.
   - **Why:** this is the first trust signal on a settings screen. A user who reads it literally will be surprised that visti/list/voti leave the machine. That is a product-truth problem, not a tone nit.
   - **Fix:** "Chiavi e preferenze restano su questo PC. SIMKL sincronizza visti, lista e voti solo quando lo chiedi." State in the section what sync sends and what disconnect removes.
   - **Suggested command:** `/impeccable clarify`

## Persona Red Flags

**Alex (power user):** No Ctrl+S, no section jump, no direct route to the TMDB key. He changes the provider, sees model detection start, and the status bar still says OPENROUTER. To commit a one-field edit he tabs through roughly twenty controls or scrolls past the interpreter and local-data sections. Esc closes the page with no warning, and reopening shows the draft, which makes the missing apply easy to miss.

**Jordan (first-timer):** "Tracciamento" sounds like analytics; the lede says tracking stays on the PC, then the page offers a SIMKL sync. "Read Access Token v4 (Bearer)", "Client ID" and "JWT" are unexplained. The key field is a row of dots, so he cannot tell which credential is stored. "Scollega SIMKL" looks like a harmless text link. After editing and closing, the form still shows his change, so he assumes it saved.

**Sam (accessibility-dependent):** The custom buttons are unlabeled in AccessKit, and the TMDB/DeepSeek/OpenRouter fields have no accessible name. The focus ring is drawn, so the failure is silent for sighted testers. The status changes ("Sincronizzo la libreria…", "✔ chiave riconosciuta") are plain labels with no live-region semantics. Disconnecting is a text-only control with no confirmation and no focus restoration.

## Minor Observations

- The masked TMDB field has no reveal, last-4, clear, or copy affordance; a user cannot confirm which key is stored before overwriting it.
- "Rivedi il primo avvio" sits inside the TMDB block (`app.rs:4974`) and leaves the page; it reads as a TMDB action and abandons unsaved work.
- "Sincronizza ora" is passed `primary = !self.simkl_syncing` (`app.rs:4693-4697`), so it becomes a secondary-looking button while syncing and stays clickable instead of showing a disabled/busy state.
- `settings_check` is truncated at 150 characters (`app.rs:4963`); a TMDB error can be cut with no way to expand it.
- The update section shows "Sei alla versione più recente." with no last-check time; "Controlla" has no post-check timestamp.
- The page displays the version three times (rail, "CineMatch 0.10.10", status bar) plus the build hash and the full config path; the footer ends on developer noise rather than a confirmation.
- The English TMDB legal line is the only English copy and one of the last things on the page; it is required, but it could sit with the build metadata and be more readable.
- At the 800×600 minimum, the 760px column gets no centering pad inside the 14px panel margin; the config path in the action row may wrap awkwardly (inferred from code, not seen live).
- The field labels above the AI inputs use 11px sans, while the system reserves 10px mono for labels; "LINGUA" is correctly mono, so the page mixes both.
- The "Controlla" button is the only default-egui button in the page; all others are custom-painted, which is also why only it is announced by screen readers.

## Questions to Consider

1. If the app can save keys and sync a library, why is the one action that commits the form the only thing not visible while editing?
2. What would this page look like as a console rather than a form: a fixed status strip, a list of sources, one command at a time, no scroll?
3. Which of these are settings and which are account actions? Should disconnect, update, and sync share a screen with credential editing at all?

## Coverage / Limitations

Reviewed the current `settings_page` source (`src/app.rs:4585-5334`) and the parent-provided 2000×1032 rendered screenshot of the same v0.10.10 build. I did not drive the live window: hover, focus, dirty-state, small-window, and screen-reader behavior are inferred from code. Native surface, so no `impeccable detect` and no browser overlay. The page below TMDB (Interprete, Lingua, Dati locali, Salva) is judged from source, not from the screenshot.
