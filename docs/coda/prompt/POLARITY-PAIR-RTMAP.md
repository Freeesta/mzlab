Read docs/coda/BRIEF.md.

# POLARITY-PAIR-RTMAP: An RT map between the two polarity methods

**Goal.** `features.pair` and `table pair` compare the two RT axes as they are. Add an opt-in `rt_warp` (off by default) that maps the negative RT axis onto the positive one before pairing, from anchor pairs, and logs the anchors and the choice.

**Why.** Two methods with different gradients or dead times shift every RT; a fixed `rt_tol` then either misses true pairs or lets decoys in.

**Read first.** `docs/agenti/polarity-pair.md`, `docs/agenti/quant-targets.md` (`rt_warp`, `rt_gap` of `union/1`), `crates/mzlab-core/src/series_union/` (the map), `crates/mzlab-feat/src/polarity_pair/pairs.rs`.

**Already on main (re-check before coding).** `quant union` maps the RT of the series onto the first one (`rt_warp=piecewise`). `pair/1` has no map: the doc says the axes are compared as they are.

**Done means.** 1. `rt_warp` and `rt_gap` as parameters of the engine, the step and `--set`, reusing the map of `series_union` and not copying it. 2. The anchors in `choices` of `pair/1` (a new key only when the map is on: the bytes of the default stay as they are). 3. A synthetic pair of tables with a known RT shift, a decoy that only pairs without the map, and the parity test site=app=CLI. 4. `docs/schemi/pair-1.json` and the golden files regenerated with their tools.

**Out of scope.** A map fitted from the whole run; identification of compounds.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
