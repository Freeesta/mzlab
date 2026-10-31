# 08 INCLUSION — the inclusion list follows the polarity and reads the products of tp.link (NL-10)

Prompt ready for a DEV chat (Sonnet): copy everything after the «---» line. Wave 3, parallel to 07 LR-MS2-TOL. Terminal name: `neg-lr-inclusion`.

---

Package 08 INCLUSION of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 item 9, gap NL-10). Area files: `docs/agenti/rust-flow.md`, `docs/agenti/flow-grafica.md`.

## Goal
export.inclusion (`crates/mzlab-cli/src/flow_inclusion.rs`) reads a JSON list `{formula, rt}` and defaults to `[M+H]+` whatever the files; wired on the canvas from `tp.link.tp` (a `.mzfeat`) it fails with `error.badIndex` in every combination. With `adducts` written it is right (`the_inclusion_list_takes_a_negative_adduct`).

## To do
1. The default adduct follows the polarity of the series (the shared rule of `flow_nature.rs`, or of the `tp` input's table when it has one).
2. The `tp` port reads the products of tp.link (`.mzfeat` with the tp rows: neutral formula, RT) as well as the JSON list, or the descriptor stops offering the wire (choose; the canvas must not offer a wire that fails).
3. Site = app: export.inclusion has no browser executor (`flow-live-jobs.js:280`); add it through the same Rust (`.wasm`) with a parity test, or write the technical limit in the PR.
4. Owned files: `flow_inclusion.rs`, its descriptor, the browser executor if added.

## Tests
- A native test: bentazone HR- and LR- through tp.link into export.inclusion gives `[M-H]-` lines at the m/z of the products; the browser parity case if the executor is added.

## Expected
- From a negative series the list is negative with no parameter written, in the browser and in the app.

## PR text
Title: `Inclusion list follows the polarity and reads tp.link`. Body: starts with `VISIBLE: yes`, the gap, the choice on the wire, the tests.

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
