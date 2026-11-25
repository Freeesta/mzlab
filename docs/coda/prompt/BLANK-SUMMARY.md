Read docs/coda/BRIEF.md.

# BLANK-SUMMARY: A CLI summary line for the blank filter

**Goal.** `flow run` prints a summary line of the blank filter: rows kept, rows dropped, threshold.

**Why.** Today the evidence is only inside `view/1` (follow-up of #3711).

**Read first.** `docs/agenti/mzfeat-table.md` (`counts.blank`), `crates/mzlab-feat/src/view.rs`, `crates/mzlab-cli/src/flow_out.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `counts.blank` is written for the plain and the optioned blank filter.

**Done means.** One line on stderr (progress) and the same three numbers as stable keys in the `--json` report; exit codes unchanged; test.

**Out of scope.** Per-row columns (row BLANK-FOLD-COLUMN).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
