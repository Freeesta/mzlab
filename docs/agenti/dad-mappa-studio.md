# RT-λ map (DAD): study of the card, causes of A/B/D, design of the MS link (C)

Study page of the "RT-λ map (DAD)" card (`panel.title.mapPda`). Study only, no behaviour change. Facts below are read from the code; what could not be run is said so (§6). The detector basics (reader, TWC/XWC, UV spectrum panel) stay in `dad.md`.

## 1. Owner's report (what has to be explained)

- A. Subtracting one DAD map from another file does not work.
- B. The strips next to the map (right: spectrum at the cursor RT; bottom: trace at the cursor wavelength) are empty/flat: cursor RT 17.10, ~220 nm, bottom strip says "XIC 220.8–221.8 · max 0".
- C. New: while hovering the DAD map show linked MS information (MS1 spectrum at the matching RT, EIC/BPC/TIC of the same file) with a DAD→MS RT offset.
- D. The "3D" view of the DAD map does not work either.

## 2. Data flow of the DAD card today

### 2.1 Engine (Rust, all logic)

- Data: `wiff/dad.rs` `DadData { time_min, twc_mau, wavelengths_nm, wl_start/stop/step_nm, matrix_mau (row-major f32, mAU) }`; for an mzML `mzml_dad.rs` (`Mzml::dad_memo`) fills the same struct, so `.wiff` and mzML answer the same JSON.
- Routes (`itemroutes.rs`, `wiff/dad_map.rs`, `wiff/dad_uv.rs`), each only when the file has a matrix, else `err.pda.none`:
  - `/api/map?kind=pda` → `pda_map(m)` → `dad_to_map_json`: base64 f32 matrix + `rt0, rt1, nrt, mz0, mz1, nmz, dmz, agg, kind:"pda"`. It takes NO parameter: no `ref`, no RT/nm window, no `nrt/nmz`, no `bg`. Above 2000 × 500 cells the matrix is block-averaged (`reduce`, `agg:"mean"`).
  - `/api/uvspec?rt0&rt1` → `dad_spectrum_json`: `{nm[], mau[], rt0, rt1, n, unit}` (mean of the rows in the window, nearest row if none).
  - `/api/uvxic?nm0&nm1` (also `/api/chrom?kind=pda&mz0&mz1`) → `dad_wavelength_json`: `{rt[], y[], nm0, nm1, n}`.
  - `/api/uvpropose` → `dad_propose_json` (XWC band from the TWC apex); `/api/delay` → `Routes::delay` → `analog::detector_delay(ta, ya, tb, yb)` = `{delay, peaks, corr}`: cross-correlation of the PDA TWC against the MS TIC (|lag| ≤ 0.6 min, parabolic apex), `null` when the traces share < 1 min or have < 20 points.
- Seams: site `.wasm` (`mzlab-wasm/src/items.rs` `item_pda_map` …, file `mzlab/web/lr-method.js` answers these paths; it returns `null` for any request with `ref=`: line `if (!it || q.get("ref")) return null;`), app (`mzlab-desktop/src/rpc_lr.rs` ops `lr_pda_map`, `lr_pda_spectrum`, `lr_pda_wavelength`, `lr_pda_propose`; test `mzlab-desktop/tests/dad.rs`), CLI: only `mzlab-cli trace chrom FILE --id uv:twc|uv:<nm>±<bw>|uv:auto [--rt a,b]` (`trace_cmd.rs`). There is NO CLI command for the map, for a subtraction, or for the delay.
- The MS map difference (`api/map&ref=&norm=&rttol=`, `maproute.rs`, `mapgrid::diff_tolerant`, `difference.rs`) exists only for MS levels; nothing equivalent exists for the optical matrix.

### 2.2 Card (plain JS, interface only)

- `grafici.js` `drawMap(p)`: `isPda = p.mapKind === "pda"`; `getMap(k, "pda", bg)` → `api/map?k=&kind=pda`; `mapImage` draws; `p._a` carries `{x0,x1,y0,y1 (y = nm), f, pda:true, ref, mat, mapv, hov}`.
- `pannelli.js` (map controls, ~l.750): file selector, `mapKind` MS/DAD selector (`pdaMapOk` probes `api/map?kind=pda` once per file), `corrWidget` (the Correction menu, option "a file"; chip `corr.file.short` = "− <label>"), `norm` select (enabled once a file is chosen), colours, then `MAPPA.decorate` adds mark / snap / sight / 3D.
- `mappa.js`: the sight (mirino) and the two strips: `show` → `drawSide` (right) / `drawBottom` (bottom); exact data via `specFor/fetchSpec` (`api/scan_axis` + `api/spectra`, MS scans) and `xicFor/fetchXic` (`api/xic?mz=&tol=`, MS XIC); preview from the page matrix `preview()` (`a.mat`) while the exact answer is in flight.
- Other pieces: `uv-spettro.js` (UV spectrum panel, `sliceUv` cuts the page matrix, `wlOpen` trace at a λ), `visualizza.js` `DISP.pdaShift/pdaDelayValue` (the existing "Align DAD and mass" display shift: `p.pdaAlign`, `p.pdaDelay` editable, default = measured `api/delay`), `mappa3d.js` (WebGL2 viewer), `dad-mrm.js`.

## 3. Cause of A (subtraction): a silent no-op, proven in the code

1. `grafici.js` `drawMap`: `const rf = !isPda && se && …` — on a DAD map the reference file is never loaded, so `B = null` and the plain matrix is drawn. The controls do not know this: `corrWidget` lists every scan file (also files without a DAD matrix), `MAPPA.setCorr(p, "f<k>")` stores `p.corr="file"`, `p.ref=<k>`, the chip shows "− <file>", the `norm` select is enabled, the legend does not change. So the owner picks the file, sees the chip, nothing changes: "no effect", no message.
2. Even if `B` were built, the engine could not serve it: `pda_map` takes no `ref`; `lr-method.js` drops every request with `ref=` (`return null`), and `diffMap` calls `api/map?level=…&ref=` which is the MS route.
3. A common grid does not exist anywhere: two DAD files differ in RT sampling (rows at `time_min[i]`, not uniform; the map payload only gives `rt0, rt1, nrt`, so the card treats the rows as uniform), in wavelength start/stop/step (`wl_start_nm, wl_stop_nm, wl_step_nm`, and block-averaged `dmz` above 500 bins) and in gain/dilution. Without a rule `A[i,j] − B[i,j]` would compare different times and nm.
4. Not reproduced in a browser here (§6); the three points above are enough to explain the symptom, and a Playwright step with two synthetic `.wiff` pairs (`tools/wiff_riferimento.py` `dad=`) will pin it in step 3.

## 4. Cause of B (strips empty): the strips are MS strips, fed with the UV cursor

On a DAD map the sight point is `(rt, nm)` but the strips code is the MS one, unchanged:

- Right strip: `fetchSpec` takes the MS scan nearest to the cursor RT (`api/scan_axis`, `api/spectra`, level `f.lv`) and `drawSide` plots it against the map's vertical axis, whose range `a.y0..a.y1` is the nm range (≈ 200–400). `drawSide` keeps only peaks with `mz` inside that range: an MS1 spectrum with nothing between m/z 200 and 400 gives `ymax = 0` → text "no peak in this m/z range" (`mappa.side.empty`), i.e. blank.
  Bars that do fall in the range are MS peaks drawn against a nm axis (wrong data, plausible look). For a file with no scans (`.wiff` MRM/SIM + DAD) `scan_axis` fails and only the page-matrix preview is drawn, marked "preview".
- Bottom strip: `fetchXic` calls `api/xic?mz=<cursor nm>&tol=<xicTol>` — a mass trace at m/z ≈ 221 (window 220.8–221.8 = nm cursor ± low-resolution tolerance) — and `drawBottom` titles it `XIC … · max <n>`. In the owner's screenshot there is nothing at m/z 221 at RT 17.10 so the trace is flat and says `max 0`. The wording "XIC" confirms the MS route is used for a UV cursor.
- The UV data the strips should show is already in the page: `A.m` (matrix) → `preview()` builds the column (spectrum at the RT) and the row (trace at the nm) with `MIRINO.column/trace`, but it is only the fallback and is replaced as soon as the MS answer arrives. The exact UV routes exist in the engine (`uvspec`, `uvxic`), plus `sliceUv` in `uv-spettro.js`.
- Subtracted maps: not applicable today (A); the design must feed the strips from the subtracted matrix (§5.3).
- Fix (step 4, card): in `show/drawSide/drawBottom`, when `a.pda`, take spectrum and trace from the page matrix (exact: the matrix IS the data, nm axis, "mAU", titles "UV spectrum at RT" / "UV trace at λ", no "XIC"), never from the MS routes. Proof by e2e: hover a synthetic DAD peak, strips non-empty with the peak at the right nm / RT.

## 5. Cause of D (3D) and design of C

### 5.1 3D: the viewer never learns the map is optical

`mappa3d.js` `open(p)` builds its request from the MS state: `api/map?k=&level=<f.lv>&rt0&rt1&mz0&mz1&nrt&nmz&mzscale=lin|log[&bg][&ref=]`, with `mz0..mz1 = a.y0..a.y1` (nm numbers read as m/z) and no `kind=pda`. Consequences, from the code: (i) a file with MS scans gets its MS map in the m/z window 200–400, drawn as if it were the DAD map, with "m/z" labels (wrong);
(ii) a DAD-only/no-scan file gets an error from the MS route (`msg(T("m3d.err.load"))`); (iii) `files = scanFiles(…)` offers only files with scans; (iv) a subtracted map cannot arrive (A). The WebGL2 side (`makeContext`, `Fail("nowebgl2")`, `webglcontextlost`) is a separate failure class that the viewer already reports with a technical box; whether the owner's browser has WebGL2 is not known and is not needed for the cause above.
Plan: step 5 teaches the viewer `kind=pda` (axes in nm, "mAU", data = the page matrix or a `dad map` region route), and checks WebGL2/lost-context handling with the existing diagnose box.

### 5.2 Formats and what carries the offset

| Source | DAD matrix | Offset DAD→MS in the file? |
|---|---|---|
| SCIEX `.wiff` (3200 QTRAP, `Device_N/DAD`) | decoded: matrix, TWC, times in the `DADIdx` rows (`wiff/dad.rs`) | not found: nothing in `DADSampleHeader` or the method decoded so far; the lab compensates with the measured `/api/delay` (tubing/flow-cell delay) |
| mzML (converted `.wiff` variant, ProteoWizard, `MS:1000804` spectra or `dad_matrix`) | read by `mzml_dad.rs` | no standard term; RT of the optical spectra is written as in the source |
| Agilent `.D` | NOT decoded (`AcqData/DAD1.*` listed with size/SHA only; `data-channels.md` §1.7) | not read; the card shows `uv.none` |
| Shimadzu | NOT decoded | – |
| Thermo `UV_VIS_n` | no matrix (`uv.none`) | – |
| Waters MassLynx `.raw` | only analog traces (`_CHROMS.INF`); no PDA matrix reader | – |

So the default offset is 0 unless the user edits it or accepts the measured delay; the code that exists for this (`DISP.pdaShift`, `p.pdaDelay`, `/api/delay`) is reused, not duplicated. If a reader gains a metadata delay later it fills the same default ("from file", shown with its source).

### 5.3 Engine design (step 2: Rust first, parity site = app = CLI byte for byte)

New module in `mzlab-core` (feature `mappa-dad`, no logic in JS), pure functions on `DadData`:

- `dad_subtract(a, b, opt)` → matrix `A − scale·B'` on A's grid, where `B'` is B resampled explicitly: RT by linear interpolation of B's rows at A's `time_min` shifted by `rt_shift_b` (min, default 0); nm by linear interpolation of B's channels at A's wavelengths; cells outside B's RT or nm range are NOT filled with 0: they are marked (`NaN` → written as 0 plus an `overlap` mask / reported ranges).
  Result JSON = the `map` JSON of `dad_to_map_json` (same keys) plus `"subtract":{ref, grid:{a:{rt,nm,step}, b:{rt,nm,step}}, resampled:{rt:bool, nm:bool}, overlap:{rt:[..], nm:[..]}, rt_shift_b, scale:{mode, value}, warnings:[codes]}`. Warning codes (stable): `grid.rt.interpolated`, `grid.nm.interpolated`, `grid.nm.partial_overlap`, `grid.rt.partial_overlap`, `grid.step.coarser` (B coarser than A: detail lost), `scale.auto.low_corr`.
- Scale option: `none` (default), `max`, `tic` (same meaning as the MS `norm`), `factor:<x>`, `fit` (least squares `A ≈ k·B` over the overlap, k and r shown). Every automatic choice is returned in the JSON and logged by the card; nothing is hidden or blocked.
- `dad_spectrum_at`/`dad_trace_at` on a plain or a subtracted matrix, `dad_ms_at` (§5.4), `dad_delay` (wraps `detector_delay` and returns `{delay, peaks, corr, apex_dad, apex_ms}` so the evidence can be shown).
- Routes: `/api/dadsub`, `/api/dadspec`, `/api/dadtrace`, `/api/dadms`, extending `/api/delay`; wasm `item_*`, desktop `lr_pda_*`, CLI:
  - `mzlab-cli dad subtract A B [--rt-shift MIN] [--scale none|max|tic|fit|factor:X] [--out FILE.json|csv] --json`
  - `mzlab-cli dad spectrum FILE --rt R [--rt-end R2] [--ref B …]`, `dad trace FILE --nm N [--bw W] [--ref B …]`, `dad ms-at FILE --rt R [--offset MIN] [--trace tic|bpc|eic --mz M --ppm P]`, `dad delay FILE`.
  - `--json`: one document per call with stable keys (above), exit codes as the other commands (`cli.md`: 0 ok, 2 usage/unreadable, plus a distinct code for "no DAD matrix" and "no overlap"; to be fixed when step 2 reads `cli.md`).
- Parity tests: for each route the site answer (Node/wasm test harness) = the app op = the CLI `--json` on the same synthetic pairs (`tools/wiff_riferimento.py` `dad=` and `tools/dati_sintetici_uv.py`), byte for byte, as `routes_parity.rs`; cases: same grid, different RT grid, different nm range and step, no overlap, no DAD, large matrix (reduced).
- Wasm cost: the pure functions are small; measure per `wasm-caps.md` (`mzlab-wasm` cap; ≤ 1 % over the old cap accepted with the `misura` entry of its `tools/moduli/*.toml`; above → stop and ask). Nothing is gated out of a module.
- Data to reproduce with: synthetic only; files of `mzlab-dati` (via `tools/dati_percorsi.py`) only for local checks, never committed.

### 5.4 Card design for C (linked MS information)

The DAD map already has two strips; C must not take them away. Proposal (mockup):

```
 ┌───────────── toolbar: file ▾  DAD ▾  – to <file> ▾  scale ▾ │ DAD→MS offset [+0.00 min] [estimate…] │ mark snap sight 3D ┐
 │                                              │ right strip: [UV | MS] toggle                                 │
 │   RT-λ heat map (nm up, RT right)            │   UV  = absorbance vs nm at the cursor RT   (default, B fix)  │
 │            ┼  cursor (RT_dad, nm)            │   MS  = MS1 spectrum (m/z) at RT_ms = RT_dad + offset         │
 │                                              │         of the SAME file (file A if the map is A − B: said)   │
 ├──────────────────────────────────────────────┴───────────────────────────────────────────────────────────────┤
 │ bottom strip: [UV | MS: TIC / BPC / EIC at m/z of the clicked MS peak] ; the MS trace is drawn on the DAD RT   │
 │ axis shifted by the offset; the cursor line sits at RT_dad on UV, at RT_dad+offset on MS                       │
 └──────────────────────────────────────────────────────────────────────────────────────────────────────────────┘
```

- Strips keep UV by default (B fix). A UV/MS toggle per strip, plus an "both" option that stacks a second small strip when the panel is tall enough (decide with the owner; proposal: toggle first, stacked later).
- Offset: signed minutes, editable field in the toolbar; default order: the value the user already chose in "Align DAD and mass" (`p.pdaDelay`, one source of truth) → the file metadata value if a reader provides one (none today) → 0, always shown with its source ("default", "from file", "estimated", "user"). Applied the same way everywhere: `RT_ms = RT_dad + offset` for the MS spectrum, the MS trace (shifted back for display) and the cursor line.
- Estimate: button runs `dad delay` (existing `detector_delay` on TWC vs TIC; optionally restricted to the visible RT window); the card shows the evidence (peaks used, correlation, the two apexes as small markers on a mini overlay of TWC and TIC) and the proposed value; the user accepts or edits it; the choice is logged in the log of automatic choices. Never applied silently.
- Subtracted map: MS comes from the first file (A) by default, written on the strip ("MS of <A>"); a selector lets the user pick B instead. UV strips come from the subtracted matrix (signed, zero line).
- Files with no MS scans (DAD + MRM/SIM only): MS toggle disabled with the reason; the TIC/MRM traces can still be shown if the file has them.
- Data path: the card asks the engine (`dad ms-at`, same op as the CLI) for `{rt_ms, scan, spectrum(mz,y), trace(rt,y)}`, so the MS side is computed once in Rust and parity-tested; the card only draws (reusing `drawSide/drawBottom` drawing code with a mode flag, not a copy).

## 6. What was and was not verified

- Verified by reading the code: §3 (1–3), §4, §5.1 (request built by `mappa3d.js`), the route list and parameters in §2.1, the existing delay estimator.
- NOT reproduced live: the container has the Rust wasm32 target but no `wasm-bindgen` CLI, so the site's engine `.wasm` could not be built (`site/static/wasm` absent; `e2e_mappa_pda_wiff.py` itself stops at a 404 of `mzlab_wasm.js`). No file of `mzlab-dati` was touched. The live repro (two synthetic `.wiff` pairs with different grids; hover at the cursor; open 3D; console and request log) is the first step of step 3/4 on CI, where the site is built; if it contradicts a statement here, this page is corrected there.
- The owner's actual files and browser (WebGL2, GPU) are unknown: item D's cause in §5.1 is independent of them; a second cause (context lost, huge grid) cannot be excluded and would show in the 3D technical box.

## 7. Split into small PRs (one PR each, after the owner's go)

1. This page (docs only).
2. Engine: `dad_subtract` + regridding + scale + `dad_spectrum_at/dad_trace_at/dad_ms_at/dad_delay` in `mzlab-core`; routes in wasm and app; CLI `dad subtract|spectrum|trace|ms-at|delay` with `--json`; parity tests site = app = CLI byte for byte; `misura` entry in `tools/moduli/*.toml`; `docs/agenti/dad.md` updated.
3. Card A: DAD subtraction (load `ref`, offer only files with a matrix, scale control, an always-visible note of grid changes/overlap/warnings, chip and legend say what is subtracted; no silent no-op), e2e with two synthetic pairs of different grids.
4. Card B: strips show the UV spectrum and UV trace (plain and subtracted), titles and units fixed; e2e on hover.
5. Card D: 3D for `kind=pda` (plain and subtracted), WebGL2/lost-context path checked, e2e where the browser has WebGL2 (else the technical box is asserted).
6. Card C: DAD→MS offset field (shared with "Align DAD and mass"), estimate with evidence, UV/MS toggle on both strips, subtracted-map file choice; e2e.
7. Docs and i18n sweep (`lang/en`, `controlla_i18n.py`), `dad.md`, INDEX rows.

Questions for the owner before step 2: (a) default scale for `subtract`: none (proposed) or fit; (b) toggle first, stacked strips later (proposed) or both at once; (c) Agilent `.D` DAD and Waters PDA are not decoded: keep them out of scope here (proposed; a reader is a separate task).
