Read docs/coda/BRIEF.md.

# STATES-PROV: table.stats and quant.curve name the states they used in prov/1

**Goal.** `table.stats` and the input filters of `quant.curve` record in `prov/1` which value states (detected, forced, filled, below_loq, absent, na) they accepted.

**Why.** A statistic that mixes forced and detected values must say so (AU-3 follow-up of #3704 and #3705).

**Read first.** `docs/agenti/value-states.md` ('Not yet'), `docs/agenti/stat.md`, `docs/agenti/calibration.md`, `crates/mzlab-feat/src/value_state.rs`. Start with `grep -n 'value states\|statistics as steps' docs/agenti/INDICE.md`, then `grep -rn 'table.stats' crates/mzlab-feat/src --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** The state words and `state` columns in the concentration outputs are merged. The `prov/1` entry has no state list (verified: no `states` key under `crates/mzlab-feat/src/stat/`).

**Done means.** 1. A `states` member in the `prov/1` entry of both steps, additive; unchanged bytes elsewhere. 2. A parameter to choose the accepted states with the current behaviour as default, logged. 3. Tests; docs line in `value-states.md`. Parity site=app=CLI.

**Out of scope.** Aligned CSV changes (row NA-CELL-STATE).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
