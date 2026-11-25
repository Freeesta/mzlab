Read docs/coda/BRIEF.md.

# FEATURES-NOMINAL: A features profile for nominal-resolution triple-quad and QTRAP data

**Goal.** A preset or profile of `features` for nominal-resolution triple-quad or QTRAP data, so one compound does not split into several features and low-mass products are not lost.

**Why.** Reported: at nominal resolution one compound splits into several features and low-mass products are lost. The running chat PARENT-DUP-ROWS fixes the duplicate rows of the parent peak; this row is the preset and the low-mass floor, after it.

**Read first.** `docs/agenti/mzfeat-lr.md` (the existing low-resolution chain: `Recipe::lr`, floor, min height), `docs/agenti/ion-groups.md` (`features.companions`), `crates/mzlab-feat/src/recipe_lr.rs`, and the PR of the chat PARENT-DUP-ROWS (merged first: read its diff; stop and report if it is not). Start with `grep -n 'unit resolution\|low-resolution\|mzfeat-lr' docs/agenti/INDICE.md`, then `grep -rn 'fn lr(' crates/mzlab-feat/src --include=*.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `Recipe::for_index` already picks a low-resolution recipe from the data; `features.companions` groups unit-mass companions. Find on a synthetic unit-resolution file (one parent, products below the floor) which parameters decide the split and the loss.

**Done means.** 1. A synthetic failing test for the low-mass product loss and for the split that remains after PARENT-DUP-ROWS. 2. A named profile (or documented `--set` bundle) with every parameter and its default printed by `flow defaults`; the choice of the profile from the data is logged. 3. Parity site=app=CLI.

**Out of scope.** Duplicate parent rows (the running chat); changing the HR defaults.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
