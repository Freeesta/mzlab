# V04: first visit without the lost page, one language file

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, then `docs/studi/VELOCITA_RICARICA.md` §4 and §6. Branch `claude/primo-arrivo` from `origin/main`, after V02 (both touch `isolateOnce`) and after landing D3 #2207 / D1b #2211 are merged.

**Problem.** (a) On the first visit the page is thrown away at 0.45–0.95 s by the isolation reload (`isolateOnce` in `mzlab/web/browser.js`), before it paints: FCP 2.6 s (mirror) to 4.1 s (live). Isolation only enables the shared-memory indexing helpers (`idxpool.js`). (b) `lang/it.js` (and `lang/en.js`) `document.write`s 31 files: one more dependency level and 31 requests before the first paint.

**Do.**
1. Isolation reload only when useful: not on the first visit while nothing is open; do it at the next navigation, or when a big file is dropped and no work is in progress (as today's `idle()` check). Keep the loop guard and `?cap-off=isolamento`. Decide with the Coordinator if it changes what students see.
2. `tools/build_site.py` concatenates each language into one file in `site/static/lang/` (same globals, same order); the source tree keeps the split files for editing. `tools/controlla_i18n.py` and the language tests keep reading the sources. The desktop ships `site/static`, so it gets the same bundle (`static/manifest.json` lists it).

**Tests.** `e2e_isolamento.py`, `e2e_avvio_lingua.py`, `e2e_lingua.py`, `e2e_avvio*.py`, `e2e_tour.py` green; a build test that the bundle equals the concatenation of the sources.

**Measure.** `misura_ricarica.py` cold, before/after, unthrottled and `--profile slow`. Expected: cold FCP −0.5–1 s, −31 requests, one page instead of two.

**Done.** PR ready with both tables and the parity line; CI green; message to the Coordinator.
