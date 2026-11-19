# Scientific-correctness study: mass and formula

Confirmed findings A1-A6 (area 1). Part of the scientific-correctness study; method, status words, suspected findings and coverage are in `study-science.md`.

## CONFIRMED - area 1, mass and formula

### A1 - wrong result - isotope grouping rejects M+2/M above 1.2
- Place: `crates/mzlab-feat/src/isotopes.rs:98-103` (`plausible`); the module header says «up to 1.2 (two bromines)».
- Expected: M+2/M of Br2 = 2 x 0.4931 / 0.5069 = 1.95; Br3 2.92; Cl4 1.28; Cl6 1.92 (binomial of the IUPAC abundances, Meija et al., Pure Appl. Chem. 2016, 88, 293).
- Observed (`isotopes::group`, synthetic features, binomial areas, same RT): Br1 areas [100, 97] -> `iso_group` [1, 1]; Br2 [100, 195, 95] -> [0, 2, 2]; Br3 [100, 292, 284, 92] -> [0, 2, 2, 2]; Cl3 -> one group; Cl4 [100, 128, 61, 13, 1] -> [0, 2, 2, 2, 2]. The monoisotopic ion of a compound with 2 or more Br, or 4 or more Cl, stays alone and M+2 becomes the head of the cluster.
- Fix: bound the M+2/M ratio by the binomial of the halogen count that the mass allows instead of the constant 1.2.
- Status: FIXED: the M+2/M cap is the averagine value plus the M+2 of the most Br or Cl the mass holds ((M - 12) / 78.918 Br, / 34.969 Cl; 0.9727 and 0.3199 each), never below the old 1.2; Br2 [100, 195, 95] and Br3 [100, 292, 284, 92] now give one group. Old cap: `IsoParams::legacy_plus2_cap` (off by default, in the recipe text only when on). Tests in `crates/mzlab-feat/src/isotopes.rs`.

### A2 - wrong result - fine pattern cut 6.5 Da above the lightest isotopologue
- Place: `crates/mzlab-core/src/isoscore.rs:133` (`fine_pattern` = `fine_pattern_cut(.., MAX_SHIFT + 0.5)`), used by `isotope_score` (`isoscore.rs:214`).
- Expected: decaBDE C12Br10O, z = -1: 16 unit clusters up to m/z 967.16, base peak 959.17, 955.17 at 50.3 % (binomial of 10 Br).
- Observed: `fine_pattern` returns 5 peaks, 951.1768-955.1727, with 955.1727 at 100 %; `fine_pattern_cut(.., INFINITY)` (the `isofine` route) and `mzlab-cli ion pattern` give the full pattern. Any formula with 8 or more Br, or an element whose main isotope lies more than 6 Da above its lightest one (Sn 120 against 112), is scored against a truncated pattern.
- Fix: cut on cumulative probability, or relative to the most abundant isotopologue.

### A3 - convention unclear - the sign of the mass error differs between routes
- Place: `crates/mzlab-core/src/composition.rs:716,749` (calculated - observed) against `crates/mzlab-core/src/adduct_table.rs:155` (observed - calculated); library deltas: `crates/mzlab-lib/src/matchres.rs:195-196` (query - library)/library, `crates/mzlab-lib/src/flowmatch/mod.rs:289` (library - query)/query, `crates/mzlab-feat/src/attributio/ms2_engine/matcher.rs:145` (entry - query)/query.
- Expected: (measured - theoretical)/theoretical x 1e6 (Brenton and Godfrey, J. Am. Soc. Mass Spectrom. 2010, 21, 1821).
- Observed: observed 195.088652 against theory 195.08765: the formula finder gives `delta_ppm` -5.126 and `delta_mda` -1, the adduct table gives `ppm` +5.13.
- Fix: one sign and one denominator everywhere, (observed - theoretical)/theoretical.
- Status: FIXED: every route reports (observed - theoretical)/theoretical (formula finder, library search `dprec`, flow match `mass_ppm`, MS2 engine matcher `ppm`, in-silico candidate `mass_ppm`, the JS library search `dprec` in `libreria-worker.js`; the adduct table and `matchres` already did; the denominator is always the theoretical value); caffeine [M+H]+ typed 195.088652 gives +5.13 ppm, test `the_mass_error_is_observed_minus_theoretical` in `crates/mzlab-core/tests/composition_ions.rs`.

### A4 - convention unclear - even-electron cations typed as `[M]+` always fail SENIOR (and LEWIS when saturated)
- Place: `crates/mzlab-core/src/composition.rs:419-441` (`broken`), with `half_ok` for `[M]+`/`[M]-`.
- Expected: a quaternary ammonium is a valid closed-shell ion; the rules of Kind and Fiehn (BMC Bioinformatics 2007, 8, 105) apply to the neutral, so the ion needs the neutral -/+ H before the test.
- Observed: choline C5H14NO+ (m/z 104.106991, ion `[M]+`) -> `rdb` -0.5, `rules_failed` ["LEWIS", "SENIOR"]; tetramethylammonium C4H12N+ the same; benzalkonium C21H38N+ (304.299877) -> `rdb` 3.5, ["SENIOR"]; the radical cation C14H10 (178.077702) -> `rdb` 10, no failure. The rules are only flagged by default; with a rule filter on, the true formula is dropped.
- Fix: for an even-electron `[M]+`/`[M]-` test the rules on the ion -/+ H (half-integer RDBE down to -0.5 and odd valence sum accepted).

### A5 - convention unclear, doc mismatch - `nominal` is the rounded monoisotopic mass
- Place: `crates/mzlab-core/src/elements.rs` (`formula_mz`: `nominal_neutral`, `nominal`), `crates/mzlab-core/src/adduct_table.rs:186-187`.
- Expected: nominal mass = sum of the mass numbers of the most abundant isotopes (Murray et al., Pure Appl. Chem. 2013, 85, 1515): tristearin C57H110O6 = 890.
- Observed: `mzlab-cli ion mz C57H110O6` -> neutral 890.8302, `nominal_neutral` 891, [M+H]+ `nominal` 892; `mzlab-cli mol anal` on the same formula -> `nominal` 890.
- Fix: compute it from integer isotope masses, or rename the field (rounded m/z).
- Status: FIXED in `formula_mz` (`ion mz`, `/api/formula`): `nominal_neutral` is the sum of the mass numbers of the main isotopes (890), `nominal` of an ion that plus the adduct shift (891); the old rounded values are the keys `neutral0` and `mz0` (the page search uses them). OPEN: the `nominal` column of `adduct_table.rs` (the calculator) is still the rounded m/z of a unit-resolution instrument.

### A6 - precision loss (at most 0.15 ppm) - four mass tables from different evaluations
- Place: `crates/mzlab-core/src/elements.rs` `MASS` (AME2016, correct to 1e-9 u) against `crates/mzlab-core/src/nuclides.rs` and `isotable.rs` (generated from OpenChemLib), `crates/mzlab-feat/src/attributio/review/adduct.rs` `MASS`, and the typed shifts in `crates/mzlab-feat/src/adducts.rs` and `crates/mzlab-target/src/entry.rs`.
- Expected: AME2020 values (Wang et al., Chin. Phys. C 2021, 45, 030003).
- Observed: `nuclides.rs` differs by 31P -4.9e-7, 32S -4.8e-7, 23Na +3.9e-7, 18O +8.1e-7, 81Br +2.9e-6, 127I -4.2e-6, 56Fe +6.6e-6, 208Pb -1.6e-5, 202Hg -1.8e-5, 195Pt -2.0e-5 u; typed shifts [M+NH4]+ 18.033823 (exact 18.033826), [M+Na]+ 22.989218 (22.989221), [M+Cl]- 34.969402 (34.969401); three proton values (1.007276452 from H - e, `PROTON` 1.00727646688, `PROTON_6DP` 1.007276). `nuclides.rs` lacks some natural isotopes outside its window of 4 (Te 120-125, Kr 78, Xe 124 and 126, Zr 96).
- Fix: generate every table and every typed shift from one AME2020 source.
- Status: OPEN (owner decision): the 6-decimal shifts (`PROTON_6DP`, 22.989218, 18.033823, 34.969402) are pinned by the Python reference and the goldens and are the common published adduct values; regenerated tables move exported masses by up to 0.15 ppm.
