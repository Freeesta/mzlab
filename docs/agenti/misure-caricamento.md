# Page load (current state)

`python3 tests_e2e/misura_primo_paint.py --site site --runs 5` on the built site (engine `.wasm` from the live site), local mirror without network latency, Chromium headless 4 vCPU cloud, median of 5, ms from the navigation (cold = second page of the first visit, after the isolation reload). Guard: `tests_e2e/e2e_budget_avvio.py`.

| Scenario | FCP | LCP | drop area | engine ready | CLS | longest task until ready +2 s | render-blocking | KB before FCP |
|---|---|---|---|---|---|---|---|---|
| cold | 341 | 399 | 334 | 1957 | 0 | 0 | 11 | 666 |
| warm (service worker) | 188 | 227 | 168 | 815 | 0 | 0 | 11 | 788 |
| cold `?lang=en` | 377 | 434 | 380 | 2022 | 0 | 0 | 11 | 655 |
| warm, dark theme saved | 189 | 266 | 141 | 841 | 0 | 0 | 11 | — |
| cold, CPU ×4 | 384 | 550 | 454 | 2267 | 0 | 60 | 11 | 666 |
| warm, CPU ×4 | 425 | 554 | 496 | 1539 | 0 | 52 | 11 | 666 |
| cold, iPad viewport + touch (Chromium) | 364 | 430 | 361 | 2045 | 0 | 0 | 11 | 666 |

- **Views** (warm, click after the page is usable): Disegno 148 ms (2 files, 13 KB); Tabui 133 ms; Flow 200 ms when the Tabui package has loaded, longer when the click comes before it: the 60 `mzfinder/` scripts load at idle after `qqReady`, all tags inserted at once in order (package done ~1.1 s warm, ~2.8 s on a cold visit). Stat panel 51 ms (×4: 157), Molecules window 222 ms the first time (module and archive; ×4: 363), then ~30 ms.
- **Startup list weight** (`test_peso_avvio`, list `L` of `carica.js`, uncompressed): 149 files, 1,869,305 B of 1,900,000 (LOAD-DIET-1: 151 files, 1,899,723 B before; `origine.js` 19,728 B is read by the `openOrigin` stub of `carica.js`, `elementare-pannello.js` 13,896 B by `tCmp` of `banco.js`, both on first use). Same table, two rounds before/after: engine ready cold 1587/1730 → 1710/1642 ms, CPU ×4 1992/2001 → 1952/1953 ms, KB until ready 5927 → 5894 (noise ±100 ms).
- **Slow link, first visit** (`tests_e2e/misura_lazy_lento.py`: 1.6 Mbit/s shared by all connections, 150 ms per answer, CPU ×4, local HTTP/1.1 server, 6 visits per site): the background fill of `sw.js` yields to the page (`primero`: while a page request is on the network the fill starts nothing and its open fetches are stopped and redone later). Page usable 23.5–23.7 s → 14.2–16.8 s; file loaded 25.0–25.4 → 24.2–24.6 s (one visit 33.9 s); first `carica` of `elementare-pannello.js` / `origine.js` 9–703 ms → 10–369 ms (a module the fill already holds answers in ~10 ms).
- **Click → composition, slow link** (same model, HR 32 MB, `BARRA.show('cmp')` right after the file loads until `#hri-body .cx`, 3 rounds on HTTP/1.0 and 1.1, medians): with only `sw.js` yielding the click took 8.8–9.0 s (`elementare-pannello.js` queued behind the mzFinder scripts); with `carica` async at high priority and the idle mzFinder package `low`, 0.79–0.84 s, ready 52.3 → 32.8–33.0 s, file loaded 13.6 → 3.1–3.2 s. The copy no longer waits for the hash worker script (it arrived 10–19 s late).
- **No flash**: theme and language are set in the head before the first paint (CLS 0, `<html lang>` right in every run); no notice or modal at load. Console at load: only Chrome's warning on the Ketcher frame (`allow-scripts` + `allow-same-origin`).
- **First visit**: `browser.js isolateOnce` reloads once for cross-origin isolation (~270 ms after the start, before the cold FCP): the engine is ready ~260 ms later than without it, once per browser; GitHub Pages cannot send COOP/COEP, so the service worker adds them.
- **Not measured here**: WebKit/Safari (not installed in the cloud), a real Fast 4G link (the CDP profile does not slow the mirror: same numbers as cold).
- **Site update**: see LOAD-FIRST-PAINT-5 (the page is ready in about 0.55 s on both reloads after a deploy; only the changed files are downloaded); behaviour in `e2e_sw_versione`.
- KB before FCP counts every request started before the paint: on a warm load non-blocking scripts start earlier, so the number grows when the page is faster.
