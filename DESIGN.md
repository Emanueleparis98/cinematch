---
name: CineMatch Browser
description: A contemporary film journal for discovery, requests and naturally illustrated answers.
colors:
  cobalt: "#2457ed"
  cobalt-hover: "#1945cc"
  focus: "#2b60d4"
  graphite: "#172033"
  cool-paper: "#f6f8fc"
  white: "#fff"
  raised: "#edf2fb"
  line: "#dce3ee"
  muted: "#5c687c"
  good: "#117453"
  danger: "#b02b43"
  selected: "#eaf0ff"
  chip-bg: "#e6edff"
  chip-text: "#214bbf"
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
    backgroundColor: "{colors.white}"
    textColor: "{colors.graphite}"
    rounded: "{rounded.control}"
    padding: "10px 14px"
  button-text:
    backgroundColor: "transparent"
    textColor: "{colors.cobalt}"
    padding: "5px 3px"
  field:
    backgroundColor: "{colors.white}"
    textColor: "{colors.graphite}"
    rounded: "{rounded.control}"
    padding: "11px 12px"
  tab-selected:
    backgroundColor: "{colors.white}"
    textColor: "{colors.cobalt}"
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
---

# Design System: CineMatch Browser

## Overview

**Creative North Star: "The Contemporary Film Journal"**

A contemporary independent film journal: cool paper, graphite text and a restrained cobalt accent make the browser extension feel clear, precise and inviting. Manrope gives the compact navigation and editorial reading one consistent voice.

This system governs the browser panel, its expanded tab and the TMDB connection page. Inline media belong visually to the host chat and inherit its font and colors. The native Windows app and Rust CLI retain their legacy LUMEN styling outside this redesign; they are not a source of browser tokens.

**Key Characteristics:**
- Cool white and graphite with cobalt reserved for actions and selection.
- Flat chronological rows, clear dividers and compact task controls.
- Locally hosted Manrope for extension-owned surfaces.
- Transparent, unboxed media that inherit the host chat.

## Colors

The palette feels like cool paper and precise graphite, with a clear cobalt action color. The frontmatter is the normative color source.

### Primary

- **Cobalt:** Primary actions, links, selected navigation, selected checkbox accent and the text action variant.
- **Cobalt Hover:** The darker hover response for primary buttons.
- **Focus:** A clearly visible keyboard focus outline, offset from controls.

### Neutral

- **Cool Paper:** The extension page canvas.
- **White:** Header, inputs, selected tab and action dock.
- **Raised:** Neutral button hover surfaces.
- **Graphite:** Headings, body copy and the wordmark.
- **Muted:** Metadata, explanatory copy and secondary labels.
- **Line:** Hairline separators and control borders.
- **Selected:** The subtle wash behind selected editorial rows.
- **Chip Background / Chip Text:** Selected movement tokens in the request view.

The Good and Danger semantic colors distinguish success and failure alongside explicit status text. They are not secondary brand accents.

**The Action Accent Rule.** Use cobalt for primary actions, links, selection and focus. Reading surfaces remain cool and neutral.

## Typography

**Display and Body Font:** Manrope, with Arial and sans-serif fallbacks. The variable font is bundled locally; attribution and its SIL Open Font License are in `extensions/cinematch/assets/`.

**Character:** Compact geometric clarity, with strong sentence-case headings and readable, quiet supporting text. No separate decorative display face is used.

### Hierarchy

- **Headline:** The frontmatter headline role introduces a panel view. Below the narrow breakpoint it reduces to (27px).
- **Title:** The title role introduces editorial detail sections and connection-page sections.
- **Body:** The body role governs extension-owned text; explanatory intros use (13px / 1.7), while detail summaries use (15px / 1.85).
- **Label:** The label role keeps form labels compact. Navigation uses (12px / 750); row titles use (14px / 750 / 1.5). Status and row metadata settle at (11px) in the final overrides.
- **Caption:** The inherited caption role is for names below inline portraits, reducing to (11px) in narrow containers. Role labels use (11px / 1.4) with reduced opacity.

**The Native Answer Rule.** Inline media inherit the chat font and colors. Leave the original answer text and heading intact, with no enclosing card or duplicate title.

## Layout

The panel is a vertical viewport layout with a fixed header and segmented navigation, an independently scrolling main region and a compact action dock when composing. The body supports widths from (280px); default horizontal gutters are (20px). Main content uses a centered maximum width of (850px), with a (28px) top inset and (30px) bottom inset. Editorial rows use a checkbox column (24px), a (10px) gap and vertical padding (16px).

At viewport widths at or below (340px), gutters become (16px), filter fields stack into one column and quantity controls wrap. Expanded-tab pages use (30px) horizontal gutters and (40px) main vertical padding. At (760px) and above the expanded movement list becomes two columns with a (32px) gap. The connection page has a maximum width of (680px); its padding reduces and its actions stack at (360px) and below.

Inline media use their own container width. Above (430px), the poster is (108px) wide with a (24px) gap to people. Portraits are (76px × 100px), while caption columns are independently (104px) wide. At or below (430px), the poster becomes (78px), the main gap becomes (17px), portraits become (58px × 78px), and caption columns remain (94px). The people strip scrolls horizontally with proximity snapping instead of squeezing names to image width. Names wrap at normal spaces, without forced intra-word breaks. Media hosts use vertical margins (18px 0 24px).

## Elevation & Depth

The extension is flat by default: cool tonal surfaces, fine borders and generous text spacing establish hierarchy. Inline answer media have transparent backgrounds and no outer border or shadow.

### Shadow Vocabulary

- **Selected tab:** `0 2px 5px #2436500b`, a slight separation from the segmented navigation track.
- **Action dock:** `0 -5px 20px #24365004`, a very faint lift above the scrolling content.

**The Flat Reading Rule.** Separate editorial rows with spacing and fine rules. Reserve the existing soft shadows for the selected tab and the action dock.

## Shapes

Controls have gently curved corners from the rounded tokens. Segmented navigation uses a slightly broader outer curve, selected rows use a compact softened rectangle, and selection chips are small rounded rectangles rather than pills. The connection status indicator is a circular dot accompanied by text.

Image crops have stable proportions: posters use (2:3), portraits use fixed dimensions with top-biased face framing (`center 25%`). Poster corners use the control radius on wide layouts and tab radius on narrow ones; portraits use tab radius on wide layouts and compact radius on narrow ones. SVG line icons accompany navigation and header controls. Existing artwork remains distinct from runtime TMDB film imagery.

## Components

### Buttons

Confident but compact. Primary buttons use Cobalt and White, becoming Cobalt Hover on hover. Standard button padding is the frontmatter primitive; the full-width generation action uses (11px) padding. Secondary buttons are white with a Line border and a Raised hover surface. Text buttons are transparent, cobalt and compact. Disabled buttons reduce opacity to (0.45), with disabled hover remaining neutral. Keyboard focus uses an offset (3px) outline.

### Inputs / Fields

White fields use the control radius, a fine Line border and the field padding token. Textareas use (1.65) line height and can resize vertically. Search is visually prominent with (13px 14px) padding. The question field has a minimum height of (110px); the editable generated prompt has a minimum height of (230px). Labels remain visible above fields; placeholders supplement them.

### Navigation

Two equal-width tabs sit in a cool neutral track. The selected tab is white with cobalt text and the Selected tab shadow. Text, outlined SVG icons and a compact count badge communicate the state. Keep actual tab semantics and keyboard focus.

### Chips

Selected movements become small removable buttons using the selection-chip primitive, with a tiny inline SVG removal mark. Their text can wrap to remain inside the available width.

### Editorial Rows / Containers

Rows are flat and separated by Line. A native checkbox marks selection; the selected row receives the Selected wash with softened corners and loses its separator. The row title is a text button; metadata sits beneath it. Detail articles use readable text measures (up to 68ch for sections). Filter and advanced-preference groups use native disclosure elements bounded by rules. There is no generic card shell to inherit.

### Action Dock

A white footer holds provider choice, concise connection status, one full-width primary action and two secondary actions. Existing-draft choices appear inline under a separator. The dock can scroll vertically when its height would otherwise exceed the viewport allowance.

### Native Answer Media

Place a poster beside a strip of director and cast portraits below the existing answer heading. People are linked figures with names and role captions; a small TMDB source link follows the strip. Keep captions wider than images. No copied title, card chrome, manual matching UI, settings or per-answer actions are inserted. Media are omitted when no useful image exists. Link hover underlines names, and keyboard focus receives an offset (2px) blue outline.

Buttons transition background, border and text over (160ms ease). Media arrive over (180ms cubic-bezier(.22,1,.36,1)) with a small (3px) vertical movement and opacity change. Both effects respect reduced-motion preferences.

## Do's and Don'ts

### Do:
- **Do** use the browser tokens and locally hosted Manrope on extension-owned pages.
- **Do** let names wrap at spaces in caption columns wider than their portraits.
- **Do** keep keyboard focus visible and respect reduced-motion preferences.
- **Do** preserve native chat typography and omit unavailable or uncertain media quietly.

### Don't:
- **Don't** revive the legacy LUMEN palette or serif typography in browser surfaces.
- **Don't** inject controls, configuration forms or duplicate film headings into AI answers.
- **Don't** force person names to break inside words to match portrait width.
- **Don't** treat the panel’s light palette as a theme for the external chat.
