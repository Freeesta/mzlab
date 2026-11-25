Read docs/coda/BRIEF.md.

# RAW-COMPANIONS: raw companions (AU-16)

**Goal.** `mzlab-cli raw companions FILE... --mz M --rt A,B ...` as in `docs/studies/RAW_ACCESS_STUDY_2.md` §5: which masses rise and fall with M, with the mass-difference table.

**Why.** Adducts, in-source losses and dimers are separated from real products.

**Read first.** `docs/studies/RAW_ACCESS_STUDY_2.md` §5, `docs/agenti/ion-groups.md` (`features.companions`, AU-10: the mass-difference table is shared), `docs/agenti/raw-access.md`.

**Already on main (checked 25 Nov, re-check before coding).** `features.companions` and its table are merged; `raw xic` gives traces and counts.

**Done means.** Schema `raw-companions/1`; the user list `liste/1` for the table shared with AU-10; `relation_set` never forced to one; parity CLI=app=wasm; synthetic test with +22 and +17 companions and an unrelated +1 ion. New site code goes in a new lazy module (pattern: `crates/mzlab-rawxic-wasm`, `tools/moduli/rawxic.toml`), never growing `raw` or `rotte` by more than 5 %.

**Out of scope.** `raw isotopes`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
