# Engine measurements
Chromium Linux, site built with the Rust engine, same machine, one run unless stated. "open" = loading + first TIC drawn; "TIC+XIC" = TIC and 5 XICs of every file.
Memory: WASM heap = sum of the worker's `WebAssembly.Memory`; IndexedDB = bytes of the saved files; page JS heap from `performance.memory` (not available in workers); RSS = sum of the Chromium processes (an empty page, about:blank, is 679.4 MB). To redo the timings: `python3 tools/build_site.py`, build the Rust as in `pages.yml`, then `python3 tools/bench.py --files 6` (LR files: `MZLAB_MZML`; Chromium: `MZLAB_CHROMIUM`).

## Validated exports and clicks (rule 26: few clicks)

### Export validation
- **mzTab-M** (`export_mztab.rs`): `jmztabm-cli` 1.0.6 (Maven Central, Java 21), run by hand on the Mac on the synthetic file of `tests/export_mztab.rs` (`MZLAB_MZTAB_OUT=<file> cargo test -p mzlab-feat --test export_mztab`: 3 files, 2 rows).
  - Structural validation (syntax and consistency, `-l Info`): 0 errors, 0 warnings; only the Info-2038 "should contain a small molecule evidence section" remains (the SME section is not written: the reference to the source spectrum is missing).
  - Semantic validation (map `cv-mapping/mzTab-M-mapping.xml`): 17 Info-3010 on empty optional fields, 2 Warn-3005, 4 Error-3010 (mandatory in the map, empty).
  - Warn-3005: the unit `MS:1001844` "MS1 feature area" is not among those allowed by the map (children of "intensity unit", "concentration unit", "substance unit"); it stays because it describes the area, while "number of detector counts" does not.
  - Error-3010 on `id_confidence_measure`, `best_id_confidence_measure`, `ms_level` and `identification_method`: the map wants a child of `MS:1002888` ("small molecule confidence measure", only scores from other programs) and the SME rows. Measures the program does not compute are not declared.
  - Summary: jmzTab-m 1.0.6 structural validation = 0 errors and 0 warnings; semantic = 6 findings, all due to (a) no SME row because there is no source spectrum and (b) no identity score from other programs (children of `MS:1002888`); mzLab does not invent them. The file opens with the `COM` line "This file carries quantification only; identification columns are not scored.".

### Clicks of the Tabui flow from the files to the TP table (historical: the page and its measuring test were deleted)
- Counted with capturing listeners in the page (trusted clicks, keys without the modifiers alone). The system file dialog (click on "Choose files...", Ctrl+A, Enter) and the paste of the SMILES (click on the field, Ctrl+V) cannot be driven with Playwright: they are constants declared in the test.
- **Result: 4 clicks + 3 keys = 7 actions**: Tabui tab (1 click), files (1 click + 2 keys), parent SMILES (1 click + 1 key), "Dig" (1 click).
- The test fails above 10 actions (without the logo): one more step between the files and the table is a regression. No confirmation, no "Compute" button in between; the Pipeline is not needed for this path.
- Comparison baseline (`rust-step.md`, "External tests of the chain": mzLab ≤ half the clicks of mzmine 4.10, same result): measured on the Mac with the screen recording of mzmine, batch loaded and batch built (no figure is recorded here). The 13 real `.raw` are not in the repository: the measurement is on the synthetic series, which has the same user steps.

### Clicks of the product network: from the node to the row (`python3 tests_e2e/e2e_rete.py`, line "MISURA clic_rete_riga")
- One click on a node of the network selects the table row (`onSelect(rowId)`); with the keyboard Enter or Space on the node. The test fails above 1 click. `selectRow(rowId)` selects the node from the table without calling `onSelect`.

## Spectral libraries (`python3 tools/bench.py --librerie --gb --scrivi`, Chromium Linux with persistent profile)
"js" = JavaScript path (`?libjs=1`, up to 10^5 spectra), "wasm" = Rust/WASM engine. Times measured from the page (they include the message passing to the worker); identity and analogs = median of 30 and 10 warm requests, "cold" = the first; "identify_all" = 5000 MS2 (400 for MassBank, which has fewer). Memory = peak RSS of the whole browser minus the base before the import. Synthetic spectra (8-40 uniform peaks: they compress little: unfavorable case for the size).
Targets: identity < 5 ms warm, analogs < 100 ms on 10^6, 1 GB MGF < 90 s with ≤ 200 MB. Browser: identity 8-12 ms (message round trip and JSON; <1 ms native), analogs 136 ms on 10^6, 1 GB MGF 130 s and 331 MB (74 s native with 110 MB). MassBank on disk: 8.0 MB against 15.7 MB MSP. `.mzlib` v1 library management: Flash indexes (fragments and neutral losses), warm search 8-12 ms, analogs ~136 ms on 10^6 spectra, decoy support and FDR estimate.

| library | engine | spectra | file_mb | disk_mb | import_s | ident_cold_ms | ident_ms | analogs_cold_ms | analogs_ms | identify_all_s | identify_all_named | rss_peak_import_delta_mb | rss_peak_mb |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| synthetic 1e5 | js | 100000 | 48.6 | 24.8 | 3.0 | 154 | 8.7 | 423.1 | 388.5 | 0.4 | 5000 | 320.0 | 1473.8 |
| synthetic 1e5 | wasm | 100000 | 48.6 | 54.2 | 5.1 | 119.2 | 11 | 165.8 | 76.3 | 2.2 | 5000 | 150.0 | 1375.4 |
| synthetic 1e6 | wasm | 1000000 | 486.9 | 541.7 | 53.4 | 118.8 | 12.4 | 782.9 | 136.4 | 19.9 | 5000 | 251.0 | 1407.6 |
| MassBank (data repository) | js | 17290 | 15.7 | 5.4 | 0.7 | 54.8 | 8.9 | 205.6 | 165.3 | 0.1 | 400 | 123.0 | 1157.6 |
| MassBank (data repository) | wasm | 17002 | 15.7 | 8.0 | 1.0 | 52.3 | 8.3 | 86 | 19.2 | 0.1 | 400 | 54.0 | 1054.6 |
| synthetic MGF 1.04 GB | wasm | 2310000 | 989.2 | 1253.2 | 130.5 | 127.9 | 11.6 | 1703.5 | 148.4 |  |  | 331.0 | 1404.8 |

Native builder (cargo example build; the memory is the peak RSS of the process):
| library | engine | spectra | file_mb | disk_mb | import_s | rss_peak_mb |
|---|---|---|---|---|---|---|
| sint_100000.msp | native | 100000 | 51.0 | 56.8 | 2.9 | 18 |
| sint_1000000.msp | native | 1000000 | 510.5 | 568.0 | 30.2 | 50 |
| massbank_eu_qft_pos.msp | native | 17002 | 16.5 | 8.4 | 0.5 | 9 |
| sint_1gb.mgf | native | 2310000 | 1037.2 | 1314.1 | 74.4 | 110 |

## Feature table match (`UNT_BENCH=1 python3 tests_e2e/e2e_untarget.py`, Chromium Linux, WebAssembly engine)
5000 features with MS2 (12 peaks) searched in a synthetic library of 20 000 entries, positive polarity, 10 ppm on the precursor: reading the table and the MGF 0.1 s, search and levels 0.7 s (threshold 15 s, a single run). With a library of 10^6 entries the cost is that of "Identify all MS2" above (5000 requests in about 20 s). Reading a real `merged_pos.csv` of 58 MB (249 columns) is not measured here: it is done only with `MZLAB_UNTARGET`.

## `.mzidx` index (Chromium Linux with persistent profile)
Site with `.mzidx` in OPFS, Rust engine: median of 3 runs of `tests_e2e/misura_pack.py` (4 CPU) on 15 synthetic Orbitrap HR DDA (61.3 MB, 11,880 scans, 30 entries: MS1 and MS2), 0.41 s/MB. Opening = sending, reading, OPFS write and index; first draw = up to the TIC; reopening = new page in the same profile.

| measure | 15 HR DDA, Rust engine |
|---|---|
| opening (send + read + index), s | 24.84 |
| first draw, s | 0.28 (click on a file, files already indexed) |
| TIC + 5 XIC of every file, s | 2.17 |
| reopening up to the first draw, s | 3.4 |
| XIC on all files (median / max), ms | 33.0 / 53.3 |
| file switch, TIC (median / max), ms | 9.9 / – |
| WASM heap after the work, MB | 32.0 (only the engine worker; the indexing workers are not seen) |
| peak browser RSS, MB | 1688.4 |

## Export / import my data (`python3 tests_e2e/misura_pack.py --parts export`, Chromium Linux, built site, median of 3 runs)
`esporta-dati-worker.js` instantiates the whole engine `.wasm` (3.6 MB, 1.17 MB gzip) only for `zip` / `unzip`. "Start" = from the `new Worker` to the first answer to a one-entry `zip`. The test server does not compress. 50 MB ZIP in OPFS (5 files in `librerie/`, random bytes) with the real `esporta-dati.js`.

| measure | ms |
|---|---|
| cold start (new browser, empty HTTP cache, localhost) | 52 |
| cold start on 10 Mbit/s and 40 ms (uncompressed) | 3234 |
| warm start (HTTP cache and compiled module) | 40 |
| start in the page with the engine already ready (first call / following) | 80 / 75 |
| export of 50 MB: OPFS read (`collect`) | 104 |
| export of 50 MB: `pack` | 34 |
| export of 50 MB: `zip` in the worker (start included) | 421 |
| export of 50 MB: Blob and download address | 46 |
| export of 50 MB: total | 613 |
| import: `unzip` in the worker (50 MB) | 475 |

## Chart drawing on the main thread (`python3 tests_e2e/misura_pack.py --parts draw`, Chromium Linux, built site, median of 3 runs)
From the action to two painted frames; long tasks = `longtask` (> 50 ms) started in the window; long frames = `long-animation-frame` with the longest script. CPU 1x and throttled 4x (CDP). Map: a synthetic Full Scan of 1500 scans × 500 centroids at 1920×1080, "f" = full screen (canvas 1604 → 1698 px). TIC and XIC: 20 synthetic Full Scan. MSn graph: `msngrafo_80.json` (80 ions, SVG, not canvas) opened by `#pick`.

| action | CPU | ms | long tasks (n / max ms) | long frames (n / max ms) | script |
|---|---|---|---|---|---|
| new TIC panel, 20 files | 1x / 4x | 30 / 139 | 0 / 1 (59) | 0 / 0 | – |
| resize the window, 20 files | 1x / 4x | 66 / 339 | 0 / 2 (153) | 0 / 2 (172) | `pannelli.js` (rAF) |
| XIC from one click, 20 files | 1x / 4x | 81 / 312 | 0 / 2 (84) | 0 / 1 (138) | `pannelli.js` (rAF) |
| map: opening | 1x / 4x | 282 / 615 | 0 / 3 (166) | 0 / 2 (106) | `eventi.js` (click) |
| map: full screen | 1x / 4x | 223 / 396 | 1 (51) / 3 (146) | 1 (63) / 1 (127) | `pannelli.js` (rAF) |
| map: resize the window | 1x / 4x | 119 / 545 | 1 (60) / 2 (255) | 0 / 2 (145) | `pannelli.js` (rAF) |
| MSn graph, 80 ions | 1x / 4x | 261 / 487 | 0 / 1 (107) | 0 / 1 (166) | `infusione-grafo.js` (loading) |

## `.mzidx` builder (Chromium Linux 4 CPU)
Rust workers pool. Time = from sending the files to all of them in the list (the index of each is built in the meantime); memory = sum of the Chromium processes (maximum sampled every 0.25 s); `digest` = hash of TIC and 5 XIC of every file; reopening = new page in the same profile. Files: anonymous examples of the site (LR: Full Scan; HR: Orbitrap excerpt repeated up to 140 MB). One run per scenario.

| scenario | files | file_mb | build_total_s | rss_peak_build_mb | first_draw_s | reopen_first_draw_s | rust_passes_max | digest |
|---|---|---|---|---|---|---|---|---|
| 20 Full Scan LR | 20 | 107.3 | 9.51 | 749.5 | 0.34 | 7.91 | 1 | 35255d6425e8 |
| 15 HR DDA (140 MB each) | 15 | 2101.0 | 252.59 | 1321.9 | 1.34 | 11.61 | 3 | 77a5810c63d1 |

Native mzML reader (`cargo run --release -p mzlab-core --example load -- FILE.mzML` and `--example mzidx -- FILE.mzML OUT.mzidx`, 4 CPU, a synthetic Orbitrap file of 101 MB with the spectra repeated 25 times, 19,800 scans and 2.55 M peaks; median of 3 runs): reading and TIC sum 0.85 s; building the `.mzidx` (`build`) 1.16 s. `.mzidx` identical with `cmp` on 37 synthetic files and on the large file, TIC identical.

## Smartphone screen (iPhone 390×844, touch, dev server, median of 5)
| | value |
|---|---|
| first draw (FCP), ms | 88 |
| bytes downloaded until the network is idle | 330 178 |

## Rust reads the index (`python3 tools/bench.py --files 6 --json`, cloud Chromium with WASM, 6 synthetic Full Scan, one run)
The Rust engine answers from the `.mzidx` index (TIC, XIC, filters, scan list, nearest scan) and opens the mzML only for single scans.

| measure | Rust |
|---|---|
| start (page + engine), s | 5.38 |
| opening the files up to the first TIC, s | 2.27 |
| TIC + 5 XIC of every file, s | 0.11 |
| browser RSS, MB | 1033 |

## Map crosshair (`python3 tools/bench_crosshair.py [lr|hr] [--site]`, Chromium Linux)
Full-screen map with the crosshair on. LR: a synthetic Full Scan of 1500 scans × 500 centroids (750 000 peaks); HR: the Full Scan scans of a synthetic Orbitrap (few, ~600 scans). "site" = built site (worker round trip and JS↔worker copies included); "dev" = dev server. One run; times in ms, median / p95.

| measure | value |
|---|---|
| one-point spectrum: total, LR / HR | 4.1 / 2.2 |
| requests, slow sweep, LR / HR | 38 / 36 |
| wait from the point to the exact draw, sweep, LR / HR (median) | 24 / 23 |
| wait p95, sweep, LR / HR | 29 / 30 |
| exact updates during the drag, LR / HR (120 events) | 68 / 90 (plus 132 / 106 previews) |
| 40 arrows pressed in a row, LR: requests / exact updates / wait | 33 / 40 / 21 |
| frame during the drag (median / p95 / max) | 16.7 / 33.4 / 50.0 (LR), 16.7 / 16.8 / 16.8 (HR) |
| crosshair work per frame (median / p95 / max), dev | 0.8 / 1.4 / 4.2 |
| long tasks (> 50 ms) during the drag | 0 |

## Transitions (`transizioni.css`, `transizioni.js`)
Size: 0.9 KB gzip the CSS + 2.2 KB gzip the JS (3.1 KB in all, threshold 4 KB). Cold open on the dev server (median of 7, `loadEventEnd`): 461 ms with, 456 ms without (no regression). While opening/closing the left panel, the right one and as a sheet: 0 *long animation frame* (`e2e_transizioni` prints them); a single `fitWidth` and a single `draw` round per transition.

## Rust and module measurements
`docs/agenti/measurements-rust.md`: Rust flow, ion families, `api/origin`, map and points, composition, profile spectra, peak detectors, HR/DDA, feature detection, map gestures, reader modules, `desktop` profile, table and `.mzidx`, Dig LR, TP interpretation, `.wasm` interp.
