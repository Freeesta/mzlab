# Value states of reported quantities (AU-3)
Files: `crates/mzlab-feat/src/value_state.rs` (the vocabulary), `crates/mzlab-ops/src/run/table/quant.rs` (`quant.calibrate`), `curve.rs`, `curve_read.rs`, `curve_write.rs` (`quant.curve`). Study: `docs/studies/AGENT_USE_STUDY.md` §2. Tests: `value_state.rs`, `curve_tests.rs`, `flow_quant_tests.rs`, `flow_curve_tests.rs` (goldens `crates/mzlab-cli/tests/data/calibration/`), `tests/test_calibrazione_golden.py`.

## Rule
A number an agent receives never stands alone: it comes with one word saying how it was obtained and, where the engine has it, the S/N behind it. An empty number cell is always explained by `absent` or `na`, never by silence, and a measured number is never printed as 0 or NaN.

## Vocabulary (`mzlab_feat::value_state::VALUE_STATES`)
- `detected`: a peak found and checked. `forced`: an area integrated where the checks found no peak (targeted pass, or a peak the Dig rejected). `filled`: the gap filling of the alignment.
- `below_loq`: a measured number under the lower end of the range (the lowest calibrator, the LLOQ); it is still shown, never hidden. It replaces `detected`/`forced`/`filled` only.
- `absent`: window covered, no peak (no number). `na`: the file does not cover the window or the row (not applicable, no number).
- The first four are the words of `aligned::EVIDENCE` (Dig points, `quant.targets`, unchanged); `cell_state` adds `na`, `under_loq` adds `below_loq`.

## Where it is reported (this slice: concentrations)
- `quant.curve` (`concentrations` CSV, last columns `state,sn`; `calibration/1` `points[]` keys `state`, `sn`): `na` = no row for the analyte and sample in the responses, `absent` = a row whose `peak` is not `ok`/`edge` (its S/N is kept as evidence), `below_loq` = note `below_range`, else `detected`. `sn` = the responses' own S/N column, empty when the file has none.
- `quant.calibrate` (CSV `compound,mz,rt,sample,signal,concentration,unit,flag,state,row_sn`): state from the cell of the aligned table (`detected`, `forced` for targeted cells, `filled`, `absent`, `na`), `below_loq` where `flag` is `below_range`. `row_sn` is the median S/N of the row's detected peaks (the table keeps no S/N per cell); empty when the table has no row summary. A cell the file does not cover is `flag` `not_applicable`, `state` `na`, with empty numbers (it printed `NaN` before).
- Existing columns keep their names, order and values; the new ones are appended (`note`/`flag` stay as they were).

## Not yet (follow-ups of AU-3)
- Aligned CSV (`export.table_csv`, `csv_full`): `snr` per cell and `na` apart from `absent` (it prints `UNKNOWN` for both, as MZmine does); per-cell S/N needs a column in the `.mzfeat` table store.
- Dig summary rows, `peak table`/`peak auto-lr` (`lr_auto_json` is the page's output), `chrom.transitions` `flag`: S/N exists, a `state` word does not.
- `table.stats` and `quant.curve` input filters naming the states they used in `prov/1`.
