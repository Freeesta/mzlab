# V03: Tabui loads after the page is usable

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, then `docs/studi/VELOCITA_RICARICA.md` §4 and §6. Branch `claude/tabui-dopo` from `origin/main`. Coordinate with LAZY-WIFF (session `session_01Ly8ApmF7tTC43eWmFeBeCs`: slim engine, lazy readers, idle prefetch in `shell.js` / `carica.js`): start after it is merged, or ask the Coordinator.

**Problem.** `mzlab/web/tpmine-loader.js` loads `static/mzfinder/` (125–164 files) and `contaminanti.json` (457 KB) at page start: 2.2–2.8 MB and 125–164 requests before the Dati page is usable, all through the service worker queue.

**Do.** Load Tabui at idle after `window.qqReady` (`requestIdleCallback` with a timeout), or at once on the first Tabui use (button, route, `#/…` link), whichever comes first. One loader, one promise; a second call waits for the same promise. Nothing changes in what Tabui shows. Tabui stays English only.

**Tests.** `e2e_tpmine_ingresso.py`, `e2e_tpmine_pannello.py`, `e2e_tpmine1.py` green; add one check: at `qqReady` no `mzfinder/` resource has been requested yet (Resource Timing), and the first Tabui click still opens it. `tests/test_peso_avvio.py` green (never raise its caps).

**Measure.** `misura_ricarica.py` before/after; expected −125–164 requests and −2.2 MB before «usable» on cold and warm.

**Done.** PR ready with both tables, parity line (the desktop loads the same page: check it opens Tabui the same way); CI green; message to the Coordinator.
