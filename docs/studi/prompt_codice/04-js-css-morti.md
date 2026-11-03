# 04 — Dead JS and CSS of the Dati page

Prompt ready for a DEV chat: copy everything after the «---» line. Wave 2 (with 05), after 03 is merged (03 owns `carica.js`; this prompt does not touch it, but both move start-up bytes: measure on the merged main).

---

**Role.** DEV chat for mzLab (public repo Freeesta/mzlab). English for code, commits, PR and messages. Read CLAUDE.md (in context) and `docs/studi/OTTIMIZZAZIONE_CODICE.md` §1.3. Branch `claude/js-css-dead` from `origin/main`.

**Goal.** Delete JS functions and CSS rules nothing reaches, with no visible change.

**Files you own.** `mzlab/web/apertura-area.js`, `mzlab/web/storia.js`, `mzlab/web/app.css`, the generated `tools/tipi/globali*.d.ts` (regenerate with the tool listed in `tools/fondi_main.py` `GENERATI`, never by hand), and the `docs/agenti/` line that describes `apertura-area.js` if it names the removed functions.

**Evidence and work (re-check each with `rg` over `mzlab flow TP_Mine tests tests_e2e tools crates/mzlab-desktop` before deleting).**
1. `apertura-area.js`: `openingStato` (:22), `openingFile` (:23), `openingFiles` (:24), `openingStart` (:25-28), `openingEnd` (:29), `openingCancel` (:31-33) are called by nobody: the opening skeleton is driven by `avvio-progressivo.js:186-198` and `area-lavoro.js:71`. Delete them, then what only they used: `openMark` (:9), `OPEN.t`, `window.MZLAB_OPEN` (:10; only `tools/tipi/globali.d.ts:84` declares it). Keep `OPEN`, `OPEN_ST`, `openingRender`, `openingView` (`shell.js:34`). Rewrite the module header (2-4 lines) to say what the file does now (draws the skeleton `#opening` for `applyView`), no history.
2. `storia.js:105-109`: `annullaVoce` and its doc comment (inner function, not in the returned object at :156).
3. `app.css`: the rules whose every selector names an element or class no file emits: `#resetnote` (and children), `.pnl .ctl select.ttlsel` (+ `:hover`), `.nlc` (+ `ul`), `.nlhint`, `.nlpp`, `.ldtab .cvst` (+ `>span`, `>span+span`), `.ldtab tr.cvr[data-st=fail] [data-cvtxt]`, `.cardhd` (+ `h3`), `#helppop .hp-s`, `.cp-total`. For each, search the bare name in JS, HTML, Rust strings and the `lang/` files (a class can be built as `"x-" + k`); delete only when nothing can emit it. Keep `.lv-*` (built as `"lv-" + k`, `flow/lazy/flow-run-end.js:28`). Several e2e already assert those elements are absent (`e2e_reset.py:50`, `e2e_perdite.py:73`, `e2e_addotti_default.py:19`, `e2e_select_freccia.py:16`): leave the tests as they are.
4. Do not delete the e2e hooks reached only by tests (`addTrace`, `ugSet`, `ibkPick`, `uipContOn`, `undoZoom`).

**Measure.** Start-up bytes before/after (`tests/test_peso_avvio.py` sum) and `app.css` bytes; put them in the PR.

**Tests.** `python3 tools/pre_pr.py` (tsc, i18n, globals), e2e: `e2e_apri_subito.py`, `e2e_progressivo.py`, `e2e_primo_disegno.py`, `e2e_reset.py`, `e2e_perdite.py`, `e2e_select_freccia.py`, `e2e_tour.py`, `e2e_annulla_tabelle.py`, plus `python3 tools/verifica.py --cambiati`. Open the page with Playwright, load an example file and check the opening skeleton appears and fades as before.

**PR.** Title «Remove dead opening helpers, an unused undo function and unused CSS». Body starts `VISIBLE: no`, then the evidence (one line per item) and the byte numbers.

**Delivery.** At green CI ONE message to the COO (≤5 lines: PR, 40-char SHA, bytes saved). PR events: skill `steward`.
