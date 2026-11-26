Read docs/coda/BRIEF.md.

# TABLE-ROW-CLI: One row of a feature table from the CLI

**Goal.** `mzlab-cli table row TABLE.mzfeat --id N [--traces --index DIR] --json` prints `row/2` (and with `--traces` `row_traces/2`) of one row, the same bytes as the site and the app.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 1 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §D.

**Read first.** `crates/mzlab-feat/src/row.rs`, wasm `table_row` and `table_row_traces` in `crates/mzlab-feat-table-wasm/src/lib.rs`, desktop `row` / `row_traces` in `crates/mzlab-desktop/src/feat.rs`; `grep -n 'table compare' docs/agenti/cli.md` for the sibling route.

**Already on main (checked 26 Nov, re-check before coding).** The model and both non-CLI routes exist; only the CLI route is missing (parity gap found by the study).

**Done means.** 1. The route, with `err.rowNotFound` as its own exit code pinned in a test. 2. A three-way byte test (wasm, desktop function, CLI) on a synthetic series (`tools/dati_sintetici.py`, `MzmlWriter`). 3. Listed in `docs/agenti/cli-agenti.md`, `cli-catalog.md`, `related.rs` task words ("one feature", "row of the table") with a `commands --find` test.

**Out of scope.** Any new field in `row/2`; the feature view (FEATURE-DETAIL).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
