Read docs/coda/BRIEF.md.

# PHONE-VIEWER: A read-only phone view

**Goal.** On a phone, beside "Open anyway", a button shows the list of feature tables and one feature, one slot at a time, read only.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 17 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §E.

**Read first.** `mzlab/web/telefono.js`, `telefono-scheda.js`, `docs/agenti/data.md`, `docs/agenti/mzfinder-table.md`.

**Already on main (checked 26 Nov, re-check before coding).** Phones get one card today.

**Done means.** 1. Only viewer modules load; no runs. 2. e2e test at 390 px. 3. No new logic (the data come from the routes of FLOW-TABLE-LIST and FEATURE-DETAIL), stated in the PR body.

**Out of scope.** Editing on the phone.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
