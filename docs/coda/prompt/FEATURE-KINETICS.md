Read docs/coda/BRIEF.md.

# FEATURE-KINETICS: The kinetics slot of the feature view

**Goal.** The kinetics slot draws, per condition, area and height over time, each normalised to its maximum, the mean with SD bars and the replicate points on top; `absent` is a hollow point at 0, `na` a gap. `mzlab-cli table kinetics TABLE --id N --samples S.json --json` prints the same `kin-series/1`.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 6 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §D.

**Read first.** `TP_Mine/lazy/groups-view.js` (conditions chart), `mzlab/web/tdati.js`, `docs/agenti/value-states.md`, `docs/agenti/samples.md`, `crates/mzlab-feat/src/legacy/lr/{kin,cond}.rs`.

**Already on main (checked 26 Nov, re-check before coding).** Per-condition data and a conditions chart exist; no per-row series with both area and height.

**Done means.** 1. `kin-series/1` in Rust (values, mean, SD, n, state per point; the SD choice logged). 2. The slot reuses the existing chart code and the data table (key T). 3. wasm + native + CLI byte test, agent catalogs.

**Out of scope.** Fits of k (the step `quant.kinetics`).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
