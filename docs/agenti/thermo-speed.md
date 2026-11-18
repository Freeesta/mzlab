# Thermo `.raw` reader: speed and zero-loss criteria, bench and baseline numbers (T-7)

Oracle and dump format: `scandump.md`. Two levels with SEPARATE criteria and numbers, never one table that mixes them: **A** = faithful and fast reader against RFR; **B** = lossless bridge to Flow on the centroids.

## How every number is declared
- Every number carries the 40-character SHA of the `main` commit it was measured on, the machine, the target (`native` or `wasm`) and the build profile (`cli` = `opt-level = 3`; `release` = `opt-level = "z"`, the one of the `.wasm`). A number without these four data does not enter the tables.
- The comparison with RFR holds only on the same Mac and only on files that open with BOTH readers; the others are listed apart with the reason (never averaged). ThermoRawFileParser, msconvert and mzmine are measured on the same Mac as the others.
- At least five laps (15 to decide a small gain); report the median, the fastest lap (`best`) and the noise (`noise`: range and standard deviation in % of the laps); a gain is declared only if it exceeds the noise measured on the same machine. The first lap (cold cache) and the warm-cache laps are separate columns; for tools that go through RFR the total cost seen by the user is measured.
- A baseline row is repeated after every PR that changes the content read or the reading path (T-3 #1537 touches the scan events: repeat row A before T-7c). Every number goes in the table even if we lose, with the cause.
- Coordinator rules: only Rust in the product; **native** (CLI, desktop) without `.wasm` caps (`mmap`, `rayon`, SIMD with `std::arch` behind a CPU check at startup); **browser** with the caps and the declared +10 % (no SharedArrayBuffer: more workers).
- `unsafe` only in small, isolated modules with tests, fuzzing and a safe twin compared bit by bit.
- A new dependency for speed only if the measured gain is ≥ 3 % on its target (`REGOLE.md` rule of 22/10: even a few percent points count) and the license is permissive; in the PR the `Cargo.toml`/`Cargo.lock` lines, license and before/after numbers.
- No speed rule overrides A1-A4. Every speed change adds a row to the cumulative table `measurements-speed.md` (date · PR · step · before · after · %).

## Level A criteria (reader)
- **A1 Byte register**: every byte belongs to a named section (header, `RawFileInfo`, `RunHeader`, OLE2 method, index, events, trailer, status, tune, error log, packets, padding), interpreted or kept raw under a name. Bytes outside every section = `unexplained_bytes`: red test on built files, goal 0 on the corpus. The register lives in an example or in a native-only feature, never in the `thermo` module.
- **A2 Round trip**: re-encoding every section gives back the same bytes (SHA-256); with A1 at 0 the sum gives back the file. The writer lives only in tests and native tools.
- **A3 Profile kept**: the reader never centroids behind the scenes; it gives the profile and, for FT scans that have it, the producer's centroid stream (m/z, intensity, noise, baseline, resolution, charge) without losing columns.
- **A4 Fingerprints**: per-scan SHA-256 of the arrays in canonical form (m/z and intensity as little-endian f64, file order) and per file (fingerprint of the fingerprints), field `peaks_sha256` of `scandump/1`. If it differs from RFR the comparator gives the maximum difference in ulp and the cause; no hidden tolerance.
- **A5 Steady-state speed**: MB/s and scans/s for open, read of all scans, one random scan (100 random, median and 95th percentile). Goal: native ≥ RFR on all three; browser within 1.7× the native.
- **A6 Time to first result**, three times from the click: TIC/BPC (index and headers, without decoding peaks), first decoded scan, everything. Goal: TIC/BPC and first scan below RFR.
- **A7 Comparison**: RFR, ThermoRawFileParser, msconvert and the `.raw` import of mzmine (with the reader it uses inside), same Mac, same files, version of each tool.

## Level B criteria (Flow on the centroids)
- **B1** The profile → centroids loss exists only in the `spectra.centroid` step (D6/L8, parameters in `prov/1`); reader, columns and matrix lose nothing.
- **B2 Lossless columns**: m/z and intensity in separate columns, delta + byte shuffle + zstd, NEVER Numpress nor rounding; decoded they give back the A4 fingerprints. New optional block of the `.mzidx` with a declared version (`error.newerFormat`); existing `.mzidx` stay byte for byte equal. Numbers: size against `.raw` and mzML, write and read MB/s.
- **B3 Scan × bin matrix (CSR)**: an XIC is a column slice, bit-equal to the brute-force sum; the feature table from the matrix is byte-for-byte equal to the one from the mzML route.
- **B4 Fused centroiding**: decoding and centroiding in one step, centroids bit-equal to `readers_centroids.rs`; a new algorithm is a parameter of the step, off by default, declared with loss and compared with `meta.compare`.
- **B5 Flow against mzmine**: time and peak memory `.raw` → centroids → feature table against mzmine (`tools/bench/corri_mzmine.py`) and against our `.raw` → mzML → feature route, same files and parameters.

## Bench
- `cargo run --profile cli -p mzlab-lettori --features thermo --example thermo_bench -- FILE.raw --sha SHA40 --machine TEXT --profile cli [--reps 5] [--target native]`: one `banco/1` row (open, TIC/BPC from the index, first scan, all scans, 100 scans at fixed positions, MB/s, scans/s; `first_lap`, `median`, `best`, `noise`). The file is read into memory before the lap (`file_read_ms` apart): it does not measure the disk. The 100 positions are xorshift64 from `0x9E3779B97F4A7C15`, `k = x % n`: the RFR oracle must use the same ones.
- `python3 tools/bench/thermo_velocita.py unisci BANCO.jsonl RFR.jsonl > UNITO.tsv` then `tabella UNITO.tsv --sha SHA40 --maquina TEXT`: per-family tables only on the files opened by both, count of the rest apart, no file names. `RFR.jsonl` = `banco/1` rows with `reader: "rfr"` and the same phases.
- Substitute Orbitrap v66 file for the speed benches (no real data): `cargo run --release -p mzlab-lettori --example thermo_synthetic -- OUT.raw [--scans 2000] [--peaks 2000] [--bins 16] [--ms2 0] [--ms2-peaks 200] [--trailer 600] [--seed 1]` (profile FTMS MS1 with centroids, `--ms2 K` dependent HCD MS2 in centroid after every MS1, written as a stream; same seed, same bytes).
- Sizes of the substitute file: heavy ≈ 2.1 GB `--scans 82500 --peaks 3000 --ms2 10`; normal ≈ 290 MB `--scans 5400 --peaks 600`. Test `thermo_synthetic_hr.rs`: cycle, same seed, and a fixed hash of all the decoded scans (centroids and profile) of a profile file, the gate for every change to the decoder.
- Phases of the path to the map and of the mzML (level B, native): `cargo run --profile cli -p mzlab-core --features thermo --example raw_times -- FILE.raw`. Browser: `python3 tools/bench_raw.py FILE.raw` (needs `wasm-bindgen`).
- `thermo_scandump` dump (`scandump.md`): its `read_all_ms` includes trailer and dump rows per scan, its `open_ms` opens twice and copies the file; it is NOT the same work as `thermo_bench` and must not be compared with it.

## Measurement noise (cloud 4 vCPU, `thermo_bench`, profile `cli`, `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`)
- Within a 15-lap run the standard deviation is 8-28 % of the mean and the range (max − min) 26-118 % of the median, on LAB-A, PXD077492, PXD064022 and MTBLS5358: the machine is shared and single laps do not separate differences of 1-2 %.
- Between five 15-lap runs on the same file (PXD077492) the median of "all" varies 32.7-34.6 ms (5.5 %) and that of open 63.6-73.5 ms (14 %); the fastest lap varies 32.0-33.2 ms (3.9 %) and 61.7-62.9 ms (2 %). So on the cloud one decides with `best` over ≥ 15 laps and with the same paired before/after run (same binary, same session, alternating); gains below 4 % are confirmed on the Mac with the bench on an idle machine.

## Baseline numbers, level A
**A-1 · Mac arm64, native, `thermo_scandump` of `a9a7468ffce0921fa03b2a99ca04d64a4b0e0453`** (with #1508, before T-3 and T-5), RFR = official RawFileReader, 221 files opened by both out of 238 (24.5 GB, 1 899 674 scans; the number of scans matches in every file). Read of all scans = `read_all` phase of the dump of both (metadata, filter, trailer, peaks per scan); files ≥ 500 scans not separated.

| Family | Files | Scans | Open s (RFR / ours) | Read s (RFR / ours) | Scans/s | MB/s | Median ratio per file |
|---|---|---|---|---|---|---|---|
| Q Exactive | 119 | 883 156 | 21.9 / 9.9 | 148.0 / 42.4 | 5 966 / 20 825 | 64 / 224 | 3.5× |
| Orbitrap Fusion, Eclipse | 68 | 744 150 | 13.5 / 8.0 | 120.3 / 22.9 | 6 186 / 32 470 | 94 / 493 | 5.0× |
| Orbitrap IQ-X | 16 | 198 790 | 3.3 / 2.9 | 29.2 / 7.4 | 6 802 / 26 878 | 122 / 480 | 4.4× |
| LTQ, Velos, Elite | 6 | 14 441 | 0.4 / 0.3 | 1.7 / 0.3 | 8 570 / 43 107 | 52 / 264 | 10.1× |
| TSQ | 6 | 59 129 | 0.5 / 3.5 | 1.2 / 0.4 | 49 233 / 140 449 | 66 / 189 | 3.4× |
| All (including 6 files of other instruments or without a name) | 221 | 1 899 674 | 39.8 / 24.7 | 300.6 / 73.5 | 6 319 / 25 854 | 82 / 334 | 3.6× |

- **Where we lose**: open. Ours is slower than RFR in 18 files out of 221 (8 IQ-X, 5 TSQ, 3 LTQ, 1 Q Exactive, 1 Fusion); on SRM TSQ the open costs 0.17-1.35 s against 0.05-0.09 s (TSQ sum 3.5 s against 0.5 s). In the read of everything we lose in no file. Cause not verified: in the dump the open is doubled and copies the file (see above), and SRM files have tens of thousands of events; A-2 measures it with the bench.
- **Opened by one only, apart**: RFR opens and we do not 7 files (format 63/64: LTQ Velos, LTQ Orbitrap XL, 4 TSQ Vantage, 1 TSQ Quantum Ultra, `AllocationTooLarge`; the T-5 #1533 work on `main` ≥ `8fc88a6350b190dba1587fa9f7793bd9968d4848` tolerates unreadable streams: the row must be redone, request R-06); we open and RFR does not 2 files; 8 are opened by nobody (5 are 4 KB AppleDouble files).
- Missing for A5-A7: random scan, TIC/BPC and first scan of RFR with the bench definitions; ThermoRawFileParser, msconvert and mzmine (requests R-07 and R-08 in the description of PR T-7a).

**A-2 · Cloud 4 vCPU Intel Xeon 2.80 GHz, 16 GB, native, `thermo_bench`, profile `cli`, 5 repetitions, median, `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`** (reader unchanged from that commit). Our reader only (no RFR here: trend, not comparison). 28 `.raw` ≤ 90 MB of the corpus opened; 7 files of the corpus do not open or have no decodable scans (three are truncated, `err.file.rawTruncated`, for example 1.4 MB out of 102 MB declared). File in memory; open = `apri` + `indice`.

| File | MB | Scans | Peaks | Open ms | TIC/BPC ms | 1st scan µs | All ms | MB/s | Scans/s | Random µs p50 / p95 |
|---|---|---|---|---|---|---|---|---|---|---|
| LAB-A | 86.8 | 7952 | 4 185 930 | 163.6 | 0.10 | 49 | 119.4 | 727 | 66 616 | 21 / 29 |
| LAB-B | 82.3 | 16 524 | 1 051 091 | 208.1 | 0.13 | 33 | 117.8 | 699 | 140 325 | 14 / 32 |
| MTBLS5358 | 34.1 | 7492 | 3 642 049 | 45.6 | 0.05 | 12 | 35.8 | 953 | 209 512 | 5 / 6 |
| PXD077492 | 31.8 | 6916 | 520 434 | 63.5 | 0.07 | 15 | 32.7 | 973 | 211 758 | 7 / 13 |
| PXD064999 | 28.8 | 4647 | 1 715 797 | 52.2 | 0.04 | 20 | 41.9 | 686 | 110 821 | 11 / 18 |
| PXD064022 (SRM) | 22.9 | 11 056 | 36 594 | 422.7 | 0.13 | 8 | 20.8 | 1100 | 531 046 | 3 / 4 |
| MTBLS9839 | 17.4 | 370 | 94 080 | 12.9 | 0.00 | 15 | 3.0 | 5883 | 125 027 | 8 / 16 |
| PXD059868 | 15.1 | 261 | 232 003 | 12.1 | 0.00 | 25 | 4.8 | 3140 | 54 194 | 19 / 51 |
| PXD006054 | 11.5 | 27 567 | 48 580 | 58.9 | 0.30 | 16 | 17.2 | 669 | 1 605 758 | 1 / 2 |
| MTBLS14724 | 10.8 | 543 | 401 233 | 12.0 | 0.00 | 18 | 8.2 | 1310 | 66 059 | 19 / 41 |
| MTBLS14496 (SRM) | 10.5 | 20 552 | 42 205 | 437.1 | 0.28 | 10 | 30.1 | 348 | 682 222 | 3 / 4 |
| MTBLS1752 | 7.5 | 17 859 | 35 607 | 44.3 | 0.18 | 15 | 10.8 | 695 | 1 649 187 | 1 / 2 |
| PXD057351 (SRM) | 7.3 | 12 526 | 100 208 | 214.4 | 0.14 | 12 | 42.7 | 170 | 293 254 | 4 / 5 |
| PXD067770 | 6.5 | 2365 | 324 499 | 39.1 | 0.02 | 16 | 15.0 | 434 | 157 558 | 9 / 17 |
| MTBLS1807 | 5.9 | 14 550 | 25 642 | 32.9 | 0.14 | 9 | 8.6 | 684 | 1 693 450 | 1 / 2 |
| other 13 files ≤ 4.2 MB | 0.1-4.2 | 1-5396 | | 1.1-63.0 | ≤ 0.04 | 4-86 | 0.0-10.8 | | | 1-76 / 1-107 |

- The open is dominated by something that is not the reading of the peaks: on the SRM files (PXD064022, MTBLS14496, PXD057351) it opens in 214-437 ms and reads everything in 21-43 ms, i.e. the open is 10-20× the read; this is where RFR wins (A-1). Causes to look for in the profile before T-7c (index of events and trailers, `GenericRecordStream`).
- Opening an Orbitrap `.raw` looked up every `ScanParams` label in each scan's trailer by comparing strings field by field (`GenericRecord::get`); the label index of `Etichette`, built once per stream, makes each lookup a map hit. Numbers in `measurements-speed-raw.md`.
- TIC/BPC and first scan are already below one millisecond and 100 µs on all files: for A6 the comparison with RFR is the only missing datum.
- The read at 0.7-1.1 GB/s with the file in memory measures decoding, not the disk: the time from disk or OPFS is in `file_read_ms` of the bench and in the chunked reads of the browser.

**A-2b · Baseline row repeated after T-3 (#1537, scan events): cloud 4 vCPU, native, `thermo_bench`, profile `cli`, `main` `239fa02602a7cf97c68f25440e033cd6e8b13dc4` against `ab4fe7d00606e58218a7ea8570f333a4026b4fc4` (A-2).** Two binaries alternating on the same file (same session), 15 laps and two runs per binary, fastest lap (`best`) of the best run; noise between the two runs of the same binary 0.3-4.5 % (up to 19 % on small files). 20 files > 1 MB; difference = new − old.

| Measure | Median of the differences | Range | Beyond the noise |
|---|---|---|---|
| Open (`apri` + index) | −3.1 % | −9.4 … +0.4 % | faster by 6-9 % on 6 SRM files or with few peaks (MTBLS14496, MTBLS7889, PXD057351, PXD059868, PXD064022, PXD019525) |
| All scans | +0.8 % | −2.4 … +15.3 % | **slower** by 13-15 % on MTBLS1752, MTBLS1807, PXD019516 (SRM, 1-2 peaks per scan, +77-87 ns per scan), 8.9 % on PXD006054, 3.7-4.6 % on LAB-B, MTBLS775, MTBLS9839 |

- In the large Orbitrap files (LAB-A, PXD077492, PXD064999, PXD067770) the read of everything does not change beyond the noise (from −2.4 to +3.3 %). The per-scan cost that T-3 adds is fixed, so it weighs only where the scan is almost empty; the cause (building the filter per step or reading the events) was not looked for: to profile before T-7c.
- TIC/BPC and first scan stay below 1 ms and 100 µs. The ratio with RFR on the Mac waits for R-15 (RFR against `thermo_bench`).

**A-3 · Cloud 4 vCPU, `wasm` target, `tools/bench_raw.py` (module `thermo`, profile `release` without `wasm-opt`, Node 22, `wasm-bindgen` 0.2.129), `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`, ONE run** (not the 5 repetitions: provisional). Native = A-2 (profile `cli`, `opt-level = 3`); the comparison mixes profile and work (the module's scan also builds the JSON meta and copies the array), so it is an upper bound of the ratio.

| File | `apri` + index ms (wasm / native) | All scans ms (wasm / native) | Open ratio | Read ratio | µs per scan (wasm) |
|---|---|---|---|---|---|
| LAB-A | 255.8 / 163.6 | 748.4 / 119.4 | 1.6× | 6.3× | 75.2 |
| PXD077492 | 248.6 / 63.5 | 146.4 / 32.7 | 3.9× | 4.5× | 17.6 |
| PXD064022 (SRM) | 1483.6 / 422.7 | 68.2 / 20.8 | 3.5× | 3.3× | 4.4 |

- **Where we lose**: A5 asks for browser ≤ 1.7× the native and today the read is 3.3-6.3× (ratio of profile `z` against `3` included). Module memory after `apri`: 32-41 MB (the initial memory cap of the module is 32 MB).

## Baseline numbers, level B
**B-1 · Cloud 4 vCPU, native, `raw_times`, profile `cli`, `main` `ab4fe7d00606e58218a7ea8570f333a4026b4fc4`, one run** (warm cache, reads in 256 KB blocks as in the module). Decoding of everything, normal route and `lettori_centroidi` route; the rest is common.

| File | Decoding (s) normal / centroids | Table s | `.mzidx` s | Table + `.mzidx` + scratch s | mzML s | Memory peak |
|---|---|---|---|---|---|---|
| LAB-A (87 MB, 7952 scans) | 0.130 / 0.137 | 0.604 | 0.563 | 0.947 | 2.282 (98.2 MB) | 297 MB |
| PXD077492 (32 MB, 6916 scans) | 0.071 / 0.041 | 0.541 | 0.156 | 0.500 | 1.290 (35.9 MB) | 234 MB |

- Not yet existing: columns (B2), matrix (B3), fused centroiding (B4), feature table from `.raw` (B5). Rows B2-B5 are filled in phases T-7d and T-7e; `.raw` → centroids → feature table against mzmine needs mzmine on the Mac (R-08). The time of the Flow `.raw` → mzML → feature table and the mzmine values on mzML (`mzfinder-benchmark.md`) have no SHA here and are not reported.

**B-2 · Cloud 4 vCPU, `wasm`, site `.mzidx` build in Node 22** (`build_mzidx_native` of the engine with the `thermo` module as external reader, `fs.readSync` reads on demand, spill to file, 64 MB budget, fingerprints given; modules without `wasm-opt`; 10 alternating runs, median [min-max] in ms; same `.mzidx` bytes). `thermo_synthetic` files: normal 288 MB (`--scans 5400 --peaks 600`), heavy 2.14 GB (`--scans 82500 --peaks 3000 --ms2 10`).

Measured with the blocks no longer zero-filled and without the adaptive window of `rawreader::Stream`, whose numbers are in `measurements-speed-raw.md`.

| file | open | scans | chromatograms | total |
|---|---|---|---|---|
| normal | 68 [65-72] | 344 [306-381] | 196 [182-252] | 617 [555-672] |
| heavy | 1246 [1199-1321] | 4018 [3393-4708] | 2691 [2468-2868] | 7920 [7085-8708] |

- The fingerprints (SHA-256 + SHA-1 + `Passata` probe, one worker) cost a further 1.0 s on the normal file and 7.4 s on the heavy one, before the build.

## Rows to repeat
- Row A on the Mac with the bench on both readers (R-15, RFR against `thermo_bench`): new SHA, same A-1 table with the `thermo_bench` definitions; the repetition on the cloud after T-3 is A-2b.
- `wasm` target: A-3 must be repeated with 5 repetitions and, for the ratio with the native, with the same profile (`release`) for both targets; cap of the `thermo` module 546 304 B (`wasm-caps.md`).

## RFR as oracle, and where to beat it
- **RFR only as oracle**: the RawFileReader license is proprietary: DLL and NuGet package enter no repository, binary, `.wasm` or CI; they are used only on the Mac, in a folder outside the repositories, to write the `scandump/1` dumps with the public API only; never copy or decompile its code. A field that RFR does not expose is omitted, never invented.
- **Where to beat RFR**: everywhere (native and `.wasm`, files do not leave the computer); speed (native equal or better, in the browser ≤ 1.7×); robustness (a truncated file opens up to the last whole scan with a warning, no panic under fuzzing); clean labels per reaction (activation, energy with units, stepped NCE, SPS precursors, FAIMS CV); errors with the cause and "Copy diagnosis".
