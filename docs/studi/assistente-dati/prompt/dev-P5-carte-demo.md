# DEV P5 — Demo cards, session recording, "Play a recorded demo", demo e2e

You are a DEV chat of mzLab (repository Freeesta/mzlab). AGENTS.md is in your context. Read `docs/agenti/llm.md` (§Pannello nel Flow), `docs/agenti/llm-lettori.md`, `docs/agenti/flow-grafica.md`, `docs/agenti/accessibilita.md`, `docs/studi/assistente-dati/{12-demo.md, 01-architettura.md §8–9}` and the A8 section of `08-prompt-dev.md` (this PR delivers the cards the demo needs). Branch `claude/assist-p5` from `origin/main`. Prerequisites: P1 and P4 merged. No real provider call, no key, no CSP change.

## Do
1. New `flow/lazy/flow-llm-cards.js` (`// @ts-check`, in its `tools/tipi/` project): closed question card for `ask_user` (option buttons + "Other" with a text field; answer returned as the `tool_result`), reading card (results with confidence and a clickable proof that opens the view or step), parameter card (value, `source`, reason, "Apply/Discard", one `S.undo` entry). Cards live in an `aria-live="polite"` region; focus moves to the first option of a new question only if focus was in the panel; everything by keyboard; targets ≥ 24×24 px (40 on touch); colours only from CSS variables.
2. Recording (owner mode only): "Record this session" writes the session as `llmreg/1` lines (requests' SHA-256 and events, as `fake::Replay::record_line`) and offers it as a download; no user text beyond the chips, no file names (aliases).
3. "Play a recorded demo": lists `static/llm/demo/*.jsonl` (copied by `tools/build_site.py`; ship one synthetic recording made from `tests/data/assist/reg/fs_pos.jsonl` until Federico's reviewed recordings arrive) and plays it in strict mode through `window.MZLAB_LLM_AGENT.replay`; available also when the proxy is down.
4. Texts only in `lang/` (Flow panel conventions); `tour.js` and `tests_e2e/e2e_tour.py` updated if a target changes.

## Acceptance (command → expected)
- `python3 tools/verifica.py --solo e2e_llm_demo` (new `tests_e2e/e2e_llm_demo.py`): the shipped recording plays in strict mode on the synthetic `fs_pos` set: Files card, reading card, 2–3 questions answered by buttons, parameter cards with Apply/Undo, plan card, chart with its data table; a recording that exceeds the 0,50 $ client cap stops at the cap; proxy unreachable → "Assistant not available" then "Play a recorded demo" works; zero hosts outside the site; axe 0 serious.
- `python3 tools/verifica.py --solo e2e_llm,e2e_llm_lettori,e2e_tour` → green.
- `python3 tools/controlla_i18n.py`, `python3 tools/pre_pr.py` → exit 0; `tipi` 0 new errors; `.wasm` caps unchanged.
At the end one message to the Coordinator (≤ 5 lines).
