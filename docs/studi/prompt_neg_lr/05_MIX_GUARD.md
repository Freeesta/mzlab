# 05 MIX-GUARD — a series that mixes polarities or resolutions is refused or warned (NL-4, NL-5)

Prompt ready for a DEV chat (Opus): copy everything after the «---» line. Wave 2, parallel to 04 NEG-DEFAULTS and 06 FACETS; after LR-TOL is merged, or coordinated with it through the COO (LR-TOL owns `recipe_map.rs` and the Da/ppm tolerance parameters). Terminal name: `neg-lr-mix`.

---

Package 05 MIX-GUARD of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 items 6 and 7, gaps NL-4 and NL-5). Area files: `docs/agenti/rust-flow.md`, `docs/agenti/mzfeat-lr.md`, `docs/agenti/flow-web.md`.

## Goal
`align_setup` (`crates/mzlab-cli/src/flow_base.rs:178`) rebuilds the recipe from the first file of the lineage and aligns whatever follows: two negative and three positive files make one table at +1 (tp.generate and tp.link positive), an LR file first aligns HR files in Da, all in silence. The browser aligns through the pool of `feat-worker.js` with the recipe of `flow-web-run.js` `recipeOf`, the same way.

## To do
1. Decide with the COO before coding, and write the choice in the PR: (a) refuse the alignment with an error key that names the files of each polarity / resolution, or (b) align per polarity (the polarity pairs of `crates/mzlab-feat/tests/polpair.rs` exist) and warn on mixed resolution. Default proposal: refuse mixed polarity (`err.flow.align.polarity`), refuse mixed resolution (`err.flow.align.resolution`); the user splits the series or adds two detect/align branches.
2. One Rust check on the estimates of the files (polarity, nominal axis), called by the native `align_setup` and by the browser alignment through the feat `.wasm`; the error reaches the page as `{error, error_key, params}` with keys in `lang/it.js` and `lang/en.js` (the Flow messages are i18n keys).
3. Owned files: `flow_base.rs`, `crates/mzlab-feat-wasm/src/run_meta.rs` (or where the browser builds the recipe), the align job of `flow/lazy/flow-live-jobs.js` only for the error path. Not yours: `recipe_map.rs` (LR-TOL), `flow_estimate.rs` (chat 04), descriptors and `check.rs` (chat 06).

## Tests
- Un-ignore `gap_a_series_of_mixed_polarity_is_refused_or_warned` and `gap_a_series_of_mixed_resolution_is_refused_or_warned` (they accept an error on `ali` or a warning that names the polarity / resolution: keep that contract or tighten it to your choice).
- An e2e case in the browser: the same mixed series gives the same error key in the page as in `flow run`.

## Expected
- No table silently mixes polarities or resolutions, in the browser or in the app.

## PR text
Title: `Refuse alignments that mix polarities or resolutions`. Body: starts with `VISIBLE: yes`, the choice and why, the keys, the tests.

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
