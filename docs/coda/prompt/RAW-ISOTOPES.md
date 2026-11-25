Read docs/coda/BRIEF.md.

# RAW-ISOTOPES: raw isotopes (AU-18)

**Goal.** Implement `raw isotopes` as in `docs/studies/RAW_ACCESS_STUDY_2.md` §7: net counts of M, M+1, M+2 over the peak region, observed ratios with delta-method sigma, expected pattern from `ion pattern` with z scores, carbon range and interference flag.

**Why.** An agent needs a counting-error verdict on a formula proposal without writing a script.

**Read first.** `docs/studies/RAW_ACCESS_STUDY_2.md` §C and §7, `docs/agenti/raw-access.md`, `crates/mzlab-core/src/rawxic.rs`, `crates/mzlab-core/src/ionpattern.rs`.

**Already on main (checked 25 Nov, re-check before coding).** `raw info|cache|xic` merged. `raw scan` (row RAW-SCAN) gives the binned view this command wants.

**Done means.** Per the study: schema `raw-isotopes/1`, exit codes of §C, `--dry-run`, `--json`, catalogs; a new lazy `.wasm` module or an existing one with room (never `rotte`, never `raw` over 5 %); parity CLI=app=wasm; synthetic test with known ratios.

**Out of scope.** `--figure`; mislabelled isotopes in a Q1 scan (row ISOTOPES-UNIT).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
