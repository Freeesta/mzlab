Read docs/coda/BRIEF.md.

# FLOW-TABLE-FINAL: Mark a feature table as the final one

**Goal.** In the Feature tables column of the Flow the chemist can mark one table as the final one (the `*` of `docs/studies/MZFLOW_SCREENS.md` §A); the mark is kept with the plan, logged as a user decision, and `mzlab-cli flow tables` reports it.

**Why.** mzflow study: the owner builds a TP feature table step by step and picks the one that goes to the report. Study: `docs/studies/MZFLOW_STUDY.md` (§5, §6 decisions).

**Read first.** `flow/lazy/flow-tables.js`, `crates/mzlab-feat/src/tables.rs`, `docs/schemi/tables-1.json`, `docs/agenti/flow-graphics.md`, the plan format `mzflow/2`.

**Already on main (checked 27 Nov, re-check before coding).** The column lists the tables with their facts; nothing marks one as final.

**Done means.** 1. The mark lives in the plan or the run record (Rust decides where; JS draws only) and is read by site, app and CLI with a byte-for-byte test. 2. `flow tables --json` carries it under a new key of `tables/1` (schema regenerated). 3. Keyboard reachable, texts through `lang/` keys, e2e assertion.

**Out of scope.** Deleting outputs.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
