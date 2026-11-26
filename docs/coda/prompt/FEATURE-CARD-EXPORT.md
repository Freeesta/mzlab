Read docs/coda/BRIEF.md.

# FEATURE-CARD-EXPORT: One card per feature, HTML and SVG

**Goal.** `mzlab-cli table card TABLE --id N --out card.html` and a Card button write one HTML page per feature (structure, notes, MS2 per polarity, UV, XIC with blank and dark on the same scale, kinetics, SIM) with SVG charts and a provenance footer listing the exact source files with SHA-256, the plan and the step versions, and the mzLab citation.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 11 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §D.

**Read first.** `crates/mzlab-feat/src/msnfigure/` (SVG writer), `mzreport/1` (`docs/agenti/data-tools.md`), `crates/mzlab-cli/src/citation.rs`, `docs/agenti/export-data.md`.

**Already on main (checked 26 Nov, re-check before coding).** Study report and MSn figures exist; no single-feature card.

**Done means.** 1. The page rendered in Rust from `feature-slots/1` and the slot outputs; empty slots say so. 2. Same bytes on site, app and CLI (times aside, as `prov/1`). 3. Agent catalogs.

**Out of scope.** PDF and PNG (owner: later).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
