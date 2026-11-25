Read docs/coda/BRIEF.md.

# COLLISION-LABEL: One label function for colliding sample names

**Goal.** The label of a table column when two chosen samples share a name is `NAME (FILESTEM #ID).ext`, computed by ONE Rust function that the site (`.wasm`), the app and the CLI all call. The decision is taken; there is no owner step.

**Why.** #3717 gave multi-sample files sample-name columns (`crates/mzlab-cli/src/sample_label.rs`). The site still names a sample with the counter of `wiffName` (`S.wiff`, `S_2.wiff`, `mzlab/web/conversione.js`), so on a collision site and CLI differ. `convert --name sample` (`SAMPLE-ID.mzML`) is a third rule for output file names and stays as it is.

**Read first.** `grep -n 'sample label' docs/agenti/INDICE.md`, then `docs/agenti/wiff-sample-choice.md` (label rule); `sample_label.rs` and `sample_label_tests.rs`; `tests/test_label_parity.py` (node `wiffName` against `features --sample --dry-run`); `docs/agenti/wasm-caps.md`. Check on `origin/main` what already moved.

**Done means.** (1) The clean step (trim, `/ \ : * ? " < > | #` become `_`, 120 characters), the tie-break and the empty-name rule live in one pure function of an existing wasm module with room (or a new lazy one), called by the CLI, the app and the Flow, and by the site where it builds column or row names. (2) The site counter is replaced for column labels only; any e2e golden that changes is regenerated deliberately and the PR body says which. (3) Extend `tests/test_label_parity.py`: byte-for-byte equality of the label on several multi-sample files with colliding names and names holding `/`, `\`, `@` and spaces; its wasm leg FAILS, not skips, in CI. (4) The PR body gives module, raw bytes before and after and % from the CI size step.

**Out of scope.** `convert --name sample` and its file names; real data (synthetic files from `tools/wiff_grande.py --names`); any cap change outside the per-PR growth rule.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
