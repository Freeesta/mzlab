Read docs/coda/BRIEF.md.

# ISOTOPES-UNIT: Unit-resolution isotope grouping and spill-over flag

**Goal.** At unit resolution group M, M+1, M+2 instead of separate features; flag that in a Q1 scan the +1 channel holding 50-80 % of the peak is instrument spill-over, not an isotope; optionally filter candidates of `tp.generate` (reachable from the parent in at most N steps and consistent with the M+2 ratio).

**Why.** Separate features for isotopes and spill-over inflate the table and give false transformation products.

**Read first.** `docs/agenti/ion-groups.md` (`features.group`, `features.companions`), `docs/agenti/mzfeat-tp.md`, `crates/mzlab-feat/src/tp/`, the merged `raw isotopes`.

**Already on main (checked 25 Nov, re-check before coding).** `features.companions` knows +1 and +2 at unit mass; `raw isotopes` gives the counting verdict. An ion at M+1 with r < 0.5 is never an isotope (study rule). Check what `tp.generate` already filters.

**Done means.** 1. Grouping with the rule, ratio bounds and tolerance as `--set` parameters, logged. 2. Spill-over flag with its evidence (ratio of the +1 channel to M, shape correlation). 3. The optional formula filter of `tp.generate` with N and the M+2 tolerance as parameters, off by default. 4. Synthetic tests. Parity site=app=CLI.

**Out of scope.** High-resolution isotope finder.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
