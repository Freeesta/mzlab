# 01 ORDER — the samples are matched to the files of the table by name (NL-8)

Prompt ready for a DEV chat (Opus): copy everything after the «---» line. Wave 1, parallel to 02 NEG-WEB and 03 NEG-ION. Terminal name: `neg-lr-order`.

---

Package 01 ORDER of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 item 1, gap NL-8). Area files: `docs/agenti/flow-web.md`, `docs/agenti/rust-flow.md`, `docs/agenti/dati.md` (the table section).

## Goal
A samples file (`campioni/2` or `/3`) is paired with the files of a `.mzfeat` by position only: `mzlab_feat_table_wasm::core::load` checks the count (`core.rs:21`) and nothing else, and the other readers of the roles (`pol::samples_v1`, `mzlab_feat::tp::TpRoles::from_json`, `flow_stats.rs`, `flow_quant.rs`, `massacc`) do the same.
The browser runner lists the OPFS indexes in name order (`flow/lazy/flow-web-run.js:38`), the samples grid keeps the order of the files open in Data: with names such as T2p5 and T10p25 every CSV/mzTab column carries another file's name and tp.link reads the wrong blank and t0, in silence. The samples must follow the table's files by name, on both sides.

## To do
1. One Rust function (suggested: `mzlab_core::campioni::order_like(text, names) -> Result<String>`) that rewrites a samples document in the order of `names` (the table's files, matched by file name with or without extension); a name missing from the samples, or a sample naming no file, is the error `error.badCampioni` with the name in `params` (never a silent pad).
2. Every place that pairs a samples document with a table calls it with the table's file names: natively the executors already hold them (`flow_export.rs` passes `files`; tp.link, tp.network, tp.dig_lr, table.filter, table.stats, quant.calibrate, qc.mass_accuracy: find them with `grep -rn "samples::from_json\|samples_v1\|TpRoles::from_json\|campioni::from_json" crates`);
   in the browser the names are `x.items` (`flow-live.js`), passed to the same Rust through the table `.wasm` and the feat `.wasm` (no JS reordering). Choose whether the `.mzfeat` should also store the file names; if you do, it is a format change: say so in the PR and keep reading older tables.
3. `tests_e2e/e2e_flow_neg_lr.py`: change ONLY the line `ORDER = sorted(NAMES)` to `ORDER = NAMES` (the injection order) and its comment; chat 02 owns the `plan()` lines of the same file.

## Tests
- Un-ignore `gap_the_samples_are_matched_to_the_files_of_the_table_by_name`; add one case where the samples name a file the table lacks (the error).
- A browser test: `e2e_flow_neg_lr.py` with `ORDER = NAMES` green in the four cases.

## Expected
- The same table gives the same CSV, mzTab, filter, tp.filter and tp.position whatever the order of the samples file and of the indexes in OPFS.

## PR text
Title: `Match the samples to the table's files by name`. Body: starts with `VISIBLE: yes` (column labels change for users whose files were not in name order), the gap, the function, the call sites, the tests.

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
