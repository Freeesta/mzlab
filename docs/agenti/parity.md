# Parity of the three faces: function x face x test
Rule (`CLAUDE.md`): every function exists in the site (`.wasm` + OPFS) and in the app (native desktop or `mzlab-cli`), with a byte-for-byte test. Here the state of each class of functions; formats in `native-reading.md` ("Recognised formats"), CLI steps in `cli-passi.md`.

## Automatic guards
- **Page engine operations**: `tests/test_parita_operazioni.py` reads every op of `mzlab/web/rust-worker.js` and requires a branch in the app (`shim/desktop-worker.js` or an `"op" =>` arm of `crates/mzlab-desktop/src/*.rs`); the site-only ops are in `SOLO_SITO` with the reason, and the list cannot name ops that the app serves or that the site does not have.
- **Flow steps in the browser**: `tests/test_parita_passi.py` reads the steps of the registries (`docs/schemi/steps-*-1.json`) and requires for each an executor in `F.liveJobs` (`flow/lazy/flow-live-jobs.js` and the files that complete it); the steps the browser does not run are in `SOLO_APP` with the reason ("gap" = to close), the list cannot name steps that the browser runs or that the registry does not have, and every browser executor is a step of the registry. The native side is `flow_rule_tests.rs` (`native: true` = executor of `flow run`).
- **Copyable command, common routes, conversion, validation, channels**: tests below, one per row.
- **Determinism**: std `HashMap`/`HashSet` have a random per-process seed in native and fixed keys in `wasm32-unknown-unknown`: an iteration order reaching an output would give different bytes between two app runs and between app and site. No engine map does so (checked on all crates outside `mzlab-lib`: only lookups, counts, min/max; `serde_json` without `preserve_order`, so `Value` is sorted).
- **Math functions**: std `exp`, `ln`, `powf`, `sin` come from the `libm` port in the `.wasm` and from the system library in native, different in the last bit: `mzlab-core`, `mzlab-picchi`, `mzlab-feat`, `mzlab-lib` and `mzlab-target` call `libm::` everywhere; `mzlab-chem` (`fm.rs`) calls `libm` in native and std in the `.wasm`, which there gives the same bits (2·10⁶ tests each). `powi` and `sqrt` stay std. Guard: `test_guard_engine_math_from_libm`; tests: `the_gaussian_apex_`, `slof_gives_the_bits_of_libm_on_every_face`, `the_origin_route_gives_the_same_report_in_the_app_and_in_the_wasm`.
- **Threads and clocks**: no f64 sum depends on thread order (parallel jobs write by index), no clock enters an output except `ms`/`heap_peak_bytes` of `prov/1` (`cli.md`, "Reproducibility").

## Page engine (ops of `rust-worker.js`)
| Function | Site | App | CLI | Test |
|---|---|---|---|---|
| Common routes (TIC, XIC, spectra, MRM, maps, channels, info, statistics) | engine `.wasm` | `rpc.rs` | `channels`, `peak table` | `crates/mzlab-desktop/tests/parity_routes.rs` (37 steps, `.wasm` with `MZLAB_WASM_PKG` = app), `test_canali_wasm.py` |
| Run type proposed by the channels (`channels::experiment_of`): no SRM scans = `lc-mrm`, SIM only = `lc-sim` (also with a DAD), DAD only = `uv-only` | engine `.wasm` | `rpc.rs` | `channels` | `channels/tests.rs` (`a_file_of_sim_chromatograms_only_is_typed_sim_also_with_a_uv_trace`); the three faces call the same function |
| MSn graph (`msngrafo`), centroids (`lr_index_cen`, `mzml2cen`), SRM for mzFinder (`natsrm`) | yes | `rpc_msn.rs` | `msn graph` | `parity_msn.rs` (app = core), `test_msngrafo_wasm.py` (`.wasm` = CLI) |
| Instrument log of a `.raw` (`natregistro`, `/api/registro`) | `thermo` module (`registroDi`) | `rpc_nat.rs` | - (no command; the CLI's `method/1` lists the series) | `crates/mzlab-desktop/tests/natregistry.rs` (app = log reader + `mzlab_lettori::json::chromatogram_json`, the function the module calls); the synthetic `.raw` has no status log: the test goes through the TIC and through the error of an absent series |
| Background mass (`masscal`) | yes | `rpc.rs` | - | `parity_masscal.rs` |
| Copyable command, short line (`lr_flow_command`, `flow/line`) | `.wasm` `rotte` | `rpc_lr.rs`, `flow/line` | `flow run` | `parity_flow_command.rs` |
| Step defaults, `step-params/1` (`lr_flow_defaults`) | `.wasm` `rotte` | `rpc_lr.rs` | `flow defaults` | `parity_flow_defaults.rs` |
| Series union, `union/1` (`lr_union`) | `.wasm` `rotte` (`union_tables`) | `rpc_lr.rs` | `quant union` | `parity_union.rs` |
| mzML validation (`api/mzml_valida`) | `valida` module | `lr_valida` | `validate` | `test_valida_wasm.py`, `crates/mzlab-desktop/tests/rpc.rs` |
| Site-only by nature | `module`, `policy`, `budget`, `down`, `lr_gunzip`, `lr_cen_have`, `lr_cen_adopt`; `lr_flow_line`/`lr_flow_sched` (the app uses `flow/line`, `flow/live/*`) | - | - | `SOLO_SITO` |
| Load "Cancel" (`lr_cancel`) | stops copies and indexes | `IdxBuilder::cancel`: the file's index builds stop at the next read or write and leave no files (neither `.part` nor index); `lr_remove` also removes the file's indexes | - | `crates/mzlab-desktop/tests/cancel.rs`, `idxbuild_tests.rs` (app); `e2e_annulla_orfani.py` (site) |

## Flow steps (`mzlab-cli flow steps`, site executors in `F.liveJobs`)
| Steps | Site | App/CLI | Test |
|---|---|---|---|
| `index.mzidx`, `features.detect/align/gapfill`, `spectra.centroid` | yes | yes | `e2e_flow_web.py`, `e2e_flow_centroid_web.py`, `test_feat_wasm.py` |
| `table.filter`, `table.drift`, `table.plausible`, `rt.predict`, `features.group`, `export.table_csv`, `export.mztab`, `export.inclusion`, `export.annotations`, `chrom.transitions`, `qc.run_order`, `report.methods`, `table.stats`, `table.pca`, `table.kmd`, `table.compare`, `spectra.lookup` | yes | yes | `e2e_flow_web_export.py` (filter and exports), `e2e_flow_web_drift.py`, `e2e_flow_web_stat.py`, `tests/test_compare_wasm.py`, `e2e_flow_web_group.py`, `e2e_flow_neg_lr.py`, `e2e_flow_export_annot.py`, `e2e_flow_transitions.py`, `e2e_flow_qcreport.py`, `e2e_flow_web_lookup.py` |
| `quant.calibrate`, `quant.curve`, `chrom.uv_peaks` | yes | yes | `e2e_flow_web_quant.py`, `test_flowcal_curve_wasm.py`, `e2e_flow_web_uv.py` |
| `spectra.ms2`, `spectra.match`, `table.match`, `formula.ms2`, `formula.ms1`, `annot.level`, `qc.mass_accuracy`, `screen.suspects`, `annot.regulatory`, `spectra.insilico` | yes | yes | `e2e_flow_web_ident.py`, `e2e_flow_web_ident2.py`, `e2e_flow_qc_mass.py`, `e2e_flow_suspects.py`, `e2e_normative_screen.py`, `e2e_flow_insilico.py` |
| `tp.*` (7), `spectra.msn_graph`, `msn.library_export`, `msn.table_export` | yes | yes | `e2e_flow_tp_*.py`, `e2e_flow_dig_lr.py`, `e2e_flow_msn.py` |
| `structure.render`, `structure.reactivity`, `structure.moiety`, `msn.figure` (svg, html) | yes | yes | `e2e_flow_web_struct.py` (`.wasm` `interp`, `chem`, `msnfigura`), `e2e_flow_web_moiety.py` (`molkit`, `flow_moiety`); the PNG stays on the computer (resvg native only) |
| `features.estimate` (one `estimates/1` per file, `<instance>-<root>.json`) | yes | yes | `e2e_flow_web_estimate.py` (`.wasm` `hr`, `flow_estimate` = `run_estimate::estimate_of`); the root of the name is given by the scheduler (`root` of `next`) |
| `import.raw`, `convert.mzml`, `features.table`, `spectra.chimeric`, `meta.compare` | no as a step (many in the `.wasm` of a panel) | yes | to do (Flow area: executors in `F.liveJobs` with a test equal to `flow run`); list in `SOLO_APP` of `tests/test_parita_passi.py` |
| `features.ms2_link`, `annot.hazard` | no | no (descriptor only) | - |
| `structure.draw` | yes (editor) | no | limit: a drawing is not a computation |

## Reading, conversion, export
| Function | Site | App/CLI | Test |
|---|---|---|---|
| Conversion to mzML (8 families) | `wasm-lettori/*` modules (`to_mzml`) | `convert`, `nat2mzml` | `test_lettori_parita.py` (same SHA-256, synthetic data) |
| "Download .mzML" (every compression) of a sample converted by the module at load | `conversione.js` `again` (`to_mzml_with`, without the optical matrix; the load file stays with the UV panel) | `convert --compression C` (conv.dad false) | `test_wiff_scarica_mzml.py` (`mistopi` sample 2, `dadmetodo` with a DAD; byte for byte), `test_conversione.py`, `e2e_numpress_modulo.py` |
| «Name files by» sample name of the mzML to save | `conversione.js` `syncNames` → `api/samples/mzml_names` (rotte .wasm) | same route `lr_samples`; `convert --name sample`; `save_mzml_folder` names | `tests_e2e/e2e_name_by.py` (site names and notes = CLI `--dry-run`), `samples_file_names.rs` (route = function), `conv_queue_tests.rs` |
| mzML with Numpress as input | `aperti` module | native path | `numpress_mzml.rs` |
| Direct reading without mzML (6 families) | via mzML in OPFS | direct | to do (open in the site Waters, Agilent LC/MS, Bruker, Shimadzu, ANDI, mzXML for the engine like Thermo) |
| Files over 4 GiB, threads inside a file | no | yes | limit: memory64 and `atomics` (`wasm-potential.md`) |
| SRM: collision energy and dwell of the id (`ce=`, `dwell=`) in the mzML | every readers module | `convert`, `nat2mzml` | `mzmlwrite/tests.rs` (`an_srm_id_with_energy_and_dwell_writes_them_in_every_build`): `srm_windows` without `cfg(feature = "sciex")` |
| Product ion: energy and precursor isolation window in the `.mzidx` of a `.wiff` | engine (without `sciex` or `precursori`) | `flow run` (`index.mzidx`) | `test_wiff_energia_parita.py` (byte for byte, synthetic `energia.wiff`); `mzmlwrite/tests.rs` (`a_plain_precursor_keeps_its_energy_and_window_in_every_build`) |
| Source name (`source.name`) of an `.mzidx` read from a vendor file | engine (`upload::set_name`) | `flow run` (`index.mzidx`) | `tests/mzidx_rows_direct.rs` (`the_source_of_a_vendor_index_is_the_dataset_name`): `X.raw`, `X.wiff`, `X.D` whatever name the page gives the list entry |
| Features from a `.raw` and from its mzML | - | `features` | limit: same result only for centroid acquisitions; a profile `.raw` uses the vendor centroids, its profile mzML uses mzLab's centroiding (area of `features` and of the Thermo reader) |

## Checks on real data (outside the repository)
- `.mzidx` of 6 mzML, channels (7 inputs), `peak table` of a `.wiff` (windows on SIM ions, with and without sums): site and CLI equal byte for byte; differences found and closed: TIC of the `.wiff` (f32/f64), SRM and precursor energy, type of a SIM-only run, source name of the native index.

## Priorities
- In order: conversion parity for every readers module; missing branches in the app for the page engine ops; site <-> app route parity route by route; `prov/1` in the site Flow; table and export steps in the site; centroids of a profile mzML in the site Flow; Attributio steps in the site; plan check in Rust instead of `flow-check.js`; direct reading in the site of the other families; a single "where it runs" registry (`runs_in` of the descriptor aligned with the real executors).
- **Declared impossible**: threads inside a file and over 4 GiB in the site, app update from the site.
