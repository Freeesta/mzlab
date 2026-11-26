Read docs/coda/BRIEF.md.

# MZTAB-CELL-STATE: per-cell value state in export.mztab

**Goal.** Say, for each abundance cell of `export.mztab`, whether the value is detected, forced, filled, below_loq, absent or not applicable.

**Why.** `export.mztab` writes areas only, so a reader of the exported file cannot tell a forced integration from a detected peak. Everywhere else the state word is available (AU-3).

**Read first.** `docs/agenti/value-states.md` ('Not yet'), `crates/mzlab-feat/src/value_state.rs`, `crates/mzlab-feat/src/aligned.rs` (`EVIDENCE`, `CELL_NA`), the mzTab export in `crates/mzlab-feat/src/export/`. Start with `grep -n 'value states\|mzTab' docs/agenti/INDICE.md`, then `grep -rn 'mztab' crates/mzlab-feat/src --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** The `.mzfeat` store keeps the state of every cell in its state column; aligned CSV and xlsx carry `feature_state` in MZmine words. The mzTab export writes the abundance columns only.

**Done means.** Only after the owner decides: whether the state goes into extra columns of the abundance table (optional `opt_` columns, which the mzTab format allows) behind an opt-in step parameter with the current bytes as default, or not at all. Then: the parameter, a test that the default bytes are unchanged, the schema and docs line, parity site=app=CLI.

**Out of scope.** Aligned CSV `NA` words and per-cell S/N (row NA-CELL-STATE).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
