Read docs/coda/BRIEF.md.

# NA-CELL-STATE: na apart from absent in the aligned CSV and the Dig points

**Goal.** Print `NA` (not applicable) apart from `absent` in the aligned CSV and make Dig `points[].state` say `na` for an NA cell, behind an opt-in step parameter.

**Why.** The CSV prints `UNKNOWN` for both, as MZmine does; printing `NA` changes an existing cell value, so it needs the owner's go or an opt-in parameter on the step (wasm, CLI, schema). Also open: per-cell S/N (a new `.mzfeat` store column).

**Read first.** `docs/agenti/value-states.md` ('Not yet'), `crates/mzlab-feat/src/aligned.rs` (`EVIDENCE`, `CELL_NA`), `crates/mzlab-feat/src/value_state.rs`, `export.table_csv` in `crates/mzlab-feat/src/export/`. Start with `grep -n 'value states' docs/agenti/INDICE.md`, then `grep -rn 'CELL_NA' crates/mzlab-feat/src --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** The `.mzfeat` store keeps `CELL_NA` in its state column; no store column is needed for `na`.

**Done means.** Only after the owner decides: the opt-in parameter name and default; the CSV and Dig change; per-cell S/N yes or no. Parity site=app=CLI, golden unchanged with the default.

**Out of scope.** Everything else of AU-3.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
