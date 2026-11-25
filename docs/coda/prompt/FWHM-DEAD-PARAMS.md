Read docs/coda/BRIEF.md.

# FWHM-DEAD-PARAMS: Map estimate's fwhm and dead time to step parameters

**Goal.** Map the `fwhm` and `dead` (dead time) values of `estimate` to step parameters, as proposals.

**Why.** They are measured but proposed nowhere (`skipped` with a reason) (follow-up of #3709).

**Read first.** `docs/agenti/estimate.md`, `crates/mzlab-core/src/estimates.rs` (value table, `fwhm`), `mzlab-cli flow defaults features.detect`.

**Already on main (checked 25 Nov, re-check before coding).** The estimate lists these values; no step parameter carries them. Find which step parameters (peak width, minimum scans, gap) correspond; if none, say so and stop.

**Done means.** Proposals with the `--set` line, the rule and the evidence, in `estimate/1` and `--emit-set`; schema regenerated; tests on synthetic files.

**Out of scope.** New step parameters.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
