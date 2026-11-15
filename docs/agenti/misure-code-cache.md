# Code cache of the scripts and the engine `.wasm` (site only)

Site only: the desktop app serves `mzlab://` locally and has no service worker, so none of this applies there. Tool: `tests_e2e/misura_avvio_sw.py` (setup and scenarios in `misure-caricamento.md`, «Restarted service worker»).

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
- A plausible cause, **not tested here**: the SW answers every script with a new `Response` built by `isolata` (`sw.js:26-33`, also for cache hits, line 194), which drops the Cache Storage marker the browser needs to store a code cache for a SW-served script. A test would serve a script straight from the cache once; that changes a product file and is the owner's decision (S3/S4 wait for this).
- `.wasm`, warm ×1 | ×4: main thread `FinishStreaming` 2.1 | 6.5 ms, `OnBytesReceived` 3.1 | 15.1 ms, `GetNativeModuleFromCache` 0.6 ms (pool thread), worker instantiate 0.9 ms; restarted the same within noise. The streaming compile runs on background threads outside these categories.
