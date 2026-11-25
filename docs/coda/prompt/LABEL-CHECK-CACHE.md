Read docs/coda/BRIEF.md.

# LABEL-CHECK-CACHE: Probe the profile label once per file

**Goal.** `lrscreen::label_check` probes up to 64 MS1 scans each time `centroids_of` asks; a feature build of a mislabelled file takes 0.06 s against 0.03 s for the same data labelled centroid. Probe once per file and reuse the answer inside one run.

**Read first.** `crates/mzlab-feat/src/lrscreen.rs` (`label_check`, `centroid_wanted`, `adjusted`), `crates/mzlab-core/src/rawinfo.rs` (`probe_index`), `docs/agenti/mzfeat-lr.md` (Label check).

**Done means.** (1) The check is computed once per `Index` in a run (no global state; carry it with the recipe or the index). (2) Bytes of every output unchanged: the existing goldens and `features_label_tests.rs` pass untouched; the `.wasm` leg `tests/test_label_check_wasm.py` too. (3) Before/after seconds on `tools/lr_synthetic.py --drift-series mislabelled`. The `hr` module has almost no headroom: measure with the CI size step and keep the growth under 5 % of its cap.

**Out of scope.** Changing bytes of existing outputs (a hint is a new key or a stderr line, never a changed value); new aliases.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
