Read docs/coda/BRIEF.md.

# FLOW-TABLE-LIST: The list of feature tables on the right of the Flow

**Goal.** The right column of the Flow lists every feature table of the runs of the plan (and tables opened from disk): block and polarity, rows, files, state, time; a click opens it in the existing Feature table view. The CLI twin `mzlab-cli flow tables DIR --json` lists the tables of a `flow run` folder with the same facts.

**Why.** mzflow study: the owner builds a TP feature table step by step and sees each step. Study: `docs/studies/MZFLOW_STUDY.md` (row 2 of §5, §6 decisions) and `docs/studies/MZFLOW_SCREENS.md` §A.

**Read first.** `docs/agenti/flow-graphics.md`, `flow-live.md`, `flow-web.md` (OPFS `mzlab/flow/live/<plan_key>.*`), `flow/lazy/flow-run-end.js`, `flow-views-features.js`, `flow-shell.js`; `.mzfeat` meta in `crates/mzlab-feat/src/store.rs`; `prov/1`.

**Already on main (checked 26 Nov, re-check before coding).** Runs, outputs in OPFS and the table view exist; the run end opens only the last table. No list exists.

**Done means.** 1. A Rust function that reads the meta of one table output (plus its `prov/1` when present) and answers one `tables/1` entry; wasm in a module with room (not `rotte`), native and CLI the same bytes, test byte for byte. 2. The column in the Flow (JS draws only), keyboard reachable, texts through `lang/` keys. 3. CLI route listed in the agent catalogs with exit codes pinned.

**Out of scope.** Marking a table as final; deleting outputs (never silently: out of scope).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
