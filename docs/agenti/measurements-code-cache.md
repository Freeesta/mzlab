# Code cache of the scripts and the engine `.wasm` (site only)

Site only: the desktop app serves `mzlab://` locally and has no service worker, so none of this applies there. Tool: `tests_e2e/misura_avvio_sw.py` (setup and scenarios in `measurements-loading.md`, "Restarted service worker").

## S1 (main): code cache (Chromium trace; categories `v8,v8.wasm,disabled-by-default-v8.compile,devtools.timeline`; traced load = 2nd reload after the first visit; 5 runs, CPU ×1 | ×4)

Event names from a control page (240 KB script and the engine `.wasm`, plain HTTP cache, no SW, 5 loads): a script that consumes its code cache has `v8.compile` / `v8.compileModule` with `args.data.consumedCacheSize` (+ `cacheKind: "normal"`, `cacheRejected: false`), seen from the 3rd
load on; `v8.produceCache` marks the cache being written; a script with no cache has `streamed: true` (or `notStreamedReason`) and none of those fields. For `.wasm` the same control shows **no** event that tells a hit from a miss (`wasm.StartStreamingCompilation`, `wasm.FinishStreaming`,
`wasm.GetNativeModuleFromCache`, the same in loads 2-5), so only times are reported.

| scenario | group | compile events | cache hits | rejected | streamed | eager compile ms (main thread / worker) |
|---|---|---|---|---|---|---|
| warm and warm-restarted | scripts of `L` (main) | 152 | **0** | 0 | 152 | 4.5 [2.7-5.9] \| ×4 15.8 [12.2-22.3] (restarted 3.4 \| 12.8) |
| warm | other scripts (main) | 136 | **0** | 0 | 127 | 3.6 [3.3-4.7] \| ×4 17.1 [15.6-24.1] |
| warm | engine worker | 32 | **0** | 0 | 0 | 15.3 [13.7-18.2] \| ×4 13.2 [12.3-19.4] |

- **No script is ever served from the code cache** (0 hits in 20 traced loads, restarted rows identical; the same on the 6th reload, `--trace-reloads 6`, 5 runs warm and restarted: 0 hits, 152 + 136 + 32 events), with the SW running or restarted. `v8.produceCache` fires 320 times per load: the cache is written (after each load) and never read back. Lazy compiles: main 0 ms, engine worker 4-14 ms; background-thread streaming compile (`BackgroundCompileTask::Run`) 360-670 ms ×1 (284-1061 range) and 380-410 ms ×4, off the main thread.
- The cause suggested here (the `isolata` copy of every hit) was tested in S3 below: half right. A built `Response` is never code-cached, but storing the headed copy does not help either.
- `.wasm`, warm ×1 | ×4: main thread `FinishStreaming` 2.1 | 6.5 ms, `OnBytesReceived` 3.1 | 15.1 ms, `GetNativeModuleFromCache` 0.6 ms (pool thread), worker instantiate 0.9 ms; restarted the same within noise. The streaming compile runs on background threads outside these categories.

## S3: headers stored with the copy, hits answered unwrapped

Question: does `sw.js` lose the code cache because `isolata` builds a `new Response` for every hit? Spike (local patch, not shipped): `guardar` stores `isolata(res)`, and `isolata` returns a response that already has COEP/COOP/CORP and `Cache-Control: no-cache` as it is,
so a hit is the `cache.match` answer itself, with every header of the contract. Chromium 153 headless, 4 vCPU runner, mirror of the live site (build `6e711c8`, its `sw.js` = `main`) at RTT 0, a new browser per run.

### Control page: which service-worker answer Chromium code-caches

240 KB script plus the engine `.wasm`, its own tiny SW, loads 3-6 traced, 2 rounds each (hit = `v8.compile` with `consumedCacheSize`):

| answer to a hit | hits on loads 3-6 | `compileStreaming` of the `.wasm` (ms) |
|---|---|---|
| no SW (HTTP cache) | 8/8 | 13-25 |
| stored **network** response, returned as is | 8/8 | 27-34 |
| stored **built** response (`new Response`, the spike), returned as is | **0/8** | 20-42 |
| `new Response` per hit (`main`) | 0/8 | 22-33 |

A built `Response` has `type: "default"` and an empty `url`, also once it is stored, and Chromium does not attach a code cache to it. A response can carry COEP/CORP/`no-cache` only if it is built (GitHub Pages does not send them), so **no answer that keeps the
contract can be code-cached**. The `.wasm` gains nothing measurable in any arm: compiling it is 10-19 ms of Liftoff (`wasm-potential.md` §3), which bounds what a wasm code cache could save.

### Site: 4th load (`--warm-reloads 2`; trace = `trace-warm` 3rd load), ms from the navigation start, median [min-max]

Arms: **A** `main`; **B** the spike; **C** upper bound, which breaks the contract and is measurement only: `main`'s storage, but a hit for a script or `.wasm` is returned raw (no COEP/CORP, GitHub Pages' `max-age=600`). A and B: 10 interleaved runs (rounds 1-5 with C, rounds 6-10 A/B only); C: 5.

| arm | scenario | CPU | engine ready | `qqReady` | longest task before `qqReady` |
|---|---|---|---|---|---|
| A | warm | ×1 | 533 [402-720] | 534 [402-720] | 0 (none ≥ 50 ms) |
| B | warm | ×1 | 423 [366-483] | 424 [366-483] | 0 |
| C | warm | ×1 | 238 [206-312] | 239 [206-312] | 0 |
| A | warm | ×4 | 957 [792-1098] | 959 [794-1102] | 0 [0-53] |
| B | warm | ×4 | 854 [722-1009] | 857 [724-1013] | 0 [0-77] |
| C | warm | ×4 | 651 [567-895] | 658 [569-898] | 0 [0-53] |
| A | warm-restarted | ×1 | 554 [443-664] | 556 [444-665] | 0 |
| B | warm-restarted | ×1 | 424 [372-513] | 424 [372-514] | 0 |
| C | warm-restarted | ×1 | 276 [264-292] | 278 [265-293] | 0 |
| A | warm-restarted | ×4 | 900 [758-1000] | 902 [760-1004] | 0 [0-50] |
| B | warm-restarted | ×4 | 891 [758-1189] | 894 [761-1204] | 0 [0-63] |
| C | warm-restarted | ×4 | 739 [582-811] | 743 [586-814] | 0 |

| arm | trace | CPU | hits `L` / other / worker | eager compile ms, main | background compile ms | wasm `FinishStreaming` ms |
|---|---|---|---|---|---|---|
| A | warm | ×1 \| ×4 | 0/0/0 | 6.7 \| 36.2 | 421 [324-454] \| 423 [235-541] | 1.5 \| 5.5 |
| B | warm | ×1 \| ×4 | 0/0/0 | 6.8 \| 37.5 | 301 [246-338] \| 308 [256-887] | 1.1 \| 4.6 |
| C | warm | ×1 \| ×4 | 147/121/0 (of 151/137/32) | 4.2 \| 14.9 | 32 [26-35] \| 33 [30-40] | 1.1 \| 4.4 |
| A | restarted | ×1 \| ×4 | 0/0/0 | 6.6 \| 34.9 | 401 \| 330 | 1.2 \| 6.5 |
| B | restarted | ×1 \| ×4 | 0/0/0 | 9.3 \| 30.1 | 299 \| 295 | 1.0 \| 5.1 |
| C | restarted | ×1 \| ×4 | 147/121/0 | 3.4 \| 23.1 | 25 \| 37 | 1.2 \| 4.8 |

- **Gate of S3 not met: B has 0 code-cache hits** (as the control page predicts), so it is not adopted on that gate (adopted later for speed, below). B is still faster than A: `qqReady` −110 ms (−21 %) warm ×1, −132 ms (−24 %) restarted ×1, −102 ms (−11 %) warm ×4, −8 ms restarted ×4 (noise). The trace shows less background
  (streaming) compile work, 421 → 301 ms, with the same number of compiles. Candidate cause, not isolated: a `cache.match` body goes to the page as stored, while `new Response(res.body)` pipes every body through the SW's JS thread (~300 answers per load). Adopting B is the owner's decision.
- **C shows what the code cache is worth here**: `qqReady` −295 ms warm ×1, −301 ms warm ×4, −159 ms restarted ×4, against A. The engine worker's 32 scripts are never cache hits, not even in C (worker scripts get no code cache). C breaks the contract (`service-worker.md`): the
  scripts lose `no-cache`, and the page scripts are plain `static/<file>` URLs without a version query, so the memory cache could hand a file of the previous version to the next page. All C runs were still `crossOriginIsolated` (same-origin scripts need no CORP).

```
python3 tests_e2e/misura_avvio_sw.py --mirror A --runs 1 --rtt 0 --cpu 1,4 --scenari warm,warm-restarted,trace-warm,trace-restarted --warm-reloads 2 --out r_A_i.json   # once per arm and round, arms interleaved
```
B and C = copies of mirror A with only `sw.js` changed, keeping its `APPV`/`BIG` constants.

### S3 adopted: the final `sw.js`, with the fill really finished

The owner adopted B for its speed (via the coordinator). Two faults found on the way make the table above approximate: `misura_avvio_sw.py` did not wait for the fill (`poll()` tests `!!(expr)`, true for any promise; now `wait_fill`), and the fill never finished on the live
site: `sw-manifest.json` listed `.nojekyll`, to which GitHub Pages answers 404, so `.fill-done` was never written, the fill retried after each pause and earlier caches were never deleted (`seal_sw` now leaves it out). Remeasured with an empty `.nojekyll` in the mirrors:
**A** `main` (build `99d7ad0418`), **B** the final `sw.js` with A's constants; 10 rounds, arm order alternated, 4th load, ms, median [min-max]; paired = B − A within a round.

| scenario | CPU | A engine ready | A `qqReady` | B engine ready | B `qqReady` | paired B − A `qqReady` |
|---|---|---|---|---|---|---|
| warm | ×1 | 489 [370-615] | 489 [372-615] | 415 [339-556] | 415 [340-557] | −73 [−142-56] (−15 %) |
| warm | ×4 | 1058 [883-1385] | 1066 [886-1390] | 922 [770-1204] | 933 [776-1219] | −102 [−600-243] (−13 %) |
| warm-restarted | ×1 | 438 [367-546] | 439 [367-546] | 457 [341-567] | 457 [341-568] | +11 [−186-192] (+4 %, noise) |
| warm-restarted | ×4 | 1117 [933-1401] | 1120 [941-1404] | 946 [864-1104] | 958 [867-1114] | −179 [−456-92] (−14 %) |

No long task ≥ 50 ms before `qqReady` in either arm at ×1; at ×4 the longest is 64-84 ms in both. Migration (mirror of A, `migra.py`-style check): old worker fills 781 unheaded copies; the final `sw.js` with the same `APPV` serves them wrapped, the page is isolated, `qqReady`, no
errors, all 777 files match the manifest with `no-cache`; a deploy changing one file gives a new cache, headed 780/781 (the `.fill-done` mark is not), every file is the new version, the changed file has the new bytes, and the old cache is deleted.

