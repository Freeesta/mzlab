# `--sample LIST`: open only some samples of a multi-sample file (CLI, PR 3 of the chooser plan)

Plan and decisions: `wiff-sample-choice.md`. List what a file holds first: `mzlab-cli sample list FILE --json` (`samples_list/1`).

## Semantics
- `LIST` is sample ids and ranges: `1,3,5`, `2-4`, `5, 2-4,3`. The ids are those of `sample list` (not positions), each taken once, the answer in the order of the file. One code for all routes: `mzlab_core::readers::selection` (`parse`, `resolve`, `selection_json`), also exported by the engine .wasm as `sample_selection(present_ids, text)` for the site (same bytes; `tests/test_sample_selection_parita.py`).
- Only the container and the index are read to check the list. The samples not chosen are never read.
- An id the file does not have stops the command before anything is written: `err.sample.unknown` names the missing ids and the ids present, exit 5 (distinct from usage, 2). A text that is not a list (`1,,2`, `3-1`, `x`): `err.sample.list`, exit 2.
- A dataset with one sample ignores the list (it is converted as it is); when no input has several samples the command stops with `err.sample.single` (exit 2), so a typo is never silent.

## Commands
- `sample select FILE --sample LIST [--json]`: the dry run; `{"samples":[1,3]}` (or ids on one line without `--json`); `error/1` on stdout with `--json` on failure; exit 0, 2, 4 (missing file), 5.
- `convert INPUT... --out DIR --sample LIST`: one `NAME-ID.mzML` per chosen sample, byte for byte the file a run without `--sample` writes for that sample (a single chosen sample keeps its `NAME-ID` name). Exit 5 before anything is written.
- `channels FILE --sample LIST`: one `channels/1` line per sample, in the order of the file, each the bytes of its own single-sample run; the whole list is checked first (no partial answer); `--channel` needs one id.
- `features FILE... --sample LIST`: every file with several samples gives one series member per chosen sample (progress lines name it `FILE@ID`), single-sample files stay one member; without `--sample` a multi-sample file is its first sample, as before.
- `sample select FILE --default [--budget-mb N] [--json]`: the samples the page dialog ticks first (`readers::selection::suggest`, the site's `sample_default`): all when their section bytes are at most a quarter of the budget (default 1024 MB), none otherwise; exactly one of `--sample` and `--default`.

## Not here
- A `samples` property of a `flow run` input (plan inputs already name a sample as `NAME@ID`) and the partial identity of Q4: the mzML of a chosen sample is the same bytes as in a full run, so no identity changes here. The page dialog is PR 4 (`wiff-sample-dialog.md`), the OPFS range copy PR 5.
- `dad --sample ID` keeps its single id.

Tests: `crates/mzlab-cli/src/sample_select_tests.rs` (synthetic `tools/wiff_grande.py` pair; python3 is part of the test), `readers/selection.rs` unit tests.
