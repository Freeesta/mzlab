Read docs/coda/BRIEF.md.

# MZ-OFFSET-POL: Mass offset estimated automatically per polarity

**Goal.** The mass offset is estimated automatically on the parent compound, separately for each polarity.

**Why.** Positive and negative runs are calibrated apart: one offset applied to both is wrong for one of them.

**Read first.** `crates/mzlab-core/src/mzcal.rs` (`Spec::Auto`, `estimate(idx, pol, ...)`), `crates/mzlab-cli/src/mz_offset.rs` and `mz_offset_tests.rs`, `docs/agenti/cli-agenti.md`.

**Already on main (checked 25 Nov, re-check before coding).** `--mz-offset DA|auto:MZ[@RT]` exists on `trace xic` and `features` and the estimate takes a polarity (`pol`). `raw xic --mz-offset` exists. Verify FIRST, on an alternating-polarity synthetic file, whether `auto` picks the right polarity section for each file and for `features`/`raw xic`; the row may reduce to tests plus a per-polarity report.

**Done means.** 1. A test on a synthetic file with +0.3 Da in positive and -0.2 Da in negative: each polarity gets its own offset. 2. The offset applied and its evidence (ion, RT, polarity, points) are printed per polarity and logged. 3. If a gap is found, fix it in the engine so site, app and CLI agree byte for byte.

**Out of scope.** RT correction (row RT-ALIGN-REF); calibrating the full mass range.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
