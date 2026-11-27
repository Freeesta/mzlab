Read docs/coda/BRIEF.md.

# WIFF-OPEN-INSTANT: Open selected is instant after the background load

**Goal.** After the index ahead (PR "Build the index of a ticked WIFF sample ahead", `rust-index-ahead.js`), "Open selected" on ticked samples no longer reads or indexes anything, but it still takes about 240 ms for 4 samples (measured by `tests_e2e/e2e_wiff_choose.py`, line "Open selected -> 4 rows listed in N ms"). That time is the engine opening each index: `lrIndexOpened` -> `lrOpen` -> `engine.open_index` + `nativo.attach` (`native_attach`) in `rust-worker.js`, which binds a slot named after the FINAL list name. Make it near zero too: do that work while the dialog is open, so Open only renames or rebinds.

**Why.** Owner priority of 27 Nov: a ticked sample must be ready to display; the "Loaded" label must be true. The file side is done (copy in #3765, index file in the index-ahead PR); the engine side is not.

**Read first.** `docs/agenti/wiff-sample-choice.md` (Index ahead), `docs/agenti/map-loading.md` (row 8, `lrOpen`), `mzlab/web/rust-index-ahead.js`, `mzlab/web/rust-worker.js` (`lrOpen`, `lrIndexOwn`, `lrIndexOpened`, `slotOf`), `crates/mzlab-wasm/src/natives_engine.rs` (`native_attach`, `open_index`).

**Already on main (re-check before coding).** The `.mzidx` of a ticked sample is built after its copy under `idx/nat~<key>~<id>~pre.mzidx` and moved to `<final name>.mzidx` at Open (`adopt`); the byte test `the_index_bytes_do_not_depend_on_the_list_entry_name` pins that the index does not depend on the list name; `natstats` counts built, adopted, missed, at-open.

**Done means.** 1. Measure first: split the 240 ms between `open_index` (header, meta, scan table), `attach` and the rest, and put the split in the PR body. 2. Open the index in an engine slot while the dialog is open (a neutral slot name, or the engine learns to rebind a slot to another name in Rust: one operation, wasm + native + CLI, a byte-for-byte test that fails, not skips, if it is an engine change; if it stays in the worker, state why that is no parity gap). Untick and X close the slot and free the memory; at most 2 jobs run at once; the dialog never blocks; no visible status text (WIFF-PICKER-POLISH owns the wording). 3. A node or e2e test that Open after the background load makes no `open_index` call at Open (count it) and the time from Open to the first drawn chromatogram is near zero; tests for a failure mid-open (memory refused: `openStored`) that leaves the opening to do it as before, two dialogs at once, Open in the copy phase, reopening after X. 4. `.wasm` caps in `tools/moduli/*.toml` move only by the CI-measured size, at most 5 % per PR, numbers in the PR body; `commands --json` compact stays under 33,000 B and docs under 25,600 B.

**Out of scope.** The Files-list counter and every per-row status text (WIFF-PICKER-POLISH); the copy and the index file themselves (done).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
