# Scientific-correctness study: chromatography and quantitation

Confirmed findings B1-B7 (area 2). Part of the scientific-correctness study; method, status words, suspected findings and coverage are in `study-science.md`.

## CONFIRMED - area 2, chromatography and quantitation

### B1 - wrong result - the default feature area is about 2 times low on uneven MS1 spacing (DDA)
- Place: `crates/mzlab-feat/src/quality.rs:260-267` (sum of y x median dt), default `area_from` = `window` in `crates/mzlab-feat/src/steps/detect.rs:210-211`.
- Expected: the area is the integral over the real retention times (trapezoid); |log2(area/true)| = 0.
- Observed: `cargo test -p mzlab-cli the_xic_area_follows_uneven_scans -- --nocapture` prints |log2| to the XIC 1.10, 1.08, 1.08, 1.07, 1.05, 1.07 with `window` and 0.0000 x 6 with `xic`; `docs/agenti/mzfeat-tests.md` records -0.48 log2 for `window` on simulated DDA. The bias follows the local scan density, so it changes with co-eluting precursors and is not a constant factor.
- Fix: make `xic` (trapezoid on the real retention times) the default of `area_from`.
- Status: FIXED: `area_from` defaults to `xic` (|log2| 0.0000 x 6 on uneven scans, was 1.10 to 1.05); the unit-resolution recipe keeps `window` (also `det.area_from=window`).

### B2 - wrong result - targeted cells lose 12 % of a clean Gaussian
- Place: `crates/mzlab-core/src/peakq.rs:197-232` (`detect_prominence`: limits where the smoothed trace exceeds 5 % of the apex, area above the straight line between the limits), consumed by `crates/mzlab-feat/src/target/extract.rs:120`.
- Expected: area/true = 1 for a noise-free Gaussian on a zero baseline; cutting at 5 % of the height and subtracting that level leaves 0.888 analytically.
- Observed: area/true = 0.876 for sigma from 2.5 to 20 points, while the cells of the detection route give about 1.0 on the same traces.
- Fix: integrate from zero between the limits with `quality::trapezoid`, or widen the limits to the baseline.
- Status: FIXED: default `frac` 0.001 (area/true 0.9952, 0.9953, 0.9966, 0.9966 at sigma 2.5, 5, 10, 20 points, was 0.8747 to 0.8761); old limit `frac` 0.05.

### B3 - wrong result - CWT peak area and FWHM depend on the points per peak
- Place: `crates/mzlab-picchi/src/peaks.rs:325-357` (`asls_baseline(y, 1e5, 0.01, 10, 160)`, lambda in points, CWT widths up to 12); consumers `chrom.uv_peaks@1` (`crates/mzlab-feat/src/uv_bands.rs:60,170`), peak labels, `tracestat`.
- Expected: area/true = 1 and FWHM/true = 1 whatever the sampling rate (Eilers and Boelens 2005: lambda scales with the fourth power of the sampling density).
- Observed (n = 3000, height 1e5, baseline 500, noise 100; the same without noise): area/true 0.987 (sigma 2.5 points), 0.977 (5), 0.924 (10), 0.796 (20); sigma 40 is split into 3 peaks (largest 0.485), sigma 80 into 6; FWHM/true 1.007, 0.996, 0.979, 0.937, 0.833.
- Fix: scale lambda and the CWT widths with the points per peak, or resample to a fixed density before detection.
- Status: FIXED: `detect_full` scales the AsLS lambda with the 4th power of the sampling density and the CWT widths with the points per peak (`Params.scale_to_width`, default on; `false` = old values, pinned by the Python golden); largest piece 0.98-0.987 at sigma 2.5-80 points without noise (before 0.987, 0.977, 0.924, 0.796, 0.485 at 2.5-40), up to sigma 40 with noise; test `crates/mzlab-picchi/tests/peaks_width.rs`.

### B4 - precision loss - S/N of the line-residual route is biased high
- Place: `crates/mzlab-core/src/peakq_lr.rs:234-251,316-318` (noise = SD of the Savitzky-Golay residual).
- Expected: the residual SD is sigma x sqrt(1 - c0), c0 the central coefficient, so the estimate must be divided by sqrt(1 - c0), as `crates/mzlab-core/src/peakq_auto.rs:206-209` does.
- Observed: true S/N 100 -> 114 (sigma 1 point), 128 (3 points), 109 (10), 101.5 (25), read through `mzlab-cli peak edges --set sn_min=`.
- Fix: divide the residual SD by sqrt(1 - c0(m)).
- Status: FIXED: the `sn_min` gate divides the residual noise by sqrt(1 - c0(m)) (`residual_shrink`); median S/N of 20 draws at true 100: 103, 98, 98 at sigma 3, 10, 25 points (before 144, 105, 101); test `crates/mzlab-core/tests/peak_width_noise.rs`. Under 3 points per sigma the smoothed apex is low (86 at 1 point): height convention. OPEN (owner decision): the edge limits (`area_noise` x noise, the 800 ratio, the valley slope) keep the residual scale they are tuned on.

### B5 - precision loss - `find_peak` FWHM is not interpolated
- Place: `crates/mzlab-core/src/peak1d.rs:164-169` (`(rt[b] - rt[a]) * 60`).
- Expected: FWHM = 2.3548 sigma, read at half height by linear interpolation (as `peakext::fwhm`, -1.4 %).
- Observed (dt 0.8 s): 2.40 s against 2.83 (-15 %, sigma 1.5 points); 5.60 against 5.65 clean and 4.80 noisy (-15 %, sigma 3 points); 10.40 against 11.30 (-8 %, sigma 6 points).
- Fix: interpolate the two half-height crossings.
- Status: FIXED: `find_peak` interpolates both half-height crossings (`crossing`); error at 3 and 6 points per sigma at most +1.2 %, at 1.5 points at most +6.1 % (sampled apex low), before -43 % to -1 %; test `crates/mzlab-core/tests/peak_width_noise.rs`. `legacy/lr/peakchk.rs` (SUSPECTED below) unchanged.

### B6 - convention unclear - several unlabelled S/N definitions
- Place: `crates/mzlab-core/src/peak1d.rs` (`find_peak`: smoothed height over 1.4826 MAD of the smoothed trace; manual table and transitions, `manint.rs:22`), flank MAD in `crates/mzlab-feat/src/quality.rs`, `peakq_lr::noise_sd`, CWT ridge, prominence over sigma of differences.
- Expected: one named estimator; the pharmacopoeial one is 2H/h (Ph. Eur. 2.2.46, USP <621>), which gives about 33 for height/sigma = 100.
- Observed: for a true height/sigma_raw of 100 `find_peak` reports 100 (smoothing 1), 166-169 (smoothing 3, the default of the manual table), 184 (smoothing 5).
- Fix: name the estimator beside every S/N and compute the noise on the raw trace (or offer 2H/h).
- Status: FIXED (owner decision: yes) for `find_peak`: noise = 1.4826 MAD of the RAW trace far from the peak, named by `peak1d::SNR_ESTIMATOR` and in the `snr_min` doc of `chrom.transitions`; median S/N of 21 draws at true 100: 99, 97, 95 at smoothing 1, 3, 5 (before 99, 160, 182); old reading `snr_basis=smoothed`; test `peak_snr_basis.rs`. The other two are named too (`quality::SNR_ESTIMATOR` in the `min_snr` doc of `features.detect`, `peakq_lr::SNR_ESTIMATOR`; tests `min_snr_names_its_estimator`, `peakq_lr_snr_name.rs`); the divisor of `peakq_lr::noise_sd` is unchanged (SUSPECTED only).

### B7 - convention unclear - areas in counts x s and counts x min
- Place: `crates/mzlab-core/src/baseline.rs` and `peakq.rs` (minutes) against the manual and feature areas (seconds); stated in `docs/agenti/data-integration.md`.
- Expected: one unit for the same quantity (H sigma sqrt(2 pi) = 125.331 counts x min = 7519.88 counts x s for the test Gaussian).
- Observed: `mzlab-cli peak area g1.csv --from 0.6 --to 1.4` -> 7519.8848; `mzlab-cli peak auto g1.csv` -> 125.291.
- Fix: counts x s everywhere, with the unit in the field name.
- Status: FIXED (owner decision: yes, at the export boundary): `peak auto-lr`, `peak auto` and the page's `lr_auto_integrate` export counts x s by default; `area_unit=min` (wasm params slot 6 = 1) keeps counts x min byte for byte. Engines keep minutes inside; JSON keys unchanged.
