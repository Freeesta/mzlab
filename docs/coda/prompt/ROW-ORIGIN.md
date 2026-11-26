Read docs/coda/BRIEF.md.

# ROW-ORIGIN: The origin of each row of the union table

**Goal.** The union table says per row which engines found it (`direct`, `suspect`, or both) with the source table of each, so the user can filter by origin.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 7 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §C.

**Read first.** `features.union` (`union/1`), `docs/agenti/flow-graphics.md`, `crates/mzlab-feat/src/` (grep `union`), `UNION-DATAFILE` and `UNION-POLARITY-NODE` rows of the queue (do not overlap them).

**Already on main (checked 26 Nov, re-check before coding).** `features.union` joins tables and lists conflicts; no origin is written.

**Done means.** 1. An opt-in parameter of `features.union` (default off: existing bytes unchanged) that writes `origin` per row from the input names. 2. wasm + native + CLI byte test. 3. The column and a filter chip in the table view.

**Out of scope.** Engine B as an origin: B classifies, it does not find rows (study §5).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
