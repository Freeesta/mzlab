# 03 NEG-MGF — the MGF of table.match follows the polarity of the files (NL-3)

Prompt ready for a DEV chat (Sonnet is enough): copy everything after the «---» line. Wave 1, parallel to 01 ORDER and 02 NEG-WEB. Terminal name: `neg-lr-mgf`.

---

Package 03 NEG-MGF of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 item 4, gap NL-3). Area files: `docs/agenti/rust-flow.md`, `docs/agenti/flow-web.md`.

## Goal
table.match writes the MGF of the table rows with `pol::recipe_polarity` (`crates/mzlab-feat-table-wasm/src/rows_mgf.rs:36`), which is +1 when the plan does not write the polarity; the library search filters by the query's polarity, so a negative table is searched as positive. `pol::of_table` (the recipe, else the sign of the files' polarities) is the rule the exports already use. The same Rust serves the browser (table `.wasm`) and `flow run`, so one fix closes both sides.

## To do
1. `rows_mgf.rs` reads the polarity with `pol::of_table`.
2. `grep -rn recipe_polarity crates`: every other writer of spectra for a library search (spectra.match, spectra.ms2, ms2 exports) that falls back to +1 instead of the files gets the same rule; list them in the PR. Leave `recipe_polarity` where the recipe is the right answer and say why.
3. Owned files: `crates/mzlab-feat-table-wasm/src/rows_mgf.rs` and the call sites found in step 2. Not yours: `core.rs` and `pol.rs::samples_v1` (chat 01), `flow/lazy/flow-live*.js` (chat 02).

## Tests
- Un-ignore `gap_the_polarity_of_the_mgf_of_table_match_is_the_one_of_the_files`.
- `flow_tablematch_tests`/`flow_match_web_tests`: a negative table finds a negative library spectrum and not the positive one, in `flow run` and with the bytes of the table `.wasm` entry the browser calls.

## Expected
- A negative series is searched as negative with the plan's defaults, the same in the browser and in `flow run`.

## PR text
Title: `table.match reads the polarity of the files`. Body: starts with `VISIBLE: yes`, the gap, the call sites, the tests.

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
