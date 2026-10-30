# Negative polarity and unit resolution in mzFlow and Tabui: the matrix

Audit of every step of the registry (`mzlab-cli steps`, 49 steps) and of Tabui in high resolution (HR) and unit resolution (LR, a triple quadrupole such as the SCIEX 3200 QTRAP), in positive and negative polarity, in the browser («This browser», `flow/lazy/flow-live-jobs.js`) and in the app (`mzlab-cli flow run`, the same `FileRunner` the desktop app calls). Goal (Federico): low resolution must be perfect and the site equal to the app. The build prompts that close the gaps are in `prompt_neg_lr/`.

## 1. How it was measured

- Fixture: the invented bentazone series of `crates/mzlab-cli/src/flow_bentazone_tests.rs` (blank, t0 and three times; the parent fades, two products grow), written in the four combinations HR+, HR-, LR+ (ITMS, ±0.05 Da) and LR-. No laboratory data.
- Native: `crates/mzlab-cli/src/flow_neglr_tests.rs` runs the base chain, Tabui and the table steps in the four combinations (`every_table_step_runs_in_both_polarities_and_both_resolutions`); its `gap_*` tests are ignored and fail today, one per gap (`cargo test --release -p mzlab-cli --lib flow_neglr -- --ignored`).
- Browser against native: `tests_e2e/e2e_flow_neg_lr.py` runs the same plan in the page (OPFS indexes, `FLOW.runners.web.run`) and in `flow run`, and compares every kept text byte for byte (gen, flt, pos, net, fil, csv, mzt and, at LR, dig). Green in the four combinations whether or not the plan writes the polarity and the unit mass (default: unwritten, both sides read them from the indexes; `NEG_LR_WRITTEN=1` writes them); the series is in injection order, not in the OPFS name order.
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
| features.estimate | P / P / P / P | — | `gap_the_estimate_follows_the_polarity_and_the_resolution_of_the_files`, `the_estimate_measures_a_negative_file_once_the_polarity_is_written`; no browser executor (`flow-live-jobs.js`): the step stays with the computer |
| features.detect, features.align, features.gapfill | P / P / P / P | P / P / P / P | `flow_bentazone_tests`, `e2e_flow_neg_lr` (csv, mzt and fil equal byte for byte); a series that mixes polarities or resolutions is refused on both sides (`err.flow.align.polarity`, `err.flow.align.resolution`, `gap_a_series_of_mixed_*`, `e2e_flow_neg_lr`); LR tolerances: LR-TOL |
| table.filter | P / P / P / P | P / P / P / P | `e2e_flow_neg_lr` (fil, fdu: `dup_da` 0.3 on LR, `dup_ppm` 10 on HR; the wrong axis is `err.flow.param.axis`) |
| table.drift, table.stats | W / W / W / W | — | `flow_drift_tests`, `flow_stats_tests` (HR+ only); polarity-free on the table |
| table.pca | D / D / D / D | — | needs more samples than the fixture (`err.stat.pca.few`), polarity-free |
| table.kmd, table.plausible | R / R / R / R | — | `every_table_step…`; `flow_kmd_tests` and `flow_plausible_tests` read the unit axis |
| features.ms2_link | — | ? / ? / ? / ? | page only; LR tolerance: LR-TOL |
| export.table_csv | P / P / P / P | P / P / P / P | `e2e_flow_neg_lr` (series in injection order), `gap_the_samples_are_matched_to_the_files_of_the_table_by_name` |
| export.mztab | P / P / P / P | P / P / P / P | `every_table_step…` (polarity word), `export_mztab.rs::same_bytes_every_time_and_negative_polarity`, `e2e_flow_neg_lr` |
| export.inclusion | P / B-10 / W / B-10 | — | `the_inclusion_list_takes_a_negative_adduct` (adduct written); default adduct and wiring: NL-10 |
| features.table | R / R / R / R | — | `formula_plan` of `flow_neglr_tests` |
| spectra.chimeric | W / ? / W / ? | — | `flow_chimeric_tests` (HR+); declares `requires resolution hr` |
| formula.ms1 | P / P / D / D | W / W / D / D | `formula_ms1_reads_a_negative_file_with_a_negative_ion`, `gap_formula_ms1_takes_the_ion_of_the_polarity_of_the_files`, `a_written_ion_of_the_other_polarity_wins_with_a_warning`, `e2e_flow_web_ident2` (HR+ and HR- with no ion written, browser = `flow run`); declares hr |
| formula.ms2 | W / ? / D / D | W / ? / D / D | `flow_formula_web_tests` (HR+); declares hr |
| spectra.match | P / ? / P / ? | W / ? / W / ? | `flow_match_tests::precursor_da_matches_a_unit_resolution_query`; a unit-resolution query (the note of the unit MGF, written by `table.match` for LR files) takes 0.5 Da fragments and a 0.7 Da precursor window when the plan writes none (`a_unit_resolution_query_follows_its_resolution_by_default`) |
| spectra.insilico | W / W / D / D | — | `flow_insilico_tests` (HR+); POS/NEG read from the MGF (`insilico.rs::rank`); declares hr |
| rt.predict | W / W / W / W | — | `flow_rtpredict_tests`; polarity-free |
| annot.level | W / ? / D / D | W / ? / D / D | `flow_level_tests` (HR+); declares hr |
| annot.regulatory, annot.hazard, structure.draw | — | W / W / W / W | page only, polarity-free |
| tp.generate, tp.link, tp.filter, tp.network | P / P / P / P | P / P / P / P | `flow_bentazone_tests` (four combinations), `flow_lr_tests::tp_link_reads_a_unit_resolution_table`, `e2e_flow_neg_lr`; both sides read the polarity and unit mass the plan leaves unwritten from the indexes (`mzlab_feat::nature`); their ports take `hr` and `lr` (NL-6) |
| tp.position | P / P / R / R | P / P / R / R | `flow_bentazone_tests` (HR), `every_table_step…` and `e2e_flow_neg_lr` (refused on LR files); its port takes `hr` only: the fragments are matched at 5 ppm (`interp::sito`), so LR data would find no site in silence (NL-6) |
| tp.review | W / ? / W / ? | W / ? / W / ? | `flow_review_tests` (HR+) |
| tp.dig_lr | R / R / P / P | — / — / P / P | `flow_lr_tests`, `crates/mzlab-finder-lr-wasm/tests/lr_negativo.rs`, `e2e_flow_dig_lr` (LR+), `e2e_flow_neg_lr` (LR±); refused by the check on HR files (facets of the files, NL-6) |
| structure.render, structure.reactivity | W / W / W / W | — | `flow_render_tests`; reactivity R in `every_table_step…`; polarity-free |
| spectra.centroid | W / W / P / W | ? / ? / ? / ? | `flow_centroid_tests` (unit grid); profile in the browser: P3 |
| spectra.ms2 | W / ? / W / ? | W / ? / W / ? | `flow_ms2_tests` (HR+); IONMODE from the scans (`flow_ms2.rs`) |
| spectra.msn_graph, msn.library_export, msn.table_export, msn.figure | W / W / ? / ? | W / ? / ? / ? | `flow_msngraph_tests`, `msn_figura_tests` (negative polarity in the figure), `msngrafo/tests_formule.rs::a_negative_infusion_is_read_the_same_way` |
| meta.compare, spectra.lookup | W / ? / W / ? | — | `flow_compare_tests`, `flow_lookup_tests` (HR+) |
| table.match | W / W / W / W | W / W / W / W | `flow_tablematch_tests`, `flow_match_web_tests` (HR+), `flow_neglr_tests::a_negative_table_is_searched_as_negative_by_table_match` (same Rust in `flow run` and in the table `.wasm` entry) |
| chrom.uv_peaks | W / W / W / W | — | `flow_uv_tests`; UV, no polarity |
| quant.calibrate | W / W / W / W | — | `flow_quant_tests`; polarity-free on the table |
| qc.mass_accuracy | R / R / B-11 / B-11 | ? / ? / ? / ? | `every_table_step…` (runs on HR; refused on LR by the check of its port, `err.flow.facet`, the same Rust in the browser) |
| screen.suspects | ? | ? | not in the registry yet |
| Tabui page (`TP_Mine/`) | P / P / P / P | P / P / P / P | the Flow tp.* steps above are the engine of Tabui; Tabui's own LR- page: `lr_negativo.rs` |

## 4. Gaps ranked by user impact

1. **NL-8 — samples matched to the files of the table by name (closed, both sides).** Every step that holds a table puts the samples document in the order of the table's files first: `mzlab_core::campioni::order_like` on the names the `.mzfeat` records (meta `inputs`), matched with or without folders and file extensions; a table file with no sample, or a sample naming no file of the table, is `error.badCampioni` with the name in `file`. Callers: `core::load` and `tp_table_cond_json` of the table `.wasm` (export.table_csv, export.mztab, table.filter, table.match and the views), `TpRoles::for_table` (tp.link native and browser), tp.network, tp.dig_lr (the names of its indexes), the proofs of the feat `.wasm`; table.stats, quant.calibrate and qc.mass_accuracy already matched by name.
   Evidence: `gap_the_samples_are_matched_to_the_files_of_the_table_by_name` (reversed samples file, same CSV), `a_sample_naming_a_file_the_table_lacks_is_refused_with_its_name`, `e2e_flow_neg_lr` with the series in injection order (T00, T2p5, T5, T10p25 against the OPFS name order).
2. **NL-7 — closed: the browser reads the polarity and the unit mass of the files.** The rule of `flow run` lives in `crates/mzlab-feat/src/nature.rs` (`of_indexes`, `open_polarity`, `estimates`); the browser runner asks it of the OPFS indexes through the proofs `.wasm` (`series_estimates` for tp.generate, `link_evidence` for tp.link, `dig_polarity` for tp.dig_lr). What the plan writes wins. Evidence: `python3 tests_e2e/e2e_flow_neg_lr.py` (unwritten) and `NEG_LR_WRITTEN=1`, green in HR+, HR-, LR+, LR-; `mzlab_feat::nature` tests.
3. **NL-2 — closed: formula.ms1 takes the ion of the files.** An `ion` the plan does not write is `[M-H]-` or `[M+H]+` by the polarity of the MS1 scans of each file (`crates/mzlab-feat/src/flow_formula_ms1.rs`, `ion_of_file`), in `flow run` and in the browser (`flow/lazy/flow-web-ident.js` leaves the unwritten ion out of the formula `.wasm`); prov records it as an estimate. A written ion of the other polarity wins with `warn.flow.formula.ion` on its rows and in prov. Evidence: `gap_formula_ms1_takes_the_ion_of_the_polarity_of_the_files`, `a_written_ion_of_the_other_polarity_wins_with_a_warning`, `e2e_flow_web_ident2.py` (negative case, browser = `flow run`).
4. **NL-3 — closed.** `table.match` writes the MGF of the rows with `pol::of_table` (the recipe, else the sign of the files), as the exports do; a negative series finds the negative library spectrum and not the positive one, in `flow run` and with `rows_mgf` of the table `.wasm`. `spectra.ms2` takes the polarity of each scan; `pol::recipe_polarity` remains only for callers that hold no file indexes.
5. **NL-1 — closed in `flow run`: features.estimate follows the files.** `flow_estimate.rs` measures each file in the polarity of its MS1 scans and with its own profile (`Recipe::for_index_estimated_in`: axis Da and the LR heights on a unit-resolution file), then applies only what the plan writes; the polarity and axis the file decided are prov estimates. The step has no browser executor (`flow-live-jobs.js`): in «This browser» it stays with the computer. Evidence: `gap_the_estimate_follows_the_polarity_and_the_resolution_of_the_files`.
6. **NL-4 — a mixed-polarity series is refused (fixed, both).** `features.align` and `features.gapfill` stop with `err.flow.align.polarity` and the names of the positive and the negative files (`mzlab_feat::nature::one_kind_of`, called by `align_setup` of `flow_base.rs` and by `align_series_to` of the feat `.wasm`); the user splits the series or aligns each polarity on its own branch. Evidence: `gap_a_series_of_mixed_polarity_is_refused_or_warned`, `e2e_flow_neg_lr`.
7. **NL-5 — a mixed-resolution series is refused (fixed, both).** The same check stops with `err.flow.align.resolution` and the names of the unit and the high-resolution files. Evidence: `gap_a_series_of_mixed_resolution_is_refused_or_warned`, `e2e_flow_neg_lr`. The Da/ppm parameters themselves: LR-TOL.
8. **NL-6 — closed: the resolution facets match the steps and come from the files (both sides).** The ports of tp.link, tp.filter, tp.network and tp.review take `hr` and `lr` (tp.generate declares none); tp.dig_lr takes `lr`; tp.position keeps `hr` (a clear refusal rather than no site at LR); formula.ms1, formula.ms2, spectra.insilico, spectra.chimeric, screen.suspects and annot.* keep `hr`. A port's `resolution` is the set it takes: every resolution of the data must be in it (`check::facets`, `flow-check.js`); resolution and polarity reach every output made from the scans.
   A scan input whose plan writes no facets takes them from its file (`mzlab_feat::nature::facets`): `flow run` in `fill_inputs` (scan headers of the mzML or the `.mzidx`), the page from the OPFS indexes (`series_estimates` of the proofs `.wasm` asked for `?facets`, `F.learnFacets`); written facets win. Known gap: the page check of the desktop app has no OPFS indexes, so it checks without the facets of the files; the native `flow run` behind it fills them before running. The refusal names step, input, what it needs and what the files are (`want`, `have`).
   Evidence: `gap_a_unit_resolution_plan_that_says_so_passes_the_check` (written and from the files), `formula_ms1_on_unit_resolution_files_is_refused_by_the_check`, `every_table_step…` (tp.dig_lr refused on HR, tp.position on LR), `flow_check_cases.json` (Rust = JS), `e2e_flow_neg_lr` (page check = `flow run --dry-run`). rt.predict is polarity- and resolution-free and stays open.
9. **NL-10 — export.inclusion: default adduct [M+H]+ whatever the files; tp.link.tp cannot feed it.** It reads a JSON list `{formula, rt}`; wired from `lnk.tp` (`.mzfeat`) it fails with `error.badIndex` in all four combinations. With `adducts` written it is right (`the_inclusion_list_takes_a_negative_adduct`).
10. **NL-11 — closed.** `tol_da` of `spectra.match` and `table.match` defaults to 0 = follow the query (0.02 Da, or 0.5 Da with `precursor_da` 0.7 for a unit-resolution query); `qc.mass_accuracy` refuses unit-resolution files (its `raw` port takes `hr` only: `err.flow.facet`) rather than reporting ppm.
11. **NL-12 — closed (LR-TOL).** `table.filter` has `dup_da` beside `dup_ppm`, chosen by the axis of the table.
12. **NL-13 — closed.** The record of `features.detect` names the `lr` preset when the files are unit resolution (`Outcome.preset`, also from the cache and from the browser).

## 5. Not verified

- MS2-based steps in negative polarity and at LR with real MS2 (spectra.ms2, spectra.match, table.match, formula.ms2, annot.level, tp.review, spectra.msn_graph and the msn.* exports): the fixture is full scan only; the cells above are read from the code and from HR+ tests.
- The browser index built by Data (`index.mzidx` in the page) and the profile path (P3).
- `screen.suspects` (not in the registry).
- Real 3200 QTRAP files (negative or positive) and real HR negative files: only synthetic data were used in this public audit.
- The desktop app itself: `flow run` was used as the native side (the app calls the same `FileRunner`); no test drove the Tauri window.
