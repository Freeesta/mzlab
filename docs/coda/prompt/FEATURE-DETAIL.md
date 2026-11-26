Read docs/coda/BRIEF.md.

# FEATURE-DETAIL: The feature view with fixed slots

**Goal.** Opening a row in a Flow table shows one view with fixed slots (structure, peak/XIC, kinetics, MS2, isotopes/adducts, UV-Vis, SIM). A slot shows existing evidence, or says which step or CLI command fills it. `mzlab-cli table slots TABLE.mzfeat --id N --json` prints the same `feature-slots/1`.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 3 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §D.

**Read first.** `TP_Mine/js/44_tp_riga.js` (`TPRIGA`), `47_tp_struttura.js`, `45_tabui_decisioni.js`, `docs/agenti/mzfinder-table.md`, `docs/agenti/value-states.md`, `crates/mzlab-feat/src/row.rs`.

**Already on main (checked 26 Nov, re-check before coding).** `TPRIGA` draws XIC (blank and dark dashed), MS1/MS2/mirror, samples, identity, structure; the row model is `row/2`.

**Done means.** 1. `feature-slots/1` in Rust: per slot `state` (filled / empty / na), the source (output, step, file SHA-256) and the step id and CLI command that fill it; never a made-up value. wasm + native + CLI, byte test. 2. The view reuses `TPRIGA` parts in a fixed grid (stacks under 600 px). 3. CLI route in the agent catalogs.

**Out of scope.** Drawing kinetics and UV (FEATURE-KINETICS, FEATURE-UV); card export.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
