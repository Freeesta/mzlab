# 06 FACETS — the resolution facets say what the steps do, and the plan declares them from the files (NL-6)

Prompt ready for a DEV chat (Opus): copy everything after the «---» line. Wave 2, parallel to 04 NEG-DEFAULTS and 05 MIX-GUARD. Terminal name: `neg-lr-facets`.

---

Package 06 FACETS of the audit `docs/studi/MATRICE_NEG_LR.md` (read §4 item 8, gap NL-6). Area files: `docs/agenti/rust-step.md`, `docs/agenti/flow-grafica.md`, `docs/agenti/rust-flow.md`.

## Goal
The ports of tp.generate, tp.link, tp.filter, tp.position, tp.network and tp.review require `resolution hr` (`crates/mzlab-feat/src/steps/tabui.rs`, `tabui_link.rs`, `hr()` in `steps/mod.rs`), yet they run and are proven at unit resolution (`flow_bentazone_tests`, `flow_lr_tests::tp_link_reads_a_unit_resolution_table`, `e2e_flow_neg_lr`): a plan that declares `facets: {resolution:
[lr]}` is refused (`err.flow.facet`, `mzlab_core::flow::check::facets`; `flow/lazy/flow-check.js` in the page). The facets are only set by hand (`flow/lazy/flow-page.js:47`), so the steps that are really HR-only (formula.ms1, formula.ms2, spectra.insilico, spectra.chimeric, rt.predict, annot.level, annot.*) run on LR files with no guard; tp.dig_lr declares lr and also runs on HR files.

## To do
1. Descriptors: the tp.* ports accept hr and lr; keep hr-only where the step needs exact masses (list each choice with its reason in the PR); tp.dig_lr stays lr.
2. The facets of a plan's scan inputs come from the files when the plan does not write them: one Rust rule (resolution and polarity from the indexes, the same rule as `flow_nature.rs`) used by `flow run` before `check::facets` and by the page before `flow-check.js` (through the `.wasm`, no JS rule). A plan written by hand keeps its facets.
3. The refusal message names the step, the port and what the files are; keys in `lang/` if the page shows it (Flow messages), English text in Tabui.
4. Owned files: `crates/mzlab-feat/src/steps/` (descriptors), `crates/mzlab-core/src/flow/check.rs`, `flow/lazy/flow-check.js`, `flow/lazy/flow-page.js`. Not yours: `flow_base.rs` (chat 05), executors of chat 04.

## Tests
- Un-ignore `gap_a_unit_resolution_plan_that_says_so_passes_the_check`; add: a plan with formula.ms1 on LR files is refused by the check, natively and in the page (e2e on the check only, no run needed).
- `cargo test --release -p mzlab-core`, `-p mzlab-feat`, `flow_rule_tests.rs` green; `e2e_flow_web_modelli` (templates still pass the check).

## Expected
- An LR plan runs every step that works at LR and is told before running which ones do not.

## PR text
Title: `Resolution facets match the steps and come from the files`. Body: starts with `VISIBLE: yes`, the table of steps and facets, the tests.

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
