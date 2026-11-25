Read docs/coda/BRIEF.md.

# RAW-SCAN: raw scan (AU-16)

**Goal.** `mzlab-cli raw scan FILE --rt A,B [--side MIN] [--gap MIN] [--bin nominal|0.1] [--mz-range LO,HI] [--top N] [--channel ID]` as specified in `docs/studies/RAW_ACCESS_STUDY_2.md` §4: mean spectrum of the window minus the mean of the two sides, per bin, with ratio, net counts and verdict.

**Why.** 'What is in the spectrum here that is not in the background?' is asked in every campaign.

**Read first.** `docs/studies/RAW_ACCESS_STUDY_2.md` §C and §4, `docs/agenti/raw-access.md`, `crates/mzlab-core/src/rawxic.rs`, `ionstat.rs`, the binner of NOMINAL-BIN if merged. Start with `grep -n 'raw-access\|raw xic' docs/agenti/INDICE.md`, then `grep -n 'pub fn' crates/mzlab-core/src/rawxic.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `raw info|cache|xic` and `ionstat` (merged) give the counts and verdict rules to reuse.

**Done means.** Schema `raw-scan/1`, the common conventions of §C (exit codes, `--limit`, `--dry-run`, `--full`), `warn.raw.oneSide`; parity CLI=app=wasm; synthetic test. New site code goes in a new lazy module (pattern: `crates/mzlab-rawxic-wasm`, `tools/moduli/rawxic.toml`), never growing `raw` or `rotte` by more than 5 %.

**Out of scope.** `raw companions`, `--figure`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
