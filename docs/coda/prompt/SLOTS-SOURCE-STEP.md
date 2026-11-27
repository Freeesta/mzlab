Read docs/coda/BRIEF.md.

# SLOTS-SOURCE-STEP: The exact step and output behind each slot

**Goal.** `source.step` of `feature-slots/1` is the step that records the evidence of that slot in the table, read from the `.mzfeat` itself, and `source.output` is the name of the table in the page too. Today the step is the one that normally writes that kind of evidence (`features.align` for the XIC, `features.ms2_link` for MS2, ...), and the page gives no output name for a table that is held in memory, so the slot shows "this table".

**Why.** FEATURE-DETAIL (`crates/mzlab-feat/src/slots.rs`): the `.mzfeat` records its recipe as a whole, not one step per column, so the step id was set by kind. A provenance line that is right only by convention is weaker than one read from the file.

**Read first.** `crates/mzlab-feat/src/slots.rs` (`writer`, `SLOTS`), `docs/agenti/mzfeat-table.md` (the recipe in the file), `docs/schemi/feature_slots-1.json`, `TP_Mine/js/34_pipeline_run.js` (where the session is opened).

**Already on main (checked 27 Nov, re-check before coding).** `feature-slots/1`, `table slots`, the `table_slots` wasm route, the `feat/table` `slots` app route and the Slots tab exist; `source.step` is by kind.

**Done means.** 1. The step id of each slot comes from the recipe of the `.mzfeat` (a new field or the existing recipe), with a test on a synthetic table whose recipe has two different steps. 2. The page passes the output name to `session.slots`. 3. wasm + native + CLI byte test; the schema and the agent catalogs updated; guards not raised.

**Out of scope.** A new table format version beyond what the recipe needs; the UV and kinetics slots (FEATURE-UV, FEATURE-KINETICS).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
