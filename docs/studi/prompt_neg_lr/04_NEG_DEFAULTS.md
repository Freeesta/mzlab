# 04 NEG-DEFAULTS — features.estimate and formula.ms1 follow the polarity and the resolution of the files (NL-1, NL-2)

Prompt ready for a DEV chat (Opus): copy everything after the «---» line. Wave 2, after 02 NEG-WEB is merged (it puts the rule «polarity and unit mass of a series» where the browser reaches it); parallel to 05 MIX-GUARD and 06 FACETS. Terminal name: `neg-lr-defaults`.

---

Package 04 NEG-DEFAULTS of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 items 3 and 5, gaps NL-2 and NL-1). Area files: `docs/agenti/rust-flow.md`, `docs/agenti/mzfeat-lr.md`, `docs/agenti/flow-web.md`.

## Goal
1. features.estimate applies its descriptor defaults (`polarity` 1, axis ppm; `crates/mzlab-cli/src/flow_estimate.rs`, `estimate_job`): on negative files noise and n are null; on LR files it reports axis ppm and an HR `min_height` of 2e5 (with axis Da written: 0.1 Da, still 2e5).
2. formula.ms1 takes the sign only from its `ion` parameter (`crates/mzlab-feat/src/flow_formula_ms1.rs:95`, default `[M+H]+`): on a negative series every row ends in `err.flow.formula.scan`.

## To do
1. Both steps take what the plan does not write from the files, with the shared rule of 02 (`flow_nature.rs` or where 02 moved it): estimate measures in the files' polarity and on the axis and LR preset of the files (`recipe_lr.rs`, `axis.rs`); formula.ms1's default ion is `[M+H]+` or `[M-H]-` by the files. A written value still wins; a written `ion` of the other polarity than the files is a warning in prov, not an error.
2. Site = app: formula.ms1 in the browser (`flow/lazy/flow-web-ident.js`, formula `.wasm`) gets the same default. features.estimate has no browser executor today (`flow-live-jobs.js:280`): do not add one here; write it in the PR and leave §3 of the matrix saying so.
3. Owned files: `crates/mzlab-cli/src/flow_estimate.rs`, `crates/mzlab-feat/src/flow_formula_ms1.rs`, the formula.ms1 part of `flow/lazy/flow-web-ident.js`. Not yours: `flow_base.rs`, `run_meta.rs` (chat 05), the step descriptors and `check.rs` (chat 06).

## Tests
- Un-ignore `gap_the_estimate_follows_the_polarity_and_the_resolution_of_the_files` and `gap_formula_ms1_takes_the_ion_of_the_polarity_of_the_files`.
- `tests_e2e/e2e_flow_web_ident2.py` (formula.ms1 browser = `flow run`) with a negative case.

## Expected
- The default plan measures and attributes a negative or LR series like a positive HR one.

## PR text
Title: `Estimate and formula.ms1 follow the files' polarity and resolution`. Body: starts with `VISIBLE: yes`, the two gaps, the tests, the browser half of features.estimate as a known gap.

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
