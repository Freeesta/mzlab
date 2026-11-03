# Page audit: every view, every header button, with and without samples

Audit of 3 November 2026 on `origin/main` (0b92219 + this branch), Chromium (Playwright, headless), viewport 1440 × 900, `prefers-reduced-motion: no-preference`
(so slides and transitions are visible to the measure), the site built by `tools/prepara_sito.py` (Rust engine, no optional `wasm-feat` modules), synthetic look-alikes of the
lab files (`tools/dati_sintetici.py`). Reproduce: `MZLAB_MZML=.verifica/dati_sintetici python3 tests_e2e/audit_pagine.py OUT_DIR` (~15 min, writes `audit.json` and screenshots).
The permanent, lighter guard is `tests_e2e/e2e_giro_intestazione.py` (smoke round, Chromium Linux).

## 1. Method

- **Matrix**: views Dati, Disegno, Tabui, Flow, Teoria × samples {none, one Full Scan file (`B_FullMass-t15`), five files (3 Full Scan + MS2 + MRM, `lib.FILES`)} × {cold
  (first visit of the view in the tab), warm (second visit)} plus the return to Dati from each view, and a cold load of `#/draw` by address.
- **Per cell**: every visible header button (`header button`, view tabs apart) is clicked, what opened is recorded (dialogs, menus, `#helppop`, `#calcdlg`, `aria-expanded`,
  pressed header buttons, side panel and its tool, full screen), then closed with Esc, else with a second click, else put back by hand (recorded as «left open»).
  With samples, every panel button of the work area (`#np-chrom`, `#np-spec`, `#np-map`, `#np-xic`, `#np-mrm`) and the sample table (`#samptab`) are opened and closed.
- **Measures**: first contentful paint and time to useful content (the drop area or the first panel; the Ketcher iframe for Disegno; the first button of Tabui/Flow);
  engine ready (`AVVIO.stato`); cumulative layout shift from `PerformanceObserver('layout-shift')` (shifts within 500 ms of a click are excluded by the browser, so the
  movement of blocks is ALSO measured directly: bounding boxes of header, `#stbar`, `#sb-pane`, `#tools`, `#v-workspace`, `#start`, every panel, before/after, and sampled
  on every animation frame for 0.7–0.9 s after each click); visible text or attributes that look like an i18n key (`a.b.c`) or a `{placeholder}`; console errors and
  warnings, failed requests and status ≥ 400, response sizes; memory through CDP (`HeapProfiler.collectGarbage` then `Performance.getMetrics`: DOM nodes including
  detached ones, JS event listeners, used JS heap) before and after 20 open/close cycles of an opener.

## 2. Matrix (numbers)

Times in ms from navigation start (cold) or from the click on the view tab (switch, includes waiting for the page to be quiet: ~1000 ms is the floor of the measure).

| | no samples | 1 sample | 5 samples |
|---|---|---|---|
| first contentful paint | 404 | 452 | 452 |
| useful content (drop area) | 1435 | 1842 | 1830 |
| engine ready | 1555 | 1973 | 1940 |
| files read and first shown | — | 1925 | 2997 |
| CLS of the cold load + first file | **0** | **0.148** | **0.158** |
| requests / bytes in the session | 523 / 87 MB | 544 / 88 MB | 592 / 88 MB |
| console errors | 1 (404) | 3 (404) | 3 (404) |
| untranslated keys / placeholders | 0 | 0 | 0 |

| view | cold switch | warm switch | back to Dati | CLS cold / warm / back | movement on return to Dati |
|---|---|---|---|---|---|
| Disegno | 2890 / 2836 / 2886 | ~1040 | ~1030–1130 | 0 / 0 / 0 | none without samples; **side panel slides in** with samples |
| Tabui | ~1030–1060 | ~1020 | ~1000–1170 | 0 / 0 / 0 | same |
| Flow | 1105 / 1095 / 1479 | ~1030 | ~1020–1185 | 0.0003 / 0 / 0 | same |
| Teoria | ~1030–1060 | ~1020–1040 | ~1020–1140 | 0 / 0 / 0 | same |
| `#/draw` by address (cold) | useful at 4783–5056 | | | 0 | |

Header buttons walked: Dati 13 (`#sb-toggle`, File, Calcolatrice, Addotti, Perdite, Liste, Tavola, Altro, Cerca, Nuova, schermo intero, Impostazioni, (i)); Disegno,
Tabui, Flow 11 (no panel toggle, no File); Teoria 2 (schermo intero, (i)). Every popup (Tavola `#refdlg`, Altro `#stmenu`, Cerca `#paletta`, Nuova `#resetdlg`,
Impostazioni `#uipset`, (i) `#helppop`) opens and closes with Esc in every view and sample state; full screen toggles with its button.

Memory (20 open/close cycles after a first opening, 1 sample; probe of the same page): DOM nodes +0…+2, listeners +0, heap +10 KB per cycle at most (GC run before
each reading) for Impostazioni, Cerca, Altro, Tavola, Nuova, Calcolatrice ↔ File and five Dati ↔ Disegno trips. The first opening of Impostazioni builds 292 nodes and 28
listeners once (lazy), the first Tavola ~900 nodes: built once, not per opening. No leak and no detached DOM found. In `audit_pagine.py` the 20-cycle runs of Cerca,
Altro, `#sb-toggle` and the view tabs can time out: the palette `#paletta` (`aria-modal`) left open by the previous cycle catches the clicks. That is a limit of the
script's reset between cycles, not of the page (a probe that closes it by Esc after each opening counts the same nodes and listeners).

## 3. Defects, ranked

1. **Calcolatrice, Addotti, Perdite and Liste did nothing outside Dati** (Disegno, Tabui, Flow, with or without samples). `toggle()` in `barra.js` asks
   `show()`, which refuses outside Dati and the landing, so the documented windows (`#calcdlg` drop-down, `#refdlg`, `LISTE.open`, `dati-barra.md`) never opened;
   `e2e_calc` was red on main (14 of 18 steps) and is not in the smoke round, so the PR CI did not see it. **Fixed here**: `toggle()` in `barra.js` routes the four tools to
   their windows outside Dati; `e2e_calc` back to 18/18; the new smoke test fails without the fix.
2. **Side panel slides in when coming back to Dati** (Federico's report; chat DATA-SLIDE). With samples, after any other view, `#sb-pane` moves from x ≈ −8…1 px to 6 px
   over ~50 frames (~0.8 s); without samples it does not move. Cause hypothesis: the `tpview` handler in `barra.js` restores `_open` from `_away` and calls
   `applyLayout()`, which toggles `sb-open` on `#v-data` without the «instant» path, so the opening transition of the drawer plays as if the student had opened it.
   Fix proposal: on a return from another view apply the layout with the transition suppressed (the `sb-first`/instant class for one frame). Not edited (DATA-SLIDE PR).
   The smoke test checks the end position only (with reduced motion), so it stays green with or without that fix.
3. **Layout shift of 0.13–0.16 when the first file shows itself** («poor» above 0.1). Two shifts: ~0.015 when the drop area, the method row and the table move 36 px down
   as rows arrive, then 0.133 when the side panel narrows from 300 px (landing, `defaultW()`) to 280 px (`byNames()`: «the names set the width once a file is shown»)
   and the whole work area jumps 20 px left, with no user input (the file opens by itself). Fix proposal (a decision for the coordinator, it changes a deliberate rule):
   keep the landing width until the student resizes or the names need MORE room, or size by names already in the landing as soon as rows exist.
4. **The page is taller than the window** in Dati (+7 px: `#dfiles` is `100vh − header − 16 px` but starts 16 px below a 12 px gap) and Disegno (+13 px: `#v-draw aside`
   is `100vh − --hh` under a 12 px gap). There is a page scroll bar, and opening the first file scrolls the page by 7 px, so every block moves up 7 px (the 51 vs 58 px
   tops seen in the audit). Fix proposal: subtract the top gap in those `calc()` (`app.css` lines 91 and 303). Not edited: top gap CSS belongs to chat GAP-TOP.
5. **A second click on a tool of the side panel closes the panel** instead of going back to the previous tool (Dati with samples: File → Calcolatrice → Calcolatrice
   leaves no panel; without samples it goes back to File). This is the documented toggle (`dati-barra.md`), listed only because it surprised the audit: a student
   who opens the calculator from the file list and closes it loses the file list. Proposal (coordinator): return to the previous tool, as the landing does.
6. **Disegno is slow the first time**: ~2.9 s from the tab to the editor, 4.8–5.1 s for `#/draw` by address (Ketcher, 29 MB script, fetched once and then from cache;
   the prefetch of `first-paint.js` works: one GET in the server log for two requests in the page). Proposal: none urgent; the warm switch is ~1 s.
7. **Flow, first opening**: the toolbar buttons (`#flx-run`, `#flx-cli-show`, `#flx-more`) shift 81 px sideways once the late part of the bar arrives (CLS 0.0003,
   visible as a jump). Proposal: reserve the width of the late buttons (min-width on the bar or the buttons drawn hidden).
8. **`wasm-feat/manifest.json` is fetched 2–5 times per session** (one `fetch` per optional module: `disegno-frammenti.js`, `rust-msngrafo.js`, …; 404 in the e2e site,
   which does not build those modules, hence one console error per fetch there). Proposal: one shared promise for the manifest.
9. **Chromium warning about the Ketcher iframe** («allow-scripts and allow-same-origin … can escape its sandboxing»), twice per session: known and by design (Ketcher
   needs both); the smoke test ignores exactly this message.

Not found: untranslated keys or placeholders, failed requests other than (8), console errors other than (8), dialogs that Esc does not close, leaks over 20 cycles,
movement of the existing panels when one is added or closed with its × (the page scrolls to the new panel, which is intended).

Red on `origin/main` with or without this branch's `barra.js`, not caused by it and left to their areas: `e2e_strumenti` (steps on the bar labels) and
`e2e_pannello_vista` (`scrollTop === 40`). The `e2e_strumenti` step «in Disegno the same buttons open the windows» is red on main and green with defect 1 fixed.

## 4. Areas of other chats (reported, not edited)

- DATA-SLIDE: defect 2. GAP-TOP: defect 4. ADD-LOSS: the floating Addotti/Perdite windows open again (defect 1, a routing fix in `barra.js`, the panels untouched).
- SETTINGS-PANEL, TOOLTIPS, CODE-03 (`carica.js`): nothing found in their areas by this audit.

## 5. Smoke test

`tests_e2e/e2e_giro_intestazione.py` (≈ 75 s on 4 CPU, in `SMOKE_CHROMIUM_LINUX`): no samples → Dati, Disegno, Flow, Tabui: every header button reacts and is closed
by Esc or a second click, nothing moves more than 2 px; the tool buttons open their window outside Dati; Dati → each view → Dati moves nothing; one sample → the same in
Dati and Disegno, the round trip also with the side panel shut, chromatogram/spectrum/map panels added and closed with their × leave the work area as it was; no console
error and no failed request (the two known messages of section 3, items 8–9, excepted). Waits are on page state (`ready`, `attendi`, frames), never fixed pauses; the
movement threshold is 2 px against jumps of 7–300 px, so timing does not decide the result.
