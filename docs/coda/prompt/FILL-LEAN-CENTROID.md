Read docs/coda/BRIEF.md.

# FILL-LEAN-CENTROID: Gap filling of a centroided series on a lean index in the browser

**Goal.** The page fills the gaps of a unit-resolution series that is centroided before detection (`centroid` auto or on) with the same cells as the app, without opening each file whole.

**Why.** The lean index of the browser fill has no scan table, so it cannot decide that a file is profile and cannot be centroided: the fill then read raw profile scans and gave other cells than the app (337 against 58 filled cells on a synthetic replicate series). The parent-duplicate PR fixed the bytes by opening such series whole (`crates/mzlab-feat-wasm/src/series_js.rs`), which holds every scan table in memory.

**Read first.** `grep -n 'lean\|centroid' docs/agenti/INDICE.md`, then `docs/agenti/mzfeat-lr.md` (screening across samples), `crates/mzlab-feat-wasm/src/series_js.rs`, `crates/mzlab-feat/src/lrscreen.rs` (`centroid_wanted`, `centroids_of`), `crates/mzlab-core/src/idxread.rs` (`open_lean`).

**Done means.** 1. The profile share of each file (`series_recipe` reads every scan table once for `file_facts`; add the share there if it is missing) reaches the fill, so `centroid_wanted` decides without a scan table, and a lean index can be centroided. 2. `tests/test_feat_wasm.py::test_the_dedup_overlap_rule_gives_the_same_bytes_native_and_in_node` stays green with the lean opener back for centroided series. 3. Peak memory of the page fill on a 30-file synthetic series is measured before and after and written in the PR body. 4. wasm caps: module, raw bytes before and after and % in the PR body.

**Out of scope.** The dedup rules and their thresholds; high-resolution series.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
