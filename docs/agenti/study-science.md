# Scientific-correctness study of the calculations

The quantities of the engine were recomputed independently (numpy, scipy, `molmass`, `matchms`, `ms_entropy`) on small synthetic inputs; a discrepancy is a finding only after the native engine (`mzlab-cli` or a `cargo test`) reproduces it. Areas: 1 mass and formula, 2 chromatography and quantitation, 3 spectral matching, 4 statistics on feature tables. Status words: FIXED (the engine follows the reference and a test holds it), NOT A BUG, OPEN (owner decision: the fix changes a default, a threshold, a cap or an exported column).

## Fixed findings (one line each; the test holds the value)
- A1 isotope grouping: the M+2/M cap is the averagine value plus the M+2 of the most Br or Cl the mass holds, never below 1.2, so Br2 and Br3 clusters group (`crates/mzlab-feat/src/isotopes.rs`; old cap `IsoParams::legacy_plus2_cap`, off by default).
- A3 mass error sign: every route reports (observed - theoretical)/theoretical (formula finder, library search `dprec`, flow match `mass_ppm`, MS2 matcher `ppm`, in-silico candidate, JS library search, adduct table, `matchres`); test `the_mass_error_is_observed_minus_theoretical` (`crates/mzlab-core/tests/composition_ions.rs`).
- A5 nominal mass: `formula_mz` gives `nominal_neutral` as the sum of the mass numbers of the main isotopes (tristearin 890) and `nominal` of an ion as that plus the adduct shift; the rounded values are the keys `neutral0` and `mz0`. The `nominal` column of `adduct_table.rs` stays the rounded m/z of a unit-resolution instrument.
- B1 feature area: `area_from` defaults to `xic` (trapezoid on the real retention times; the `window` sum was about 2 times low on uneven MS1 spacing in DDA); the unit-resolution recipe keeps `window` (`det.area_from=window`).
- B2 targeted cells: `detect_prominence` default `frac` 0.001 (area/true 0.995-0.997 at sigma 2.5-20 points; the old 0.05 gave 0.876).
- B3 CWT peaks: `detect_full` scales the AsLS lambda with the 4th power of the sampling density and the CWT widths with the points per peak (`Params.scale_to_width`, default on; `false` keeps the values pinned by the Python golden); test `crates/mzlab-picchi/tests/peaks_width.rs`.
- B4 S/N of the line-residual route: the `sn_min` gate divides the residual noise by sqrt(1 - c0(m)) (`residual_shrink`); test `crates/mzlab-core/tests/peak_width_noise.rs`. The edge limits (`area_noise` x noise, the 800 ratio, the valley slope) keep the residual scale they are tuned on (OPEN, owner decision).
- B5 FWHM: `find_peak` interpolates both half-height crossings (`crossing`); error at 3 and 6 points per sigma at most +1.2 %.
- B6 S/N definition: `find_peak` noise is 1.4826 MAD of the RAW trace far from the peak, named by `peak1d::SNR_ESTIMATOR` (old reading `snr_basis=smoothed`; test `peak_snr_basis.rs`). The flank MAD of `quality.rs` and `peakq_lr::noise_sd` are not named.
- B7 area unit: `peak auto-lr`, `peak auto` and the page's `lr_auto_integrate` export counts x s; `area_unit=min` (wasm params slot 6 = 1) keeps counts x min byte for byte; engines keep minutes inside.
- C1 entropy similarity: every route (engine, `libreria-worker.js`, Python reference) returns 0 when either spectrum has no positive intensity; tests `crates/mzlab-core/src/entropy_tests.rs`, `crates/mzlab-lib/tests/empty_spectrum.rs`.
- D1 replicate CV: QC CV and `standard_cv` divide by n - 1 (`qc::cv`); the population divisor is `"cv_ddof":0` in the view spec; test in `crates/mzlab-feat/tests/view.rs`.

## Not a bug
- C3 hybrid search: it differs from `ms_entropy` 1.5.3 because `search_hybrid` runs `np.searchsorted` on uint64 and wraps when a query peak has no library ion below it, dropping valid neutral-loss pairs. The engine follows the definition (0.810011 on the minimal case); `tools/parity_libraries.py` gives every synthetic spectrum a first peak of its own to step around the reference.

## Open owner decisions
- A4 `[M]+` / `[M]-` even-electron ions: SENIOR (and LEWIS when saturated) always fail for a closed-shell cation typed as `[M]+` (choline C5H14NO+ 104.106991 gives `rdb` -0.5 and `rules_failed` LEWIS, SENIOR; tetramethylammonium the same; benzalkonium C21H38N+ gives SENIOR). Place `crates/mzlab-core/src/composition.rs` (`broken`, `half_ok`).
  The rules of Kind and Fiehn (BMC Bioinformatics 2007, 8, 105) apply to the neutral. Proposed fix: for an even-electron `[M]+`/`[M]-` test the rules on the ion -/+ H (half-integer RDBE down to -0.5 and odd valence sum accepted). The rules are only flagged by default; with a rule filter on, the true formula is dropped.
- A6 mass tables: `elements.rs` `MASS` (AME2016, correct to 1e-9 u) differs from `nuclides.rs` and `isotable.rs` (from OpenChemLib), `attributio/review/adduct.rs` `MASS` and the typed shifts in `adducts.rs` and `crates/mzlab-target/src/entry.rs` by up to 2e-5 u per nuclide and 0.15 ppm on adducts; three proton values coexist (`PROTON`, `PROTON_6DP`, H - e).
  Proposed: generate every table from one AME2020 source. The 6-decimal shifts (22.989218, 18.033823, 34.969402) are pinned by the Python reference and the goldens and are the common published values, so regenerating moves exported masses.
- C2 library cleaning (`crates/mzlab-lib/src/clean.rs`): the engine cuts the noise first and merges once; `ms_entropy` 1.5.3 `clean_spectrum` (Li and Fiehn, Nat. Methods 2023, 20, 1475) merges peaks within 0.05 Da repeatedly and cuts the noise last. On 200 synthetic spectra with close peaks the cleaned spectra differ on 176 (entropy similarity of the two cleanings at least 0.957).
  The parity goldens avoid it (peaks at least 0.15 Da apart). Proposed: merge (repeated) before the noise cut; this changes the bytes of every library built afterwards (pinned hash `crates/mzlab-lib/tests/build_sha.rs`) and libraries already built keep the old cleaning until rebuilt.
- D2 PCA 95 % ellipses (`crates/mzlab-core/src/pca.rs`, `ellipse`; reference `mzlab/statistica.py`): the semi-axes use the chi-square(2) quantile 5.991, the spread of a population with known covariance. For n replicates with an estimated covariance the 95 % prediction ellipse (Hotelling T2) is 13.3, 3.45, 2.26, 1.84, 1.36 times wider at n = 3, 4, 5, 6, 10. Proposed: name the ellipse (covariance of the replicates) or switch to Hotelling; the other choice changes the size of every drawn ellipse.

## Suspected (seen once or only read; not confirmed by an engine run)
- arPLS weights (`crates/mzlab-feat/src/untargeted/arpls.rs`: population SD, logistic weight also below the baseline) follow mzmine, not exactly Baek et al., Analyst 2015, 140, 250.
- `peakq_lr::noise_sd` divides the residuals by n - 1; n - 2 is the unbiased divisor.
- `cv_after` of the drift correction (`crates/mzlab-feat/src/drift.rs`) is computed on the QC injections that fit the curve, so it is optimistic; a leave-one-out CV would be fair.
- `auto_integrate` area/true is 0.964 at sigma 40 points and 0.885 at 80 (AsLS lambda fixed in points); `legacy/lr/peakchk.rs` has a non-interpolated FWHM.
- After the log transform a constant feature stays in the PCA matrix; with unit-variance scaling a zero SD needs an explicit rule.

## Checked and found correct
- Masses and ions: the `MASS` values against AME2016/AME2020 to 1e-9 u; electron and proton (CODATA 2018); IUPAC abundances within 5e-4; caffeine, reserpine, glucose, chlorobenzene and the 20-adduct catalogue; loss and contaminant masses.
- Patterns and formula rules: chlorobenzene, caffeine and tristearin isotope ratios; decaBDE unit pattern (`ionpattern`); `D_PLUS2`; averagine; `merge_res` follows R proportional to 1/sqrt(m/z); RDBE, the Seven Golden Rules ratios, Senior conditions, nitrogen rule, Kendrick mass and defect.
- Signal and integration: `savgol` against scipy to 2e-14, `asls_baseline` against Eilers to 1.6e-8, `snip_baseline` to 3e-13, arPLS formulae; `baseline::integrate` and `peak_area` exact; `peakq::quality` asymmetry; XIC window, TIC, BPC, `quant.calibrate@1`, `below_loq`, LOESS and RANSAC of `rtwarp.rs`.
- Spectral scores: cosine and modified cosine equal `matchms` greedy (Watrous et al., PNAS 2012, 109, E1743); the weighted dot product equals `matchms` (Stein and Scott, J. Am. Soc. Mass Spectrom. 1994, 5, 859); the pair entropy similarity follows Li et al., Nat. Methods 2021, 18, 1524; identity, open and neutral-loss library scores equal `ms_entropy` to 1e-6. The assignment is greedy, as in `matchms` (greedy 0.526 against 0.999 optimal on a built case).
- Statistics: Welch t, ANOVA F, Benjamini-Hochberg and log2 fold change equal scipy to 2.2e-14; t, F and chi-square tails; PCA (none/log, none/pareto/auto) against numpy SVD to 3e-14; PCA imputation is the MetaboAnalyst convention; blank filter, normalisation factors, the drift curve against LOWESS to 3e-10.

## Not checked
- Calibration fit, weighting, R2, LOD and LOQ (the engine imports curves and never fits a line); Simpson integration, plate count, USP tailing and resolution are not implemented.
- `mzlab-target`, `ticbg.rs`, `chrom_background.rs`, `intwin.rs`, `broad.rs`, `centwave.rs` numerics; `rtmap::build`; the S/N and integration of the JS page; the m/z quantisation of the index at the XIC bounds; formula annotation of `msngraph`; `attributio/review/isotopes.rs`.
- Spectral matching binning and tolerance edge cases, reverse and forward scores, decoys and FDR. k-means, PQN and Holm do not exist in the engine.
