# Area files: which one to read
One row per area file, in file-name order (max 300 characters): if you touch the area on the left, read the file on the right (only that one). A new file = a new row in its place. To find a row: `grep -n '<word>' docs/agenti/INDICE.md`. Every `docs/agenti/*.md` has its own row; order and one row per file are checked by `tools/gate_release.py`.

| If you touch… | Read |
|---|---|
| accessibility of the whole application (WCAG 2.1/2.2 AA, axe-core, controls, known gaps and tolerated rules) | `docs/agenti/accessibility.md` |
| panel actions (`actions/1` registry, right-click menu, Shift+F10) | `docs/agenti/actions.md` |
| headless 2D depiction for agents (`mol draw` study: own Rust layout vs RDKit MinimalLib, Indigo, Rust crates; sizes, licences, quality on a synthetic set with RDKit as oracle, gates) | `docs/agenti/agent-draw.md` |
| Agilent `.D` LC/MS: QqQ MRM/dMRM, Q-TOF profile/centroids/MS2, LC/MS method, DAD (`crates/mzlab-core/src/agilent_ext/`, feature `estesi`) | `docs/agenti/agilent-lcms.md` |
| Agilent `.D` folder reader (`crates/mzlab-core/src/agilent/`) | `docs/agenti/agilent.md` |
| loading animation (`base`) and the switched-off animated analyzers (`an-*.js`: quadrupole, TOF, Orbitrap, ESI, trap, FT-ICR, QqQ, Paul, sector, IMS, MALDI) | `docs/agenti/analyzers.md` |
| anonymous public proxy of the assistant (`proxy/`, Cloudflare Worker, spending `Ledger`, ports, hash list) | `docs/agenti/assistant-proxy.md` |
| Banco 2b (area: Attributio in Rust (level cascade, Banco 2b, MZ binary of…) | `docs/agenti/attributio-banco.md` |
| exports (area: Attributio in Rust (level cascade, Banco 2b, MZ binary of…) | `docs/agenti/attributio-export.md` |
| isotopes and elemental composition (area: Attributio in Rust (level cascade, Banco 2b, MZ binary of…) | `docs/agenti/attributio-isotopi.md` |
| experimental MS2 engine, MetFrag-style in silico fragmentation, MS-FINDER-style fragment attribution, RT prediction (`attributio/ms2_engine/`) | `docs/agenti/attributio-ms2.md` |
| parity with mzmine (area: Attributio in Rust (level cascade, Banco 2b, MZ binary of…) | `docs/agenti/attributio-mz-parita.md` |
| MZ binary (area: Attributio in Rust (level cascade, Banco 2b, MZ binary of…) | `docs/agenti/attributio-mz.md` |
| panel (area: Attributio in Rust (level cascade, Banco 2b, MZ binary of…) | `docs/agenti/attributio-pannello.md` |
| review, matching and identity (area: Attributio in Rust (level cascade, Banco 2b, MZ binary of…) | `docs/agenti/attributio-revisione.md` |
| Attributio in Rust (level cascade, Banco 2b, mzmine-style detection MZ binary: `crates/mzlab-feat/src/attributio/`, `untargeted/`, `.wasm` `attributio*` and `untargeted`) | `docs/agenti/attributio.md` |
| smeared peaks: synthetic batch, MZmine default/wide configurations, comparison table (`tools/bench/confronto_larghi.py`) | `docs/agenti/bench-wide-peaks.md` |
| Bruker timsTOF `.d` reader (`crates/mzlab-lettori/src/bruker/`: minimal SQLite, zstd, `tdf_bin`/`tsf_bin`, m/z calibration, `BrukerD` in `agilent-d.js`) | `docs/agenti/bruker.md` |
| calibration curve on integrated peaks (`quant.curve`: 1/x..1/y² weights, QR, internal standard, ICH LOD/LOQ and S/N, logged exclusions, `calibration/1`, `calib.rs`, `curve.rs`) | `docs/agenti/calibration.md` |
| browser capabilities, feature detection, "not available in this browser: use the app" notice (`capacita.js`, `?cap-off=`) | `docs/agenti/capabilities.md` |
| conditional GitHub workflows (concurrency, pages, skipped jobs) | `docs/agenti/ci-fast.md` |
| CI pytest shards (`target` cache, `doppi`, `aree`, `prebuilt`, order and halves of slow tests) | `docs/agenti/ci-selection.md` |
| chemical classes from rules (`crates/mzlab-feat/src/classes/`, table `data/classi-1.json`, `classi/1`: diagnostic fragments and neutral losses) | `docs/agenti/classes.md` |
| mzlab-cli for AI agents: `--json` commands, exit codes, stdout/stderr, `--dry-run`, `flow steps --json|--template`, `flow check`, `--json` list, prebuilt binaries, `--cache`, citation rule, `--version` commit, renamed commands, panics (`err.run.internal`) | `docs/agenti/cli-agenti.md` |
| `mzlab-cli file convert`: vendor files and folders to mzML, `--peaks`, `--spectrum` (centroid/profile term, sparse data), `--report`, exit codes | `docs/agenti/cli-convert.md` |
| step by step (area: command line `mzlab-cli features` (`crates/mzlab-cli`: feature…) | `docs/agenti/cli-passi.md` |
| command line `mzlab-cli features` (`crates/mzlab-cli`: feature table as CSV, same engine and recipe as the page; `features-notes/1` at `--notes` or beside `--out`) | `docs/agenti/cli.md` |
| running the verification in the cloud container (Chromium, `--setup`, tests against the built site) | `docs/agenti/cloud-environment.md` |
| Safari and iPadOS (WebKit): API × minimum version × fallback, 16.4 floor, `compat.js` fallbacks (`AbortSignal.any`, `createWritable`, clipboard, `accept`); Safari, iPad, iPhone memory (budget without `deviceMemory`, calm refusal, `estimate::check_reader`) | `docs/agenti/compat-safari.md` |
| RT-λ (DAD) map: study of the panel — data flow, causes of subtraction/side strips/3D not working, formats (Agilent/Waters/SCIEX) and DAD→MS discard, design of the link with the MS, split into PRs | `docs/agenti/dad-mappa-studio.md` |
| DAD (UV/Vis) detector of the `.wiff`: matrix, RT-λ map, UV spectrum, TWC/XWC (`xwc.js`), XWC integration, DAD on the MRM panel (`dad-mrm.js`) | `docs/agenti/dad.md` |
| calculator ("what it could be" groups under the number), Adducts panel, facts on 3200 data, gradient, analog traces, data tables, theme | `docs/agenti/data-3200.md` |
| sidebar of the Data view, tools, hints, tour | `docs/agenti/data-bar.md` |
| data channels, `file info`, `trace`, `spectrum` (file → channels: TIC, filters/transitions/precursors, UV, analog; experiment type as a label; open work) | `docs/agenti/data-channels.md` |
| load a folder, pointed and remembered folder (greyed list, "Others", desktop), background warm-up of the pointed folder's files (queue, `lr_warm`, `lr_warm_claim`) | `docs/agenti/data-folder.md` |
| peak integration in the Data view (automatic tool, one click = all peaks in HR, parameters, `auto_integrate`) | `docs/agenti/data-integration.md` |
| first load of several files (only the first opens), fields of the load table | `docs/agenti/data-loading.md` |
| RT×m/z map, crosshair, navigation | `docs/agenti/data-map.md` |
| lists, correction, workflow, QA/QC, statistics, report | `docs/agenti/data-tools.md` |
| low-resolution Data view: loading, tabs, TIC/XIC/spectrum (default XIC window `xicWin`), integration | `docs/agenti/data.md` |
| experimental design from the CLI (`sample design propose|validate|canonical`, `design/1`, `design_proposal/1`, `design_check/1`, `design_canonical/1`, problems by key, `design_*` ops of the samples route, port `schema` in `flow steps`) | `docs/agenti/design-cli.md` |
| MS2 pseudo-spectra from DIA (`dia.rs`, `spectra.ms2` with `dia_pseudo`) | `docs/agenti/dia.md` |
| file read errors: short cause, "Details", "Copy diagnosis" (`diagnosi.js`), synthetic msconvert-style mzML | `docs/agenti/diagnosis.md` |
| Drawing right-click menu (`#ctx`, `disegno-menu.js`, `menuAdd` registry, touch copy/cut/paste `disegno-appunti.js`, "Information…", outside searches, `selection/1`, `link/1`; CDXML/CDX files: routes, `open/1`, RDKit fixtures) | `docs/agenti/drawing-menu.md` |
| Disegno: unsaved changes (`disegno-pulito.js`, `confirmDiscard`, `beforeunload`, the «not saved» dialog, Tauri window close) | `docs/agenti/drawing-unsaved.md` |
| Drawing (Ketcher) | `docs/agenti/drawing.md` |
| the Italian still left in the repository (counts per area, families to rename in order, what is kept on purpose; `tools/check_english.py`) | `docs/agenti/english-remaining.md` |
| export of all user data: browser storage inventory, ZIP writer (`zip.rs`); chunked exports in OPFS (`dl()`, `export-pezzi.js`, `export-worker.js`, provenance, `export/` cleanup) | `docs/agenti/export-data.md` |
| contract of the vendor readers (`readers.rs`, `Lettore` trait, registry, `Limiti`), adding a format | `docs/agenti/extending.md` |
| "XIC" button in the spectrum bar, XIC lens on peak hover (`xic-lente.js`), bench and baseline table of viewer times (`tools/bench_viewer.py`, `viewer_times.rs`) | `docs/agenti/fast-viewer.md` |
| `file pack` / `file unpack` / `file check-pack`: split archives (`pack/1` manifest, parts, SHA-256, exit codes, ZIP64, `.wasm` `pack` over OPFS) | `docs/agenti/file-pack.md` |
| graphical Flow (tab: step palette `step/1`, chain, parameter modules, `meta.compare` comparison, `mzflow/2` export/import; `flow/`) | `docs/agenti/flow-graphics.md` |
| state scheduler of the "live" Flow (`mzlab_core::flow::sched`, `run_sched.rs`) | `docs/agenti/flow-live.md` |
| sample metadata grid (area: graphical Flow (tab: step palette `step/1`, chain, modules…) | `docs/agenti/flow-meta-grid.md` |
| mapping to mzmine steps (area: graphical Flow (tab: step palette `step/1`, chain, modules…) | `docs/agenti/flow-mzmine.md` |
| Flow: names, groups and view cards of the MSn infusion model | `docs/agenti/flow-ux.md` |
| web runner of the Flow ("This browser": `flow/lazy/flow-web-{run,ui,worker}.js`, `flow:web-step` events, `estimate_series` estimate, phone/tablet warning, Run off above the high level) | `docs/agenti/flow-web.md` |
| external agents using `mzlab-cli`: task → command table, `commands --find` (`catalog/find.rs`, `related.rs`), `cli` profile, tested chains; in `docs/studies/`: `AGENT_USE_STUDY.md` (LR), `RAW_ACCESS_STUDY.md` (`raw`, AU-12..19), `TP_SCREENING_RULES.md` (TP checks) | `docs/agenti/for-agents.md` |
| "Group by" of the file list (`#sb-by`, `groups.js`, `groups-for.js`, key, "Then by", "Contains", follows new files, undo) | `docs/agenti/groups-by.md` |
| experiment groups in Tabui: conditions, `groups/1`, `crates/mzlab-feat/src/groups/`; "Compare conditions" view (`TP_Mine/lazy/groups-view.js`: TP × condition matrix, UpSet, charts, exports) and graft into Tabui LR | `docs/agenti/groups.md` |
| header (full screen, logo, (i)), loading area and table, progressive startup (`AVVIO`), "interface at once" rule; page audit (CLS, shifting panels, memory, errors; `tests_e2e/audit_pagine.py`, `e2e_giro_intestazione`) | `docs/agenti/header.md` |
| statistics in HR: feature table, blank, difference from the reference, PCA; `table compare` (each feature against reference and blank, candidate products, `compare/1`) | `docs/agenti/high-resolution-statistics.md` |
| HR world: Banco, formulas, isotopes, libraries, MSn tree | `docs/agenti/high-resolution.md` |
| registry of state-changing actions (`commands.js`, generated `docs/actions.md`), undo/redo, "removed" entries (remove a file, close a panel, clear marks: no warning, `HISTORY` entries) | `docs/agenti/history-removed.md` |
| "XIC" button and XIC lens off on infusion files, `infusion.rs` data rule (keys, blocks, name), `infusion_diagnosis` command, drawing traces from the window (`decima.js`); "Fragmentation graph" tab in the Data view | `docs/agenti/infusion-graph.md` |
| in silico candidate package (`mzlab-cli db insilico pack`, `insilico-scarica.js`, `flow-web-insilico.js`, `crates/mzlab-desktop/src/insilico.rs`, `spectra.insilico` in the browser) | `docs/agenti/insilico.md` |
| per-row TP interpretation (`crates/mzlab-feat/src/interp/`, `interp/1`, `filtri/1`, `interp` block in the `.mzfeat`, `.wasm` `interp`, `table_interp` routes) | `docs/agenti/interp.md` |
| grouping the ions of a compound (`features.group`, `ionnet/1`: correlation, isotopes, ion identity networking, GNPS edges; `crates/mzlab-feat/src/ionnet/`) | `docs/agenti/ion-groups.md` |
| texts, `lang/`, `help.js`, English only; marked-up texts (italic m/z, superscript MS2) written once, before first paint | `docs/agenti/languages.md` |
| identification levels (Schymanski scale 2a/2b/3a/3b, prediction scale, "L3" and "Prev. 2" labels, Tabui, Attributio, untargeted) | `docs/agenti/levels.md` |
| `.mzlib` format, library parsers, indexes and scores (`crates/mzlab-lib`) | `docs/agenti/libraries.md` |
| user lists (transformations, adducts, suspects: CSV with per-row errors, `liste/1`, SHA-256 for `prov/1`) | `docs/agenti/lists.md` |
| LLM assistant and external agents: principles, choices, risks, unrealized proposals | `docs/agenti/llm-choices.md` |
| LLM assistant evaluation: case sets (G/U/R/smoke), metrics, thresholds, `mzlab-cli llm eval`, red-team attack classes (`tests/data/assist/`) | `docs/agenti/llm-eval.md` |
| LLM assistant parameter proposals: `param-proposal/1`, signals, rule ids `R-*`, acquisition-to-plan table (`params_rules*.rs`) | `docs/agenti/llm-params.md` |
| subagents, reads and reader runner on the `llm-agente.js` page (area: LLM assistant: `tools/1` catalog) | `docs/agenti/llm-subagents.md` |
| LLM assistant: `tools/1` catalog | `docs/agenti/llm.md` |
| 3D map viewer (`mappa3d.js`, `failure3d`, viewer commands and menu, `preload3d` preload) | `docs/agenti/map-3d.md` |
| map of the open path of every file (click → screen, site and app, by format), strategies present/absent, file batches for external assistants (`tools/batch_gemini.py`) | `docs/agenti/map-loading.md` |
| Flow input formats (area: engine measurements, export validation (mzTab-M), clicks…) | `docs/agenti/measurements-formats.md` |
| page load (FCP, LCP, engine ready, KB before paint by scenario; `tests_e2e/misura_primo_paint.py`); first file vs ten files, `rotte`, `thermo`, memory (`misura_caricamento.py --scenarios`) | `docs/agenti/measurements-loading.md` |
| measurement on slow computers (CPU throttled 1×/4×/6×, 2/4 cores: demo open, line drag, maps, XIC, memory; `tests_e2e/misura_macchine.py`) | `docs/agenti/measurements-machines.md` |
| warm page loads: navigation preload and the `?nav=cache` switch of the service worker measured against main (RTT, CPU, restarted SW, loads until a deploy shows; `tests_e2e/misura_avvio_sw.py --variants`) | `docs/agenti/measurements-navigation.md` |
| loading an mzML in one pass (OPFS copy and index from the same bytes): limit, switched-off spike `una-passata.js`, design (`tests_e2e/misura_una_passata.py`, `build_mzml_streamed`) | `docs/agenti/measurements-one-pass.md` |
| Rust and module measurements (area: engine measurements, export validation (mzTab-M), clicks…) | `docs/agenti/measurements-rust.md` |
| vendor readers, `.raw`, `.wiff`, `.D` (area: cumulative speed-gain table (date · PR · step ·…) | `docs/agenti/measurements-speed-raw.md` |
| cumulative table of site load speed gains (page, first paint, file batches, side list, "Remove all", first file of a batch) | `docs/agenti/measurements-speed-site.md` |
| cumulative speed-gain table (date · PR · step · before · after · %); targeted extraction speed study (`docs/studies/SPEED_STUDY.md`, `tools/bench/speed_study.py`) | `docs/agenti/measurements-speed.md` |
| engine measurements, export validation (mzTab-M), Tabui flow and network clicks | `docs/agenti/measurements.md` |
| memory: budget choice rules, notice user experience, open proposals (`memoria-avviso.js`) | `docs/agenti/memory-choices.md` |
| "file too large" in the browser: when it appears (files opened together, formats read whole), measurements by size and file count, `openFiles`, `whole`, scans shared between slots | `docs/agenti/memory-limits.md` |
| memory contract of the WASM modules (`mzlab-core::budget`, offset outputs, slabs, test flow) | `docs/agenti/memory.md` |
| acquisition method (`method/1`, `method.rs`, `method_wiff`, `method_raw`, schema) | `docs/agenti/method.md` |
| main metrics over time (`tools/metriche.py`, `stat-ci` branch, trends) | `docs/agenti/metrics.md` |
| own 2D layout (`mzlab-mol::layout`), `mol draw`: molfile, `draw/1` scene, KET/CDXML, SVG/PNG pictures (`chem/draw*.rs`), edit patch `--set edit=` (`chem/edit*.rs`, `draw_edit/1`), RDKit quality gates, `draw_d1/d3.json` parity | `docs/agenti/mol-draw.md` |
| chemical structure record `mol/1` (`crates/mzlab-mol`: types, JSON, core hash; SMILES/molfile readers; CIP R/S and E/Z descriptors, `cip/1`, `disegno-cip.js`) | `docs/agenti/mol.md` |
| molecule database (`crates/mzlab-core/src/molecules/`, `molecules/1`: record, CSV and canonical JSON, search, catalog/imported/user levels, catalog `mzlab/web/molecules/catalogue.csv`, `api/molecules/*` routes) | `docs/agenti/molecules.md` |
| infusion MSn fragmentation graph (`msn-graph/1`: ions merged across levels and energies, proposed fragment formulas, "Fragmentation graph" tab) and standards' MSn tree library (`msn-lib/1`, acquisition protocol) | `docs/agenti/msn-graph.md` |
| MSn fragment interpretation (losses, rules, cleavages), fragment formulas (`msngraph/formulas.rs`, `candidates.rs`, `annotate.rs`: DAG constraint), cuts for a node (`strutture`), `vista` key (best path, layouts, `completa`) | `docs/agenti/msn-interpret.md` |
| list of MSn experiments of a DDA file, "MSn tree" window (Experiments/Tree tabs, `msn-elenco.js`, `lev`/`z` columns of `/api/dda`); MS3-only files (`msn_level`, level, `/api/ms2tr`, titles); MSn graph figure; rule tests (`msn rules`) | `docs/agenti/msn-manual.md` |
| library (area: infusion MSn fragmentation graph (`msn-graph/1`: ions…) | `docs/agenti/msnlib.md` |
| mutation tests (`cargo-mutants`) of `mzlab-lettori` and the wiff module: commands, samples, survivors and why they stay | `docs/agenti/mutants.md` |
| mzDraw competitor study: editors, depictors, toolkits and Rust crates (matrix, sizes, licences, layout benchmarks, licence verdict, ideas to copy) | `docs/agenti/mzdraw-competitors.md` |
| mzDraw formats: conformity matrix (read/write, what is lost, spec terms, public golden files), images and clipboard carrying the molecule, settings file `mzdraw-settings/1` and journal presets, document format `mzdraw/1`, annotations, import report | `docs/agenti/mzdraw-formats.md` |
| mzDraw plan: feature priorities P0-P3, risks, owner decisions, ready implementation prompts (format catalog, settings, layout benchmark, images with molecule, RXN/RDF, CML/MRV) | `docs/agenti/mzdraw-plan.md` |
| mzDraw, our own molecule editor in Rust (`crates/mzdraw`, `mzdraw-wasm`, page `mzlab/web/draw2/`): scope, architecture, `mzdraw/1` document, format conformity matrix, settings file, command log, build order | `docs/agenti/mzdraw.md` |
| broad bands (`broad.rs`), targeted pass options (`target.rs`: estimated peaks, /\ shape check, merging overlapping rows), RT correction and alignment (`rtmap.rs`, `rtwarp.rs`), kinetics as evidence (`kinetic.rs`, `kinetic/`) | `docs/agenti/mzfeat-broad.md` |
| layering of `crates/mzlab-feat` modules (base modules, feature gates, rules for a new dependency edge) | `docs/agenti/mzfeat-cycles.md` |
| low-resolution feature pipeline (full scan, Da axis, `lr` profile: `recipe_lr.rs`, `axis.rs`) | `docs/agenti/mzfeat-lr.md` |
| Feature table in the Pipeline (steps 5-6: QC, filter bar, `PIPET` with `TPTAB` and expanded row `TPRIGA`) | `docs/agenti/mzfeat-table-view.md` |
| table, ids and samples (area: HR feature pipeline in Rust (`crates/mzlab-feat`)) | `docs/agenti/mzfeat-table.md` |
| tests and measurements (area: HR feature pipeline in Rust (`crates/mzlab-feat`)) | `docs/agenti/mzfeat-tests.md` |
| `tp/2` products view (area: HR feature pipeline in Rust (`crates/mzlab-feat`)) | `docs/agenti/mzfeat-tp.md` |
| "Pipeline" panel and on-demand file family, `family_record` (area: mzFinder in the browser (`crates/mzlab-feat-wasm`,…) | `docs/agenti/mzfeat-wasm-panel.md` |
| mzFinder in the browser (`crates/mzlab-feat-wasm`, `TP_Mine/files/feat-worker.js`, Pipeline panel `TP_Mine/js/32_pipeline.js`, parameter estimation `crates/mzlab-feat/src/params.rs`) | `docs/agenti/mzfeat-wasm.md` |
| HR feature pipeline in Rust (`crates/mzlab-feat`) | `docs/agenti/mzfeat.md` |
| mzFinder benchmark against other tools (`tools/bench/`, `bench/1` table, P1-P5 metrics, statistics) | `docs/agenti/mzfinder-benchmark.md` |
| mzFinder input: `.mzidx` for every chosen file or from IndexedDB (`TP_Mine/files/finder-idx.js`), user transformation list in Rust | `docs/agenti/mzfinder-input.md` |
| LR dig (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-lr.md` |
| MS2 interface and rule, MS2 targets and step "1 · Files" (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-ms2.md` |
| mzFinder: HR products table (`TPTAB`, `TPRIGA`, `TPDEC`), engine progress, parent-TP pairs (`mass_network`, prediction) | `docs/agenti/mzfinder-table.md` |
| untargeted tables, identity and third-party table readers (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-untargeted.md` |
| validation (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/mzfinder-validation.md` |
| mzFinder (`TP_Mine/`, `tpmine-loader.js`) | `docs/agenti/mzfinder.md` |
| `.mzidx` format | `docs/agenti/mzidx.md` |
| mzML conformance matrix (variants, `tools/mzml_variants.py` fixtures, known gaps) | `docs/agenti/mzml-matrix.md` |
| `mzrun/2` file (`*.mzrun.json`: record and presets of an analysis, `self_sha256` fingerprint, application rules, schema `docs/schemi/mzrun-2.json`) | `docs/agenti/mzrun.md` |
| native reading of `.raw`, `.wiff`, `.D` in the site (samples, OPFS, progress, mzML on request) | `docs/agenti/native-reading.md` |
| product network (`crates/mzlab-feat/src/network/`, `rete/1`: edges by rule on `tp/2`, steps, coordinates, edge reason) | `docs/agenti/network.md` |
| derived files (`.mzidx`, `.mzfeat`, `.mzfeatf`) of a newer format than the program: `error.newerFormat`, read-only | `docs/agenti/new-format.md` |
| builder of the EU and US regulatory lists (`mzlab-cli db normative build`, `crates/mzlab-cli/src/normative/`, `normative/sources.json`, sources, pin, `resolve`, output) | `docs/agenti/normative.md` |
| hand-typed numeric fields (`campi.js`, `CAMPI.num`, `data-num`, calculators, `campo.*` rejection note, `numIt` without Infinity, `lib_campi.sweep`) | `docs/agenti/number-fields.md` |
| extremes of large arrays without spread, binary search of points in a window (`numeri.js`: `maxOf`, `minOf`, `maxAbs`, `spanOf`) | `docs/agenti/numbers.md` |
| discarded attempts, CI cuts already measured and Rust job times (area: cumulative speed-gain table (date · PR · step ·…) | `docs/agenti/optimizations-tried.md` |
| panel titles (single rules per panel type × signal, `titoli.js`, "spectrum of" label, exported file names) | `docs/agenti/panel-titles.md` |
| docking and moving panels (handle, drop zones, dividers, undo) | `docs/agenti/panels-side-by-side.md` |
| site ↔ app parity (functions and Flow steps on one side only, engine operations missing in the app, missing parity tests, next steps); function × face × test, guard for `rust-worker.js` ops without a branch (`test_parita_operazioni.py`) | `docs/agenti/parity.md` |
| peak box on a spectrum (hover, keys, live region), highlighted stick, pinned/removed labels (`peak-info.js`, `p.lblOn`/`lblOff`) | `docs/agenti/peak-info.md` |
| spectrum peak table to copy (TSV), y axis in % up to 100, XIC button only on MS¹ | `docs/agenti/peak-table.md` |
| periodic table dialog and `elements/1` (`crates/mzlab-core/src/periodic.rs`, `tavola-periodica.js`, `mzlab-cli ion elements`) | `docs/agenti/periodic-table.md` |
| built-in list of pesticides and environmental contaminants (`mzlab/chem/pesticides.csv`, `pesticides_meta.json`, `tools/download_pesticides.py`, second list of `contaminanti.json`) | `docs/agenti/pesticides.md` |
| substitution position in TP isomers (MS2-compatible classes, `chem/positions.rs`, `posiz/1`) | `docs/agenti/positions.md` |
| single loading bar for every format (`progresso.js`, `PROGRESSO`, `mzlab-index` event, phase weights, monotonicity tests) | `docs/agenti/progress.md` |
| `qc.mass_accuracy` (mass accuracy, table correction, calibration check from the TIC right-click `masscal.js`: `crates/mzlab-feat/src/massacc/`, `flow_massacc.rs`, `flow-web-qc.js`, `flow-views-qc.js`) | `docs/agenti/qc-mass.md` |
| target list over many files (`quant.targets`, `quant targets`: (m/z, RT window) table, one sweep per file, `peak auto-lr` engine, flags below_threshold/edge/in_blank, `--offset auto`, `quant-targets-log/1`, wasm size measures) | `docs/agenti/quant-targets.md` |
| `raw info` / `raw cache`: what a raw file holds for an agent (`raw-info/1`, `raw-cache/1`, `raw-plan/1`: grid, ion quantum, label check, offset, cache entries; route `/api/rawinfo`) | `docs/agenti/raw-access.md` |
| `.raw` → mzML conversion: bench `raw_mzml_bench`, `tools/bench/raw_mzml.py`, T_min, starting points V1-V9, causes of D8 and D11, synthetic dense case, parallel encoding (`convert_times`) | `docs/agenti/raw-mzml.md` |
| register of raw-file fields: extracted or declared ("no loss" rule) | `docs/agenti/raw-not-extracted.md` |
| validation on experimental sites and discarded experiments (area: parent reactivity engine (crates/mzlab-chem, .wasm chem,…) | `docs/agenti/reactivity-studies.md` |
| parent reactivity engine (crates/mzlab-chem, .wasm chem, reatt/1, Tabui 3D viewer) | `docs/agenti/reactivity.md` |
| "andi" module (netCDF/ANDI `.cdf`: `crates/mzlab-lettori/src/andi/`) | `docs/agenti/readers-andi.md` |
| reader modules, one per family (`crates/mzlab-lettori*`, separate `.wasm` in `wasm-lettori/<famiglia>/`, feature `estesi`, `LETTORI_FAM`, rule R in `shell.js`, `LETTORI_MOD`, `?lettori=modulo`, ceilings) | `docs/agenti/readers-module.md` |
| "aperti" module (mzML with numpress, mzXML: `crates/mzlab-lettori/src/open/`, `numpress.rs`) | `docs/agenti/readers-open.md` |
| Thermo `.raw`: activation, "unknown", rejected event table, damaged scans and controllers (`warn.raw.damaged`), precursors (target, selected ion, MSn chains, SPS, MSX, SRM from the event) | `docs/agenti/readers-thermo-precursors.md` |
| recipes for whole campaigns: kinetic LR campaign (vendor files, `--name sample`, roles, `quant.targets`, `peak auto-lr` check, `tp.dig_lr` per condition, LR export; plan template `docs/recipes/`, `workflow_kinetic_tests.rs`) | `docs/agenti/recipes.md` |
| Known contaminants panel: Add list (CSV, template, row errors), provenance of the lists, `reflist/1`, `mzlab-cli db ref-list` | `docs/agenti/reference-lists.md` |
| release: tag `vX.Y.Z`, `release.yml`, `CHANGELOG.md`, single version, SBOM, gate (`gate_release.py`); Dependabot, CodeQL, security settings to turn on by hand | `docs/agenti/release.md` |
| full reset and Files mode of the browser reset: what is left in OPFS and storage, how to measure (`tools/prepara_sito.py`, `tests_e2e/misura_reset.py`) | `docs/agenti/reset.md` |
| native mzFinder (area: Rust engine (`crates/`, golden data `tools/golden.py`)) | `docs/agenti/rust-desktop-finder.md` |
| desktop (area: Rust engine (`crates/`, golden data `tools/golden.py`)) | `docs/agenti/rust-desktop.md` |
| `mzflow/2` plans, check and `prov/1` (area: Rust engine (`crates/`, golden data `tools/golden.py`)) | `docs/agenti/rust-flow.md` |
| mzidx (area: Rust engine (`crates/`, golden data `tools/golden.py`)) | `docs/agenti/rust-mzidx.md` |
| routes (area: Rust engine (`crates/`, golden data `tools/golden.py`)) | `docs/agenti/rust-routes.md` |
| `step/1` descriptors (area: Rust engine (`crates/`, golden data `tools/golden.py`)) | `docs/agenti/rust-step.md` |
| Rust engine (`crates/`, golden data `tools/golden.py`) | `docs/agenti/rust.md` |
| sample file roles, times and metadata (`samples/1`, `mzlab-core/src/samples/`: proposals from names, name patterns, CSV, free columns, data from file, warnings, indexes for the TP path) | `docs/agenti/samples.md` |
| scandump, RFR oracle | `docs/agenti/scandump.md` |
| service worker, per-version site cache, `sw-manifest.json`, updates | `docs/agenti/service-worker.md` |
| Shimadzu `.qgd` and `.lcd` reader (`crates/mzlab-lettori/src/shimadzu/`, `shimadzu` module) | `docs/agenti/shimadzu.md` |
| multi-library spectral matching engine for mzFlow ("Attribution" node, `table.attribute`) and MSⁿ side panel: `libset`, `libmatch/1`, presets, analogs with ΔFormula, `libraries/2` catalog | `docs/agenti/spectral-match.md` |
| MS² spectrum card header (`ms2c`), the removed info rows (notes, retention times, CE, sample) and where `p.note` is still stored, `mzlab-cli file items` | `docs/agenti/spectrum-rows.md` |
| progressive startup (`AVVIO`), «interface at once» rule; loading area and table (area: header (full screen, logo, (i)), loading area and table,…) | `docs/agenti/startup.md` |
| statistics as steps: Welch/ANOVA with BH, PCA and KMD in Rust (`crates/mzlab-feat/src/stat/`, `table.stats`, `table.pca`, `table.kmd`, `stat/1`) | `docs/agenti/stat.md` |
| scientific correctness study: fixed findings (one line each) and the open owner decisions A4, C2, D2 | `docs/agenti/study-science.md` |
| security study: exploitable weaknesses with a crafted file or by probing the published site (Rust readers, front end, desktop app, supply chain), severity, `file:line`, how to reproduce, fix | `docs/agenti/study-security.md` |
| spectral study on synthetic data: fragmentation tree generator, scenarios E0-E6, arms against the engine (calibrated null, coincidence p-value, sub-formulas, multi-shift analogs), MISURA studio_* rows | `docs/agenti/study-spectra.md` |
| suspect index (area: mzFinder (`TP_Mine/`, `tpmine-loader.js`)) | `docs/agenti/suspects.md` |
| test structure, new tests | `docs/agenti/test.md` |
| Thermo `.raw` reader speed: criteria A1-A7 and B1-B5, bench `thermo_bench`, `tools/bench/thermo_velocita.py`, baseline numbers against RFR (T-7) | `docs/agenti/thermo-speed.md` |
| chemical background of the TIC/BPC (persistent background ions, per-ion baseline, quiet window, blank, excluded m/z windows: `ticbg.rs`, `api/tic_background`, `mzlab-cli trace background`) | `docs/agenti/tic-background.md` |
| provisional TIC: zoom/pan of the RT axis while the index is being built, window handoff to the first panel | `docs/agenti/tic-provisional.md` |
| SIM/MRM transition table of a triple quadrupole (`chrom.transitions`, `transitions/`, per-sample integration, RT candidates, example presets) | `docs/agenti/transitions.md` |
| .wasm ceilings: rules, where each ceiling and measurement lives (`tools/moduli/*.toml`), what costs most, move steps | `docs/agenti/wasm-caps.md` |
| WebAssembly potential (`opt-level` profiles, SIMD128, binaryen, zlib-rs, threads, memory64, minimum browsers; measurements) | `docs/agenti/wasm-potential.md` |
| Waters MassLynx `.raw` folder reader (`crates/mzlab-lettori/src/waters/`, `waters` module, grouping in `waters-raw.js`) | `docs/agenti/waters.md` |
| full binary specification and what a sample is made of (area: `.wiff` + `.wiff.scan` reader `crates/mzlab-core/src/wiff/`; read-only `file inventory|verify`) | `docs/agenti/wiff-full-spec.md` |
| a `.wiff` or `.wiff.scan` loaded alone: which file is its companion (shared Rust rule, `mzlab-cli file pair`), the folder prompt of the page (`wiff-pair.js`), what each browser does | `docs/agenti/wiff-pair.md` |
| copying only the byte ranges of the chosen samples of a big `.wiff.scan` into OPFS (`readers::ranges`, `sample select --ranges`, `sample_ranges`, `putSparse`, partial identity Q4 left open) | `docs/agenti/wiff-range-copy.md` |
| choosing samples of a `.wiff` (or other multi-sample file): listing, `selection::suggest`, `--sample LIST`, the page dialog `wiff-choose.js`, what is not built | `docs/agenti/wiff-sample-choice.md` |
| other SCIEX instruments, sMRM, PIS/NL, feature `estesi` (area: `.wiff` + `.wiff.scan` reader (`crates/mzlab-core/src/wiff/`) | `docs/agenti/wiff-variants.md` |
| `.wiff` + `.wiff.scan` reader (`crates/mzlab-core/src/wiff/`) | `docs/agenti/wiff.md` |
| signed offset after the m/z or formula in the "Extract ions" window (`288.2+16`), `split_offset`, `mzlab-cli ion mz`, `err.formula.offset`, `err.formula.mzLow`; "m/z from / to" fields and m/z / RT pairs (`xic-bordi.js`, `ctl.range.order`); `tabNuovo`, `msg.noFiles` | `docs/agenti/xic-discard.md` |
