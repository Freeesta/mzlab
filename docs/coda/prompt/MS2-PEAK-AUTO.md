Read docs/coda/BRIEF.md.

# MS2-PEAK-AUTO: spectrum get --peak auto: apex, 2 x FWHM window, background from both sides

**Goal.** `spectrum get --peak auto` finds the apex in the chromatogram of the product-ion scan, sums over about 2 x FWHM and subtracts the background taken from both sides. Every choice is logged with its evidence.

**Why.** Agents pick the window by hand and get it wrong; a fixed window sums background and drops the apex.

**Read first.** `crates/mzlab-cli/src/spectrum_cmd.rs`, the `--sum` of the row MS2-SUM (merged first), `docs/agenti/peak-info.md`, `crates/mzlab-core/src/ionstat.rs`, `docs/studies/RAW_ACCESS_STUDY_2.md` §4 (side windows `--side`, `--gap`).

**Already on main (checked 25 Nov, re-check before coding).** Peak finding and FWHM exist for traces (`peak auto-lr`); side-window background exists in `raw xic`. Reuse them.

**Done means.** 1. `--peak auto` with `--set` for the window factor (default 2), the side width and the gap; the output names the apex time, FWHM, window, side windows and the background level, and keeps `n_scans` and `total_ions`. 2. When no apex is found, no spectrum is invented: a verdict with the evidence. 3. Synthetic test with a known apex and a flat background. Parity site=app=CLI.

**Out of scope.** Compare two spectra (row SPECTRUM-COMPARE).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
