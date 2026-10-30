# Negative polarity and unit resolution in mzFlow and Tabui: the matrix

Audit of every step of the registry (`mzlab-cli steps`, 49 steps) and of Tabui in high resolution (HR) and unit resolution (LR, a triple quadrupole such as the SCIEX 3200 QTRAP), in positive and negative polarity, in the browser («This browser», `flow/lazy/flow-live-jobs.js`) and in the app (`mzlab-cli flow run`, the same `FileRunner` the desktop app calls). Goal (Federico): low resolution must be perfect and the site equal to the app. The build prompts that close the gaps are in `prompt_neg_lr/`.

## 1. How it was measured

- Fixture: the invented bentazone series of `crates/mzlab-cli/src/flow_bentazone_tests.rs` (blank, t0 and three times; the parent fades, two products grow), written in the four combinations HR+, HR-, LR+ (ITMS, ±0.05 Da) and LR-. No laboratory data.
- Native: `crates/mzlab-cli/src/flow_neglr_tests.rs` runs the base chain, Tabui and the table steps in the four combinations (`every_table_step_runs_in_both_polarities_and_both_resolutions`); its `gap_*` tests are ignored and fail today, one per gap (`cargo test --release -p mzlab-cli --lib flow_neglr -- --ignored`).
- Browser against native: `tests_e2e/e2e_flow_neg_lr.py` runs the same plan in the page (OPFS indexes, `FLOW.runners.web.run`) and in `flow run`, and compares every kept text byte for byte (gen, flt, pos, net, fil, csv, mzt and, at LR, dig). Green in the four combinations when the plan writes the polarity and the unit mass and the files are in name order; `NEG_LR_UNWRITTEN=1` shows gap NL-7.
- Gaps owned by other chats are named, not re-audited: LR-TOL (tolerance parameters with minimum 0 in features.align, features.gapfill and features.ms2_link; `recipe_map.rs` overwriting the Da recipe with ppm), LR-NOISE, P3 (the browser cannot centroid a profile mzML), P4 (LR template).

## 2. Legend

- **P**: proven by the named test, output checked.
- **R**: runs without error in a named test; the values are not checked.
- **W**: works by reading the code, no test in this combination.
- **B-n**: broken, gap NL-n of §4 (evidence there).
- **D**: blocked by design (the step needs what the combination does not have).
- **?**: unknown, not verified.
- **—**: the step does not exist on that side.

## 3. The matrix

Native = `flow run` (and the desktop app); Browser = «This browser». Columns: HR+ / HR- / LR+ / LR-.

| Step | Native HR+ / HR- / LR+ / LR- | Browser HR+ / HR- / LR+ / LR- | Tests |
|---|---|---|---|
| import.raw, convert.mzml | W / W / W / W | — | conversion tests of the readers; polarity is the reader's |
| index.mzidx | P / P / P / P | ? / ? / ? / ? | bentazone fixture (the index of every native case); the browser index is built by Data, not by the plan (the e2e puts the native `.mzidx` in OPFS) |
| features.estimate | R / B-1 / B-1 / B-1 | — | `every_table_step…` (runs), `the_estimate_measures_a_negative_file_once_the_polarity_is_written`, gap NL-1 |
| features.detect, features.align, features.gapfill | P / P / P / P | P / P / P / P | `flow_bentazone_tests`, `e2e_flow_neg_lr` (csv, mzt and fil equal byte for byte); mixed series: B-4, B-5; LR tolerances: LR-TOL |
| table.filter | P / P / P / P | P / P / P / P | `e2e_flow_neg_lr` (fil); `dup_ppm` has no Da form (NL-12) |
| table.drift, table.stats | W / W / W / W | — | `flow_drift_tests`, `flow_stats_tests` (HR+ only); polarity-free on the table |
| table.pca | D / D / D / D | — | needs more samples than the fixture (`err.stat.pca.few`), polarity-free |
| table.kmd, table.plausible | R / R / R / R | — | `every_table_step…`; `flow_kmd_tests` and `flow_plausible_tests` read the unit axis |
| features.ms2_link | — | ? / ? / ? / ? | page only; LR tolerance: LR-TOL |
| export.table_csv | P / P / P / P | P / P / P / P | `e2e_flow_neg_lr` (series in injection order), `gap_the_samples_are_matched_to_the_files_of_the_table_by_name` |
| export.mztab | P / P / P / P | P / P / P / P | `every_table_step…` (polarity word), `export_mztab.rs::same_bytes_every_time_and_negative_polarity`, `e2e_flow_neg_lr` |
| export.inclusion | P / B-10 / W / B-10 | — | `the_inclusion_list_takes_a_negative_adduct` (adduct written); default adduct and wiring: NL-10 |
| features.table | R / R / R / R | — | `formula_plan` of `flow_neglr_tests` |
| spectra.chimeric | W / ? / W / ? | — | `flow_chimeric_tests` (HR+); declares `requires resolution hr` |
| formula.ms1 | P / B-2 / D / D | W / ? / D / D | `formula_ms1_reads_a_negative_file_with_a_negative_ion`, gap NL-2, `e2e_flow_web_ident2` (HR+, browser = `flow run`); declares hr |
| formula.ms2 | W / ? / D / D | W / ? / D / D | `flow_formula_web_tests` (HR+); declares hr |
| spectra.match | P / ? / P / ? | W / ? / W / ? | `flow_match_tests::precursor_da_matches_a_unit_resolution_query`; default `tol_da` 0.02: NL-11 |
| spectra.insilico | W / W / D / D | — | `flow_insilico_tests` (HR+); POS/NEG read from the MGF (`insilico.rs::rank`); declares hr |
| rt.predict | W / W / W / W | — | `flow_rtpredict_tests`; polarity-free |
| annot.level | W / ? / D / D | W / ? / D / D | `flow_level_tests` (HR+); declares hr |
| annot.regulatory, annot.hazard, structure.draw | — | W / W / W / W | page only, polarity-free |
| tp.generate, tp.link, tp.filter, tp.position, tp.network | P / P / P / P | P / P / P / P | `flow_bentazone_tests` (four combinations), `flow_lr_tests::tp_link_reads_a_unit_resolution_table`, `e2e_flow_neg_lr`; the browser needs the polarity and unit mass in the plan: B-7; facets: B-6 |
| tp.review | W / ? / W / ? | W / ? / W / ? | `flow_review_tests` (HR+) |
| tp.dig_lr | R / R / P / P | — / — / P / P | `flow_lr_tests`, `crates/mzlab-finder-lr-wasm/tests/lr_negativo.rs`, `e2e_flow_dig_lr` (LR+), `e2e_flow_neg_lr` (LR±); it also runs on HR files (no facet declared) |
| structure.render, structure.reactivity | W / W / W / W | — | `flow_render_tests`; reactivity R in `every_table_step…`; polarity-free |
| spectra.centroid | W / W / P / W | ? / ? / ? / ? | `flow_centroid_tests` (unit grid); profile in the browser: P3 |
| spectra.ms2 | W / ? / W / ? | W / ? / W / ? | `flow_ms2_tests` (HR+); IONMODE from the scans (`flow_ms2.rs`) |
| spectra.msn_graph, msn.library_export, msn.table_export, msn.figure | W / W / ? / ? | W / ? / ? / ? | `flow_msngraph_tests`, `msn_figura_tests` (negative polarity in the figure), `msngrafo/tests_formule.rs::a_negative_infusion_is_read_the_same_way` |
| meta.compare, spectra.lookup | W / ? / W / ? | — | `flow_compare_tests`, `flow_lookup_tests` (HR+) |
| table.match | W / B-3 / W / B-3 | W / ? / W / ? | `flow_tablematch_tests`, `flow_match_web_tests` (HR+), gap NL-3 |
| chrom.uv_peaks | W / W / W / W | — | `flow_uv_tests`; UV, no polarity |
| quant.calibrate | W / W / W / W | — | `flow_quant_tests`; polarity-free on the table |
| qc.mass_accuracy | R / R / B-11 / B-11 | ? / ? / ? / ? | `every_table_step…` (runs); at LR it measures ppm on unit data: NL-11 |
| screen.suspects | ? | ? | not in the registry yet |
| Tabui page (`TP_Mine/`) | P / P / P / P | P / P / P / P | the Flow tp.* steps above are the engine of Tabui; Tabui's own LR- page: `lr_negativo.rs` |

## 4. Gaps ranked by user impact

1. **NL-8 — samples matched to the files of the table by name (closed, both sides).** Every step that holds a table puts the samples document in the order of the table's files first: `mzlab_core::campioni::order_like` on the names the `.mzfeat` records (meta `inputs`), matched with or without folders and file extensions; a table file with no sample, or a sample naming no file of the table, is `error.badCampioni` with the name in `file`. Callers: `core::load` and `tp_table_cond_json` of the table `.wasm` (export.table_csv, export.mztab, table.filter, table.match and the views), `TpRoles::for_table` (tp.link native and browser), tp.network, tp.dig_lr (the names of its indexes), the proofs of the feat `.wasm`; table.stats, quant.calibrate and qc.mass_accuracy already matched by name.
   Evidence: `gap_the_samples_are_matched_to_the_files_of_the_table_by_name` (reversed samples file, same CSV), `a_sample_naming_a_file_the_table_lacks_is_refused_with_its_name`, `e2e_flow_neg_lr` with the series in injection order (T00, T2p5, T5, T10p25 against the OPFS name order).
2. **NL-7 — the browser does not read the polarity or the unit mass of the files (broken, browser).** `flow run` takes them from the indexes (`flow_nature.rs`); the browser runner passes `polarity: NaN` when the plan does not write it (`flow-live-jobs.js:272-273`).
   The page writes them from the files open in Data (`inheritNature`, `flow-samples.js`), so an imported or hand-built plan, or a plan run with the Data tab empty, runs negative files as positive: `err.flow.link.parent` (mz 241.064) in HR- and LR-, and in LR+ tp.generate keeps the HR masses (92 native hypotheses, 145 in the browser). Evidence: `NEG_LR_UNWRITTEN=1 python3 tests_e2e/e2e_flow_neg_lr.py`.
3. **NL-2 — formula.ms1 keeps [M+H]+ on negative files (broken, native).** The sign comes only from the `ion` parameter (`crates/mzlab-feat/src/flow_formula_ms1.rs:95`, default [M+H]+): every row of a negative series ends in `err.flow.formula.scan`. Evidence: `gap_formula_ms1_takes_the_ion_of_the_polarity_of_the_files`. The browser executor (`flow/lazy/flow-web-ident.js`, formula `.wasm`) runs the same Rust: not run in negative here.
4. **NL-3 — table.match writes the MGF of a negative table as positive (broken, native; the browser calls the same Rust, not run there).** `rows_mgf.rs:36` reads `pol::recipe_polarity`, which is +1 when the plan does not write the polarity, instead of `pol::of_table` (the files); the library search filters by the query's polarity, so a negative table finds nothing or the wrong polarity. Evidence: `gap_the_polarity_of_the_mgf_of_table_match_is_the_one_of_the_files` (1 instead of -1).
5. **NL-1 — features.estimate ignores the files' polarity and resolution (broken, native).** `flow_estimate.rs` applies the defaults (`polarity` 1, axis ppm): on negative files noise and n are null; on LR files it reports axis ppm and an HR `min_height` of 2e5 (with axis Da written it gives 0.1 Da, still 2e5). Evidence: `gap_the_estimate_follows_the_polarity_and_the_resolution_of_the_files`.
6. **NL-4 — a mixed-polarity series is aligned in silence (broken, both).** 2 negative + 3 positive files: one table, tp.generate and tp.link at +1, no warning. `align_setup` (`flow_base.rs`) rebuilds the recipe from the first file. Evidence: `gap_a_series_of_mixed_polarity_is_refused_or_warned`.
7. **NL-5 — a mixed-resolution series is aligned on the first file's axis in silence (broken, both).** An LR file first aligns HR files in Da (and tp.generate goes nominal false). Evidence: `gap_a_series_of_mixed_resolution_is_refused_or_warned`. The Da/ppm parameters themselves: LR-TOL.
8. **NL-6 — the resolution facets are wrong and never declared (broken, both).** tp.generate, tp.link, tp.filter, tp.position, tp.network and tp.review declare `requires resolution hr` but run and are proven at LR; a plan that declares `facets: {resolution: [lr]}` is refused (`err.flow.facet`, `mzlab_core::flow::check::facets`, `flow-check.js`).
   The facets are set only by hand (`flow-page.js:47`), so the HR-only steps (formula.ms1, formula.ms2, spectra.insilico, spectra.chimeric, rt.predict, annot.*) run on LR files unguarded. tp.dig_lr declares lr but also runs on HR files. Evidence: `gap_a_unit_resolution_plan_that_says_so_passes_the_check`.
9. **NL-10 — export.inclusion: default adduct [M+H]+ whatever the files; tp.link.tp cannot feed it.** It reads a JSON list `{formula, rt}`; wired from `lnk.tp` (`.mzfeat`) it fails with `error.badIndex` in all four combinations. With `adducts` written it is right (`the_inclusion_list_takes_a_negative_adduct`).
10. **NL-11 — LR MS2 and QC tolerances.** `spectra.match` and `table.match` default `tol_da` 0.02, too narrow for unit-resolution MS2; `qc.mass_accuracy` measures ppm on LR files instead of refusing or switching to Da.
11. **NL-12 — `table.filter dup_ppm` has no Da form** (`crates/mzlab-feat/src/filterspec.rs:106`): at LR the duplicate window is 10 ppm of a 0.1 Da axis.
12. **NL-13 — cosmetic: prov records the features.detect preset as «hr» on LR files.**

## 5. Not verified

- MS2-based steps in negative polarity and at LR with real MS2 (spectra.ms2, spectra.match, table.match, formula.ms2, annot.level, tp.review, spectra.msn_graph and the msn.* exports): the fixture is full scan only; the cells above are read from the code and from HR+ tests.
- The browser index built by Data (`index.mzidx` in the page) and the profile path (P3).
- `screen.suspects` (not in the registry).
- Real 3200 QTRAP files (negative or positive) and real HR negative files: only synthetic data were used in this public audit.
- The desktop app itself: `flow run` was used as the native side (the app calls the same `FileRunner`); no test drove the Tauri window.
