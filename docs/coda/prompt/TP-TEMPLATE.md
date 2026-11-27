Read docs/coda/BRIEF.md.

# TP-TEMPLATE: A TP workflow template

**Goal.** A saved `mzflow/2` template for TP studies: inputs parent, files, metadata; blocks A direct, B kinetic class, C suspect, union with origin, pillars; offered in the Flow templates and runnable with `mzlab-cli flow run`.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 15 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §A.

**Read first.** `docs/recipes/kinetic-lr.mzflow.json`, `docs/agenti/recipes.md`, `flow/lazy/flow-names.js` (`F.TEMPLATES`), `crates/mzlab-cli/src/workflow_kinetic_tests.rs`.

**Already on main (checked 26 Nov, re-check before coding).** One kinetic LR recipe exists.

**Done means.** 1. The template, tested end to end on synthetic files. 2. Same outputs from the site runner and `flow run`. 3. Listed in the agent recipes.

**Out of scope.** Two parents.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
