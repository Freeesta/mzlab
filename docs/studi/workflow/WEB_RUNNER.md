# Flow web runner — plan and wasm64 spike

Goal: run an `mzflow/2` plan in the browser with the same Rust that `FileRunner` (`crates/mzlab-cli/src/flow*.rs`) runs natively. Nothing here is built yet; this is the plan and the spike result.

## Shape
- **Orchestrator in JS** (a new module of the Dati page, no bundler): reads the plan, asks the Rust `flow` check (already compiled for `Platform::Wasm`) for the problems, then runs the levels of `levels()` in waves, one job per `Job` of `execute`. It owns the clock, the cache (`cache_key` in OPFS) and the `prov/1` text from `prov_json`.
- **One executor per step family, lazily loaded**, each a pure function from names + bytes to a string (the 4a refactor): `match` (`flow_match`: mzlab-lib), `formula` (`flow_formula`: mzlab-feat untargeted), `level` (`flow_level`: livelli-1.json), `compare` (`flow_compare`, only the comparison, ~no engine). A step of a family not loaded yet loads its `.wasm` on the first job; `err.flow.unavailable` stays the answer for a step with no module.
- **Module split**: move the five executor files out of `mzlab-cli` into a crate with no `std::fs` and no CLI (`mzlab-flow-exec`, features `match`, `formula`, `level`, `compare`), so `mzlab-cli` and one `.wasm` per feature share the code. `match` is the one that matters for size: the spike core (parse + build + identity of mzlab-lib) is 378 KB at `opt-level=z`; the others are mostly code that the existing `attributio-all`, `feat` and lib modules already hold, so they should be added to those modules (their tetti are in `wasm-tetti.md`) rather than shipped twice. Sizes to be measured when the crate exists; no cap is touched by this plan.

## How files get in
- Plan inputs are `File`s from the file picker or a dropped folder (File API); they are copied once to OPFS and hashed (`sha256` of the plan input must match, as `fill_inputs` does).
- Executors get `Uint8Array`s read from OPFS and write their string result back to OPFS under its `sha256`; the artifact table is the `Artifact{sha256,bytes,ty,label}` of the core. Intermediate files never leave the browser.
- Small steps (`match`, `formula`, `level`, `compare`) read whole files. `convert.mzml` and `features.table` need the scan readers and the index: they already run in the engine worker (`rust-worker.js`, mzML and `.mzidx` in OPFS), so the orchestrator calls the engine routes for them instead of the executors.

## Desktop only
- `convert.mzml` of vendor files that are too large for the 4 GiB wasm32 memory, `features.table` over a long series, and anything that wants more than the per-module budget (`docs/agenti/memoria.md`): the plan checks `Platform::Wasm` and says `err.flow.unavailable`; the same plan runs unchanged in the desktop app or `mzlab-cli`.
- `meta.compare` runs its variants one after the other in the browser (memory adds up); the native runner may use threads.

## wasm64 (memory64) spike — result: no-go for now
One executor core (parse an MSP library, build it, `identity` search of 100–200 queries) built from the same source for both targets and run in Chromium 141:

| | wasm32 (stable 1.97, prebuilt std) | wasm64 (nightly, `-Zbuild-std`) |
|---|---|---|
| size, `opt-level=z`, lto, no wasm-opt | 378 200 B | 405 052 B (+7 %) |
| 5 000 library spectra, 100 queries, best of 5 | 177 ms | 210 ms (+19 %) |
| 40 000 library spectra, 200 queries, best of 5 | 1 525 ms | 1 959 ms (+28 %) |
| memory after the run | 210 MB | 242 MB |

- **Works**: `wasm64-unknown-unknown` builds the unchanged crate on nightly, and Chromium 141 runs it. Node 22 refuses the module (the 64-bit function table of rustc output is not supported there), so the Node-based tests would need a newer Node.
- **Costs**: tier-3 target, nightly and `build-std` in CI; the engine is slower because memory64 loses the guard-page bounds checks; pointers double.
- **Open**: Safari and Firefox support could not be confirmed (sources disagree), so the site would need a wasm32 fallback anyway; `wasm-bindgen` support for the target was not found, and the bridge crates use it; allocation above 4 GiB was not tried.
- **Recommendation**: stay on wasm32. The steps that need more than 4 GiB are the vendor conversion and the long feature tables, which are desktop-only in this plan; the identification executors work on files of megabytes. Revisit when memory64 ships in Safari and Rust has it on stable.
