Read docs/coda/BRIEF.md.

# ECOSAR-IO: ECOSAR by file exchange

**Goal.** Export the accepted structures as an ECOSAR input list; import its predictions as a column of the table, marked as predictions, with provenance.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 20 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md`.

**Read first.** `docs/agenti/export-data.md`, `annot.hazard` descriptor (no executor).

**Already on main (checked 26 Nov, re-check before coding).** No ECOSAR route.

**Done means.** 1. Export and import in Rust with a synthetic result file. 2. Site, app, CLI the same bytes. 3. Agent docs.

**Out of scope.** Running ECOSAR.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
