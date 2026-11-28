Read docs/coda/BRIEF.md.

# POLPAIR-CLI: A CLI route for the polarity pair of two HR tables

**Goal.** `mzlab-cli table polpair POS.mzfeat NEG.mzfeat --json` prints the `polpair/1` that the wasm `cross_polarity` gives today.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 13 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md`.

**Read first.** `crates/mzlab-feat-table-wasm/src/lib.rs` (`cross_polarity`), `docs/agenti/polarity-pair.md` (LR pairing, do not overlap).

**Already on main (checked 26 Nov, re-check before coding).** The site function exists; no CLI and app route found by the study.

**Done means.** 1. CLI and app routes, `error.polpair.files` as its own exit code. 2. Three-way byte test. 3. Agent catalogs.

**Out of scope.** The LR pairing (`table pair`).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
