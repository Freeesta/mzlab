Read docs/coda/BRIEF.md.

# FLOW-TABLE-DISK: Facts of a .mzfeat opened from disk

**Goal.** A `.mzfeat` chosen with «Open .mzfeat…» in the Feature tables column of the Flow shows rows, files and polarity like the tables of a run, not only its name and size.

**Why.** The column of FLOW-TABLE-LIST lists the tables of the plan with the facts of Rust (`tables/1`); a file from disk only has its name, because the worker op `table_entry` reads OPFS paths.

**Read first.** `flow/lazy/flow-tables.js`, `TP_Mine/files/feat-worker.js` (`table_entry`, `openAll`), `crates/mzlab-feat/src/tables.rs` (`entry` reads the header and the meta only), `docs/agenti/flow-graphics.md`.

**Already on main (checked 27 Nov, re-check before coding).** The entry of a table of OPFS comes from the table wasm; a disk file is listed by name and size.

**Done means.** 1. The facts of a disk file come from the same Rust `entry` (a head slice of the file plus its size is enough); no logic in JS. 2. e2e assertion on the list. 3. No new `.wasm` growth beyond the rule of `docs/agenti/wasm-caps.md`.

**Out of scope.** Copying the file into OPFS; marking a table as final.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
