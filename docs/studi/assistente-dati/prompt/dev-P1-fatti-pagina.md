# DEV P1 — Model-free facts in the page (inspection, parameter proposals, reader and chart sources)

You are a DEV chat of mzLab (repository Freeesta/mzlab). AGENTS.md is in your context. Read `docs/agenti/llm.md`, `docs/agenti/llm-subagenti.md`, `docs/agenti/llm-lettori.md`, `docs/agenti/wasm-tetti.md` and `docs/studi/assistente-dati/{11-stato.md, 03-parametri.md, 12-demo.md}`. Branch `claude/assist-p1` from `origin/main`. No real provider call, no key, no `Cargo.toml`/workflow/CSP change.

## Why
Rust already builds `ispezione/1` (`mzlab_llm::inspect::build`) and `proposta-param/1` (`mzlab_llm::params_rules::propose`), but the `.wasm` `llm` does not export them, `flow/lazy/flow-llm.js` gives no `readerInput` (every reader is `skipped` in the real panel) and `F.chartSources` is undefined (charts say "no data"). The public demo and the "Assistant not available" fallback both need these facts in the page without any model.

## Do
1. `crates/mzlab-llm-wasm`: export `inspect_build(sources_json, budget)` and `params_propose(inspection_json, answers_json, steps_json)` (same bytes as the `mzlab-llm` golden tests; add wasm tests on `tests/data/ispezione_sintetici.json`). Worker ops `inspect.build`, `params.propose` in `mzlab/web/llm/llm-worker.js`; methods in `llm-agente.js`.
2. Page sources: one function in `flow/lazy/flow-llm.js` (or a new `flow/lazy/flow-llm-facts.js` with `// @ts-check`, added to its `tools/tipi/` project) that collects, from engine routes that exist in BOTH the site and the desktop app, the inspection sources (`Item.info()`, scan stats, `metodo/1`, `features.estimate`, `campioni/2`), `tabcheck/1` (`mzlab-feat-table-wasm` in the site, `feat.rs` route in the app) and the chart sources. Wire `readerInput(role)` (`{"ispezione/1":…}`, `{"proposta-param/1":…}`, `{"tabcheck/1":…}`; null only when the page truly has none) and define `F.chartSources`. `data_summary` answers with the inspection instead of `err.llm.tool.unavailable`.
3. "Files" card shown by Rust facts alone, before any model call and without consent: type, polarity, resolution class with its basis, acquisition, isolation window, gradient, proposed roles; parameter proposals with "Apply/Discard" (one `S.undo` entry each). All texts in `lang/it.js` and `lang/en.js` (the Flow panel strings follow `flow-llm-errors.js` conventions).
4. CLI parity rule: if `mzlab-cli llm inspect RUNDIR` and `mzlab-cli llm params RUNDIR` do not exist, add them next to `llm context` (same bytes as the page for the same files; one line in `docs/agenti/cli.md` pointing to `docs/agenti/llm.md`).
5. Docs: rewrite the lines of `docs/agenti/llm.md` and `llm-lettori.md` that say these are not exported / skipped; one row change in `docs/studi/assistente-dati/11-stato.md`.

## Wasm cap
Measure `llm` after the change. If it stays under `limit_llm` 344 064 B, keep the cap and write the new measure. If it exceeds it, the new cap is measured +10 % and you declare it to the Coordinator in the PR body; if the growth is above 64 KB stop and propose a separate lazily loaded module instead.

## Acceptance (command → expected)
- `cargo test -p mzlab-llm-wasm` → new exports equal the `mzlab-llm` golden bytes.
- `cargo test -p mzlab-llm && cargo test -p mzlab-cli --lib llm_` → green.
- new `tests_e2e/e2e_llm_fatti.py` (`python3 tools/verifica.py --solo e2e_llm_fatti`) → with the synthetic files open and the assistant off: Files card and proposals visible, "Apply" undoable, zero requests outside the site; with a `replay` recording the readers are `accepted`, not `skipped`; axe 0 serious; targets ≥ 24×24 px.
- `python3 tools/controlla_i18n.py`, `python3 tools/pre_pr.py` → exit 0; `tipi` 0 new errors; `e2e_tour.py` green.
At the end one message to the Coordinator (≤ 5 lines: PR, 40-char SHA at green CI, wasm measure, what is left).
