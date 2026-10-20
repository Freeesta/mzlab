# Step descriptors `step/1` (`mzlab_core::step`, `mzlab_feat::steps`)

Each core step of the flagship chain describes itself as Rust data next to the code that runs it; the contract is `docs/studi/workflow/contract.schema.json`, the study `docs/studi/WORKFLOW_MODULARI.md`. A descriptor changes no result and is not shown in the UI yet (titles are English data for the future generated form). Ten steps, all `package: core`, `id@1`: `import.raw`, `convert.mzml`, `index.mzidx` (owned by `mzlab-core`), `features.estimate`, `features.detect`, `features.align`, `features.gapfill`, `table.filter`, `features.ms2_link`, `export.table_csv` (owned by `mzlab-feat`).

## Functions
- `step::Step`, `Port`, `Param`, `Algorithm`, `Preset`, `Reference`, `Applicability`, `Facets`, `RunsIn`, `DataType`: plain structs of `step/1` (`mzlab-core/src/step/mod.rs`); no serde, the JSON is written with `json::Out`.
- `Step::core(id, title, doc)`: a `core` step at version 1 with empty ports and parameters; descriptors fill the rest with struct update syntax.
- `Port::new(name, type, title)` with `.many()`, `.accepts(types)`, `.requires(facets)`: an input or output port.
- `Param::number|integer|boolean|choice|choice_num|pair(name, title, ..)` with `.default_num|default_bool|default_str|default_pair`, `.min`, `.max`, `.unit`, `.doc`, `.advanced`, `.estimate`, `.noresult`: a property of a `paramSchema`; no default means the step estimates the value or the rule is off.
- `Step::param(name)` and `Step::algorithm_param(algorithm, name)`: look a parameter up; `Param::default_f64()` reads its default as a number; `step::find(registry, id)` finds a descriptor.
- `step::json::step_json(&Step)` and `registry_json(&[Step])`: the `step/1` JSON, keys in the order of `step.features.detect.json`; the registry has one step per line.
- `step::core_steps()` (`step/core_steps.rs`): the descriptors of `import.raw`, `convert.mzml`, `index.mzidx`.
- `mzlab_feat::steps::registry()` (`steps/mod.rs`, with `detect.rs` and `table.rs`): the ten descriptors in the order of the chain; the registry lives in `mzlab-feat` because `mzlab-core` cannot see the feature steps.

## Tests
- `cargo test -p mzlab-feat --test steps`: the recipe rebuilt from the descriptor defaults (starting from a perturbed recipe) has the same `file_hash`, `table_hash` and `file_text` as `Recipe::default()`; every default lies inside its limits; the export equals `docs/schemi/steps-1.json`. `MZLAB_UPDATE_STEPS=1` rewrites that file after a descriptor changes.
- `cargo test -p mzlab-feat --lib steps` (presets `hr` and `lr` equal the constants of `Recipe::default` and `Recipe::lr`) and `cargo test -p mzlab-core --lib step` (defaults of `convert.mzml` and `index.mzidx` equal `Opzioni::default()` and `DEFAULT_BUDGET`).
- `tests/test_step_descrittori.py`: every descriptor of `docs/schemi/steps-1.json` validates against `$defs/step` of the contract (`jsonschema`, a test-only dependency in `pyproject.toml`), agrees with the study sketch, and every error key is a string of the code.

## Traps
- The parameter names are those of the engine structs (`rt_gap`, `rt_tol`, `tol_ppm`), not the keys of the Pipeline panel (`apply_params` in `mzlab-feat-wasm/src/run.rs`, where `rt_tol` sets two fields); the mapping belongs to the plan reader.
- `ppm` fields read a negative value as Da at unit resolution (rule 4 of the study forbids it in the contract); the descriptors say so and the `lr` preset of `features.detect` carries only the detection part of `Recipe::lr`.
- `features.align`, `features.gapfill` and `features.ms2_link` run today inside one call each of `series::align_and_fill` and `pipeline::analyse`; the descriptors are the contract of the steps the plan will call separately.
- `features.detect` polarity default is 1 (`Recipe::default`); `recipe_on` reads an absent key as all MS1 scans.
