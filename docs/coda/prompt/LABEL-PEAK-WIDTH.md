Read docs/coda/BRIEF.md.

# LABEL-PEAK-WIDTH: Peak-width cue for the profile-label check

**Goal.** The label check reads zero-suppressed profile peaks of only 3 points as sparse ion counts (documented in `crates/mzlab-cli/src/features_label_tests.rs`), so such a profile file keeps its profile points instead of being centroided. Decide with numbers whether a peak-width cue fixes it without turning a real mislabelled file into a false negative.

**Read first.** `crates/mzlab-core/src/spectrum_type.rs` (`Runs`, constants), `crates/mzlab-feat/src/lrscreen.rs` (`label_check`), `docs/agenti/mzfeat-lr.md` (Label check), `tools/lr_synthetic.py` (`mislabel`).

**Done means.** (1) Synthetic files that cover zero-suppressed 3-point peaks, normal profile, and centroids under a profile label; the cut and its false-positive and false-negative counts in the PR body. (2) `raw info` and the label check keep one detector (they must classify a file alike). (3) The user still decides: the notice carries the numbers; `--centroid on|off` overrides. No change to existing goldens without saying so.

**Out of scope.** Changing bytes of existing outputs (a hint is a new key or a stderr line, never a changed value); new aliases.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
