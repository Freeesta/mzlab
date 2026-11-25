Read docs/coda/BRIEF.md.

# RT-DRIFT-PROPOSAL: estimate proposes rt_warp=piecewise from measured drift

**Goal.** `estimate` measures the RT drift between files and proposes `rt_warp=piecewise` with its evidence.

**Why.** A follow-up of AU-6 (#3709): the tolerance is estimated, the warp is not.

**Read first.** `docs/agenti/estimate.md`, `crates/mzlab-core/src/estimates.rs`, `crates/mzlab-feat/src/rtwarp.rs`, `crates/mzlab-cli/src/flow_drift.rs`, the merged RT-ALIGN-REF.

**Already on main (checked 25 Nov, re-check before coding).** `rt_warp` exists on `features.union`; a drift step exists. Reuse the measurement of RT-ALIGN-REF if merged.

**Done means.** 1. A drift member of `estimate/1` (per file, anchors used) and a `--set` proposal line for `rt_warp`; never applied. 2. Synthetic files with a known drift. 3. Schema regenerated. Parity site=app=CLI; `rotte` must not grow over the brief's rule.

**Out of scope.** Applying the warp.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
