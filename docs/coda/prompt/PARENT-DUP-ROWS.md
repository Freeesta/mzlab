Read docs/coda/BRIEF.md.

# PARENT-DUP-ROWS: One compound, one feature row (also the nominal-resolution profile)

**Goal.** The parent compound's peak no longer splits into several duplicate rows of the feature table. This row also covers the nominal-resolution preset (a profile of `features` for unit-resolution triple-quad or QTRAP data).

**Why.** Reported: the parent peak appears as several rows; at nominal resolution one compound splits into several features and low-mass products are lost.

**Read first.** `docs/agenti/mzfeat-lr.md` (the existing low-resolution chain: `Recipe::lr`, floor, min height), `docs/agenti/ion-groups.md` (`features.group`, `features.companions`, AU-10), `crates/mzlab-feat/src/recipe_lr.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `Recipe::for_index` already picks a low-resolution recipe from the data; `features.companions` groups unit-mass companions. Find, on a synthetic unit-resolution file with one parent and a split peak (m/z 106.7 and 107.0 style), WHERE the duplicates come from (detection split in m/z, in RT, or after alignment) before choosing a fix.

**Done means.** 1. A synthetic failing test with the cause named. 2. The fix in the grouping or in the nominal recipe, with the rule and tolerance logged and exposed as `--set` parameters (defaults printed by `flow defaults`). 3. Low-mass products are kept: a test with a product below the current floor. 4. If a `features` profile is the right shape, name it and list the parameters it sets. Parity site=app=CLI.

**Out of scope.** Changing the HR defaults; the bin command itself (row NOMINAL-BIN).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
