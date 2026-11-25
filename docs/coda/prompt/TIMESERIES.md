Read docs/coda/BRIEF.md.

# TIMESERIES: Time-series helper: windows, injection check, first-order k

**Goal.** One helper for a time series: areas in fixed RT windows for an ion list, normalisation on the parent UV peak as an injection check, and a first-order rate constant estimate.

**Why.** The kinetic campaign needs these three steps by hand today.

**Read first.** `docs/studies/AGENT_USE_STUDY_2.md` §8 (the AU-8 recipe, kinetics from LR files), `docs/agenti/recipes.md` and `docs/recipes/kinetic-lr.mzflow.json`, `crates/mzlab-core/src/kinetics.rs`, `docs/agenti/calibration.md` (`quant curve`), `crates/mzlab-cli/src/workflow_kinetic_tests.rs`.

**Already on main (checked 25 Nov, re-check before coding).** The recipe, `kinetics.rs` (first-order decay, AICc, bootstrap), `quant targets` and `quant.curve` are merged. Reuse them; do not duplicate. What is missing is only the glue for fixed windows, the UV normalisation (see `dad trace`) and the k with its uncertainty printed.

**Done means.** 1. Decide on main whether this is a Flow step, a command or a recipe extension (prefer a step: it runs on site and app). 2. The areas, the normalisation factor per file and the k with its uncertainty and warnings; logged choices. 3. A synthetic series with a known k. Parity site=app=CLI.

**Out of scope.** Second-order and formation-decay models (exist in `kinetics.rs`).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
