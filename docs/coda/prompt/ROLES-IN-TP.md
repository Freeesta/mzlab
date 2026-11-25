Read docs/coda/BRIEF.md.

# ROLES-IN-TP: Sample roles reach tp.dig_lr and the screening steps

**Goal.** Roles (irradiated, dark, blank, standard, QC) read from a file or a name rule are used by `tp.dig_lr` and the screening steps, and a plan that contains a standard does not fail.

**Why.** Reported bugs: dark controls are not recognised, and `tp.dig_lr` fails when the plan contains a standard. Reproduce on synthetic data first; if you cannot, say so in the PR body with the synthetic case you tried.

**Read first.** `docs/agenti/samples.md` (AU-2, `samples/1`, `sample import` merged), `docs/agenti/mzfeat-tp.md`, `crates/mzlab-feat/src/tp/meta.rs` and `gates.rs` (role counts, `dropped_by_dark`), `crates/mzlab-core/src/samples/names.rs` (name parser; a dark keeps its time but its role is `dark`).

**Already on main (checked 25 Nov, re-check before coding).** `samples::propose` and `role_and_minutes` give roles from names; `tp/2` meta already counts `dark_over_t0` and blank kinds. Check how the roles of `sample import` (CSV or JSON) are handed to the `tp.dig_lr` step: `grep -rn 'dig_lr' crates/mzlab-cli/src/flow_lr_tests.rs`.

**Done means.** 1. A synthetic series (names with dark, blank, standard and QC words in the usual spellings, plus one with roles from an imported file) run through `flow run` with `tp.dig_lr`: a failing test first. 2. Fix the cause: roles are read once, from the one model, by all the steps; unknown roles are listed, never guessed silently (log the rule used). 3. The standard in the plan is skipped for the TP logic with a logged reason, or used as the standard role; never an error. 4. Parity site=app=CLI with a byte test.

**Out of scope.** New role words beyond the existing parser; the calibration of `quant.curve`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
