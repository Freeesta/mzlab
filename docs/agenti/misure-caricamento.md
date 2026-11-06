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
- **No flash**: theme and language are set in the head before the first paint (CLS 0, `<html lang>` right in every run); no notice or modal at load. Console at load: only Chrome's warning on the Ketcher frame (`allow-scripts` + `allow-same-origin`).
- **First visit**: `browser.js isolateOnce` reloads once for cross-origin isolation (~270 ms after the start, before the cold FCP): the engine is ready ~260 ms later than without it, once per browser; GitHub Pages cannot send COOP/COEP, so the service worker adds them.
- **Not measured here**: WebKit/Safari (not installed in the cloud), a real Fast 4G link (the CDP profile does not slow the mirror: same numbers as cold).
- **Site update**: see LOAD-FIRST-PAINT-5 (the page is ready in about 0.55 s on both reloads after a deploy; only the changed files are downloaded); behaviour in `e2e_sw_versione`.
- KB before FCP counts every request started before the paint: on a warm load non-blocking scripts start earlier, so the number grows when the page is faster.
