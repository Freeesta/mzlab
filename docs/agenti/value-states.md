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

## Audit (slice 3): surface -> has the state word
| surface | state | note |
|---|---|---|
| `quant targets` / `quant.targets` (CSV, JSON) | yes | `state` column; evidence of every forced row in the log `decisions` |
| `quant.curve`, `quant.calibrate` | yes | `state`, `sn` / `row_sn` |
| `chrom.transitions` peaks, `peak auto-lr`, `peak table --auto` | yes | `state` beside `sn` |
| `peak table` CSV | opt-in | `--columns state` (the page's export keeps its columns) |
| Dig (`tp.dig_lr`) `rows[]`, `traces[]`, `points[]`, MRM tables | yes | `controls` are ratios of detected peaks only; the state of each control cell is in `points[]` |
| `raw xic` rows | yes (slice 3) | `state` after `verdict`: peak detected, trace forced, nothing absent, no_data na |
| `.mzfeat` | partly | `state` column with `CELL_NA`; no per-cell S/N |
| aligned CSV / xlsx sheet (`export.table_csv`, `csv_full`) | partly | `feature_state` in MZmine words (`DETECTED`, `ESTIMATED`/`TARGETED`/`FILLED`, `UNKNOWN` for absent and na); no S/N (below) |
| `export.mztab` | no | areas only (`NaN` where absent): mzTab 2.0 has no per-cell state; a follow-up is an optional-column |
| `features-report/1`, `run-report/2`, `screen.suspects` | n/a | decisions and identification levels, no quantity |

## Slice 3: detected is a choice, with its evidence
A peak at S/N 3 to 5 with a one-scan blip shape was `detected` in matrix controls and blanks (defaults `min_sn` 3, `min_points` 5, `min_shape` 0.5).
- `quant.targets` shape gates, all `0` = off, so the defaults give the same states and flags: `min_half` (scans in an unbroken run at or over half the height, at least), `max_fwhm` (s, at most), `max_asym` (at 10 % of the height, the larger of right/left and left/right, 1 = symmetric, at most; below 1 refused). A miss is `bad_shape`, the row is `forced` with its numbers, never hidden. A blip is 1 to 2 scans over half height, a peak 4 or more: `--set min_half=3` drops the blips that `--min-sn 5` alone keeps (test `sn_alone_keeps_taller_blips_and_a_half_height_gate_drops_them_all`).
- Evidence in the log: `decisions[]` has a line for every row that is not detected. A forced row lists `checks[]` (`name`, `value`, `limit`, `op`, `pass`): the three gates are always listed with their measured value and `limit` null while the gate is off, so the value that would separate the peaks can be read; `value` null = no number, which passes.
- An absent row has its reason in `flag` (`no_peak`, `no_scans`) and no checks. Detected rows are in the table; `files[].detected_min` gives the smallest S/N and the fewest half-height scans among the detected rows of each file, with the target ids, to show how close accepted peaks are to the thresholds. Also `files[].forced` and the three gates in `params`.
- Hint: the warning `warn.quant.targets.forced` (stderr, `warnings`, one text for every surface) names the gates. It is added to `warnings` of every run with a forced row, defaults included; table bytes are unchanged.
- Words across the two commands: `raw xic` says `na` for `no_data` (no scan to judge), `quant targets` says `absent` with flag `no_scans` for a window with no scan, because its rows are values of a table (changing it would change the table bytes). The same window can be `detected` in `raw xic` (ion-count criteria, the `rule` cell holds the test with its values) and `forced` in `quant targets` (S/N, shape and the gates): the answer to why is the `decisions` line of the log of that run.
- Findable: `commands --find` rows `quant targets` and `quant.targets` (words: forced, weak, blank, matrix, shape, gate), help paragraph, task table of `for-agents.md`; test `the_state_words_and_shape_gates_of_quant_targets_are_found_from_the_task_and_the_help`.
- Tests: `crates/mzlab-feat/tests/quant_targets_gates.rs` (synthetic matrix and blank: old and new behaviour), `quant_targets_cli.rs` (hint, log, exit 2 for a gate out of range), `raw_xic_cli.rs`; parity `tests/test_quant_targets_wasm.py` (wasm = CLI = `flow run`, gates off and on; in CI a missing tool FAILS) and `tests/test_raw_xic_wasm.py`.

## Not yet (follow-ups of AU-3)
- Gates elsewhere: the Dig has `min_snr`, `max_fwhm_s`, `min_points` (`peakchk`); `peak auto-lr` only `lr_sn` and `lr_min_pts`; `chrom.transitions` has no shape gate. `quant.targets` gates are not yet in `raw xic` verdicts (`--set` there: `min_sn`, `min_points`).
- `export.mztab`: a per-cell state as optional columns.
- Aligned CSV (`export.table_csv`, `csv_full`): `na` apart from `absent` and `snr` per cell. The `.mzfeat` store already keeps `CELL_NA` in its `state` column, so `na` needs no store column, but the CSV prints `UNKNOWN` for both as MZmine does: printing `NA` changes the value of an existing cell, so it needs an opt-in parameter on the step (wasm, CLI, schema) or the owner's go. Per-cell S/N needs a column in the `.mzfeat` table store.
- Dig `points[].state` still maps an `NA` cell to `absent` (`evidence_word`); it moves to `na` together with the CSV.
- `table.stats` and `quant.curve` input filters naming the states they used in `prov/1`.
