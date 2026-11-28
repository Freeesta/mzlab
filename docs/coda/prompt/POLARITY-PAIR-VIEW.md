Read docs/coda/BRIEF.md.

# POLARITY-PAIR-VIEW: Show the polarity pairs in the Flow page

**Goal.** The result of the step `features.pair` opens in the Flow page as a table: one line per pair with the neutral mass, the mass error (Da and ppm), the RT difference, the rule and `mutual`, then the unmatched rows of each side; the CSV and the `pair/1` stay the files of the run.

**Why.** The step runs in the page and writes `pair/1`, but the owner reads the pairs in the JSON or in a spreadsheet; the point of the step (which species behave as acids) is read at a glance in a table.

**Read first.** `docs/agenti/polarity-pair.md`, `docs/agenti/flow-web.md` (grep `union` for how a `Report` output is shown), `flow/lazy/flow-web-pair.js`, `docs/schemi/pair-1.json`.

**Already on main (re-check before coding).** The engine, the step, `table pair`, the `flowtab` export `flow_pair` and the glue `flow-web-pair.js` exist. There is no view of `pair/1`.

**Done means.** 1. A view of `pair/1` opened from the output of the step, every text through `lang/` keys. 2. A sort by neutral mass and by |RT difference|. 3. A Playwright test on a synthetic pair of tables (`crates/mzlab-feat/tests/data/pair/`). 4. No new `.wasm` growth beyond the caps.

**Out of scope.** Changing the pairing rule or the bytes of `pair/1`; compound identification.

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
