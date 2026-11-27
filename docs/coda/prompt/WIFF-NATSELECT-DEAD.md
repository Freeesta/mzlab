Read docs/coda/BRIEF.md.

# WIFF-NATSELECT-DEAD: Drop the site route nothing calls

**Goal.** The simplified "Choose samples" dialog no longer has a number box, so the worker op `natselect` (`rust-worker.js`, `rpc_nat.rs`, `nativeAsk`) and the wasm export `sample_selection` have no caller in the page; the CLI `--sample` keeps its own Rust path. Check with grep that nothing else in the site or the app calls them, then remove the unused leg or give it a real caller (`--sample` parity with the CLI must stay tested another way).

**Read first.** `docs/agenti/wiff-sample-choice.md` (`--sample LIST`), `tests/test_sample_selection_parita.py`, `docs/agenti/wasm-caps.md`.

**Done means.** (1) No dead op or export. (2) `--sample` of the CLI and its parity test still pass or move to the leg that remains. (3) `.wasm` sizes: put module, bytes before, after and % in the PR body.

**Out of scope.** Any change to `sample select` output bytes.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
