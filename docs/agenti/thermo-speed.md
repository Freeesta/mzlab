# Thermo `.raw` reader: speed rules and bench

Oracle and dump format: `scandump.md`. Two levels with SEPARATE criteria and numbers, never one table that mixes them: **A** = faithful and fast reader against RFR (Thermo RawFileReader); **B** = the Flow on the centroids. Measured numbers live in `measurements-speed-raw.md` and the cumulative table `measurements-speed.md`.

## How a number is declared
- Every number carries the SHA of the `main` commit it was measured on, the machine, the target (`native` or `wasm`) and the build profile (`cli` = `opt-level = 3`; `release` = `opt-level = "z"`, the one of the `.wasm`). A number without these four data does not enter the tables.
- The comparison with RFR holds only on the same Mac and only on files that open with BOTH readers; the others are listed apart with the reason (never averaged).
- At least five laps (15 to decide a small gain); report the median, the fastest lap (`best`) and the noise (`noise`: range and standard deviation in % of the laps); a gain is declared only if it exceeds the noise measured on the same machine. The first lap (cold cache) and the warm-cache laps are separate columns.
- A baseline row is repeated after every change of the content read or of the reading path. Every number goes in the table even if we lose, with the cause.
- Only Rust in the product; **native** (CLI, desktop) without `.wasm` caps (`mmap`, `rayon`, SIMD with `std::arch` behind a CPU check at startup); **browser** with the caps (no SharedArrayBuffer: more workers). `unsafe` only in small, isolated modules with tests, fuzzing and a safe twin compared bit by bit.
- A new dependency for speed only if the measured gain is ≥ 3 % on its target and the license is permissive (`rust.md`, "Speed"); the PR shows the `Cargo.toml`/`Cargo.lock` lines, license and before/after numbers. Every speed change adds a row to `measurements-speed.md`.

## Level A (reader)
- **Profile kept**: the reader never centroids behind the scenes; it gives the profile and, for FT scans that have it, the producer's centroid stream (m/z, intensity, noise, baseline, resolution, charge) without losing columns.
- **Steady-state speed**: MB/s and scans/s for open, read of all scans, one random scan (100 random, median and 95th percentile). Target: native at least as fast as RFR on all three; browser within 1.7× the native.
- **Time to first result**, three times from the click: TIC/BPC (index and headers, without decoding peaks), first decoded scan, everything.
- **Comparison**: RFR, ThermoRawFileParser, msconvert and the `.raw` import of mzmine, same Mac, same files, version of each tool.

## Level B (Flow on the centroids)
- The profile → centroids loss exists only in the `spectra.centroid` step (parameters in `prov/1`); reader, columns and matrix lose nothing.
- Time and peak memory of `.raw` → centroids → feature table against mzmine (`tools/bench/corri_mzmine.py`) and against our `.raw` → mzML → feature route, same files and parameters.

## Bench
- `cargo run --profile cli -p mzlab-lettori --features thermo --example thermo_bench -- FILE.raw --sha SHA40 --machine TEXT --profile cli [--reps 5] [--target native]`: one `banco/1` row (open, TIC/BPC from the index, first scan, all scans, 100 scans at fixed positions, MB/s, scans/s; `first_lap`, `median`, `best`, `noise`). The file is read into memory before the lap (`file_read_ms` apart): it does not measure the disk. The 100 positions are xorshift64 from `0x9E3779B97F4A7C15`, `k = x % n`: the RFR oracle must use the same ones.
- `python3 tools/bench/thermo_velocita.py unisci BANCO.jsonl RFR.jsonl > UNITO.tsv` then `tabella UNITO.tsv --sha SHA40 --maquina TEXT`: per-family tables only on the files opened by both, count of the rest apart, no file names. `RFR.jsonl` = `banco/1` rows with `reader: "rfr"` and the same phases.
- Substitute Orbitrap v66 file for the speed benches (no real data): `cargo run --release -p mzlab-lettori --example thermo_synthetic -- OUT.raw [--scans 2000] [--peaks 2000] [--bins 16] [--ms2 0] [--ms2-peaks 200] [--trailer 600] [--seed 1]` (profile FTMS MS1 with centroids, `--ms2 K` dependent HCD MS2 in centroid after every MS1, written as a stream; same seed, same bytes). Sizes: heavy ≈ 2.1 GB `--scans 82500 --peaks 3000 --ms2 10`; normal ≈ 290 MB `--scans 5400 --peaks 600`.
  Test `thermo_synthetic_hr.rs`: cycle, same seed, and a fixed hash of all the decoded scans (centroids and profile) of a profile file, the gate for every change to the decoder.
- Phases of the path to the map and of the mzML (level B, native): `cargo run --profile cli -p mzlab-core --features thermo --example raw_times -- FILE.raw`. Browser: `python3 tools/bench_raw.py FILE.raw` (needs `wasm-bindgen`).
- The `thermo_scandump` dump (`scandump.md`): its `read_all_ms` includes trailer and dump rows per scan and its `open_ms` opens twice and copies the file; it is NOT the same work as `thermo_bench` and must not be compared with it.

## Findings
- **Noise** (cloud, 4 vCPU, shared): within a 15-lap run the standard deviation is 8-28 % of the mean; between runs the median of "all" varies about 5 % and that of open about 14 %, the fastest lap 2-4 %. On the cloud one decides with `best` over at least 15 laps and a paired before/after run (same binary pair, same session, alternating); gains below 4 % are confirmed on an idle Mac.
- **Native read** runs at 0.7-1.1 GB/s with the file in memory (decoding, not the disk; the disk or OPFS time is `file_read_ms` and the chunked reads of the browser). TIC/BPC and first scan take under one millisecond and 100 µs.
- **Open dominates on SRM files** (open 10-20× the read, tens of thousands of events): this is where RFR is faster. Reading everything is faster than RFR on every family measured on the Mac.
- **Per-scan fixed cost** weighs only where the scan is almost empty (SRM files with 1-2 peaks per scan).
- **Browser** read is several times slower than native (profile `z` against `3` and the module's JSON meta and array copy included); the target is ≤ 1.7×. The module cap is in `tools/moduli/lettori-thermo.toml` (`wasm-caps.md`).
- The substitute-file `.mzidx` build in the site (`build_mzidx_native` with the `thermo` module as external reader) and its phases are in `measurements-speed-raw.md`.

## RFR as oracle, and where to beat it
- **RFR only as oracle**: the RawFileReader license is proprietary: DLL and NuGet package enter no repository, binary, `.wasm` or CI; they are used only on the Mac, in a folder outside the repositories, to write the `scandump/1` dumps with the public API only; never copy or decompile its code. A field that RFR does not expose is omitted, never invented.
- **Where to beat RFR**: everywhere (native and `.wasm`, files do not leave the computer); speed; robustness (a truncated file opens up to the last whole scan with a warning, no panic under fuzzing); clean labels per reaction (activation, energy with units, stepped NCE, SPS precursors, FAIMS CV); errors with the cause and "Copy diagnosis".
