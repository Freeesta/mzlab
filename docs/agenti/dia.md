# MS2 pseudo-spectra from DIA (`dia.rs`)

From a DIA run (SWATH, MSE) it derives one MS2 pseudo-spectrum per feature: the fragments whose trace follows that of the precursor. Off by default; no MCR deconvolution (`deconvolve.rs` stays DDA-only).

## Engine: `crates/mzlab-feat/src/dia.rs`
- Pure functions on slices (`DiaScan`, `Prec`, `DiaParams`), no files or threads, sums in a fixed order: native and WASM give the same bits. `pseudo_ms2(scans, precs, params) -> Vec<Pseudo>`; peaks `(m/z, intensity at the apex, correlation)` in m/z order. Scans must have their peaks in m/z order.
- Per precursor: window = the MS2 window that contains its m/z (the one with the nearest centre; `iso` absent = all ions, MSE); cycles = MS2 scans of that window with RT in the interval (fewer than `min_points`: no pseudo-spectrum); precursor trace = maximum of the MS1 scans within `frag_ppm`, interpolated onto the cycles; candidates = peaks of the cycle at the apex >= `min_rel` x base peak; kept if Pearson >= `min_corr` and apex within `max_shift` cycles (defaults 0.9; 5; 1; 10 ppm; 0.01).

## CLI: `spectra.ms2@1`
- Parameters `dia_pseudo` (false), `dia_min_corr` (0.9), `dia_min_points` (5), `dia_frag_ppm` (10). With `true` the `features` port is required (absent: `err.flow.port.missing`, `port=features`); RT interval = RT ± `rt_tol`; one MGF record per feature with a pseudo-spectrum, TITLE `<file>.<id>.pseudo`, PEPMASS = the feature m/z, RTINSECONDS = apex. DDA MS2 scans are not written. With `false` the output is the usual one. Isolation window read from `MS:1000827` ± offset `MS:1000828/9`.
- Tests: `crates/mzlab-feat/tests/dia_pseudo.rs` (synthetic batch: 600 s, 4 windows, 40 precursors, 8 interferents per window at apex delta 6-12 s, background and isolated peaks), `crates/mzlab-cli/src/flow_ms2_dia_tests.rs`.

## Measures on the synthetic batch (fragment by fragment, within 10 ppm)
recall 0.981 (true fragments with apex intensity >= 3x noise), precision 0.995, precision on co-eluting pairs 0.994.

## Limits
- Isomers that elute together (apex delta 0): the fragments end up in both (asserted by the test).
- Overlapping windows: the window with the nearest centre wins, they are not merged.
- Unit resolution and real data not tested: synthetic data only.
