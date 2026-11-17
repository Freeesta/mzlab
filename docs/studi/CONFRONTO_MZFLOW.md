# mzFlow vs. MZmine and the Commercial Reference Suite

Audit anchored to `origin/main` (`88cf7b6a5`). Compares **mzFlow** (`mzlab` CLI, desktop, browser `.wasm`+OPFS) against **MZmine** (`[S23]`) and **the commercial reference suite**.

Aliases: `core/`=`crates/mzlab-core/src/`, `feat/`=`crates/mzlab-feat/src/`, `st/`=`crates/mzlab-feat/src/steps/`, `cli/`=`crates/mzlab-cli/src/`, `lib/`=`crates/mzlab-lib/src/`, `wasm/`=`crates/mzlab-feat-wasm/`.

---

## Part 1 — Architectural and Scientific Comparison

### 1A. 15-Stage Pipeline Comparison

| stage | mzFlow today (step names, defaults, file:line) | MZmine | commercial suite | gap or advantage | priority (value for a research lab vs effort) |
|---|---|---|---|---|---|
| 1. Raw reading and centroiding | `import.raw@1` (`core/step/core_steps.rs:11-28`, `cli/flow_import.rs:11-20`): `.raw`,`.wiff`,`.dam`,`.d`,`mzML` (`core/readers.rs:16-35`). `convert.mzml@1` (`30-74`). `spectra.centroid@1` (`cli/flow_centroid.rs:18-45`, `core/centroids/mod.rs:133`): auto `fwhm`, `3`σ, `MERGED`/`SATURATED`. | Vendor/mzML; IMS. | Vendor ingest & centroid. | `+` Rust readers (`test_lettori_parita.py`); peak flags. `-` Web converts at upload; no Bruker IMS (`parita.md:41-45`). | Med (high value; high effort) |
| 2. Mass detection | `index.mzidx@1` (`core_steps.rs:76-104`, `core/mzidx.rs:26`, `64 MiB`). `features.detect@2` (`st/detect.rs:57-133,237-272`): HR `20k`/`200k`, LR `3k`/`7k`/`snr=3`; `adap_localmin` `noise_factor=0`. `features.estimate@1` (`39-55`). | Centroid, Exact, LocalMax, Recursive, Factor. | Adaptive noise & m/z pick. | `+` `estimate@1` logs noise in `prov/1`; sliceable `.mzidx`. `-` `LocalMaxMassDetector` (`untargeted/localmax.rs:5-36`) unwired. | High (Item 8 — S) |
| 3. Chromatogram building / ROI | `features.detect@2`: `masstrace` (`st/detect.rs:278-293`, `feat/trace.rs:22-45`) log-ppm (`4e-6`, `min_pts=5`, `max_gap=1`, `long=400`, FT check, `96 MB` slab); `adap_localmin` (`328-385`, `untargeted/adap.rs:324-420`, `[M17]`). | ADAP ROI, GridMass, IMS. | Proprietary ROI & spike filter. | `+` Slabbed ADAP + MassTrace match CLI/OPFS. `-` `arpls` (`untargeted/arpls.rs:5`, `[B15]`) & `smooth_window` (`pipeline.rs:88`) hidden. | High (Item 7 — M) |
| 4. Peak detection and deconvolution | `features.detect@2`: `masstrace` (minima); `centwave` (`st/detect.rs:295-325`, `feat/centwave.rs:14-43`, `ppm=25`, `width=[5,30]s`, `snr=10`, `[T08]`); `adap_localmin` (`untargeted/resolve.rs:271-290`, `chrom_thr=0.85`, `ratio=1.7`). | ADAP LocalMin, Wavelets, SG, arPLS. | Joint component pick. | `+` 3 solvers + `broad` bands (`centwave.rs:31`). `-` `arpls` & SG unwired (`detect.rs:328-385`); `threshold` `Planned` (`387-405`). | High (Item 7 — M; Mac test) |
| 5. Isotope grouping and adduct/fragment annotation | `features.detect@2`: `isotopes="averagine"|"grouper"|"finder"` (`st/detect.rs:136-205`, `feat/isotopes.rs:184`). `features.group@1` (`st/group.rs:62-96`, `flowtab`): IIN (`rt_tol=0.1`, `min_corr=0.75`, `ppm=5`, `max_charge=2`, `[S21]`, `[K12]`). | 13C Finder, Isotope Grouper, IIN + fine isotopes. | Componentization merging adducts. | `+` Non-destructive `ionnet/1` with GNPS parity. `-` No compound-collapsed Quant CSV; `isofine.rs` separate from `group@1`. | Low-Med (solid parity) |
| 6. Alignment / RT correction | `features.align@1` (`st/table.rs:54-141`, `table_algo.rs:33-65`, `famalign.rs:291-300`, `slabs.rs:1-34`): `groups` (`family`, `ppm=5`, `rt_gap=0.1`) & `join` (`3`/`1`, `rt_tol=0.2`); `rt_warp="off"|"piecewise"|"loess"|"ransac"`. | Join, RANSAC, Loess/spline & IMS. | Non-linear RT warp + joint pick. | `+` Family align + slabbed RAM (`LEAN_ROW=39`, `WORK_ROW=40`). `-` `rt_warp` defaults `"off"` (`table.rs:117`); polarity merge separate. | Med (Mac benchmark) |
| 7. Gap filling | `features.gapfill@1` (`st/table.rs:144-196`, `table_algo.rs:71-101`, `gapfill.rs:199-253`, `untargeted/gapfill.rs:171-196`): `integrate` (`floor=20k`, `ppm=5`, `rt_tol=0.1`, `max_gap=2`) & `peak_finder` (`tol=0.2`), via `.mzidx`. | Multi-thread Peak Finder & Same-Range. | Re-integrates missing cells. | `+` Cell states (`detected`/`targeted`/`filled`/`absent`) in `.mzfeat` & `qc/1`. `-` Single-thread WASM; ignores upstream `RtMap`. | Low (strong parity) |
| 8. Blank subtraction and background filtering | `table.filter@2` (`st/table.rs:199-323`, `filters.rs:21-50`, `filters_blank.rs:329-355`): `blank_stat="mean"`, `scope="max_feature"`, `blank_match="all"` (`341` refuses non-`All`), `qc_min=3`, `dup_rt=0.1`, `BroadBlank`. Persistence (`trace.rs:28`). | Blank Subtract, Duplicate & Row Filters. | Background ratio & trace subtract. | `+` View-based filter + `qc/1`/`tabcheck/1`. `-` `filters_blank.rs:341` blocks `batch`/`matrix` (`table.rs:281-285`); no diluted-QC gate. | High (Item 9 — M) |
| 9. Normalisation and QC | `table.filter@2` `normalize="none"|"sum"|"median"|"istd"` (`st/table.rs:289-323`, `norm.rs:26-34,131-164`). `table.drift@1` (`326-358`, `drift.rs:18,223-322`): `linear`/`loess` (`MIN_QC=5`). `qc.run_order@1`, `qc.mass_accuracy@1`, `table.plausible@1` (`st/stats.rs:130-160`). | Drift (LOESS/RF), TIC/median norm, QC RSD. | QC-RLSC (`[D11]`), PQN/ISTD, QC plots. | `+` `qc/1`, `tabcheck/1`, `run_order`, `mass_accuracy`; `plausible` PR #3208. `-` No `normalize="pqn"` (`[D06]`); `drift.rs` lacks batch bridge. | High (Items 4, 6, 9) |
| 10. Statistics (PCA, volcano, clustering) | `table.stats@1` (`st/stats.rs:16-45`, `stat/compare.rs:172-335`, `stat/json.rs:140-202`): Welch t, ANOVA, BH FDR, log2FC, `heat` z-score, refuses `<3` files (`core/drawing/mod.rs:15`), bans `"significant"` (`feat/tests/stat.rs:328-330`). `table.pca@1`, `table.kmd@1`. | PCA, PLS-DA, H-clust, Volcano, KMD. | Volcano, PCA/OPLS-DA, H-clust, Box. | `+` `MIN_REPLICHE=3`, zero `"significant"`, LOO PCA, `TDATI`. `-` `table.stats@1` heatmap (`compare.rs:314-335`) lacks UPGMA; no 2-way ANOVA. | High (Item 3 W1; W3) |
| 11. MS2 handling and spectral-library matching | `features.ms2_link@1` (`st/detect.rs:408-454`), `spectra.ms2@1`, `spectra.chimeric@1` (`st/ident_spectra.rs:13-47`, `[St21]`), `spectra.match@1` (`lib/flowmatch/mod.rs:144-296`, `lib/flash.rs:1-35`: `spectral_entropy` `[L21]`, `cosine`, `modified_cosine` `[W12]`, `tree`), `table.match@1`. | Entropy/cosine, FBMN, chimeric. | Spectral DBs, DDA/DIA deconv. | `+` Flash `.mzlib` v1 runs offline in `.wasm` + FDR. `-` `modified_cosine` drops `delta_formula_mda`; `chimeric` `SOLO_APP`. | High (Item 1 W1; W2/W3) |
| 12. Formula assignment | `formula.ms1@1` (`st/ident_formula.rs:30-89`, `flow_formula_ms1.rs:1-40`): Golden Rules (`[K07]`), `require_c13=true`, `tol=5 ppm`, `rdb>=0`. `formula.ms2@1` (`103-152`): `subformula` & `h_rearrangement` (`[T16]`, `ppm_frag=10`). | Formula pred; SIRIUS/ZODIAC. | Exact mass + SFit + MS2 subformulas. | `+` Golden Rules + H-rearrangement in `attributio-all` & lazy `formula` `.wasm` (`wasm-tetti.md:36`). `-` Web `formula.ms1` buffers mzML `<=256 MiB`, not `.mzidx`. | Med-High (Item 10) |
| 13. Structure / compound identification and scoring | `spectra.insilico@1` (`st/ident_spectra.rs:357-406`, `[W10]`, `[R16]`), `screen.suspects@1` (`st/ident_screen.rs:12-44`), `rt.predict@1` (`st/ident_formula.rs:166-259`), `annot.regulatory@1`, `annot.hazard@1` (`st/ident_annot.rs:133-221`, T1-T5). | Lipid/Compound DBs, SIRIUS + CANOPUS. | DB lookup & fragment scoring. | `+` `.mzsus`, PubChemLite, T1-T5 (`tox/hazard.rs`). `-` `annot.hazard@1` lacks CLI/WASM exec (`flow_rule_tests.rs:16`); `rt.predict` `SOLO_APP`. | High (Item 5 S; W3) |
| 14. Confidence levels (Schymanski) | `annot.level@1` (`st/ident_annot.rs:9-114`, `flow_level/mod.rs:1-4,172-240`, `feat/data/livelli-1.json:377`): `[S14]`, `[H22]`, `sim>=0.70`, `peaks>=4`. No auto-2b; LR capped Level 4 (`"tetto":"4"`). `AuditEntry` (`attributio/annotate/types.rs:163-182`). | Partial tags; no gate audit. | Rank scores; no 2a/2b lock. | `+` **Best-in-class** Schymanski gates (`GateResult`), name-conflict downgrade (`3`), human-only 2b (`flow_level/mod.rs:3`), LR cap 4. `-` No Flow 2b drawer yet. | Low (core strength) |
| 15. Export / reporting and reproducibility | `export.table_csv@1` (`st/table.rs:361-388`), `export.mztab@1` (`export_mztab.rs:1-40`), `export.{annotations,inclusion}@1`, `report.methods@1` (`st/qcreport.rs:51-69`), `mzrun/1` (`cli/flow_mzrun.rs:31-79`), `meta.compare@1` (`cli/flow_compare.rs:292-318`). | `mzbatch`, FBMN MGF, mzTab-M; unsigned. | Archive, Excel/PDF; unsigned. | `+` Verified `mzrun/1` (`self_sha256`, `plan_sha256`, `results_digest`, `flow verify`) + `report.methods@1`. `-` `meta.compare@1` JSON only; no FBMN bundle. | High (Item 2 W1; W3) |

**References**:
- `[B15]` Baek et al., *Analyst* 2015 `doi:10.1039/C4AN01061B`; `[D06]` Dieterle et al., *Anal. Chem.* 2006 `doi:10.1021/ac051632c`; `[D11]` Dunn et al., *Nat. Protoc.* 6:1060 2011; `[E98]` Eisen et al., *PNAS* 1998 `doi:10.1073/pnas.95.25.14863`; `[H22]` Hoffmann et al., *Nat. Biotechnol.* 2022 `doi:10.1038/s41587-021-01045-9`; `[K07]` Kind & Fiehn, *BMC Bioinf.* 2007 `doi:10.1186/1471-2105-8-105`.
- `[K12]` Kuhl et al., *Anal. Chem.* 2012 `doi:10.1021/ac202450g`; `[L21]` Li et al., *Nat. Methods* 2021 `doi:10.1038/s41592-021-01331-z`; `[M17]` Myers et al., *Anal. Chem.* 2017 `doi:10.1021/acs.analchem.7b00947`; `[R16]` Ruttkies et al., *J. Cheminform.* 2016 `doi:10.1186/s13321-016-0115-9`; `[S14]` Schymanski et al., *Environ. Sci. Technol.* 2014 `doi:10.1021/es5002105`; `[S21]` Schmid et al., *Nat. Commun.* 2021 `doi:10.1038/s41467-021-23953-9`.
- `[S23]` Schmid et al., *Nat. Biotechnol.* 2023 `doi:10.1038/s41587-023-01690-2`; `[St21]` Stancliffe et al., *Nat. Methods* 2021 `doi:10.1038/s41592-021-01195-3`; `[T08]` Tautenhahn et al., *BMC Bioinf.* 2008 `doi:10.1186/1471-2105-9-504`; `[T16]` Tsugawa et al., *Anal. Chem.* 2016 `doi:10.1021/acs.analchem.6b00770`; `[W10]` Wolf et al., *BMC Bioinf.* 2010 `doi:10.1186/1471-2105-11-148`; `[W12]` Watrous et al., *PNAS* 2012 `doi:10.1073/pnas.1203689109`.

---

### 1B. Ergonomics, Speed & Memory at Scale, and Unique Differentiators

- **Guided presets & visual QC**: Templates (`flow/lazy/flow-shell.js:59-107`), `Simple`/`Advanced` forms (`flow-form.js:78-100`), and `hr`/`lr` presets (`flow-issues.js:30`) prevent raw-JSON fatigue.
  - Every stage emits QC (`qc/1` `feat/qc.rs:123-145`, 15-check `tabcheck/1` `feat/tabcheck.rs:1,25`, `plausible/1` `feat/plausible.rs:11-88`, `drift.rs:124`, `qc.run_order@1` `st/qcreport.rs:8-48`, `report.methods@1` `51-69`); plots pair with WCAG 2.2 AA `TDATI` keyboard tables (`docs/agenti/alta-risoluzione-statistica.md:30-35`).
- **Speed & memory at scale**:
  - `index.mzidx@1` builds m/z-sorted sliceable indexes within `DEFAULT_BUDGET = 64 MiB` (`core/mzidx.rs:26`).
  - `features.detect@2` runs in `budget = 96_000_000` B slabs (`SLAB_PEAKS = 1_500_000`, `BYTES_PER_PEAK = 64`, `SLAB_GAP_PPM = 50.0`, `feat/trace.rs:31-36`); `features.align@1` streams `LEAN_ROW = 39` / `WORK_ROW = 40` rows (`feat/slabs.rs:1-34`).
  - Flash `.mzlib` v1 inverted indexes use L2-flat `FlashAcc` (`lib/flash.rs:1-35`); browser WebWorkers stream OPFS byte slices lazily (`wasm/src/lib.rs:1-4`).
- **Unique differentiators over MZmine and the commercial reference suite**:
  1. **Zero-upload `.wasm` + OPFS parity**: Raw files stay local; deterministic `libm` and `SortC` ensure byte-for-byte CLI/WASM parity (`docs/agenti/parita.md:8-10`, `tests/test_parita_passi.py`).
  2. **SHA-256 `mzrun/1` verification**: `flow record`/`verify` (`cli/flow_mzrun.rs:31-79`, `docs/agenti/mzrun.md:7-25`) fingerprints `plan_sha256`, `results_digest`, `self_sha256`, 40-char `engine.commit`, `resolved.pin` (`fixed`/`estimate`), and `merge.params_sha256`.
  3. **Audit trail & strict guardrails**: Overrides record `AuditEntry` and `origin()` (`"auto"|"auto-2b"|"user"`, `attributio/annotate/types.rs:163-182`). `flow_level` forbids auto-2b (`flow_level/mod.rs:3`) and caps LR at Level 4 (`livelli-1.json:377`); `stat/compare.rs:187,229` refuses p-values below `MIN_REPLICHE = 3`; `feat/tests/stat.rs:328-330` forbids `"significant"`.

---

### 1C. Top-10 Ranked Gaps Summary Table

| # | Gap / Opportunity | Stage | Impact | Size | Target Track |
|---:|---|---|---|---|---|
| 1 | Analogue `delta_formula` missing in `modified_cosine` hits | 11 | High | S | Wave 1 (`claude/flow-delta-formula`, `lib` `.wasm`) |
| 2 | `meta.compare@1` cannot diff `.mzfeatf` / `.mzfeat` tables | 15 | High | M | Wave 1 (`claude/flow-compare-tables`, CLI) |
| 3 | `table.stats@1` heatmap lacks hierarchical UPGMA ordering | 10 | High | S | Wave 1 (`claude/flow-stats-cluster-order`, `flowstat`) |
| 4 | Browser Flow execution parity for `table.plausible@1` | 9 | High | S | In progress (`PR #3208`, `table` `.wasm`) |
| 5 | `annot.hazard@1` lacks CLI & browser Flow executors | 13 | High | S | Wave 2 (`attributio-all` + CLI) |
| 6 | Missing Probabilistic Quotient Normalisation (`normalize="pqn"`) | 9 | High | M | Wave 2 (`flowtab` `.wasm`) |
| 7 | `arpls` baseline & smoothing hidden from `adap_localmin` | 3–4 | High | M | Wave 2 (`untargeted` `.wasm`) |
| 8 | `LocalMaxMassDetector` (`localmax.rs`) unwired in `detect@2` | 2 | Med-High | S | Wave 2 (`untargeted` `.wasm`) |
| 9 | Batch/matrix blank matching blocked; QC-RLSC bridge missing | 8–9 | High | M | Wave 2 (`flowtab` `.wasm`) |
| 10 | Browser `formula.ms1@1` hits `256 MiB` whole-mzML RAM cap | 12 | Med-High | M/L | Wave 3 (stream `.mzidx` in lazy `formula` `.wasm`) |

---

## Part 2 — Ranked Implementation Decisions

Rules (`CLAUDE.md`, `docs/agenti/wasm-tetti.md`): **never raise caps**, keep **`hr` & `table` at `+0 B`**, cap growth at `<= +10%`.

| `.wasm` module (`tools/build_feat_wasm.sh`) | Measured (B) | Cap (B) | Headroom (B) | Policy |
|---|---:|---:|---:|---|
| `hr` (`mzlab-feat-wasm` default) | 1,715,152 | 1,716,224 | 1,072 | **+0 B frozen** |
| `table` (`mzlab-feat-table-wasm`) | 734,138 | 737,280 | 3,142 | **+0 B frozen** |
| `lib` (`mzlab-lib-wasm`) | 493,972 | 614,400 | 120,428 | Item 1 (`~+2 KB`) |
| `flowtab` (`--features flow_tab`) | 384,768 | 512,000 | 127,232 | Items 6 & 9 (`<+10%`) |
| `flowstat` (`--features flow_stat`) | 329,884 | 512,000 | 182,116 | Item 3 (`~+1.5 KB`) |
| `untargeted` (`--features untargeted`) | 465,717 | 585,728 | 120,011 | Items 7 & 8 (`<+5 KB`) |
| `attributio-all` (`--features all`) | 1,483,595 | 1,536,000 | 52,405 | Item 5 (`~+1–2 KB`) |
| `formula` (`--features flow_formula`) | 403,567 | 512,000 | 108,433 | Item 10 (`~+5–8 KB`) |

### Item 1 — Analogue `delta_formula` (`Wave 1`, `claude/flow-delta-formula`, **S**)
- **What**: Wire `delta_formula_mda` (`st/ident_spectra.rs:340`, `5.0 mDa`) into `mzlab_lib::flowmatch` (`spectra.match@1`/`table.match@1`).
- **Why**: Shift labels fail today because `analogues()` (`lib/flowmatch/mod.rs:266-296`) omits `delta_formula`.
- **Design**:
  1. Add bounded CHNOPS solver in `mzlab-lib` (zero graph deps).
  2. Add `delta_formula: Option<String>` to `Cand` (`lib/flowmatch/mod.rs:144-155`).
  3. Fill in `analogues()` if `delta_formula_mda > 0` (min mDa err, min heteroatoms, valid RDBE, `SortC`).
  4. Emit `"delta_formula"` in `write_cand()` (`346-401`) if set (`0.0` disables); preserve CLI/WASM parity.
- **Acceptance**: `cargo test -p mzlab-lib flowmatch` -> `+15.9949 Da` gives `"+O"`, `0.0` omits, WASM matches CLI.
- **Wasm**: `lib` `.wasm` `~+2 KB` (`<+0.5%` of `120,428` B headroom); `hr`/`table` `+0 B`.

### Item 2 — Feature/Table Diff in `meta.compare@1` (`Wave 1`, `claude/flow-compare-tables`, **M**)
- **What**: Extend `meta.compare@1` (`cli/flow_compare.rs:292-318,413-448`) to diff `.mzfeatf`/`.mzfeat` via `mz_ppm` & `rt_tol`.
- **Why**: Solver/aligner sweeps (`masstrace`/`centwave`/`adap_localmin`; `groups`/`join`) lack native diff metrics.
- **Design**:
  1. Dispatch header magic in `flow_compare.rs`: JSON vs `.mzfeatf` (`read_file_from`) vs `.mzfeat` (`read_table`).
  2. Greedy 1-to-1 nearest match by `(dmz_ppm/tol_ppm)^2 + (drt/rt_tol)^2` within bounds.
  3. Emit `count_a`, `count_b`, `both`, `only_a`, `only_b`, `mean_overlap`, `mean_rank_corr`.
  4. Add `mz_ppm`/`rt_tol` to `compare_step()` (`15-54`); `SOLO_APP` unchanged.
- **Acceptance**: `cargo test -p mzlab-cli flow_compare` -> `.mzfeatf`/`.mzfeat` overlap & rank corr + goldens pass.
- **Wasm**: `+0 B` everywhere (`mzlab-cli` native).

### Item 3 — UPGMA Heatmap Ordering (`Wave 1`, `claude/flow-stats-cluster-order`, **S**)
- **What**: Add UPGMA (`[E98]`) over `table.stats@1` heatmap (`feat/stat/compare.rs:314-335`) as `rowOrder`/`colOrder` (`stat/json.rs:188-200`).
- **Why**: Unclustered heatmaps hide co-regulated blocks vs MZmine and the commercial suite.
- **Design**:
  1. Compute Euclidean distances over `z` rows & `heat.groups` columns (`libm::sqrt`).
  2. Run UPGMA with lowest-index tie-breaking (`SortC`, zero `HashMap`).
  3. Emit `"rowOrder"`/`"colOrder"` in `write_stats()`, keeping `heat.z` order intact.
  4. Verify CLI/`flowstat` `.wasm` (`wasm/src/run_stat.rs`) parity.
- **Acceptance**: `cargo test -p mzlab-feat --test stat` -> planted clusters group contiguously; WASM matches CLI.
- **Wasm**: `flowstat` `.wasm` `~+1.5 KB` (`<+0.5%` of `182,116` B headroom); `hr`/`table` `+0 B`.

### Item 4 — Browser `table.plausible@1` (`In progress, PR #3208`, **S**)
- **What**: Wire `F.liveJobs["table.plausible"]` to `table_plausible` in `table` `.wasm` (`crates/mzlab-feat-table-wasm/src/core.rs:211`).
- **Why**: Clears `"table.plausible"` from `SOLO_APP` (`tests/test_parita_passi.py:18`) without desktop fallback.
- **Design**:
  1. Read OPFS `.mzfeat` slices + thresholds (`st/stats.rs:130-160`).
  2. Call `table_plausible(read, size, options_json)` with zero Rust edits.
  3. Cache `plausible/1` in OPFS and verify CLI parity.
- **Acceptance**: `pytest -q tests/test_parita_passi.py` -> `"table.plausible"` cleared; browser matches `flow run`.
- **Wasm**: `+0 B` (`table` stays `734,138 / 737,280` B).

### Item 5 — CLI & Browser `annot.hazard@1` (`Proposed Wave 2`, **S**)
- **What**: Add native CLI & `mzlab-attributio-wasm` executors for `annot.hazard@1` (`st/ident_annot.rs:198-221`).
- **Why**: Clears `"annot.hazard"` from `HOLES` (`cli/flow_rule_tests.rs:16`) & `SOLO_APP` (`test_parita_passi.py:16`).
- **Design**:
  1. Wrap `attributio::tox::{hazard, snapshot}` in `flow_hazard::hazard_text` (`Io`/`MemFiles`).
  2. Enforce `SortC` and keep Level-3 intrinsic-hazard caveats.
  3. Register in `cli/flow.rs`, `mzlab-attributio-wasm/src/lib.rs` (`all`), and `flow-web-ident.js`.
- **Acceptance**: `cargo test -p mzlab-cli flow_rule_tests && pytest -q tests/test_parita_passi.py` -> cleared; CLI/WASM match.
- **Wasm**: `attributio-all` `~+1–2 KB` (`<+0.15%` of `52,405` B headroom); `hr`/`table` `+0 B`.

### Item 6 — PQN Normalisation (`normalize="pqn"`) (`Proposed Wave 2`, **M**)
- **What**: Add `"pqn"` (`[D06]`) to `norm::Method` (`feat/norm.rs:26-34`) and `table.filter@2` (`st/table.rs:289-323`).
- **Why**: Dilution variance and outlier spikes skew sum/median scaling.
- **Design**:
  1. Compute per-row median (`norm::decides`), then per-file `1 / median(area / ref)`.
  2. Guard zero-overlap rows and cross-check vs NumPy in `tools/`.
  3. Run browser PQN via `flowtab` `.wasm` (`wasm/src/run_tab.rs`), keeping `table` `+0 B`.
- **Acceptance**: `cargo test -p mzlab-feat norm` -> 2x-diluted + spike recovers `2.0x` (`1e-12`) in CLI/`flowtab`.
- **Wasm**: `flowtab` `~+2.5 KB` (`<+0.7%` of `127,232` B headroom); `hr`/`table` `+0 B`.

### Item 7 — Expose `arpls` & Smoothing (`Proposed Wave 2`, **M**)
- **What**: Expose `baseline_arpls`, `baseline_lambda`, `baseline_exclude_peaks`, `smooth_method`, `smooth_window` in `adap_localmin()` (`st/detect.rs:328-385`).
- **Why**: `arpls.rs` (`[B15]`) & `smooth.rs` link in `untargeted` (`pipeline.rs:38-89`) without Flow plumbing.
- **Design**:
  1. Add `.advanced()` params defaulting to `baseline_arpls=false`, `smooth_method="none"`, `smooth_window=0`.
  2. Wire params in `steps::recipe_map` to `UntParams { baseline, smooth_window, .. }`.
  3. Emit active flags in `prov/1` & `report.methods@1`.
- **Acceptance**: `cargo test -p mzlab-cli flow_rule_tests && cargo test -p mzlab-feat untargeted` -> flags reach `untargeted`; defaults intact.
- **Wasm**: `untargeted` `~+1–2 KB` (`<+0.4%` of `120,011` B headroom); `hr`/`table` `+0 B`.

### Item 8 — Wire `LocalMaxMassDetector` (`Proposed Wave 2`, **S**)
- **What**: Wire `untargeted::localmax::detect` (`feat/untargeted/localmax.rs:5-36`) into `pipeline_stream.rs` + add `mass_detector`.
- **Why**: Activates verified dead code for MZmine local-maximum mass detection parity.
- **Design**:
  1. Dispatch scan detection in `pipeline_stream.rs` for `mass_detector="localmax"` (`floor` stays default).
  2. Add `mass_detector` + `localmax_min_points=3` `.advanced()` params in `st/detect.rs:328-385`.
  3. Verify CLI/`untargeted` `.wasm` parity.
- **Acceptance**: `cargo test -p mzlab-feat localmax && cargo test -p mzlab-cli flow_rule_tests` -> identical CLI & WASM output.
- **Wasm impact**: `untargeted` `~+2–3 KB` (`<+0.6%` in `120,011` B headroom); `hr`/`table` `+0 B`.

### Item 9 — Batch/Matrix Blanks, `QCs` Dilution, & QC-RLSC (`Proposed Wave 2`, **M**)
- **What**: Enable `blank_match="batch"|"matrix"` (`filters_blank.rs:341`), `QcDilution`, and batch QC-RLSC (`drift.rs:223-322`, `[D11]`).
- **Why**: Multi-batch/matrix runs fail where `subtraction()` blocks non-`All` and `drift.rs` lacks batch bridging.
- **Design**:
  1. Implement `Matching::Batch`/`Matrix` in `filters_blank.rs` matching `st/table.rs:281-285`.
  2. Add `QcDilution(min_r2, max_cv)` in `filters.rs` for `QCs` linearity.
  3. Add `bridge="batch_median"` & `relax_ends=true` in `drift.rs` via `flowtab`.
- **Acceptance**: `cargo test -p mzlab-feat drift filters_blank` -> 2x shift aligns QC medians (`CV<15%`) in CLI/`flowtab`.
- **Wasm**: `flowtab` `~+4–6 KB` (`<+1.6%` of `127,232` B headroom); `hr`/`table` `+0 B`.

### Item 10 — Stream `.mzidx` in Browser `formula.ms1@1` (`Proposed Wave 3`, **M/L**)
- **What**: Replace whole-mzML RAM reads (`MAX_MZML=256 MiB`, `flow/lazy/flow-web-ident.js:13,143-169`) with `.mzidx` `ReadAt` in lazy `formula` `.wasm` (`build_feat_wasm.sh:36-38`, `wasm-tetti.md:36`).
- **Why**: `wasm/Cargo.toml:17` bars `flow_formula` from `hr` (`1,072` B left) & `untargeted` (`120,011` B vs `~134 KB`), so `formula` stays lazy (`403,567/512,000` B); `.mzidx` lifts the `256 MiB` cap.
- **Design**:
  1. Add `ReadAt` nearest-MS1 lookup in `feat/flow_formula_ms1.rs` via OPFS `.mzidx`.
  2. Export `flow_ms1_rows_idx` in `wasm/src/run_formula.rs`, keeping `flow_ms1_rows` fallback.
  3. Stream OPFS `.mzidx` handles in `flow/lazy/flow-web-ident.js` (`hr`/`untargeted` `+0 B`).
- **Acceptance**: `python3 tools/ci_wasm_dimensioni.py && pytest -q tests_e2e/e2e_flow_web_ident2.py` -> `formula` `<415 KB`, `hr`/`untargeted` `+0 B`.
- **Wasm**: Lazy `formula` `~+5–8 KB` (`<+2.0%` of `108,433` B headroom); `hr`, `untargeted`, `table` `+0 B`.

---

### Real-File Verification Plan (Mac Helper — Aggregate Counts Only, Never Committed)

> [!CAUTION]
> Never commit vendor files, sample names, feature exports, or mzML paths. Log aggregate counts, correlations, wall time, and peak RSS only.

1. **Detection & Alignment Matrix (after Item 2)**: Benchmark `features.detect@2` (`masstrace` vs `centwave` vs `adap_localmin`) and `features.align@1` (`groups` vs `join`) via `meta.compare@1` on a multi-injection pooled-QC + blank LC-HRMS run -> record `count_a`, `count_b`, `both`, `mean_overlap`, `mean_rank_corr`, peak RSS (`MiB`), wall time (`s`).
2. **Analogue `delta_formula` Check (after Item 1)**: Run `spectra.match@1` (`modified_cosine`, `delta_formula_mda=5.0`) on experimental DDA MS2 vs a public `.mzlib` -> record `total_hits`, `analogue_hits`, resolved `delta_formula` count.
