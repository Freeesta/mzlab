# Rust engine (`crates/`)
Editing rule: this file states how the Rust engine is now, verified against the code; rewrite the line a change makes false, never add history, dates or PR numbers.
Keep it in English and under 16 000 bytes: a new module gets its detail in its own header or area file, here at most a word in the crate line.

Module-level detail lives in the code (module headers) and in the area files listed at the end; to find a module: `python3 tools/indice.py` then `grep -n '<name>' docs/agenti/mappa.md`.

## What runs where
- One engine, two targets: the site runs the crates as `.wasm` inside Web Workers (`static/wasm*/`); the desktop app and `mzlab-cli` run the same crates natively (mmap, threads, no wasm caps). Every function exists in both, with a byte-for-byte parity test (AGENTS.md §1).
- Python (`mzlab/`, `tools/*_riferimento.py`, `tools/golden.py`) is only the reference of the parity tests: a new computation is born in Rust; Python changes only to fix a bug the parity found.
- Rust returns i18n keys and parameters (`error.rs`, JSON `{error, error_key, params}` at the bridge), never user text.

## Crates (workspace in `Cargo.toml`; `vendor/opentfraw` is excluded)
- `mzlab-core`: the engine. mzML reader (`mzml*.rs`, quick-xml, base64, flate2 with `zlib-rs`), mzML writer (`mzmlwrite/`), `.mzidx` builder and readers (`mzidx*.rs`, `idx*.rs`), every `api/...` route of the page (`*route*.rs`, `fileroutes.rs`, `hrroutes.rs`, `lr.rs`, `iteminfo.rs`, …).
- `mzlab-core`, continued: chemistry (`elements.rs`, `composition.rs`, `isoscore.rs`, `subformulas.rs`), statistics (`stats.rs`, `statmat.rs`, `pca.rs`), centroiding (`centroidi/`), maps, the MSn tree and graph (`msnalbero.rs`, `msntree.rs`, `msngrafo/`), method readers (`metodo*.rs`), the memory budget (`budget_auto.rs`), the Flow step descriptors (`step/`).
- `mzlab-core` features: `thermo` (default; the `.raw` reader via `vendor/opentfraw`), `precursori`, `mappa-dad` (default), `estesi`, `sciex`, `valida`, `indice`. The engine `.wasm` (`mzlab-wasm`) turns off `thermo` and `estesi`: vendor readers reach the page through the readers modules (`lettori-modulo.md`).
- `mzlab-picchi`: peak picking, signal processing, compact sorts (`sortc`); its own crate so it builds at opt-level 3. `mzlab-core` re-exports its modules under the same paths (`mzlab_core::sortc`, …).
- `mzlab-wasm`: wasm-bindgen bridge of `mzlab-core` (`Engine`, one slot per opened file); runs ONLY in a Web Worker (`rust-worker.js`).
- `mzlab-lettori` + `mzlab-lettori-wasm`: vendor-format readers, one `.wasm` per family (`aperti`, `andi`, `thermo`, `agilent`, `sciex`, `waters`, `shimadzu`, `bruker`, `valida`), loaded on demand; never in the engine `.wasm` (`tests/test_lettori_wasm.py`). See `lettori-modulo.md`.
- `mzlab-lib` + `mzlab-lib-wasm`: spectral libraries (MSP/MGF/MassBank/JSON parsers, `.mzlib`, Flash indexes, scores); used only by `libreria-worker.js` (`librerie.md`).
- `mzlab-feat` + `mzlab-feat-wasm`, `-table-wasm`, `-grafo-wasm`, `-import-wasm`, `mzlab-attributio-wasm`, `mzlab-finder-lr-wasm`: feature detection, tables, mzFinder and Attributio (`mzfeat.md`, `attributio*.md`, `wasm-tetti.md`).
- `mzlab-msngrafo-wasm`, `mzlab-msnregole-wasm`: the MSn fragmentation graph and its structure rules, separate `.wasm` loaded on first use (`msn-grafo.md`).
- `mzlab-chem` + `mzlab-chem-wasm`: 3D geometry from SMILES and reactivity (`reatt/1`), administrator mode only.
- `mzlab-mol`: the `mol/1` structure record (SMILES/CXSMILES, molfile, deterministic JSON, core hash); in no `.wasm` (`mol.md`).
- `mzlab-target`: target and suspect lists (RFC 4180 CSV). `mzlab-llm` + `-wasm`: the assistant's tool catalog and streams.
- `mzlab-cli`: the native command line and the Flow runner (`flow.rs` registry, one `flow_*.rs` per step; `cli.md`, `cli-passi.md`, `rust-flow.md`, `rust-step.md`). Never in a WASM graph.
- `mzlab-desktop`: the Tauri v2 app, the page behind `mzlab://` with a native engine (`rust-desktop.md`).

## Rules enforced by CI and tests
- **Lint** (`python3 tools/controlla_rust.py`, job `rust-lint` of `rust.yml`): clippy on the whole workspace (`--all-targets`) and `cargo fmt --check` against `tools/clippy_base.json`, which can only shrink. A new lint in a file, or a file fmt no longer accepts, is red. `--aggiorna` rewrites the base and refuses if anything got worse; `--solo-fmt` is the fmt half (pre-push hook via `pre_pr.py`).
- Parsing modules carry `#[warn(clippy::unwrap_used, clippy::indexing_slicing, clippy::cast_possible_truncation)]` on their `mod` line in `mzlab-core/src/lib.rs`; a new parsing module gets the same line.
- `cargo machete` (same job): a dependency no source names is removed from its `Cargo.toml`.
- **No std sorts in `mzlab-core`** (`crates/mzlab-core/clippy.toml` disallows `slice::sort*`: ~2.5 KB of `.wasm` per call site). Use `sortc::slice_by`, `slice_by_key`, `slice_ord`; floats with `sortc::asc_nan_last` / `desc_nan_last` (NaN last), not `partial_cmp(..).unwrap_or(Equal)`. `mzlab-feat` and `mzlab-chem` use the `sortc::SortC` trait methods (`sortc_by`, `sortc_by_key`, `sortc`); `mzlab-lib` and `mzlab-feat-wasm` use `mzlab_picchi::sortc` (`by`, `any_by`, `SortC`).
- The std sorts that stay carry `#[allow(clippy::disallowed_methods)]` and the reason: measured hot spots (std ~2× faster) and the `partial_cmp(..).unwrap_or(Equal)` cases where a NaN makes the order partial and changing the algorithm would change results (`statref.rs`, `statmat.rs`, `pca.rs`).
- **WASM graph is pure Rust**: `rust-wasm` fails if `cargo tree -e no-dev --target wasm32-unknown-unknown` of `mzlab-wasm`, `mzlab-lib`, `mzlab-lib-wasm`, `mzlab-feat`, `mzlab-feat-wasm`, `mzlab-feat-table-wasm` or `mzlab-feat-grafo-wasm` contains `memmap2`, `zstd-sys`, `libz-sys` or `cc`. Zstandard is always `ruzstd`; `memmap2` only in `mzlab-desktop`.
- **No relaxed SIMD** in build files (`tests/test_guardie_regole.py`). Every `.wasm` is built with `+simd128` (`.cargo/config.toml`), one binary with no fallback; the page checks support with `WebAssembly.validate` in `capacita.js`.
- **Mass constants** `ELECTRON`, `PROTON`, `PROTON_6DP` (the rounded value of the golden data and the Python reference) and `C13_STEP` are written only in `mzlab-core/src/elements.rs`; other files import them by name (`test_guardia_costanti_di_massa_solo_in_elements`).
- **Memory** (`memoria.md`, `tests/test_guardie_memoria.py` in `rust-wasm-guardie`): the peak linear memory of each phase on the synthetic series stays under `tools/guardie_memoria.json` + 10 %; an exported WASM function returning a whole `Vec<u8>` is refused unless listed in `tools/api_byte_ammesse.txt` (new outputs write to a `WriteAt` sink; `tools/controlla.py`).
- **Size caps** of every `.wasm` (bytes after `wasm-opt`, via `tools/wasm_opt.sh`): the `limit_*` variables of `tools/build_feat_wasm.sh` and `tools/build_lettori_wasm.sh`, the library cap in the build steps of `pages.yml`; numbers and margins in `wasm-tetti.md`. A build over its cap is red.
- **Determinism**: the same synthetic Flow gives the same SHA-256 natively and as WASI (`tests/test_flow_determinismo.py`, job `rust-det`).
- **Dependencies**: `cargo deny` (`deny.toml`: licences and sources) blocks when manifests, lockfile, `vendor/` or `deny.toml` change; RustSec advisories run daily in `advisories.yml`, outside the PR checks.
- **`unsafe`**: `#![forbid(unsafe_code)]` in `mzlab-core`, `mzlab-wasm`, `mzlab-lib`, `mzlab-lib-wasm`, `mzlab-feat`, `mzlab-feat-wasm`, `mzlab-feat-table-wasm`, `mzlab-attributio-wasm`; the only `unsafe` in the workspace is `Mmap::map` in `mzlab-desktop/src/mapped.rs`.
- Readers and converters (`mzlab-lettori`, `mzlab-core::mzmlwrite`, CLI, `vendor/opentfraw`) may use `unsafe` only in a small isolated module (SIMD, mmap, decoding), never in the browser build: crate `forbid` → `deny`, module `#[allow(unsafe_code)]`, and per block a `SAFETY:` comment, a test, a fuzz target and a comparison with the safe version.
- `rust-ok` is the only required name for the protected `main`: it is green when every Rust job passed, or when `changes` found no Rust path (then the jobs are skipped). `rust.yml` has no path filter for this reason.

## Build profiles and memory
- Toolchain pinned in `rust-toolchain.toml` (with the `wasm32-unknown-unknown` target); `Cargo.lock` is committed. `.wasm` and `pkg/` are never committed: `pages.yml` builds them into `site/static/`.
- `release`: `opt-level = "z"`, LTO, one codegen unit, `panic = "abort"` (a panic kills the instance; the JS supervisor restarts the worker, and two deaths in a minute make the page ask for a reload). `mzlab-core`, `mzlab-picchi`, `quick-xml`, `base64`, `zlib-rs`, `miniz_oxide` build at opt-level 3 (measured with `tools/banco_wasm.py`). `desktop` and `cli` profiles inherit `release` at opt-level 3.
- WASM memory: 32 MB initial, 4 GiB maximum (`.cargo/config.toml`; it reserves nothing). The working budget is chosen by `mzlab-core::budget_auto` (`memoria.md`). Large allocations use `try_reserve` and fail with an i18n key, never abort.
- Numbers: m/z `f64`, intensity `f32` (an array is read only at the width it needs), entropy `f64`.

## Parity with the Python reference
- Pattern: `crates/<crate>/tests/<name>_parity.rs` compares the Rust output with `tools/<name>_riferimento.py` (JSON compared leaf by leaf, tolerance stated in the test). Without python3 and numpy the test is skipped; `MZLAB_GOLDEN_STRICT=1` turns the skip into a failure.
- Small references are committed in `tests/golden/ref/` (`componenti`, `composizione`, `composizione_rotta`, `msntree`, `msntree_rotta`, `statistica`, `statmat`, `stats`) and need no python3; rewrite them with `python3 tools/crea_golden_ref.py [name]` only after a deliberate change of the reference. The big ones (maps, origin, example indexes) are written by the test into a temporary folder.
- Golden reader data: `crates/mzlab-core/tests/golden.rs` against `tests/golden/` (edge indices identical, then areas 1e-4, m/z 1e-6, entropy 1e-9). `python3 tools/golden.py --controlla` checks, `--sintetici DIR` writes the synthetic mzML the Rust tests read, `--crea` only when the reference changes on purpose (it also rewrites `esempio_*` with format-only differences: commit only the file you meant).
- A value out of tolerance is different logic: fix the logic, never widen the tolerance.
- Traps for bit-identical results: Python ≥ 3.12 `sum()` on floats is Neumaier-compensated, so where Python uses `sum()` Rust uses `pysum`; `ndarray.sum()` is pairwise (`stats.rs`); numpy's `default_rng(seed)` is reproduced bit for bit by `rng.rs` (`Pcg64`, SeedSequence, Lemire bounded ints); `jsonb.rs` writes NaN as `null`; rounding is `elements::round_half_up`; `statmat::svd`/`eigh` match numpy up to the sign of eigenvectors, which the caller fixes.
- Tests without a Python reference (e.g. `peakq.rs`) prove themselves on synthetic traces in their `tests` module.

## Speed
- A speed gain counts, even a few percent, when measured beyond the noise (repeated, median, same result) with small readable code: the PR gives the % on the step and on the total and adds a row to `misure-velocita.md`.
- A dependency for speed is accepted if it gains ≥ 3 % measured on its target, has a permissive licence (`cargo deny`) and the PR shows the Cargo lines and the before/after numbers.
- Load time of a file: `cargo run --release --example carico -- FILE.mzML`; other timing examples sit in `crates/mzlab-core/examples/*_tempi.rs`; WASM benches across browsers: `tools/banco_wasm.py` (`banco-wasm.yml`).

## Build and test
- Native: `cargo test --release -p mzlab-core` (or `-p <crate>`); CI splits the workspace into `rust` (mzlab-feat), `rust-core` (core, lettori, lib, mol, target), `rust-misc-a` (Attributio, chem, llm), `rust-misc-b` (the rest).
- WASM: `cargo build --release --target wasm32-unknown-unknown -p mzlab-wasm`; check the graph with `cargo tree -e no-dev --target wasm32-unknown-unknown -p mzlab-wasm`.
- Before pushing: `python3 tools/pre_pr.py`; full: `python3 tools/verifica.py` (`--senza-e2e` without the browser).
- The site with the real `.wasm`: `sito-rust.yml` (PRs touching `crates/`, the page or the build; by hand with `workflow_dispatch` and `solo=e2e_lr,e2e13,…`). It is not required, and it is the engine's test bench when the container cannot build the `.wasm`.
- Page-side debugging: `MZLAB_TRACE` (an array defined on `window` before loading) records rpc and fetch with timings; every route served by `lr.js` gives the same JSON as Python (`e2e_lr.py`).

## Area files
- Engine in the page, `api/...` routes, Thermo `.raw`: `rust-rotte.md`. `.mzidx` builder and the site's worker pool: `rust-mzidx.md`, format in `mzidx.md`.
- Readers modules: `lettori-modulo.md`. Memory and budget: `memoria.md`. `.wasm` caps: `wasm-tetti.md`. Measures: `misure-rust.md`, `misure-velocita.md`.
- Desktop app: `rust-desktop.md` (mzFinder native: `rust-desktop-finder.md`). Flow: `rust-flow.md` (`mzflow/2`, `prov/1`), `rust-step.md` (`step/1`), `cli.md`.
- `mol/1`: `mol.md`. Libraries: `librerie.md`. Features: `mzfeat.md`. MSn graph: `msn-grafo.md`.
