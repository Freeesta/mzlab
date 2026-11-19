# Flow measurements by input format

Time of index → detect → align → gapfill on 12 synthetic files (1365 scans, 185544 peaks each, series of `tools/dati_sintetici.py`) rewritten in the format to be tested with `tools/measure_formats.py genera`; `tools/measure_formats.py misura BIN DIR...` makes a single `mzlab-cli flow run` with 4 workers and sums the per-instance `ms` of `prov.json`. Binary `--profile cli`, main `3e5dd52cc51d6b74305af40a1558a4de0d387e8a`, 4 CPUs, no SHA-NI, files in the system cache (a cold cache cannot be obtained in the container); the noise between two runs is ±15 %.

| format | MB | index s | detect s | align s | gapfill s | total s (wall) | MB/s (wall) | index MB/s per core |
|---|---|---|---|---|---|---|---|---|
| mzML raw | 74 | 1.29 | 0.51 | 0.05 | 0.09 | 1.07 | 69 | 57 |
| mzML raw indexed | 75 | 1.10 | 0.45 | 0.05 | 0.09 | 0.98 | 77 | 68 |
| mzML zlib | 73 | 1.63 | 0.47 | 0.05 | 0.09 | 1.14 | 64 | 45 |
| mzML zlib indexed | 75 | 1.39 | 0.44 | 0.04 | 0.10 | 1.03 | 73 | 54 |
| mzXML raw | 54 | 1.73 | 0.48 | 0.05 | 0.09 | 1.01 | 54 | 31 |
| mzXML zlib | 44 | 2.49 | 0.47 | 0.05 | 0.09 | 1.36 | 32 | 18 |
| ANDI/AIA .cdf (*) | 19 | 0.88 | 1.17 | 0.47 | 0.61 | 1.82 | 10 | 22 |

How to read the columns: the seconds per step are the sum over the 12 instances (CPU time, not wall time); "total" is the wall time of the whole run with 4 workers; "index MB/s per core" is MB / index s. (*) The .cdf is written by a CDF1 encoder in `measure_formats.py` (it is not an instrument file) and has no analyser: the detection recipe is the unit-resolution one, so detect, align and gapfill are NOT comparable with the other rows; only the index column is valid.

## The worst and why
- **Slowest: mzXML with zlib, index step, 18 MB/s per core** (mzXML raw 31, mzML 45-68). Profile (callgrind, one file): `MzxmlReader::completa` runs 3 times for every scan (from `scansione`, `indice` and `cromatogramma`, 21 % each of the total) and every time it decodes base64 and inflates the peaks; in addition memset/memcpy 24 %, character search and UTF-8 conversion 12 %, SHA-256 + SHA-1 of the file 26 % (in mzML the SHA-1 is gone, and without SHA-NI the cost here is overestimated).
- The detect step is the same for all centroided formats (0.44-0.51 s): the format weighs only on the index.
- Align and gapfill do not depend on the format.

## Bruker timsTOF `.d` (synthetic DDA-PASEF)
`cargo run --release -p mzlab-lettori --features bruker --example bruker_bench -- scrivi X.d --cicli 100` writes 1100 frames (100 MS1 of 60 000 points, 1000 PASEF of 15 000 points with 12 precursors), `analysis.tdf_bin` 80 MB in zstd (codec 2); `misura X.d` gives the ms per phase and the FNV signature of all points (equal before and after every change). 4 CPUs, files in the system cache, noise ±5 %.

| phase (12 100 spectra, 16 M points) | native ms | `.wasm` in Node (mzML) |
|---|---|---|
| open (tables) | 16 | — |
| index (TIC of the MS2: decoding of every frame) | 423 | — |
| BPC | 245 | — |
| all spectra | 910 | — |
| mzML (`readers_mzml::scrivi`, new open) | 1 610 | 6 040 |

- Profile (callgrind, 88 frames, all phases): frame decoding is 50 % of the instructions and zstd is 58 % of that; the deflate of the written mzML 33 %. The merge by TOF index (11-bit radix of 1024 points, `spettri::sort_tof`) and the cut by scans (offset `Picchi::inizio`) weigh less than 10 %, against the 41 % of point-by-point sorting and filtering that was there before (same measures: index 670 → 423 ms, BPC 335 → 245, all spectra 1 450 → 910, mzML 2 560 → 1 610; `.wasm` 8.97 → 6.04 s, same SHA-256).
- The index step decodes every PASEF frame once for the TIC of the MS2, and the conversion decodes them again: the double pass remains.

## Not measured (the files of Federico's Mac are needed)
Thermo .raw (our reader), Agilent .D, real Bruker .d, SCIEX .wiff/.wiff.scan, instrument .cdf and MGF: the container has no real files and no converters. The ratio with mzmine (series 23-29: 0.56-0.64) has no number in the repository, so it is not reported.

## Loading multi-sample SCIEX files (.wiff + .wiff.scan)
Synthetic pairs from `tools/wiff_grande.py` (3000 scans × 3333 profile points per sample, 16 MB of `.wiff.scan` each); 4 CPUs, 16 GB, Chromium with an on-disk profile (a normal window), the site's Rust engine, `tests_e2e/e2e_wiff_240.py` (`MZLAB_CARICO_DIR=FOLDER` measures a folder with `counts.json` {file: samples}). The empty browser already takes ≈1.1 GB; the peak is the sum of the Chromium processes.

| load (site) | samples | s | s/sample | peak MB | OPFS GB |
|---|---|---|---|---|---|
| 1 file × 24 samples | 24 | 49 | 2.0 | 1868 | 5.2 |
| 4 files × 11 samples | 44 | 84 | 1.9 | 1969 | 9.6 |
| 11 files (10 × 11 + 1 × 24) | 134 | ≈240 | 1.8 | — | 26 (disk full) |
| 240 small pairs (60 × 300) | 240 | 41 | 0.17 | 2200-2900 | 0.14 |

- **Space**: the index of a profile sample weighs 191-218 MB (about 12 times its `.wiff.scan`): 240 samples need ≈46 GB of OPFS. With 134 samples the container's disk filled up at 25.7 GB: 117 loaded, 17 with `error.quota` at 96-99 % of their index (each sample fails on its own, no reload).
- The message says "10240 MB left" because the browser quota exceeds the real disk space. A private Chromium window holds about 200 MB of OPFS.
- **Builders**: at most 3 together (peak observed with 4 CPUs); the page memory (JS heap) stays under 230 MB.
- **Reopening**: 0.15 s per saved sample (240 samples: 36 s); the limit of the start screen (`?ripresa=ms`) is 30 s plus 1 s per saved file, counted from when the worker has the engine (`api.up` of `rust-bridge.js`; a failed download or one over 5 min makes it start at once, and a failed download always ends in the error).
- **List**: the order is that of `ordine.js` (time, concentration or natural order of the names) applied to the whole list, not the arrival order: the same with the files left in reverse order.

| load (`mzlab-cli file convert --peaks mzlab`) | `--workers` | s | s/file (median) | max RSS MB |
|---|---|---|---|---|
| 240 pairs of 1 sample (1200 × 3333) | 1 / 2 / 4 | 498 / 249 / 133.5 | 2.05 / 2.04 / 2.15 | 23 / 37 / 53 |
| 1 file × 11 samples | 1 / 4 | 58.5 / 18.8 | — | 31 / 102 |
| 11 files, 134 samples, 1.7 GB of mzML | 4 | 220.7 | — | 174 |

The mzML bytes are identical with 1, 2 and 4 workers (SHA-256 of all files). Gaps: desktop (no build in the container), Shimadzu PDA (the generator does not write it), 3333 points per scan instead of the ≈3900 real ones.
