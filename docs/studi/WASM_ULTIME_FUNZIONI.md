# Latest WebAssembly features and toolchain for mzLab (study)

Study chat, 29-30 Oct 2026, base `origin/main` `f362c83`. Federico's rule (29/10): everything also runs in the browser, with the latest WebAssembly features and current builds; size caps are never raised. Builds on `docs/agenti/wasm-potenziale.md` (profiles, SIMD128, binaryen, zlib-rs) and `docs/studi/wasm-potenza.md` (browser APIs, isolation, OPFS): what those already settled is not repeated. Build prompts: `docs/studi/prompt_wasm/`.

## 1. Answer
- **The feature set is already right.** Every feature the Rust toolchain can use on stable is on (SIMD128 + the default set: bulk memory, sign-ext, non-trapping float-to-int, multi-value, reference types, mutable globals). The newer Wasm 3.0 items either do not apply to Rust on stable (GC, exnref, tail calls, JS string builtins), break determinism (relaxed SIMD) or are missing in Safari (memory64, relaxed SIMD). Measured: tail calls + extended-const change the engine by +118 B and `hr` by +158 B, nothing else.
- **The levers left are in the optimiser, not in new opcodes** (all measured, same check values in Node, Chromium, Firefox, WebKit):
  1. `wasm-opt -O3` instead of `-Oz` (same rustc output): the 200 formula compositions take 34-35 % less time in Firefox and WebKit (2 rounds, 8 runs), neutral in Chromium and Node; every module grows 0.2-1.8 % and stays under its cap.
  2. `opt-level = 3` on the parsing crates (`quick-xml`, `base64`, `zlib-rs`, `miniz_oxide`): mzML read −13 to −28 %, HR index −6 to −24 % (§4); +6-13 KB per reader, +12 KB engine, all modules still under their caps (§3).
  3. **Std sorts**: after `-Oz`, `slice::sort*` instances are 106 KB of the engine, 136 KB of `attributio-all`, 66 KB of `hr`+formula. Moving them to `mzlab_core::sortc` (already used by mzFinder) is the largest size reserve found.
- **`formula.ms1` fits in the browser without raising a cap**: a lazy module of its own (`mzlab-feat-wasm --features flow_formula`) is 403 567 B, under the standard 512 000 B limit of the group. Inside `hr` it is 1 652 566 B, 46 934 B over the cap: the sort reserve above could also close that gap.
- **Not now**: memory64, threads in Rust (`-Zbuild-std`), relaxed SIMD, opt-level `s`, Rust 1.99 for speed (neutral), Wasm compilation hints (behind a flag), brotli (needs another host).

## 2. Feature matrix (sources §9; first stable version; "flag" = not on by default)
| feature | Chrome | Firefox | Safari | Rust stable can emit | mzLab |
|---|---|---|---|---|---|
| MVP + mutable globals, sign-ext, non-trapping f→i, multi-value, bulk memory, reference types | ≤ 96 | ≤ 79 | 15 | yes, default of `wasm32-unknown-unknown` (bulk memory and non-trapping since Rust 1.87/LLVM 20) | on |
| fixed-width SIMD128 | 91 | 89 | 16.4 | `+simd128` | on (`.cargo/config.toml`), minimum of the site |
| extended-const | 114 | 112 | 17.4 | `+extended-const` | off: measured +118/+158 B, no use |
| tail calls | 112 | 121 | 18.2 | `+tail-call` (Rust never needs it) | off |
| exception handling (exnref) | 137 | 131 | 18.4 | only `panic=unwind` + `-Zbuild-std` (nightly) | off: `panic = "abort"` |
| GC, typed function references | 119 | 120 | 18.2 | no (Rust has its own linear memory) | n/a |
| multi-memory | 120 | 125 | no | no | n/a |
| memory64 | 133 | 134 | flag (Technology Preview 251) | `wasm64-unknown-unknown` is tier 3 (nightly) | no (§5) |
| relaxed SIMD | 114 | 145 | flag | `+relaxed-simd` | **forbidden** (§6) |
| threads + atomics | 74 | 79 | 14.1 | the browser has them; Rust std for `wasm32-unknown-unknown` is built without atomics | no Rust threads (§5) |
| JS string builtins | 130 | 134 | 26.2 | not used by `wasm-bindgen` 0.2.129 | n/a |
| JSPI | 137 | 153 | 27 | n/a (our reads are synchronous in workers) | n/a |
| branch hinting | 137 | flag | 16 | no stable Rust attribute | n/a |
| compilation hints | flag | no | no | no | n/a |
| streaming compilation | yes | yes | yes | — | on (`compileStreaming`, one `Module` for all workers) |

- **Supported Safari on old macOS**: macOS 11 Big Sur stops at Safari 16.x (16.4 brought SIMD128 to Big Sur, March 2023); everything above 16.4 in the table (extended-const, tail calls, exnref, GC) would drop it. The site keeps exactly the set that Safari 16.4 has.
- **Cross-origin isolation** is already done by `sw.js` (COOP/COEP/CORP added to every same-origin response, `docs/studi/wasm-potenza.md` §3b): `SharedArrayBuffer` works on GitHub Pages after one reload; a `coi` shim is not needed.
- **Transport**: GitHub Pages serves gzip only (no brotli, no zstd, no compression dictionaries): sizes in §3 are given with gzip -9.

## 3. Sizes per module (bytes after the site's `wasm-opt`)
Built here with Rust 1.97.0, `wasm-bindgen` 0.2.129, binaryen 132 (the CI's tarball, same SHA-256), the site's flags (`-Oz` + `@motore`, readers `@estesi`). "base" = today's build. The other columns are differences from base.

| module | cap | base (gzip) | margin | binaryen 133 | `-O3` | deps O3 | deps O3 + `-O3` | worst vs cap |
|---|---|---|---|---|---|---|---|---|
| engine | 2744320 | 2491403 (1059407) | 252917 | -3594 | +16866 | +12228 | +28925 | under 2520328 |
| lib | 614400 | 521081 (235785) | 93319 | -124 | +6820 | -527 | +6315 | under 527901 |
| hr | 1605632 | 1510268 (676520) | 95364 | -971 | +23885 | +7600 | +31856 | under 1542124 |
| hr+formula | 1605632 | 1652566 (734666) | -46934 | -1065 | +25423 | — | — | OVER 1677989 |
| formula (lazy, new) | 512000 | 403567 (186454) | 108433 | -268 | +2616 | +5044 | +7770 | under 411337 |
| untargeted | 585728 | 533816 (223392) | 51912 | -356 | +7915 | +827 | +8556 | under 542372 |
| untargeted+formula | 585728 | 666813 (287559) | -81085 | -509 | +8845 | — | — | OVER 675658 |
| interp | 623200 | 563910 (248668) | 59290 | -146 | +8912 | +525 | +9434 | under 573344 |
| table | 737280 | 667923 (297013) | 69357 | -537 | +8488 | +882 | +9322 | under 677245 |
| grafo | 512000 | 214468 (103245) | 297532 | -83 | +1716 | +148 | +1772 | under 216240 |
| chem | 569344 | 558415 (255610) | 10929 | -174 | +3920 | +528 | +4326 | under 562741 |
| libro | 159744 | 149109 (68061) | 10635 | -119 | +1496 | +0 | +1496 | under 150605 |
| msngrafo | 790528 | 710956 (303175) | 79572 | -653 | +3116 | +4427 | +7627 | under 718583 |
| msnregole | 196608 | 173919 (77770) | 22689 | -183 | +661 | +99 | +805 | under 174724 |
| llm | 344064 | 306441 (140830) | 37623 | -90 | +2903 | -27 | +2842 | under 309344 |
| import | 512000 | 178730 (85781) | 333270 | -52 | +376 | -967 | -520 | under 179106 |
| attributio-all | 1536000 | 1453843 (630821) | 82157 | -718 | +26246 | -718 | +25277 | under 1480089 |
| finder-lr | 892928 | 797764 (367087) | 95164 | -413 | +10629 | +1890 | +12336 | under 810100 |
| aperti | 561152 | 489765 (230798) | 71387 | -282 | +3234 | +13012 | +16165 | under 505930 |
| andi | 344064 | 289904 (141535) | 54160 | -142 | +1359 | +6550 | +7969 | under 297873 |
| thermo | 774144 | 678592 (308615) | 95552 | -756 | +7406 | +8716 | +16231 | under 694823 |
| agilent | 606208 | 518506 (240352) | 87702 | -1057 | +2220 | +8267 | +10597 | under 529103 |
| sciex | 475136 | 409361 (196674) | 65775 | -543 | +2129 | +6245 | +8358 | under 417719 |
| waters | 380928 | 320620 (154649) | 60308 | -137 | +1872 | +6688 | +8561 | under 329181 |
| shimadzu | 380928 | 320539 (154778) | 60389 | -133 | +1875 | +7977 | +9804 | under 330343 |
| bruker | 532480 | 457398 (200282) | 75082 | -249 | +4335 | +6633 | +11073 | under 468471 |
| valida | 479744 | 470309 (158042) | 9435 | -355 | +878 | +4056 | +5113 | under 475422 |

- **Every module stays under its cap with `-O3` and the deps at O3 together** ("worst" column); the smallest margins are `valida` 4 322 B, `libro` 9 139 B, `chem` 6 603 B. Deps at O3 weigh +6-13 KB on the readers (quick-xml and zlib-rs are their core), +12 KB on the engine, ±1 KB on the modules that do not parse. My numbers differ from the CI's by a few KB (engine 2 491 403 here, 2 493 713 in `wasm-tetti.md`): every PR measures on CI.

- `hr+formula` = `hr` with `flow_formula` (what `formula.ms1` needs in `hr`); `formula` = `mzlab-feat-wasm --no-default-features --features flow_formula` alone (a lazy module of its own); `untargeted+formula` = 666 813 B against 585 728.
- **binaryen 133** saves 52-3 594 B per module (engine −3 594), same wasm-opt time: worth taking with the next touch of `tools/wasm_opt.sh`, not on its own.
- **opt-level `s`** for the cold crates (`release` profile `s`, hot crates still at 3): engine +1.6 %, `hr` +9.3 %, `attributio-all` +14.8 %, `untargeted` +7.4 %, `aperti` +4.0 %, `thermo` +1.1 %: worse than `z` everywhere; no.
- **Rust 1.99.1** (stable of 13 Oct 2026, LLVM newer): engine +14 479, `hr` −2 421, `attributio-all` −184, `thermo` +8 050, `untargeted` +1 652, `aperti` −2 299; speed within noise (§4). No reason to move the toolchain for the `.wasm` alone.
- **SIMD128 off** (bench crate): +0.2 % size, 3-20 % slower on read and index: SIMD stays.
- **Where the bytes are** (`hr`, before `wasm-opt`, by crate from the name section): `mzlab-feat` 493 KB, `core` generics 247 KB, `alloc` generics 231 KB, `mzlab-core` 222 KB, `mzlab-picchi` 61 KB. What `formula.ms1` adds to `hr` (+150 KB before `wasm-opt`): `core`+`alloc` generics 77 KB, `mzlab-core` 39 KB (mzML reading for features), `mzlab-feat` 27 KB.
- **Std sorts after `-Oz`** (kept names with `-g`): engine 105 754 B in 81 functions (4.5 %), `attributio-all` 136 339 B in 310 (10.8 %), `hr`+formula 65 657 B in 161 (4.4 %); `core::fmt` 21-37 KB. `mzlab_core::sortc` (one compact stable merge sort per element type) already replaces them in the modules of mzFinder; 128 call sites of `sort*` remain in `mzlab-core` and `mzlab-feat`.

## 4. Speed (bench crate `tools/banco_wasm`, real `mzlab-core` functions)
Copy of `tools/banco_wasm` with the site's profile (`z` + LTO, `mzlab-core` and `mzlab-picchi` at 3) as base; median of 5 runs (second round; the first round of 3 runs gives the same signs). 4 vCPU container, Node 22.22, Chromium 153.0.8010.12, Firefox 157.0, WebKit 27.2 (Playwright; WebKit on Linux is the engine of Safari, not macOS Safari). Check values identical in all 48 runs (6 variants × 4 engines × 2 rounds). Ratio = variant / base (< 1 is faster).

| job | base ms (Cr / Fx / Wk) | `wasm-opt -O3` | deps at O3 | deps at O3 + `-O3` | all crates O3 + `-O3` |
|---|---|---|---|---|---|
| mzML HR read (87 MB) | 858 / 884 / 828 | 0.93 / 0.96 / 0.96 | 0.86 / 0.87 / 0.85 | 0.91 / 0.78 / 0.83 | 0.79 / 0.77 / 0.84 |
| `.mzidx` HR index | 3705 / 3980 / 3729 | 0.98 / 0.97 / 0.96 | 0.94 / 0.89 / 0.93 | 0.94 / 0.85 / 0.87 | 0.83 / 0.81 / 0.85 |
| centroids, 100 HR scans | 2695 / 4200 / 2737 | 0.99 / 0.99 / 0.98 | 1.02 / 0.98 / 0.96 | 1.08 / 0.97 / 1.00 | 1.05 / 0.96 / 0.98 |
| entropy 300×300 MS2 | 1233 / 1543 / 1316 | 1.02 / 1.02 / 1.00 | 1.07 / 1.01 / 0.97 | 1.04 / 1.03 / 1.05 | 1.11 / 1.04 / 1.01 |
| 200 formula compositions | 3076 / 2706 / 2569 | 0.97 / **0.65** / **0.66** | 1.00 / 0.96 / 0.97 | 0.96 / **0.64** / **0.70** | 0.97 / 0.62 / 0.66 |
| inflate 64 MB | 328 / 319 / 323 | 1.03 / 1.05 / 0.99 | 1.01 / 0.99 / 0.91 | 1.01 / 0.95 / 1.00 | 1.04 / 0.95 / 0.92 |
| bench size (gzip) | 594 998 (268 535) | +3 029 (−34) | +11 532 (+2 344) | +14 493 (+2 500) | +15 124 (+648) |

- Node 22 follows Chromium (same V8): read 0.72 and index 0.76 with the deps at O3, compositions 0.96 with `-O3`.
- Compile time of the bench module: 12-31 ms Chromium, 58-60 ms Firefox, 27-41 ms WebKit; no lever (download dominates, `wasm-potenza.md` §3d).
- The jobs not in the bench (centWave, alignment, feature table, library search) live in `mzlab-feat`/`mzlab-lib` at `z`: the PR of prompt W1 measures them with `tools/bench.py` before and after.

## 5. Memory, threads, large inputs
- **memory64**: no. Safari has it only behind a flag, Rust has no stable `wasm64` target, and 64-bit addresses cost bounds checks. The engine already works under 4 GiB with budget levels and spill (`docs/agenti/memoria.md`): a file larger than RAM is read in blocks from OPFS, never whole.
- **Threads**: isolation is in place (`sw.js`); Rust threads still need `-Zbuild-std` (nightly) for an atomics-enabled std: no. The way is several wasm instances on one `SharedArrayBuffer` (`wasm-potenza.md` §3b, prompt WASM-E): stable toolchain, same bytes as `rayon` in the app.
- **WebKit on the macOS runner** (nightly `browser (macos-latest, webkit)`, `continue-on-error`): the engine does not start ("unknown transient reason, e.g. out of memory"). Not reproduced on Linux WebKit 27.2: 64 `WebAssembly.Memory` with `initial` 32 MB and `maximum` 4 GiB in the page, 12 workers with one each, and a grow to 1 GiB all succeed (same in Firefox 157). Likely the macOS WebContent process limit on the 7 GB arm64 runner (virtual reservations of several 4 GiB memories, or the real footprint of the engine plus pool workers), not a Wasm feature. Prompt W4 diagnoses it on the runner with a matrix (`--max-memory` 4 GiB vs 1 GiB, number of pool workers 4 vs 1, `--initial-memory` 32 vs 16 MB) before any change; the `.wasm` flags are per-build, so a lower `max-memory` would apply to every browser and needs a decision.
- **Large Thermo/Bruker/mzML**: the measured HR open (24.8 s, 61 MB) is read + index; the deps at O3 cut read by 13-28 % and index by 6-24 % (§4), the 4 MiB OPFS block (WASM-C) and the parallel index (WASM-E) are the other two levers.

## 6. Determinism and risks
- **Relaxed SIMD must never be used** in any module whose output is compared byte for byte (every module of the site: site = app): `relaxed_madd`, `relaxed_min/max`, `relaxed_dot`, `relaxed_swizzle`, `relaxed_trunc` give hardware-dependent results by specification. A guard test should reject `relaxed-simd` in `.cargo/config.toml` and in the scripts (prompt W1).
- **SIMD128, `-O3`, O3 profiles**: deterministic; LLVM never reassociates floating point without fast-math, binaryen keeps IEEE semantics; check values identical in every run. NaN payloads are the only freedom Wasm leaves; no output of ours prints them.
- **Unstable → stable sorts** (sort reserve): same order only where equal elements are identical; every changed call site needs the golden and parity tests unchanged, otherwise it stays.
- **Reproducible builds**: pinned Rust 1.97.0 (`rust-toolchain.toml`), `--locked`, binaryen tarball with SHA-256, `wasm-bindgen` 0.2.129 from the lock; `-O3` keeps the build reproducible.
- **CI time**: `wasm-opt -O3` takes the same 2-39 s per module as `-Oz` (measured, single-threaded, already parallel to cargo); O3 deps add little compile time (small crates).

## 7. Parity exceptions and the browser (`docs/studi/parita-browser.md`)
| exception | why today | proposal | needs |
|---|---|---|---|
| `formula.ms1` | `hr` +142 KB (1 652 566 > 1 605 632), `untargeted` 666 813 > 585 728 | (a) lazy module `formula` 403 567 B, standard limit 512 000 (new module line, loaded only by the Flow executor of `formula.ms1`); (b) or inside `hr` after the sort reserve (−47 KB needed, ~66 KB of sorts in it) | (a) COO/Federico yes to a new module; (b) none (cap unchanged) |
| files over 4 GiB of work | wasm32 limit | stays: memory64 not in Safari; block reading + spill already cover inputs larger than memory | — |
| threads inside one file | no Rust threads on stable | instances on `SharedArrayBuffer` (WASM-E) | already planned |
| `.wiff` with DAD | module size to measure | `sciex` has 65 775 B of margin (409 361 / 475 136): measure the DAD reader there first | measure |
| direct reading of 6 families | not wired | the reader modules exist and all have margin (§3) | wiring, no cap |
| tutor of the Teoria | no `.wasm` crate | size unknown: measure a `mzlab-tutor-wasm` build before deciding | measure |
| library download, cloud LLM | CSP / CORS, a choice | not a Wasm question | — |

## 8. Recommendations ranked by gain/risk
| # | change | gain (measured) | risk | prompt |
|---|---|---|---|---|
| 1 | `wasm-opt -O3` for every module (binaryen 133 with it), guard against `relaxed-simd` | compositions −35 % Firefox/WebKit; other jobs ±4 % | none on bytes; +0.2-1.8 % size, all under caps | W1 |
| 2 | `formula.ms1` in the browser as a lazy `formula` module | removes a parity exception | low; one new module (decision) | W3 |
| 3 | O3 on `quick-xml`, `base64`, `zlib-rs`, `miniz_oxide` | read −13-28 %, index −6-24 % | +6-13 KB on readers, +12 KB engine; all under caps | W2 |
| 4 | std sorts → `sortc` in `mzlab-core`/`mzlab-feat` hot modules | up to ~100 KB per big module (estimate: 66-136 KB of sorts) | order of equal elements: golden tests decide per site | W5 |
| 5 | WebKit/macOS diagnosis | turns the nightly WebKit job into a real check | none (CI only) | W4 |
| — | Rust 1.99, opt-level `s`, tail calls, extended-const, memory64, relaxed SIMD, Rust threads | none or negative | — | no |

## 9. Sources (read 29-30 Oct 2026)
- WebAssembly feature status, machine-readable: https://webassembly.org/features.json (versions of §2; Safari memory64 "Technology Preview 251", relaxed SIMD "JavaScriptCore flag", compilation hints flag-only in Chrome).
- Rust target docs: https://doc.rust-lang.org/rustc/platform-support/wasm32-unknown-unknown.html (default features; bulk memory and non-trapping float-to-int since Rust 1.87 / LLVM 20); LLVM change: https://lists.llvm.org/pipermail/cfe-commits/Week-of-Mon-20241021/636237.html; checked with `rustc --print cfg --target wasm32-unknown-unknown` on 1.97.0.
- Rust stable channel: https://static.rust-lang.org/dist/channel-rust-stable.toml (1.99.1, 13 Oct 2026).
- wasm-bindgen versions: https://crates.io/api/v1/crates/wasm-bindgen (0.2.129 of 25 Sep 2026 is the latest).
- binaryen: https://github.com/WebAssembly/binaryen/releases/download/version_133/binaryen-version_133-x86_64-linux.tar.gz (SHA-256 `2dc9c7813f5375db93d96ead4b78222fcc3e2677bbb832297af4797782a37489`; version_134 does not exist yet).
- Safari 16.4 (SIMD, also for macOS Big Sur): https://webkit.org/blog/13966/webkit-features-in-safari-16-4/ and https://tidbits.com/watchlist/safari-16-4/.
- V8 explicit compile hints (JavaScript only, Chrome 136): https://v8.dev/blog/explicit-compile-hints.

## 10. Decisions for Federico (recommendation first)
1. **`formula.ms1` in the browser**: a lazy `formula` module with the standard 512 000 B limit (**recommended**: no cap raised, parity restored now), or wait for the sort reserve and put it in `hr`.
2. **Minimum browsers stay** Chrome/Edge 91, Firefox 89, Safari 16.4 (macOS 11 included): **no** feature above Safari 16.4 is enabled (tail calls, extended-const, exnref, GC).
3. **Toolchain**: stay on Rust 1.97.0 for the `.wasm` (1.99.1 measured neutral); take binaryen 133 with W1 (**recommended**).
4. **WebKit on macOS**: after W4, whether a lower `--max-memory` (e.g. 2 GiB) for every browser is acceptable if it is the cause (it caps the budget levels of `memoria.md` at 2 GiB).

## 11. How to redo the measurements
Scripts of this study (not in the repository): a module build loop over the 25 `cargo build` lines of `tools/build_feat_wasm.sh`, `tools/build_lettori_wasm.sh` and `pages.yml` with an own `CARGO_TARGET_DIR` per variant and the profile changes passed as `cargo --config 'profile.release.package.<crate>.opt-level=3'`; `wasm-bindgen --target web`; the site's `wasm-opt` line of `tools/wasm_opt.sh` with `-Oz`/`-O3`; the speed numbers from `tools/banco_wasm` with an extra profile `site` (= `z` + `mzlab-core`/`mzlab-picchi` at 3) and `js/run.html` in each Playwright browser (`PLAYWRIGHT_BROWSERS_PATH` for Firefox and WebKit). Prompt W1 adds these variants to `tools/banco_wasm.py`, so the next study runs one command.
