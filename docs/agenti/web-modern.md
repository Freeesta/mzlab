# Modern web platform study of `mzlab/web/` (JS to replace or simplify)

Study only: no product code changed. Goal: a lighter site with less custom JS, without breaking the rules (logic in Rust, no bundler, English only, parity and guards unchanged, wasm modules and caps untouched). Read with `docs/agenti/compat-safari.md` (the floor) and `docs/coda/CODA.md` (the rows this study added).

## Method and evidence
- A skill named like "modern web guidance" is **not available** in this environment (the skill list and a skill search found none). The study used the repo (`python3 tools/indice.py`, grep over `mzlab/web/*.js` and `*/*.js` without `vendor/`, `elements.js`, `esempi/`: 394 files, about 4.07 MB, 50 800 lines) and current browser support from MDN browser-compat-data **8.1.10** (npm `@mdn/browser-compat-data`, read through jsDelivr; "version added" per browser, a number is the first release).
- Counts are grep hits (one hit = one line); "lines removed" is an estimate from reading the sites, not a measurement. Re-measure with the commands at the end.

## The finding that decides everything: the Safari floor
The site floor is Safari/iPadOS **16.4**, Chrome/Edge 111, Firefox 121 (`compat-safari.md`; `tools/safari_api.py` fails a use above it). Most platform features that would remove real JS arrive **after** that floor:

| Feature | Chrome | Firefox | Safari (macOS, iOS) | Above the floor? |
|---|---|---|---|---|
| `popover` attribute, `popovertarget` | 114 | 125 | 17.0 | yes (16.4 shows the content inline) |
| `<details name>` exclusive group | 120 | 130 | 17.2 | yes |
| CSS nesting, `light-dark()`, `@starting-style`, `text-wrap` | 117-130 | 117-129 | 17.2-17.5 | yes |
| `AbortSignal.any` (already shimmed in `compat.js`) | 116 | 124 | 17.4 | yes |
| `Promise.withResolvers`, `Object.groupBy` | 117-119 | 119-121 | 17.4 | yes |
| `startViewTransition` (used, guarded) | 111 | 144 | 18 | yes |
| `dialog requestClose()` | 134 | 139 | 18.4 | yes |
| CSS anchor positioning, `position-try` | 125-128 | 147 | 26 | yes |
| `Navigation API`, `URLPattern`, scroll-driven animations | 102 / 95 / 115 | 147 / 142 / 159 | 26-26.2 | yes |
| `dialog closedby` (light dismiss) | 134 | 141 | 27.3 | yes |
| `interestfor` (hover popovers) | 142 | no | no | yes, one engine |
| File System Access save, `FileSystemObserver` | 86 / 133 | no | no | yes, one engine |
| `adoptedStyleSheets`, `CSSStyleSheet()` | 73 | 101 | 16.4 | no |
| `AbortSignal.timeout` | 124 | 100 | 16.0 | no |
| `crypto.randomUUID` (secure context only) | 92 | 95 | 15.4 | no |
| `Blob.text/arrayBuffer`, `replaceChildren`, `toSorted`, `:has`, container queries | 76-110 | 69-121 | 14-16 | no |

So the big structural wins (native popovers, anchored tooltips and menus, light-dismiss dialogs, exclusive accordions) are **blocked by the floor**, not by the code. Raising the floor to Safari 17.0 is the owner's decision (it also drops devices that stop at iPadOS 16; to be confirmed with the lab's iPad fleet); this study does not change it. Below the floor, the safe wins are small duplicate removals and four native swaps.

## Inventory by role (hits in the JS above)
| Role | What exists today | Count |
|---|---|---|
| Dialogs | `<dialog>` + `showModal()` (already native), `cancel` handlers, no backdrop-click code | 27 `showModal`, 21 files |
| Popovers (panel controls, chips, peak table) | hand-made `hidden` boxes: outside-click, Esc, focus return, placement | about 10 popovers in 8 files (`pannelli.js`, `canali.js`, `sottrazione.js`, `tabella-picchi.js`, `xwc.js`, `mappa-dad-ms.js`, `zoom-reset.js`, `xic-bordi.js`) |
| Tooltips | own `tooltip.js` (76 lines): delay, keyboard, touch long press, WCAG 1.4.13 | 160 `title` or `data-tip` hits |
| Menus | right-click menu in `explore.js`, `menu-sotto.js` (41 lines), 15 `contextmenu` files | no native menu element exists |
| Tabs | 4 `role=tab` hits (`libreria-finestra.js`); `tabs.js` is the experiment-type switch, not a widget | |
| Drag and drop | `drop-unico.js` (27 lines), `cartella-carica.js` (`webkitGetAsEntry`) | 14 `drop` hits |
| File access | `showDirectoryPicker`, `showOpenFilePicker`, `webkitdirectory`, OPFS, all feature-detected | 5 / 1 / 4 files |
| Storage | `localStorage` 139 hits in 48 files, `sessionStorage` 30, `indexedDB` 18 | about 78 inline `try { localStorage... }` |
| Clipboard | `compat.js` `clipboardCopy`; 14 files still call `navigator.clipboard.writeText` directly | 68 hits |
| Downloads | `saveBlob` (`ordine.js`) used in 11 files; 3 files hand-roll it | 4 `.download =` |
| Layout in JS | `getBoundingClientRect` 105 hits (54 files), `.style.left/top/width/height` 112, 14 `resize` listeners, `ResizeObserver` 6 | |
| Style injection | a `<style>` created from a JS string | 39 files, about 230 KB of CSS text (regex estimate) |
| Reduced motion | `matchMedia("(prefers-reduced-motion...)")` | 19 sites in 16 files |
| Polyfills | `compat.js` (158 lines: Safari gaps, all feature-detected and tested) | |
| Hand-written helpers | `wait`, `hasOwnProperty.call`, `JSON.parse(JSON.stringify)`, base URL of the desktop app | see below |

## Candidates
Verdict: **do-now** (inside the floor, tested, small risk), **do-later** (needs the floor or a larger move), **not-worth** (evidence below). Lines are estimates.

### Do-now
Each do-now candidate: where and what it does today, the replacement, support, risk, covering tests, estimated lines removed.

**CLIP-SAVE-UNIFY** (about 20 lines) - done: every copy goes through `clipboardCopy`, every Blob download through `saveBlob`; only the boot-failure page (`avvio-progressivo.js`, runs before `compat.js`) keeps its own `writeText`.
- Today: `elementare.js` (`copyText`), `perf.js` (`#perf-copy`), `tdati.js` (copy button) and `area-copia.js` (`legacy`, `put`) each re-implement the textarea + `execCommand("copy")` fallback that `compat.js` `clipboardCopy` already has. Ten more files call `navigator.clipboard.writeText` with no fallback (the Safari gesture rule is in `compat-safari.md`). `bug-report.js`, `liste-target.js` and `rapporto-flow.js` re-implement the `saveBlob` of `ordine.js` (anchor, object URL, revoke).
- Replacement: `clipboardCopy` for every copy, `saveBlob` for every Blob download. Both are already used by 11 files; no new platform feature.
- Risk: low; the order must stay (async API inside the gesture, then the textarea).
- Tests: `e2e_area_copia`, `e2e_perf`, `e2e_tabelle_dati`, `e2e_banco_barra`, `e2e_hr_composizione`, `e2e_bug_report`, `e2e_liste_target`, `e2e_rapporto_flow`, `tests/test_compat.py`, `tests/test_bug_report.py`.

**NATIVE-SWAPS-SMALL** (about 10 lines)
- Today: `explore.js:283` `JSON.parse(JSON.stringify(x))`; `archivio.js:34` `AbortController` + `setTimeout(abort)`; `file-sicuri.js:90` and `workflow.js:227` `Object.prototype.hasOwnProperty.call`; `wait` defined twice in `disegno-appunti.js` and inline in `reset.js`.
- Replacement: `structuredClone` (Chrome 98, Firefox 94, Safari 15.4); `AbortSignal.timeout` (Safari 16.0, Firefox 100, Chrome 124: keep the controller for Chrome 111-123); `Object.hasOwn` (Safari 15.4, listed as allowed); one shared `wait`.
- Risk: very low; `structuredClone` throws on functions where JSON drops them, so check the value is plain data.
- Tests: `tests/test_archivio.py`, `tests/test_file_sicuri.py`, `e2e_reset`, `e2e20`, `e2e23`.

**CALM-MOTION-HELPER** (about 10 lines)
- Today: 19 `matchMedia("(prefers-reduced-motion: reduce)")` sites (`area-lavoro.js`, `pannelli.js`, `tour.js`, `settings.js`, `mappa3d.js`, `avvio.js`, ...). Nine `scrollIntoView({behavior: "smooth"})` or `scrollTo` calls (`banco.js`, `infusione-albero.js`, `libreria.js`, `libreria-finestra.js`, `tabs.js`, `pannelli.js:90`, `telefono-scheda.js`) ignore reduced motion and the in-app `data-a11y-reduce` switch.
- Replacement: one helper that reads both the system setting and `data-a11y-reduce` and returns `"auto"` or `"smooth"`; the nine calls use it. No new platform feature.
- Risk: low; behaviour changes only under reduced motion (a fix).
- Tests: `e2e_a11y_dati`, `e2e_scroll`, `e2e_tour`, `e2e_tema_impostazioni`, `e2e_pannelli_stabili`.

The three do-now rows remove about 40 lines and fix two small bugs (copy without fallback; smooth scroll under reduced motion). That is the honest size of what the floor allows.

### Do-later (needs the floor, or a bigger move)
| Id | Where | Replacement | Why not now | Lines |
|---|---|---|---|---|
| POPOVER-NATIVE | the ten popovers above: outside-click closers (24 handlers in 13 files), Esc, focus return, `z-index`, placement | `popover="auto"` + `popovertarget`; light dismiss, top layer and Esc come free | Safari 17.0 (floor 16.4); a popover element shows inline in 16.4, so no polyfill-free path. Trigger: the owner raises the floor to 17.x | about 150 |
| ANCHOR-PLACEMENT | `tooltip.js` `show` (10 lines of placement), `calcPlace`, `hintPlace`, arrange menu, `menu-sotto.js` right-edge flip, `getBoundingClientRect` + `style.left/top` pairs | CSS anchor positioning with `position-try` | Safari 26, Firefox 147: years from a floor | about 60 |
| DETAILS-NAME | accordions built from `aria-expanded` + `hidden` where only one stays open | `<details name>` | Safari 17.2; no JS-exclusive accordion was found (no `details.open=false` loops), so the gain is unproven | unknown |
| STYLE-OUT-OF-JS | 39 files inject a `<style>` from a JS string (about 230 KB: `mappa.js` 31 KB, `mappa3d.js` 23 KB, `draw.js` 22 KB, `libreria-finestra.js` 15 KB ...) | `.css` files loaded with `<link>` on first use, or a single injector over `adoptedStyleSheets` (Safari 16.4, in the floor) | no wire saving (same bytes), only cache, parse and about 110 boilerplate lines; several inject into an iframe document (Draw); `tests/test_csp_hash.py`, `tests/test_csp_desktop.py` and the service-worker precache list must follow. A study per file is needed first | about 110 JS lines |
| DIALOG-LIGHT-DISMISS | none: no dialog closes on backdrop click today, so there is no code to remove | `closedby="any"` | Safari 27.3 | 0 |
| LS-WRAPPER | 78 inline `try { localStorage.getItem }` plus 4 private wrappers (`libreria-corpo.js` `LS`, `logo-piccone.js`, `libreria-normative.js`, `llm-consenso.js`) | one tiny `store` module | not a platform feature: the lines stay one-liners; byte gain only | about 0 lines, 5-8 KB |

### Not-worth (with the evidence)
| Candidate | Evidence |
|---|---|
| Tooltips as `popover=hint` / `interestfor` | `interestfor` is Chrome 142 only. `tooltip.js` also carries the 1.0 s vs 1.25 s delays, keyboard focus, the 0.6 s touch long press and the absorbed click (tested by `e2e_safari_ipad`, `e2e_tip_*`). A native tooltip would drop those WCAG behaviours |
| Native menus | No menu element exists; `role=menu` JS is the platform answer (`menu-sotto.js`, `explore.js`). 15 files keep the `contextmenu` event |
| `crypto.randomUUID` for 11 `Date.now().toString(36) + Math.random()` ids | secure contexts only. The desktop app serves from `mzlab://localhost` (non-Windows): whether WebKit treats that scheme as secure is unproven, and these ids are file names, not secrets |
| `FileReaderSync` (7 files), the synchronous `XMLHttpRequest` in `libreria-cartella.js:135` | the Rust readers need synchronous reads inside workers; `Blob.arrayBuffer` is async. No replacement exists |
| `naturalCmp` of `ordine.js` to `Intl.Collator({numeric: true})` | the comment states the point: no locale, the same order everywhere; it is also run by node in `tests/test_ordine.py` |
| `innerHTML = ""` (45 sites) to `replaceChildren()` | churn without a size or speed win |
| Desktop base URL (`/Windows/.test(navigator.userAgent) ? "http://mzlab.localhost/" : ...`, 8 files) as one constant | the line is already one line per site; 5 sites live in Draw's iframe and `libreria-*`/`llm` run in other contexts, so a shared global gains about 3 lines for a load-order risk |
| `scroll.js` (223 lines, arrow-key walk), `arrange.js` drag, `shell.js` resize debounce | domain logic; no platform feature does it; `ResizeObserver` is already used where a size is needed |
| `localeCompare`, `Intl`, `AbortSignal.any`, clipboard shims, `DecompressionStream` | already native or already shimmed in `compat.js` with tests |
| `startViewTransition`, container queries, `:has`, `<dialog>`, `inert`, WAAPI FLIP in `transizioni.js` | already used; nothing left to replace |

## How the queue was cut
`docs/coda/CODA.md` got one row and one prompt for each do-now id, ordered by lines removed per risk: CLIP-SAVE-UNIFY, NATIVE-SWAPS-SMALL, CALM-MOTION-HELPER. Every row keeps the parity and guard rules of `docs/coda/BRIEF.md`; none touches a `.wasm` or a cap. After a do-now row lands, add a new allowed-use line to `tools/safari_api_ammesse.txt` only if `tests/test_safari_api.py` asks for it.

## Re-measure
- Inventory: `grep -cE '<pattern>' mzlab/web/*.js mzlab/web/*/*.js` (skip `vendor/`, `elements.js`, `esempi/`); style injection: `grep -lE 'createElement\("style"\)'`.
- Support: install nothing; read the `@mdn/browser-compat-data` JSON through a CDN and print `support.<browser>.version_added` of the path (for example `api.HTMLElement.popover`).
- Floor check: `python3 tools/safari_api.py --list`.
