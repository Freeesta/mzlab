Read docs/coda/BRIEF.md.

# BLANK-CONTAMINANTS: Flag contaminant ions that also appear in blanks

**Goal.** Automatically flag contaminant ions that also appear in blanks, before an enhanced product ion scan is spent on them.

**Why.** An acquisition slot spent on a blank ion is wasted instrument time.

**Read first.** `docs/agenti/mzfeat-table.md` (`counts.blank`, `blank_fold`, AU-7), `crates/mzlab-feat/src/view.rs::blank_evidence`, `docs/agenti/tic-background.md` (persistent background ions, `trace background`), `crates/mzlab-cli/src/dda_window_cmd.rs` (`spectrum dda-window`). Start with `grep -n 'blank\|background' docs/agenti/INDICE.md`, then `grep -n 'fn blank_evidence' crates/mzlab-feat/src/view.rs`.

**Already on main (checked 25 Nov, re-check before coding).** AU-7 blank subtraction and `counts.blank` (plain and with options) are merged; `trace background` finds persistent background ions. Reuse them. Missing: the flag in a list meant for an acquisition or inclusion list.

**Done means.** 1. A flag column or output key on the inclusion or target list: `contaminant` with the blank fold, the blank value and the rule; the threshold is a `--set`. 2. Synthetic sample and blank with known contaminants. 3. Nothing is removed silently. Parity site=app=CLI.

**Out of scope.** Changing the blank filter output (rows BLANK-SUMMARY, BLANK-FOLD-COLUMN).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
