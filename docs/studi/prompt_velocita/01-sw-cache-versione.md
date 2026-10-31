# V01: service worker cache-first by site version

You are a chat of mzLab (repository Freeesta/mzlab). Read AGENTS.md, then `docs/studi/VELOCITA_RICARICA.md` §2, §4, §6–8. Branch `claude/sw-cache-versione` from `origin/main`.

**Problem.** `mzlab/web/sw.js` is network-first for every app file: `fetch(req, {cache: "no-cache"})`, then `cache.put`. Each of the ~400 files of a load waits in the worker's queue for a revalidation (median 130–190 ms each, even on localhost). Measured: warm reload 0.8–1.2 s with the worker, 0.35–0.5 s without; after a reset 2–9 s.

**Do.**
1. App files (`mzlab-app-<hash>`): cache-first. The cache name already carries the hash of the whole site (`tools/build_site.py` writes it into `sw.js`), so a file in that cache is by construction the right version. A miss goes to the network and is stored.
2. At `install`, fill the versioned cache in the background from `static/manifest.json` (paths and SHA-256 of every file; skip `static/librerie/`, `vendor/` goes to `mzlab-big-*` as now). Do not block `install` on the whole list: the first page must not wait for it.
3. At `activate`, delete the old `mzlab-app-*` caches (as now) and keep `skipWaiting` + `clients.claim`; the `aggiorna.js` notice stays the way updates are announced.
4. Keep the `isolata()` wrapper (COOP/COEP/CORP) on every response, cached or not, and `index.html` / navigations network-first with a short timeout (so a new deploy is seen at the next navigation).
5. Never cache an `api/` request or a user file.

**Tests.** Extend `tests/test_sw_cache_node.py` (cache-first hit, miss → network → put, navigation network-first, old caches removed). Add to `tests_e2e/e2e_aggiorna.py` (or a new `e2e_sw_versione.py`) two builds served one after the other in the same browser: the page of build N never gets a file of build N+1, and after the update every file is N+1. `e2e_isolamento.py` green.

**Measure.** Before and after: `python3 tests_e2e/misura_ricarica.py --mirror <dir> --out a.json` (twice, the first is the warm-up) and `--profile slow`; summary with `tools/misura_ricarica_riassunto.py`. Expected: warm ≤ 0.6 s unthrottled, «SW→network» near 0 on a warm load.

**Done.** PR ready with both tables; one line on parity (browser only: the desktop never downloads `sw.js`); CI green; message to the Coordinator.
