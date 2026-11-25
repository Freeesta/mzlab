Read docs/coda/BRIEF.md.

# RAW-MS2: raw ms2 (AU-15)

**Goal.** `raw ms2 list|spectrum|xic` as in `docs/studies/RAW_ACCESS_STUDY_2.md` §8: precursors fragmented, mean product spectra per precursor channel, product XIC per precursor.

**Why.** `trace xic` at level 2 sums every precursor.

**Read first.** `docs/studies/RAW_ACCESS_STUDY_2.md` §8, `docs/agenti/raw-access.md`, `docs/agenti/data-channels.md` (`channels/1`), `crates/mzlab-cli/src/items_cmd.rs` (`ms2_exps`), the merged MS2-SUM.

**Already on main (checked 25 Nov, re-check before coding).** `channels/1` and `items/1` list precursors; `spectrum get --sum` (row MS2-SUM) gives the summed spectrum. Do not duplicate it: `raw ms2 spectrum` calls the same code.

**Done means.** Schema `raw-ms2/1`; the neutral-loss list shared with `raw companions`; exit 5 with the nearest three precursors; parity CLI=app=wasm; synthetic file with two precursors. New site code goes in a new lazy module (pattern: `crates/mzlab-rawxic-wasm`, `tools/moduli/rawxic.toml`), never growing `raw` or `rotte` by more than 5 %.

**Out of scope.** `raw batch`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
