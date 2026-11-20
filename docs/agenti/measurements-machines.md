# Measurements on slow computers

Report only: no threshold that fails. It shows where the page breaks first when the computer is 4-6 times slower than a MacBook M1 Pro (input for thresholds, light/normal/powerful profiles, chunked work, memory cap).

## How to run
- **Command**: `python3 tools/prepara_sito.py` (once: `site/` with the .wasm engine) and then `python3 tests_e2e/misura_macchine.py --md OUT.md [--json OUT.json] [--reps 12] [--rates 1,4,6] [--cores 2,4]`. It is not part of the CI tests (the name does not start with `e2e_`) and never fails on a number.
- **Duration**: 4.6 minutes for the 6 configurations (1×, 4×, 6× × 2 and 4 cores) with `--reps 12` on a 4-CPU container (22 s at 1×, 46 s at 4×, 68 s at 6× per configuration); building the site the first time takes a few minutes more.
- **How it measures**: every configuration is a new browser (empty cache and storage); `Emulation.setCPUThrottlingRate` as in `e2e_cls.py`, `navigator.hardwareConcurrency` redefined by a start-up script. The time starts before the action and stops after the first draw following the result, inside the page (without Playwright's delay). The phases come from hooks that wrap `fetch`, `Response.json`, `draw()` and `drawMap()` without touching the product: request = union of the `api/` calls, engine = sum of `X-Py-Ms`, JSON, draw = sum of the durations of `draw()`. The phases overlap.
- **Data**: only the five Full Scan example files (`mzlab/web/esempi/`, downloaded from the site with "Try with the example files"), never lab data.

## Results
Machine: cloud container, 4 CPUs, headless Chromium; `--reps 12` (2D and 3D maps: 4 repetitions; opening: one only). Cells `p50 / p95 ms`; for the drag, interval between two frames and dropped frames (intervals over 25 ms, counted as 16.7 ms each) over the two passes forth and back of the line. Last row: peak JS heap / engine memory in MB.

| action | 1× / 2 cores | 1× / 4 cores | 4× / 2 cores | 4× / 4 cores | 6× / 2 cores | 6× / 4 cores |
|---|---|---|---|---|---|---|
| open Full Scan demo: first chart | 429 | 345 | 878 | 823 | 1384 | 1420 |
| open Full Scan demo: ready | 2535 | 2254 | 3082 | 3105 | 4609 | 5124 |
| drag the line: interval between frames | 16.7 / 16.8 (0 dropped) | 16.7 / 16.7 (0 dropped) | 16.7 / 83.3 (314 dropped) | 16.7 / 66.7 (265 dropped) | 16.7 / 116.7 (604 dropped) | 16.7 / 133.3 (675 dropped) |
| drag the line: one `draw()` (chromatogram or spectrum) | 4.1 / 52.1 | 3.8 / 46.5 | 26.5 / 139.5 | 23.1 / 137.9 | 37.7 / 174.9 | 43.9 / 206.4 |
| new panel (chromatogram) | 44.8 / 45.4 | 44.2 / 45.5 | 110.8 / 124.5 | 99.1 / 145.9 | 183.1 / 320.5 | 204 / 254.8 |
| file switch | 30.5 / 280 | 30 / 313.3 | 136.6 / 427.3 | 132.2 / 430.8 | 191.4 / 493.3 | 202.3 / 493.8 |
| 2D map | 62.3 / 199.9 | 56.3 / 186.6 | 208.9 / 647.1 | 202.6 / 546.7 | 392.3 / 1096 | 415.9 / 916 |
| 3D map | 1323 / 1390 | 1380 / 1503 | 2533 / 2573 | 2448 / 2709 | 3325 / 3638 | 3221 / 3267 |
| XIC from a click | 61.7 / 124.5 | 61.1 / 119.5 | 245 / 401.4 | 244.1 / 289.7 | 457.3 / 555.7 | 381.8 / 467.8 |
| JS heap / engine (MB) | 61.8 / 32 | 59.1 / 32 | 68 / 32 | 66.4 / 32 | 67.8 / 32 | 65 / 32 |

## The 5 worst actions at 6× (4 cores, p50) and where the time goes
Median phases in ms (they overlap): request / engine / JSON / draw; in brackets the same at 1×.
1. **Opening, ready**: 5124 ms (2254 at 1×); the first chart is at 1420 ms. Not broken down by the hooks (it includes reading the 5 files and drawing the panels).
2. **3D map**: 3221 ms (1380); request 271 (96), JSON 70 (6), the rest is WebGL drawing. Here WebGL is software (container without a GPU): the number is pessimistic and must be reconfirmed on a real laptop.
3. **2D map**: 416 ms with p95 916 (56 / 187); draw 559 (57), no request in the count: the time is all in drawing the image.
4. **XIC from a click**: 382 ms (61); request 139 (29), engine 322 (69), draw 267 (40). The engine costs 4.6 times more at 6×: Chromium's CPU slowdown also slows the engine's worker.
5. **New panel 204 ms and file switch 202 ms** (44 and 30): draw 69 and 129 (8 and 12); no request.
Dragging the line is the most visible action: in about 11 s of dragging, 265-314 frames are dropped at 4× and 604-675 at 6× (0 at 1×); at 6× one `draw()` costs 38-44 ms median and 175-206 at p95 (4 at 1×), with 118 long tasks in 18 s at 6×.

## What the numbers say
- With the slowdown the actions cost 2.3 to 7 times more (6×: XIC 6.2, file switch 6.7, 2D map 7.4, new panel 4.6, 3D map 2.3, drag `draw()` 9-12). All exceed 16 ms already at 1× except the drag `draw()`; 2D map, XIC and file switch are above 130 ms already at 4×.
- The declared number of cores (2 against 4) changes nothing here: the values differ within the noise. The measuring machine has 4 CPUs and does not limit real threads; the real core difference must be measured on a real computer (or by limiting the process's CPUs with `taskset`).
- Memory is not the problem in this scenario (JS heap ~65 MB, engine 32 MB, regardless of the slowdown). `measureUserAgentSpecificMemory` is not available without cross-origin isolation: the site column stays `performance.memory`.
- The p95 of the file switch at 1× (280-313 ms against a p50 of 30) is the first switch, cold: with only 3 repetitions it becomes the p50.
- Limits: one round, one machine, small `n` for the maps and for opening; the p95 values are indicative. The comparison holds only between cells of the same round.

## Opening "ready" at 6×: measured cause
- **Bench**: `--open-reps N` repeats only the opening in new browsers (a single one is noisy); `--patch FILE.js` adds a start-up script to try an idea (A/B) without touching the product.
- **Where the time goes** (Chromium CPU profile, 6×, 4 cores): the first chart is at 1.4 s; then the page makes one more round for each added file (`api/session`, `explore`, TIC, spectrum, `metodo`, `method_warnings`: 5 rounds, each 0.4-0.8 s at 6×) up to 4.5-5 s. The first round lasts 3.4 s: the TIC of file 0 costs 1031 ms of engine against 93 at 1× (11×, more than the 6× slowdown: cold start of the engine) and `metodo` 1.1-1.2 s. The main thread has 21 long tasks in the period; native (`(program)`) 3.1 s, GC 0.56 s, JS about 2 s.
- **Routes with no gain beyond the noise (±5 %)**: memoising `namesWidth`, caching the TIC and the spectra of the files already open, one draw per frame of the file list; numbers in `optimizations-tried.md`. The critical path is the engine (one round per file, in series), not the main thread.
- **Opening with fewer rounds** (redraw of `openRest` every 3 times the duration of the last one): no gain (4881 against 4884 ms), see `optimizations-tried.md`. Starting the engine before the click is also already like that (it starts at page load) and 10 s of quiet before the click do not change the ready time (4529 against 4458 ms): the cold TIC of file 0 (1031 ms against 93 at 1×) is not the start-up but the engine's first work; see `optimizations-tried.md`.

## Line drag and XIC from a click at 6×
- **Drag** (CPU profile, 6×, 18 s): 81 chromatogram `draw()` calls (about 40 ms each) and 81 spectrum ones (about 75 ms); 662 dropped frames, 124 long tasks. Self time: `showHover` 1.0 s, `placeLeg`/`legInk` 0.8-1.6 s, `ctl` 0.5 s, `drawImage` 0.5 s, `cursorLine` 0.5 s, `setup` (shell.js) 0.5 s; 71 % of the samples are native (`(program)`, paint, GC). No single spot exceeds 8 %: holding back the legend during the drag gave no gain (see `optimizations-tried.md`).
- **XIC from a click** (6×, 4 cores: 382 ms): about half request (engine 322 ms against 69 at 1×, because the CPU slowdown also slows the worker) and half panel drawing (267 ms). Reopening the engine file cannot be kept and the panel drawing is already optimised: no small fix.
- **2D map** (6×, 8 profiled openings): every opening draws two maps (`pda` and `level=1`); `mapImage` weighs about 116 ms per opening out of 416, the rest is paint and native work. A fix on the percentile gave no effect above the noise (see `optimizations-tried.md`).
