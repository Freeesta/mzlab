# Memory: choice rules, user experience and open proposals
Continuation of `memory.md` (contract, budget, slabs, tests) and `compat-safari.md` ("Memory on WebKit"): why the budget is built this way, what the user sees when memory is not enough, what remains to be built.

## Rules
- A memory refusal is first looked at upstream: too many noise features (fixed threshold below the data noise) weigh more than any ceiling; the threshold estimated from the data (`params::noise_from_hist`) lives in `Recipe::for_index`, so the site, `mzlab-cli` and the Flow steps have the same recipe.
- A bigger budget only changes the route (memory or slabs), never the bytes of the result; every phase estimates before allocating. The 4 GiB build ceiling reserves no memory.
- Rust decides budget and descent (`budget_auto`, `budget_down`) from the device facts; JS collects the facts (`deviceMemory`, cores, phone, iOS, growth probe) and writes the registry, with no hand-written MB thresholds.
- The growth probe runs in a separate instance (`rust-sonda.js`), because WASM memory does not shrink: a probe inside the engine would leave it gigabytes. It measures the browser limit, not the free RAM; beyond 2 s the level table applies.
- `measureUserAgentSpecificMemory` is not used: it requires the cross-origin isolation that GitHub Pages does not give.
- Spilling the aligned table (read randomly by `finish` and `rowmeta`) or the engine cache (rebuilt from the index) is not worthwhile.
- The Flow steps run on the same contract on the site, in the app and in `mzlab-cli flow run` (native: budget = RAM/4): same estimate per step, same route, same bytes.

## User experience
- Safe default: no question while the estimate fits in the safe budget; slabs are silent (same bytes), only the progress is visible.
- Warning before starting, once per run, when the estimate exceeds the safe budget but fits in the high one (`memoria-avviso.js`): the safe button is the default (Enter, Esc, click outside), "Use more memory" is the user's choice. Beyond the high budget the refusal carries `libero_mb` (the excess is against the free memory, not against the ceiling) and the advice "Try fewer files at once.".
- After an engine death: one line in the status, not a window ("The engine ran out of memory…; finished steps are saved; it restarts with less memory"), finished phases stay from the cache; the descent holds for the session.
- Never mute errors: a failed step shows key, numbers and advice.

## Unrealized proposals
- Unrealized proposal: a single "big permit" per origin, Web Lock `mzlab-mem-grande` (exclusive also across tabs), for a worker that uses more than the safe budget; whoever used it closes its own worker at the end of the step if it exceeds 512 MB.
- Unrealized proposal: third button "Work in parts (slower)" in the warning, where the step has slabs, and "≈ X MB · in memory/in parts" with "part N of M" on every canvas step before and during the run.
- Unrealized proposal: a single `flow::stima_passo(job, ingressi) -> {mb, via}` for CLI, desktop and site, with `mem_mb` and `via` per step in `prov/1` and `run-report/1`.
- Unrealized proposal: `Stored` and `Spill` of the web runner on OPFS files read in ranges with `createSyncAccessHandle` (worker heap during `align` ≤ one slab + one file), instead of copying all the `.mzfeatf` into the worker heap.
- Unrealized proposal: `--import-memory` fallback (the page creates the `WebAssembly.Memory` with the chosen maximum, same `.wasm`) if a browser does not instantiate a module with a 4 GiB maximum.
