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
- **Startup list weight** (`test_peso_avvio`, list `L` of `carica.js`, uncompressed): 144 files, 1,838,247 B of 1,900,000 (61,753 B of room) (a new tool goes behind `carica()` or `import()`; `origine.js` 19,728 B is read by the `openOrigin` stub of `carica.js`, `elementare-pannello.js` 13,896 B by `tCmp` of `banco.js`, both on first use). Same table, two rounds before/after: engine ready cold 1587/1730 → 1710/1642 ms, CPU ×4 1992/2001 → 1952/1953 ms, KB until ready 5927 → 5894 (noise ±100 ms).
- **Slow link, first visit** (`misura_lazy_lento.py`: 1.6 Mbit/s shared, 150 ms per answer, CPU ×4, HTTP/1.1, 6 visits per site): the background fill of `sw.js` yields to the page (`primero`: the fill pauses while a page request is on the network). Page usable 23.5–23.7 s → 14.2–16.8 s; file loaded 25.0–25.4 → 24.2–24.6 s; `carica` of `elementare-pannello.js` / `origine.js` 9–703 ms → 10–369 ms.
- **Click → composition, slow link** (same model, HR 32 MB, `BARRA.show('cmp')` to `#hri-body .cx`, 3 rounds on HTTP/1.0 and 1.1, medians): before, the click took 8.8 s; with `sw.js` yielding and `carica` at high priority, 0.65–0.67 s, ready 52.2 → 32.8 s, file loaded 13.6 → 3.0 s.
- **No flash**: theme and language are set in the head before the first paint (CLS 0, `<html lang>` right in every run); no notice or modal at load. Console at load: only Chrome's warning on the Ketcher frame (`allow-scripts` + `allow-same-origin`).
- **First visit**: `browser.js isolateOnce` reloads once for cross-origin isolation (~270 ms after the start, before the cold FCP): the engine is ready ~260 ms later than without it, once per browser; GitHub Pages cannot send COOP/COEP, so the service worker adds them.
- **Head round trips** (local HTTP/2 mirror like GitHub Pages: gzip, 40 ms per answer, shared 30 Mbit/s; Chromium, 6 interleaved visits, medians): with `preload` of `reset.js`, `telefono.js`, `browser.js`, `sito.js` the head scripts arrive together instead of one round trip each. First visit, navigation to engine ready: 1885 → 1426 ms, FCP 597 → 345 ms; reload: 610 → 573 ms. Firefox first visit 2664 → 2510 ms (3 visits). No file downloaded twice; KB and requests before ready unchanged.
- **Early hints tried and dropped** (same mirror, Chromium, 6 interleaved visits, medians, min-max): `preload` of `lang/<L>.js` from the inline language script: first-visit FCP 312 [272-337] → 341 [308-376] ms; `modulepreload` of the bridge's static imports: engine ready 3450 [3206-3536] → 3460 [3426-3684] ms (modules start earlier, `.wasm` still sets the pace); warm unchanged (742 → 746 ms). Gate was 20 ms; see `ottimizzazioni-tentate.md`.
- **Not measured here**: WebKit/Safari (not installed in the cloud), a real Fast 4G link (the CDP profile does not slow the mirror: same numbers as cold).
- **Site update**: see LOAD-FIRST-PAINT-5 (the page is ready in about 0.55 s on both reloads after a deploy; only the changed files are downloaded); behaviour in `e2e_sw_versione`.
- KB before FCP counts every request started before the paint: on a warm load non-blocking scripts start earlier, so the number grows when the page is faster.

## Restarted service worker, latency, code cache, boot order

Tool: `tests_e2e/misura_avvio_sw.py` (measures, never asserts; reuses the mirror of `misura_ricarica.py`). Chromium 153 headless, 4 vCPU runner, new browser per run, median [min-max] of 5 interleaved runs (every combination once per round), ms from the navigation start of the measured page.
Site = the live site (build `9d2427b3ed`) copied into a local mirror with GitHub-Pages headers; the latency is a delay the mirror adds to **every** answer (HTTP/1.1, 6 connections per host, so a queue is worse than on the HTTP/2 Pages; a file the HTTP cache keeps pays nothing). CDP latency was tried
first and rejected: it does not reach the service worker's own fetches (the manifest was answered in 34 ms at RTT 150). CPU ×4 = CDP throttling of the page, workers and service worker. Site-only: the desktop app has no service worker.

```
python3 tests_e2e/misura_avvio_sw.py --mirror DIR --runs 5 --rtt 0,40,150 --cpu 1,4 --scenari warm,warm-restarted,cold,trace-warm,trace-restarted --out run.json   # DIR is filled from --url on the first run
python3 tests_e2e/misura_avvio_sw.py --mirror DIR --runs 5 --cpu 1 --scenari trace-warm,trace-restarted --trace-reloads 6                                                 # 6th reload
python3 tests_e2e/misura_avvio_sw.py --url https://freeesta.github.io/mzlab/ --runs 5 --rtt 0 --cpu 1 --scenari warm,warm-restarted,cold                                  # live rows
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
- **Why (observed, cause not isolated):** with the SW running, `navegar` (`sw.js:214`) asks the manifest first and only then `index.html` (the server sees the page request at 160 ms at RTT 150, after the manifest answer at 157). After a restart the server sees the page request at 2 ms, before the SW has asked for the manifest (14 ms): the page request does not wait behind the manifest. Candidate (not verified): this Chromium starts the navigation request in parallel with the worker start; other Chrome versions were not measured.
- **The manifest wait in `app()` (`visto` false, `sw.js:193`) is not a cost:** it answers in 18-21 ms at RTT 0, in parallel with the page request after a restart; the 3 s `NAV_MS` race was never reached.

### 4. Boot order (ms from the navigation start; last script of `L`, `shell.js`, last module, engine worker `ready`, `api/state`, `qqReady`)

| scenario | RTT | CPU | last classic script | `shell.js` ran | last module | worker `ready` | engine ready | `api/state` answered | `qqReady` |
|---|---|---|---|---|---|---|---|---|---|
| cold | 0 | ×1 | 1480 [1464-1523] | 1497 [1485-1538] | 1500 [1487-1540] | 1839 [1832-1878] | 1868 [1846-1901] | 1869 [1847-1903] | 1870 [1847-1903] |
| cold | 0 | ×4 | 1720 [1679-1850] | 1773 [1741-1931] | 1781 [1759-1949] | 2043 [1917-2118] | 2075 [1944-2136] | 2083 [1952-2142] | 2083 [1953-2142] |
| warm | 0 | ×1 | 367 [296-487] | 377 [304-503] | 388 [330-514] | 506 [402-678] | 523 [417-701] | 524 [417-702] | 524 [417-702] |
| warm | 0 | ×4 (`cpu4warm`) | 694 [633-813] | 745 [671-886] | 809 [732-972] | 1018 [879-1219] | 1032 [897-1232] | 1034 [902-1234] | 1035 [907-1235] |
| warm-restarted | 0 | ×1 | 386 [328-458] | 395 [337-471] | 404 [350-487] | 506 [425-597] | 524 [438-621] | 525 [438-622] | 525 [438-622] |
| **live** cold | — | ×1 | 894 [686-1006] | 913 [698-1019] | 966 [751-1050] | 1290 [1036-1354] | 1300 [1050-1367] | 1302 [1052-1368] | 1302 [1052-1368] |

Cold at RTT 40 / 150 (mirror): `qqReady` 3359 / 7585 (HTTP/1.1 queue, see above).

- **The last step is the engine, not the script tail, in every scenario and in every one of the 165 measured runs (the worker `ready` always comes after the last script of `L`).** `shell.js` runs 10-60 ms after the last classic script and the last module 2-100 ms after that; the engine worker posts
  `ready` 100-360 ms later (warm 118 ms, `cpu4warm` 209 ms, cold 339 ms, ×4 cold 262 ms), `MZLAB_LR` is set 17-32 ms after that and `api/state` answers ~1 ms after the engine (`shell.js` calls it at once, the answer is a worker call that is ready) and `qqReady` is set in the same task. So `qqReady`
  = worker `ready` + ~20 ms: shortening the script tail (list `L`, slices of 48) does not move `qqReady` until the worker ready arrives earlier than the tail.

### 3. Code cache

Moved to `misure-code-cache.md` (S1 trace of main and the S3 test of serving cached copies without a new `Response`).

### 5. Isolation reload (cold; first page not isolated, second page after `isolateOnce`)

| RTT | CPU | requests of page 1 | KB sent (gzip) | files of page 2 | also in page 1 | of those: HTTP cache | revalidated (304) | full 200 | new files |
|---|---|---|---|---|---|---|---|---|---|
| 0 | ×1 | 33 [25-33] | 142 [129-142] | 243 [240-244] | 30 [22-30] | 29 [21-29] | 0 | 1 | 213 [210-221] |
| 150 | ×1 | 31 | 137 [137-140] | 240 | 28 | 28 [27-28] | 0 | 0 [0-1] | 212 |
| 0 | ×4 | 27 | 170 | 229 [229-243] | 23 | 23 | 0 | 0 | 206 [206-220] |
| 150 | ×4 | 32 [31-33] | 141 [139-177] | 240 [228-240] | 29 [28-30] | 29 [28-29] | 0 | 0 [0-1] | 210 [199-212] |

- The first page starts 27-33 requests and 137-170 KB (the reload comes ~270 ms after the start; bytes are what the mirror sent, gzip). The second page asks again for 23-30 of those files and gets 21-29 of them from the HTTP cache
  (the SW does `fetch(req)` with the HTTP cache first, `traer`); at most one goes to the network (200), none is revalidated. The isolation reload re-downloads almost nothing: ~210 of the ~240 files of page 2 are new. "From the HTTP cache" = the file is in page 2's resource list and the mirror
  never saw a request for it during page 2's boot (the SW cache is still empty then: the fill starts 2.5 s after the worker).

### Answers to the open unknowns

- **OPFS clean-up timer** (`rust-worker.js:168`): `setTimeout(..., 8000)` after the worker starts. Off the critical path.
- **Reproducibility**: all numbers above are one runner, Chromium only (WebKit/Firefox not measured), the mirror's latency is a per-answer delay, not a real link. Keep the 6 s settle (`SETTLE_MS`) when comparing: 1.5 s changed `warm` by ~200 ms.


## First file and many files: critical path

Measurement only (no product change), `tests_e2e/misura_caricamento.py --scenarios uno,dieci,rotte,thermo,subito,interazione,memoria` (module `tests_e2e/scenari_caricamento.py`; never asserts, not in the CI). Site only: the desktop app serves `mzlab://` locally, has no service worker (SW) and no network, and its native path is different, so none of this applies there.
Setup: `main` `2cf7180`, site from `tools/prepara_sito.py`, Chrome for Testing 153 headless, one persistent profile per run (a normal window; cross-origin isolated), cloud machine 4 vCPU Intel Xeon 2.10 GHz.

Files: synthetic mzML 50.7 MB (2400 scans, `mzml_msconvert_synthetic.py --peaks1 5000 --peaks2 500`) and synthetic Thermo `.raw` 50.6 MB (`thermo_synthetic --scans 950 --peaks 600`); a drop of ten = the same file first plus nine copies with other names. **warm** = copy of a profile whose SW fill is done (`.fill-done`); **cold** = empty profile, the drop 3 s after `qqReady` unless the arm says otherwise.

N = 5 per cell, arms interleaved run by run, median [min-max], ms from the `change` of the file input. CPU 1× and 4× throttle the page's main thread only (workers run at full speed). Provisional TIC = `#ticprov` drawn; first chart = a panel drawn; all indexed = nothing read or opened any more; copy start = first `mzlab-index` event (for a `.raw` the first «read» event, there is no OPFS copy).

**(c) one file alone vs first in a drop of ten (warm)**

| file | CPU | arm | provisional TIC | first chart | all indexed | long tasks to first chart |
|---|---|---|---|---|---|---|
| mzML | 1× | uno | 553 [503-580] | 1611 [1594-1874] | 1635 | 1 (98 ms) |
| mzML | 1× | dieci | 2251 [2165-2349] | 3814 [3496-4081] | 7122 [6714-7341] | 1 (123 ms) |
| mzML | 4× | uno | 651 [571-829] | 2529 [2435-2735] | 2637 | 2 (446 ms) |
| mzML | 4× | dieci | 2615 [2466-2704] | 5168 [4216-5361] | 9425 [8987-9620] | 2 (604 ms) |
| `.raw` | 1× | uno | 230 [214-255] | 1013 [947-1130] | 1045 | 1 (61 ms) |
| `.raw` | 1× | dieci | 520 [486-628] | 4121 [3583-4886] | 5172 [4902-5997] | 1 (114 ms) |
| `.raw` | 4× | uno | 424 [335-437] | 1798 [1740-1855] | 1889 | 2 (391 ms) |
| `.raw` | 4× | dieci | 854 [690-996] | 5578 [5149-5812] | 8250 [7497-8472] | 5 (673 ms) |

**(a)(b) a module held 3 s after the drop** (cold, one file; the server answers the request for the `.wasm` 3 s after the drop whatever the moment it was asked, the SW fill's request before the drop included)

| module | file | CPU | base: first chart | held: first chart | base: copy start | held: copy start |
|---|---|---|---|---|---|---|
| `rotte` | mzML | 1× | 1514 [1369-1698] | 1611 [1531-1746] | 39 | 39 |
| `rotte` | mzML | 4× | 2432 [2329-2582] | 2307 [2242-2457] | 93 | 94 |
| `thermo` | `.raw` | 1× | 985 [960-1045] | 3608 [3591-3623] | 50 | 62 |
| `thermo` | `.raw` | 4× | 1784 [1701-1865] | 4104 [4031-4154] | 84 | 83 |

**(d) drop at `qqReady` + 0 vs + 20 s (mzML, one file)**

| profile | CPU | drop at | provisional TIC | first chart | requests drop → chart | long tasks to chart |
|---|---|---|---|---|---|---|
| cold | 1× | +0 | 792 [741-829] | 2118 [2022-2308] | 333 (11 MB) | 1 (138 ms) |
| cold | 1× | +20 s | 561 [493-704] | 1647 [1527-1761] | 0 | 1 (119 ms) |
| warm | 1× | +0 | 574 [548-660] | 1663 [1523-1765] | 0 | 1 (98 ms) |
| warm | 1× | +20 s | 557 [458-719] | 1680 [1447-1909] | 0 | 1 (115 ms) |
| cold | 4× | +0 | 1195 [1019-1228] | 3528 [3302-4125] | 438 (13 MB) | 3 (536 ms) |
| cold | 4× | +20 s | 629 [552-802] | 2576 [2294-2779] | 0 | 1 (420 ms) |
| warm | 4× | +0 | 760 [702-1049] | 2545 [2249-3011] | 0 | 2 (546 ms) |
| warm | 4× | +20 s | 668 [640-707] | 2680 [2187-2895] | 1 | 2 (552 ms) |

The 333-438 requests of the cold +0 arm are the SW's fill (all the `.wasm` modules, the library engine, the readers, the example files excluded) and the mzFinder/OpenChemLib files; the warm arms serve them from the SW cache (0 network requests).

**(e) first chart's responsiveness while files 2..10 index** (mzML, warm; 20 Ctrl+wheel zooms, 20 Shift+drags of 3 pointermoves, 5 clicks on the TIC; input → next frame is two rAFs after the event; click → last `spec` draw; Event Timing durations ≥ 16 ms from the first chart on; «busy» = the loading still running when the event happened)

| arm | CPU | wheel p50 / p95 | wheel in the busy window | drag p50 / p95 | drag in the busy window | Event Timing p50 / p98 | click → spectrum | rAF delta p95 (first chart → all indexed) |
|---|---|---|---|---|---|---|---|---|
| uno+gesti (nothing else loads) | 1× | 21 / 28 | 0 | 19 / 20 | 0 | 24 / 48 | 66 [52-70] | n/a (no window) |
| dieci+gesti | 1× | 22 / 38 | 20 of 20 | 19 / 24 | 21 of 60 | 32 / 64 | 64 [63-68] | 36 [31-40] (without gestures 36 [33-46]) |
| uno+gesti | 4× | 48 / 94 | 0 | 22 / 34 | 0 | 48 / 112 | 198 [188-198] | n/a |
| dieci+gesti | 4× | 52 / 83 | 9 of 20 | 24 / 35 | 0 of 60 | 56 / 136 | 209 [181-236] | 189 [181-233] (without gestures 187 [182-253]) |

At 4× the gestures start after the first chart (5 s) and last about 9 s, so only 9 wheels fall in the loading window and no drag; the rest ran on a quiet page.

**(f) peak memory during (c) `dieci`** (mzML, warm, 530 MB of files)

| CPU | `measureUserAgentSpecificMemory` peak | page JS heap peak | RSS of all browser processes, peak | first chart | all indexed |
|---|---|---|---|---|---|
| 1× | 64 [64-66] MB | 33 [30-40] MB | 1940 [1883-2006] MB | 3529 [3485-3763] | 6951 [6686-7051] |
| 4× | 65 [65-108] MB | 30 [29-33] MB | 1777 [1716-1823] MB | 5189 [4821-5477] | 9596 [9154-9793] |

`measureUserAgentSpecificMemory` answers 5-10 s after the call (the peaks came back about 12 s after the drop, after the loading), so its 500 ms sampling is a series of overlapping calls; it covers the page's own agent cluster and not the engine's WASM memories, so it is a floor. The RSS (read from `/proc` every 500 ms, all processes started with the profile: page, builders and their helpers, engine worker, GPU, network) is a ceiling: it holds file-backed pages the copy of 530 MB touched.

**(b) `thermo` asked at the drop: no gain** (cold profile, drop at `qqReady` + 0, one `.raw` of 50.6 MB, `--scenarios raw-subito`, N = 12 per arm interleaved run by run with the order alternating, median [min-max] ms, same machine and files as above). The arm that asked the engine worker for the module (`prepara`) as soon as the `.raw` was recognised, before the quota check, against the module asked at the read:

| CPU | arm | first chart | copy start |
|---|---|---|---|
| 1× | module asked at the read | 1297 [1132-4135] | 98 [41-540] |
| 1× | module asked at the drop | 1402 [1019-1703] | 82 [45-150] |
| 4× | module asked at the read | 2443 [2119-3005] | 172 [90-238] |
| 4× | module asked at the drop | 2523 [1992-2991] | 180 [117-235] |

Medians: +105 ms (1×) and +80 ms (4×) for the arm asked at the drop, inside the spread of both arms. A first run of N = 5 had shown −146 / −303 ms with overlapping ranges; N = 12 does not confirm it. Not adopted (`ottimizzazioni-tentate.md`).

### Verdicts

The verdicts (a)-(f) of this section, with the copy gate that answers (c): `misure-copie.md`, «Critical path: verdicts».
