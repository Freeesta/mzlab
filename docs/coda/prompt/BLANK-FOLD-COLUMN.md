Read docs/coda/BRIEF.md.

# BLANK-FOLD-COLUMN: blank_fold as an opt-in column of the table export

**Goal.** An opt-in column `blank_fold` in the table export, per row.

**Why.** Today the fold is only in the evidence list of dropped rows (follow-up of #3711).

**Read first.** `crates/mzlab-feat/src/export/`, `crates/mzlab-feat/src/view.rs::blank_evidence`, `docs/agenti/export-data.md`. Start with `grep -n 'export' docs/agenti/INDICE.md`, then `grep -n 'fn blank_evidence' crates/mzlab-feat/src/view.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `counts.blank` has fold per dropped row only. The default export bytes must not change.

**Done means.** A step parameter off by default; the column when on; goldens unchanged; test for both. Parity site=app=CLI.

**Out of scope.** Changing the default export.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
