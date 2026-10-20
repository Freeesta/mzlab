# Step descriptors `step/1` (`mzlab_core::step`, `mzlab_feat::steps`)

Each core step of the flagship chain describes itself as Rust data next to the code that runs it; the contract is `docs/studi/workflow/contract.schema.json`, the study `docs/studi/WORKFLOW_MODULARI.md`. A descriptor changes no result and is not shown in the UI yet (titles are English data for the future generated form). Ten steps, all `package: core`, `id@1`: `import.raw`, `convert.mzml`, `index.mzidx` (owned by `mzlab-core`), `features.estimate`, `features.detect`, `features.align`, `features.gapfill`, `table.filter`, `features.ms2_link`, `export.table_csv` (owned by `mzlab-feat`).

Nine more, `package: attributio`, all requiring `hr` (principle 1: no formula, match or level at unit resolution): `spectra.chimeric`, `formula.ms1`, `formula.ms2`, `spectra.match`, `spectra.insilico`, `rt.predict`, `annot.level`, `annot.regulatory`, `annot.hazard` (`steps/ident*.rs`). Their defaults are those of `ReviewSettings`, `AnnotationSettings`, `InSilicoOpts`, `SearchOpts` and `formula_id::Opts`; constants of the code appear only in the text. Planned: `spectra.chimeric` `library_deconvolution`. CANOPUS stays Level 4 as in `livelli.md`.

Seven more, `package: tabui`: `tp.generate` (`hypo::generate`, `ipotesi/1`; its defaults are read from `HypoParams::default()`), `tp.filter` (F1-F5 and the decoys of `interp::filtri`), `tp.link` (`interp::ms2link`, `modifica`; algorithms `pooled_ms2`, `isomer_traces`), `tp.position` (`interp::sito`; algorithms `msn_tree`, `cleavage_likelihood`)
`structure.render` (`interp::disegno`, the one form of the drawing) and `structure.draw` (Ketcher, page only: no `.wasm`, `deterministic: false`) and `structure.reactivity` (`mzlab-chem`, defaults read from `Opts::parse("")`; algorithms `gfn2_xtb`, `extended_huckel` as the engine fallback).
They run in the `.wasm` `hr`, `interp` and `chem` with their ceilings written as data.

## Functions
- `step::Step`, `Port`, `Param`, `Algorithm`, `Preset`, `Reference`, `Applicability`, `Facets`, `RunsIn`, `DataType`: plain structs of `step/1` (`mzlab-core/src/step/mod.rs`); no serde, the JSON is written with `json::Out`.
- `Step::core(id, title, doc)`: a `core` step at version 1 with empty ports and parameters; descriptors fill the rest with struct update syntax.
- `Port::new(name, type, title)` with `.many()`, `.accepts(types)`, `.requires(facets)`: an input or output port.
- `Param::number|integer|boolean|text|choice|choice_num|pair(name, title, ..)` with `.default_num|default_bool|default_str|default_pair`, `.min`, `.max`, `.unit`, `.doc`, `.advanced`, `.estimate`, `.noresult`: a property of a `paramSchema`; no default means the step estimates the value or the rule is off.
- `Step::param(name)` and `Step::algorithm_param(algorithm, name)`: look a parameter up; `Param::default_f64()` reads its default as a number; `step::find(registry, id)` finds a descriptor.
- `step::json::step_json(&Step)` and `registry_json(&[Step])`: the `step/1` JSON, keys in the order of `step.features.detect.json`; the registry has one step per line.
- `step::core_steps()` (`step/core_steps.rs`): the descriptors of `import.raw`, `convert.mzml`, `index.mzidx`.
- `mzlab_feat::steps::registry()` (`steps/mod.rs`, with `detect.rs` and `table.rs`): the ten descriptors in the order of the chain; the registry lives in `mzlab-feat` because `mzlab-core` cannot see the feature steps.

- `formula.ms1` is not `per_item` (the table reads every file) and its ports accept `report` (the CSV of `features.table`) and `mzml` besides `table` and `mzidx`, the types the native runner passes.
- `mzlab_feat::steps::attributio()` (`steps/ident.rs`, with `ident_spectra.rs`, `ident_formula.rs`, `ident_annot.rs`): the nine identification descriptors in the order of the chain, apart from `registry()` so that `steps-1.json` is unchanged; `Applicability` has `ms_levels` and `needs` (`internal_standards`, `ms2`, `blank`, `reference_sample`, `library`).
- `mzlab_feat::steps::records::records()` and `records_json()`: the fields of the shared `candidates` and `annot` records with the struct they are read from; export `docs/schemi/records-1.json`.

- `mzlab_feat::steps::tabui()` (`steps/tabui.rs`, `tabui_link.rs`): `tp.generate`, `tp.filter`, `tp.link`, `tp.position`, `structure.render`, `structure.draw` (`tabui_draw.rs`); `mzlab_chem::step::tabui()` (`mzlab-chem/src/step.rs`) adds `structure.reactivity` and is the whole family; `mzlab-feat` cannot see `mzlab-chem`.

## The `convert.mzml` file
- `lettori_mzml::scrivi_indicizzato(reader, sample, out, progress, Opzioni, &[ProcStep])` (`mzmlwrite::MzmlWriter::indicizzato`, `mzmlwrite/indice.rs`) is what `mzlab_cli::flow::FileRunner` runs for `convert.mzml`: `indexedmzML` (`mzML1.1.2_idx.xsd`), PSI-MS CV 4.2.6, byte `<offset>` of every spectrum and chromatogram, `indexListOffset`, SHA-1 `fileChecksum` (all bytes up to the opening tag).
  One `processingMethod` (with its own `software`, user parameters from `ProcStep::params`) after the conversion for each entry of `Header::steps`. Spectra and chromatograms are the bytes of `scrivi_con`; the same bytes on every run. `scrivi` and `scrivi_con` (site, desktop, golden tests) still write the plain file with CV 4.1.163 and one conversion method.
- Tests: `cargo test -p mzlab-core --lib mzmlwrite` (offsets, checksum, read back, steps), `--test lettori_mzml` (Agilent synthetic), `cargo test -p mzlab-lettori --features aperti --test aperti_numpress` (indexed file reopened by the reader of open mzML), `cargo test -p mzlab-cli --lib` (plan `convert.mzml` then `features.table`).
- Checks on real files, outside CI: `docs/studi/workflow/PROVE_W3.md`.

## Tests
- `cargo test -p mzlab-feat --test steps`: the recipe rebuilt from the descriptor defaults (starting from a perturbed recipe) has the same `file_hash`, `table_hash` and `file_text` as `Recipe::default()`; every default lies inside its limits; the export equals `docs/schemi/steps-1.json`. `MZLAB_UPDATE_STEPS=1` rewrites that file after a descriptor changes.
- `cargo test -p mzlab-feat --lib steps` (presets `hr` and `lr` equal the constants of `Recipe::default` and `Recipe::lr`) and `cargo test -p mzlab-core --lib step` (defaults of `convert.mzml` and `index.mzidx` equal `Opzioni::default()` and `DEFAULT_BUDGET`).
- `tests/test_step_descrittori.py`: every descriptor of `docs/schemi/steps-1.json` validates against `$defs/step` of the contract (`jsonschema`, a test-only dependency in `pyproject.toml`), agrees with the study sketch, and every error key is a string of the code.

- `cargo test -p mzlab-feat --test steps_attributio`: order and package of the nine steps, defaults inside their limits, export equal to `docs/schemi/steps-attributio-1.json` and `records-1.json` (same `MZLAB_UPDATE_STEPS=1`). `--features attributio,untargeted --test steps_attributio_parity` compares every default with the engine structs.
- `tests/test_step_identificazione.py`: the nine descriptors validate against `$defs/step`, DOI and `citation_key` have the shape of the study, the only `planned` algorithm is `library_deconvolution`, every error key is a string of the code.

- `cargo test -p mzlab-chem --test steps_tabui`: order and package of the seven steps, defaults inside their limits, export equal to `docs/schemi/steps-tabui-1.json` (same `MZLAB_UPDATE_STEPS=1`). `cargo test -p mzlab-feat --features interp --test steps_tabui_parity` compares the defaults of `tp.generate`, `tp.filter`, `tp.link` and `tp.position` with the engine.
- `tests/test_step_tabui.py`: the seven descriptors validate against `$defs/step`, DOI and `citation_key` have the shape of the study and agree with the identification chain, every error key is a string of the code.

## Traps
- The parameter names are those of the engine structs (`rt_gap`, `rt_tol`, `tol_ppm`), not the keys of the Pipeline panel (`apply_params` in `mzlab-feat-wasm/src/run.rs`, where `rt_tol` sets two fields); the mapping belongs to the plan reader.
- `ppm` fields read a negative value as Da at unit resolution (rule 4 of the study forbids it in the contract); the descriptors say so and the `lr` preset of `features.detect` carries only the detection part of `Recipe::lr`.
- `features.align`, `features.gapfill` and `features.ms2_link` run today inside one call each of `series::align_and_fill` and `pipeline::analyse`; the descriptors are the contract of the steps the plan will call separately.
- `features.detect` polarity default is 1 (`Recipe::default`); `recipe_on` reads an absent key as all MS1 scans.
- `tp.generate` merges all its sources in one pass (the absent inputs switch a source off), so it has one algorithm; the alternatives of `meta.compare` are parameter variants (`max_steps`, `nominal`). The `citation_key` takes the first letters-only word of the title (`bannwarth2019gfn` for «GFN2-xTB…»).
- `spectra.ms2` (mzML → `spectra`) has its descriptor in `mzlab-cli` (`flow_ms2.rs`, export `docs/schemi/steps-cli-1.json`), like `features.table` and `meta.compare`; see `rust-flow.md`.
- `annot.level` runs natively in `mzlab-cli` (`flow_level.rs`) with the rule of `livelli.js` on the data of `livelli-1.json`, not with the Attributio cascade its parameters describe; see `rust-flow.md`.
- `meta.compare` has its descriptor in `mzlab-cli` (`flow_compare.rs`), not in the registry of `mzlab-feat`: the plan is expanded before the check, so the descriptor describes the expanded compare instance (`step`, `labels`); see `rust-flow.md`.
