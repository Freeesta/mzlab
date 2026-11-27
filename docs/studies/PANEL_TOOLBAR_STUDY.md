# Study: one shared tool bar for the panels of the Data view

Owner idea (27 Nov): a single menu of tools (areas, download, full screen, rulers…) shared by all panels, at the top; the user selects a panel and the bar shows the buttons that apply to it. Questions: does it make sense, what do other programs do, which design, how much work in this code base. Written for a chemist, not a developer. Interface only: no engine, CLI or `.wasm` change, so parity site = app = CLI is untouched.

## Short answer
1. **Yes, as a hybrid**, not as a pure replacement: one shared bar for the selected panel, plus a few buttons that stay on every panel (expand, close), plus a well-organised right-click menu with the same commands.
2. **The saving is not vertical.** Today the buttons sit on the same row as the panel title (`addPanel`, `pannelli.js`); that row stays because the title, the file chip and the close button live on it. The real gains are consistency, room for words instead of icons, and the end of the «⋯» overflow code. 
3. First slice: make every header button a registry action (useful whatever you pick). Biggest risk: 64 e2e files name the old buttons (222 references). Effort: 7 PRs (4 medium, 3 small).

## 1. Does it make sense?

| Point | Shared bar for the selected panel | Today: a row on every panel |
|---|---|---|
| Vertical space | One 40 px row for the whole stack, but the panel title row is still there (title, chips, expand, close): **about the same height** | N rows of 26 px (40 px on touch) |
| Horizontal space | The whole width is free: words next to icons, no «⋯» | Each header fights for width; `fitHd` moves buttons into «⋯» (5 e2e files guard it) |
| Consistency | Same place, same order for every kind; one place to learn | Each kind has its own set (11 to 16 buttons), order differs |
| Discoverability | All tools visible in one place, with labels | Icon-only, crowded, some hidden in «⋯» |
| Frequent actions | **Extra click** the first time (select, then act). A click on the panel already selects it, so zoom, log and download on the panel you just used cost nothing extra | One click, always |
| Several panels | Comparing: change a setting on panel 1, then panel 2: two selections instead of two clicks; state of the other panels is not visible unless the header shows a small dot chip for non-default states | Every state visible at once |
| Linked panels (`←1`, `→2`) | Badge stays on the panel header; the bar says «② spectrum, follows ①»; selecting the spectrum never changes the chromatogram | Same, but cramped |
| Panel kinds | Bar shows only the groups the kind has (chromatogram, XIC, SIM/MRM, DAD trace, spectrum, UV spectrum, RT-m/z map; a table would have copy, CSV, expand, close) | Each kind builds its own HTML |
| Touch / 390 px | One scrollable row with 44 px targets and a «More» sheet is easier than N small rows; hover-only bars do not work | 40 px rows already, but overflow into «⋯» |
| Keyboard | The bar is ONE toolbar with one Tab stop (APG toolbar pattern); a key jumps to it; today each panel has up to 16 Tab stops | Many Tab stops |

Verdict: sensible because a click already selects a panel (`setActive`, `.act`) and keys 1-9 already choose a panel; the cost is the extra step on a different panel. Do not move **selectors** (kind menu, file chip, range chip, level, colour keys): they say *what the panel shows*, and must stay on the panel. Move **tools** (what you can do).

## 2. How comparable programs do it
Status: V = a page was opened and says it, P = partial (page read, or source code, but not every detail), U = unverified (memory or a search snippet only). Numbers in brackets are the sources at the end of the section.

| Program | Pattern | How a pane becomes the active one | Right-click menu | |
|---|---|---|---|---|
| Thermo FreeStyle 1.8 | **Shared contextual toolbars**: six toolbars, each shows only the tools of the selected view | Click a view: darker title bar, «~» prefix, green name bar; the last selected view of each type receives linked actions | Chromatogram tools: Reset Scaling, Copy, Undo/Redo Zoom, Ranges, Average Spectrum; spectrum: Link to Chromatogram, Clear Structure Annotations | V [1] |
| Thermo Xcalibur Qual Browser | Cells with menus and toolbars; commands apply to the active (pinned) cell | Click the cell: grey border; «the cursor action is always applied to the pinned cell» (2.2 guide) | «Ranges», toggle peak detection in this plot; full list not read | P [2] |
| Sciex Analyst (guide of 2025; 3200-era version not checked) | **Global Explore toolbar + right-click per pane** | Click a pane; new panes open under the active pane | Chromatogram: Show Spectrum, Extract Ions, Show Base Peak Chromatogram, Set/Clear Subtract Range, Save to Text File, Add Caption, Delete Pane. Spectrum: Show TIC, Extract Ions (range or maximum), Select Peaks For Label, Search Library, Delete Pane | V [3] |
| Sciex OS Explorer | **Toolbar in every pane (three layers) + a master bar under the menu that mirrors the generic icons for the active pane** | One active pane, coloured border and toolbar; Ctrl-click applies an action to all panes; axis click reverts zoom | Not listed in the tutorial; Analytics: «Options» button with Active Graph or All Graphs (right-click U) | V [4] |
| Waters MassLynx 3.5 (UNIFI not verified) | One toolbar at the top of each Chromatogram and Spectrum window, customisable | Coloured title bar; click it or pick it in the Window menu; arrow keys move between traces | None documented: right mouse is a mapping tool (right-click a peak = mass chromatogram, right-drag = range) | V [5] |
| Agilent MassHunter Qualitative | Main toolbar + a toolbar inside each window; windows can be tabbed, floated, layouts saved | Not stated | Chromatogram: Extract Chromatograms…, Integrate Chromatogram, Integrate and Extract Spectrum; spectrum: Extract EIC, Move to Background Spectrum | P [6] |
| Bruker DataAnalysis | Could not read any page | - | - | U |
| DataApex Clarity | Window toolbars (integration icons on the left, Overlay toolbar); View > Toolbars, Maximize Graph | One signal is active: bold in the legend, debossed icon, graph outline; Ctrl-click hides a signal | «Properties…»; double click restores the view; full list U | P [7] |
| MZmine 3/4 | **Vertical icon bar at the right of every plot tab**, same functions repeated in the right-click menu | Each plot is its own tab | Reset Zoom, Set Range on Axis, Copy chart, Print, Export graphics…, Export data (Excel, clipboard) | P [8] |
| Skyline | Shared menu bar; docked panes with right-click menus; no per-pane button bar documented | Panes follow the node chosen in the Targets tree | Legend, Remove Peak, Synchronize Integration; keys F11 (auto-zoom), Ctrl+T (tiled) | P [9] |
| MS-DIAL | Fixed multi-panel window; small icons and tabs inside panels; no plot toolbar | Click a spot: it becomes the «focused spot» and drives EIC and spectrum | Menu exists, items not listed; mouse only | P [10] |
| OpenChrom | Eclipse views and editors; a menu «Chromatogram Edit»; chart popup menu | The editor holds the selection; views update | Only «Reset 1:1» confirmed | P [11] |
| Office contextual tabs | **Ribbon tab appears when a chart is selected** | Selection | - | V [12] |
| Figma / Photoshop | Properties panel changes with selection; with nothing selected it shows page-level items (Figma V [13]); Photoshop options bar follows the tool (U) | Selection | - | V/U |
| Chrome DevTools | One toolbar per panel; panels are tabs; a command menu (Ctrl+Shift+P) reaches everything | - | - | V/P [14] |
| Plotly | **Hover-only modebar** per chart (option to keep it visible) | - | - | V [15] |
| Bokeh | `gridplot` **merges the tools of all plots into one toolbar** that shares the active tool | - | - | V [16] |
| Highcharts | One always-visible export button per chart | - | - | P [17] |
| Grafana | **Hover menu icon per panel** (View, Edit, Share, Inspect…); keys act on the hovered panel (`e`, `v`) | Hover | The menu | V [18] |
| Origin | **Mini Toolbar next to the selected object**, content depends on the object; Object Manager is a second way to select | Click an object | not read | V [19] |

### Lessons
1. **Selected-pane model is the norm** in instrument software (FreeStyle, SCIEX OS, Analyst, Qual Browser, Clarity's active signal). The indicator is a border or title colour, as mzLab already does (`.pnl.act`).
2. **Two families.** Shared contextual toolbar: FreeStyle, Office, Figma, Bokeh. One toolbar per pane: SCIEX OS, MassLynx, MZmine, Agilent. SCIEX OS does both (pane icons plus a master bar for the active pane) and MZmine repeats the toolbar in the menu: duplication is tolerated when one list feeds both. mzLab's registry makes this cheap.
3. **Right-click menus are about operations at the pointer**: extract, integrate, subtract a range, set ranges, copy, export. That supports the split in 3.7 (menu = at the pointer plus a panel tail; bar = panel-wide view and output).
4. **Linked panes**: FreeStyle gives linked actions to the last selected view of each type; Skyline panes follow the selected tree node. A badge-based link as in mzLab is in line with this.
5. **Multi-pane actions** exist (SCIEX OS Ctrl-click applies to all panes, Analytics «Active Graph or All Graphs»): precedent for slice 7.
6. **Nothing selected**: Figma shows page-level items; follow this (3.3).
7. **Hover-only controls** (Plotly, Grafana) fail on touch and for keyboard users unless a visible trigger and a key path exist; NN/g: click or tap is the clearest signal of intent, hover reveals need about 0.3-0.5 s (V [20]). Never hover-only.
8. **Menus**: NN/g (V [21]): about 10-12 items, every contextual command also in a main place, show shortcuts, give non-gesture alternatives. Accessibility: APG toolbar (one Tab stop, arrows, optional jump key, label) V [22]; targets at least 24 px, WCAG 2.2 SC 2.5.8 V [23]; keyboard operable, SC 2.1.1 V [24]. Right-click also on the Menu key V (MDN [25]); long press and Shift+F10 are U outside the repo (mzLab implements both).
Not verified: Apple HIG and Material guidance, Photoshop help, Bruker, UNIFI, Origin right-click menus.

### Sources
[1] https://docs.thermofisher.com/r/FreeStyle-User-guide/enGUID-5E847996-7079-414E-A5F2-3F55E26A0A5E
[2] https://chemistry.unt.edu/system/files/thermo_xcalibur_2_2_0_48_acquisition_and_processing.pdf
[3] https://sciex.com/content/dam/SCIEX/pdf/customer-docs/user-guide/an-software-user-guide-en.pdf
[4] https://sciex.com/content/dam/SCIEX/pdf/customer-docs/user-guide/sciex-os-explorer-tutorial-en.pdf
[5] https://groups.chem.cmu.edu/bier/wp-content/uploads/sites/12/2025/09/masslynx_35.pdf
[6] https://community.agilent.com/knowledge/masshunter-portal/w/masshunter-knowledgebase/10153/introduction-to-the-layout-of-navigator-view-in-masshunter-qualitative-analysis-software
[7] https://www.dataapex.com/documentation/Content/Help/030-chromatogram/030.040-chromatogram/030.040-overlay-other.htm
[8] https://mzmine.github.io/mzmine_documentation/latest/visualization_modules/raw_data_overview/raw_data_overview.html (menu names from the source file `EChartViewer.java` on GitHub)
[9] https://skyline.ms/tutorials/24-1/MethodRefine/en/index.html
[10] https://systemsomicslab.github.io/mtbinfo.github.io/MS-DIAL/tutorial
[11] https://pmc.ncbi.nlm.nih.gov/articles/PMC2920884
[12] https://learn.microsoft.com/office/dev/add-ins/design/contextual-tabs
[13] https://help.figma.com/hc/en-us/articles/360039832014-Design-properties-and-the-properties-panel
[14] https://developer.chrome.com/docs/devtools/command-menu
[15] https://plotly.com/javascript/configuration-options/
[16] https://docs.bokeh.org/en/latest/docs/user_guide/basic/layouts.html
[17] https://api.highcharts.com/highcharts/exporting.buttons.contextButton
[18] https://grafana.com/docs/grafana/next/panels-visualizations/panel-overview/
[19] https://docs.originlab.com/origin-help/mini-toolbar
[20] https://www.nngroup.com/articles/timing-exposing-content/
[21] https://www.nngroup.com/articles/contextual-menus/
[22] https://www.w3.org/WAI/ARIA/apg/patterns/toolbar/
[23] https://www.w3.org/WAI/WCAG22/Understanding/target-size-minimum.html
[24] https://www.w3.org/WAI/WCAG22/Understanding/keyboard.html
[25] https://developer.mozilla.org/en-US/docs/Web/API/Element/contextmenu_event

## 3. Recommended design: «Panel bar» (hybrid)

### 3.1 Parts
- **Panel bar** `#pbar`: one sticky row under the main tool bar (`#stbar`), above the stack. Tools of the selected panel, grouped: View · Analyse · Extract · Output.
- **Header of each panel keeps**: number, pair badge (`←1`, `→2`), title, selectors (kind, file chip, range chip, level), readout, **expand** and **close** (always inline, as the owner asked), and small dot chips for non-default states (log, %, smoothing off).
- **Right-click menu** per kind (3.7): point actions, then a «Panel» tail with the same commands as the bar.
- **Palette** (Ctrl/⌘K) already lists commands; unchanged.
One list of commands in code: the `actions/1` registry (`azioni.js`). Bar, menu, palette and keys all read it.

### 3.2 Mockups
Chromatogram selected (wide window):
```
 [stbar: Files  Calc  Lists …                                     undo redo ]
 ┌ ② TIC · sample A ▾ ┬ View ──────────┬ Analyse ─────────────┬ Extract ──┬ Output ─────┐
 │  (selected panel)  │ ⌕ box  ⤢ fit   │ ∫ auto  ∫ all▾  ∫ ✎  │ XIC  UV@RT│ ⤓ download │
 │                    │ ~ smooth  log  │ ✕ delete  baseline   │           │ copy  ⛶     │
 └────────────────────┴────────────────┴──────────────────────┴───────────┴─────────────┘
 ┃① TIC  sample A ▾  [RT 0-12 min]                   ·log   ⛶ ×   ← selected: left bar
 ┃  ~~~ plot ~~~
  ② MS¹  ←1 follows ①  [m/z 50-500]                                 ⛶ ×
     ~~~ plot ~~~
```
Spectrum selected: the same bar, groups View (box, fit, ruler, lock y, %), Analyse (peak picking ▾, peak table), Extract (XIC, MS2), Output. Nothing about integration (a kind that lacks a group shows nothing, never a grey gap).

Phone, 390 px (44 px targets, one scrollable row, priority order, rest in a bottom sheet):
```
┌──────────────────────────────┐
│ ② TIC │⤢│⌕│∫│XIC│⤓│ ⋯ More ▴ │  ← 44 px, swipe sideways, edge fade
├──────────────────────────────┤
│ ① TIC ▾ sample A      ⛶  ×  │
│  ~~~ plot ~~~                │
```

### 3.3 Selection
- Shown by what exists today: left bar on the panel (`.pnl.act`, coupling colour if paired) **plus** the number chip at the left of `#pbar` in the same colour. Not colour alone: the chip carries number, kind and file.
- **Nothing selected**: the first panel is selected when the stack opens, and a click on empty space keeps the last selection (today it clears `E.active`, `explore.js`). If no panel exists the bar shows the global actions (open file, undo, redo, show/hide panels) and the hint «Click a panel to see its tools».
- **Several selected** (slice 7, optional): Shift-click on number chips; the bar shows only actions all selected kinds have and applies them to all (fit, download zip, close, link zoom). v1 is single selection.
- Selection follows keys: 1-9 (exists), Tab into a panel, and focus inside a panel selects it.
- Linked spectrum (`←1`): selecting it never selects its chromatogram; the bar chip says «follows ①» and the existing `followMenu` stays on the badge.

### 3.4 Always inline
Expand (F), close (Esc closes expanded; × closes the panel), pair badge, selectors. Reason: they are used on any panel without selecting it, and close must never be a hidden step.

### 3.5 Phone (390 px)
`barra.js` already has `PHONE_W = 600` and a sheet pattern (`SHEET = 900`). The bar becomes one row, targets 44 px (the app's touch floor; WCAG 2.2 2.5.8 needs 24 px, VERIFIED), priority order per kind, remaining tools in «More» as a bottom sheet. No horizontal page scroll (axe at 390, `e2e_intestazioni`). The channel picker popup already stays inside a 390 px screen (#3771).

### 3.6 Keyboard and accessibility
- `#pbar` is `role="toolbar"` with `aria-label` «Tools for panel 2, TIC»; one Tab stop, arrows move, Home/End optional (APG toolbar, VERIFIED).
- A registered key (to be chosen free of conflicts; `test_azioni_tasti.py` checks duplicates) moves focus to the bar; Esc returns to the chart (`ESC.registra`, 4.4).
- Selecting by keyboard announces the panel in `#sb-live`.
- Disabled-versus-hidden: follow the repo rule (`AZIONI.clean`: unavailable entries are not shown); this differs from NN/g, which prefers greyed items (VERIFIED), because mzLab menus are kept short.

Owner's earlier requests, where they fit:
- **Colour keys on traces** (swatches by the lists, `sim-ui.js`, `canali.css`): stay with the data selectors and the legend on the plot, not in the bar.
- **SIM channel picker**: a selector, stays in the header; the bar only has its area toggle.
- **Empty-panel hint**: stays on the plot; the bar greys nothing and shows a one-line «No data in this panel» chip when the selected panel is empty.
- **Linked spectrum clarity**: the bar chip names the link («follows ①»); the badge colour matches the left bar.

### 3.7 Right-click menu per panel kind
Today's menu (`AZIONI.forContext`, `docs/agenti/actions.md`) is flat in registry group order. Proposed: three blocks, each group separated; the keys come from `AZIONI.keyText`; **same entries as the bar** (one registry; a new field `scope: "point"|"panel"` decides the block).
1. **At the pointer** (scope point): readout row (RT, m/z), then what the point can do.
2. **Analyse** (point or selection).
3. **Panel** (scope panel): the bar's commands, so a touch or keyboard user never needs the bar. Download sits in one submenu.

| Kind | At the pointer | Analyse | Panel |
|---|---|---|---|
| Chromatogram (TIC, BPC) | RT row · Spectrum at RT (Enter) · Average spectrum of selection · XIC (X) · UV at RT · annotate | Integrate peak (I, not on totals) · identify all · MSn tree | Reset zoom · peak labels · options… · copy · download ▸ · full screen (F) |
| XIC | RT row · Add ion (X) · Spectrum at RT | Integrate peak (I) | same Panel block |
| SIM/MRM | RT row · extract ion (`mrm.estrai`) · UV at RT | Integrate peak (I) | same + area |
| DAD trace / UV spectrum | RT or λ row · UV at RT · average UV | Integrate | same |
| Spectrum | m/z row · peak: MS2 or XIC (Enter) · XIC (X) · measure (R) · annotate | Isotope profile · library match · contaminants · MS2 beside | Reset zoom · peak table · options… · copy · download ▸ (MGF, MSP, MassBank, SIRIUS) · full screen |
| RT-m/z map | point row · XIC · spectrum · lock crosshair | features (HR) | Reset zoom · copy coordinates · gestures · (3D) from above, side RT, side m/z |
| Table (future) | cell · copy cell | sort | copy · CSV · full screen |

Rules: at most 12 top-level rows (NN/g, VERIFIED), the rest in one submenu. Open by right click, **long press 550 ms** (exists), **Shift+F10 or Menu key** (exists, focus goes to the first entry, Esc returns). No logic duplication: each entry is `{id, scope, name, shortcut, available, run}`; the bar and the menu are two views of one list. Tests: `e2e_menu_coerenti` (Shift+F10 = right click = registry).

## 4. Feasibility in this code base

### 4.1 Where the code is
| What | File |
|---|---|
| Panel creation, header HTML, close, expand | `mzlab/web/pannelli.js` (`addPanel` line ~301-314, `ctl`, `syncTb`, `fitHd`, `hdRestore`) |
| Selection state | `STATO.attiva` → `E.active` (`stato.js`), `setActive` (`pannelli.js`), clear on blank click (`explore.js`) |
| Registry, menu, keys | `azioni.js`, `azioni-tasti.js`, `azioni-icone.js`, `azioni-esc.js`; ~50 `AZIONI.register` calls in 24 files |
| Tool-specific | `picchi.js` and `integra-*.js` (integration), `xic-spettro.js`, `xwc.js`, `sim-ui.js`, `dad-mrm.js`, `zoom-reset.js`, `visualizza.js`, `mappa.js` (own `.mznav` bar) |
| Style | `app.css` (`.pnl .hd`, `.tbg`, `.bt`, touch floor 40 px) |
| Notebook state | panel object `p` (`p.smooth`, `p.log`, `p.tl`, `p.zoom`…) saved by `uiSave` |

Finding: the header buttons are hand-written HTML, **not** registry entries; the registry feeds the menu and palette. Bar and menu are two lists kept by hand. The registry already has `barra: true` and `AZIONI.list(c, "barra")`, but no panel action uses them.

### 4.2 Buttons per panel kind (header HTML, counted from `addPanel`)
| Kind | Tools in the header | Of which shared with other kinds |
|---|---|---|
| Chromatogram (TIC, BPC, UV, analog) | 16: fit, download, expand, close, «⋯», zoom lens, link zoom, integrate auto / all ▾ / manual / delete / format / clear, smoothing, log, XIC | fit, download, expand, close, «⋯», lens, link, integration set, smoothing, log |
| XIC | 15: as above without XIC button | same |
| SIM/MRM | 15-16: as XIC plus the area toggle (`SIMUI.areaBtn`) | same + 1 specific |
| Spectrum | 11: XIC (+ ion chip), ruler, peak-picking ▾, %, lock y, fit, download, expand, close, «⋯» | 5 common; 6 specific |
| RT-m/z map | 9: `.mznav` (+, −, features, ?), fit, download, expand, close | 4 specific |

25 distinct `data-a` ids in `pannelli.js` (27 files in all use `data-a`): 4 in every kind (`fit`, `dl`, `max`, `more`; close is the `.x` button), 10 in chromatogram, XIC and SIM/MRM (lens, link, the integration set, smoothing, log), 1 chromatogram only (`xic`), 6 spectrum only, 1 map (`mapnav`, four buttons), 3 range selectors (`cpar`, `cparchip`, `rngx`).

### 4.3 Risks
- **e2e tests**: 64 files with 222 references to header `data-a` ids (`cpar` 28 and `rngx` 9 stay: range is a selector; `xicb` 27, `fit` 19, `xic` 17, `dl` 12, `iauto` 12, `rul` 10, `par` 8, `max` 7, `izoom` 7… move). About 170 references change, mostly mechanically through a helper `bar(pg, panel, id)` in `tests_e2e/lib.py`. Tests that guard the old overflow (`e2e_intestazioni*`, `e2e_pannelli_stabili`, `e2e_xic_stretto`, `e2e_intestazione_cromato`) are deleted with the code in the last slice, never skipped.
- **Interaction regressions**: pointer events (mousedown selects), the map and 3D viewer own their bars, DDA trio panels are docked.
- **Parity**: not affected; interface only, so site = app = CLI stays byte for byte. No `.wasm` size change (JS only). No new engine function, so no CLI route is needed; say so in each PR body.
- **Text**: new strings only through `lang/en`; most keys are reused (`panel.*.title`).
- **Light rule**: no framework; the bar is one renderer over the registry.

### 4.4 Order
Slices are independent PRs; the setting `qqq.pbar` keeps the old header until the last one.
1. `PBAR-REGISTRY-BUTTONS`: header buttons become registry actions, one renderer; no visible change. Useful with any design.
2. `PBAR-SKELETON`: `#pbar` and selection model, off by default.
3. `PBAR-MOVE-COMMON`: fit, download, ruler, log, smoothing, link, expand.
4. `PBAR-MOVE-SPECIFIC`: integration, XIC, peak picking, lock, %, area.
5. `PBAR-MENU-SCOPE`: right-click menu with `scope`, per kind.
6. `PBAR-PHONE-KEYS`: 390 px and keyboard.
7. `PBAR-REMOVE-OLD`: default on, delete the old rows and overflow code and obsolete tests.
Optional later: multi-select, configurable bar. Effort: slices 1, 3, 4 and 7 medium (25 ids; test migration), 2, 5 and 6 small; no PR touches the engine.

## 5. What this PR adds
This study; seven rows in `docs/coda/CODA.md` with ready prompts in `docs/coda/prompt/`, all `needs owner decision: panel toolbar design` until the owner picks. `docs/agenti/INDICE.md` is one row per area file of `docs/agenti/` (checked by `gate_release.py`), so no study row is added there; the `actions.md` row points here.
