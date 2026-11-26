Read docs/coda/BRIEF.md.

# FEATURE-UV: The UV-Vis slot of the feature view

**Goal.** The UV slot shows the PDA spectrum at the feature apex, the difference spectrum against the chosen blank or T0, lambda max, and the list of other features inside the UV peak window (possible co-elution, listed, never decided). `mzlab-cli table uv TABLE --id N --pda FILE --json` prints the same.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 8 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §D.

**Read first.** `docs/agenti/dad.md`, `crates/mzlab-core/src/pdapeaks.rs`, `crates/mzlab-cli/src/dad_cmd.rs`, `mzlab/web/uv-spettro.js`, `DAD-DELAY-APPLY` prompt.

**Already on main (checked 26 Nov, re-check before coding).** `dad spectrum`, `dad subtract`, `dad peaks`, `dad ms-at`, `dad delay` exist; the delay is estimated, not applied.

**Done means.** 1. A Rust function over the existing `dad` code; the delay used is printed and logged as `auto` or `user`. 2. The slot reuses `uv-spettro.js`. 3. wasm + native + CLI byte test, agent catalogs.

**Out of scope.** Peak purity algorithms.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
