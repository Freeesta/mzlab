# Where the engine is slow, and what would make it faster

Study of the hot paths of mzLab (indexing and opening, TIC/XIC, centroids, peaks, Flow features, map and spectrum, Data view, wasm start-up), ranked by measured gain per risk. Every number below was measured on `origin/main` at `e4ab1f8` in the cloud container (4 vCPU, native `release` profile, so `mzlab-core` at O3 as in `Cargo.toml`), unless marked **site** (taken from the tables already in `docs/agenti/`) or **estimate**. It does not repeat what is done: read `docs/agenti/misure-velocita.md` and `ottimizzazioni-tentate.md` first; the earlier studies are `OTTIMIZZAZIONE_CORE.md` (size, mzML reader), `RAW_VELOCE.md`, `VELOCITA_RICARICA.md`, `docs/agenti/wasm-potenziale.md`, `visore-veloce.md`, `mappa-fluida.md`.

Build prompts: `docs/studi/prompt_prestazioni/` (wave 1: 01-03, wave 2: 04-05).

## 1. Summary
- **Biggest measured waste: one click on the TIC of an HR centroid file.** The spectrum of ONE scan is answered after two full passes over every peak of the MS1 section, then the scan is read again from the mzML. Natively one pass costs 48.5 ms on a 9 M-peak file and 190 ms on a 30 M-peak file. In the site the same gesture (G1) costs 804 ms / 2 427 ms, of which the engine takes 574 / 1 775 ms (`visore-veloce.md`). G6 (ArrowRight) follows the same path: 104 / 287 ms. The fix decides "one scan" from the scan table before touching the peaks. The output stays byte-identical. Prompt 01.
- **Fingerprints in the browser:** at the site's opt-level `z`, wasm SHA-1 hashes 223 MB/s. At O3 it hashes 626 MB/s (2.8×). SHA-256 goes from 245 to 281 MB/s. For a `.raw`/`.wiff` (both hashes) the hashing rate goes from 117 to 194 MB/s (−40 % of the hash time); for an mzML (SHA-256 only), −13 %. The change is two profile lines, and the hash values cannot change. The cost is a few KB of `.wasm`, to be measured against the caps. Prompt 02; prompt 05 (two workers) only if 02 shows the hash is on the critical path.
- **Data view with hundreds of files:** `renderFileList` rebuilds the whole list (`innerHTML`) and is called once per file event, which is quadratic in the number of files. I could not measure it here (no site build); at 6 files it was already 646 ms at 6× CPU (`ottimizzazioni-tentate.md`). Prompt 03 measures at 100/300 files first and adopts only above the threshold.
- **Cases that never open in the site:** `hrp` (400 × 50 000 profile) and `hr15` (15 × 150 MB) never open within 25-27 min (`visore-veloce.md`). Natively the same `hrp` file prepares in 2.2 s and indexes in 1.7 s, so the cause is not engine compute. Prompt 04 diagnoses the cause first.
- **Not hot (measured, leave alone):** mzML decoding, XIC from the index, centroider, peak picker, Flow on LR, wasm compile (§3).

## 2. Ranking
| # | Where | Measured | Change | Expected gain (basis) | Risk | Identity proof | Prompt |
|---|---|---|---|---|---|---|---|
| 1 | `hrroutes.rs:24` `spectrum_exact`, `hrroutes.rs:14` `hr_spectrum`, `spectrum.rs:91` `binned_cells` (`each_peak(si, 0, total, …)` at `spectrum.rs:127`); `rust-bridge.js:147` asks `route` then `specexact` | full-section pass 48.5 ms (hrc, 9 M peaks), 190 ms (hrc_big, 30 M), 6.1 ms (lr, 0.75 M), native; site G1 engine 574 / 1 775 ms | select the scans of the window from the scan table (`selected`-like, no peaks); when exactly one scan is in the window, return `None` (route) or read the mzML (exact) without the pass | site G1 HR 804 → ~150 ms, hrc_big 2 427 → ~400 ms (**estimate**: two passes removed out of 574 / 1 775 ms engine; the mzML read of one scan stays); G6 the same | low: same JSON; the "empty scan" case must keep its answer | old vs new JSON byte for byte on every scan of hrc, HR_DDA-Exploris, a file with an empty scan, MS2 with precursor and `filt` | 01 |
| 2 | `Cargo.toml` profiles: `sha1`, `sha2` (and their `-asm`-free soft backends) at `z` | wasm (Node 22, `+simd128`, LTO, no wasm-opt): SHA-1 223 → 626 MB/s, SHA-256 245 → 281 MB/s (z → O3) | `[profile.release.package.sha1]` / `sha2` `opt-level = 3` | fingerprint of a 1 GB `.raw`: 8.6 → 5.2 s of worker CPU; mzML 4.1 → 3.6 s | low: size against the caps (measure every `.wasm` that links them); never raise a cap | SHA values are fixed by the input: existing fingerprint tests + a bench job with a check value | 02 |
| 3 | `explore.js:772` `renderFileList` (full `innerHTML`, one call per file event, 50+ callers) | not measured here; 11 calls = 646 ms at 6× CPU with 6 files (earlier) | one render per animation frame while files arrive (coalescing), same final DOM | **unknown**; quadratic term predicted from the code | medium: tour, focus, `aria-current`, handlers wired after render | final `#flst` HTML identical; e2e of the list | 03 |
| 4 | `hrp` / `hr15` in the site (`visore-veloce.md:45`) | site: no panel within 25-27 min; native: `hrp` 320 MB prepare 2.2 s, `.mzidx` 1.7 s; native `api/spectrum` on `hrp` answers `None` for 1- and 3-scan windows, but only because my harness did not call `prime_steps` (the site reads the step of a single-precision profile section from the mzML, `profspec.rs:68`), so it says nothing about the site | diagnose the blocking phase in Chromium, then fix within the caps | from "never" to opening (unknown size of the fix) | unknown until measured | depends on the fix | 04 |
| 5 | `impronta.js` / `impronta-worker.js`: SHA-256 and SHA-1 of a `.raw`/`.wiff` in one worker, serially | after 02: 194 MB/s serial, 281 MB/s if the two run in two workers (**estimate** from the rates above) | one worker per hash for vendor files | −31 % of hash time after 02, only if the hash is on the critical path | low (same values), +1 worker of memory | same `[sha256, sha1, probe]` on every test file | 05 |

## 3. Measured and not hot (no prompt)
| Path | Number (native, this container) | Why it stays |
|---|---|---|
| mzML parse, all arrays decoded (`mzml_tempi`, 55.5 MB, 3 000 scans, 3.9 M peaks) | 0.083 s; scan table + prints 0.434 s (0.344 s with prints on a second thread); `.mzidx` 0.63 s (0.59 s with spool) | `decode_array` fix of `OTTIMIZZAZIONE_CORE.md` P2 is in; the rest is zlib-rs and SHA-256 |
| `.mzidx` build (`mzidx_tempi`, 20 000 scans, 17.1 M peaks) | decode 0.034 s, table 0.257 s, index 2.94 s (2.68 s with scratch). Callgrind on 2 000 × 2 000, after removing the bench's own SHA-256 of the output (57 % of instructions, not product code): the radix sort `mzidx_ordine::sequenziale` ~48 %, `Sec::flush` ~12 %, `Sec::carica` ~11 %, `Prima::add` ~9 %, `memset` ~6 % | the radix sort with constant-digit skipping is already there; a further gain needs a different algorithm (per-scan merge), uncertain, not proposed |
| XIC from the index (`idx_xic_tempi`, 20 000 × 2 000, 568 MB index) | ±0.5 Da 0.065 ms/XIC (5.3 reads); ±20 Da 0.83 ms/XIC (13 reads) | site G2 is 19-30 ms, almost all drawing and transport (`visore-veloce.md`) |
| Profile centroider (`centroidi_tempi`, 9 000 points) | 0.87 ms/scan (~97 ns/point); in callgrind the `exp` calls belong to the bench's data generator, not the centroider | already O3 in `mzlab-core` |
| Peak picker (`picchi_tempi`, 20 000 scans, 440 peaks) | 8 ms per trace | `mzlab-picchi` at O3 |
| Flow `features` (`mzlab-cli`, release) | 7 LR Full Scan files 0.18-0.22 s; 4 × 55 MB HR synthetic 2.3-2.6 s (1 file 1.16 s, index-dominated; the synthetic HR has no features, so detect/align are not exercised) | the tp.link/align work of October is in; real HR series are in the private repository, not here |
| wasm compile and start-up | 10-19 ms per module, `compileStreaming` with one shared `Module` (`wasm-potenziale.md` §(g)) | the cost is download, not compile |

## 4. Not measured here (said plainly)
- Nothing in Chromium: building the site needs `wasm-bindgen` and `wasm-opt` (not installed in this container) and ~15 min. The site numbers above are the latest from `visore-veloce.md`, `mappa-fluida.md` and `misure-velocita.md`.
- The OPFS copy rate against the hash rate, so whether the fingerprints are on the critical path of opening a vendor file (prompt 02 measures it, prompt 05 depends on it).
- The Data view at 100+ files (prompt 03), the real lab files (the private repository is not in this container), memory peaks in wasm.
- The SHA rates come from a scratch crate with the site's flags (`+simd128`, LTO, `codegen-units = 1`, `panic = "abort"`), 256 MB hashed, best of 5, in Node 22. Prompt 02 must repeat the measurement on the real `.wasm` after `wasm-opt`.

## 5. Waves and file ownership
Each prompt owns its files; no two prompts of one wave touch the same file. `docs/schemi/steps-cli-1.json`, `F.liveJobs` and the build scripts are not touched by any prompt. `Cargo.toml` (workspace profiles) belongs to 02 alone.
- **Wave 1** (parallel): 01 `crates/mzlab-core/src/hrroutes.rs`, `spectrum.rs`, `examples/visore_tempi.rs`, a new test, `docs/agenti/visore-veloce.md`, `misure-velocita.md`, `ottimizzazioni-tentate.md` · 02 `Cargo.toml`, `tools/banco_wasm/` · 03 `mzlab/web/explore.js`, `mzlab/web/shell.js`, one new `tests_e2e/` script.
- **Wave 2** (after wave 1 is in `main`): 04 diagnosis of `hrp`/`hr15` (`tools/bench_visore.py`, `crates/mzlab-core/src/profspec.rs` or whatever the diagnosis names, `docs/agenti/visore-veloce.md`) · 05 `mzlab/web/impronta.js`, `impronta-worker.js`, `rust-nativo.js`, `misure-velocita.md`, `ottimizzazioni-tentate.md`.
- In wave 1 only 01 writes `misure-velocita.md` and `ottimizzazioni-tentate.md`. 02 and 03 put their row in the PR body under "Row for misure-velocita.md", and 05 copies those rows in.

## 6. For Federico (needs a decision, not in any prompt)
- Nothing proposed here raises a cap or `MAX_TOTAL`. If prompt 02 finds that a module would cross its cap, it stops and reports the bytes; the choice is Federico's.
- Known gains that would need a cap raise, already recorded: O3 on the whole engine (`wasm-potenziale.md` §3) and wasm threads (needs nightly `atomics`, §(c)).

## 7. How to reproduce
```
cargo build --release -p mzlab-core --examples && cargo build --release -p mzlab-cli
E=target/release/examples
$E/mzml_tempi genera D/hr.mzML 3000 3000 && $E/mzml_tempi D/hr.mzML
$E/mzidx_tempi; $E/idx_xic_tempi; $E/centroidi_tempi; $E/picchi_tempi
(cd tools && python3 -c "import bench_mappa as b; b.write_file('D/hrc.mzML','hrc',3000,3000,1); b.write_file('D/hrcb.mzML','hrc',6000,5000,3); b.write_file('D/hrp.mzML','hrp',400,50000,4)")
$E/visore_tempi indicizza D/hrc.mzML D/hrc.mzidx      # then time r.handle("/api/spectrum", rt0..rt1 around one scan, level=1)
CARGO_PROFILE_RELEASE_DEBUG=1 CARGO_TARGET_DIR=target/prof cargo build --release -p mzlab-core --example mzidx_tempi
valgrind --tool=callgrind target/prof/release/examples/mzidx_tempi 2000 2000
```
SHA rates in wasm: a scratch `cdylib` with `sha1`/`sha2` (`default-features = false`, the versions of `Cargo.lock`) exporting `sha256_mb(n)`/`sha1_mb(n)` over a 1 MB buffer, built with `CARGO_PROFILE_RELEASE_OPT_LEVEL=z|2|3 RUSTFLAGS="-C target-feature=+simd128" cargo build --release --target wasm32-unknown-unknown`, timed in Node with `WebAssembly.Instance` (no imports). Prompt 02 adds this as a bench job instead of a scratch crate.
