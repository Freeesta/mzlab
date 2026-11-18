# Flow: from mzmine steps to mzLab steps

Working table of the graphical Flow (`docs/agenti/flow-graphics.md`). It says what exists today, without promising: **step** = has a `step/1` descriptor in the palette; **engine** = the computation exists in Rust (`crates/mzlab-feat`, binary `untargeted`) but has no descriptor yet, so the Flow does not put it in a chain; **planned** = in the descriptor it is a `planned` algorithm or a row of the study, with no executor; **none** = does not exist. No row invents an algorithm.

| mzmine step | In mzLab | Status |
|---|---|---|
| Raw data import | `import.raw` (RAW, WIFF, D... files) and `convert.mzml` | step |
| Mass detection: centroid, exact mass | `features.detect` reads the centroids of the index; no separate mass detector | none |
| Mass detection: local maxima | `untargeted/localmax.rs` (from `LocalMaxMassDetector` 4.10.90), outside the pipeline | engine |
| Mass detection: factor of lowest signal | `features.detect`, algorithm `adap_localmin`, parameter `noise_factor` (per-scan threshold, `untargeted/pipeline_stream.rs`); 0 = absolute threshold `floor` | step |
| Mass detection: wavelet, auto, recursive threshold | no implementation | none |
| Scan index for window access | `index.mzidx` | step |
| Chromatogram building (ADAP / ion chromatogram builder) | `features.detect`, algorithm `masstrace` (HR and LR) or `adap_localmin` (ADAP of `untargeted/adap.rs`, bridge `untargeted/flow.rs`: `floor`, `min_height`, `min_points` shared, tolerance `ppm`/`abs_da`) | step |
| Alternative feature detection | `features.detect`: `centwave` (Ricker wavelets on the traces, `centwave.rs`) available; `threshold` | `threshold` planned |
| Parameters from the data | `features.estimate` (noise, m/z tolerance, peak width, dead time) | step |
| Smoothing | Savitzky-Golay in `features.detect` `adap_localmin` (`smooth_window`, 0 = off; only the shape it searches minima on, height and area from the raw data); Gaussian in `untargeted/smooth.rs`; LOESS no | step (SG), engine (Gaussian), none (LOESS) |
| Baseline | `untargeted/arpls.rs`, optional | engine |
| Feature resolving: local minimum | `features.detect` `adap_localmin` (`untargeted/resolve.rs`, from `MinimumSearchFeatureResolver`: `chrom_threshold`, `search_rt`, `min_ratio`, `duration`; the cut of `resolve.rs` does not apply); with `masstrace` the cut of `resolve.rs` | step |
| Feature resolving: ADAP wavelets, Savitzky-Golay, baseline resolver, noise amplitude | no implementation | none |
| Deisotoping: isotope grouper | `features.detect`, parameter `isotopes`: `averagine` by default (`isotopes.rs`, natural-abundance ratios), `grouper` = the mzmine grouper (`untargeted/isotopes.rs`, bridge `untargeted/flow_iso.rs`: `iso_ppm`, `iso_abs_da`, `iso_rt_tol`, `iso_max_charge`, `iso_monotonic`, `iso_representative`; steps of 1.0033 Da with no ratio check); group and isotopologues only marked, never discarded; also for weak candidates | step |
| Deisotoping: isotope finder 4.10 | `features.detect`, `isotopes` = `finder` (`untargeted/isofinder.rs`, bridge `untargeted/flow_isofinder.rs`: `finder_ppm`, `finder_abs_da`, `finder_max_charge`, `finder_elements`, `finder_require_c13`, `finder_auto`): the MS1 spectrum at the apex of each feature, from the most intense; rows whose trace covers that scan and whose m/z is a peak of the winning pattern join the group; pattern charge; also for weak candidates; refinement across scans and mobility not done | step, engine |
| Deisotoping: isotope finder, candidate search | `find_candidates` (`untargeted/isofinder.rs`): the descent starts from the point below the target when the m/z values are finite, `max_diff` ≥ 0 and every isotopic difference exceeds the tolerance between the target and the top of the spectrum, otherwise from the top; same candidates bit for bit (`tests/untargeted_isofinder_start.rs`). 500 targets × 3 charges, median of 11: 145 → 15 ms on 9,500 peaks, 9.7 → 5.9 ms on 300 peaks in 24 Da, 1.7 → 0.9 ms on 60 | engine |
| Alignment: join aligner | `features.align`, algorithm `join` (`untargeted/join.rs`, bridge `untargeted/flow_align.rs`: `mz_weight`, `rt_weight`, `rt_tol`, tolerance `ppm`/`abs_da`; rows per ion in any `mode`, in memory only, so a series that does not fit is rejected); `groups` by default ("per ion" and "per family" modes) | step, engine |
| Alignment: RANSAC, RT correction between samples | `features.align`, parameter `rt_warp` (`piecewise`, `loess`, `ransac`; `crates/mzlab-feat/src/rtwarp.rs`): anchors from a coarse alignment (`rt_coarse_gap`), curve per file, RT brought back to the common one before grouping; `off` by default | step |
| Gap filling (same RT and m/z) | `features.gapfill`, algorithm `integrate` by default; algorithm `peak_finder` (`untargeted/gapfill.rs`, bridge `untargeted/flow_align.rs`: `intensity_tol`, tolerance `ppm`/`abs_da`, `rt_tol`, `min_points`, `floor` shared; `max_gap`, `long_trace` and the RT map do not apply) | step, engine |
| Duplicate feature filter | `table.filter`, parameters `dup_ppm` and `dup_rt` (rule `duplicati`, `filters.rs`): keeps the strongest row within the window; alignment already discards FT lobes | step |
| Feature list rows filter | `table.filter` (minimum samples, area, height, blank ratio, MS2 required) | step |
| Blank subtraction | `table.filter`, parameter `blank_ratio`; `bianco.js` in the HR Data view | step |
| Normalization | `table.filter`, parameter `normalize` | step |
| Ion mobility | no processing (the Bruker reader reads the data) | none |
| Adduct, complex, fragment search | ion families in `features.align` ("per family"), `ionfamily` | engine |
| Correlation grouping (metaCorrelate), ion identity networking | no step; family links are in `features.align` | none |
| MS2 scan pairing | `features.ms2_link` | step |
| MS2 spectral library search | `spectra.match` (spectral entropy, cosine, modified cosine) | step |
| Local compound database search | `annot.regulatory` covers only lists and suspect screening, not an arbitrary local database | partial (step) |
| Lipid annotation | none | none |
| Formula prediction | `formula.ms1`, `formula.ms2` | step |
| SIRIUS / CSI:FingerID | none; no external service is called | none |
| In silico fragmentation | `spectra.insilico` | step |
| Identification level | `annot.level` (Schymanski level cascade) | step |
| Export: CSV | `export.table_csv` | step |
| Export: GNPS FBMN/IIN, SIRIUS, mzTab-M | only the interface exports (`attributio-export.md`), no step | none |
