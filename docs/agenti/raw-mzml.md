# `.raw` → mzML conversion: bench, roofline, measured causes

Measurement bench of the converter (`crates/mzlab-core/examples/raw_mzml_bench/`, launcher `tools/bench/raw_mzml.py`) and baseline numbers for the V1-V9 criteria (section "Acceptance criteria"). The bench changes no output byte: it reads with the real `Lettore` and writes with the real `Conversione`/`MzmlWriter`. Cumulative table of gains: `measurements-speed.md`; Thermo reader: `thermo-speed.md`.

## How to run
- `python3 tools/bench/raw_mzml.py --file X.raw` (real file) or `--denso 2` (synthetic dense case of 2 GB, generated in `--tmp` and deleted); `--zlib-rs` redoes everything with the `banco-zlib-rs` feature; options after `--` go to the bench. It writes `.verifica/raw-mzml/<label>.<backend>.json` (fields: machine, git, rustc, kernel, `cli` profile, disk bandwidth measured with `dd`).
- Direct: `cargo build --profile cli -p mzlab-core --example raw_mzml_bench`, then `target/cli/examples/raw_mzml_bench --file X.raw --giri 5 --json out.json`. `--solo fasi,scala,cli,micro,d8` chooses the sections (micro and d8 only if requested), `--thread 1,2,3,4`, `--peaks tutto|file`, `--senza-impronte` (no SHA-256 of the output in the scale series), `--senza-conta` (the counting allocator does not count: for multi-thread timings, see D8), `--forma lab-a|lab-b` (with `--denso`: small spectra of the two shapes of the LAB files), `--gb`, `--cancella`.
- Every measurement is repeated `--giri` times (normally 5): the JSON has the seconds of each round, median, interquartile range (`iqr_s`), `migliore_s`, `primo_giro_s`, `mediana_senza_primo_s`, CPU, peak RSS (reset with `clear_refs`), heap peak, number and bytes of allocations (counting allocator, bench only), page faults. The scale series run `scrivi_con` at zlib levels 0/1/6 and `indicizzato` (level 1) with 1…N threads.
- Noise: within one run the interquartile range is ≤ 3 % on small sources and on the dense case; between two runs the medians are within 5 %; the first round on a file not in cache costs up to +9 % (on PRIDE at 1.7 GB +6 s: [16.0 · 10.0 · 10.1] s at 1 thread), so for a comparison use `mediana_senza_primo_s` or `migliore_s` over ≥ 15 rounds. A change below 1-2 % shows only with many rounds on the same binary; do not launch cargo, clippy or the pre-push while the bench is measuring.
- The dense generator is deterministic (integer SplitMix64, only IEEE operations): same SHA-256 on every round (`dense_test.rs`); the fake `Lettore` serves the packets from the file through the real decoders (`opentfraw::pacchetto::decodifica`), bit-exact re-read on 3 scans; the 8 GB variant is generated and deleted in an `#[ignore]` test (`cargo test --profile cli -p mzlab-core --example raw_mzml_bench -- --ignored`).

## Machine and T_min
- Machine for these numbers: Xeon 2.8 GHz, 4 cores, 16 GB, Linux 6.18 (VM), `cli` profile, zlib `miniz_oxide` (measurements before the switch to zlib-rs), no SHA-NI (SHA-1 ≈ 440 MB/s, SHA-256 ≈ 204 MB/s). Disk: sequential write 216 MB/s (`dd conv=fdatasync`), read 3.0 GB/s (page cache).
- `T_min = max(B_in/BW_read, B_out/BW_write, W_cpu/N, W_serial)`; `W_serial` = SHA-1 of the input in the mzML header (a single stream), which on large files is already a floor. Micro-benchmarks (`--solo micro`): base64 1.87 GB/s; freq→m/z conversion 4.1 ns/point; zeroing 63 GB/s; malloc+free 65 ns; XML 80-86 µs/spectrum; whole-packet FT decoding 8.4 µs (lab-a, 11 KB), 10.6 µs (lab-b, 4 KB), 4.6 ms (dense, 1.5 MB), of which peak list 4.2 µs / 0.83 µs / 154 µs; deflate level 0/1/6 on the dense case:
  214 / 160 / 22 MB/s (compressed/original ratio 1.00 / 0.80 / 0.80).

## Baseline for V1-V9
Measurements in seconds, median without the first round. Lab-b and lab-a: `--senza-conta`, 7 rounds; the other columns with the counting allocator on. LAB-A, LAB-B and BAF are not reachable from this machine: not verified on real files; the numbers measured earlier on real files hold (LAB-B `convert` 5.97 s, writer 2.87 s; LAB-A 3.96 s) together with the synthetic sources of the same shape (`--forma lab-b`, 16,731 scans of 4 KB, 69 MB; `--forma lab-a`, 6,336 scans of 11 KB, 70 MB), which give a 1-thread writer at 2.1 s (lab-b) and 0.8 s (lab-a), the same order of magnitude as the real ones.

| | PRIDE (stand-in, 1.71 GB, 46,730 scans) | dense 0.4 GB (271 scans) | dense 2 GB | dense 8 GB | synthetic lab-b | synthetic lab-a |
|---|---|---|---|---|---|---|
| T_min (s) | 5.7 (input SHA-1); write 3.3; CPU/4 2.4 | 3.3 (write 711 MB); CPU/4 1.8 | ≈ 16 (3.55 GB/216 MB/s) | ≈ 66 (14.2 GB) | ≈ 1 | ≈ 0.5 |
| V1/V2/V3 writer, level 1, 1 thread | 11.8 | 7.2 | indexed 42.4 | indexed 168 | 2.1 | 0.8 |
| V4 level 0, 4 threads | 13.7 | 2.46 | 11.6 | 46.3 | 0.61 | 0.21 |
| V4 level 1, 4 threads | 13.3 | 2.46 | 11.95 | 48.6 | 0.62 | 0.24 |
| V4 level 6, 4 threads | 16.9 | 8.5 | 41.8 | 168.8 | 0.81 | 0.35 |
| V5 level 1, 1→2→4 threads | 11.8 → 12.6 → 13.3 (slower) | 7.2 → 2.65 → 2.46 (2.9×) | — | 168 → 72 (indexed 1→4, 2.3×) | 2.14 → 0.86 → 0.62 (3.5×) | 0.79 → 0.36 → 0.24 (3.3×) |
| V6 peak RSS, 1 thread (4 threads) MB | 339 | 26 (111) | 31 indexed (122) | 48 indexed (158) | 13 | 7 |
| V7 allocations per scan | ≈ 670 | ≈ 59,000 | ≈ 59,000 | ≈ 59,000 | ≈ 1,720 | ≈ 1,590 |
| V9 opening (s) | 0.95 | 0.00005 | — | — | — | — |

Readings: V5 is met on the synthetic lab-b and lab-a (4 threads: 2.7-3.5× at level 0-1, 3.6× at level 6) and on the dense case (2.9× with 4 threads at level 1, 3.9× at level 6 on 4 cores: 33→8.5 s); PRIDE is not re-measured after the pool fix (D8): its row is the old pool's. V6: RSS is not constant with file size (4 threads: 111 → 122 → 158 MB for 0.4 → 2 → 8 GB;
1 thread indexed 70 → 31 → 48 MB) and PRIDE sits at 339 MB: cause not identified (in the dense case the heap peak is 12 MB: the part beyond the heap is system/mapped memory). V7: the gap from the target (≤ 1 per 1000 scans) is 6-8 orders of magnitude; at 1 thread the counter estimates 10-20 % of the time in allocation, against about 10 % from callgrind (same order of magnitude). V8 (browser) is not measured here (RM-5). V10: see below.

Breakdown of a PRIDE write without zlib (phases, seconds out of the total 4.08 xml-only → 1.66 sink write): decoding 1.5, extra per scan 1.5, XML with empty arrays 1.66, base64 3.5, output SHA-1 and offsets 1.2, file write 0.77; zlib level 1 adds 1.25 s, level 6 25 s (miniz_oxide).

## zlib: miniz_oxide versus zlib-rs
Same bench, build with `--features banco-zlib-rs`, 1 thread, total seconds (miniz → zlib-rs; output bytes): PRIDE level 1 8.76 → 7.26 s (−17 %; 722 → 837 MB, +16 %), level 6 32.8 → 17.6 s (−46 %; 718 → 717 MB); dense 0.4 GB level 1 6.99 → 10.8 s (+54 %; 711 → 921 MB, +30 %), level 6 32.5 → 19.1 s (−41 %; 712 → 796 MB).
Level 0 oscillates more (PRIDE 7.5 → 4.2 s, dense 6.3 → 3.1 s: the first round pays for the cold cache), not to be read as a gain. zlib-rs changes the compressed bytes: it requires a new `conv/1` version with before/after; it gains where the input is made of centroid spectra and loses on dense profiles at level 1. The build uses zlib-rs (read and write) with default level 2 (`mzmlwrite::ZLIB_LEVEL`): on the dense profile of the DAD test, zlib-rs level 1 writes 16.3 MB against 12.1 for miniz, level 2 12.9 MB; `conv/1` declares `compressore`.

## D11: the fingerprints of `mzlab-cli convert`
On PRIDE (RM-0, `--solo cli` measurement, no SHA-NI) the command took ≈ 35 s against 20 s for the indexed writer alone: it was not conversion but fingerprints computed several times on the same bytes (input SHA-256 8.6 s, input SHA-1 5.7 s inside the header, output SHA-256 3.7 s in `keep` and again 3.7 s in the report, input read 3×, output re-read 2×).
Every byte is hashed once per purpose: `flow_set::hash_file` (`hashio::hash_read_with`) reads the input once and takes its SHA-256, and the SHA-1 if the plan writes an mzML (`flow_set::wants_sha1`); `file_set` passes the SHA-1 to the reader (`Membro.sha1`), which does not re-read the file for the header; the writer goes through `hashio::HashWrite` and `write_mzml_with` returns the bytes and the SHA-256 of the output, which `FileRunner::keep_known` and the `convert.rs` report use without re-reading it (`flow_set::remember_sha`). A file taken from the cache is hashed as before.
Synthetic measurement (mzXML 320 MB, output 323 MB without zlib, machine with SHA-NI, 3 alternating rounds): 2.93 → 2.73 s; identical output bytes (`cmp`). The gain is proportional to the cost of the hashes: not re-measured on PRIDE.

## D8: more encoding threads do not slow down
The writer pool (`mzmlwrite/pool.rs`) hands the spectra to the workers in blocks (≤ 32 spectra or ≈ 100,000 points) and has them returned together with the text: the thread that allocated them frees them.
Synthetic lab-b (`--forma lab-b --solo scala --senza-conta --thread 1,2,3,4`, 7 rounds, seconds): level 0 1.65 · 0.65 · 0.61 · 0.61, level 1 2.14 · 0.86 · 0.58 · 0.62, level 6 2.94 · 1.23 · 0.84 · 0.81; lab-a level 0 0.63 · 0.28 · 0.20 · 0.21. With the same bench the pool with one message per spectrum gave at level 0 1.57 · 3.28 · 3.39 · 3.40 s (lab-b) and 0.61 · 1.00 · 0.94 · 0.86 s (lab-a). The output bytes are identical (equal SHA-256 at 1 and 4 threads, levels 0/1/6 and indexed; test `tests_pool`).
- Cause: the vectors of each spectrum are allocated by the reading thread; if a worker frees them, the blocks go back to the reader's glibc arena and contend for its lock (`futex`: 516,000 calls in five passes of lab-b at 4 threads with one message per spectrum, 8,800 with blocks and return). Blocking alone is not enough (at 4 threads lab-b level 0: 2.7 s against 1.8 at 1 thread); returning the vectors to the reader thread brings lab-b to 0.8 s and the block of 32 to 0.6 s (level 6: 1.23 → 0.93 s).
- The bench with the counting allocator (`count.rs`: four shared atomics per allocation) slows multi-thread runs by itself (lab-b level 0 at 4 threads 5.1 s with the counter, 2.95 without it before the fix): for multi-thread timings use `--senza-conta`.
- `--solo d8` measures the isolated mechanisms: channel with 1 task per message 12-24 µs overhead per task, with 32 tasks per message 0.3-1 µs; 8 KB blocks freed by another thread 1.4 µs at 1 thread, 5.2 at 2, 10.6 at 4, against 0.08-0.17 µs if freed by the same thread. Ruled out: the number of glibc arenas (`MALLOC_ARENA_MAX=1` and `=4`) and compression (present even at level 0).

## Dense synthetic
`Parametri::denso`: 20,500 centroid peaks with ≥ 200,000 profile points per scan (8-12 bins per peak), 271 scans per 0.4 GB, ≈ 1.4 MB per packet; decoding 4.6 ms/scan (peak list 154 µs), 57,468 allocations per scan in the decoder (one per peak and per bin: V7). Uncompressed output 892 MB (0.4 GB), 4.46 GB (2 GB), 17.8 GB (8 GB): at level 0 with 4 threads the limit becomes the write disk (216 MB/s → 4.1 s for 892 MB against 2.46 s measured: the system cache absorbs the difference).
The 8 GB case takes 8 GB of disk plus the output: it deletes itself with `--cancella`.

## Parallel encoding: `convert_times`

`convert_times` (`crates/mzlab-core/examples/`) converts an mzML already in memory with `Conversione::scrivi` and prints time, heap peak, process peak, bytes and SHA-256; the SHA-256 runs on a separate thread as a file write would. 4-core container, synthetic mzML from `tools/mzml_huge.py` (0.29 GB: 5271 scans, `--peaks1 20000 --peaks2 2000`; 0.4 GB: small scans, defaults), same SHA-256 with 1, 2 and 4 threads:

- large spectra: 1 thread 4.09 s, 2 threads 2.83 s, 4 threads 1.75 s (2.3×); heap peak 5.5 MB → 18 MB (largely the tail of the example's SHA);
- small spectra: 1 thread 6.66 s, 2 threads 3.95 s, 4 threads 3.18 s (2.1×); process peak unchanged (237 → 239 MB, the file read).

The window is 4×`thread` spectra: MS1 cost 10 times an MS2 and the spectrum at the head of the order stalls the threads, so 2× is not enough (1.5-1.7×).

## Acceptance criteria
The thresholds change only with a Coordinator decision written here, never to make a PR pass. "HR corpus": LAB-A, LAB-B, BAF, INF-A, the Thermo files of the local corpus and the public files. Before any speed gain the fidelity checks apply; `unsafe` stays isolated; a new dependency requires a gain ≥ 3 %.
- **F1** centroids: m/z (f64) and intensity bit-equal to RawFileReader (RFR) and to msconvert `peakPicking vendor` on 100 % of the FT scans, same number of peaks.
- **F2** per-peak columns (charge, flag, resolution, noise, baseline) equal to RFR's `CentroidStream`, with the standard CV terms (MS:1000516, MS:1002529, MS:1002742, MS:1002530; noise nodes MS:1002743/1002744/1002745) and the list length; flags without a term as MS:1000786 with a name. Depends on D1.
- **F3** profile: non-zero points bit-equal to RFR and msconvert; with zeros "as RFR" the whole array equal to msconvert, zeros declared with a `userParam` per spectrum and counted in `conv/1`. Depends on D2.
- **F4** from the mzML (m/z, coefficients and fudge per scan) the frequency bins of 100 % of the points are recomputed; exact f32 intensities (round-trip test in Rust).
- **F5** metadata: filter, trailer fields, nearest status and tune, method text; equal to msconvert on MS level, polarity, analyzer, resolving power (MS:1000800), activation (CID/HCD/ETD/EThcD/UVPD), energy, precursor m/z and charge, isolation window, RT. An `@cid` MS2 is CID (MS:1000133), never HCD; `is_dia` never on a targeted MS2.
- **F6** number of spectra = scans of the file; TIC and BPC equal to msconvert (f32). **F7** a field absent in the file is absent in the mzML; zeros only with the F3 option, marked.
- **F8** 0 errors with the PSI validator, OpenMS `FileInfo -v`, pyteomics, mzR and msconvert re-reading; every offset points to the first byte of `<spectrum ` / `<chromatogram `, correct `fileChecksum` SHA-1.
- **F9** equal output SHA-256 with 1/2/4/N threads, on Linux x86-64, macOS arm64 and `.wasm`, with permuted inputs and in two rounds. **F10** where a PR does not change the output, `raw_equivalenza.rs`, `raw_parity.rs`, `msntree_parity.rs` stay green; where it changes it, before/after on one file and `conv/1` version bumped.
- **C1-C4** our own centroiding, only on scans without an instrument list or with `--peaks mzlab`, never in the base path of FT. Simulated (≥ 10,000 peaks, S/N from 3 to 10⁴): median |Δm/z| ≤ 0.2 ppm (or ≤ 0.02 FWHM) and 99th percentile ≤ 1 ppm, median area ≤ 1 %, two peaks at 1.0 FWHM resolved ≥ 95 %, ghosts ≤ 0.1 %, recall ≥ 99.5 % at S/N ≥ 10.
- Against the instrument list: precision and recall ≥ 0.98 within 1 ppm at S/N ≥ 10, |median of the intensity ratio − 1| ≤ 2 %; calibrants: median error within 0.1 ppm of the instrument's, isotope ratios within 1 %; ion trap: ±0.05 Th from msconvert, precision and recall ≥ 0.95 (if it fails the numbers are reported and the Coordinator decides).
- **V1/V2** LAB-B and LAB-A with indexed `mzlab-cli convert`, default level, 4 cores: ≤ 0.6 s and ≤ 0.5 s, and ≤ 3× T_min. **V3** BAF ≤ 3× T_min and faster than msconvert and TRFP on the same Mac. **V4** dense ≥ 2 GB: without compression ≤ 1.5× B_out/BW_write, with zlib ≤ 1.3× W_cpu/N.
- **V5** speedup ≥ 0.7·N up to 4 cores, never slower than 1 thread. **V6** RSS ≤ 64 MB + 16 MB × threads + 4 × the largest scan, constant with file size. **V7** ≤ 1 allocation per 1000 scans at steady state. **V8** in the browser 1 worker ≤ 1.7× native at 1 thread, W workers ≥ 0.6·W, bytes equal to native, `.wasm` +≤ 10 % declared. **V9** first spectrum written ≤ 50 ms after opening.
- **V10** default compression: zlib-rs L1 with reused stream if on the HR corpus it reduces size ≥ 15 % with time ≤ 1.3× without compression; otherwise no compression.
- External checks: RFR `scandump/1` dumps compared with `tools/rosina/scandump_compare.py` (bit-exact); msconvert with and without `peakPicking vendor`; ThermoRawFileParser only with the Coordinator's authorization; the five F8 validators; OpenMS, pyteomics and mzR on non-standard arrays (if they fail the columns go in a file alongside, D7); mzmine and SIRIUS on LAB-B from our mzML and from msconvert's (features within 1 ppm and 0.05 min, areas within 1 %); timings on the local lab's Mac with cold cache, 5 rounds.

## Open doubts
- **D1** high bits of the centroid descriptor: index in the low 16 bits, probable charge in bits 24-31 and flags in bits 16-23; closed with RFR's `CentroidStream.Charges` and `Flags`, peak by peak.
- **D2** RFR/msconvert zero rule (typical runs of 8 between pieces and 4 at the edges, exact rule unknown): derived from RFR's profile dumps. **D3** the msconvert reference of LAB-B covers only scans 1-4718 out of 16,524: a complete mzML is needed, in profile and with `peakPicking vendor`.
- **D4** per-peak noise: does RFR interpolate from the ~40 nodes or give the nodes? If it interpolates, the formula must be verified bit by bit, otherwise the nodes are written. **D5** Exploris and Astral: packet not seen, public files needed. **D6** FAIMS, DIA, polarity switching, msx: shape in the filter and trailer and msconvert's CV terms, from public files.
- **D7** third-party readers with non-standard arrays, different `arrayLength` in the same spectrum, zlib with stored blocks: fallback = per-peak columns in the columnar block of the `.mzidx` and standard-only mzML. **D9** `.raw` SHA-1 on the critical path only if the file is not already read in full; if T_min is dominated by the SHA-1 on multi-GB files, the limit is declared. **D10** activation from the filter (`@cid`, `@hcd`, `@etd`, `@ethcd`, `@uvpd`, `@sa`) and from the trailer to CV terms like msconvert, F5 check on the whole corpus.

## Coordinator decisions
- Zeros: by default only the points of the file (nothing invented). Indexed mzML also from `scrivi`, in the same pass with offsets and SHA-1. The per-peak columns in the reader (`vendor/opentfraw`, `scan_data.rs`) are done one PR at a time. Switching to zlib-rs changes the compressed bytes: new `conv/1` version and before/after in the PR (F10).
- Proposal not carried out: `--peaks tutto` (profile as a standard array, centroid list as a non-standard array, per-peak columns with standard CV terms and `arrayLength`); the default value `file` stays until third-party readers pass (D7).
- Proposal not carried out: `--set conv.zeri=rfr` for zero parity with msconvert and RFR (F3).
