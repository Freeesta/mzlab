# Value states of reported quantities (AU-3)
Files: `crates/mzlab-feat/src/value_state.rs` (the vocabulary), `crates/mzlab-core/src/value_word.rs` (the words of the core outputs), `crates/mzlab-ops/src/run/table/quant.rs` (`quant.calibrate`), `curve.rs`, `curve_read.rs`, `curve_write.rs` (`quant.curve`). Study: `docs/studies/AGENT_USE_STUDY.md` §2. Tests: `value_state.rs`, `curve_tests.rs`, `flow_quant_tests.rs`, `flow_curve_tests.rs` (goldens `crates/mzlab-cli/tests/data/calibration/`), `tests/test_calibrazione_golden.py`.

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

## Slice 2: the Dig, `peak auto-lr`, `peak table`, `chrom.transitions`
Only keys and one trailing column are added; every older key, column and value keeps its bytes. One word, `state`, sits beside the existing S/N.
- `chrom.transitions` (`peaks` CSV): trailing column `state` after `below`, from the `peak` flag (`flag_state`): `ok`/`edge` `detected`, `no_peak` `absent` (window covered, nothing above the S/N floor), `no_trace` and `no_rt` `na` (nothing was measured). `peak` stays as it was; `quant.curve` reads its responses by name and ignores the new column.
- `peak auto-lr` (`lr_auto_json`, the page's `lr_auto_integrate` and the CLI, the same bytes): every peak has `state` after `sn` (always `detected`: a listed peak passed the S/N floor); the whole trace has `state` before `peaks`: `detected` with peaks, `absent` searched with none, `na` under 7 points (`value_word::trace_state`).
- `peak table --auto` (`manint_auto` windows): `state` after `sn` (`detected`; also for `unmatched`, which only says no other sample has the peak).
- `peak table` CSV: new optional column group `--columns state` (`STATE_HEAD`, last; the default bytes are unchanged), from `origin` (`origin_state`): `manual`/`applied`/`shifted`/`too_far` `forced` (an area integrated where you put the window), `no_peak` `absent`, `auto`/`unmatched` `detected`. It is opt-in because the table is the page's export and its columns are fixed.
- Dig detail (`api.detail`, `lr/detail.rs`): `state` beside `snr` in `rows[]` (one per file) and in `traces[].peak`, the same word as `points[].state` of the summary row (`points::word`: the table cell's evidence word, else `detected`/`absent` from the cell). The summary rows' `points[]` already had `state` and `snr`. MRM tables (`lr/mrm.rs`): `state` beside `snr` of each ion (`detected` or `absent`).
- Tests: `value_word.rs`, `value_state.rs`, `peakq_lrauto_test.rs`, `manint_auto_tests.rs`, `manint_fig_tests.rs`, `transitions/tests.rs`, `mzlab-finder-lr-wasm/tests/lr_dig.rs`. The page and the app share the Rust functions, so the bytes are the same by construction; the existing parity tests (`peak_area_tests.rs`, `peaks_auto_lr_cmd_tests.rs`) compare them.

## Not yet (follow-ups of AU-3)
- Aligned CSV (`export.table_csv`, `csv_full`): `na` apart from `absent` and `snr` per cell. The `.mzfeat` store already keeps `CELL_NA` in its `state` column, so `na` needs no store column, but the CSV prints `UNKNOWN` for both as MZmine does: printing `NA` changes the value of an existing cell, so it needs an opt-in parameter on the step (wasm, CLI, schema) or the owner's go. Per-cell S/N needs a column in the `.mzfeat` table store.
- Dig `points[].state` still maps an `NA` cell to `absent` (`evidence_word`); it moves to `na` together with the CSV.
- `table.stats` and `quant.curve` input filters naming the states they used in `prov/1`.
