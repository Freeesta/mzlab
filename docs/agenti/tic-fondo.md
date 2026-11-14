# TIC/BPC without the chemical background (`api/tic_background`)
Engine: `crates/mzlab-core/src/ticbg.rs` (algorithm, pure), `ticbg_route.rs` (route, parameters, JSON), `ticbg_synth.rs` (synthetic run of the tests). Same function in the site (lazy `.wasm` `ticfondo`, crate `mzlab-ticfondo-wasm`, `tic_background`, which opens the `.mzidx` of the file and of its blank itself; worker op `ticbg` through `rust-ticfondo.js`; `lr-map.js`), the app (`rpc.rs` op `ticbg`) and the CLI (`mzlab-cli background`).

## Algorithm
- Bins: unit resolution 1 Da (`bin`, nearest multiple); high resolution (`res=hr`, or the file's `hr1`) `ppm` (10) of the lower power of two of the m/z: width 10-20 ppm, no logarithm, same bits in the `.wasm` and natively.
- Background ion: a bin present in at least `persist` (0.8) of the scans of the level (high resolution: the bin plus its two neighbours, so a line on a bin edge counts whole). Adjacent bins are one ion in the answer.
- Baseline of each ion (`mode=auto`): medians of blocks of `block` (1 %) of the scans, running median of `window` (15) blocks, linear between block centres; a second pass without the points more than 3 robust sigma above the first (the peaks). A peak narrower than about 5 % of the run keeps its area, drift is followed.
- `action=subtract` (default) subtracts the baseline, so a real peak on a background m/z keeps its signal; `action=exclude` drops the whole ion.
- `floor=1` (default, `mode=auto` only): the same baseline of the summed trace goes too (sparse chemical noise that no single ion explains).
- `mode=quiet` (`rt0`, `rt1` in min): constant baseline = the ion's median in that window. `mode=blank` (`blank=K`): the blank's ions persistent in `persist` of its scans are the background, their median in the blank is subtracted.
- `ex=MZ:TOL,...` (TOL in Da, default 0.5): manual windows, every peak inside leaves the trace. `keep=MZ,...`: detected ions left raw (the user put them back).
- BPC: per scan the highest binned ion after subtraction (not the raw single centroid).
- Sources (ideas only, no code copied): persistence of background ions (Windig 1996 CODA; Zhu 2009 BgS-NoRA), control/blank subtraction (Zhang & Yang 2008), local robust baselines best for peak areas on noisy traces (Niezen 2022), sigma clipping of the baseline (González-Fernández 2023 NECTAR).

## Answer
- `{"rt","y","raw","kind","ions":[{mz,lo,hi,persistence,cv,level,share,confidence}],"removed","windows","floor","params"}`: `y` cleaned, `raw` the trace of the index; `cv` = 1.4826 MAD / median where present; `confidence` = persistence / (1 + cv); `share`, `removed`, `windows`, `floor` = fractions of the raw TIC (sum over the run).
- `params`: canonical text (keys sorted, every default written), the text a run record or audit log keeps; the same request replays the same bytes.
- Errors: `err.request.badValue` with `name` (mode, action, res, persist, bin, ppm, block, window, rt0, ex, keep, floor, kind, blank); `filt` or `prec` = not served.

## Interface
- `mzlab/web/tic-fondo-ui.js` (ES module, loaded by `rawSeries` of `grafici.js` on the first TIC/BPC drawing): ONE icon button in its own group of the chromatogram header, shown only on a whole TIC/BPC (no m/z range, precursor or filter); no text on screen, title = aria-label (`ctl.ticbg.on/off`, ≤ 90 characters). Click = `mode=auto`, click again = raw trace (`p.ticbg.on`).
- Right click, long press (550 ms), ↓ or the menu key open the options (`.tb-pop`, `role=dialog`): mode, action, blank file, `rt0`/`rt1` (asked only when both are written), `ex`, `keep`, `persist`, the table of the ions (`TDATI`, with the `params` of each answer). Esc closes and gives the focus back. The URL writes only what differs from the defaults (`TICBG.url`), so it matches `mzlab-cli background --set`. A refused request draws the raw trace and says why (`info`). Not saved with the session.

## Tests
- `cargo test -p mzlab-core --lib ticbg`: on the synthetic run the cleaned TIC recovers each peak area within 5 % (+1.8 %, +4.3 %), peak-to-baseline (apex over median + 3 robust sigma off the peaks) improves 5.2x and 6.3x; a peak on two background lines keeps 99.9 % under subtraction and loses it under exclusion; windows, keep, quiet, blank; high-resolution bins merge a split line.
- `cargo test -p mzlab-cli --lib background`; parity app = `.wasm`: `crates/mzlab-desktop/tests/parity_ticbg.rs` on the synthetic run (`tests/test_parita_rotte_app.py`); in the browser `tests_e2e/e2e_tic_fondo.py` (module fetched on the first request, JSON = `mzlab-cli background`; the button: drawn `y` and bytes = CLI, raw again, options without text, blank = CLI `--blank`, Esc, ↓, ions table).
- Generator twin for manual runs: `python3 tools/tic_fondo_sintetico.py OUT_DIR [--scans N] [--seed N] [--blank]` (mzML + `_truth.json` with the true peak areas).
- Misure del modulo (byte dopo `wasm-bindgen` e `wasm-opt`): TIC senza fondo (`ticbg`, modulo nuovo `ticfondo`): `ticfondo` 142 156 B (tetto 512 000); motore 2 155 135 → 2 155 131 (invariato; con la rotta dentro 2 179 890, oltre il tetto 2 174 976); `rotte` invariato (con la rotta 855 782 → 915 746, oltre 884 736).
