Read docs/coda/BRIEF.md.

# NOMINAL-BIN: Nominal-mass bins for low-resolution profile data

**Goal.** `--bin nominal` gives nominal-mass centroids or bins for low-resolution linear-trap profile data, where a fragment is split between two integer values (for example 106.7 and 107.0), honouring `--mz-offset`.

**Why.** At unit resolution one ion appears as two peaks; every downstream list is wrong by one ion.

**Read first.** `docs/studies/RAW_ACCESS_STUDY_2.md` §C and §4 (`--bin nominal|0.1`), `crates/mzlab-cli/src/spectrum_cmd.rs`, `crates/mzlab-core/src/mzcal.rs`, `crates/mzlab-core/src/hrscanbin.rs`.

**Already on main (checked 25 Nov, re-check before coding).** No `--bin` on `spectrum get` today. `--mz-offset DA|auto` exists on `trace xic`, `features` and `raw xic`; the binning must apply the offset first and log it. Check `grep -rn 'hrscanbin' crates --include=*.rs -l` for an existing binner.

**Done means.** 1. `spectrum get --bin nominal` (and `0.1`): sums the profile points into half-open bins around integers after the offset, outputs the bin, the centre of mass and the counts; the rule and offset are logged. 2. Synthetic profile with a fragment split 106.7/107.0 gives one bin. 3. Route shared by site and app, CLI flag, schema keys, parity byte test.

**Out of scope.** Spectrum comparison (row SPECTRUM-COMPARE); the full `raw scan`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
