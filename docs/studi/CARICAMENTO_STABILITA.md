# Loading stability: what moves and what is fetched while files load

Study of the Dati page from page open to the first file shown, on `origin/main` (f771f16) and on the Landing D3 branch `claude/landing-d-open` (1f6604f, main already merged in).
Findings only: no product code was changed.

## Method
- Probe: `tests_e2e/misura_caricamento.py` (headless Chromium, built site with the Rust engine). From the first frame it records, at every animation frame, the box of 27 elements (header, `#avvio-fase`, side panel `#dfiles`/`#flst`/`#fl-up`, `#samptab`, `#tools`, `#opening`, `#wdata`, `#up`, `#start`, `#drop`, `#wtable`, `#hrmix`, `#opbtn`, `#dpanels`, first panel and canvas, `#loading`), the page state (`UPBUSY`, `OPENNOW.busy()`, `data-work`, veil, `scrollY`), every side-list row as it appears, the `layout-shift` entries with their sources, every request, and phase marks.
- Steps of one run: open → engine ready → drag-over + drop on `#drop` (a real `DataTransfer`) → each file read → batch end (on main: click «Load data» 0.5 s later) → first plot → «Sample table» and back → remove a hidden file → remove the shown file → Undo → «+ Load more files» and «Cancel» → reload with the saved session.
- Files (private data repository, not committed): 1 file = `B_FullMass-t0.mzML` (5.8 MB); 5 files = FullMass t0…t30; folder = the 23 SCIEX 3200 QTRAP mzML (FullMass, MRM, MS2); `.wiff` = `MS2.wiff` + `.wiff.scan` (13 MB); MS2+MRM = one of each; LR+HR mix = FullMass-t0 + an 82 MB Orbitrap mzML. Thermo `.raw` was not measured (it needs the separate readers module and an 87 MB file).
- Widths 1400, 800, 400 px; light and dark (5 files at every width): **dark gives the same boxes and shifts as light** in every pair. Second visit (isolation reload skipped) unless «cold».
- Times are ms; one machine, 4 CPU, runs one after the other: compare rows, not absolute values.

## 1. Timeline to the first file (D3 unless «main»)

| run | `#drop` visible | engine ready | drop → 1st progress row | → 1st file read | → batch end | → 1st plot | CLS open → 1st click | `#drop` moves before the drop | veil |
|---|---|---|---|---|---|---|---|---|---|
| 1 file, 1400 | 565 | 1797 | 11 | 795 | 808 | 944 | 0.015 | 0 | |
| 1 file, 400 | 476 | 1936 | 11 | 744 | 856 | 945 | 0.084 | 3 (y 279→341) | |
| 5 files, 1400 | 495 | 1754 | 23 | 1442 | 1825 | 1960 | 0.013 | 0 | |
| 5 files, 1400, cold | 519 | 2337 | 24 | 1435 | 2233 | 2364 | 0.016 | 0 | |
| 5 files, 800 | 566 | 2353 | 27 (hidden) | 1664 | 2206 | 2347 | 0.012 | 0 | |
| 5 files, 400 | 592 | 2200 | 31 (hidden) | 1667 | 2199 | 2325 | 0.420 | 3 (y 279→341) | |
| folder 23, 1400 | 556 | 2296 | 191 | 2598 | 7509 | 7823 | 0.033 | 0 | **yes** |
| folder 23, 400 | 572 | 2329 | 192 (hidden) | 2488 | 7087 | 7269 | 0.069 | 3 | |
| `.wiff`, 1400 | 537 | 2006 | 12 | 2336 | 2715 | 2777 | 0.015 | 0 | |
| MS2+MRM, 1400 | 469 | 2005 | 34 | 610 | 864 | 944 | 0.020 | 0 | **yes** |
| LR+HR, 1400 | 516 | 1875 | 19 | 1070 | 6737 | 6864 | 0.013 | 0 | |
| main 5 files, 1400 | 557 | 2483 | — (table) | click | 1931 | 2344* | 0.004 | 0 | |
| main folder, 1400 | 587 | 2359 | — (table) | click | 4693 | 5137* | 0.040 | 0 | |
| main LR+HR, 1400 | 593 | 2441 | — (table) | click | 8169 | 8597* | 0.004 | 0 | |
| main 1 file, 400 | 589 | 2723 | — (table) | click | 746 | 1165* | 0.419 | 3 | |

\* main includes the 0.5 s the probe waits before clicking «Load data».

Order of events in D3 (5 files, 1400): header and landing painted at ~0.5 s (header buttons keep arriving until engine ready, §4 F1) → engine ready ~1.8 s → drop: the side list gets one grey row per file within 25 ms (`#flst` 0 → 168 px), the landing does not move (`#drop` y = 171 throughout; `#up` is absolute on the landing, `app.css:504`) → files read in parallel, each progress row becomes a file row (row height 59 → 27/52 px, `#flst` moves 6 px when the head line goes) → batch end: in one frame `#start` hides, `#tools` and `#dpanels` show at opacity 0, side panel 300 → 280 px (F4) → first panel built and fades in (160 ms, y 100 → 94) → 500 ms later the page scrolls 5–9 px (F5).
**Main vs D3**: D3 removes the click and the work-area load table; the first plot comes ~0.4 s sooner for 5 files. For the folder D3 is slower: batch end 7.5 s vs 4.7 s (per-file `api/explore` + `bootSession` every 200 ms while the other files are read).

## 2. What is fetched, and when (D3, 5 files, 1400, second visit)
Bytes are uncompressed (as `MAX_TOTAL`); the service worker refetches what the page asks, counted once.

| when | trigger | what | requests / bytes | blocking |
|---|---|---|---|---|
| page open | `index.html` | CSS (`app.css` 94 KB, 4 sheets) + head scripts (`nomi`, `reset`, `telefono`, `browser`, `sito`, `lang/it/*` 31 files) | 42 files, 429 KB | **render-blocking** |
| page open | `carica.js` list `L` | 145 classic scripts and modules | 1 896 101 B (cap 1 900 000: **3.9 KB left**; main 1 885 055) | no (after first paint) |
| page open | `browser.js` at DOMContentLoaded (~1.1 s) | `mzlab_wasm_bg.wasm` 3.6 MB + `mzlab_wasm.js` 95 KB, `rust-worker` | 3.7 MB | no |
| page open | `tpmine-loader.js` (Tabui, «run when the page starts») | 56 files of `static/mzfinder/` incl. `composti.csv` 109 KB | 0.98 MB, outside `L` and the cap | no |
| page open (~1.7 s) | Tabui | `contaminanti.json` | 457 KB | no |
| drop | `upload0` | `impronta-worker`, `idx-worker`, `archivio`, `rust-nativo`, `lettura-sync`, `lettore-esterno`, `rust-opfs`, `decima`… | 14 files, 170 KB | no |
| batch end | first panel | `peaks-worker.js` | 2 KB | no |
| first plot (idle) | `prefetchIdle` (`shell.js:420`) | `openchemlib.js` 1.1 MB, then Teoria (5 files, 118 KB) | 1.22 MB | no |
| ~2–5 s after batch end | Ketcher warm-up (`first-paint.js`, «no file loading») | `ketcher/assets/index-*.js` 29.1 MB + CSS 183 KB | 29.3 MB | no |
| `.wiff` drop | engine | (no extra module: the SCIEX reader is in the main `.wasm`) | — | — |

Open total: 369 requests, 8.2 MB (script 2.8 MB, wasm + JSON 5.2 MB). First visit (cold) is the same plus the service-worker precache; the isolation reload fires at ~230 ms, before the first paint (invisible).

## 3. Right after a file is added (D3)
- No step waits for a click: the first file opens by itself when the whole batch has ended (`OPENNOW.note` → `first`).
- No «Load data», no «Loading finished» / «Caricamento finito» (`conv.row.groupDone` removed), no work-area load table, no card.
- Left over: the full-page veil `#loading` at the auto-open when the first file is of another kind (F2); the side «+ Load more files» still opens the separate «adding» screen (F3); after removing the last file the work area is empty (F7).

## 4. Findings

| # | sev. | what (measured) | cause | proposed fix | zone |
|---|---|---|---|---|---|
| F1 | high | Header buttons arrive after the first paint: `#nav` 126 → 192 → 252 → 329 → 350 px between 0.4 s and engine ready, `#stbar` slides 320 px right. At 400 px the header wraps in three 30 px steps (0.5, 1.3, 2.2 s) and pushes the landing and `#drop` down 62 px: CLS 0.35–0.42 at open, 0.15 at every reload. Same on main. `e2e_primo_disegno` already catches it and is red 8/8 on main and D3 (not in `noti_rossi.txt`). | `sito.js:22` (Teoria), `tpmine-loader.js:96-107` (Tabui placeholder, its real label, Flow), `settings.js:239-241` (gear) | write every header tab in `index.html` with its final label (Tabui/Flow as placeholders the loader fills), or reserve the width of `#nav`; then `e2e_primo_disegno` green | header / Tabui loader |
| F2 | high | D3 still shows the full-page `#loading` veil (1400×900) for ~1.1 s at the auto-open when the first file is not of the current tab (folder of 23, MS2+MRM), again on a click on a file of another kind. Not on main. | `open-now.js:115` `first` → `soloFile` → `setTab` (`lista-file.js:37`) → `tabs.js:35` `loading(true)` | no veil from `setTab` when the panels are built hidden at their final size (as the auto-open already does); keep it only for the world switch | D3 (`open-now.js`), `tabs.js` |
| F3 | high | Side «+ Load more files» (`#addf`) swaps the work area for the «adding» screen: side panel hidden, a second landing (`#start` 900×615–665 at x 250, `#drop` at y 256–306, vs 900×530 at x 403, y 171), CLS 0.19–0.47 in and again on «Cancel». Same on main. | `eventi.js:531` `STATO.aggiunta(true)`; `S.adding` / `leaveAdding` (`shell.js:36-38,387`) | `#addf` opens the file picker (or the drop target of the side list) and stays in the session | D5 |
| F4 | medium | When the first file arrives the side panel narrows 300 → 280 px: work area and toolbar move 20 px left, the first panel widens over 4 frames (1092 → 1102); back to 300 when the last file goes. CLS 0.009 at every first load and every restore. Same on main. | `barra.js:70` `effW` (`byNames` fits the names) | fit to names only upward (`max(defaultW, names)`), or only after a student resize | side bar (`barra.js`) |
| F5 | medium | 500 ms after the first file opens (and after each click on a file) the page smooth-scrolls 5–9 px (the document is a few px taller than the window); at 400 px more. Not a layout-shift entry, but everything under the header moves. | `lista-file.js:38` `soloFile` → `scrollToBlock` (`tabs.js:20`, `scrollIntoView` on a panel already in view) | scroll only when the first panel of the tab is out of view; never on the auto-open | D3 (`lista-file.js`), `tabs.js` |
| F6 | medium | Below 900 px the side list is not shown, so the D3 progress rows are invisible: the landing stays identical for 2.2 s (5 files) to 7 s (folder). The 800 px landing also shows the empty work toolbar (`#tools`: file selector, «+ Chromatogram»…, undo). | `app.css:493` (`sb-first` hides `#dfiles` < 900 px); `#tools` not hidden in landing mode | under 900 px show the `#fl-up` rows in reserved space under `#drop` (or in the overlay `#up`); hide `#tools` while `data-work=landing` | D3 (`upload-rows.js`), D1 (landing CSS) |
| F7 | medium | Removing the last file leaves the work area empty (`#v-workspace` 4 px high, no landing, no table) until Undo. Same on main. | `fileRemoval` / `applyView` with no file left | back to the landing (`data-work=landing`) when no file is in the session | D5 |
| F8 | medium | Reload after «remove a hidden file → remove the shown file → Undo» restores no visible file: the session comes back on the sample table (6/6 runs; plain reload and single remove + Undo restore correctly; main restores correctly). | the Undo of `fileRemoval` does not save the visibility again (`uiSave`) | `uiSave()` after `fileRemoval(...).back()`; e2e: the sequence then reload | D4/D5 |
| F9 | medium | The first file waits for the whole batch: LR+HR mix, LR file ready at 1.07 s, shown at 6.86 s (the 82 MB HR file, then set aside, keeps the batch open); folder: first file read at 2.6 s, shown at 7.8 s. Folder batch end 7.5 s vs 4.7 s on main. | `open-now.js:107` `first` waits for `UPBUSY === 0`; per-file `api/explore append` + `bootSession` (200 ms) while reading | do not count files of the other world (to be set aside) in the wait; batch the appends of a large folder (one `bootSession` per 1 s or at the end) | D3 |
| F10 | low | Reload at 400 px paints `#drop` for one frame (≈8 ms) before the «restoring» skeleton; the skeleton `#opening` is 360 px high vs the 530 px landing, then panels arrive (y 249 → 285). | `first-paint.js` / `applicaIndizioSessione` run after the first narrow-width layout | apply the session hint before the landing box is laid out at every width | D4 |
| F11 | low | Removing or restoring the shown file: the chromatogram panel header (`span.ctl`, `.tbs`) changes height, canvas moves 5 px: CLS 0.02–0.028. | panel header content depends on the file (`pannelli.js`) | fixed header height | panels |
| F16 | medium | A click on a file of another kind (MS2 or MRM after a full scan, batch MS2+MRM+full scan) rebuilds the panels of that tab in view: 13 successive panel shifts, CLS 0.18, more than 500 ms after the click (so not excused as input). | `setTab` → `ensureLayout` (`tabs.js:34-37`) lays the new tab out in place | build the panels of the new tab hidden at their final size, then swap in one frame (the rule of the auto-open) | `tabs.js`, panels |
| F12 | low | At 400 px the first panel is laid out 600 px wide for one frame, then fitted to 388 px. | panel built before `fitWidth` | build at `#dpanels` width | panels |
| F13 | low | Side list: progress row 59 px → file row 27/52 px, head line «N of M ready» goes at the end (`#flst` +6 px). CLS ≤ 0.008. | `upload-rows.js:21-27` | same row height for progress and file rows; keep the head line's space until the batch ends | D3 |
| F14 | info | Start-up list `L` is at 1 896 101 / 1 900 000 B: the next file added to it fails `test_peso_avvio`. | D3 added `open-now.js` + `upload-rows.js` (+11 KB) | move a rarely used script of `L` behind `carica()`; never raise the cap | Coordinator |
| F15 | info | 29.3 MB of Ketcher are fetched 2–5 s after a batch ends and 1.1 MB of OpenChemLib after the first plot; a second drop then competes with them. Tabui (0.98 MB) and `contaminanti.json` (457 KB) are fetched at every open, outside the cap. | `first-paint.js` warm-up; `shell.js:420`; `tpmine-loader.js:129-134` | warm Ketcher only on «Disegno» hover/focus or after a long idle with no upload; count Tabui in a cap of its own | Disegno / Tabui |

**Federico's earlier complaints, checked on D3**
| complaint | D3 |
|---|---|
| top progress bar pushes the drop area down | fixed while reading (`#up` absolute on the landing, `#drop` still at y 171/199); before the drop the header still pushes it at narrow widths (F1) |
| leftover spinner and «Loading finished» | text gone; the spinner veil is still there for multi-kind batches (F2) |
| upload button leads to another page | still: «+ Load more files» (F3) |
| several different landing pages | still four: landing `#start`, the «adding» screen (F3), the restoring skeleton `#opening` (F10), the empty work area after the last removal (F7) |

## 5. Guards
- Existing: `e2e_apri_subito` (D3: `#drop` still while reading, first row < 100 ms), `e2e_carico_stabile` (load table), `e2e_apertura_fluida`, `e2e_primo_disegno` (header + landing at start-up: red, F1), `e2e_cls` (Disegno, Teoria).
- Not covered (worth adding to `e2e_apri_subito` once fixed): side panel width and `scrollY` unchanged from the drop to 1 s after the first plot (F4, F5); no `#loading` with a mixed-kind batch (F2); reload after remove + Undo shows a file (F8).
- Re-measure: `python3 tests_e2e/misura_caricamento.py --site site --files <files> --width 400` (prints shifts per element and phase, requests per phase; `--out run.json` keeps everything).
