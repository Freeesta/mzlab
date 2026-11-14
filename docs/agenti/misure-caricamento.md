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
- **Startup list weight** (`test_peso_avvio`, list `L` of `carica.js`, uncompressed): 152 files, 1,899,911 B of 1,900,000 (a new tool goes behind `carica()` or `import()`; `origine.js` 19,728 B is read by the `openOrigin` stub of `carica.js`, `elementare-pannello.js` 13,896 B by `tCmp` of `banco.js`, both on first use). Same table, two rounds before/after: engine ready cold 1587/1730 → 1710/1642 ms, CPU ×4 1992/2001 → 1952/1953 ms, KB until ready 5927 → 5894 (noise ±100 ms).
- **Slow link, first visit** (`tests_e2e/misura_lazy_lento.py`: 1.6 Mbit/s shared by all connections, 150 ms per answer, CPU ×4, local HTTP/1.1 server, 6 visits per site): the background fill of `sw.js` yields to the page (`primero`: while a page request is on the network the fill starts nothing and its open fetches are stopped and redone later). Page usable 23.5–23.7 s → 14.2–16.8 s; file loaded 25.0–25.4 → 24.2–24.6 s (one visit 33.9 s); first `carica` of `elementare-pannello.js` / `origine.js` 9–703 ms → 10–369 ms (a module the fill already holds answers in ~10 ms).
- **Click → composition, slow link** (same model, HR 32 MB, `BARRA.show('cmp')` right after the file loads until `#hri-body .cx`, 3 interleaved rounds on HTTP/1.0 and 1.1, medians, built from source): before, the click took 8.8 s (`elementare-pannello.js` queued behind the mzFinder scripts); with `sw.js` yielding, `carica` async at high priority and the idle mzFinder package `low`, 0.65–0.67 s, ready 52.2 → 32.8 s, file loaded 13.6 → 3.0 s, same SHA-256 of the file. The copy no longer waits for the hash worker script (it arrived 10–19 s late).
- **No flash**: theme and language are set in the head before the first paint (CLS 0, `<html lang>` right in every run); no notice or modal at load. Console at load: only Chrome's warning on the Ketcher frame (`allow-scripts` + `allow-same-origin`).
- **First visit**: `browser.js isolateOnce` reloads once for cross-origin isolation (~270 ms after the start, before the cold FCP): the engine is ready ~260 ms later than without it, once per browser; GitHub Pages cannot send COOP/COEP, so the service worker adds them.
- **Head round trips** (local HTTP/2 mirror like GitHub Pages: gzip, 40 ms per answer, shared 30 Mbit/s; Chromium, 6 interleaved visits, medians): with `preload` of `reset.js`, `telefono.js`, `browser.js`, `sito.js` the head scripts arrive together instead of one round trip each. First visit, navigation to engine ready: 1885 → 1426 ms, FCP 597 → 345 ms; reload: 610 → 573 ms. Firefox first visit 2664 → 2510 ms (3 visits). No file downloaded twice; KB and requests before ready unchanged.
- **Not measured here**: WebKit/Safari (not installed in the cloud), a real Fast 4G link (the CDP profile does not slow the mirror: same numbers as cold).
- **Site update**: see LOAD-FIRST-PAINT-5 (the page is ready in about 0.55 s on both reloads after a deploy; only the changed files are downloaded); behaviour in `e2e_sw_versione`.
- KB before FCP counts every request started before the paint: on a warm load non-blocking scripts start earlier, so the number grows when the page is faster.

## First package: what the browser must download and run (2026-11-14)

Question from the owner: «the main problem is the initial package; other sites show their interface much faster». Measured with `tests_e2e/misura_payload.py` (not run in CI; `--report DIR [--tables]` prints every table below from the run files). Built site (`tools/build_site.py`) plus the engine `.wasm` of the live site (`--prime`), Chromium headless, 4 vCPU cloud, Playwright.

**Method.** Local Node HTTP/2 server (self-signed certificate, Chromium with `--ignore-certificate-errors`) with GitHub-Pages-like headers and **gzip -9**. The link is shaped by the server (one shared downlink plus one delay per answer: «Fast 4G» 9 Mbit/s and 170 ms, «Slow 4G» 1.6 Mbit/s and 150 ms), because CDP throttling does not reach the service worker; CPU x4 over CDP.

Cold = new context (empty cache, no service worker; the worker installs during the visit and `browser.js isolateOnce` reloads once, so a first visit is **three navigations**). Warm = second visit of the same context after the worker's fill ended. Median of 3 runs (Slow 4G: 2). Times are ms from the start of the first navigation.
- **Encoding.** GitHub Pages sends gzip, never brotli: `curl -H 'Accept-Encoding: br, gzip'` on the live site answers `content-encoding: gzip` for html, js and the engine `.wasm` (`identity` gives the raw size). The page API cannot give compressed sizes of files answered by the service worker, so every «gzip» size below is the **computed gzip -9 of the file on disk**. Against the live `content-length` it is 0.1-4 % smaller (carica.js 2288 vs 2291, app.css 29785 vs 30747, the engine 930352 vs 938885): real numbers are 1-2 % higher.
- **Model limits.** One RTT per request (no TCP/TLS handshake, no loss); real GitHub Pages adds those. The earlier HTTP/1.1 local model (6 connections, `--http h1`) overstated Fast 4G cold: engine ready 9.9 s instead of 3.9 s; keep it only as an upper bound.

### Milestones (ms) and what is needed before each

| link | CPU | visit | FCP | drop area visible | last carica.js script | engine ready |
|---|---|---|---|---|---|---|
| none | x1 | cold | 140 | 132 | 762 | 1023 |
| none | x4 | cold | 268 | 275 | 1385 | 2045 |
| Fast 4G | x1 | cold | 728 | 701 | 2850 | 3910 |
| Fast 4G | x4 | cold | 792 | 831 | 3260 | 4377 |
| Slow 4G | x1 | cold | 1162 | 1136 | 5100 | 20568 |
| Slow 4G | x4 | cold | 1272 | 1246 | 7294 | 20316 |
| none | x1 | warm | 120 | 94 | 362 | 545 |
| none | x4 | warm | 360 | 360 | 779 | 1476 |
| Fast 4G | x1 | warm | 510 | 496 | 838 | 1892 |
| Fast 4G | x4 | warm | 574 | 567 | 891 | 2057 |

Slow 4G warm (engine ready 28-31 s) is **not** a page cost and is not used: the page took nothing from the network, the `.wasm` entry waited behind the worker's own transfers; not understood (open question: the worker fill seems to run again after the reload on a slow link). «Drop area visible» is the static drop area; files are accepted only after engine ready (`AVVIO.pronto`).

Needed **before** each milestone (files finished by then; Fast 4G, CPU x1, cold):

| milestone | requests | gzip KB | decoded KB |
|---|---|---|---|
| first contentful paint (712 ms) | 22 | 111 | 374 |
| drop area visible (701 ms) | 22 | 111 | 374 |
| last script of the carica.js list (2850 ms) | 184 | 812 | 2335 |
| engine ready (3910 ms) | 248 | 2008 | 5323 |

With no throttle the same milestones need 30, 29, 184 and 250 requests and 123, 123, 910 and 2019 KB gzip.

**(b) Whole critical chain** (until engine ready, the document of the first navigation left out): **248 requests, 2008 KB gzip, 5323 KB decoded** (2.65x); 2078 KB on Slow 4G cold. By kind: engine `.wasm` 1 file, 909 KB gzip (2107 decoded); 153 scripts inserted by `carica.js`, 684 (1926); 74 other scripts (ES modules, `rust-*.js`, worker, catalog `lang/it-resto.js`), 327 (1006); 4 render-blocking CSS, 33 (138); 12 parser scripts, 32 (86).

### Request table, reference run (Fast 4G, CPU x1, cold, median run by engine-ready time)

Request order; flags B render-blocking, P preload, C inserted by `carica.js`, E engine (`.wasm`); sizes in bytes (gzip -9 / decoded); ms from the first navigation. Head files carry the times of the second navigation (the first one is cut by the isolation reload). Files under 10 KB beyond the first 24 are summed in the last row. The full 248-row table of this run is in the PR body (`--report DIR --tables`).

| # | file | kind | gzip B | decoded B | start | end | flags |
|---|---|---|---|---|---|---|---|
| 1 | static/app.css | css (render-blocking) | 29785 | 128592 | 200 | 463 | B |
| 2 | static/appname.js | script (parser) | 444 | 749 | 200 | 406 | P |
| 3 | static/avvio-progressivo.js | script inserted by carica.js | 4485 | 14656 | 200 | 421 | PC |
| 4 | static/avvio.js | script (parser) | 3716 | 8674 | 200 | 417 | P |
| 5 | static/canali.css | css (render-blocking) | 1067 | 3013 | 200 | 406 | B |
| 6 | static/first-paint.js | script (parser) | 1929 | 4064 | 200 | 423 | P |
| 7 | static/frame.js | script (parser) | 541 | 1047 | 200 | 408 | P |
| 8 | static/i18n.js | script (parser) | 5433 | 13801 | 200 | 407 | P |
| 9 | static/impostazioni-comp.css | css (render-blocking) | 1814 | 6221 | 200 | 406 | B |
| 10 | static/tour.js | script (parser) | 7546 | 27025 | 200 | 415 | P |
| 11 | static/transizioni.css | css (render-blocking) | 1234 | 3568 | 200 | 406 | B |
| 12 | static/version.js | script (parser) | 105 | 98 | 200 | 407 | P |
| 13 | static/barra.js | script inserted by carica.js | 19900 | 69226 | 201 | 468 | PC |
| 14 | static/browser.js | script (parser) | 6163 | 15557 | 201 | 451 | P |
| 15 | static/carica.js | script (parser) | 2288 | 5497 | 201 | 441 | P |
| 16 | static/icons-modi.js | script inserted by carica.js | 1859 | 6444 | 201 | 425 | PC |
| 17 | static/nomi.js | script (parser, render-blocking) | 1463 | 5405 | 201 | 451 | B |
| 18 | static/reset.js | script (parser) | 3761 | 9123 | 201 | 445 | P |
| 19 | static/sito.js | script (parser) | 512 | 810 | 201 | 446 | P |
| 20 | static/telefono.js | script (parser) | 760 | 1296 | 201 | 446 | P |
| 21 | static/lr.js | script (other, ES module / worker / catalog) | 6623 | 19778 | 470 | 649 |  |
| 22 | static/lang/it.js | script (other, ES module / worker / catalog) | 12032 | 38368 | 472 | 659 | B |
| 23 | static/lr-map.js | script (other, ES module / worker / catalog) | 2064 | 6393 | 651 | 831 |  |
| 24 | static/lr-method.js | script (other, ES module / worker / catalog) | 1879 | 5149 | 651 | 831 |  |
| 32 |  (document) | navigation | 12683 | 47281 | 856 | 1258 | B |
| 39 | static/lang/it-resto.js | script (other, ES module / worker / catalog) | 101725 | 327540 | 1965 | 2935 |  |
| 47 | static/conversione.js | script inserted by carica.js | 13366 | 41078 | 1966 | 2289 | C |
| 69 | static/explore.js | script inserted by carica.js | 29855 | 92424 | 1970 | 2584 | C |
| 82 | static/grafici.js | script inserted by carica.js | 22642 | 63578 | 1975 | 2592 | C |
| 86 | static/pannelli.js | script inserted by carica.js | 34508 | 109387 | 1975 | 2818 | C |
| 89 | static/picchi.js | script inserted by carica.js | 21199 | 62423 | 1976 | 2598 | C |
| 91 | static/eventi.js | script inserted by carica.js | 21026 | 64311 | 1980 | 2607 | C |
| 96 | static/dda.js | script inserted by carica.js | 10700 | 35382 | 1981 | 2507 | C |
| 102 | static/banco.js | script inserted by carica.js | 18708 | 58220 | 1982 | 2816 | C |
| 112 | static/liste.js | script inserted by carica.js | 15388 | 50650 | 1982 | 2542 | C |
| 117 | static/mappa.js | script inserted by carica.js | 31438 | 99950 | 1982 | 2804 | C |
| 154 | static/shell.js | script inserted by carica.js | 25261 | 72113 | 1991 | 2850 | C |
| 174 | static/wasm/mzlab_wasm_bg.wasm | engine (.wasm) | 930352 | 2157096 | 1998 | 3865 | E |
| 181 | static/canali.js | script inserted by carica.js | 10249 | 32028 | 2009 | 2700 | C |
| 199 | static/draw.js | script (other, ES module / worker / catalog) | 23811 | 74976 | 2966 | 3193 |  |
| 216 | static/rust-worker.js | script (other, ES module / worker / catalog) | 15959 | 49204 | 3300 | 3296 |  |
| 219 | static/wasm/mzlab_wasm.js | script (other, ES module / worker / catalog) | 16382 | 102415 | 3303 | 3495 |  |
| 230 | static/rust-nativo.js | script (other, ES module / worker / catalog) | 10456 | 33428 | 3304 | 3535 |  |
| … | 205 more files < 10 KB gzip (mostly scripts inserted by carica.js and rust-*.js) | | 572967 | 1482901 | 651 | 3840 | |


### (c) The 10 biggest files of the chain and whether the first paint needs them

| # | file | gzip KB (decoded) | for | does the first paint need it? first function that needs it |
|---|---|---|---|---|
| 1 | `wasm/mzlab_wasm_bg.wasm` | 909 (2107) | Rust engine | no. `api("api/state")` in the boot of `shell.js` (line 31), through `createBridge` (`browser.js:78`); it sets `qqReady` |
| 2 | `lang/it-resto.js` | 99 (320) | second half of the text catalog | no. First `I18N.t` of a key outside the first-view part (`i18n.js` header: such a key gives an empty text until it arrives); `carica.js` loads it first (`I18N.rest`) so every later script finds its texts |
| 3 | `pannelli.js` | 34 (107) | floating panels | no. `place` / `plotFiles` (`pannelli.js:575`) when a file is opened |
| 4 | `mappa.js` | 31 (98) | RT x m/z map: crosshair, lock, expand | no. `window.MAPPA`, used by `drawMap` (`grafici.js:570`) when the map is shown |
| 5 | `explore.js` | 29 (90) | Data view base: toast, theme, file list | the boot, not the paint: `showStart` (`shell.js:420`), `mergeFiles`, `renderFileList` (`explore.js:773`) |
| 6 | `app.css` | 29 (126) | all styles | yes (render-blocking); how much of it the first screen uses was not measured |
| 7 | `shell.js` | 25 (70) | page shell, boot, views | the boot: `init()` (`shell.js:472`); the first paint is drawn before it (static HTML plus `first-paint.js`) |
| 8 | `draw.js` | 23 (73) | Disegno (structure editor) | no. `applyView` with the Disegno view (`shell.js:35`); `shell.js:472` appends it at every boot |
| 9 | `grafici.js` | 22 (62) | chromatogram drawing | no. `draw` (`grafici.js:182`), first chart of a file |
| 10 | `picchi.js` | 21 (61) | integration, spectra | no. `drawSpec` (`picchi.js:333`), first spectrum |

Only #6 is needed by the paint. The list is also loaded **in order** (`async=false`) and `shell.js` comes after every classic script, so engine ready waits for #3-#5, #9, #10 and about 140 more scripts, although none draws anything before a file is opened. `barra.js` (20 KB gzip) is part of the first paint by design (`first-paint.js`, the toolbar).

### (d) Same measure on public landing pages (real network through the runner proxy, CDP throttling of the page, CPU x1, median of 3)

| page | link | FCP ms | requests / gzip KB before FCP | total requests / gzip KB |
|---|---|---|---|---|
| htmx.org (docs) | none / Fast 4G | 504 / 776 | 11 / 286, 8 / 185 | 55 / 794 |
| sqlite.org (docs index) | none / Fast 4G | 420 / 540 | 2 / 16 | 9 / 90 |
| vuejs.org (guide) | none / Fast 4G | 604 / 1160 | 18 / 188, 17 / 188 | 79 / 544 |
| mzLab, this model (cold, local h2) | none / Fast 4G | 140 / 728 | 30 / 123, 22 / 111 | 250 / 2019 until engine ready |

The live mzLab site through the same script reads FCP 221 / 257 ms, but its bytes are not comparable (the final page is answered by the service worker, so encoded equals decoded, and CDP throttling does not reach the worker): use the local row. **The first paint is not what is slow**: FCP and the bytes before it are in the range of the other pages. What differs is the time to a usable page: engine ready after 2008 KB and 248 requests (Fast 4G, 3.9 s cold), against 90-800 KB for the whole of those pages.

### (e) Where the cold time goes

Chrome trace of the first cold run of each combination (`browser.start_tracing`, categories `devtools.timeline, v8.execute, v8, loading, toplevel`); main thread of every renderer, exclusive time per event name (script: EvaluateScript, compile, function calls, timers; parse: ParseHTML, stylesheets; render: style, layout, paint; «other»: tasks the trace does not label, e.g. GC and IPC); busy = top-level events; network wait = idle while at least one request of the page is in flight.

| link | CPU | until | window ms | script | parse | render | other | busy | idle, request in flight | idle, nothing in flight |
|---|---|---|---|---|---|---|---|---|---|---|
| none | x1 | engine ready | 1229 | 142 | 41 | 120 | 307 | 594 | 524 | 111 |
| none | x4 | engine ready | 2197 | 545 | 134 | 264 | 881 | 1812 | 318 | 68 |
| Fast 4G | x1 | FCP | 728 | 12 | 12 | 23 | 34 | 81 | 640 | 8 |
| Fast 4G | x1 | engine ready | 3882 | 114 | 29 | 71 | 228 | 430 | 2838 | 614 |
| Fast 4G | x4 | engine ready | 4377 | 503 | 165 | 337 | 831 | 1819 | 2006 | 552 |
| Slow 4G | x1 | engine ready | 21064 | 345 | 32 | 104 | 417 | 871 | 16636 | 3557 |

On a fast computer the wait is the network: Fast 4G x1 is idle with a request in flight for 73 % of the 3.9 s, busy for 11 %, idle with nothing in flight for 16 % (a 0.6 s gap between the arrival of the last document and the start of the `carica.js` list; cause not determined). With no network limit and CPU x4 the processor is the limit (busy 1.8 of 2.2 s). The first visit also loads the document and the head three times (service worker install, isolation reload): the last document arrives at 1258 ms against 186 ms for the first (Fast 4G).

### Candidate reductions of the first package (none implemented; estimates unless marked «measured»; 1 KB is about 0.9 ms on Fast 4G and 5 ms on Slow 4G, from the link rates; ranked by estimated Fast 4G saving per effort)

| # | candidate | gzip KB | Fast 4G cold, engine ready | Slow 4G cold | note |
|---|---|---|---|---|---|
| 1 | start the engine `.wasm` fetch from `<head>` (`<link rel=preload as=fetch>` in `index.html`) instead of after DOMContentLoaded (`browser.js:76`; today it starts at 1998 ms, the document arrived at 1258 ms) | 0 | about -0.6 s (estimate) | about 0 (bandwidth-bound) | a few lines, same bytes |
| 2 | no reload on the first visit: start the engine not isolated and isolate on the next visit (`browser.js isolateOnce`); the gap between the first and the last document is **1.07 s (measured)** | 0 | -0.5 to -1.0 s (estimate) | similar | the engine may need isolation for threads: check first |
| 3 | split the engine into a small startup part (what `api/state` needs) and lazy parts (the site already has lazy `.wasm` families); the size breakdown was not measured | -300 to -450 | -0.3 to -0.4 s (estimate) | -1.5 to -2.2 s (estimate) | native app unaffected; parity test on both halves |
| 4 | load the chart and tool scripts (`pannelli, mappa, grafici, picchi, eventi, banco, liste, conversione, dda` = **184.5 KB gzip, measured**) after engine ready, before the first file opens; `shell.js` stops waiting for them | -185 | -0.17 s | -0.9 s | the first file open waits for them (hidden by choosing the file); `test_peso_avvio` cap |
| 5 | concatenate the 152 list scripts into a few files in `build_site.py` (plain concatenation, not a bundler): **659 to 584 KB gzip and -150 requests, measured on disk** | -75 | -0.07 s, plus fewer requests (not measured on h2) | -0.4 s | the worker's per-file update diff gets coarser |
| 6 | `lang/it-resto.js` in parts loaded on first use of an area | -99 | -0.09 s | -0.5 s | catalog data, no logic; `controlla_i18n.py` |
| 7 | `draw.js` (and its modules) loaded on the first pointer or focus on the «Disegno» tab (`MZLAB_WARM_KETCHER` exists) instead of at boot (`shell.js:472`) | -23 | -0.02 s | -0.12 s | |
| 8 | split `app.css` into a blocking first-screen part and a non-blocking rest | -15 to -20 on the paint | FCP about -20 ms | FCP about -0.1 s | how much CSS the first screen uses was not measured |

Not candidates: brotli (GitHub Pages cannot send it), a bundler, a CDN, anything that sends a user file out of the computer.
