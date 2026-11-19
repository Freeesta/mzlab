# Scientific-correctness study of the calculations

The confirmed findings are split by area into `study-science-mass.md` (A1-A6), `study-science-chromatography.md` (B1-B7) and `study-science-matching-stats.md` (C1-C3, D1-D2). Each finding gives severity, place, expected value with source, observed value, reproduction, one suggested fix and its status: FIXED (the engine follows the reference and a test holds it), NOT A BUG, or OPEN (owner decision: the fix changes a default, a threshold, a cap or an exported column, so the owner chooses).

## Method and coverage

- The implementation and its tests are read, the quantity is recomputed independently (numpy, scipy, `molmass`, `matchms`, `ms_entropy`) on small synthetic cases, and a discrepancy is listed as CONFIRMED only after the native engine (`mzlab-cli` or a throw-away `cargo test`) shows it. Only synthetic inputs are used.
- Area 1 (mass and formula): complete.
- Area 2 (chromatography and quantitation): complete; findings B1 and B7 are re-run a second time, B2-B6 are measured once on the engine and their code path is read.
- Area 3 (spectral matching): the three differences found earlier are analysed to their cause (C1-C3); the edge cases listed under Not checked remain.
- Area 4 (statistics on feature tables): Welch, ANOVA and BH are compared end to end; k-means and PQN do not exist in the engine.
- Severity words: wrong result, precision loss, convention unclear, doc mismatch.

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
- Integration: `baseline::integrate` 125.33141373 = H sigma sqrt(2 pi); `peak_area` with line and zero baselines exact; `peak auto` limits -0.47 % clean, -1.2 % noisy; `peakq::quality` asymmetry at 10 % 1.99992 on a bi-Gaussian of 2.0.
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
