Read docs/coda/BRIEF.md.

# MS2-SUM: A true scan sum for spectrum get

**Goal.** `spectrum get --sum` returns the SUM of the scans in `--rt A,B` (today `--rt A,B` returns the mean), with the number of scans and the total ion count (cps x fill time), so an agent can judge whether an MS2 spectrum is credible (24 ions versus 490 on an enhanced-product-ion file).

**Why.** A mean hides how many ions stand behind the spectrum. Without the totals an agent cannot say whether a fragment list is evidence or noise.

**Read first.** `docs/studies/RAW_ACCESS_STUDY_2.md` §8 (AU-15 `raw ms2`) first, then `crates/mzlab-cli/src/spectrum_cmd.rs` (usage text, `--rt A,B`), `docs/agenti/data-channels.md`, `crates/mzlab-core/src/ionstat.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `spectrum get` has `--rt MIN`, `--rt A,B` (mean), `--scan`, `--minus`. There is no `--sum`. `raw xic` (AU-13) and `ionstat` are merged: reuse the ion quantum and the fill-time reading.

**Done means.** 1. `--sum` with `n_scans`, `total_ions` and the scale used (cps x fill time) in the output; the default mean is unchanged byte for byte. 2. When the fill time cannot be read, say so and give the counts in cps only (logged). 3. Same route for site and app (`api/spectrum`), CLI flag, catalog entries, schema `spectrum/1` gains the keys. Tests on a synthetic MS2 file with known scan counts and fill times; parity byte test.

**Out of scope.** Peak picking of the apex (row MS2-PEAK-AUTO); the full `raw ms2` command (row RAW-MS2).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
