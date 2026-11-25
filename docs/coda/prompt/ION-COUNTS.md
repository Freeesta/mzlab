Read docs/coda/BRIEF.md.

# ION-COUNTS: Counts in ions as well as cps, per scan type

**Goal.** Export both scales, cps and ions. The cps-to-ions factor depends on the scan type (enhanced MS, Q1 scan, SIM as a function of dwell time, enhanced product ion) and is read from the method.

**Why.** Poisson S/N only makes sense in ions. A wrong factor makes every counting verdict wrong in a systematic way.

**Read first.** `crates/mzlab-core/src/ionstat.rs` and `rawxic*.rs` (AU-13: ion quantum, `sn_floor`, Currie L_C/L_D), `docs/agenti/raw-access.md`, `docs/studies/RAW_ACCESS_STUDY.md` (ion statistics, §4), `docs/agenti/method.md` (fill time and dwell are in `method/1`). Start with `grep -n 'ionstat\|raw xic' docs/agenti/INDICE.md`, then `grep -rn 'sn_floor' crates/mzlab-core/src --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** `raw xic` merged: it estimates the ion quantum from the data and reports counts. Reuse it; do not write a second estimator. What is missing: the factor per scan type from the method, and both scales in the exports.

**Done means.** 1. One function gives the factor for a scan type, with the evidence (method field or data estimate) logged; when the method cannot be read, the data estimate is used and the choice is logged. 2. `raw xic` and the trace and peak outputs gain opt-in ion-scale columns or keys (existing bytes unchanged). 3. Synthetic files for each scan type with a known factor; tests for the factor and for the Poisson S/N. Parity site=app=CLI.

**Out of scope.** Re-deriving the quantum; the real instrument constants (ask the coordinator for the format facts only).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
