Read docs/coda/BRIEF.md.

# ISOTOPES-SPILL-RAW: spill-over verdict in `raw isotopes`

**Goal.** Let `raw isotopes` say, from the Q1 scan itself, whether the +1 channel of a peak is instrument spill-over: the ratio of the +1 bin to M inside the peak region, its shape correlation with M and a verdict, with the band and the correlation floor as `--set` parameters (defaults of `features.companions`: `spill_min` 0.5, `spill_max` 0.8, `iso_min_corr` 0.5).

**Why.** `features.companions` flags spill-over from a feature table; a user with one file and no table has only the counting verdict of `raw isotopes`, which cannot tell a +1 channel at 65 % of M from a large-molecule isotope.

**Read first.** `docs/agenti/raw-access.md` (`raw isotopes`), `docs/agenti/ion-groups.md` (isotope grouping and spill-over), `crates/mzlab-core/src/rawiso.rs`, `rawiso_json.rs`, `crates/mzlab-feat/src/companions/iso.rs` (the rule to share).

**Already on main (check before coding).** `raw isotopes` gives net counts, ratios with sigma, `min_r` for the M+1 correlation and the counting verdicts; `features.companions` with `iso_group` gives the table-level spill-over verdict. The two do not share one function.

**Done means.** 1. One shared rule (ratio band plus correlation floor) used by both routes. 2. A `spillover` entry in `raw-isotopes/1` with the ratio, its sigma and the correlation, and a warning in `warnings`. 3. Synthetic test with a known +1 channel; parity site = app = CLI (the wasm leg fails, not skips, in CI). 4. Entry point findable with `commands --find`.

**Out of scope.** The high-resolution isotope finder; changing the table-level step.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
