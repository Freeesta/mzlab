Read docs/coda/BRIEF.md.

# BLANK-BAND: Blank level per m/z band in estimate

**Goal.** `estimate` proposes a blank level per m/z band using the sample roles.

**Why.** One blank level for the whole range hides a noisy low-mass region (follow-up of AU-6, #3709).

**Read first.** `docs/agenti/estimate.md`, `crates/mzlab-core/src/estimates.rs`, `docs/agenti/samples.md` (roles), the schema `docs/schemi/estimate-1.json`.

**Already on main (checked 25 Nov, re-check before coding).** `estimate` (merged) aggregates per value; roles come from `--samples` or names (AU-2, merged).

**Done means.** 1. `blank` bands with the width as a `--set`, per-band median and evidence, a `--set` line for `blank_ratio` or the floor per band if the plan has such a parameter (otherwise evidence only, logged). 2. Schema regenerated with its tool. 3. Synthetic blank with a noisy low band. Parity site=app=CLI (`estimate_set` is in `rotte`: do not grow it; see the brief).

**Out of scope.** RT drift (row RT-DRIFT-PROPOSAL).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
