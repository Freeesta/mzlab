Read docs/coda/BRIEF.md.

# RAW-PROFILE: raw profile (AU-17)

**Goal.** `mzlab-cli raw profile FILE... --mz M --rt A,B --samples PATH [--quantity area|height]` as in `docs/studies/RAW_ACCESS_STUDY_2.md` §6: time course per series with the shape class and its rule.

**Why.** Carryover in washes and formation curves are read in one call.

**Read first.** `docs/studies/RAW_ACCESS_STUDY_2.md` §6, `docs/agenti/raw-access.md`, `docs/agenti/samples.md`, `crates/mzlab-core/src/rawxic.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `raw xic` rows and `samples/1` (merged); shape classes absent, sporadic, rise, decay, rise_fall, flat, irregular are specified, not built.

**Done means.** Schema `raw-profile/1`; the thresholds beside the class (`shape_rule`); reuse `raw xic` rows, do not recompute; parity CLI=app=wasm; synthetic series per class. New site code goes in a new lazy module (pattern: `crates/mzlab-rawxic-wasm`, `tools/moduli/rawxic.toml`), never growing `raw` or `rotte` by more than 5 %.

**Out of scope.** `--figure`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
