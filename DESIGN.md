---
name: CineMatch Browser
description: A dark contemporary film journal for discovery, requests and naturally illustrated answers.
colors:
  cobalt: "#315fd1"
  cobalt-hover: "#3c6ee1"
  accent: "#9cbbff"
  focus: "#91b5ff"
  deep: "#0c111b"
  surface: "#151d2a"
  raised: "#1d293b"
  text: "#eef3ff"
  white: "#fff"
  muted: "#a8b6cc"
  line: "#2c3b52"
  control-hover-line: "#537099"
  good: "#79dab4"
  danger: "#ff99ad"
  selected: "#1c2e4d"
  chip-bg: "#20375c"
  chip-text: "#c2d4ff"
  check-bg: "#111a28"
  check-line: "#52647e"
  check-selected-line: "#6e94f0"
  picker-hover: "#1b283b"
  picker-open: "#1b2a42"
  picker-open-line: "#83a8ff"
  popover: "#172235"
  popover-line: "#3d526f"
  option-active: "#2a3a56"
  option-selected: "#263f6a"
  option-text: "#dce7fa"
  option-selected-text: "#eef4ff"
typography:
  headline:
    fontFamily: "Manrope, Arial, sans-serif"
    fontSize: "30px"
    fontWeight: 800
    lineHeight: 1.14
    letterSpacing: "-0.035em"
  title:
    fontFamily: "Manrope, Arial, sans-serif"
    fontSize: "20px"
    fontWeight: 750
    lineHeight: 1.3
    letterSpacing: "-0.025em"
  body:
    fontFamily: "Manrope, Arial, sans-serif"
    fontSize: "14px"
    fontWeight: 400
    lineHeight: 1.55
  label:
    fontFamily: "Manrope, Arial, sans-serif"
    fontSize: "11px"
    fontWeight: 750
    lineHeight: 1.55
  caption:
    fontFamily: "inherit"
    fontSize: "12px"
    fontWeight: 550
    lineHeight: 1.45
rounded:
  checkbox: "6px"
  compact: "8px"
  tab: "9px"
  row: "10px"
  control: "11px"
  navigation: "13px"
spacing:
  xs: "4px"
  sm: "8px"
  control: "12px"
  md: "16px"
  gutter: "20px"
  lg: "24px"
  section: "28px"
components:
  button-primary:
    backgroundColor: "{colors.cobalt}"
    textColor: "{colors.white}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
  button-primary-hover:
    backgroundColor: "{colors.cobalt-hover}"
    textColor: "{colors.white}"
  button-secondary:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
  button-text:
    backgroundColor: "transparent"
    textColor: "{colors.accent}"
    padding: "5px 3px"
  field:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "11px 12px"
  tab-selected:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.accent}"
    rounded: "{rounded.tab}"
    padding: "9px 6px"
  selection-chip:
    backgroundColor: "{colors.chip-bg}"
    textColor: "{colors.chip-text}"
    rounded: "{rounded.compact}"
    padding: "6px 9px"
  movement-row-selected:
    backgroundColor: "{colors.selected}"
    rounded: "{rounded.row}"
    padding: "16px 10px"
  checkbox:
    backgroundColor: "{colors.check-bg}"
    rounded: "{rounded.checkbox}"
    width: "20px"
    height: "20px"
  checkbox-checked:
    backgroundColor: "{colors.cobalt}"
  picker-trigger:
    backgroundColor: "{colors.surface}"
    textColor: "{colors.text}"
    rounded: "{rounded.control}"
    padding: "11px 12px"
  picker-trigger-open:
    backgroundColor: "{colors.picker-open}"
  picker-menu:
    backgroundColor: "{colors.popover}"
    textColor: "{colors.text}"
    rounded: "{rounded.navigation}"
    padding: "6px"
  picker-option:
    backgroundColor: "transparent"
    textColor: "{colors.option-text}"
    rounded: "{rounded.compact}"
    padding: "10px 11px"
  picker-option-active:
    backgroundColor: "{colors.option-active}"
  picker-option-selected:
    backgroundColor: "{colors.option-selected}"
    textColor: "{colors.option-selected-text}"
---

# Design System: CineMatch Browser

## Overview

**Creative North Star: "The Contemporary Film Journal"**

A contemporary independent film journal: deep graphite, navy surfaces and restrained cobalt make the browser extension feel clear, precise and inviting. Pale blue links and selection details remain legible on the dark canvas. Manrope gives compact navigation and editorial reading one consistent voice.

This system governs the browser panel, its expanded tab and the TMDB connection page. These extension-owned surfaces are dark only. Inline media belong visually to the host chat and inherit its font and colors; the extension does not change the remote account theme. The native Windows app and Rust CLI retain their legacy LUMEN styling outside this redesign and are not sources of browser tokens.

**Key Characteristics:**

- Dark graphite and navy with cobalt actions and light-blue links.
- Flat chronological rows, clear dividers and compact task controls.
- Locally hosted Manrope for extension-owned surfaces.
- Styled native checkbox inputs and keyboard-accessible popover choices.
- Transparent, unboxed photographs immediately below the recognized title.

## Colors

The palette retains the contemporary journal identity while moving its paper ground to deep graphite and navy. The frontmatter is the normative color source; root properties live in panel.css and choice states in controls.css under extensions/cinematch/.

### Primary

- **Cobalt / Cobalt Hover:** Solid primary actions and checked checkbox fill, with a brighter primary-button hover response.
- **Pale Blue Accent / Focus:** Links, selected navigation and keyboard orientation. Focus remains distinct from action fill.

### Neutral

- **Deep:** The extension page canvas; native controls also declare a dark color scheme.
- **Surface / Raised:** Header, fields and dock; hover lift for secondary actions.
- **Text / Muted:** Main copy and supporting metadata. White supplies strong action contrast and checkbox ticks.
- **Line / Control Hover Line:** Separators, default borders and hover delineation.
- **Selected / Chip Background / Chip Text:** Row selection wash and removable movement tokens.
- **Check Background / Check Line / Check Selected Line:** Checkbox rest and checked boundaries.
- **Picker Hover / Picker Open / Picker Open Line:** Combobox trigger feedback, distinct from the floating list.
- **Popover / Popover Line:** Opaque floating choice surface and its boundary.
- **Option Active / Option Selected / Option Text / Option Selected Text:** Pointer or keyboard target, committed choice and readable option copy.

Good and Danger distinguish success and failure alongside explicit status text. They are semantic feedback rather than extra brand accents.

**The Dark Surface Rule.** Keep the panel, connection page and their menus dark. Inline media inherit the host chat theme.

**The Action Accent Rule.** Use cobalt for primary actions and checked controls; use pale blue for links, selected navigation and focus. Reading surfaces remain neutral.

## Typography

**Display and Body Font:** Manrope, with Arial and sans-serif fallbacks. The variable font is bundled locally; attribution and its SIL Open Font License are in `extensions/cinematch/assets/`.

**Character:** Compact geometric clarity, with strong sentence-case headings and readable, quiet supporting text. No separate decorative display face is used.

### Hierarchy

- **Headline:** The frontmatter headline role introduces a panel view. Below the narrow breakpoint it reduces to (27px).
- **Title:** The title role introduces editorial detail sections and connection-page sections.
- **Body:** The body role governs extension-owned text; explanatory intros use (13px / 1.7), while detail summaries use (15px / 1.85).
- **Label:** The label role keeps form labels compact. Navigation uses (12px / 750); row titles use (14px / 750 / 1.5). Status and row metadata settle at (11px) in the final overrides. Picker trigger text uses (13px / 500); options use (12px / 550 / 1.5).
- **Caption:** The inherited caption role is for names below inline portraits, reducing to (11px) in narrow containers. Role labels use (11px / 1.4) with reduced opacity.

**The Native Answer Rule.** Inline media inherit the chat font and colors. Leave the original answer text and heading intact, with no enclosing card or duplicate title.

## Layout

The panel is a vertical viewport layout with a fixed header and segmented navigation, an independently scrolling main region and a compact action dock when composing. The body supports widths from (280px); default horizontal gutters are (20px). Main content uses a centered maximum width of (850px), with a (28px) top inset and (30px) bottom inset. Editorial rows use a checkbox column (24px), a (10px) gap and vertical padding (16px).

At viewport widths at or below (340px), gutters become (16px), filter fields stack into one column and quantity controls wrap. Expanded-tab pages use (30px) horizontal gutters and (40px) main vertical padding. At (760px) and above the expanded movement list becomes two columns with a (32px) gap. The connection page has a maximum width of (680px); its padding reduces and its actions stack at (360px) and below.

Inline media use their own container width. Above (430px), the poster is (108px) wide with a (24px) gap to people. Portraits are (76px × 100px), while caption columns are independently (104px) wide. At or below (430px), the poster becomes (78px), the main gap becomes (17px), portraits become (58px × 78px), and caption columns remain (94px). The people strip scrolls horizontally with proximity snapping instead of squeezing names to image width. Names wrap at normal spaces, without forced intra-word breaks. Media hosts use vertical margins (12px 0 22px).

Floating picker menus stay within the viewport with side insets (12px), a maximum height of (320px) and a trigger gap (7px). They open above the trigger when that offers more usable room. Menu width follows the trigger with a preferred minimum of (220px), bounded by the viewport.

## Elevation & Depth

The extension is flat by default: dark tonal surfaces, fine borders and generous text spacing establish hierarchy. Inline answer media have transparent backgrounds and no outer border or shadow. Choice menus are opaque surfaces in the browser top layer, crossing the scrolling panel and dock without clipping.

### Shadow Vocabulary

- **Selected tab:** `0 2px 5px #00000020`, a slight separation from the segmented navigation track.
- **Action dock:** `0 -5px 20px #00000025`, a very faint lift above the scrolling content.
- **Picker menu:** `0 14px 40px #0007`, diffuse separation from the task beneath it.

**The Flat Reading Rule.** Separate editorial rows with spacing and fine rules. Reserve soft shadows for selected navigation, the action dock and floating choice menus.

## Shapes

Controls have gently curved corners from the rounded tokens. Checkboxes use compact softened squares; popover menus share the broader navigation curve. Segmented navigation uses a slightly broader outer curve, selected rows use a compact softened rectangle, and selection chips are small rounded rectangles rather than pills. The connection status indicator is a circular dot accompanied by text.

Image crops have stable proportions: posters use (2:3), portraits use fixed dimensions with top-biased face framing (`center 25%`). Poster corners use the control radius on wide layouts and tab radius on narrow ones; portraits use tab radius on wide layouts and compact radius on narrow ones. SVG line icons accompany navigation, header controls and choice pickers. Checkbox ticks use CSS strokes. Existing artwork remains distinct from runtime TMDB film imagery.

## Components

### Buttons

Confident but compact. Primary buttons use Cobalt and White, becoming Cobalt Hover on hover. Standard button padding is the frontmatter primitive; the full-width generation action uses (11px) padding. Secondary buttons use Surface with a Line border and a Raised hover surface. Text buttons are transparent, pale blue and compact. Disabled buttons reduce opacity to (0.45), with disabled hover remaining neutral. Keyboard focus uses a (3px) outline offset by (2px).

### Inputs / Fields

Dark Surface fields use the control radius, a fine Line border and the field padding token. Textareas use (1.65) line height and can resize vertically. Search is visually prominent with (13px 14px) padding. The question field has a minimum height of (110px); the editable generated prompt has a minimum height of (230px). Labels remain visible above fields; placeholders supplement them.

### Checkboxes

Native input semantics with custom contemporary drawing. A (20px) square uses a (1.5px) boundary, the checkbox radius and a dark well. Hover brightens the border. Checked state uses Cobalt, a lighter blue boundary and a white two-stroke tick; disabled state uses (0.4) opacity. Preserve visible keyboard focus. Fill and border transition over (140ms ease), and the tick scales over (130ms cubic-bezier(.2,.8,.2,1)).

### Choice Pickers

A visible combobox trigger replaces the operating-system menu surface while the hidden native select remains the form value. The trigger has a minimum height of (43px), a truncated value and an outlined SVG chevron that turns over (140ms) when expanded. Hover and expanded states use the named picker colors. The provider variant is compact with a minimum height of (35px), compact radius and (7px 9px) padding.

The floating listbox uses the menu primitive and a fine Popover Line boundary. Options have a minimum height of (39px), compact radius and a trailing outlined SVG check only for the committed choice. The active wash follows pointer or keyboard navigation; selected state has a separate wash and lighter text. Disabled options use (0.4) opacity. Arrow keys, Home, End and prefix typing move the active choice. Enter or Space commits it. Escape closes and restores trigger focus; Tab and outside pointer or focus close it. The trigger announces the active descendant.

### Navigation

Two equal-width tabs sit in a dark neutral track. The selected tab uses Surface with pale-blue text and the Selected tab shadow. Text, outlined SVG icons and a compact count badge communicate the state. Keep actual tab semantics and keyboard focus.

### Chips

Selected movements become small removable buttons using the selection-chip primitive, with a tiny inline SVG removal mark. Their text can wrap to remain inside the available width.

### Editorial Rows / Containers

Rows are flat and separated by Line. A custom-drawn native checkbox marks selection; the selected row receives the Selected wash with softened corners and loses its separator. The row title is a text button; metadata sits beneath it. Detail articles use readable text measures (up to 68ch for sections). Filter and advanced-preference groups use native disclosure elements bounded by rules. There is no generic card shell to inherit.

### Action Dock

A dark Surface footer holds provider choice, concise connection status, one full-width primary action and two secondary actions. Existing-draft choices appear inline under a separator. The dock can scroll vertically when its height would otherwise exceed the viewport allowance.

### Native Answer Media

Place a poster beside a strip of director and cast portraits immediately below the recognized title line, before the description. Semantic headings take precedence; a title line ending at a line break or strong/bold text also supports placement. The shadow host is a span so it remains valid inside a paragraph while its presentation is block-level. Preserve original title and prose nodes. People are linked figures with names and role captions; a small TMDB source link follows the strip. Keep captions wider than images. No copied title, card chrome, manual matching UI, settings or per-answer actions are inserted. Media are omitted when no useful image exists. Link hover underlines names, and keyboard focus receives a (2px) blue outline offset by (5px). The host chat owns the media font and colors, including when that chat uses a light theme.

Buttons transition background, border and text over (160ms ease). Choice menus arrive over (120ms ease-out). Media arrive over (180ms cubic-bezier(.22,1,.36,1)) with a small (3px) vertical movement and opacity change. All effects respect reduced-motion preferences.

Runtime photographs come from TMDB and retain their source link and asset attribution; they are not generated identity artwork. Bundled icon provenance and font licensing remain with the shipped assets. Source evidence: panel.css, controls.css, controls.js, options.css and enrich.js under extensions/cinematch/; surface direction: docs/design/cinematch-browser.md.

## Do's and Don'ts

### Do:

- **Do** use the browser dark tokens and locally hosted Manrope on extension-owned pages.
- **Do** place media immediately below the recognized title line and before the description.
- **Do** let names wrap at spaces in caption columns wider than their portraits.
- **Do** keep keyboard focus visible, preserve picker keyboard operation and respect reduced-motion preferences.
- **Do** preserve native chat typography and omit unavailable or uncertain media quietly.
- **Do** retain source attribution and provenance for imagery and bundled assets.

### Don't:

- **Don't** reintroduce a light extension theme or the legacy LUMEN palette and serif typography.
- **Don't** inject controls, configuration forms or duplicate film headings into AI answers.
- **Don't** force person names to break inside words to match portrait width.
- **Don't** force the external chat to adopt the panel theme.
- **Don't** replace the styled checkbox and popover choice surfaces with operating-system menu styling.
