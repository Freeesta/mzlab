# mzFlow Engineering & Scientific Roadmap

Anchored to `origin/main` (`88cf7b6a5`) and paired with [`CONFRONTO_MZFLOW.md`](./CONFRONTO_MZFLOW.md).

Status vocabulary used throughout this roadmap:
- `done / in Wave 1` — scoped and implemented on dedicated Wave 1 feature branches (`claude/flow-delta-formula`, `claude/flow-compare-tables`, `claude/flow-stats-cluster-order`); numbers reference branches directly without invented PR IDs.
- `in progress (PR #3208)` — open PR implementing the item on `origin/main`.
- `proposed` — approved architectural direction with design sketch, size (`S`/`M`/`L`), and `.wasm` cap impact (rule: **never raise caps**, **`hr` (`1,072` B headroom) and `table` (`3,142` B headroom) stay at `+0 B`**, modified lazy modules stay `< +10%`).
- `rejected` — explicitly out of bounds due to epistemological guardrails, offline-privacy constraints, or `.wasm` budget ceilings.

---

## (A) Code Improvements

### A1. Speed
- **Flash `.mzlib` v1 inverted-index MS2 & analogue matching** (`done / baseline on origin/main`, `crates/mzlab-lib/src/flash.rs:1-35`): flat cache-friendly `FlashAcc` bucketed accumulators score fragment and neutral-loss queries without full-matrix allocations.
- **Analogue `delta_formula` resolution in `modified_cosine`** (`done / in Wave 1` — branch `claude/flow-delta-formula`, **S**, `lib` `.wasm` `~+2 KB`): fast bounded CHNOPS difference solver inside `crates/mzlab-lib/src/flowmatch/mod.rs` avoiding heavy molecular graph dependencies.
- **Stream `.mzidx` apex scans inside lazy `formula` `.wasm`** (`proposed` — Wave 3, **M/L**, `formula` `.wasm` `~+5–8 KB`, `hr`/`untargeted` `+0 B`): replace full `.mzML` buffer reads in `flow/lazy/flow-web-ident.js:143-169` with range-sliced `Index<R: ReadAt>` lookups (`flow_formula_ms1.rs`), eliminating repeated XML decompression and unlocking cohorts `> 256 MiB` in-browser.
- **Range-indexed `spectra.ms2@1` extraction in browser OPFS** (`proposed` — Wave 3, **M**, `hr` `+0 B` via lazy `formula`/`flowtab` or small helper without touching `hr` cap): migrate `spectra.ms2@1` (`flow-web-ident.js:52-76`) from whole-mzML `arrayBuffer()` calls (`MAX_MZML = 256 MiB`) to OPFS `.mzidx` slice extraction where MS2 scans are already indexed.
- **Parallel slab workers across multi-file `features.detect@2` in browser** (`proposed` — Wave 3, **M**, `+0 B` `.wasm`): fan out independent sample `.mzidx` detection jobs across existing WebWorker pool slots up to `MemBudget` RAM/N caps.

### A2. Memory & `.wasm` Budget Discipline
- **Out-of-core slabbed detection & alignment** (`done / baseline on origin/main`, `crates/mzlab-feat/src/trace.rs:31-36`, `crates/mzlab-feat/src/slabs.rs:1-34`): `budget = 96 MiB` detection slabs (`SLAB_PEAKS = 1_500_000`, `SLAB_GAP_PPM = 50.0`) and `LEAN_ROW = 39` / `WORK_ROW = 40` alignment streaming keep linear memory predictable on desktop and OPFS.
- **Browser `table.plausible@1` execution reusing `table` `.wasm`** (`in progress (PR #3208)`, **S**, `+0 B` `.wasm`): wires `F.liveJobs["table.plausible"]` to existing `table_plausible` export without expanding `table` (`734,138 / 737,280` B, only `3,142` B left).
- **Isolated table & stats growth inside `flowtab` and `flowstat` `.wasm`** (`done / in Wave 1` + `proposed` — Wave 2, `flowstat` `~+1.5 KB` for UPGMA; `flowtab` `~+6–8 KB` for PQN & batch blank/QC-RLSC): preserves `table` and `hr` at `+0 B` (`tools/build_feat_wasm.sh:39-44`).
- **Automated `twiggy` size audit & symbol trimming on `hr` and `table`** (`proposed` — Wave 2, **S**, target `-5 KB` to `-15 KB` recovery inside `hr`/`table` via `tools/twiggy_wasm.sh`): frees safety margin without altering caps or behavior.
- **Fusing `flow_formula` into `hr` or `untargeted` or raising any `.wasm` cap** (`rejected`): `crates/mzlab-feat-wasm/Cargo.toml:17` explicitly documents `+145 KB` over `hr` and `+134 KB` over `untargeted`; caps are hard architectural contracts (`docs/agenti/wasm-tetti.md:36`).

### A3. Architecture & Modularity
- **Extended `meta.compare@1` comparator for `.mzfeatf` and `.mzfeat` binaries** (`done / in Wave 1` — branch `claude/flow-compare-tables`, **M**, `+0 B` `.wasm`): extends `crates/mzlab-cli/src/flow_compare.rs:292-318` cleanly via binary header dispatch so `detect` and `align` sweeps share one `compare/1` contract.
- **Close remaining `SOLO_APP` and `HOLES` gaps cleanly** (`proposed` — Wave 2/3):
  - `annot.hazard@1` (`proposed` — Wave 2, **S**, `attributio-all` `~+1–2 KB`): clears `"annot.hazard"` from `crates/mzlab-cli/src/flow_rule_tests.rs:16` and `tests/test_parita_passi.py:16`.
  - `spectra.chimeric@1`, `rt.predict@1`, `quant.calibrate@1`, `chrom.uv_peaks@1` browser executors (`proposed` — Wave 3, **M** each, hosted inside lazy `flowtab`/`flowstat`/`attributio-all` or dedicated lazy modules, `hr`/`table` `+0 B`): closes `SOLO_APP` (`tests/test_parita_passi.py:19-24`).
- **Unify `untargeted` baseline/smoothing/mass-detector plumbing into `features.detect@2`** (`proposed` — Wave 2, **S–M**, `untargeted` `~+3–5 KB`): eliminates dead-code drift between `untargeted/{arpls,localmax,smooth}.rs` and `steps/detect.rs:328-385`.
- **Unlock `BlankOpts::Matching::{Batch, Matrix}` in `filters_blank.rs`** (`proposed` — Wave 2, **M**, `flowtab` `~+2–4 KB`): completes the enum contract already exposed in `steps/table.rs:281-285` instead of refusing at `filters_blank.rs:341`.

### A4. Test Coverage & Parity
- **Byte-for-byte determinism guards (`libm`, `SortC`, zero `significant`)** (`done / baseline on origin/main`, `docs/agenti/parita.md:8-10`, `crates/mzlab-feat/tests/stat.rs:328-330`).
- **Wave 1 & Wave 2 golden + WASM parity suites** (`done / in Wave 1` & `proposed` — Wave 2, **S** each):
  - Wave 1 unit + WASM parity tests for `delta_formula`, `.mzfeatf`/`.mzfeat` `meta.compare`, and UPGMA `rowOrder`/`colOrder`.
  - Wave 2 NumPy reference cross-checks (`tools/`) for PQN (`norm.rs`), `arpls`/`localmax` Flow wiring, batch blank/QC-RLSC drift, and `annot.hazard@1` snapshot parity.
- **Mac real-file regression benchmark suite (external to git)** (`proposed` — Wave 2, **S**, `+0 B`): automated local script recording aggregate `meta.compare@1` overlaps, `delta_formula` hit counts, peak RSS, and wall time on multi-injection pooled-QC + blank cohorts (results logged as counts only).
- **Bypassing WASI/browser byte parity for convenience** (`rejected`): parity across CLI, desktop, and `.wasm` is a non-negotiable invariant (`CLAUDE.md`, `tests/test_parita_passi.py`).

---

## (B) New Scientific & UX Capabilities Across the 15 Stages

### Stages 1–4: Raw Ingest, Mass Detection, ROI, Peak Resolution
- **Expose `arpls` baseline subtraction & SG/Gaussian smoothing in `adap_localmin`** (`proposed` — Wave 2, **M**, `untargeted` `~+1–2 KB`): wires existing Baek et al. 2015 (`doi:10.1039/C4AN01061B`) and `smooth.rs` solvers into `features.detect@2` (`detect.rs:328-385`) behind `.advanced()` flags.
- **Wire `LocalMaxMassDetector` & explicit `mass_detector` preset** (`proposed` — Wave 2, **S**, `untargeted` `~+2–3 KB`): activates `untargeted/localmax.rs:15-36` alongside `floor` and `factor_lowest`.
- **Direct browser vendor `.raw`/`.wiff`/`.d` streaming `import.raw@1` bypassing `convert.mzml` without `4 GiB` split** (`rejected` for Wave 1–2): `wasm32` 4 GiB address limits and reader module caps make upload-time `.wasm` conversion to OPFS `mzML` + `.mzidx` the safer architecture (`docs/agenti/parita.md:44-45`).
- **Ion mobility (TIMS/CCS/PASEF) feature & mobilogram extraction** (`proposed` — Wave 4+, **L**, separate `ims` lazy `.wasm`): adds 4D m/z-RT-CCS-intensity detection beyond current Bruker metadata extraction.

### Stages 5–7: Isotopes, Adducts, Alignment & Gap Filling
- **Cross-polarity `[M+H]+`/`[M-H]-` linkage inside aligned table views** (`proposed` — Wave 3, **M**, `flowtab` `~+3–4 KB`): elevates `polpair/1` correlation evidence directly onto table rows as non-destructive cross-links (preserving separate polarity rows).
- **RT-warp-aware window lookup during `features.gapfill@1`** (`proposed` — Wave 3, **S**, `untargeted`/`detect` parity check, `hr` `+0 B` if `detect` avoids growth or runs in `untargeted`): feeds fitted `RtMap` shifts from `features.align@1` (`rt_warp != "off"`) directly into gap-fill EIC window centers.
- **Silent merging of adduct/in-source fragment rows into collapsed intensities** (`rejected`): mzFlow explicitly preserves all ion rows inside non-destructive `ionnet/1` families (`features.group@1`, `crates/mzlab-feat/src/steps/group.rs:62-96`) so downstream MS2 and blank checks remain auditable.

### Stages 8–10: Blank Filtering, Normalisation, QC & Statistics
- **Probabilistic Quotient Normalisation (`normalize="pqn"`)** (`proposed` — Wave 2, **M**, `flowtab` `~+2.5 KB`, `table` `+0 B`): implements Dieterle et al. 2006 (`doi:10.1021/ac051632c`) dilution scaling in `norm.rs` with median-quotient QC invariance.
- **Batch/matrix-aware blank subtraction, diluted-QC gating, & cross-batch QC-RLSC** (`proposed` — Wave 2, **M**, `flowtab` `~+4–6 KB`): enables `blank_match="batch"|"matrix"` (`filters_blank.rs:341`), adds `QcDilution` linearity filtering, and adds pooled-QC batch-median bridging (`table.drift@1`, Dunn et al., *Nat. Protoc.* 6:1060, 2011).
- **Hierarchical UPGMA clustering (`rowOrder`, `colOrder`) on `table.stats@1` heatmap** (`done / in Wave 1` — branch `claude/flow-stats-cluster-order`, **S**, `flowstat` `~+1.5 KB`): deterministic Eisen et al. 1998 (`doi:10.1073/pnas.95.25.14863`) leaf ordering with SVG/canvas heatmap + `TDATI` accessibility update.
- **Multi-factor 2-way ANOVA & supervised multivariate PLS-DA/OPLS-DA with permutation CV** (`proposed` — Wave 3, **L**, `flowstat` `~+12–18 KB`, well inside `182,116` B headroom): expands `table.stats@1` / `table.pca@1` for longitudinal or treatment x time designs while maintaining `MIN_REPLICHE >= 3`.
- **Displaying p-values for `< 3` replicates or printing `"significant"`** (`rejected`): permanently banned by `MIN_REPLICHE = 3` (`drawing/mod.rs:15`, `stat/compare.rs:187,229`) and CI word-guard test (`crates/mzlab-feat/tests/stat.rs:328-330`).

### Stages 11–14: MS2, Formula Assignment, Compound ID & Confidence Levels
- **Analogue `delta_formula` annotation in `modified_cosine`** (`done / in Wave 1` — branch `claude/flow-delta-formula`, **S**, `lib` `~+2 KB`): empirical CHNOPS precursor shift annotation at `delta_formula_mda=5.0`.
- **CLI & browser Flow executor for `annot.hazard@1`** (`proposed` — Wave 2, **S**, `attributio-all` `~+1–2 KB`): offline T1–T5 acute/chronic aquatic hazard annotation from local snapshots.
- **In-browser `spectra.chimeric@1` (MCR-ALS / `library_deconvolution`) & `rt.predict@1` execution** (`proposed` — Wave 3, **M**, `attributio-all` or lazy worker module): clears two more `SOLO_APP` entries cleanly.
- **Full DIA SWATH/AIF MS1-MS2 chromatogram deconvolution** (`proposed` — Wave 4, **L**, separate `dia` lazy `.wasm` so `hr`/`untargeted` stay untouched): extends current DIA precursor-window viewer (`alta-risoluzione-statistica.md:22`) into pseudo-MS2 extraction for `spectra.match@1`.
- **Automatic Schymanski Level 2b, lifting LR Level-4 cap, or bundling external cloud DB lookups** (`rejected`): violates human-only 2b curation (`flow_level/mod.rs:3`), LR resolution ceiling (`data/livelli-1.json:377`), zero-upload offline architecture, and `.wasm` size limits.

### Stage 15: Reporting, UX & Reproducibility
- **Feature-level `.mzfeatf` and aligned `.mzfeat` comparison in `meta.compare@1`** (`done / in Wave 1` — branch `claude/flow-compare-tables`, **M**, `+0 B` `.wasm`): native cross-algorithm / cross-preset benchmark metric generation.
- **One-click FBMN / SIRIUS combined bundle export (`export.fbmn_bundle@1`)** (`proposed` — Wave 3, **S**, `flowtab` `~+1–2 KB`): emits synchronized quant CSV (`export.rs:127`) + MS2 MGF (`spectra.ms2@1`) + provenance manifest in a single canvas node.
- **Interactive Flow drawer for manual Level 2b review (`AuditEntry`)** (`proposed` — Wave 3, **M**, `attributio-all` `~+2–4 KB`): brings `attributio/review/bench` human audit decisions (`attributio/annotate/types.rs:163-182`) directly onto Flow run results.
- **Silently altering canonical step defaults (`masstrace`, `groups`, `spectral_entropy`, `hr` preset)** (`rejected`): default presets belong to reproducible benchmarks and product governance; algorithm exploration uses explicit presets and `--set` overrides logged in `mzrun/1`.
