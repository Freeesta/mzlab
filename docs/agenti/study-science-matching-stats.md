# Scientific-correctness study: spectral matching and statistics

Confirmed findings C1-C3 (area 3) and D1-D2 (area 4). Part of the scientific-correctness study; method, status words, suspected findings and coverage are in `study-science.md`.

## CONFIRMED - area 3, spectral matching

### C1 - wrong result - entropy similarity with a spectrum without intensity is 0.5
- Place: `crates/mzlab-core/src/entropy.rs` (`entropy_similarity`), `crates/mzlab-lib/src/prep.rs` (`entropy_with_query`, behind `pair_scores`, `entropy_from_pairs` and the library scan), the JS fallback `mzlab/web/libreria-worker.js` (`entropySimilarity`), the Python reference `tools/golden.py`.
- Expected: 0, as `ms_entropy` and as the sum of the pair contributions (`contrib::entropy_contributions`) say: an empty spectrum shares nothing.
- Observed: with no peaks, or every peak at 0, S_AB = (S_B + ln 2)/2 and the formula gives 1 - ln 2/ln 4 = 0.5 (0.4999999999999999 on the engine); the JS fallback followed the same formula, the Python reference divided by zero.
- Status: FIXED: every route returns 0 when either spectrum has no positive intensity; tests `crates/mzlab-core/src/entropy_tests.rs`, `crates/mzlab-lib/tests/empty_spectrum.rs`, `tests/test_libreria.py`; no golden holds such a spectrum.

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
- Status: FIXED: the QC replicate CV and `standard_cv` divide by n - 1 (`qc::cv`); two replicates give sqrt(2) times the old value, test `replicate_cv_uses_the_sample_sd_and_the_population_sd_stays_selectable` in `crates/mzlab-feat/tests/view.rs`; old divisor n: `"cv_ddof":0` in the view spec.

### D2 - convention unclear - the 95 % ellipses of the PCA use the chi-square quantile 5.991
- Place: `crates/mzlab-core/src/pca.rs` (`ellipse`), reference `mzlab/statistica.py` (`ellipse`, same constant).
- Expected: 5.991465 is the exact chi-square(2) 0.95 quantile, which describes the spread of a population with known covariance. For n replicates with an estimated covariance the 95 % prediction ellipse uses 2(n-1)(n+1)/(n(n-2)) F(0.95; 2, n-2) (Hotelling T2): 13.3, 3.45, 2.26, 1.84, 1.36 times wider semi-axes at n = 3, 4, 5, 6, 10.
- Observed: semi-axes sqrt(5.991 x eigenvalue): the 3.9e-5 relative difference from the reference ellipses is the rounded constant (sqrt(5.991/5.991465) - 1 = -3.9e-5).
- Fix: name the ellipse (covariance of the replicates) or switch to the Hotelling prediction ellipse.
- Status: OPEN (owner decision): the other choice changes the size of every drawn ellipse.
