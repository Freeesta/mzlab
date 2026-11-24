# Screening rules for transformation products at unit resolution: audit and specification

Input: a field vademecum of an expert who looks for transformation products (TP) on a low-resolution triple quad (EMS NEG, Q1 POS, ion counting). Core idea: at unit resolution every nominal mass is a hypothesis; a TP is accepted only when kinetics, controls and ion consistency tell the same story. The expert asks that the first five rules become automatic checks. Placeholders: parent P, products TP1, TP2; no real compound, sample or study name.

Part A checks each rule against the code (an earlier report claimed missing commands that exist, so every row names the file). Part B specifies the rules as machine-checkable checks. Part C is the order of the implementation chats. Rule R1 is implemented with this study (`mzlab_core::ionstat`, `mzlab-cli peak noise`, mzlab-wasm `noise_floor`).

## A. Rule -> exists? where -> gap
Dig = the LR Dig (`crates/mzlab-feat/src/legacy/lr/`, Flow step `tp.dig_lr`, site `finder-lr` `.wasm`), after DIG-ROBUST (#3675, merged).

| rule of the vademecum | exists? | where | gap |
|---|---|---|---|
| filter against blanks | yes | `labels.rs` criterion "absent in the blank": max blank area / max treated area <= 0.20 (`RATIO_MAX`); `points.rs` `controls.blank` (detected areas only); `table.filter` `blank_ratio` on feature tables | blank and dark control are pooled in one criterion (`kind` blank or control) |
| filter against dark controls | yes, pooled | role `dark` -> kind `control` (`plan.rs::apply_sample`), same criterion as the blank; `controls.dark` as evidence; parent decay judged per condition (`parent.rs`) | no criterion of its own (a TP formed in the dark, i.e. hydrolysis, reads as "in the blank") |
| filter against standards | evidence only | `controls.standard` (`points.rs`); standards are never kinetic points (`plan::columns_for`) | no rule: an ion present in the parent standard is an impurity or an in-source fragment (R2 uses it) |
| filter against washes | partial | `blank_type` `wash` exists in `samples/1` (`samples/mod.rs::BLANK_TYPES`, "no reference"); the Dig does not read `blank_type`, so a wash counts as a blank | carry-over check (an ion in the wash after a strong sample) absent |
| persistence, S/N >= 5 in 2 points | yes | `peakchk.rs`: a cell is a peak with S/N >= `min_snr` 5, FWHM <= 60 s, >= 5 scans; `labels.rs` "persistent": peak in `min_files` (3) treated samples or in two consecutive ones with apexes within `rt_tol_min` | the S/N has no one-ion floor (R1) |
| isotopes excluded | yes, coarse | `rows.rs::isotope_of`: a stronger ion at -1.00335, -2.0067 or -1.99580 Da, same RT, area >= 2x: criterion "not an isotope peak" fails, score <= 40 | no expected ratio from a formula (R3); the 0.5 bound flags a real +2 Da TP at 30 % of a strong neighbour and can miss an M+2 whose precursor cell is not `detected` (for example a saturated precursor that fails the peak check: its area counts 0) |
| +45 Da companions excluded | no | no +45 rule in `legacy/lr/` (grep `45`, `HCOO`); `[M+HCOO]-` exists only in the HR ion networks (`ionnet/ions.rs`); `coelute.rs` flags a candidate lighter by >= 3 Da than a co-eluting strong one | the vademecum's claim is not true of mzLab today; it is part of R4 (adduct list per polarity) |
| R1 one-ion noise floor | **done here** | `mzlab_core::ionstat` (new); before it: five S/N definitions (`RAW_ACCESS_STUDY.md` §2.2), none with a floor of one ion; `peakchk` uses 1.4826 MAD or the 90th percentile of deviations on a nearly zero trace | in the Dig as evidence (TP-1: `points[].sn_floor`, `checks[noise_floor]`); as a gate, and in `quant targets`, owner decision D1 |
| R2 absent at T0 and grows | yes | `labels.rs` "absent at time zero" (t0 / treated <= 0.20), "grows over time" (Spearman >= 0.8 up to the maximum, rise >= x2); `kin.rs` parent decay | no class "present at T0 and decays" with its reading (impurity or companion of P) and no correlation with the parent curve |
| R3 isotopes against the expected ratio (S, Cl) | partial | `ion pattern FORMULA ADDUCT` (`ion_pattern/1`) gives the expected unit-resolution pattern; `rows.rs` uses shifts, not ratios; RAW AU-18 `raw isotopes` (planned) | observed M+1/M, M+2/M against the pattern with Poisson error, per row |
| R4 adducts NH4+ (+17), Na+ (+22), co-elution r > 0.8 | no | `family.rs` at nominal mass keeps adducts `Undetermined` (`Rules::nominal()`); `features.group` HR only (`steps/group.rs`); `lrscreen_dedup.rs` merges only within 0.6 Da; AU-10 `features.companions` and AU-16 `raw companions` planned | pair check per row (mass difference, apex, Pearson over samples), never a merge |
| R5 irradiated matrix blank without P | no | roles of `samples/1`: blank, dark, ...; `BLANK_TYPES` solvent, procedure, wash | a matrix blank kind and its own criterion |
| final call without gap filling | yes | DIG-ROBUST: `forced` and `filled` never count as detected (`points.rs`, `aligned::EVIDENCE`); AU-3 for the feature tables | none in the Dig |
| visual XIC review per series and time | yes, site; partial, CLI | Dig `detail` with the XICs from the `.mzidx` (`detail.rs`); `trace xic`; AU-19 figure grid planned | no per-row grid from the CLI (AU-19) |
| re-inject an anomalous series (x10-100 between twins) | no | twin series are not compared | R6 below |
| parent linearity from standards (saturation lowers k) | no | MRM `analyse` has no calibration line (`mrm.rs`, `calibration: null`); `kin.rs` fits the parent decay without a linearity check | R7 below |
| declared scan range as perimeter | partial | `scan_window` of the run route (`itemroutes.rs`) | a hypothesis outside the window is `no_data`, not absent (R8) |
| full scan of all nominal traces, both polarities | partial | the pipeline detects over the whole axis (unexpected ions, `rows::free_rows`); `mapscan.rs` 1 Da cells for maps; AU-16 binned view planned | per-trace census in one call (AU-16) |
| confidence level <= 5 without MS2, 3 with MS2 and fragments, 1 with standard RT and MS2 | differs | `levels/1` profile `lr`: never above 4 (`MAX_LEVEL_UNIT` "4", `evid/graph.rs`: an MS2 does not raise a level at unit resolution, Tabui decision D6), 4 if the nominal formula alone explains the ion, else 5; Dig `level` null, `confidence` 4/5 without MS2 (`summary.rs`) | the vademecum's 3 with MS2 and 5 without it against D6: owner decision D3 below |
| NEG/POS pairs 2 Da apart at one RT | no | no cross-polarity pairing | R9 below |
| in-source fragments following the parent curve | yes | `lr/isf.rs::isf_eval` (`probs.isf`), `insource` of the unexpected ions (lighter than P within 0.5 `rt_tol_min`), `coelute.rs` | correlation with the parent curve is not printed as a number (R10) |

Commands an agent may think are missing and that exist: `ion pattern` (expected isotopes), `trace xic` (up to 500 m/z per file), `peak table --traces --auto`, `table.filter` blank rule, `features.group` (HR), `sample table` (roles from names).

## B. The rules as checks
### B.1 Contract shared by every check
- Each Dig row gains `checks[]`; one entry per rule: `{"id","status","value","threshold","evidence":{...},"rule"}`. `status` is `pass`, `warn`, `fail` or `n/a` (inputs missing, with the reason in `rule`). The words are those of `labels.rs::Crit` so the page needs no new vocabulary.
- A check never removes, hides or reorders a row. It enters the score only when the owner says so (D1); until then it is evidence beside the criteria.
- Every threshold is a parameter of `tp.dig_lr` (`--set dig.NAME=VALUE`), written into `settings` and `prov/1`.
- One log line per check and row with a non-pass status goes to `summary.log` (`"TP1 r=1042 check adduct_pair warn: ..."`) and to the CLI stderr as `note:`.
- `rule` holds the numbers in words, for example `"noise 2222 = one_ion (MAD 0.000, 0.500 SD 311, ion term 2222); S/N 2.0 < 5.0 -> fail"`.
- Parity: the checks are computed inside `lr::dig`, so the site (`finder-lr` `.wasm`) and the app/CLI (`flow_digl.rs`) give the same bytes; each chat adds a flow = page test on a synthetic series (`tests/lr_robust.rs` style).

### B.2 R1 noise floor of one ion (implemented)
- Inputs: the background points of a trace (cps) and an optional peak window; `one_ion` = cps of one ion (`auto`: the quantum, the smallest positive value or step between values, accepted when >= 90 % of positive values are within 0.02 of its multiples, there are >= 3 and their median is at most 100 quanta, so integer-valued analog data at 1000, 1001, 1003 is not a counting trace; a number: set, for example 2222 in EMS NEG and 1000 in Q1 POS on the example instrument; 0: off).
- Formula: noise = max(1.4826 MAD, `sd_weight` SD, `one_ion` sqrt(max(lambda, 1))), lambda = mean background counts per point. Near zero the ion term is one ion; above one count per point it is the Poisson error. Ties go to the first of mad, sd, ion term.
- Defaults: `sd_weight` 0.5, `min_sn` 5 (the Dig `min_snr`), `one_ion` auto.
- Evidence: `terms{mad, sd, sd_term, one_ion, one_ion_source, lambda, ion_term}`, `estimator`, `quantum{q, support, n_positive, level, accepted}`, `peak{height, sn, sn_mad, ions}`; `sn_mad` is the S/N a pure MAD gives (null when the MAD is 0, the explosion the vademecum describes).
- Verdict: `pass` S/N >= `min_sn` with an ion term; `fail` below; `warn` without an ion term (off, or no accepted quantum: analog or processed data) or with noise 0; `n/a` when no finite point is in the peak window, with the reason in `rule`.
- Today: `mzlab-cli peak noise TRACE.csv --rt A,B [--set one_ion=auto|CPS|0]` prints `noise-floor/1`; mzlab-wasm `noise_floor` (worker op `noise`, `QUALITA.noiseFloor`) gives the same bytes. TP-1 (done): the same function on the raw XIC of each checked cell in `peakchk`, as `points[].sn_floor`, `rows[].checks[noise_floor]` and `summary.noise_floor` (log), evidence only until D1; `tp.dig_lr` `one_ion` (absent = auto), `noise_sd_weight`. On the demo files the auto quantum (one ion = 13333 cps) is accepted in 95 % of the cells.

### B.3 R2 kinetic shape: absent at T0, grows
- Inputs: per condition the detected areas of the row over time (DIG-ROBUST `points`), the parent's curve (`kin.rs`), the standard of P.
- Formula: `t0_ratio` = max t0 area / max treated area; `rho_up` = Spearman up to the maximum; `rise` = max / min before it; `rho_parent` = Pearson of the row against the parent curve over the timed samples; `in_std` = row area in the parent standard / max treated area.
- Classes (in order): `grows` (t0_ratio <= 0.20, rho_up >= 0.8, rise >= 2) -> pass; `decays_from_t0` (t0_ratio > 0.5 and the last time <= 50 % of the maximum) -> fail, reading "impurity of P" when `in_std` >= 0.05, "companion of P" when `rho_parent` >= 0.8, else "decays"; `flat` (max/min < 2) -> warn; otherwise `irregular` -> warn.
- Defaults: 0.20, 0.8, 2, 0.5, 0.05, 0.8; all `--set dig.kin_*`.
- Evidence: the numbers above and `k_points`. Log: `"TP2 check kinetics fail: decays_from_t0, t0_ratio 1.00, last 0.31 of max, rho_parent 0.93 -> companion of P"`.
- Overlap: AU-17 `raw profile` shape classes use the same names; one function in `mzlab-feat` serves both.

### B.4 R3 isotope ratio against the formula
- Inputs: row m/z, its candidate formula (hypothesis) or none, the co-eluting rows at +1 and +2 Da, their areas per sample; for a precursor that failed the peak check its forced area (DIG-ROBUST `forced`) is used and the evidence says so.
- Formula: observed r1 = A(M+1)/A(M), r2 = A(M+2)/A(M), median over samples where both are detected, with the spread; counts n = height / one_ion (R1) give the Poisson error sigma(r) = r sqrt(1/n_M + 1/n_iso); expected e1, e2 from `ion pattern` of the formula (S: M+2 about 4.4 % per atom; Cl: about 32 %); z = (r - e) / sigma.
- Verdict, as the isotope partner: |z| <= 3 -> the +1/+2 row is the isotope of M (`fail` for that row as a TP, `isotope_of` set); |z| > 3 -> `warn` "not the expected isotope, possible TP at +n Da". As the monoisotopic row with a formula: |z| <= 3 -> `pass`, else `warn` (formula inconsistent). Without a formula: the carbon range from r1 (1.1 % per C) and "S or Cl suggested" when r2 > 0.04, as evidence, `warn`.
- Defaults: `iso_z` 3, `iso_min_ions` 10 (fewer ions: `n/a`).
- Overlap: AU-18 `raw isotopes` (counts from raw), AU-10 `features.companions` (table edges). Build R3 on the AU-18 function.

### B.5 R4 adduct pairs
- Inputs: rows of one polarity; the list per polarity (POS: +17 [M+NH4]+ vs [M+H]+, +22 [M+Na]+, +38 [M+K]+; NEG: +36 [M+Cl]- vs [M-H]-, +46 [M+HCOO]- vs [M-H]-, i.e. +45 from the neutral; an editable `liste/1`).
- Formula: a pair (a, b) with |mz_b - mz_a - d| <= `tol_da`, |apex_b - apex_a| <= 0.5 `rt_tol_min`, Pearson r of detected areas over >= 3 samples.
- Verdict: r > 0.8 -> `warn` on the heavier row "adduct of row a" (`adduct_of` set), `pass` on the lighter "has an adduct partner" (support for TP a); r <= 0.8 -> no edge. Never a merge; `coelute.rs` keeps its flag and names the pair when one exists.
- Defaults: `adduct_r` 0.8, `adduct_min_samples` 3, list as above.
- Overlap: AU-10 and AU-16 share the mass-difference table; R4 is the Dig use of it.

### B.6 R5 irradiated matrix blank
- Inputs: files of a new blank kind `matrix` (irradiated or treated matrix without P; `blank_type` = `matrix` in `samples/1`).
- Formula: `matrix_ratio` = max matrix area / max treated area (detected only).
- Verdict: <= 0.20 -> `pass`; > 0.20 -> `fail` "matrix photoproduct, not a product of P"; no matrix file -> `warn` "no irradiated matrix blank: matrix photoproducts cannot be told apart" (the vademecum calls the control mandatory; mzLab warns and never blocks).
- Defaults: `matrix_ratio` 0.20. The blank and the dark control get their own criteria at the same time (`absent in the blank`, `absent in the dark control`), so a dark-formed TP (hydrolysis) is no longer read as a blank signal.
- Overlap: SAMPLE-NAMES-ROLES (#3674) owns the roles and names; R5 adds one `blank_type` value and the name words (`matrix`, `mtx`).

### B.7 Later rules (spec only)
- R6 twin series: two series of one condition differing by x10-100 at matching times -> `warn` "re-inject before interpreting" with the ratio per time.
- R7 parent linearity: response factor of P over the standards; the highest standards below 80 % of the low-level slope -> `warn` "saturation: k from the decay is low" with the points.
- R8 scan range: a hypothesis outside `scan_window` -> `n/a` `no_data`, never "absent".
- R9 polarity pairs: a POS row and a NEG row 2 Da apart ([M+H]+ / [M-H]-) at one RT with r > 0.8 -> `pass` support.
- R10 in-source: print `rho_parent` with the `insource` verdict.

### B.8 Confidence levels
| level | needs (unit resolution) | today |
|---|---|---|
| 1 | standard of the TP: RT within tolerance and MS2 match | not emitted (the chemist decides, `TPDEC`) |
| 2-3 | MS2 with diagnostic fragments shared or shifted from P (`ms2.rs` `note`); the vademecum gives 3 at unit mass | not emitted: `MAX_LEVEL_UNIT` is 4, an MS2 does not raise a level at unit resolution (Tabui D6; D3) |
| 4 | nominal formula alone explains the ion, all of R1-R5 pass | `levels/1` gives 4; the vademecum gives 5 (D3) |
| 5 | a mass of interest that passes R1, R2 and R5 | `levels/1` 5 |
A row that fails R2 or R5 keeps its number and its level and shows the failed check; the level is never raised by a check alone.

## C. Implementation chats
| id | chat | size | depends on | overlaps with chats running now |
|---|---|---|---|---|
| TP-0 | R1 in `ionstat`, `peak noise`, wasm `noise_floor` (this PR) | S | none | the `ionstat` of AU-13 grows from this module |
| TP-1 | R1 in the Dig: `peakchk` XIC -> `points[].sn_floor`, `checks[noise_floor]`, evidence only (done); the gate waits for owner D1 | S | TP-0, CONTRACT-ENGLISH (#3677: Dig keys) | QUANT-TARGETS (#3681) can take `sn_floor` as a column (AU-3) |
| TP-2 | `checks[]` scaffold in the Dig rows and `summary.log`; R2 kinetic classes shared with AU-17 | M | TP-1 | AU-17 `raw profile` |
| TP-3 | R5 matrix blank kind; blank and dark as separate criteria | S-M | SAMPLE-NAMES-ROLES (#3674) | #3674 (roles) |
| TP-4 | R4 adduct pairs (mass-difference table shared with AU-10/AU-16) | M | TP-2, AU-10 table | AU-10, AU-16 |
| TP-5 | R3 isotope ratio on AU-18 | M | TP-2, AU-18 | AU-18 |
| TP-6 | R6-R10 | M | TP-2 | none |
Order: TP-1 after CONTRACT-ENGLISH merges; TP-2; TP-3 in parallel with TP-2; TP-4 and TP-5 when their raw/table functions land; TP-6 last.

## D. Owner decisions
- D1: the checks as evidence only (recommended first: the score and labels do not move, goldens stay) or as part of the score.
- D2: `one_ion` default `auto` (recommended; the quantum is shown with its support) or a per-instrument table in the settings.
- D3: the scheme of B.8 against the existing Tabui decision D6. Today (D6, `MAX_LEVEL_UNIT` 4) an MS2 does not raise a level at unit resolution and a nominal formula alone gives 4. The vademecum gives 3 with MS2 and diagnostic fragments and 5 without MS2. Keep D6 (no change, recommended until a unit-resolution MS2 proof exists), or adopt B.8 as a `levels/1` profile of its own.
- D4: R5 without a matrix blank: `warn` (recommended) or `fail`.
