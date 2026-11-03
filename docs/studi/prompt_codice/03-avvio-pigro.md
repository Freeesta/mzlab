# 03 — Start-up JS: `riferimento.js` and `libreria.js` load on first use

Prompt ready for a DEV chat: copy everything after the «---» line. Wave 1 (with 01 and 02). You own `mzlab/web/carica.js` for this wave.

---

**Role.** DEV chat for mzLab (public repo Freeesta/mzlab). English for code, commits, PR and messages. Read CLAUDE.md (in context), `docs/agenti/dati.md` (start-up list `L`, `carica()`, globals) and `docs/studi/OTTIMIZZAZIONE_CODICE.md` §3. Branch `claude/lazy-ref-lib` from `origin/main`.

**Goal.** The start-up list `L` of `mzlab/web/carica.js:6` weighs 1 871 402 B of `MAX_TOTAL` 1 900 000 (`tests/test_peso_avvio.py:8`): 28 KB of headroom. Two scripts are only needed after a click; move their bodies behind the existing lazy loader `carica(name)` and keep a small stub at start-up. Expected: −62…64 KB of start-up, no visible change, `MAX_TOTAL` untouched (never raise it; you may propose a lower cap in the PR text).

**Pattern to copy (already in the codebase).** `riferimento.js:222-227`: `STAT.add({ id: "kmd", …, mount: box => { carica("kmd-vista.js").then(() => carica("kmd.js")).then(() => KMD.mount(box)) } })`. Commit #2479 («Load the KMD and PFAS sections on first use») did the same for KMD/PFAS: read its diff first (`git show 995ff55 --stat`).

**Files you own.** `mzlab/web/carica.js`, `mzlab/web/riferimento.js`, `mzlab/web/libreria.js`, new `mzlab/web/riferimento-corpo.js` and `mzlab/web/libreria-corpo.js` (names free if a better one fits the codebase), their `tools/tipi/` project entries and `globali*.d.ts` lines (regenerate with the tool, never by hand), `docs/agenti/dati.md` lines on these two files. Do not touch `settings.js`, `ident-pannello.js`, `libreria-finestra.js`, `pca.js`, `kmd*.js`, `pfas*.js` except if a call breaks (then say why in the PR).

**Work.**
1. `riferimento.js` (19 983 B). Stub at start-up = the four `STAT.add(...)` of `:221-227`; the `reference` section's `mount` becomes `box => carica("riferimento-corpo.js").then(() => RIFERIMENTO.mount(box))`. The body (everything that computes and draws the «Difference from the reference», the `tabella`/`campioni` listeners of `:218-220`, which act only on a mounted box) moves to the body file, which defines `window.RIFERIMENTO` with the same members (`mount, run, inspect, tsv, params, result`). `e2e_differenza_hr.py:64-158` reads `RIFERIMENTO.result` after the section is opened: it must pass unchanged.
2. `libreria.js` (49 555 B). Stub at start-up:
   - the three `AZIONI.registra` of `:416-430`, unchanged in id, group, order, views, labels and visibility;
   - the synchronous helpers those need (`isHr` `:382` and what it calls: `queryOf`, `profOf`), moved, not copied: the body imports them from the stub (one copy only);
   - `window.LIB` with the SAME member names as `:431` (`open, addFiles, queryOf, profOf, searchFrom, whatIs, isHr, mirror, cosVis, call, identifyAll, groupRows, groupAssess, assess, scansOf, engine, ensureCss`). Members that need the body are async forwarders: `async (...a) => (await body()).open(...a)` with `body = () => carica("libreria-corpo.js").then(() => window.LIBCORE)`. Check each caller for a member used synchronously (e.g. `settings.js:206` `if (window.LIB) LIB.open()`, `ident-pannello.js:42-43`, `libreria-finestra.js:10`, `e2e_libreria*.py` `LIB.engine`, `LIB.call`): a sync member stays sync (stub or a getter that loads first and is awaited by its caller). Whatever you choose, no test is changed to wait less or check less.
   - The worker supervisor and the dialog code go to the body; the worker is still created on first use, never at start-up.
3. Remove both bodies from `L`, keep both stubs in `L` in the same position (order of classic scripts matters: CLAUDE.md). A body loaded by `carica()` must not redeclare a global of another script.
4. Measure with `python3 -c "import sys; sys.path.insert(0,'tests'); from test_peso_avvio import *; print(sum((WEB/n).stat().st_size for n in startup_files()))"` before/after; put both numbers in the PR. Also the first-paint e2e (`e2e_primo_disegno.py`) and the `?perf` start-up numbers if available.
5. Parity site/app: the desktop app loads the same page, so it gets the same change; run the desktop smoke e2e that CI runs if it touches the library window.

**Tests.** `python3 -m pytest tests/test_peso_avvio.py`, `tools/controlla_i18n.py`, `tsc` via `tools/pre_pr.py` (0 new errors; `base_errori.json` only goes down), e2e: `e2e_differenza_hr.py`, `e2e_libreria.py`, `e2e_libreria_scarica.py`, `e2e_inglese_hr.py`, `e2e_tour.py`, `e2e_primo_disegno.py`, `e2e_axe_grafici.py`, and `python3 tools/verifica.py --cambiati` (needs a browser: run it if the container has Chromium, which it does). Front end not proven = broken: open the page with Playwright, open Statistics → Difference from the reference, and the spectrum menu «Search the libraries» on an MS2 spectrum; both must work at first click.

**PR.** Title «Load the reference-difference and library code on first use». Body starts `VISIBLE: no` (same screens, same behaviour), then the start-up bytes before/after and the e2e list.

**Delivery.** At green CI ONE message to the COO (≤5 lines: PR, 40-char SHA, start-up bytes before → after). PR events: skill `steward`.
