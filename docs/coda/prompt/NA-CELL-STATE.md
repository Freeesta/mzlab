Read docs/coda/BRIEF.md.

# NA-CELL-STATE: na apart from absent in the aligned CSV and the Dig points

**Goal.** Print `NA` (not applicable) apart from `absent` in the aligned CSV and make Dig `points[].state` say `na` for an NA cell, behind an opt-in step parameter.

**Why.** The CSV prints `UNKNOWN` for both, as MZmine does; printing `NA` changes an existing cell value.

**Owner decision (26 Nov: "fino alla 3 ok").** Opt-in. A new step parameter prints `NA` for `na` cells; without it the bytes of every existing output (CSV, xlsx, Dig, `.mzfeat`) stay identical, pinned by a test. The default is off and is printed with the other defaults (wasm, CLI, schema). Per-cell S/N (a new `.mzfeat` store column) is not decided: leave it out and name it in the PR body as a follow-up.

**Read first.** `docs/agenti/value-states.md` ('Not yet'), `crates/mzlab-feat/src/aligned.rs` (`EVIDENCE`, `CELL_NA`), `crates/mzlab-feat/src/value_state.rs`, `export.table_csv` in `crates/mzlab-feat/src/export/`. Start with `grep -n 'value states' docs/agenti/INDICE.md`, then `grep -rn 'CELL_NA' crates/mzlab-feat/src --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** The `.mzfeat` store keeps `CELL_NA` in its state column; no store column is needed for `na`.

**Done means.** The opt-in parameter (name in the step's schema style, default off); with it the aligned CSV prints `NA` for an `na` cell and Dig `points[].state` says `na`; without it the CSV and Dig bytes are unchanged (golden unchanged). Parity site=app=CLI byte for byte, with the parameter on and off; a docs line in `docs/agenti/value-states.md`; the entry point for an agent (see the brief).

**Out of scope.** Everything else of AU-3.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
