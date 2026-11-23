# Speed study: targeted extraction over hundreds of files

Why one targeted extraction (N (m/z, RT window) targets x M files, each trace integrated) took about 15 minutes with mzLab and 15-25 s per polarity with MZmine 4 in a field comparison, measured on synthetic data, and how to make mzLab clearly faster. Harness: `tools/bench/speed_study.py` (data `tools/speed_synthetic.py`, in-process modes `crates/mzlab-cli/examples/speed_targets.rs`). Not run in CI.

## Setup

- Machine: cloud container, 4 vCPU, 15 GB RAM, `cli` profile (opt-level 3, LTO), page cache warm (every row reads from memory, as an agent's second run would).
- LR set (`speed_synthetic.py`): 245 unit-resolution triple-quad full-scan centroid mzML, 3.89 MB each (954 MB), 2400 scans over 20 min, about 132 000 peaks per file (about 55 non-zero ion-count channels per scan: Poisson background plus compounds), 275 targets (`mz,rt_lo,rt_hi`, RT window 1 min). 245 x 275 = 67 375 traces, the size of the field run (about 67 000 calls).
- HR set (`synthetic_data_large.py --files 12 --scans 2000 --compounds 1500 --noise 800`): 12 Orbitrap-like DDA mzML, 434 MB.
- Per-call route as the agent ran it: `mzlab-cli trace xic FILE --mz M --da 0.5 --rt A,B --format csv | mzlab-cli peak auto-lr -`, one pipe per target and file, one after the other. 300 pipes are sampled over files and targets and extrapolated linearly.
- Wall time from the harness; CPU = user + system of the process and its reaped children (`wait4`); peak RSS = largest process; bytes read = `rchar` (bytes through read syscalls, page cache included); passes = bytes read / bytes of the input file.
- Identity: every in-process mode writes the `peak auto-lr` JSON of each (file, target). On the 4 files the per-call mode covers, all modes give the same bytes as the CLI pipe; `list` and `onepass` (1 and 4 threads) also give the same SHA-256 over all 67 375 rows.

## Baseline (origin/main)

| row | wall | CPU | peak RSS | read | passes per file |
|---|---|---|---|---|---|
| process start (`mzlab-cli --version`) | 2 ms | 2 ms | 73 MB | 0 | - |
| `file info` (open + index, 1 file) | 81 ms | 134 ms | 73 MB | 17.7 MB | 4.6 |
| `file index --cache` (1 file) | 100 ms | 151 ms | 73 MB | 22.0 MB | 5.7 |
| one `trace xic --format csv` | 82 ms | 135 ms | 73 MB | 17.0 MB | 4.4 |
| one pipe `trace xic --rt | peak auto-lr -` | 83 ms | 141 ms | 73 MB | 17.0 MB | 4.4 |
| one `trace xic --mz` list of 275 ions (CSV) | 374 ms | 428 ms | 93 MB | 19.9 MB | 5.1 |
| per-call pipes, 300 sampled | 85.9 ms per call | 143 ms per call | 73 MB | 17 MB per call | 4.4 per call |
| per-call pipes, extrapolated to 67 375 | **5 789 s (96 min)** | 2.7 CPU-h | | 1.15 TB | 1 200 per file |
| per-call pipes with `--cache` (indexes pre-built), 300 sampled | 17.6 ms per call | 18.5 ms per call | 73 MB | 11.8 MB per call | 3.0 per call |
| per-call pipes with `--cache`, extrapolated | **1 187 s (20 min)** | | | 0.8 TB | 825 per file |
| in-process per-call (`mzlab_cli::execute`, no process), 4 files x 275 | 88.6 s (80.4 ms per call) | 150.8 s | 41 MB | 18.7 GB | 1 200 |
| in-process: one `trace xic --mz LIST` per file, 1 thread, 245 files | 98.0 s | 110.9 s | 146 MB | 5.2 GB | 5.45 |
| same, 4 threads | 26.3 s | 98.4 s | 397 MB | 5.2 GB | 5.45 |
| **in-process one pass per file (prototype), 1 thread** | **20.7 s** | 34.9 s | 55 MB | 954 MB | **1.00** |
| **same, 4 threads** | **5.1 s** | 18.7 s | 82 MB | 954 MB | **1.00** |
| `file convert` of a synthetic Waters `.raw` (2400 scans, 26.5 MB read) | 0.18 s | 0.21 s | 123 MB | 26.5 MB | - |
| `features` (Dig + align), 12 HR files | 3.03 s | 9.80 s | 199 MB | 2.8 GB | - |
| `features` (Dig + align), all 245 LR files | 8.36 s | 23.7 s | 123 MB | 7.1 GB | - |

The field run (67 000 calls in about 15 min, 13 ms per call) matches the cached per-call route on a faster machine or smaller files; the uncached route would have taken over an hour here.

### One call, step by step (in memory, 3.9 MB file, 3 files, ms)

| step | ms | share of a call |
|---|---|---|
| whole call through `mzlab_cli::execute` (`trace xic` + `peak auto-lr`) | 66-98 | 100 % |
| scan table + SHA-256 fingerprints (`upload::prepare`) | 16-20 | about 20 % |
| `.mzidx` build (decode every scan, sort the peaks by m/z, write; `mzidx::build_mzml_spill`) | 48-68 | about 65 % |
| of which: decoding the mzML alone (`mzml::parse`) | 35-52 | |
| chromatogram headers, file facts, scratch folder, index write and re-read (rest) | about 10 | about 10 % |
| open the index (`Index::open`, `Routes::new`) | 2.4-6 | 3 % |
| the XIC itself (first call, fence and RT array loaded) | 0.43-0.56 | 0.6 % |
| the XIC route steady state, per ion, text included | 0.30-0.35 | |
| rendering the trace as JSON text (4800 rounded numbers) | 0.27-0.36 | 0.4 % |
| `peak auto-lr` on the cut trace | 0.04-0.10 | 0.1 % |

## Where the time goes

1. **Re-reading and re-indexing the file per call: about 95 %.** Each call decodes the whole mzML (every scan, base64 + zlib), sorts its 132 000 peaks into a `.mzidx`, writes it to a scratch folder and reads it back, to answer one window of about 130 peaks. 4.4 passes over the file per call, 1 200 per file for the run. The cache (`--cache`) removes the decode and the sort but keeps a SHA-256 of the file, a checksum of the entry and a copy of it per call (3.0 passes, 17.6 ms).
2. **Process start: 2 ms (about 2 %).** Not the problem: in-process calls through `mzlab_cli::execute` cost 80.4 ms against 85.9 ms through two processes and a pipe.
3. **Single thread per call.** A call uses about 1.6 cores (fingerprints on a second thread); the agent ran calls one after the other. Threads over files work: the list mode scales 3.7x and the one pass 4.1x on 4 cores.
4. **Output formatting.** Per trace the JSON text costs more than the XIC (0.27 ms against about 0.06 ms of engine work): every number is rounded (exact, `jsonround.rs`) and then printed through `format!`, one allocation per number. In the list route the CLI then split that text into tokens again and built the wide CSV with one `String` per token and one `format!` per cell (660 000 cells for 275 ions): about a third of the call (fixed in this PR, below).
5. **Decoding is the floor of a one-pass design.** One pass is 85 ms per file on one thread, of which 35-52 ms are the mzML decode (75-110 MB/s per core); the rest (2 x 660 000 window searches, 660 000 rounded values, 275 integrations) is not split further. Allocation and cache misses are not visible at this scale: peak RSS stays at 55-82 MB.
6. **Memory mapping would not help.** Re-reading a file already in the page cache costs almost nothing (`measurements-one-pass.md`: 9 ms for 53 MB); the cost is CPU in decoding, so `mmap` (and its `unsafe`) buys nothing here.

## What MZmine does (public documentation only, not run)

- Raw data: "Keep in Memory" defaults to none, i.e. spectra are memory-mapped to a temporary folder; mass lists can be kept in RAM; an advanced import applies mass detection while reading so "only loading those filtered mass spectra" ([performance](https://mzmine.github.io/mzmine_documentation/latest/performance.html)).
- Mass detection builds "a mass list (...) for each scan, in each raw data file" (a full pass) before chromatogram building ([mass detection](https://mzmine.github.io/mzmine_documentation/latest/module_docs/featdet_mass_detection/mass-detection.html)).
- ADAP chromatogram builder: per data file, MS1 only, all data points sorted by decreasing intensity, each joined to an EIC within the scan-to-scan m/z tolerance ([ADAP](https://mzmine.github.io/mzmine_documentation/latest/module_docs/lc-ms_featdet/featdet_adap_chromatogram_builder/adap-chromatogram-builder.html)).
- Targeted feature detection: a CSV/TSV of targets (m/z or formula/SMILES, RT), a search window of m/z and RT tolerance per target, the best candidate in the window checked for shape, overlapping targets merged ([targeted](https://mzmine.github.io/mzmine_documentation/latest/module_docs/lc-ms_featdet/targeted_featdet/targeted-featdet.html)). The page does not say whether scans are walked once per file or once per target.
- Batch mode: one JVM process for the whole batch (`-batch`, `-input` glob), concurrent tasks per file (`-threads`); "the startup counts towards" the runtime ([command line](https://mzmine.github.io/mzmine_documentation/latest/commandline_tool.html)).
- What makes it fast in the field run is structural, not Java: one process, the files read once into memory-mapped scans, every target extracted from the scans already in memory, files in parallel. The mzLab per-call route did the opposite on every point.
- Field figure: 15-25 s per polarity for about 245 files, i.e. 61-102 ms of wall per file on the owner's machine. The one-pass prototype here: 21 ms of wall per file on 4 cores (84 ms of CPU per file), including the integration. Different machines and data: a same-machine comparison stays to be made by the owner.

## Best-in-class design

| element | what it removes | measured or expected gain (baseline above) |
|---|---|---|
| one process, one pass per file extracting every target trace | 1 200 passes per file down to 1; the index sort; the text round trip | measured: 5 789 s -> 20.7 s on one thread (280x), output byte-identical |
| bounded thread pool over files (one file per worker, results in input order) | idle cores | measured: 20.7 -> 5.1 s on 4 cores (4.1x); RSS 55 -> 82 MB |
| integrate the arrays in memory, never through JSON/CSV text | 0.3 ms of render and re-parse per trace (20 s for 67 375 traces in one thread) | included in the prototype |
| scans sorted by RT with the m/z already sorted in each scan; targets sorted by m/z | the per-file peak sort of the `.mzidx` (part of 48-68 ms) | included: the prototype cuts each window by binary search in the scan |
| from an index already built (site OPFS, `--cache`): XIC from the index, no decode | the decode floor (35-52 ms per file) | expected: about 3 ms open + 275 x under 0.1 ms (the steady-state route minus its render) = 20-30 ms per file -> 1.3-2 s for 245 files on 4 cores (the SHA-256 of each file adds about 0.6 s) |
| lazy decoding: skip the scans outside the union of the RT windows (headers only) | decode of unused scans | none for this list (the windows cover the run); up to the uncovered fraction for short lists |
| branch-free window cut: merge walk of sorted targets against the sorted scan | 2 binary searches per target and scan | expected: window cut about -50 % (a few ms per file) |
| SIMD for the sparse ion-count sums | | small: about 55 peaks per scan and one channel per window; worth it only on HR profile data |
| persistent trace cache keyed by SHA-256 of the file + target list + parameters + engine version, byte-identical with and without it | everything but the SHA-256 on a repeated run | expected: second run about the hashing time (954 MB, about 0.6 s on 4 cores) |
| faster mzML decode (base64 and XML scanning specialised for `binaryDataArray`, zlib-rs already in) | the new floor | expected 1.5-2x on decode, i.e. 5.1 -> about 3.5 s |
| memory-mapped input | nothing measurable (page cache) | none: not worth the `unsafe` |

### WASM constraints for the site

- No threads in the `.wasm` (no `SharedArrayBuffer` on GitHub Pages; `wasm-potential.md`); parallelism is the message-based worker pool, which already scales x4 across files: the pool over files is the site's form of the thread pool.
- Each worker is a separate instance with its own memory and the module caps of `wasm-caps.md`: a one-pass extraction must stream (the prototype peaks at 55 MB per file natively) and keep only the target traces, never the whole file.
- On the site the files are already indexed in OPFS once loaded, so the from-index path is the natural one there: no decode at all, only the windows of the targets read in blocks.
- SIMD128 is on for every module (+5-15 % on read and index); relaxed SIMD is not usable (results differ across CPUs).
- The same Rust step (one pass or from-index) serves the CLI, the app and the site, with the byte-for-byte parity test of every new function.

## Quick win in this PR

`trace xic --mz LIST --format csv` (the best route an agent has today: one call per file for up to 500 ions) built its wide CSV with one owned `String` per token and one `format!` per cell. The cells are now the route's tokens, borrowed and pushed once (`trace_xic.rs` `wide_csv`).

| measure | before | after | % |
|---|---|---|---|
| `trace xic FILE --mz <275 ions> --da 0.5 --format csv`, synthetic LR file, 15 alternated runs, median (min) | 357 ms (330) | 238 ms (229) | -33 % |

Bytes identical by SHA-256 on the CSV, the CSV with `--rt 3,12` and the JSON `trace-set/1` of the same call; `trace_xic` unit tests unchanged.

## Roadmap (ranked)

| id | what | expected speed-up | size | risk | depends on |
|---|---|---|---|---|---|
| SP1 | bulk targeted step (QUANT-TARGETS): one process, one pass per file over every target, worker pool over files, integration on the arrays, CLI `--json` + site + app with parity | 5 789 s -> 5.1 s measured (about 1 100x against the per-call route, 230x against the cached one), 3-5x under the field MZmine figure | M | low: the prototype is byte-identical to the per-call bytes | - |
| SP2 | from-index path in SP1 when an index exists (site OPFS, `--cache`): XIC from the `.mzidx` arrays (`xic_raw`), no decode | about 4x on top of SP1 (expected 1.3-2 s for 245 files) | S | low | SP1 |
| SP3 | persistent trace cache: key = SHA-256 of the file + targets + parameters + engine version; stored traces and areas byte-identical with and without the cache | a repeated run costs the hashing only (expected under 1 s) | M | medium: invalidation, disk use (opt-in like `--cache`) | SP1 |
| SP4 | RT-window lazy decode in SP1: scans outside every window not decoded | 0 % on wide lists, up to 5-10x on short lists | M | low | SP1 |
| SP5 | `Out::num` writes into the string (`write!`) instead of `format!` per number | expected -20-30 % of every JSON render (0.27 ms per 4800-number trace) | S | low: same `Display` bytes; touches every `.wasm`, check the caps | - |
| SP6 | cache hit of the per-call route: open the verified entry read-only in place instead of copying it | expected 17.6 -> about 10 ms per call (3.0 -> 2 passes) | S | medium: the copy was chosen so nothing a caller writes reaches the cache | - |
| SP7 | faster mzML decode for the one-pass floor (base64, `binaryDataArray` scanning) | expected 1.5-2x on decode (5.1 -> about 3.5 s on 4 cores) | L | medium: reader parity tests | SP1 |
| SP8 | merge-walk window cut in SP1 (sorted targets against the sorted scan) | expected a few ms per file (about -10 % of SP1) | S | low | SP1 |
| SP9 | one large HR file across several workers (scan ranges) | only for files much larger than the pool's share | L | medium | SP1 |
| SP10 | `mmap` of the input | none measurable | S | `unsafe` | discarded |

### Note for QUANT-TARGETS (the bulk step)

- Do not go through the routes' text: integrate the `f64` arrays (after the same `json::round(rt, 4)` and `json::round(y, 1)` the route applies, which keeps the areas byte-identical to `trace xic | peak auto-lr`).
- One file per worker, all targets per file in one decode (`mzml::parse` callback, MS1 only, window `lo <= m/z <= hi` summed in m/z order as `Index::sum_window`), results gathered in input order: 245 x 275 in 5.1 s on 4 cores, 82 MB peak, 1.00 pass per file.
- When an index already exists (OPFS on the site, `--cache` in the CLI), read the targets from it instead of decoding (SP2).
- Peak memory is bounded by one file per worker plus the target traces (2400 x 275 x 8 B = 5 MB per file).

## Reproduce

```
cargo build --profile cli -p mzlab-cli --bin mzlab-cli --example speed_targets
python3 tools/bench/speed_study.py /tmp/speed --no-build --keep    # about 8 minutes on 4 vCPU; WORK/speed_study.json + the table
```
