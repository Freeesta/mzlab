# Adducts & losses: one page with the calculator

Design note for merging the «Calcolatrice» drawer tool into «Addotti e perdite» (the main tool keeps that name), with ESI+ and ESI− selectable together, one list style for adducts, neutral losses and contaminants, and a clear high / low resolution distinction. The page with its three tabs is the first PR; the spectrum and palette entry points to the Contaminants tab are the second (§7).

## 1. What is wrong today
- Two drawer tools answer the same question («what can this number be?») in two styles: the calculator shows collapsible groups under the keypad (losses as underlined links with «30.0 · +0.0 Da», the adduct table squeezed below and often off screen), the Adducts & losses panel shows tabs with their own field and their own controls.
- The Neutral losses tab has two polarity selectors (`#nl-pol`, a filter that allows both, and the `nlq` choices row, which allows only one); the Adducts tab allows only one polarity.
- «Cerca in Δm» is long and only knows the losses; the result well takes a full row and the button another one.
- «Tolleranza» sits above a field that holds three characters.
- The resolution in use is only a `data-mode` attribute: nothing tells the student whether a list is read at unit or at high resolution.
- At unit resolution the error of a loss («+0.0 Da») carries no information.

## 2. Layout of the merged page (drawer tool `addloss`)
```
┌ Adducts & losses ───────────────────────────── × ┐
│ [▦ Calculator ▴]                                  │  ← toggle (calculator icon, aria-expanded), remembered
│  [ 96-66                                       ]  │  ← #calcin
│  [› Search]                               30      │  ← one row: Cerca on the left, result right-aligned
│  ( ) ← C / 7 8 9 ÷ / 4 5 6 × / 1 2 3 − / 0 , = +  │  ← compact keypad (36 px keys, 40 px on touch)
│ ───────────────────────────────────────────────── │
│ [🔍 m/z, Δm or formula                         ]  │  ← ONE field for all the lists (#al-q)
│ [0.5] Tolerance [Da|ppm]  [ESI+][ESI−]  Unit res. │  ← one choices row; chip says LR / HR and why
│ [Adducts] [Neutral losses] [Contaminants 1]       │  ← tabs; Contaminants counts its matches
│  list rows in one style (mass | name/formula | …) │  ← the list takes the remaining height
└───────────────────────────────────────────────────┘
```
- The page is a flex column: calculator and controls keep their natural height, the active list fills the rest and scrolls on its own; nothing has a minimum height, so two or three rows sit right under the controls.
- The calculator is shown by default and folds with its toggle; the choice is a per-viewer convenience in `localStorage` (`NOMI.KEY("addloss.calc")`). Folded, the lists gain the whole height.
- Under the docked calculator there are no groups any more (`#calcqui` stays only in the floating calculator of Disegno; docked, only the line «read as C14H13F4N3O2S» stays): the lists below are the answer.

## 3. Polarity: ESI+, ESI− or both
- `CALCQUI.st.pol` takes `1`, `-1` or `0` (both). Unset, it follows the files: only positive = ESI+, only negative = ESI−, both kinds = both, no file = ESI+.
- The two buttons are independent toggles (`aria-pressed`); switching off the last one switches to the other, so one is always on (the behaviour the Neutral losses filter already had).
- Adducts list the rows of the selected polarities; losses keep `±` losses always and `+`/`−` ones by polarity; the contaminant lists are searched in both index halves when both are on. The Neutral losses tab has no polarity filter of its own.

## 4. One list style, HR vs LR
- Every row is one line: mass column (accent, tabular) | formula or adduct name | short label (muted, ellipsis) | chevron for details. Adducts keep their table (columns are needed for M, m/z and error); contaminants and loss combinations are flat rows (`.cq-flat`) without folds, since the tab already names them.
- The difference between the lists is said once by the tab and by a one-line caption above the rows: Adducts = «ion = M ± shift» (the number is an observed m/z), Neutral losses = «Δm between two peaks» (the number is a difference), Contaminants = «known background ions at this m/z».
- Resolution chip in the choices row: «Unit resolution» / «High resolution» (title: from the open files with the instrument name, or from the decimals typed). LR: one decimal, no ppm, no error column for losses; HR: four decimals, error in ppm for adducts and in mDa for losses.
- Removed: «+0.0 Da» on loss rows at unit resolution.

## 5. «Cerca» handoff and entry points
- The calculator button reads «Cerca» / «Search». It puts the result (or the formula typed) into the page field; a difference (an expression with a binary minus, also after «=» through the last line of the tape) selects Neutral losses, a formula selects Adducts, otherwise the tab stays as it is.
- Toolbar: one button, «Adducts & losses». The `calc` command leaves the bar in the Dati view and on the start page (`onBar`; the command palette still finds it and opens the page with the calculator unfolded and focused); in Disegno the calculator button stays and opens the floating drop-down, unchanged.
- `BARRA.setTab("calc")` = `addloss` with the calculator unfolded; a saved state with `tab: "calc"` reopens `addloss`. `BARRA.isOpen("calc")` is true while that page is open with the calculator unfolded.
- `CALCQUI.apri(n, {gruppo})` (spectrum ruler, `CALCADD.showDelta`) fills the page field and picks the tab (`diff` → Adducts with the differences open, otherwise Neutral losses).
- `ricerca.js` (`RIC_GO.adduct`, `RIC_GO.loss`, `RIC_GO.cont`), `spettro.js` `dmOpen` and `spec.cont`, `tables.js` `open("ad"|"ls")` go to the page sections, switching to the Dati view when needed: these lists live only in the side panel, never in a pop-up (Federico).

## 6. Contaminants
- Third tab «Contaminants» (first PR) with the `liste` group of `CALCQUI.gruppi` (Keller 2008, laboratory list, user suspect lists) in the same row style: name | adduct · m/z · list · error. Typing a number in the one field fills all three tabs; the tab counts its matches; a formula is not looked up there.
- Second PR — spectrum: the peak context menu has «Is it a known contaminant?» (`spec.cont`), which opens the page on that m/z with the Contaminants tab: the m/z as written on the chart sets the tolerance, the polarity is the page's. The command palette's contaminant hits (`RIC_GO.cont`) open the same tab; the Lists tool (`LISTE.open`) stays for managing lists.
- No new engine route: the index is the one `LISTE` already builds from `/api/contaminants`.

## 7. Dependencies and delivery
| Area | PR 1 (page, ESI both, cleanup, Contaminants tab) | PR 2 (entry points to contaminants) |
|---|---|---|
| `index.html` | `#sb-panel-addloss` holds `#al-calc` (toggle + docked `#calcdlg`), `#al-q`, `#al-ctl`, three tabs, `#al-cap`; `#sb-panel-calc` removed | — |
| `barra.js` | `calc` alias of `addloss` (`setTab("calc")`, state migration), `onBar` keeps the palette entry; `cont` sub-tab; outside Dati the page switches to Dati | — |
| `calc-qui*.js`, `tables.js` | `pol` 0 = both; shared field and choices; losses list without its own field/filter; no «+0.0 Da»; resolution chip; flat views; Contaminants view and count | — |
| `eventi.js` | «Cerca» handoff to `#al-q`; result row; `calcToggleFloat` in Dati opens the page | — |
| `spettro.js`, `ricerca.js` | point at the page sections | peak menu entry, `RIC_GO.cont` |
| i18n (`lang/it`, `lang/en`) | `calc.nl` = Cerca/Search, `al.q.*`, `al.t.cont`, `cq.res.*`, `cq.cap.*`, `cq.g.combos` | menu entry |
| Tests | `e2e_addotti_default` (ESI both, Contaminants, fold), `e2e_calc*`, `e2e_perdite`, `e2e_strumenti*`, `e2e_a11y_hr` (axe on the three tabs) and the others that opened the calculator follow the new ids | e2e for the peak menu |
| Tour | no step targets the calculator or this panel: unchanged | — |
| Engine / wasm | none (all page-side; `api/adduct_table` already returns both polarities) | none |
