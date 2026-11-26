Read docs/coda/BRIEF.md.

# KINETIC-CLASS: One kinetic class column per row

**Goal.** A new Flow step `table.kinetic_class@1` (opt-in: no existing bytes change) gives each row, per condition, a class (bell, rising, decreasing, background) and the numbers Max/T0, Max/blank, treated/dark, tmax and t1/2 with its uncertainty. It classifies, never deletes or hides.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 5 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §B, §C.

**Read first.** `crates/mzlab-feat/src/kinetic/` (forms by AICc, evidence), `crates/mzlab-core/src/kinetics.rs`, `docs/agenti/high-resolution-statistics.md` (`table compare`), `docs/studies/TP_SCREENING_RULES.md` (dark pooled with blank in the Dig), `docs/agenti/groups.md`.

**Already on main (checked 26 Nov, re-check before coding).** Forms formation, rise-and-fall, flat and decay; `table compare` ratios vs T0 and blank; LR per-condition labels. Reuse them; the new parts are the dark ratio, the one class column and tmax / t1/2 per row.

**Done means.** 1. Class map from the existing forms (study §6), thresholds as `--set` parameters whose defaults come from the existing engines and are printed. 2. `na` times never count as 0; replicate means with n logged. 3. wasm + native + CLI byte test; `flow steps` catalog and agent docs.

**Out of scope.** Drawing the curves (FEATURE-KINETICS).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
