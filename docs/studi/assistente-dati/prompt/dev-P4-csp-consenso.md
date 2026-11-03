# DEV P4 — Transport CSP, per-visit consent, owner passphrase, "Assistant not available"

You are a DEV chat of mzLab (repository Freeesta/mzlab). AGENTS.md is in your context. Read `docs/agenti/llm.md` (§Profilo, impostazioni, consenso; §Trasporto nel sito), `docs/agenti/lingue.md`, `docs/agenti/accessibilita.md`, `docs/studi/assistente-dati/{09-worker.md §3 §7 §10, 14-decision-card.md D4}`. Branch `claude/assist-p4` from `origin/main`. Prerequisites: P3 merged; the proxy host given by Federico through the Coordinator (never invent one). The CSP change needs the Coordinator's explicit go-ahead in your launch message.

## Do
1. `mzlab/web/llm/trasporto.html`: `connect-src 'self' http://localhost:* http://127.0.0.1:* https://<proxy host>`; nothing else (no `api.anthropic.com`: no personal keys in the site). `tests/test_llm_csp.py` lists the new allowed source.
2. Consent: new class `inspection` in `llm-consenso.js`; for the provider `mzlab-demo` the consent is stored in `sessionStorage` (one visit), not `localStorage`; the dialog text is D4 of the decision card (keys `llm.consenso.demo.*` in `lang/it.js` and `lang/en.js`), with "What will be sent" showing the exact JSON. Refusing keeps all model-free parts.
3. Owner passphrase: a field shown only when the proxy answers 403 `err.llm.proxy.closed` or in admin mode; stored only in the transport iframe IndexedDB (`mzlab_llm`), sent as `x-mzlab-owner`; "Forget" deletes it. Never in the main page, logs, `prov/1` or exports (`tests/test_llm_chiavi.py` extended).
4. Errors: `err.llm.proxy.{closed,off,budget,body,origin}` and `err.llm.limit.spend` with `limit` param, in the catalogs and in `flow/lazy/flow-llm-errors.js` (`tests/test_llm_errori.py`). Budget reached, proxy off or unreachable → the card says "Assistant not available" once, switches to rules only, no retry loop.
5. Spend counter in the panel from `x-mzlab-spend` (spent / cap in $, two decimals), one notice at `x-mzlab-soft`.
6. Docs: rewrite the «Trasporto nel sito» lines of `docs/agenti/llm.md`.

## Acceptance (command → expected)
- `pytest tests/test_llm_csp.py tests/test_llm_chiavi.py tests/test_llm_errori.py` → green.
- `python3 tools/verifica.py --solo e2e_llm_consenso` with a fake proxy on `localhost` (served by the test, adapter `memoria.js` of P2 behind a tiny server): consent per visit, 403 → passphrase field, 503 → "Assistant not available" and rules only, no host outside the site and the fake proxy, axe 0 serious, targets ≥ 24×24 px.
- `python3 tools/controlla_i18n.py`, `python3 tools/pre_pr.py` → exit 0; `tipi` 0 new errors.
At the end one message to the Coordinator (≤ 5 lines).
