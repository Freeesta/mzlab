# LLM: parameter proposals (`param-proposal/1`, rules `R-*`)
Code: `crates/mzlab-llm/src/params_rules.rs` (contract, `TARGETS`, `fit`), `params_rules_det.rs` (acquisition rules) and `params_rules_tab.rs` (table rules). Schema `docs/schemi/param-proposal-1.json`. Context: `llm.md`, `llm-subagents.md`.

## Principle
- The proposals are numbers computed by deterministic Rust rules (`mzlab_llm::params_rules::propose(&inspection, &answers, &flow::steps())`) from an `inspection/1` and the user's answers. The model invents no value: it chooses among the proposals (`param-choices/1`), asks when a signal is missing and explains; a value it changes carries `source: model` with the reason, a kept one `source: rule:<id>`.
- Every proposal is applied and removed with one click (card "Apply"/"Discard", an entry of `S.undo`); the form field keeps the "estimated" mark until the user touches it.
- Never verdicts in the LR world: at unit resolution the proposals concern detection and alignment, not the identity of products; the profile `course` has no assistant.
- Confidence: `high` = measured from the data with enough traces (`n` ≥ 30, or ≥ 3 QC files for the drifts); `medium` = from the method metadata or few traces; `low` = typical value of the analyzer type, to be confirmed (the panel shows it as a question, not as a proposal).
- The rules read the first readable file of the series that is not a blank (`choices` records it as `reference_file`, with `resolution_class` and whether answers were given). Typical values follow the documented settings of XCMS centWave, IPO/AutoTuner and mzmine.

## Contract `param-proposal/1`
```json
{"schema":"param-proposal/1","series":"s1","choices":[{"key":"reference_file","value":"f1","reason":"…"}],
 "proposals":[{"step":"features.detect@1","algorithm":"centwave","name":"peak_width_s","value":[4,20],
  "rule":"R-width-1","signals":["fwhm_median_s=8.5","traces=184"],"confidence":"medium","range_ok":true,
  "proof":"inspection:f1:peak_width"}],
 "missing":[{"signal":"blank_files","asks":"Q-blank"}],"warnings":["warn.params.mixed_polarity"]}
```
- One proposal per parameter named by the rule in `TARGETS` (rule id → `(step, algorithm, name)` list; the test checks that each target exists in the `step/1` registry). `algorithm` is present only for algorithm parameters.
- `value` is always inside the limits of the `step/1` descriptor: `fit` cuts to the limit and sets `range_ok: false`; a choice value outside the list also gives `range_ok: false`.
- `proof` = `inspection:<alias>:<kind>` (kinds: `polarity`, `class`, `mass_error`, `model`, `typical`, `noise`, `peak_width`, `cycle`, `qc`, `gradient`, `dead_time`, `roles`, `answer`, `isolation`, `ppm`).
- `missing[].asks` is a question id `Q-*` (`Q-files`, `Q-pol`, `Q-resolution`, `Q-width`, `Q-cycle`, `Q-gradient`, `Q-blank`, `Q-qc`, `Q-groups`, `Q-identify`, `Q-centroid`); a signal that is absent is a question, never an invented value. `warnings` are `warn.params.*` (`mixed_polarity`, `no_ms2`).
- `answers` = `{objective?, identify?, weak_signals?, elements?, groups?}`. The same input gives the same bytes (golden tests).

## Signals (`inspection/1`)
Measured by Rust, never by the model: polarity per scan (mixed = alternating, separate series), resolution class (`unit` if FWHM in m/z ≥ 0.3 Da, `high` if R ≥ 10 000) with its `class_basis`, data mode (profile/centroid), acquisition types (full scan, SIM, MRM, product ion, DDA, DIA, alternating polarity; never from the file name), infusion (flat TIC, no chromatographic peaks), MS1 cycle time, isolation windows, gradient (`method/1`), dead time, noise, mass error, FWHM of the strongest traces, RT and intensity drift of the traces common to the QC files (`qc/1`, ≥ 3 files).
Also: roles (`samples/1`, proposed from names but never a proof), injection order (file creation time, local time without zone).

## Rules (`TARGETS`)
| Id | Step.parameter | Rule | Confidence |
|---|---|---|---|
| R-pol-1 | `features.estimate/detect/table.polarity` | the polarity of the series when single; mixed or alternating gives the warning and `Q-pol` | high |
| R-axis-1 | `features.estimate/detect.axis` | `Da` at class `unit`, `ppm` at `high` (`Q-resolution` if unknown) | high with `class_basis`, else medium |
| R-ppm-1 | `ppm` of `features.detect` (centWave), `features.align`, `features.gapfill` | HR with measured error: 3 robust sigma rounded up to 0.5 ppm, 2 to 30 | high if n ≥ 30, else medium |
| R-ppm-2 | same | HR without a measure: Orbitrap family 5, TOF 10 | medium; other HR model 10 low |
| R-da-1 | `tp.dig_lr.tol_da`, `spectra.ms2.tol_da` | unit class: abs(bias) + 3 sigma rounded up to 0.05 Da, 0.1 to 1; without a measure 0.35 | high if n ≥ 30, else medium; typical low |
| R-floor-1 | `features.detect.floor`, `features.gapfill.floor` | the noise measured in the file | medium |
| R-height-1 | `features.detect.min_height`, `table.filter.min_height` | 10 × noise | medium |
| R-snr-1 | `features.detect` centWave `snr` | 10; 5 if the answer `weak_signals` is true | medium with answer, else low |
| R-width-1 | centWave `peak_width_s` | [0.5, 2] × median FWHM in seconds (`Q-width` if absent) | medium |
| R-pts-1 | `min_points` of `features.detect`, `features.gapfill` | max(3, floor(points / 2)), points = FWHM / MS1 cycle (`Q-cycle` if absent) | medium |
| R-gap-1 | `max_gap` of `features.detect`, `features.gapfill` | 2 for DDA with more than 5 MS2 per cycle, else 1 | medium |
| R-rt-1 | `features.align.rt_gap`, `features.gapfill.rt_tol`, `features.table.rt_tol` | the `qc/1` proposal (3 robust deviations of the QC drift, at least the peak width) | high if `qc/1` is high and ≥ 3 QC, else medium |
| R-rt-2 | same | without QC: 0.5 % of the gradient length, at least 0.05 min (`Q-gradient` if no length) | medium |
| R-warp-1 | `features.align.rt_warp` | `piecewise` or `off` as the `qc/1` proposal | as R-rt-1 |
| R-win-1 | `tp.dig_lr.rt_min` | unit class only: the dead time | medium |
| R-blank-1 | `table.filter.blank_ratio` | 3 if the series has blanks; else `Q-blank` | medium |
| R-qc-1 | `table.filter.qc_cv_max`, `qc_min_files` | 30 % and ceil(QC / 2) (at least 2) with ≥ 3 QC; with fewer, `Q-qc` only for a `nontarget` or `quant` objective | medium |
| R-det-1 | `table.filter.min_detected` | max(2, floor(smallest group / 2)); groups from the answers, else the number of samples (`Q-groups` if none) | medium |
| R-ms2-1 | `table.filter.require_ms2` | true only if `identify` and the files are DDA; `Q-identify` if not answered; `warn.params.no_ms2` if identify without DDA | medium |
| R-drift-1 | `table.drift.method`, `span` | only with ≥ 5 QC: `linear`, `loess` with span 0.75 from 8 QC | medium |
| R-iso-1 | `features.ms2_link.default_half_window` | half of the widest isolation window read | high |
| R-cent-1 | `spectra.centroid.threshold` | profile files: the noise; the FWHM in m/z is asked (`Q-centroid`) | medium |
| R-form-1 | `formula.ms1.tol_ppm`, `formula.ms2.ppm_ion` | HR: the ppm proposal | high if the ppm is high, else medium |
| R-form-2 | `formula.ms1.elements` | the answer `elements`, else `CHNO` | high with answer, else medium |

A new rule is one entry in `TARGETS`, one function in the `_det`/`_tab` file, one golden test and one row here.

## Questions the assistant asks when a signal is missing
Matrix (water, soil, tissue, standard solution), which files are blanks and QC (if `samples/1` did not recognize them), expected compounds (parent as formula or m/z; internal standards for the RT drift), objective (non-targeted screening, transformation products, quantification, quality check only), groups or conditions (for `table.stats` and `min_detected`). Closed questions with options (`ask_user`), at most 5 at a time, never one that Rust can answer.

## Acquisition type to starting plan
| Acquisition | Starting plan |
|---|---|
| full scan LR | `features.estimate` → `features.detect` (axis `lr`, Da) → `features.align` → `features.gapfill` → `table.filter` → `export.table_csv`; no HR step |
| full scan HR | the same with ppm, plus `table.drift` with ≥ 5 QC |
| DDA HR | as full scan HR + `features.ms2_link` → `spectra.chimeric` → `formula.ms1` → `formula.ms2` → `spectra.match` → `annot.level`; identity only as candidates with a level |
| DDA LR | full scan LR + `spectra.ms2` |
| DIA | as full scan HR; MS2 is not linked by precursor (stated as a limit, no invented step) |
| MRM / sMRM | `quant.calibrate` only in the profile `research`; in the teaching world the students make the lines |
| product ion, MSn, infusion | `spectra.msn_graph` → `msn.library_export` / `msn.table_export` |
| transformation products | `tp.generate` → `tp.filter` → `tp.link` (HR) or `tp.dig_lr` (LR); needs the parent named by the user |
| alternating polarity | two series, one plan per polarity |

## Proofs
- `cargo test -p mzlab-llm --test inspection` (signals), `cargo test -p mzlab-cli --lib llm_params` (real registry; series G01 at unit resolution and G08 DDA at high resolution, cases G15-G22 of `llm-eval.md`: value, confidence, `range_ok`, stable bytes), `tests/test_llm_param_proposal.py` (schema).
- Comparison with a chemist (local, private data only): for 16 cases a chemist chooses the parameters without seeing the proposals; metric = share of proposals inside the interval the chemist accepts (`llm-eval.md`).
