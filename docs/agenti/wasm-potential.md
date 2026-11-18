# WebAssembly potential: what we build, what we leave on the table
Target: up-to-date desktop browsers (Chrome/Edge, Firefox, desktop Safari) and the desktop app; no phones or iPads.

## 1. Summary
The site's bottleneck is size, not compatibility. `opt-level = "z"` stays the `release` profile of the `.wasm` files, with `opt-level = 3` on the hot crates (§2): on real code (Chromium 153, 4 vCPU) O3 reads an HR mzML 1.5× faster, indexes it 1.4× faster and does peak picking 2.7× faster, with bit-identical results; O3 on everything would cost +32-37 % on the engine.
SIMD128 is worth +5-15 % (all target browsers support it), zlib-rs instead of miniz 1.5-2× on inflate; both in use. Module compile times are negligible (10-19 ms per module, even at O3). Wasm threads, memory64 and relaxed SIMD are not worth it (§7).

## 2. How we build (audit)
- **Toolchain**: stable Rust 1.97.0 (`rust-toolchain.toml`), only `wasm32-unknown-unknown`, no nightly and no `-Zbuild-std`. `wasm-bindgen` 0.2.129 (`--target web`). `wasm-opt` = **binaryen version_133** from the official GitHub tarball with a pinned SHA-256 (`tools/wasm_opt.sh`; fallback npm `wasm-opt@1.4.0` = version_112 only if the tarball is unusable).
- **Profile** (`Cargo.toml`): `release` = `opt-level = "z"`, `lto = true` (fat), `codegen-units = 1`, `panic = "abort"`; per-package `opt-level = 3` on `mzlab-core`, `mzlab-picchi`, `quick-xml`, `base64`, `zlib-rs`, `miniz_oxide`, `sha1`, `sha2`, `opentfraw` (each entry with its measurement in the comment). No `strip` (not needed: `wasm-opt` removes the `name` section, 352 KB out of 850 KB in the bench; no DWARF). `desktop` and `cli` inherit with `opt-level = 3`, but do not build `.wasm`.
- **wasm-opt**: `-Oz --enable-bulk-memory --enable-sign-ext --enable-mutable-globals --enable-nontrapping-float-to-int --enable-reference-types --enable-multivalue --enable-simd`; the readers add `--converge --low-memory-unused --vacuum --dae …` (−2.5 %).
- **Active target features** (`rustc --print cfg --target wasm32-unknown-unknown`, Rust 1.97): `bulk-memory`, `multivalue`, `mutable-globals`, `nontrapping-fptoint`, `reference-types`, `sign-ext` (target defaults, not chosen by us). **Turned on by `.cargo/config.toml`**: `simd128` (`+simd128`, all modules). **Off**: `extended-const`, `tail-call`, `exception-handling`, `relaxed-simd`, `atomics`. No `-C target-cpu`. `.cargo/config.toml` sets initial memory 32 MB, maximum 4 GiB and `+simd128`.
- **zlib-rs**: backend of `flate2` in `mzlab-core`, `mzlab-lettori`, `mzlab-cli`, `mzlab-desktop` (read and write; `conv/1` declares `compressore`).
- **Loading**: `rust-supervisor.js` `compileOnce` uses `WebAssembly.compileStreaming` and passes the same `WebAssembly.Module` to the workers (engine, `idx-worker.js` pool); the `sw.js` service worker keeps the files in cache (cache first, per site version). Files are read from OPFS with a read-only `SyncAccessHandle` (`lettura-sync.js`), one JS → wasm memory copy per block.
- **Parallelism**: no `SharedArrayBuffer` (no COOP/COEP on GitHub Pages); message-based worker pools (`idxpool`, `createFeatPool`, size from `hardwareConcurrency`). The desktop app uses the **native** engine (`rayon`, `opt-level = 3`): the levers in this file concern only the site.
- **Split and ceilings** (`wasm-caps.md`): one module per group, loaded on demand; ceiling = measurement × 1.10, only downward. The ceilings keep `z` as the base: that is where speed is lost.

## 3. Measurements
Bench: a test crate outside the repository with the real `mzlab-core` functions (`mzml::read`, `upload::prepare` + `mzidx::build_mzml_to_vec`, `Routes::xic_raw`, `centroids::centroids`, `peaks::detect_peaks`, `composition::compose`, `entropy::entropy_similarity`, `flate2` inflate). Files: synthetic profile HR mzML of 87 MB (1200 MS1 × 400 profile peaks, 900 MS2, zlib) and `B_FullMass-t15` from `tools/dati_sintetici.py` (2.5 MB, LR). Median of 5 runs (Chromium) or 3 (Node 22). Outputs bit-identical in all variants (checksums to 17 digits).

**Chromium 153, ms** (base = `z` + `-Oz` on everything):

| work | base | O3 | O3+SIMD | O3+SIMD+zlib-rs | gain |
|---|---|---|---|---|---|
| HR mzML read 87 MB | 1146 | 778 | 671 | 618 | ×1.86 |
| HR `.mzidx` index | 4542 | 3278 | 3002 | 2744 | ×1.66 |
| LR mzML read | 19.8 | 13.2 | 11.1 | 10.3 | ×1.9 |
| LR index | 54.8 | 41.8 | 38.5 | 38.2 | ×1.43 |
| 50 XIC + peaks (AsLS) LR | 223 | 83 | 81 | 80 | ×2.8 |
| centroids 100 HR scans | 2737 | 2499 | 2527 | 2527 | ×1.08 |
| 200 elemental compositions | 3724 | 3419 | 3424 | 3315 | ×1.12 |
| inflate 64 MB | 621 | 416 | 396 | 307 | ×2.0 |
| 200 XIC from the HR index | 51 | 51 | 49 | 51 | = |
| entropy 300×300 MS2 | 1159 | 1186 | 1129 | 1277 | = |

**Node 22 (older V8), ms**: base → O3: HR read 1599 → 979, HR index 6209 → 3623, centroids 6823 → 2356 (×2.9), LR peaks 301 → 89, compositions 6792 → 3539, inflate 822 → 426; zlib-rs alone on `z`: inflate 822 → 353, HR index 6209 → 4446. Chrome 153's V8 recovers part of the `z` gap (wasm inlining); a less aggressive engine (Firefox, Safari: not measured, not installed here) is probably between the two.

**Size** (bytes after `wasm-opt -Oz`, gzip -9 in parentheses):

| module | `z` | O3 on everything | O3 only on hot crates¹ |
|---|---|---|---|
| `mzlab-wasm` engine | 1 765 995 (766 236) | 2 419 539 (+37 %) | 2 336 860 (+32 %) |
| HR `mzlab-feat-wasm` | 1 287 826 (564 255) | 1 557 979 (+21 %) | 1 324 332 (+2.8 %) |
| Attributio `all` | 1 392 870 (599 771) | 1 928 195 (+38 %) | 1 403 170 (+0.7 %) |
| bench (hot code only) | 451 523 | 534 719 (+18 %) | 534 330 |

¹ `[profile.release.package.<crate>] opt-level = 3` for `mzlab-core`, `miniz_oxide`, `quick-xml`, `base64`: in the bench it gives the same speed as O3 on everything; in the engine almost all the code is `mzlab-core`, so it weighs the same as O3.
- **`wasm-opt` level**: does not change speed (O3 + `-Oz` = O3 + `-O3` within noise, 535 vs 537 KB): only rustc's `opt-level` matters. Keep `-Oz`.
- **O2 / Os**: O2 like O3 (slower centroids, 547 KB); Os in between (499 KB, HR read 1168 ms in Node). Not worth it.
- **SIMD128 (auto-vectorization)**: −1.5 % size with `z`, +0.6 % with O3; +5-15 % on read and index, none on the rest. Consistent with `ottimizzazioni-tentate.md` (22/10, 20 LR Full Scan: 9.1 vs 9.3 s): the gain is small but free.
- **zlib-rs** (`zlib-rs` feature of `flate2`, already in `Cargo.lock` 0.6.8): +42-43 KB (+8 %) on the bench; inflate ×1.35-2.3, HR index −9 % (Chromium).
- **SHA crates** (`sha1`, `sha2` at O3 in the `release` profile; `upload::Passata` fingerprints of a file while it is copied into OPFS): SHA-256 + SHA-1 on 64 MB ×1.68 in Node (774 → 461 ms) and ×1.34 in Chromium 153 (654 → 489 ms), median of 9, `tools/banco_wasm.py` variant `sol_hash`, identical check value; ±1 KB per module. With 300 MB in Chromium the hash (2030 ms) remains slower than the copy into OPFS (1511 ms): copy + hash 2156 ms, so the copy waits for the hash.
- **binaryen 132/133 vs 112**: same size (−0.1 %), same speed, `wasm-opt` **8× faster** (1.5 vs 12.8 s for the bench).
- **Compile and instance** (Chromium 153, `compileStreaming`, median of 5, cold, no HTTP cache): engine 14-19 ms, HR 11-14 ms, Attributio 11-14 ms at `z` and at O3; synchronous `new WebAssembly.Module` 4-8 ms; instance < 1 ms. V8 compiles with Liftoff and optimizes in the background: the "heavy module compile cost" does not exist at our sizes; only the download matters (+205-225 KB gzip for the engine at O3, once: then `sw.js`).
- **Worker pool** (Chromium, 4 vCPU, O3 module, independent jobs): 1, 2, 4 workers in 444, 449, 456 ms (scales ×4), 8 workers 941 ms. Parallelism across files is already there and scales linearly up to the cores.

## 4. Browser support (webassembly.org/features.json, October 2026)
| feature | Chrome | Firefox | Safari | for us |
|---|---|---|---|---|
| bulk memory, sign-ext, mutable globals, nontrapping, multivalue, reference types | ≤ 96 | ≤ 79 | 15 | already on (target defaults) |
| SIMD128 | 91 | 89 | 16.4 | yes |
| extended-const | 114 | 112 | 17.4 | yes, free, no gain |
| tail calls | 112 | 121 | 18.2 | useless (Rust does not generate them by itself) |
| exception handling (exnref) | 137 | 131 | 18.4 | no: needs `panic = "unwind"` → `-Zbuild-std` |
| relaxed SIMD | 114 | 145 | flag only | **no**: results differ across CPUs (FMA, min/max, dot) → breaks determinism |
| threads + atomics | 74 | 79 | 14.1 | the browser has it; COOP/COEP and stable Rust are missing (§7) |
| memory64 | 133 | 134 | Technology Preview only | no |
| multi-memory | 120 | 125 | no | no |
| JS String Builtins | 130 | 134 | 26.2 | not used by `wasm-bindgen` 0.2.129 |
| WasmGC | 119 | 120 | 18.2 | **not needed by Rust** (own linear memory) |

**SIMD128 alone sets the minimum**: Chrome/Edge ≥ 91, Firefox ≥ 89, Safari ≥ 16.4; the declared minimum browsers are in §8.

## 5. Levers in use
- **SIMD128**: `+simd128` compilation on all modules, a single binary; LLVM auto-vectorization does not reassociate floating-point sums without fast-math: identical outputs (measured). A browser without SIMD128 shows `error.noSimd` (`capacita.js`).
- **O3 on the hot crates** (§2): the biggest lever (§3); the price is size and ceilings, not compilation.
- **zlib-rs** (`flate2` 1.1.10, `zlib-rs` 0.6.8): inflate ×1.35-2.3, HR index −9 %, +43 KB per module that reads mzML.
- **binaryen 133**: same size as 112, `wasm-opt` 8× faster.
- **Packages**: `quick-xml` 0.42.0 and `base64` 0.23 are the latest; `serde_json` 1.0.151; `ruzstd` 0.9.1 for Bruker; `rayon` native only.

## 6. Open levers
- **Real LTO on the site** (§10): with `crate-type = ["cdylib", "rlib"]` cargo discards `lto = true`; `cargo rustc --crate-type cdylib` restores it. Measured in the bench (`lto1`, `lto16`), not adopted: HR mzML read −12 % with `lto16` in Chromium, but "200 XIC from the HR index" +9-27 %.
- **O3 on the readers**: among the readers only `opentfraw` is at 3; size and speed of `mzlab-lettori` at O3 still to be measured (`wasm-caps-measurements.md`).
- **One large file across several workers**: across files the pool already scales ×4; within ONE large file, without threads, the index could be split by scan ranges across the pool workers (separate study if real files call for it).

## 7. Discarded paths
Wasm threads, memory64, relaxed SIMD, exception handling, explicit SIMD, `DecompressionStream`, double build, O2/Os, `wasm-opt` levels other than `-Oz`, `wasm32v1-none`, `-Zbuild-std`, `Module` cache in IndexedDB, `simd-json`: one line each, with the numbers, in `ottimizzazioni-tentate.md`, section "WebAssembly: leve scartate".

## 8. Decisions in force
- **One build only, the fast one**: SIMD128 on, `opt-level = 3` where time is spent, recent binaryen `-Oz` for size, no fallback. Two builds would cost a second round of `sito-wasm-*` (~600 s of CPU in CI per PR), double the bytes in `site/` and double the e2e tests.
- **Declared minimum browsers** (the only place that fixes them): Chrome/Edge 111, Firefox 121, Safari 16.4. The engine requires SIMD128, OPFS and synchronous handles (`capacita.js`), `DecompressionStream` needs Firefox 113, `:has()` Chrome 105 and Firefox 121. Below the floor the code has no fallbacks for `ResizeObserver`, `IntersectionObserver`, `MutationObserver`.
- **Ceilings**: in `wasm-caps.md` (measurement × 1.10 rounded to 4 KiB); raising them for O3 requires the measurement in the same PR.
- **binaryen from GitHub** instead of npm (`wasm-opt@1.4.0` is frozen since 2023; `wasm-opt@2.0.1` has per-platform packages): `version_133` tarball with pinned SHA-256.

## 9. How to redo the measurements
The bench (`tools/banco_wasm.py`, crate `tools/banco_wasm/`) builds the same crate in the variants `base` (`z`, `-Oz`), `o3`, `o3simd`, `o3simd_zrs`, `mix` and runs it in Node (`initSync`) and in Chromium (Playwright, `compileStreaming`) with the synthetic 87 MB HR mzML generated by `mzmlwrite` (fixed seed) and `B_FullMass-t15` from `dati_sintetici.py`. Each variant must give the same checksums; timings are compared only on the same machine, median of 5 runs.
In the cloud the bench does not run (`tools/wasm_opt.sh` only in CI): the `banco-wasm.yml` workflow (`workflow_dispatch` only, inputs `varianti` and `giri`) launches it in CI on a branch and writes the timing table in the run summary; one branch at a time (to compare two branches, launch it twice).

## 10. Link with real LTO
With `crate-type = ["cdylib", "rlib"]` cargo discards `lto = true`: the site modules are linked without LTO. `cargo rustc --crate-type cdylib` builds only the cdylib and LTO runs. The bench has the variants `lto1` and `lto16` (same profile and same `-O3` as `site_z`, 1 or 16 codegen units), to compare with `site_z`.
Measurements (4 vCPU, Chromium 153 and Node 22, median of 5 runs, same checksums in all variants; ms, base = `site_z`). Bench size after `wasm-opt -O3`: 656 055 B → 600 914 (`lto1`) → 579 129 (`lto16`).

| work | Chromium base | `lto1` | `lto16` | Node base | `lto1` | `lto16` |
|---|---|---|---|---|---|---|
| HR mzML read | 742.7 | 757.9 | 651.2 | 837.7 | 840.5 | 816.8 |
| HR index | 3376.3 | 3428.9 | 3227.6 | 3670.3 | 3658.6 | 3522.8 |
| 200 XIC from the HR index | 22.9 | 27.3 | 25.0 | 23.4 | 29.8 | 26.1 |
| centroids 100 HR scans | 1076.3 | 1048.3 | 1068.5 | 1069.3 | 1034.6 | 1080.8 |
| 50 XIC + HR peaks | 24.2 | 21.7 | 21.8 | 20.7 | 21.5 | 21.4 |
| entropy 300×300 MS2 | 611.8 | 636.7 | 612.7 | 659.6 | 612.7 | 619.2 |
| LR mzML read | 19.5 | 18.3 | 15.6 | 20.7 | 18.4 | 19.8 |
| LR index | 51.0 | 42.8 | 45.2 | 47.7 | 44.0 | 47.4 |
| 200 XIC from the LR index | 2.8 | 2.5 | 2.7 | 2.7 | 2.7 | 2.9 |
| 50 XIC + peaks (AsLS) LR | 40.9 | 37.8 | 58.0 | 48.8 | 46.4 | 44.5 |
| 200 compositions | 2683.8 | 2707.4 | 2753.9 | 2682.5 | 2717.7 | 2692.3 |
| inflate 64 MB | 333.1 | 330.7 | 305.5 | 327.0 | 314.1 | 336.6 |
| SHA-256 + SHA-1 64 MB | 492.3 | 517.7 | 563.7 | 555.5 | 505.9 | 557.2 |

"200 XIC from the HR index" is slower than `base` in both variants and in both engines (+9-27 %); jobs under 30 ms fluctuate by a few ms between runs.
