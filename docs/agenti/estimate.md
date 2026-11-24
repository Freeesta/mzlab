# `estimate`: detection parameters measured on the data (AU-6, slice 1)

`mzlab-cli estimate FILE... [--axis da|ppm] [--polarity +|-] [--sample ID] [--emit-set PLAN.json] [--json] [--out PATH]`
(`estimate_cmd.rs`; engine `mzlab_core::estimates::route`, schema `estimate/1`, `docs/schemi/estimate-1.json`).

## What it does
`estimate/1` (this command, singular: many files put together) is not `estimates/1` (plural: one file, the output of the Flow step `features.estimate`, which the files of `estimate/1` carry).
- Each file is measured by the per-file engine of `features.estimate@1` (`run_estimate::estimate_of`, the same function `flow run` and the site call): `estimates/1`.
- The aggregate gives, per value (`noise_floor`, `min_height`, `ppm`, `da`, `fwhm`, `rt_tol`, `dead`, `defect`), the median over the files with min, max, points and the per-file figures.
- The plan parameters those values would set (`params`): `INSTANCE.NAME=VALUE` for `--set`, the value now in the plan (`current`), the measure (`from`), the rule and the evidence. Nothing is written to a plan.
  Without a plan the instance is the last part of the step id (`detect`, `align`, `gapfill`); with `--emit-set PLAN.json` it is the id of each instance of the step in that plan, and the output is the `--set` lines with the evidence as a comment.
- Mapping (`estimates.rs`, `TARGETS`): `noise_floor` -> `features.detect.floor`, `features.gapfill.floor`; `min_height` -> `features.detect.min_height`; `defect` -> `features.detect.mass_defect`; `ppm` -> `features.align.ppm`, `features.gapfill.ppm` (ppm axis); `da` -> `features.align.tol_da`, `features.gapfill.tol_da` (Da axis); `rt_tol` -> `features.gapfill.rt_tol`. Proposed values are the median to 4 significant figures.
- The `rule` of each proposal names the group that fed it (polarity, axis, number of files). `points` is evidence (peaks behind a ppm, Da or fwhm value), never the weight of the median: every file counts once.
- `skipped` says why a parameter is not proposed: `axis` (it is for the other axis), `noValue` (not measured), `noInstance` (the plan has no instance of the step).

## Choices that are logged (`choices`, `left_out`, `notes`)
- Files that do not agree on polarity and axis: the group most files share gives the values; the others are in `left_out`; note `warn.estimate.mixed`.
- Fewer than 3 files: the values are given with `note.estimate.fewFiles`, never refused.
- An estimate is a measurement of THESE files, not a setting; the user decides.

## Exit codes
0 done, 1 a file gave no estimate or an error (the others are answered), or no value has a median, 2 usage, 3 the plan is not a plan (`err.flow.shape`, `err.flow.json`), 4 a file or the plan is missing, 5 unknown sample, 6 output not written.

## Parity
What ships: the aggregator in the `rotte` `.wasm` (`estimate_set`) and the app op `lr_estimate`; no page code calls them, because the site has no command line (its Flow runs `features.estimate` per file in `hr`). The `.wasm` leg of `parity_estimate.rs` only runs with `MZLAB_WASM_PKG`, which `tests/test_parita_rotte_app.py` sets after building the module; a plain `cargo test` skips it.
`lr_estimate` (app, `rpc_lr.rs`) and `estimate_set` (`rotte` `.wasm`) take `{"files": [estimates/1 or {file, error}], "plan": mzflow/2 or null}` and give the same bytes as the CLI core (the CLI adds `citation`): `parity_estimate.rs`. The per-file `estimates/1` is the Flow step `features.estimate` in the site (`hr`). Tests: `estimate_cmd_tests.rs`.

## Not in this slice (follow-ups)
Blank level per m/z band (needs the sample roles of AU-2), RT drift with an `rt_warp=piecewise` proposal, merging the rules of `llm params` (`llm-params.md`), mapping `fwhm` and `dead` to step parameters.
