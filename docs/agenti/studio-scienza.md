# Scientific-correctness study of the calculations

Each finding gives severity, place, expected value with source, observed value, reproduction, one suggested fix and its status: FIXED (the engine follows the reference and a test holds it), NOT A BUG, or OPEN (owner decision: the fix changes a default, a threshold, a cap or an exported column, so the owner chooses).

## Method and coverage

- The implementation and its tests are read, the quantity is recomputed independently (numpy, scipy, `molmass`, `matchms`, `ms_entropy`) on small synthetic cases, and a discrepancy is listed as CONFIRMED only after the native engine (`mzlab-cli` or a throw-away `cargo test`) shows it. Only synthetic inputs are used.
- Area 1 (mass and formula): complete.
- Area 2 (chromatography and quantitation): complete; findings B1 and B7 are re-run a second time, B2-B6 are measured once on the engine and their code path is read.
- Area 3 (spectral matching): the three differences found earlier are analysed to their cause (C1-C3); the edge cases listed under Not checked remain.
- Area 4 (statistics on feature tables): Welch, ANOVA and BH are compared end to end; k-means and PQN do not exist in the engine.
- Severity words: wrong result, precision loss, convention unclear, doc mismatch.

## CONFIRMED - area 1, mass and formula

### A1 - wrong result - isotope grouping rejects M+2/M above 1.2
- Place: `crates/mzlab-feat/src/isotopes.rs:98-103` (`plausible`); the module header says «up to 1.2 (two bromines)».
- Expected: M+2/M of Br2 = 2 x 0.4931 / 0.5069 = 1.95; Br3 2.92; Cl4 1.28; Cl6 1.92 (binomial of the IUPAC abundances, Meija et al., Pure Appl. Chem. 2016, 88, 293).
- Observed (`isotopes::group`, synthetic features, binomial areas, same RT): Br1 areas [100, 97] -> `iso_group` [1, 1]; Br2 [100, 195, 95] -> [0, 2, 2]; Br3 [100, 292, 284, 92] -> [0, 2, 2, 2]; Cl3 -> one group; Cl4 [100, 128, 61, 13, 1] -> [0, 2, 2, 2, 2]. The monoisotopic ion of a compound with 2 or more Br, or 4 or more Cl, stays alone and M+2 becomes the head of the cluster.
- Fix: bound the M+2/M ratio by the binomial of the halogen count that the mass allows instead of the constant 1.2.
- Status: FIXED: the M+2/M cap is the averagine value plus the M+2 of the most Br or Cl the mass holds ((M - 12) / 78.918 Br, / 34.969 Cl; 0.9727 and 0.3199 each), never below the old 1.2; Br2 [100, 195, 95] and Br3 [100, 292, 284, 92] now give one group. The old cap stays selectable (`IsoParams::legacy_plus2_cap`, off by default, written in the recipe text only when on). Tests in `crates/mzlab-feat/src/isotopes.rs`.

### A2 - wrong result - fine pattern cut 6.5 Da above the lightest isotopologue
- Place: `crates/mzlab-core/src/isoscore.rs:133` (`fine_pattern` = `fine_pattern_cut(.., MAX_SHIFT + 0.5)`), used by `isotope_score` (`isoscore.rs:214`).
- Expected: decaBDE C12Br10O, z = -1: 16 unit clusters up to m/z 967.16, base peak 959.17, 955.17 at 50.3 % (binomial of 10 Br).
- Observed: `fine_pattern` returns 5 peaks, 951.1768-955.1727, with 955.1727 at 100 %; `fine_pattern_cut(.., INFINITY)` (the `isofine` route) and `mzlab-cli ion-pattern` give the full pattern. Any formula with 8 or more Br, or an element whose main isotope lies more than 6 Da above its lightest one (Sn 120 against 112), is scored against a truncated pattern.
- Fix: cut on cumulative probability, or relative to the most abundant isotopologue.

### A3 - convention unclear - the sign of the mass error differs between routes
- Place: `crates/mzlab-core/src/composition.rs:716,749` (calculated - observed) against `crates/mzlab-core/src/adduct_table.rs:155` (observed - calculated); library deltas: `crates/mzlab-lib/src/matchres.rs:195-196` (query - library)/library, `crates/mzlab-lib/src/flowmatch/mod.rs:289` (library - query)/query, `crates/mzlab-feat/src/attributio/ms2_engine/matcher.rs:145` (entry - query)/query.
- Expected: (measured - theoretical)/theoretical x 1e6 (Brenton and Godfrey, J. Am. Soc. Mass Spectrom. 2010, 21, 1821).
- Observed: observed 195.088652 against theory 195.08765: the formula finder gives `delta_ppm` -5.126 and `delta_mda` -1, the adduct table gives `ppm` +5.13.
- Fix: one sign and one denominator everywhere, (observed - theoretical)/theoretical.
- Status: OPEN (owner decision): one sign changes the exported `delta_ppm`/`delta_mda`/`ppm` columns of some routes.

### A4 - convention unclear - even-electron cations typed as `[M]+` always fail SENIOR (and LEWIS when saturated)
- Place: `crates/mzlab-core/src/composition.rs:419-441` (`broken`), with `half_ok` for `[M]+`/`[M]-`.
- Expected: a quaternary ammonium is a valid closed-shell ion; the rules of Kind and Fiehn (BMC Bioinformatics 2007, 8, 105) apply to the neutral, so the ion needs the neutral -/+ H before the test.
- Observed: choline C5H14NO+ (m/z 104.106991, ion `[M]+`) -> `rdb` -0.5, `rules_failed` ["LEWIS", "SENIOR"]; tetramethylammonium C4H12N+ the same; benzalkonium C21H38N+ (304.299877) -> `rdb` 3.5, ["SENIOR"]; the radical cation C14H10 (178.077702) -> `rdb` 10, no failure. The rules are only flagged by default; with a rule filter on, the true formula is dropped.
- Fix: for an even-electron `[M]+`/`[M]-` test the rules on the ion -/+ H (half-integer RDBE down to -0.5 and odd valence sum accepted).

### A5 - convention unclear, doc mismatch - `nominal` is the rounded monoisotopic mass
- Place: `crates/mzlab-core/src/elements.rs` (`formula_mz`: `nominal_neutral`, `nominal`), `crates/mzlab-core/src/adduct_table.rs:186-187`.
- Expected: nominal mass = sum of the mass numbers of the most abundant isotopes (Murray et al., Pure Appl. Chem. 2013, 85, 1515): tristearin C57H110O6 = 890.
- Observed: `mzlab-cli ion-mz C57H110O6` -> neutral 890.8302, `nominal_neutral` 891, [M+H]+ `nominal` 892; `mzlab-cli mol-anal` on the same formula -> `nominal` 890.
- Fix: compute it from integer isotope masses, or rename the field (rounded m/z).
- Status: FIXED in `formula_mz` (`ion-mz`, `/api/formula`): `nominal_neutral` is the sum of the mass numbers of the main isotopes (890), `nominal` of an ion that plus the adduct shift (891); the old rounded values are the new keys `neutral0` and `mz0` (the page search keeps using them). OPEN: the `nominal` column of `adduct_table.rs` (the calculator) is still the rounded m/z, which is what a unit-resolution instrument shows.

### A6 - precision loss (at most 0.15 ppm) - four mass tables from different evaluations
- Place: `crates/mzlab-core/src/elements.rs` `MASS` (AME2016, correct to 1e-9 u) against `crates/mzlab-core/src/nuclides.rs` and `isotable.rs` (generated from OpenChemLib), `crates/mzlab-feat/src/attributio/review/adduct.rs` `MASS`, and the typed shifts in `crates/mzlab-feat/src/adducts.rs` and `crates/mzlab-target/src/entry.rs`.
- Expected: AME2020 values (Wang et al., Chin. Phys. C 2021, 45, 030003).
- Observed: `nuclides.rs` differs by 31P -4.9e-7, 32S -4.8e-7, 23Na +3.9e-7, 18O +8.1e-7, 81Br +2.9e-6, 127I -4.2e-6, 56Fe +6.6e-6, 208Pb -1.6e-5, 202Hg -1.8e-5, 195Pt -2.0e-5 u; typed shifts [M+NH4]+ 18.033823 (exact 18.033826), [M+Na]+ 22.989218 (22.989221), [M+Cl]- 34.969402 (34.969401); three proton values (1.007276452 from H - e, `PROTON` 1.00727646688, `PROTON_6DP` 1.007276). `nuclides.rs` lacks some natural isotopes outside its window of 4 (Te 120-125, Kr 78, Xe 124 and 126, Zr 96).
- Fix: generate every table and every typed shift from one AME2020 source.
- Status: OPEN (owner decision): the 6-decimal shifts (`PROTON_6DP`, 22.989218, 18.033823, 34.969402) are pinned by the Python reference and the goldens and are the common published adduct values; regenerated tables move exported masses by up to 0.15 ppm.

## CONFIRMED - area 2, chromatography and quantitation

### B1 - wrong result - the default feature area is about 2 times low on uneven MS1 spacing (DDA)
- Place: `crates/mzlab-feat/src/quality.rs:260-267` (sum of y x median dt), default `area_from` = `window` in `crates/mzlab-feat/src/steps/detect.rs:210-211`.
- Expected: the area is the integral over the real retention times (trapezoid); |log2(area/true)| = 0.
- Observed: `cargo test -p mzlab-cli the_xic_area_follows_uneven_scans -- --nocapture` prints |log2| to the XIC 1.10, 1.08, 1.08, 1.07, 1.05, 1.07 with `window` and 0.0000 x 6 with `xic`; `docs/agenti/mzfeat-prove.md` records -0.48 log2 for `window` on simulated DDA. The bias follows the local scan density, so it changes with co-eluting precursors and is not a constant factor.
- Fix: make `xic` (trapezoid on the real retention times) the default of `area_from`.
- Status: OPEN (owner decision): reproduced (|log2| 1.10, 1.08, 1.08, 1.07, 1.05, 1.07 with `window`, 0 with `xic`); the fix is a new default of `area_from`.

### B2 - wrong result - targeted cells lose 12 % of a clean Gaussian
- Place: `crates/mzlab-core/src/peakq.rs:197-232` (`detect_prominence`: limits where the smoothed trace exceeds 5 % of the apex, area above the straight line between the limits), consumed by `crates/mzlab-feat/src/target/extract.rs:120`.
- Expected: area/true = 1 for a noise-free Gaussian on a zero baseline; cutting at 5 % of the height and subtracting that level leaves 0.888 analytically.
- Observed: area/true = 0.876 for sigma from 2.5 to 20 points, while the cells of the detection route give about 1.0 on the same traces.
- Fix: integrate from zero between the limits with `quality::trapezoid`, or widen the limits to the baseline.
- Status: OPEN (owner decision): reproduced (`detect_prominence` area/true 0.8747, 0.8758, 0.8760, 0.8761 at sigma 2.5, 5, 10, 20 points); either fix changes the 5 % limit (`frac`) or the baseline convention of a function the resolver and `peak-auto` share.

### B3 - wrong result - CWT peak area and FWHM depend on the points per peak
- Place: `crates/mzlab-picchi/src/peaks.rs:325-357` (`asls_baseline(y, 1e5, 0.01, 10, 160)`, lambda in points, CWT widths up to 12); consumers `chrom.uv_peaks@1` (`crates/mzlab-feat/src/uv_bands.rs:60,170`), peak labels, `tracestat`.
- Expected: area/true = 1 and FWHM/true = 1 whatever the sampling rate (Eilers and Boelens 2005: lambda scales with the fourth power of the sampling density).
- Observed (n = 3000, height 1e5, baseline 500, noise 100; the same without noise): area/true 0.987 (sigma 2.5 points), 0.977 (5), 0.924 (10), 0.796 (20); sigma 40 is split into 3 peaks (largest 0.485), sigma 80 into 6; FWHM/true 1.007, 0.996, 0.979, 0.937, 0.833.
- Fix: scale lambda and the CWT widths with the points per peak, or resample to a fixed density before detection.
- Status: OPEN (owner decision): reproduced without noise (largest piece 0.987, 0.977, 0.924, 0.795, 0.485 at sigma 2.5, 5, 10, 20, 40 points; a clean Gaussian also yields 30-40 tiny pieces under 0.2 %); the fix changes the fixed AsLS lambda and CWT widths.

### B4 - precision loss - S/N of the line-residual route is biased high
- Place: `crates/mzlab-core/src/peakq_lr.rs:234-251,316-318` (noise = SD of the Savitzky-Golay residual).
- Expected: the residual SD is sigma x sqrt(1 - c0), c0 the central coefficient, so the estimate must be divided by sqrt(1 - c0), as `crates/mzlab-core/src/peakq_auto.rs:206-209` does.
- Observed: true S/N 100 -> 114 (sigma 1 point), 128 (3 points), 109 (10), 101.5 (25), read through `mzlab-cli peak-edges --set sn_min=`.
- Fix: divide the residual SD by sqrt(1 - c0(m)).
- Status: FIXED: the `sn_min` gate divides the residual noise by sqrt(1 - c0(m)) (`residual_shrink`); median S/N of 20 draws at true 100: 103, 98, 98 at sigma 3, 10, 25 points (before 144, 105, 101), test `crates/mzlab-core/tests/peak_width_noise.rs`. Under 3 points per sigma the smoothed apex is low (86 at 1 point): height convention. OPEN (owner decision): the edge limits (`area_noise` x noise, the 800 ratio, the valley slope) keep the residual scale they are tuned on.

### B5 - precision loss - `find_peak` FWHM is not interpolated
- Place: `crates/mzlab-core/src/peak1d.rs:164-169` (`(rt[b] - rt[a]) * 60`).
- Expected: FWHM = 2.3548 sigma, read at half height by linear interpolation (as `peakext::fwhm`, -1.4 %).
- Observed (dt 0.8 s): 2.40 s against 2.83 (-15 %, sigma 1.5 points); 5.60 against 5.65 clean and 4.80 noisy (-15 %, sigma 3 points); 10.40 against 11.30 (-8 %, sigma 6 points).
- Fix: interpolate the two half-height crossings.
- Status: FIXED: `find_peak` interpolates both half-height crossings (`crossing`); error at 3 and 6 points per sigma at most +1.2 %, at 1.5 points at most +6.1 % (the sampled apex is low), before -43 % to -1 %; test `crates/mzlab-core/tests/peak_width_noise.rs`. `legacy/lr/peakchk.rs` (SUSPECTED below) is unchanged.

### B6 - convention unclear - several unlabelled S/N definitions
- Place: `crates/mzlab-core/src/peak1d.rs` (`find_peak`: smoothed height over 1.4826 MAD of the smoothed trace; manual table and transitions, `manint.rs:22`), flank MAD in `crates/mzlab-feat/src/quality.rs`, `peakq_lr::noise_sd`, CWT ridge, prominence over sigma of differences.
- Expected: one named estimator; the pharmacopoeial one is 2H/h (Ph. Eur. 2.2.46, USP <621>), which gives about 33 for height/sigma = 100.
- Observed: for a true height/sigma_raw of 100 `find_peak` reports 100 (smoothing 1), 166-169 (smoothing 3, the default of the manual table), 184 (smoothing 5).
- Fix: name the estimator beside every S/N and compute the noise on the raw trace (or offer 2H/h).
- Status: OPEN (owner decision): naming the estimator or computing it on the raw trace changes exported S/N columns.

### B7 - convention unclear - areas in counts x s and counts x min
- Place: `crates/mzlab-core/src/baseline.rs` and `peakq.rs` (minutes) against the manual and feature areas (seconds); stated in `docs/agenti/dati-integrazione.md`.
- Expected: one unit for the same quantity (H sigma sqrt(2 pi) = 125.331 counts x min = 7519.88 counts x s for the test Gaussian).
- Observed: `mzlab-cli peak-area g1.csv --from 0.6 --to 1.4` -> 7519.8848; `mzlab-cli peak-auto g1.csv` -> 125.291.
- Fix: counts x s everywhere, with the unit in the field name.
- Status: OPEN (owner decision): one unit changes exported area fields.

## CONFIRMED - area 3, spectral matching

### C1 - wrong result - entropy similarity with a spectrum without intensity is 0.5
- Place: `crates/mzlab-core/src/entropy.rs` (`entropy_similarity`), `crates/mzlab-lib/src/prep.rs` (`entropy_with_query`, behind `pair_scores`, `entropy_from_pairs` and the library scan), the JS fallback `mzlab/web/libreria-worker.js` (`entropySimilarity`), the Python reference `tools/golden.py`.
- Expected: 0, as `ms_entropy` and as the sum of the pair contributions (`contrib::entropy_contributions`) say: an empty spectrum shares nothing.
- Observed: with no peaks, or every peak at 0, S_AB = (S_B + ln 2)/2 and the formula gives 1 - ln 2/ln 4 = 0.5 (0.4999999999999999 on the engine); the JS fallback followed the same formula, the Python reference divided by zero.
- Status: FIXED: every route returns 0 when either spectrum has no positive intensity; tests `crates/mzlab-core/src/entropy_tests.rs`, `crates/mzlab-lib/tests/empty_spectrum.rs`, `tests/test_libreria.py`. No golden contains such a spectrum, so no expected value moves.

### C2 - precision loss - the library cleaning cuts the noise before merging, in one pass
- Place: `crates/mzlab-lib/src/clean.rs` (`clean`, `centroid`), used to build every `.mzlib` and to clean every query.
- Expected: `ms_entropy` 1.5.3 `clean_spectrum` (Li and Fiehn, Nat. Methods 2023, 20, 1475), the order the module header claims: peaks above precursor - 1.6 removed, then peaks within 0.05 Da merged, repeated until none is closer, then the peaks under 1 % of the highest removed.
- Observed: the engine removes the noise first and merges once (a re-implementation in that order matches it on 200 of 200 synthetic spectra with close peaks, the reference order on 25). The cleaned spectra differ on 176 of 200; the entropy similarity of the two cleanings of the same spectrum is at least 0.957 (median 1.000, 4 of 200 under 0.99). Likely cause of the earlier identity scores up to 0.09 under `ms_entropy` (0.911423 against 1.000000; not re-run). The parity goldens avoid it (peaks at least 0.15 Da apart, none under 1 %).
- Fix: merge (repeated) before the noise cut.
- Status: OPEN (owner decision): the fix changes the bytes of every library built from now on (the pinned hash of `crates/mzlab-lib/tests/build_sha.rs`), and libraries already built keep the old cleaning until rebuilt.

### C3 - NOT A BUG - the hybrid search differs from `ms_entropy`
- Observed: 0.673981 against 0.539823 (and two similar cases) on one library entry with shifted fragments; identity, open and neutral-loss agree to 1e-6.
- Cause: in `ms_entropy` 1.5.3 (`flash_entropy_search_core.py`, `search_hybrid`) `np.searchsorted(product_peak_match_idx_max - 1, ...)` runs on uint64 and wraps when a query peak has no library ion below it, so valid neutral-loss pairs are dropped. Minimal case: `ms_entropy` gives 0.515850 alone and 0.810011 once another library spectrum with a 10 Da ion is added; the definition gives 0.810011, the engine 0.810011 without quantisation and 0.810009 through the `.mzlib`.
- Status: NOT A BUG: the engine follows the definition; `tools/parity_libraries.py` gives every synthetic spectrum a first peak of its own to step around the reference bug.

## CONFIRMED - area 4, statistics

### D1 - convention unclear - replicate CV uses the population SD
- Place: `crates/mzlab-feat/src/qc.rs:53-55` (divisor n), reported as `cv_mediano`.
- Expected: CV = s/mean with s the sample SD, divisor n - 1 (ICH Q2(R2), repeatability; Broadhurst et al., Metabolomics 2018, 14, 72 for QC CV).
- Observed: two groups of replicates -> 0.0909 and 0.1859; numpy with `ddof=1` gives 0.1286 and 0.2277, with `ddof=0` 0.0909 and 0.1859. With 3 replicates the CV is low by sqrt(2/3) = 0.816.
- Fix: divide by n - 1.
- Status: FIXED: the QC replicate CV and `standard_cv` divide by n - 1 (`qc::cv`); two replicates now give sqrt(2) times the old value, test `replicate_cv_uses_the_sample_sd_and_the_population_sd_stays_selectable` in `crates/mzlab-feat/tests/view.rs`. The old divisor n stays selectable with `"cv_ddof":0` in the view spec.

### D2 - convention unclear - the 95 % ellipses of the PCA use the chi-square quantile 5.991
- Place: `crates/mzlab-core/src/pca.rs` (`ellipse`), reference `mzlab/statistica.py` (`ellipse`, same constant).
- Expected: 5.991465 is the exact chi-square(2) 0.95 quantile, which describes the spread of a population with known covariance. For n replicates with an estimated covariance the 95 % prediction ellipse uses 2(n-1)(n+1)/(n(n-2)) F(0.95; 2, n-2) (Hotelling T2): 13.3, 3.45, 2.26, 1.84, 1.36 times wider semi-axes at n = 3, 4, 5, 6, 10.
- Observed: semi-axes sqrt(5.991 x eigenvalue): the 3.9e-5 relative difference from the reference ellipses is the rounded constant (sqrt(5.991/5.991465) - 1 = -3.9e-5).
- Fix: name the ellipse (covariance of the replicates) or switch to the Hotelling prediction ellipse.
- Status: OPEN (owner decision): the other choice changes the size of every drawn ellipse.

## SUSPECTED (seen once or only read; not confirmed by a minimal engine run)

- Area 2: arPLS weights (`crates/mzlab-feat/src/untargeted/arpls.rs:76-92,116`: population SD, logistic weight also below the baseline) follow mzmine and not exactly Baek et al., Analyst 2015, 140, 250; largest baseline difference 0.0075 counts under a 4000-count peak.
- Area 2: `peakq_lr::noise_sd` (`crates/mzlab-core/src/peakq_lr.rs:158-188`) divides the line residuals by n - 1; n - 2 is the unbiased divisor.
- Area 2 and 4: `cv_after` of the drift correction (`crates/mzlab-feat/src/drift.rs`) is computed on the QC injections that fit the curve; with span 0.5 and 0.3 on the synthetic set the median QC CV after correction is 0.00 (before 6.15 %), an optimistic figure. A leave-one-out CV would be the fair one.
- Area 2: `auto_integrate` area/true 0.964 at sigma 40 points and 0.885 at sigma 80 points (AsLS lambda fixed in points).
- Area 2: `crates/mzlab-core/src/legacy/lr/peakchk.rs:58` has the same non-interpolated FWHM as B5.
- Area 4: after the log transform a constant feature stays in the PCA matrix (it adds nothing to the scores, which match the reference); with unit-variance scaling a zero SD needs an explicit rule.

## Checked and found correct

- Masses: the 15 values of `MASS` against AME2016/AME2020 to 1e-9 u; electron 0.000548579909 and proton 1.00727646688 (CODATA 2018); the abundances of `isoscore::ISO` within 5e-4 of IUPAC.
- Ions: caffeine neutral 194.0804, [M+H]+ 195.08765; reserpine [M+H]+ 609.28066; glucose [M+H]+ 181.07066, [M+Na]+ 203.05261, [M-H]- 179.05611; caffeine [M+HCOO]- 239.07858; chlorobenzene M+. 112.007429 (electron removed); the 20-adduct catalogue with multimers, multiple charges and `m_if`; adduct differences of `iimn.rs`; loss and contaminant masses (computed from formulas).
- Patterns: chlorobenzene M+1 6.547 %, M+2 32.17 %; caffeine [M+H]+ M+1/M 0.103166, M+2/M 0.008937; tristearin average mass 891.4819, M+1 63.143 %; decaBDE unit pattern of `ionpattern` (maximum at +10); `D_PLUS2` steps; averagine; `merge_res` follows the Orbitrap law R proportional to 1/sqrt(m/z) (a chain merge, also applied to TOF data: convention).
- Formula rules: RDBE coefficients, the element-ratio ranges of the Seven Golden Rules (H/C 0.2-3.1, N/C 1.3, O/C 1.2, P/C 0.3, S/C 0.8, F/C 1.5, Cl/C 0.8, Br/C 0.8), Senior conditions on the neutral of adduct ions, the electron-parity option (nitrogen rule), Kendrick mass and defect (`kendrick.rs`).
- Signal: `signal::savgol_coeffs` against scipy to 2e-14, `savgol`/`savgol_d` equal to scipy `mode='mirror'`, `peakq_lr::sg_filter` to `mode='nearest'`; `centroids/smooth.rs` coefficients -3/35, 12/35, 17/35; `signal::noise_level` 6.965 for a true SD of 7.036; `asls_baseline` against the Eilers reference to 1.6e-8; `snip_baseline` to 3e-13; arPLS formulae and its 5 tests.
- Integration: `baseline::integrate` 125.33141373 = H sigma sqrt(2 pi); `peak_area` with line and zero baselines exact; `peak-auto` limits -0.47 % clean, -1.2 % noisy; `peakq::quality` asymmetry at 10 % 1.99992 on a bi-Gaussian of 2.0.
- By reading: XIC window inclusive, sum, half-width = tolerance; TIC = sum, BPC = maximum; `quant.calibrate@1` = (signal - intercept)/slope x dilution; `below_loq`; LOESS and RANSAC of `rtwarp.rs`; feature table in counts x s.
- Spectral scores on hand pairs (identical, disjoint, two peaks, scaled, shifted, two peaks in one window): cosine and modified cosine equal `matchms` greedy (score and matched peaks; Watrous et al., PNAS 2012, 109, E1743); the weighted dot product equals `matchms` with m/z power 1.3 and intensity power 0.53 and with 3 and 0.6 (Stein and Scott, J. Am. Soc. Mass Spectrom. 1994, 5, 859); the pair entropy similarity equals the definition of Li et al., Nat. Methods 2021, 18, 1524 on unmerged pairs.
- Library search: identity, open and neutral-loss scores equal `ms_entropy` to 1e-6. Convention to know: the assignment is greedy, as in `matchms`; on a built case greedy gives 0.526 where the optimal assignment gives 0.999.
- Statistics end to end: on a synthetic 6 x 12 table with three conditions and a missing cell, `stat::compare` Welch t, p and q, ANOVA F, p and q and log2 fold change equal scipy `ttest_ind(equal_var=False)`, `f_oneway` and `false_discovery_control` to 2.2e-14; PCA imputation (1/5 of the smallest positive value of the feature) is the MetaboAnalyst convention.
- Statistics: two-sided t p-value against scipy (largest relative error 1.2e-7, only at 1e7 degrees of freedom), F upper tail 5.8e-10, chi-square 2.9e-12; Benjamini-Hochberg to 2e-16, with missing values; PCA of `api/stat/pca` (none/log, none/pareto/auto) against numpy SVD: explained variance, scores and loadings to 3e-14; blank filter; reference-difference columns; normalisation factors (sum, median, internal standard); the drift curve against an independent LOWESS to 3e-10.

## Not checked

- Calibration fit, weighting, R2, LOD and LOQ: the engine imports curves and never fits a line; Simpson integration, plate count, USP tailing and resolution are not implemented.
- `mzlab-target`, `ticbg.rs`, `chrom_background.rs`, `intwin.rs`, `broad.rs`, `centwave.rs` numerics; `rtmap::build`; the S/N and integration of the JS page; the m/z quantisation of the index at the XIC bounds.
- Formula annotation of `msngraph`, exhaustiveness of the sub-formula enumeration, `attributio/review/isotopes.rs`.
- Area 3: binning and tolerance edge cases, reverse and forward scores, decoys and FDR.
- Area 4: k-means, PQN (Dieterle et al., Anal. Chem. 2006, 78, 4281) and Holm are not implemented in the engine, so there is nothing to check.
