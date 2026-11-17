# Measurements of the Rust engines and modules (continuation of `misure.md`)
Rust flow without Pyodide, ion families, origin of an ion, map, composition, spectra, peaks, HR/DDA, features, gestures, readers, `desktop` profile, TPs.

## Rust flow without Pyodide (`tests_e2e/e2e_lr.py`, "MISURA" line)
Cloud container (Chromium 1194), 2 synthetic Full Scan files (`tools/dati_sintetici.py`), one round. "start→page ready" = from opening the address to the start screen; "opening" = from loading the two files to the first TIC drawn.

| flow | start→page ready (s) | opening (s) |
|---|---|---|
| Rust (default) | 0.46 | 1.3 |
| classic (`?motore=python`) | 2.47 | 1.2 |

Opening in Rust costs building the index, about 0.7 s (worker start, 0.2 s of computation per file, in parallel). Without `indexList` (the synthetic files) the scan reads 1 MB for every spectrum and every synchronous `Blob` read in OPFS costs about 1 ms: `idxread::Windowed` serves the small reads from a 1 MB window and the scan keeps its own buffer (688 reads of 1 MB → 5). With real files (hundreds of MB, normally with `indexList`) not measured.
Progressive opening and early TIC: when the scan table is extracted (~0.05-0.1 s), the worker emits `early_tic` in the progress event. The page shows the provisional TIC at once and answers `api/chrom` requests without waiting for the `.mzidx` to complete. When the index ends, the provisional trace is replaced with exact parity.

## Ion families per file (`cargo run --release -p mzlab-feat --example families -- FILE.mzidx [--truth verita.json]`, 4 CPUs, `analyse_families` only)
Synthetic identity series (9672 features in `t05min`): 0.03 s, correct families 98.9 % (bases with the correct family 181/183), ISF 12/13, adducts 31/31, false ISF declared ISF 0/13, NH4 left open 8/8. Three Orbitrap files of about 80 MB (index 54-56 MB): 141 208 features → 119 317 families in 0.76 s (the noisiest file), 77 197 → 65 645 in 0.32 s, 79 560 → 67 594 in 0.32 s. Two thirds of the time go into reading the traces (`traces::gather` scans almost the whole peak table) and the candidates' m/z windows.

## "Where does this ion come from?" (`api/origin`, `cargo test --release -p mzlab-core --test origin_parity` and `tools/origin_reference.py`, native, real and synthetic LR data, one report with 200 bootstraps, 400 shifts, 25 candidates)
| Engine | Time per report (ms) |
|---|---|
| Python (numpy, `ionfamily.origin_report`) | 320-365 |
| Rust (`origin::origin_report`) | 80-90 |

The time includes ROI, traces, profile similarity with permutation p-value, bootstrap report, kinetics, roles and candidates. In the browser (WebAssembly) not measured here. Parity: same indices and counts, numbers equal within 1e-9, identical Monte Carlo p-values (same `default_rng`).

## RT×m/z map and marked points (`api/map`, `mapsnap`, `mappunti`, `mappunti3d`; `cargo test --release -p mzlab-core --test map_parity -- --nocapture` and `tools/map_reference.py`, native, three real Full Scan files)
| Request | Python (ms) | Rust (ms) |
|---|---|---|
| whole map (first request, cold) | 216 | 12 |
| whole map with background (median along the run) | 20 | 22 |
| difference with `ref` and `norm=max` | 6 (maps already in memory) | 12 |
| region with zoom 120 × 100 | 3.6 | 2.7 |
| snap, point measures, 3D points (300 points) | 0.3-5 | 0.1-6 |

In the browser (synthetic, two files of 2.4 MB, first request of each route, `e2e_lr`): whole map 54 ms in Rust against 46 in an already started Pyodide, with background 102 against 73, region 36 against 24, snap 7 against 2, points 26 against 3, 3D points 22 against 37. The gain is not the speed of a single request (Pyodide already has the table in memory) but that Pyodide does not start: page start 1.3 s against 5.0 s. Parity: cells and points bit for bit equal; of the 3D points cut at `max` the sets are compared by intensity (numpy picks at random among equal intensities).

## Compatible formulas (`api/composition`, `cargo test --release -p mzlab-core --test composition_route_parity -- --nocapture` and `tools/composition_route_reference.py`, native) and XIC from a formula (`api/subxic`, `--test subxic_parity` and `tools/subxic_reference.py`, two real Orbitrap files of 80 MB)
| Request | Python (ms) | Rust (ms) |
|---|---|---|
| composition with element ranges (C 0-20, H 0-40, N 0-5, O 0-8, S 0-2) | 1-7 | 0.1-2.5 |
| composition with the base ranges | 20-44 | 5-7 |
| composition with isotope peaks and fragments | 1-5 | 0.5-2.4 |
| XIC from a formula, 24 000 subformulas at ±5 ppm | 540-660 | 100-220 |
| XIC from a formula, 26 000 subformulas at ±10 ppm | 700-1240 | 140-280 |
| XIC from a formula, 480 subformulas | 9-21 | 9-45 (the time is that of loading the peak table) |

Parity: same candidates in the same order and numbers within 1e-9 for `composition`; identical rows, order and numbers for `subxic` (also on the real files). In the browser not measured: the HR page switches to Python at the first spectrum request, so `subxic` reaches it only from an already started engine; `composition` needs no file.

## Profile spectra (`api/spectrum` and `api/spectra` of a profile LR file; `cargo test --release -p mzlab-core --test profile_parity -- --nocapture` and `tools/profile_reference.py`, native; `e2e_lr` in the browser, synthetic file of 400 scans)
| Request | Python (ms) | Rust (ms) | Browser: Pyodide already started / Rust (ms) |
|---|---|---|---|
| spectrum of a window of 14 scans | 7-11 | 4-9 | 84 / 38 |
| average spectrum of the whole run | 15-20 | 4-5 | 58 / 27 |
| 11 single scans | 14-20 | 8-9 | 38 / 31 |
| 60 single scans | 92 | 29-30 | 220 / 85 |
| one single scan | 1.6-2.7 | 4-5 (one read of the whole index) | – |

Parity: same peaks, same line with the zeros put back, numbers within 1e-9; the points are summed on the grid `rint(m/z / step)` with the step read from twenty scans in the middle of the run, as `Item._step`; if none of them has more than 20 points (a converter that omits zero bins, like the `.wiff` reader) the step comes from the scans of the section that have them (up to twenty, the shortest interval).
Step of sparse profiles (product MS2 and EPI of the 3200 QTRAP: few points per scan, none above 20): common spacing larger than the intervals inside the scans (`sparse_step`: the largest `short / k`, k up to 8, of which every interval is a multiple within 5 %) or, if no scan has two points, of the intervals between the distinct m/z of the section; the 0.06 bin remains even without two close bins (`profile_sparse.rs`, `wiff_sparse_spectrum.rs`). A single point gets a zero at ±step (`pad_zeros`), so its line is visible.

## Peak detectors and quality measures (`peakq.rs` against `peaks::detect_peaks`, native, release, mean of 20 rounds, synthetic 15 min trace with three Gaussian peaks and noise)
| Trace points | CWT (ms) | prominence + local minimum (ms) | quality measures (µs per peak) |
|---|---|---|---|
| 2000 | 5.9 | 0.20 | 2.3 |
| 20 000 | 54 | 9.3 | 20 |

The CWT is the default method (edge and area parity with Python in `golden.rs`); the prominence mode costs a tenth and separates double peaks where the CWT merges them (synthetic tests in `peakq.rs`: partly resolved double, shoulder, tail, noise only).

## HR spectrum and DDA table (`cargo test -p mzlab-core --test hr_parity --release -- --nocapture`, same machine, one round)
Two synthetic Orbitrap DDA files (`tools/hr_reference.py`, ~15,000 MS1 peaks per scan); "rust" = native `Routes::handle`/`spectrum_exact` from the index, "python" = native `api.dispatch` (without the Pyodide cost). Parity ≤1e-9 relative on all values. Times in ms per request:

| request | rust | python |
|---|---|---|
| `api/dda` | 0.7 | 1.4 |
| `api/spectrum` MS1, window of 12 s (about 20 scans) | 2.0 | 2.9 |
| `api/spectrum` MS1, whole file | 59 | 99 |
| `api/spectrum` MS2, whole file | 0.9 | 1.7 |
| `api/spectrum` MS2 with precursor | 0.2 | 0.5 |
| `api/spectrum` MS1, a single scan (reads the mzML) | 1.5 | 1.7 |

## Feature detection (`cargo run --release -p mzlab-feat --example feat -- FILE.mzidx...`, native, 13 real Orbitrap files of about 80 MB with 3.8 M MS1 peaks each)
1 259 216 features in total, the same as `features.detect` file by file; 0.5-0.7 s and 62-78 MB peak per file (Python: 3.5 s and 565 MB); two runs and a budget of 8 MB (peak 42-43 MB) give the same SHA-256.

## Map gestures (`python3 tools/bench_map.py lr,hrc,hrp --wd DIR`, headless Chromium 4 CPUs, WASM engine, synthetic files)
- Full table, native comparison and diagnosis in `mappa-fluida.md`. First map: LR 1500×500 791 ms, HRC 3000×3000 (9 M peaks) 6332 ms (native 254 ms). Sharp after wheel: 176 ms LR, 249 ms HR; RT-only zoom in HR 1617 ms. Warm file switch: 211 ms LR, 441 ms HR. Frames during the wheel p95 33-50 ms. HR WASM peak 175 MB.
- Accelerated colour rendering (`grafici.js`, `mapImage`): 32-bit writing with `Uint32Array` (`buf32` and `tab32`) and sampling at 10 000 cells for the 99.5th percentile: `ImageData` generation on a 600×901 grid reduced from 314 ms to 52 ms (6× faster), warm file switch on HRC ≤ 50 ms.

## Reader modules (`tools/build_lettori_wasm.sh`, after `wasm_opt -Oz`)
- `.wasm` of mzFinder and of the readers: measures, caps (a merged module has a group cap, not the per-module ones) and separate build in `docs/agenti/wasm-tetti.md`. The engine `.wasm` weighs 1 439 240 B (without the Thermo reader, which lives in the `thermo` module).
- Memory of the `thermo` module (`tools/bench_raw.py`, initial memory 32 MB) on the 595 MB DDA BAF (56 061 scans): after `apri` 33.5 MB (`apri` 1.0 s), after `indice` 48.0 MB (186 ms, JSON of 13.7 MB), peak of all steps 122 MB (`to_mzml` 21.8 s); on smaller files 44.4 MB for 7 952 scans and 85.1 MB for 16 524 (before compact memory).

## `desktop` profile (`Cargo.toml`, `opt-level = 3` against `"z"`)
- `mzlab_wasm.wasm` with `cargo build --locked --release --target wasm32-unknown-unknown -p mzlab-wasm`: SHA-256 `d5cca2b7…d00da` before and after adding the profile (identical). Native `.raw` times with `raw_times` and the `desktop` profile (595 MB DDA): table 8.33 → 5.30 s, `.mzidx` 7.07 → 3.59 s, mzML 13.99 → 12.15 s.

## Table and `.mzidx` with one decode (rows straight from `Spectrum`)
- `upload::index_reader` decodes every scan only once for the table and the first pass of `build`, with no mzML head and no re-read; later passes only if the budget is exceeded. `.mzidx` byte-for-byte identical to the road via mzML (`tests/mzidx_rows_direct.rs`). SHA-1 of the head not in the index path (`index_reader` uses the SHA-256 already computed). Weight of `mzlab_wasm.wasm` before `wasm-opt`: 3 409 576 → 3 438 828 B.
- Pass 1 memory efficiency: the MS1 groups of HR files keep `Peak<f32>` (12 B instead of 24 B), passing them to the reserve already rounded, halving the memory footprint during grouping and reducing the build peak RSS.
- LTO of the `desktop` profile (synthetic `mzidx_times`, 570 MB): `lto = "fat"` table + `.mzidx` 4.41 s, build 99 s; `"thin"` 4.16 s, 68 s: difference within the noise, so `lto = true` stays as in `release`.
- Times on real data (BAF, LAB-A, PXD077492, `.wiff`, `.D`): to be measured locally with `cargo run --release --example raw_times -p mzlab-core -- FILE.raw` (phase "table + .mzidx (one call)"); target table ≤ 1.3 s and `.mzidx` ≤ 3 s on the BAF. Without real data: `cargo run --release --example mzidx_times -p mzlab-core`.
- Fast mzML (`examples/mzml_times.rs`, native, 4 CPUs, synthetic centroid mzML of 452.7 MB, 44 000 scans, 28.3 M peaks, browser budget of 64 MB): `.mzidx` without reserve 67.7 s (18 passes) → with reserve 11.6 s (1 pass), same 950.6 MB; reading all peaks 3.5 s, `prepare` 2.9 s (without the whole-file SHA-1).
- mzML writing (`examples/mzml_write.rs`, 2000 scans × 5000 peaks): zlib 1 149.0 MB in 1.60 s, zlib 3 148.6 MB in 3.22 s, zlib 6 148.6 MB in 3.61 s, numpress 67.8 MB in 1.28 s. Engine `.wasm` 1 469 691 B after bindgen and `wasm-opt`. Browser times: to be redone on the real file.

## `.mzidx` rounds with the scratch (`limbench`, native, synthetic profile `tools/mzml_grande.py OUT 2000 20000`: 40 M peaks, 550 MB; N=10 per case, alternated)
Median (min-max) and max RSS, before → after `ROUND_MAX` (32 MB): 64 MB 2684 (2574-2757) → 2601 (2531-2717) ms, 47 → 47 MB; 192 MB 2886 (2834-3062) → 2566 (2523-2899) ms, 111 → 48 MB; 1024 MB 2944 (2912-2999) → 2706 (2659-2861) ms, 527 → 249 MB (pass 1 keeps `budget / 4`). One pass only in every case, identical index SHA.

## LR Dig with several conditions (`cargo test --release -p mzlab-finder-lr-wasm --test lr_groups -- --ignored --nocapture scale`, `N=20` or `N=40`, native, 4 CPUs, synthetic files of `tests/data/gruppi/`)
20 files (3 conditions, blank, standard): indices 90 ms, pipeline + Dig 130 ms, `summary` with `gruppi` 2 ms, memory peak 8.1 MB. 40 files (6 conditions, blank, standard, 2 more replicates): 170-195 ms, 240-300 ms, 3 ms, 11.3 MB. Ratio 40/20: time 1.9-2.3×, memory 1.4×. `finder-lr.wasm` without `wasm-opt`: 1 925 570 → 2 055 819 bytes (+6.8 %).
- Browser (`tools/measure_groups.py`, headless Chromium, site built without `wasm-opt`, 2 files per condition: t0 and t10): Dig from "Run" to the table, 20 files (10 conditions) 0.9-1.9 s (median 1.3 s), 40 files (20 conditions) 2.4-2.9 s (median 2.4 s): ratio 1.8× (single tests 1.5-2.7×, short and noisy times). Browser memory peak during the Dig 770-840 MB with 20 files, 850-915 MB with 40 (1.03-1.1×). First draw of the "Compare conditions" view 38-67 ms.

## Interpretation of the TPs on real data (`tools/valida/tp_timololo.py`)
Metrics (numbers only in the private repository): literature TPs in view, concordant regions, position disagreements to review, new candidates, decoy FDR (`decoys.survived / decoys.rules`), null distribution of the margin, agreement with the mzmine gold (if the list is there).

## Interp `.wasm` (`tools/build_feat_wasm.sh`, `cargo build --release --target wasm32-unknown-unknown -p mzlab-feat-wasm --no-default-features --features interp`)
After `wasm-bindgen --target web` and `wasm-opt -Oz` (binaryen from npm): 496 569 B after `wasm-opt` (388 039 B without the sites and the drawing): above the gate of 470 000 B and below the cap of 512 000 B, which does not rise; the MSn tree is still built by `interp` itself and is an exception accepted for now: `interp` must not grow beyond this; every new weight is paid first by moving the tree into `finder_msn` (a small separate PR). The other `.wasm` files do not change weight.
Tabui click and mzTab-M: `misure.md`, "Validated exports and click".
Viewer gestures, site and native: `tools/bench_viewer.py`; table in `visore-veloce.md`.

## Spectral libraries: `libset_bench` (native, 1 million synthetic spectra, `cargo run --release -p mzlab-lib --example libset_bench -- 1000000 unit|hr [libraries]`)
hr, 1 library: identity 0.06 ms/query warm (0.28 cold); analogs 51 ms; hybrid open search with flat accumulators 49–52 ms against 138 ms for the pair-based version (×2.8). unit, 1 library: identity 1.4 ms (1.7 cold); analogs 73–93 ms; hybrid open 77–106 ms (the machine varies) against 207–230 ms pair-based (×2.1–2.8). unit, 3 libraries of 1M each merged: identity 1.5 ms, hybrid open 70–81 ms per library searched.

## `corsa/1` report of a large run (`mzlab-cli flow report RUNDIR`)
A dense synthetic HR file (`synthetic_data_large.py --files 1 --scans 12000 --compounds 20000`): the `features.group` step writes a 631 MB `ionnet/1` (7.5 million "MS1 correlation" edges: with fewer than `min_corr_samples` files every co-eluting pair is an edge). `corsa_build` reads from each `.json` only the members it uses (`json::parse_members`, the other values checked but not built): 9.1 s and 5 581 MB peak RSS before, 2.0 s and 636 MB after; `corsa/1` byte-for-byte identical on three folders (1, 2 and 24 files); `main` `e6bdc0e4c7dc4da88b5901a2729ee9d624d5fb42`.

## Spectral libraries: "Load folder…" (`tools/bench_librerie_cartella.py`)
Synthetic folder (`tools/librerie_cartella_sintetica.py`: NIST MSP of 110 000 spectra, 29 MB, 4 mixes, 733 other files), Chromium Linux, `mzlab-lib-wasm`, 6 alternated before/after rounds, median, `main` `88cf7b6a5c21bdae8a4c086e23061f8458f35ded`. No long task on the main thread (the import runs in the worker). The `.mzlib` produced is identical by `cmp`.

| step | before | after | % |
|---|---|---|---|
| large MSP, `LIB.add` (`Importer.finish` re-reads the records from the scratch only once: 110 509 OPFS calls → 0) | 2647 ms | 2319 ms | −12 % |
| whole folder (9 files read, 733 ignored) | 3182 ms | 2858 ms | −10 % |
| optimised `.wasm` | 494 207 B | 495 492 B | +0.26 % |

Phases of the large file before: `push` (reading and cleaning) 0.94 s, `finish` (sorting, index, writing) 1.17 s, SHA-256 in JS 0.19 s; scan and folder list ≤ 50 ms. `opt-level 3` on `mzlab-lib`: `push` −35 %, but `.wasm` +23.7 %, discarded.
