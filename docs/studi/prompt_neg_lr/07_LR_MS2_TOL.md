# 07 LR-MS2-TOL — unit-resolution tolerances for MS2, QC and duplicates (NL-11, NL-12, NL-13)

Prompt ready for a DEV chat (Sonnet or Opus): copy everything after the «---» line. Wave 3, parallel to 08 INCLUSION; after LR-TOL and 06 FACETS are merged. Terminal name: `neg-lr-ms2tol`.

---

Package 07 LR-MS2-TOL of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 items 10-12). Area files: `docs/agenti/mzfeat-lr.md`, `docs/agenti/qc-massa.md`, `docs/agenti/rust-flow.md`.

## Goal
1. spectra.match and table.match default `tol_da` 0.02 (`grep -rn tol_da crates/mzlab-feat/src/steps`), too narrow for unit-resolution MS2.
2. qc.mass_accuracy measures ppm errors on LR files (`crates/mzlab-feat/src/massacc/`, `flow_massacc.rs`, `flow/lazy/flow-web-qc.js`) instead of refusing or reporting in Da.
3. table.filter `dup_ppm` has only a ppm form (`crates/mzlab-feat/src/filterspec.rs:106`): at LR a 10 ppm window is far below the 0.1 Da axis.
4. prov records the features.detect preset as «hr» on LR files (cosmetic).

## To do
1. When the plan does not write them, the MS2 tolerances follow the files' resolution (an LR preset in Da; propose the values in the PR with their source, e.g. the 3200 QTRAP unit resolution 0.7 FWHM); qc.mass_accuracy at LR either refuses with a key or reports in Da (choose, say why); `dup_da` beside `dup_ppm`, chosen by the table's axis; the preset label says lr.
2. Same Rust on both sides: the browser executors (`flow-web-ident.js`, `flow-web-qc.js`, table.filter in `flow-live-jobs.js`) pass nothing new in JS beyond the parameter.
3. Owned files: the descriptors and executors named above. Not yours: `recipe_map.rs` and the align/gapfill/ms2_link tolerances (LR-TOL).

## Tests
- New native tests on the bentazone LR fixture (one per point) and a browser parity case in `e2e_flow_neg_lr.py` for table.filter with `dup_da` (add a case, do not change the existing ones).
- `flow_match_tests`, `flow_massacc_tests`, `flow_tablematch_tests` green.

## Expected
- Every default of an LR plan is in Da and meaningful at unit resolution, the same in the browser and in the app.

## PR text
Title: `Unit-resolution defaults for MS2 matching, QC and duplicates`. Body: starts with `VISIBLE: yes`, the values and their source, the tests.

## Common rules
- Start from `origin/main` once the audit PR (branch `claude/neg-lr-audit`) is merged: the matrix and the `gap_*` tests live there. If `crates/mzlab-cli/src/flow_neglr_tests.rs` is missing from `origin/main`, ask the COO before starting.
- You are a DEV chat of mzLab (repository `Freeesta/mzlab`); AGENTS.md is in your context: follow it. English only. Coordinator (COO) = `session_01NawSmneBkv8sosEHpoC8k4`: at the end ONE `send_message` of at most 5 lines (PR, 40-character SHA at green CI, what is left). Blocked or a choice between two behaviours: PR open with the explanation, ask the COO, no merge.
- Read only `docs/studi/MATRICE_NEG_LR.md` (the gap named above) and the area files named here; find code with `python3 tools/indice.py` then `grep -n`, or the `cercatore` subagent.
- Site = app (AGENTS.md §1): every behaviour you change exists in the browser and in `flow run` / the desktop app, with a parity test; a half is accepted only for a real technical limit, written in the PR.
- Do not touch: `.github/workflows/`, `mzlab/web/teoria/`, the `.wasm` caps (they never rise: if a `.wasm` grows past its cap, PR open and ask), the files owned by the other chats of your wave (listed in `docs/studi/prompt_neg_lr/README.md`).
- Rust for every computation; no new Python function; JS only for the interface. Tabui texts in English with no `lang/` key and never the word «formula» alone; every other page text is a key in `lang/it.js` and `lang/en.js`.
- No laboratory data in the public repository, no surname of `nomi_vietati.txt`; synthetic fixtures only (the bentazone series of `crates/mzlab-cli/src/flow_bentazone_tests.rs`).
- Remove the `#[ignore]` of your `gap_*` test in `crates/mzlab-cli/src/flow_neglr_tests.rs` (keep its name and assertions; tighten them if you can, never loosen them) and rewrite its row in §3 and its item in §4 of `docs/studi/MATRICE_NEG_LR.md` to what is true; one line in the area file for each new behaviour; docs describe the current state only.
- Proofs: `cargo test --release -p <crate>` of the crates you touch, `cargo test --release -p mzlab-cli --lib flow_neglr`, `python3 tests_e2e/e2e_flow_neg_lr.py` (needs `wasm-bindgen`), `python3 tools/pre_pr.py` before pushing.
- Flow: branch `claude/<short-name>` from `origin/main`, PR ready for review, never merged from the chat; PR events with the skill `steward`. If the context passes 200k tokens at the end of the PR: hand-over block to the COO (AGENTS.md §2) and stop.
