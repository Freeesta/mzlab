Read docs/coda/BRIEF.md.

# DETECT-VS-FORCED: Detection must be distinguishable from forced integration

**Goal.** A user (and an agent) can tell a detected peak from a forced integration in every output that reports a quantity, and can tune the detection thresholds with `--set`: the S/N threshold and a peak-shape check.

**Why.** Reported symptom: peaks at S/N 3-5 were integrated in matrix and blank controls, where another tool that does not force-integrate found none of 35. Forced values look like detections downstream.

**Read first.** `docs/agenti/value-states.md` (AU-3, merged), `docs/agenti/mzfeat.md`; `grep -rn 'value_state' crates --include=*.rs -l`; the detection step parameters: `mzlab-cli flow defaults features.detect` and `features.gapfill`.

**Already on main (checked 25 Nov, re-check before coding).** AU-3 words detected, forced, filled, below_loq, absent, na (`mzlab_feat::value_state`) reach `quant.curve`, `quant.calibrate`, `chrom.transitions`, `peak auto-lr`, `peak table`, the Dig detail. They do NOT yet reach the aligned feature-table CSV, the report and the exports (`na` prints `UNKNOWN` there; that bytes change is the row NA-CELL-STATE, owner decision: do not touch it here). `raw xic` (AU-13) already has `min_sn`, `min_points`, `alpha` as `--set`.

**Done means.** 1. List, with evidence, every output where a forced or filled value is still shown as a plain number; fix the ones that add a key or an opt-in column (never change an existing byte). 2. Detection gets `--set` parameters for the minimum S/N and a peak-shape check (for example minimum points above half height, symmetry), defaults equal to today's behaviour, printed by `flow defaults`; parity site=app=CLI. 3. A synthetic test: a trace with noise-only bumps at S/N 3-5 is detected under the old defaults and refused when the new threshold is raised, with the refusal logged with its evidence (S/N, shape).

**Out of scope.** Changing the aligned CSV cell text; re-deciding what counts as detected in `quant targets`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
