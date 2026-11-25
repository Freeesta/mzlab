Read docs/coda/BRIEF.md.

# DAD-DELAY-APPLY: Apply the PDA-to-MS delay at conversion

**Goal.** The PDA (UV) delay against MS (about 0.15 min) is applied at conversion when the user accepts it, and logged.

**Why.** Overlaying UV and MS traces is off by the delay; it is today evidence only.

**Read first.** `crates/mzlab-cli/src/dad_cmd.rs` (`dad delay`: 'evidence to accept or not, never applied'), `crates/mzlab-cli/src/dad_ms.rs`, `docs/agenti/dad.md`, `crates/mzlab-cli/src/convert.rs` and `convert_help.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `dad delay` estimates the offset by matching UV peaks to the TIC; `dad ms-at` takes `--offset auto`. Conversion does not apply a delay.

**Done means.** 1. `file convert --dad-delay auto|MIN` (name it after checking the subcommand-name guard): applies the offset to the UV time axis and writes the value and its evidence in the conversion report. 2. Default off; an accepted estimate is a logged automatic choice. 3. Synthetic test with a known delay. Parity site=app=CLI.

**Out of scope.** Changing UV data values.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
