---
name: FlowTrace
description: The situation report for an organisation's own money, every movement drawn with its proof state.
colors:
  report-blue: "oklch(0.48 0.13 255)"
  report-blue-ink: "oklch(0.99 0.004 255)"
  coverage-blue: "oklch(0.71 0.08 255)"
  signal-orange: "oklch(0.6 0.18 40)"
  rejection-red: "oklch(0.55 0.19 35)"
  report-paper: "oklch(0.972 0.004 120)"
  sheet: "oklch(0.993 0.002 120)"
  ink-navy: "oklch(0.24 0.035 250)"
  ink-muted: "oklch(0.45 0.03 250)"
  gap-grey: "oklch(0.89 0.012 250)"
  hairline: "oklch(0.86 0.014 250)"
  field-stroke: "oklch(0.8 0.018 250)"
  wash: "oklch(0.94 0.006 250)"
  rail: "oklch(0.948 0.007 250)"
  rail-hover: "oklch(0.9 0.02 255)"
  masthead-ink: "oklch(0.24 0.035 250)"
  masthead-paper: "oklch(0.96 0.006 250)"
  report-blue-dark: "oklch(0.72 0.11 255)"
  report-blue-ink-dark: "oklch(0.17 0.03 255)"
  coverage-blue-dark: "oklch(0.52 0.08 255)"
  signal-orange-dark: "oklch(0.72 0.16 45)"
  rejection-red-dark: "oklch(0.68 0.17 35)"
  report-paper-dark: "oklch(0.185 0.02 250)"
  sheet-dark: "oklch(0.215 0.022 250)"
  ink-navy-dark: "oklch(0.94 0.008 250)"
  ink-muted-dark: "oklch(0.72 0.02 250)"
  gap-grey-dark: "oklch(0.31 0.02 250)"
  hairline-dark: "oklch(1 0 0 / 13%)"
  field-stroke-dark: "oklch(1 0 0 / 20%)"
  wash-dark: "oklch(0.25 0.02 250)"
  rail-dark: "oklch(0.165 0.02 250)"
  masthead-ink-dark: "oklch(0.13 0.02 250)"
typography:
  display:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "clamp(3.25rem, 7vw, 5.25rem)"
    fontWeight: 750
    lineHeight: 0.95
    letterSpacing: "-0.01em"
    fontVariation: "'wdth' 78"
  headline:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "3rem"
    fontWeight: 750
    lineHeight: 1
    letterSpacing: "-0.01em"
    fontVariation: "'wdth' 78"
  title:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "1.5rem"
    fontWeight: 750
    lineHeight: 1.25
    letterSpacing: "-0.01em"
    fontVariation: "'wdth' 78"
  figure:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "2.75rem"
    fontWeight: 800
    lineHeight: 1
    letterSpacing: "-0.01em"
    fontFeature: "'tnum'"
    fontVariation: "'wdth' 75"
  body:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "0.875rem"
    fontWeight: 400
    lineHeight: 1.625
    fontFeature: "'tnum'"
    fontVariation: "'wdth' 100"
  lede:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "1.125rem"
    fontWeight: 400
    lineHeight: 1.625
    fontVariation: "'wdth' 100"
  section-label:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "0.75rem"
    fontWeight: 700
    lineHeight: 1rem
    letterSpacing: "0.14em"
    fontVariation: "'wdth' 100"
  label:
    fontFamily: "Archivo Variable, Arial Narrow, system-ui, sans-serif"
    fontSize: "0.6875rem"
    fontWeight: 700
    letterSpacing: "0.08em"
  mono:
    fontFamily: "IBM Plex Mono, ui-monospace, monospace"
    fontSize: "0.8125rem"
    fontWeight: 400
rounded:
  none: "0px"
  hairline: "2px"
spacing:
  cell: "16px"
  gutter-mobile: "16px"
  gutter: "32px"
  section-app: "32px"
  section-landing: "80px"
  rule-heavy: "3px"
components:
  button-primary:
    backgroundColor: "{colors.report-blue}"
    textColor: "{colors.report-blue-ink}"
    rounded: "{rounded.none}"
    padding: "0 16px"
    height: "40px"
    typography: "{typography.body}"
  button-primary-lg:
    backgroundColor: "{colors.report-blue}"
    textColor: "{colors.report-blue-ink}"
    rounded: "{rounded.none}"
    padding: "0 20px"
    height: "44px"
  button-outline:
    backgroundColor: "transparent"
    textColor: "{colors.ink-navy}"
    rounded: "{rounded.none}"
    padding: "0 16px"
    height: "40px"
  button-ghost:
    backgroundColor: "transparent"
    textColor: "{colors.ink-navy}"
    rounded: "{rounded.none}"
    height: "40px"
  input:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink-navy}"
    rounded: "{rounded.none}"
    padding: "4px 12px"
    height: "36px"
  badge:
    backgroundColor: "{colors.report-blue}"
    textColor: "{colors.report-blue-ink}"
    rounded: "{rounded.none}"
    padding: "0 6px"
    height: "20px"
    typography: "{typography.label}"
  demo-tag:
    backgroundColor: "transparent"
    textColor: "{colors.ink-muted}"
    rounded: "{rounded.none}"
    padding: "0 6px"
    height: "20px"
  card:
    backgroundColor: "{colors.sheet}"
    textColor: "{colors.ink-navy}"
    rounded: "{rounded.none}"
    padding: "16px"
  masthead:
    backgroundColor: "{colors.masthead-ink}"
    textColor: "{colors.masthead-paper}"
    height: "56px"
    padding: "0 20px"
  nav-item:
    backgroundColor: "transparent"
    textColor: "{colors.ink-navy}"
    rounded: "{rounded.none}"
    height: "40px"
    padding: "0 12px"
  nav-item-active:
    backgroundColor: "{colors.ink-navy}"
    textColor: "{colors.report-paper}"
    rounded: "{rounded.none}"
    height: "40px"
  coverage-bar:
    backgroundColor: "{colors.gap-grey}"
    height: "20px"
    rounded: "{rounded.none}"
---

# Design System: FlowTrace

## Overview

**Creative North Star: "The Situation Report"**

FlowTrace is set like the sitrep an NGO leader already knows how to read: cool report paper, ink navy, one report blue, and a strict grid of sections that each open on a heavy ink rule. Figures are the loudest thing on any page. They are set condensed, heavy and tabular, and their size carries their magnitude. Prose stays at normal width so it reads as prose.

The system's own invention is that **verification state is drawn by line pattern, never by colour alone**: a solid line is evidenced, a dashed line is awaiting proof, a long dash crossed with an X is flagged, a dotted line is rejected. That grammar runs from the wordmark through the landing coverage bars to every edge of the flow map, where line weight carries the amount. Colour confirms the pattern; it never carries the meaning by itself.

Density is report density: hairlines divide figures, nothing floats, nothing is rounded, nothing glows. Depth comes from the ink masthead and from rules, not from shadows.

**Key Characteristics:**
- Report paper and ink navy, one report blue, signal orange held back for flags.
- Archivo Variable on its width axis: condensed and heavy for headlines and figures, normal width for prose and section labels.
- Tabular numerals on everything (set on `body`).
- 3px ink rule opens every section; 1px hairlines divide inside it.
- Square corners everywhere.
- Proof state as line pattern; amount as line weight; selection isolates.

## Colors

A cool, low-chroma paper-and-ink palette with a single saturated blue and one reserved signal orange. Values live in the frontmatter in OKLCH, which is canonical in `flowtrace/src/styles.css`; each light token has a `-dark` sibling applied under `.dark`.

### Primary
- **Report Blue** (`report-blue`, dark `report-blue-dark`): the one action and "evidenced" colour. Primary buttons, the evidenced share of a coverage bar, checked checkboxes, the focus ring, text selection, caret, the demo banner. `--success` and `--chart-1` are aliased to it, so "verified" and "primary" are the same blue.
- **Coverage Blue** (`coverage-blue`): the lighter report blue for received-but-not-yet-evidenced coverage, and the selected-edge halo and selected-node fill on the flow map.

### Secondary
- **Signal Orange** (`signal-orange`, dark `signal-orange-dark`): flags only. Flagged lines and their cross, the "No recipient recorded" node stroke, flagged status text. `--warning` and `--chart-5` share this hue.
- **Rejection Red** (`rejection-red`, dark `rejection-red-dark`): rejected proof (dotted line), destructive actions, field errors. Close in hue to Signal Orange, which is why the pattern, not the colour, has to carry the difference.

### Neutral
- **Report Paper** (`report-paper`): the page background, a cool off-white with the faintest green cast.
- **Sheet** (`sheet`): cards, inputs, dialogs, flow-map nodes; one step brighter than paper.
- **Ink Navy** (`ink-navy`): all text, every heavy rule, the inverted active nav item, verified line strokes.
- **Ink Muted** (`ink-muted`): secondary text, labels under figures, awaiting-proof strokes, the Demo data tag.
- **Gap Grey** (`gap-grey`): the unfunded remainder of a coverage bar; the skeleton shimmer's midpoint.
- **Hairline** (`hairline`): 1px dividers between figures, rows and section headers.
- **Field Stroke** (`field-stroke`): input and textarea borders.
- **Wash** (`wash`): hover backgrounds for ghost buttons and list rows.
- **Rail / Rail Hover** (`rail`, `rail-hover`): the desktop nav rail and its hover state.
- **Masthead Ink / Masthead Paper** (`masthead-ink`, `masthead-paper`): the app masthead band and the landing's audience section. In dark mode the masthead drops below the page (`masthead-ink-dark`) so it still reads as the darkest band.

### Named Rules
**The One Blue Rule.** Report Blue is the only saturated colour in ordinary use. It means "act here" or "this has proof". A second accent is not added.

**The Orange Is a Flag Rule.** Signal Orange appears only where something is flagged. It is never decoration, never a highlight, never a hover.

**The Pattern Before Colour Rule.** Any verification state must stay legible in greyscale. If removing colour makes two states identical, the drawing is wrong.

## Typography

**Display Font:** Archivo Variable, width axis 75–78% (with Arial Narrow, system-ui)
**Body Font:** Archivo Variable at 100% width (same family)
**Label/Mono Font:** IBM Plex Mono 400/500, for identifiers and codes only

**Character:** One grotesque used at two widths. Condensed and heavy, it is the voice of headlines and figures; at normal width it is a plain, legible reading face. The width shift, not a second family, is what separates display from prose.

### Hierarchy
- **Display** (750, `wdth` 78, 3.25rem → 4.5rem → 5.25rem, line-height 0.95): the landing's single h1.
- **Headline** (750, `wdth` 78, 2.25rem → 3rem, line-height 1): app page titles ("Money flow", "Trust Score") and landing section heads.
- **Title** (750, `wdth` 78, 1.5–1.875rem): dialog titles, detail-panel headings, trace step names.
- **Figure** (800, `wdth` 75, tabular, line-height 1): key figures. Three scales: sm 1.875rem, md 2.25rem → 2.75rem, lg 3rem → 3.75rem. The row's most important number takes `lg`.
- **Body** (400, 0.875rem, line-height 1.625): app prose and table cells. Ledes are 1.125rem with a 34rem / `max-w-xl` measure.
- **Section label** (700, 0.75rem, uppercase, tracking 0.14em, `wdth` 100): the title of a report section, sitting on the heavy rule. It is the heading of that section, not a label above another heading.
- **Label** (700, 0.6875rem, uppercase, tracking 0.08em): badges, chain-link state words, table column heads (0.75rem, 0.12em).

All `h1`–`h3` get the condensed display treatment and `text-wrap: balance` from the base layer. A section heading that should be a section label opts back to normal width.

### Named Rules
**The Size Is Magnitude Rule.** In a row of figures, the largest number is set largest. Type size reports importance; it is not chosen for balance.

**The Tabular Everywhere Rule.** Every figure, amount and date column uses tabular numerals so columns align. This is set on `body`; do not switch it off.

**The Two Widths Rule.** Condensed is for headlines and figures. Prose, labels and controls stay at normal width.

## Layout

App pages sit in a `max-w-6xl` column with 16px side gutters on mobile and 32px from `sm`, and 24–32px vertical padding. The landing uses `max-w-7xl` on a 12-column grid (headline 5/12, report 7/12) with 64–80px section padding.

The rhythm is set by rules, not boxes. Each section opens with a 3px ink rule, then a 44px-minimum header row (section label left, meta and actions right) closed by a hairline. Inside, key figures sit in a 4-up row (2-up on mobile) divided by vertical hairlines with 16px cell padding. Lists are ruled rows with hairlines between them, not separate cards.

The shell is a 56px sticky masthead, an optional 34px demo banner beneath it, and a 224px desktop nav rail that collapses to 64px. Under `md` the rail is replaced by a right-hand drawer.

### Named Rules
**The Rule Opens the Section Rule.** Every section starts on a 3px ink rule. Hairlines divide the content inside it. Do not add a box around something a rule already contains.

## Elevation & Depth

Flat by default. Depth comes from the ink masthead, the heavy rules and the one-step brightness difference between paper and sheet. Ordinary surfaces carry no shadow.

### Shadow Vocabulary
- **Overlay lift** (`0 12px 40px -12px oklch(0.24 0.035 250 / 0.35)`): the report dialog only.
- **Drawer edge** (`-8px 0 24px -12px rgb(0 0 0 / 0.35)`, and its upward twin on the mobile flow details sheet): surfaces that slide over the page.
- **Menu** (Tailwind `shadow-md` / `shadow-lg`): dropdown, select and combobox popups.

### Named Rules
**The Overlays Only Rule.** Shadows exist only on things that float over the page: dialogs, drawers, menus. A card, section or button never gets one.

## Shapes

Square. `--radius` is 2px, and because the small radius steps are computed as `--radius` minus 2–4px, `rounded-sm` and `rounded-md` resolve to 0. Buttons, badges, inputs, cards, nav items, checkboxes, flow-map nodes and the avatar block are all square-cornered. The recurring geometry is the line and the small square: the wordmark is three square stations on one line, trace steps are 14px squares with 3px ink borders, and flow-map nodes are rectangles.

Strokes have a fixed set of weights: 3px for heavy rules and status lines, 1.5px for node outlines (3px when selected), 1px for hairlines. Flow edges run 2–16px.

### Named Rules
**The Square Corner Rule.** Corners are square (0px; 2px at most). Pills and rounded cards are not part of this world.

## Components

### Buttons
Plain, square and confident.
- **Shape:** square (0px).
- **Primary:** Report Blue fill, near-white text, 600 weight, 40px tall with 16px padding; `lg` is 44px with 20px padding; `sm` is 32px. Hover drops to 88% opacity of the blue, active to 80%. Transition: colours, 150ms.
- **Focus:** a 2px ring in Report Blue at 50% opacity, plus the global 2px outline with 2px offset.
- **Outline:** a 70% ink border on a transparent fill. It is meant to invert to ink on hover (see drift note).
- **Ghost:** no fill; Wash on hover. Used on the masthead (with masthead-paper at 12% for hover) and for icon buttons.
- **Link:** Report Blue text, underlined on hover. On the landing, quiet text links use an ink underline at 30–40% that goes to full ink on hover.
- A trailing Phosphor arrow nudges 2px right on hover on the landing CTAs.

### Badges and the Demo data tag
- **Badge:** 20px tall, square, 0.6875rem bold uppercase with 0.08em tracking. Default is Report Blue; outline and secondary are the quiet forms.
- **Demo data tag:** a 1px hairline box with "DEMO DATA" in muted 600-weight, 0.12em tracking. It is required on every block of figures drawn from the bundled demo dataset and sits at the right of that section's header row.

### Cards / Containers
- **ReportSection** is the main container: the heavy rule, then a header row with a section label, then content. Use it for any titled block in the app.
- **Card** is a bordered variant for content that has to stand apart: Sheet background, 1px hairline border with a 3px ink top border, square corners, 16px padding (12px at `sm`). Its title is the section-label style. No shadow.
- **ReportNotice** handles empty and error states in the report's own voice: centred, 40px vertical padding, a 600-weight title, a muted line at a 28rem measure, and an optional action. Errors switch the title to Rejection Red and set `role="alert"`.

### Key figures
`KeyFigures` is a 2-up (mobile) or 4-up grid divided by hairlines. `KeyFigure` is a figure-style number, then a muted 0.875rem label, then an optional 0.75rem detail line. Use `scale="lg"` on the one figure that matters most.

### Coverage bar
A 20px bar on Gap Grey. Coverage Blue shows the received share and Report Blue the evidenced share, drawn on top. Pending money is not drawn as a bar: it is a 3px dashed muted line (dash 8, gap 6) across the full width. Each bar has `role="img"` and a spoken label.

### Status marks (the line-pattern grammar)
- **StatusLine:** a 3px SVG line sample. Evidenced is solid ink. Awaiting proof is muted, dashed 6/5. Flagged is Signal Orange, a long dash 14/8 with a 2px X at the midpoint. Rejected is Rejection Red, dotted 2/6.
- **StatusMark:** a 22px StatusLine plus its label in 0.75rem semibold, with the label tinted to match the state. Use it wherever a verification status is shown, in place of a coloured chip.
- **ProofLegend:** the key for evidenced, awaiting proof and flagged. It sits under any drawing that uses the grammar.
- The labels are fixed: Evidenced, Awaiting proof, Flagged, Rejected.

### Inputs / Fields
- **Style:** a 36px input on Sheet with a 1px Field Stroke border, square corners and 12px horizontal padding. The textarea matches with a 64px minimum height. Select and date-field share the frame; the date field adds a 40px calendar icon button at the right and tabular numerals.
- **Focus:** the border turns Report Blue with a 1px ring at 50%.
- **Error / Disabled:** `aria-invalid` turns the border Rejection Red with a 1px ring. Disabled is 50% opacity. `FormField` sets the label at 0.875rem semibold with "(required)" in muted, and the error (0.75rem semibold red) or hint underneath, wired to the control through `aria-describedby`.
- **Checkbox:** a 20px square with a 1.5–2px ink border at 70%. When checked it fills Report Blue with a bold check.

### Dialog
`ReportDialog` is a centred sheet up to 32rem wide with a 3px ink top rule and a condensed title row closed by a hairline. The body scrolls; the footer of actions sits on a hairline, right-aligned from `sm` and stacked on mobile. The backdrop is ink at 40%. It enters with 150ms opacity plus an 8px rise. Focus trap, Escape to close and focus return come from Base UI.

### Navigation
- **Masthead:** a 56px Masthead Ink band holding the wordmark (FlowTrace mark plus "FlowTrace" at 800 weight, `wdth` 87), a 1px divider, the organisation name, then the theme toggle, the account menu (a square initials block in inverse colours) and the mobile menu trigger. In demo mode a Report Blue banner sits directly beneath it.
- **Rail:** a 224px rail on Rail grey with 40px items (0.875rem medium, 18px Phosphor icon). Hover is Rail Hover. **Active is inverted**: an ink block with paper text and a bold icon. The rail collapses to 64px icons.
- **Mobile drawer:** a 288px panel from the right with a 3px ink left edge, a "MENU" section label, and 48px items using the same inverted active state. The backdrop is ink at 40%. It slides in over 300ms.
- **Landing masthead:** a paper band closed by a 3px ink rule, carrying the wordmark, a tracked uppercase dateline ("Situation report · Demo NGO Kenya · as of 4 Feb 2026"), text links and a small primary "Explore the demo" button.

### Flow map (signature)
An SVG map with columns of 196px square nodes (Sheet fill, 1.5px ink stroke) joined by edges. **Line weight is amount**: edges run 2–16px on a square-root scale. **Line pattern is proof state**, using the StatusLine grammar, with dashes scaled to the line's width so thick lines still read as dashed. Unassigned money gets a Signal Orange dashed node.

**Select to isolate:** selecting or focusing a node lights everything upstream and downstream of it, and selecting an edge lights that edge; everything else drops to 12% opacity over 200ms. A selected edge gets a Coverage Blue halo and an ink amount tag. Every node and edge is a keyboard `button`, and Tab follows the money left to right. The landing's donor rows use the same isolation: hover or focus dims the other rows to 30%.

### Motion
There is one authored moment, isolation: a 200ms opacity fade when selecting a flow or a donor row. Everything else is functional. Pages fade up 4px over 350ms on `cubic-bezier(0.16, 1, 0.3, 1)`, colours transition over 150ms, the drawer slides over 300ms, and skeletons shimmer. `prefers-reduced-motion` turns off page entry and the shimmer, and the flow map's transitions use `motion-reduce:transition-none`.

## Do's and Don'ts

### Do:
- **Do** open every section with the 3px ink rule and a section-label header row.
- **Do** draw verification state with the line pattern: solid for evidenced, dashed 6/5 for awaiting proof, a long dash 14/8 plus an X in Signal Orange for flagged, dotted 2/6 for rejected. Pair it with its label through StatusMark.
- **Do** let line weight carry the amount, and make selection isolate (dim the rest to 12%).
- **Do** set figures condensed, heavy and tabular, with the most important one set largest.
- **Do** tag every block of figures from the bundled dataset with the Demo data tag, and date figures with "as of".
- **Do** keep corners square and show the active nav item as an inverted ink block.
- **Do** respect `prefers-reduced-motion` on any new motion.

### Don't:
- **Don't** put an eyebrow or kicker above a heading. The section label is the heading, never a label stacked over another heading.
- **Don't** use gradients, glass, backdrop blur or glow.
- **Don't** use coloured left borders or side stripes to signal state.
- **Don't** use pills, rounded cards or rounded buttons.
- **Don't** use emoji. Icons are Phosphor.
- **Don't** carry verification state by colour alone, or use Signal Orange for anything that is not a flag.
- **Don't** invent proof: no customer counts, volumes, testimonials or logos that the demo data does not contain.
- **Don't** put shadows on cards, sections or buttons. Shadows are for overlays only.
