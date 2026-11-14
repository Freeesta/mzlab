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

## Restarted service worker, latency, code cache, boot order

Tool: `tests_e2e/misura_avvio_sw.py` (measures, never asserts; reuses the mirror of `misura_ricarica.py`). Chromium 153 headless, 4 vCPU cloud runner, a new browser per run, median [min-max] of 5 interleaved runs (every combination once per round), ms from the navigation start of the measured page.
Site = the live site (build `9d2427b3ed`) copied into a local mirror with GitHub-Pages headers; the latency is a delay the mirror adds to **every** answer (HTTP/1.1, 6 connections per host, so a queue is worse than on the HTTP/2 Pages; a file the HTTP cache keeps pays nothing). CDP latency was tried
first and rejected: it does not reach the service worker's own fetches (the manifest was answered in 34 ms at RTT 150). CPU ×4 = CDP throttling of the page, workers and service worker. Site-only by nature: the desktop app serves `mzlab://` locally and has no service worker.

```
python3 tests_e2e/misura_avvio_sw.py --mirror DIR --runs 5 --rtt 0,40,150 --cpu 1,4 --scenari warm,warm-restarted --out warm.json        # table 1, 2 (DIR is filled from --url on the first run)
python3 tests_e2e/misura_avvio_sw.py --mirror DIR --runs 5 --rtt 0,40,150 --cpu 1,4 --scenari cold --out cold.json                          # table 2, 5
python3 tests_e2e/misura_avvio_sw.py --mirror DIR --runs 5 --cpu 1,4 --scenari trace-warm,trace-restarted [--trace-reloads 6]            # table 3
python3 tests_e2e/misura_avvio_sw.py --url https://freeesta.github.io/mzlab/ --runs 5 --rtt 0 --cpu 1 --scenari warm,warm-restarted,cold    # live rows
```

Scenarios. `warm`: first visit, wait for `.fill-done` in the app cache, 6 s for the idle work of the page (prefetch at +3 s, mzFinder package; with 1.5 s the old page was still busy and `warm` looked 200 ms slower than it is), then reload, SW running. `warm-restarted`: the same, but
`ServiceWorker.stopAllWorkers` (CDP) first, so the SW starts again with `visto` false (`sw.js:44`, `app` waits for the manifest, line 193). The restarted cost is the difference of the two rows. Boot steps come from the page (init script, no product change): `load` event of each script of list `L`,
the engine worker's `ready` message, setters on `qqReady` and `MZLAB_LR` (set right before `perfMark("motore pronto")`), a wrapper of `fetch` for `api/state`. `manifest_*`: the SW's own request to `static/sw-manifest.json` (CDP of the SW target); `index_srv`: arrival of the page request at the
mirror.

### 1-2. Warm vs warm-restarted (navigation start → …)

| scenario | RTT | CPU | index.html end | `index.html` at server | manifest asked | manifest answered | FCP | worker ready | engine ready | `qqReady` |
|---|---|---|---|---|---|---|---|---|---|---|
| warm | 0 | ×1 | 17 [16-24] | 8 [-1-11] | 2 [1-2] | 6 [4-8] | 112 [105-168] | 506 [402-678] | 523 [417-701] | 524 [417-702] |
| warm-restarted | 0 | ×1 | 30 [26-32] | 2 [1-3] | 14 [14-19] | 18 [16-23] | 114 [104-129] | 506 [425-597] | 524 [438-621] | 525 [438-622] |
| warm | 40 | ×1 | 100 [99-106] | 50 [47-54] | 2 [2-3] | 48 [44-52] | 210 [205-243] | 631 [586-737] | 657 [603-765] | 658 [604-766] |
| warm-restarted | 40 | ×1 | 67 [63-73] | 2 [2-2] | 13 [11-18] | 56 [54-62] | 172 [150-190] | 562 [473-631] | 580 [487-649] | 581 [488-650] |
| warm | 150 | ×1 | 324 [320-329] | 160 [159-162] | 2 [1-2] | 157 [156-158] | 441 [398-510] | 841 [780-1272] | 868 [806-1297] | 868 [807-1298] |
| warm-restarted | 150 | ×1 | 177 [175-178] | 2 [2-3] | 14 [11-16] | 167 [164-169] | 272 [264-289] | 606 [580-656] | 622 [593-673] | 624 [594-673] |
| warm | 0 | ×4 | 19 [17-21] | 10 [9-12] | 2 [1-4] | 6 [5-7] | 226 [206-252] | 1018 [879-1219] | 1032 [897-1232] | 1035 [907-1235] |
| warm-restarted | 0 | ×4 | 33 [29-37] | 3 [2-3] | 17 [16-21] | 21 [20-24] | 251 [215-264] | 1020 [960-1072] | 1038 [976-1090] | 1048 [980-1096] |
| warm | 40 | ×4 | 106 [101-107] | 53 [50-53] | 3 [2-5] | 49 [47-51] | 315 [296-365] | 1243 [1214-1574] | 1263 [1234-1601] | 1271 [1240-1606] |
| warm-restarted | 40 | ×4 | 75 [65-77] | 2 [2-4] | 18 [13-21] | 62 [56-64] | 286 [259-332] | 1181 [1002-1345] | 1195 [1018-1363] | 1199 [1021-1368] |
| warm | 150 | ×4 | 324 [319-327] | 161 [157-165] | 2 [1-3] | 158 [154-160] | 544 [514-562] | 1558 [1309-1674] | 1601 [1327-1706] | 1603 [1334-1721] |
| warm-restarted | 150 | ×4 | 181 [178-186] | 2 [2-3] | 16 [12-18] | 169 [165-175] | 399 [357-423] | 1182 [1057-1257] | 1202 [1070-1275] | 1208 [1071-1278] |
| **live** warm | — | ×1 | 93 [79-109] | — | 2 [1-2] | 43 [35-57] | 173 [161-216] | 560 [478-623] | 578 [493-633] | 578 [493-633] |
| **live** warm-restarted | — | ×1 | 65 [60-68] | — | 14 [11-16] | 53 [49-59] | 162 [149-182] | 592 [543-618] | 609 [572-636] | 610 [572-636] |

- **A restarted SW costs nothing on the critical path here; with latency it is faster.** At RTT 0 the two rows are equal (`qqReady` 524 vs 525; ×4 1035 vs 1048). The SW start is ~1-2 ms (`workerStart`), the manifest is asked 14 ms after the navigation start instead of 2 ms. With latency the restarted row wins by one round trip or more: RTT 150 `qqReady` 868 → 624 (×1), 1603 → 1208 (×4); FCP 441 → 272.
- **Why (observed, cause not isolated):** with the SW running, `navegar` (`sw.js:214`) asks the manifest first and only then `index.html` (the server sees the page request at 160 ms at RTT 150, after the manifest answer at 157). After a restart the server sees the page request at 2 ms, before the SW has asked for the manifest (14 ms): the page request does not wait behind the manifest. Candidate: this Chromium starts the navigation request in parallel with the worker start; not verified, and an older or different Chrome may not do it (not measured: only the Chromium of the cloud runner).
- **The manifest wait in `app()` (`visto` false, `sw.js:193`) is not seen as a cost:** the manifest answers in 18-21 ms at RTT 0 and in one RTT + ~20 ms after a restart, in parallel with the page request. The 3 s `NAV_MS` race was never reached (manifest answered in <200 ms in every run).

### 4. Boot order (ms from the navigation start; last script of `L`, `shell.js`, last module, engine worker `ready`, `api/state`, `qqReady`)

| scenario | RTT | CPU | last classic script | `shell.js` ran | last module | worker `ready` | engine ready | `api/state` answered | `qqReady` |
|---|---|---|---|---|---|---|---|---|---|
| cold | 0 | ×1 | 1480 [1464-1523] | 1497 [1485-1538] | 1500 [1487-1540] | 1839 [1832-1878] | 1868 [1846-1901] | 1869 [1847-1903] | 1870 [1847-1903] |
| cold | 0 | ×4 | 1720 [1679-1850] | 1773 [1741-1931] | 1781 [1759-1949] | 2043 [1917-2118] | 2075 [1944-2136] | 2083 [1952-2142] | 2083 [1953-2142] |
| warm | 0 | ×1 | 367 [296-487] | 377 [304-503] | 388 [330-514] | 506 [402-678] | 523 [417-701] | 524 [417-702] | 524 [417-702] |
| warm | 0 | ×4 (`cpu4warm`) | 694 [633-813] | 745 [671-886] | 809 [732-972] | 1018 [879-1219] | 1032 [897-1232] | 1034 [902-1234] | 1035 [907-1235] |
| warm-restarted | 0 | ×1 | 386 [328-458] | 395 [337-471] | 404 [350-487] | 506 [425-597] | 524 [438-621] | 525 [438-622] | 525 [438-622] |
| **live** cold | — | ×1 | 894 [686-1006] | 913 [698-1019] | 966 [751-1050] | 1290 [1036-1354] | 1300 [1050-1367] | 1302 [1052-1368] | 1302 [1052-1368] |

Cold at RTT 40 / 150 (mirror, ×1): `qqReady` 3359 / 7585; ×4: 3493 / 7749 (all columns in the run file; HTTP/1.1 queue, see above).

- **The last step is the engine, not the script tail, in every scenario and in every one of the 165 measured runs (the worker `ready` always comes after the last script of `L`).** `shell.js` runs 10-60 ms after the last classic script and the last module 2-100 ms after that; the engine worker posts
  `ready` 100-360 ms later (warm 118 ms, `cpu4warm` 209 ms, cold 339 ms, ×4 cold 262 ms), `MZLAB_LR` is set 17-32 ms after that and `api/state` answers ~1 ms after the engine (`shell.js` calls it at once, the answer is a worker call that is ready) and `qqReady` is set in the same task. So `qqReady`
  = worker `ready` + ~20 ms: shortening the script tail (list `L`, slices of 48) does not move `qqReady` until the worker ready arrives earlier than the tail.
- The engine starts at `DOMContentLoaded` and runs in parallel with the script tail; its own time is mostly the module graph plus the 2 MB `.wasm` (see 3) and the OPFS reopen.

### 3. Code cache (Chromium trace; categories `v8,v8.wasm,disabled-by-default-v8.compile,devtools.timeline`; traced load = 2nd reload after the first visit; 5 runs, CPU ×1 | ×4)

Event names found with a control page (a 240 KB script and the engine `.wasm` on a plain HTTP cache, no SW, 5 loads): a script that consumes its code cache has `v8.compile` / `v8.compileModule` with `args.data.consumedCacheSize` (+ `cacheKind: "normal"`, `cacheRejected: false`), seen from the 3rd
load on; `v8.produceCache` marks the cache being written; a script with no cache has `streamed: true` (or `notStreamedReason`) and none of those fields. For `.wasm` the same control shows **no** event that tells a hit from a miss (`wasm.StartStreamingCompilation`, `wasm.FinishStreaming`,
`wasm.GetNativeModuleFromCache`, the same in loads 2-5), so only times are reported.

| scenario | group | compile events | cache hits | rejected | streamed | eager compile ms (main thread / worker) |
|---|---|---|---|---|---|---|
| warm | scripts of `L` (main) | 152 | **0** | 0 | 152 | 4.5 [2.7-5.9] \| ×4 15.8 [12.2-22.3] |
| warm | other scripts (main) | 136 | **0** | 0 | 127 | 3.6 [3.3-4.7] \| ×4 17.1 [15.6-24.1] |
| warm | engine worker | 32 | **0** | 0 | 0 | 15.3 [13.7-18.2] \| ×4 13.2 [12.3-19.4] |
| warm-restarted | scripts of `L` (main) | 152 | **0** | 0 | 152 | 3.4 [2.7-15.1] \| ×4 12.8 [10.6-19.0] |
| warm-restarted | other scripts (main) | 136 | **0** | 0 | 127 | 3.6 [2.8-5.9] \| ×4 17.1 [14.6-20.2] |
| warm-restarted | engine worker | 32 | **0** | 0 | 0 | 12.6 [10.4-17.3] \| ×4 11.6 [11.3-12.4] |

- **No script is ever served from the code cache** (0 hits in 20 traced loads; the same on the 6th reload, `--trace-reloads 6`, 5 runs warm and restarted: 0 hits, 152 + 136 + 32 events), with the SW running or restarted. `v8.produceCache` fires 320 times per load: the cache is written (after each load) and never read back. Lazy function compiles on the main thread: 0.0 ms; in the engine worker 4-14 ms; background-thread streaming compile (`BackgroundCompileTask::Run`) 360-670 ms ×1 (284-1061 range) and 380-410 ms ×4, off the main thread.
- A plausible cause, **not tested here**: the SW answers every script with a new `Response` built by `isolata` (`sw.js:26-33`, also for cache hits, line 194), which drops the Cache Storage marker the browser needs to store a code cache for a SW-served script. A test would serve a script straight from the cache once; that changes a product file and is the owner's decision (S3/S4 wait for this).
- `.wasm` (`mzlab_wasm_bg.wasm`), warm ×1 | ×4: `FinishStreaming` 2.1 [1.3-4.1] | 6.5 [5.6-8.2] ms and `OnBytesReceived` 3.1 | 15.1 ms on the main thread, `GetNativeModuleFromCache` 0.6 ms on a pool thread, worker instantiate 0.9 ms; restarted: 1.8 | 6.4, 3.2 | 15.0, 0.7, 1.0 ms. The streaming compile itself runs on background threads outside these categories (`v8.wasm.detailed` not requested).

### 5. Isolation reload (cold; first page not isolated, second page after `isolateOnce`)

| RTT | CPU | requests of page 1 | KB sent (gzip) | files of page 2 | also in page 1 | of those: HTTP cache | revalidated (304) | full 200 | new files |
|---|---|---|---|---|---|---|---|---|---|
| 0 | ×1 | 33 [25-33] | 142 [129-142] | 243 [240-244] | 30 [22-30] | 29 [21-29] | 0 | 1 | 213 [210-221] |
| 40 | ×1 | 33 [32-33] | 142 [140-142] | 240 | 30 [29-30] | 29 [28-29] | 0 | 1 | 210 |
| 150 | ×1 | 31 | 137 [137-140] | 240 | 28 | 28 [27-28] | 0 | 0 [0-1] | 212 |
| 0 | ×4 | 27 | 170 | 229 [229-243] | 23 | 23 | 0 | 0 | 206 [206-220] |
| 40 | ×4 | 27 [27-32] | 170 [141-170] | 228 [228-240] | 23 [23-29] | 23 [23-29] | 0 | 0 | 205 [200-217] |
| 150 | ×4 | 32 [31-33] | 141 [139-177] | 240 [228-240] | 29 [28-30] | 29 [28-29] | 0 | 0 [0-1] | 210 [199-212] |

- The first page starts 27-33 requests and 137-170 KB (the reload is ~270 ms after the start, before most of the page's scripts are asked; bytes are what the mirror sent, gzip, aborted transfers included). The second page asks again for 23-30 of those files and gets 21-29 of them from the HTTP cache
  (the SW does `fetch(req)` with the HTTP cache first, `traer`); at most one goes to the network (200), none is revalidated. So the isolation reload re-downloads almost nothing: ~210 of the ~240 files of page 2 are new. "From the HTTP cache" = the file is in page 2's resource list and the mirror
  never saw a request for it during page 2's boot (the SW cache is still empty then: the fill starts 2.5 s after the worker).
- Live site cold: page-1 columns not available (no server log); page 2 asked 249 files, `qqReady` 1302 [1052-1368] ms.

### Answers to the open unknowns

- **OPFS clean-up timer** (`rust-worker.js:168`): `setTimeout(..., 8000)` after the worker starts (`limpia`: leftovers of the saved files). It runs in the engine worker after `ready` (~0.5-1.9 s), far from the critical path.
- **Reproducibility**: all numbers above are one runner, Chromium only (WebKit/Firefox not measured), the mirror's latency is a per-answer delay, not a real link. The 1.5 s vs 6 s settle before the measured load changed `warm` by ~200 ms: any future comparison must use the same settle (`SETTLE_MS`).
