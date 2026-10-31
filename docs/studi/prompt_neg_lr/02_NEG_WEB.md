# 02 NEG-WEB — the browser reads the polarity and the unit mass of the files (NL-7)

Prompt ready for a DEV chat (Opus): copy everything after the «---» line. Wave 1, parallel to 01 ORDER and 03 NEG-ION. Terminal name: `neg-lr-web`.

---

Package 02 NEG-WEB of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 item 2, gap NL-7). Area files: `docs/agenti/flow-web.md`, `docs/agenti/flow-live.md`, `docs/agenti/rust-flow.md`.

## Goal
`flow run` takes the polarity and the unit mass of a series from its indexes when the plan does not write them (`crates/mzlab-cli/src/flow_nature.rs`). The browser runner does not: tp.link gets `polarity:
NaN` (`flow/lazy/flow-live-jobs.js:272-273`) and tp.generate keeps HR masses on LR files, unless the page wrote them from the files open in Data (`inheritNature`, `flow/lazy/flow-samples.js`). An imported plan, a plan built by hand or run with the Data tab empty runs negative files as positive (`err.flow.link.parent` mz 241.064 in HR- and LR-; LR+ tp.generate tries 145 hypotheses instead of 92).

## To do
1. The browser runner decides the polarity and the unit mass of the series with the SAME Rust rule as `flow_nature.rs` (move the rule into a crate both reach if it lives only in `mzlab-cli`; the browser reads the `.mzidx` of OPFS through the feat `.wasm`). Apply it to every step that reads them in the browser: tp.generate, tp.link, tp.dig_lr, and any other `F.liveJobs` entry whose native executor calls `flow_nature` (`grep -n "flow_nature\|nature::" crates/mzlab-cli/src`).
2. What the plan writes still wins; prov/the run log record the same value as `flow run`.
3. `tests_e2e/e2e_flow_neg_lr.py`: you own the `plan()` function and the `UNWRITTEN` lines: the default becomes the plan WITHOUT polarity and unit mass (both sides decide them), and `NEG_LR_WRITTEN=1` keeps the written variant; chat 01 changes only the `ORDER` line.

## Tests
- `python3 tests_e2e/e2e_flow_neg_lr.py` green in the four cases with nothing written in the plan; also `e2e_flow_dig_lr`, `e2e_flow_tp_web`, `e2e_flow_tp_link`.
- A Rust test of the shared rule (negative HR, positive LR, mixed series as `flow_nature.rs` answers it today).

## Expected
- The same plan, written or not, gives the same bytes in the browser and in `flow run` for HR+, HR-, LR+ and LR-.

## PR text
Title: `Browser Flow reads polarity and unit mass from the indexes`. Body: starts with `VISIBLE: yes`, the gap, where the rule lives, the e2e result per case.

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
