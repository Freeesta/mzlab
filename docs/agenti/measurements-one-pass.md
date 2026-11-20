# One pass over a dropped mzML: bound, spike, design

The site loads a plain mzML in two passes: the copy to OPFS (`rust-carica.js` `copyPrints`, 8 MiB pieces, SHA-256 and probe on the side), then the builder (`idx-worker.js`) reads the copy again for the scan table (`prepare_upload`) and for the index (`build_mzidx`). The one pass hands each copied piece to a builder that runs during the copy.

Measured with `tests_e2e/misura_una_passata.py` (not run in CI). Synthetic mzML from `tools/dati_sintetici.py` (P53 = 53 MB, 2 400 scans; P489 = 489 MB). Maker formats (`.raw`, `.wiff`) are out of scope: their path already decodes once (`rows_before` + `build_after`).

Machine: cloud container, 4 vCPU, 15 GB, headless Chromium (Playwright), warm persistent profile (service worker filled), cross-origin isolated. Each run is a fresh copy of the warm profile and drops one file. Files, arms and CPU rates are interleaved run by run. Numbers are medians of 5 runs [min-max], in ms from the drop. CPU 4× is CDP throttling, which slows the page's main thread only. Safari and Firefox are not available in the cloud and were not measured.

## Bound (the two passes)

Phases come from the `mzlab-index` events: copy (`load`), scan table (`scan`), builder read with decoding (`read`) and rounds over the scratch file (`index`). Afterwards a worker reads the OPFS copy alone (I/O, read-only SyncAccessHandle in 4 MiB blocks). Gain ≤ min(scan + read, copy + I/O). The CPU-aware bound (`bound_cpu`, from /proc every 25 ms) puts the builder's work only on the cores the copy leaves idle.

| file | CPU | drop→indexed | drop→prov. TIC | copy | scan | read | rounds | I/O of the copy | bound | bound % | bound_cpu % |
|---|---|---|---|---|---|---|---|---|---|---|---|
| P53 | 1× | 1821 [1718-1951] | 565 | 334 [302-474] | 149 | 792 | 0 | 9 | 343 | 18.8 [18.1-24.9] | 18.8 |
| P489 | 1× | 10011 [9569-10716] | 2801 | 2183 [2045-2615] | 526 | 3678 | 2852 | 73 | 2254 | 22.5 [22.1-25.4] | 22.5 |
| P53 | 4× | 3186 | 750 | 354 | 197 | 938 | 0 | 10 | 364 | 11.8 | 7.9 |
| P489 | 4× | 11523 | 3179 | 2259 | 630 | 3842 | 2854 | 78 | 2348 | 19.7 | 12.6 |

- The re-read of the copy costs almost nothing: 9 ms for 53 MB and 73 ms for 489 MB, because the copy is still in the page cache. The builder phase is decoding CPU, so the gain can only come from overlapping that decoding with the copy, never from skipping I/O.
- Busy cores at 1×: about 1.7 during the copy and 1.8 during the builder (4 cores), so the overlap fits without starving the copy. At 4× both phases use about 2.4 cores.
- The bound passes the 15 % threshold at 1× on both sizes. At 4× it passes on P489 only.

## Spike (off by default)

Arms: `due` = the two passes, `una` = the spike. The spike is on only when the OPFS directory `mzlab` holds the switch file `una-passata.on` (`tests_e2e/misura_una_passata.py --arms due,una`). Same machine and method as the bound, 5 interleaved runs per row, 40 runs and no failures. RSS is the peak of the summed RSS of the browser's processes. `live` = 8 MiB pieces that went through the ring during the copy; `from_file` = MB the builder read from the dropped File.

| file | CPU | arm | drop→indexed | drop→prov. TIC | copy (bar 0-40 %) | peak RSS MB | busy cores in copy | live | from_file |
|---|---|---|---|---|---|---|---|---|---|
| P53 | 1× | two-pass | 1807 [1603-2066] | 581 [514-726] | 324 | 1518 | 1.89 | - | - |
| P53 | 1× | one-pass | 1522 [1354-1735] (−15.8 %) | 609 [572-711] | 538 | 1550 | 3.02 | 5 | 17 |
| P489 | 1× | two-pass | 10731 [10016-10798] | 2833 [2783-3231] | 2242 | 1690 | 1.86 | - | - |
| P489 | 1× | one-pass | 9742 [9265-10676] (−9.2 %) | 3213 [3028-3287] (+380) | 3074 | 1743 | 2.79 | 11 | 393 |
| P53 | 4× | two-pass | 3153 [2877-3309] | 773 [632-827] | 350 | 1517 | 2.61 | - | - |
| P53 | 4× | one-pass | 2738 [2638-2936] (−13.2 %) | 734 [686-853] | 562 | 1547 | 3.30 | 4 | 17 |
| P489 | 4× | two-pass | 11911 [11791-12649] | 3085 [2941-3308] | 2308 | 1686 | 2.54 | - | - |
| P489 | 4× | one-pass | 12040 [11187-12658] (+1.1 %) | 3656 [3392-3844] (+571) | 3346 | 1766 | 3.18 | 10 | 401 |

- `.mzidx` SHA-256: one value per file in all 20 runs of both arms (P53 `8f9111013d74…`, P489 `51ac26f5f1c5…`). The one pass held in all 20 spike runs.
- The copy of the spike includes the scan table, which is made at the end of the copy, before the bar moves on. During the copy the builder and its decoding helpers take the idle cores (3.0 busy instead of 1.9), so the copy stretches.
- On P489 the provisional TIC therefore comes later: +0.4 s at 1×, +0.6 s at 4×.
- The builder decodes more slowly than the copy writes. Only 10-12 of 59 pieces of P489 fit the 32 MiB ring; the rest come from the File (`FileReaderSync`).
- The rounds over the scratch file (about 2.9 s on P489) stay after the copy, as in `due`. They bound what one pass can save on large files.
- The peak RSS grows by 32-80 MB: the 32 MiB ring plus one more builder worker.
- A first version skipped every piece after the first full ring, which starved the builder for the rest of the copy. It measured P489 at 1× 10579 → 10711 ms (+1.2 %).

## Design

- **Rust** (`crates/mzlab-core/src/mzidx/streamed.rs`, `scantable/guess.rs`; export `build_mzidx_streamed` in `crates/mzlab-wasm/src/streamed.rs`):
  - `build_mzml_streamed` runs pass 1 (`First`) on the bytes as they arrive, using each spectrum's level and polarity.
  - The `hr1` of the level-1 groups is guessed from the file head (instrument configurations) and the filter string of the first MS1 scan (`HrGuess`).
  - At the end it asks for the scan table and checks it against what pass 1 saw (hr1, the number of scans, every level and polarity). If they agree, the rounds follow (`build_after`).
  - On any difference, or a failure of pass 1, it falls back to `build_mzml_corriere` from `again`. The bytes are therefore always those of that path (`crates/mzlab-core/tests/mzidx_streamed.rs`).
  - The parser reuses the instantiation of `build_mzml_corriere` (`late::Fermabile`), so the engine `.wasm` grows by +8.6 KB optimised (+0.39 %).
- **Worker glue** (`una-passata.js` in the engine worker, `una-passata-lettore.js` in the builder):
  - The ring is 4 SharedArrayBuffer slots of 8 MiB. The copy pushes each piece it wrote, before the hash worker takes it by transfer.
  - A piece that finds the ring full is skipped, so the copy never waits for the builder. The builder reads every gap, and whatever is left once the copy ends, from the dropped File (`FileReaderSync`, one read per gap). The builder decodes more slowly than the copy writes, so on P489 most bytes come from the File (`from_file` in the console line).
  - At the end of the copy the engine worker makes the scan table (`prepare_upload` on the copy) and sends it through the ring after the last byte of the file. The provisional TIC comes from that table.
  - The builder waits with `Atomics.wait`; its synchronous wasm call cannot open an OPFS handle.
  - Later rounds read the scratch file (`.sp.part`). A round that decodes again, and the fallback, read the dropped File (`FileReaderSync`), not the OPFS copy.
  - The builder keeps its budget, its spill and its decoding helpers.
- **Failure and cancel**:
  - A failed copy stops the builder (`abort`), and `lr_cancel` stops it too (`una.cancel`).
  - A scan table that fails sets the ring's FAIL flag, so the builder throws.
  - Any fatal end of the builder (an unreachable trap, out of memory, a worker error) answers null, and `lr_index` builds with the pool. A key error of the builder (a damaged file) is the answer.
- **Fallback**: without cross-origin isolation, or for any file that is not `.mzML`, loading stays the two passes.
- **Desktop/CLI**:
  - There is no copy: the file is mapped (`Mapped::open`), the table is made with SHA-256 on a second thread (`upload::prepare_parallelo`), then `build_mzml_spill` runs.
  - The maker formats already decode once (`upload::with_parallel_prints`).
  - Nothing changes there: native output stays byte-identical (`mzidx_streamed.rs` compares the streamed build with `build_mzml_corriere`).
