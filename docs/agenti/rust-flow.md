# Workflow plans `mzflow/2` and `prov/1` (`mzlab_core::flow`)

Reads a workflow file, checks the whole plan against the `step/1` descriptors before anything starts, resolves every instance to its canonical parameters, runs the plan in a fixed order and writes the `prov/1` record. Pure Rust, no I/O and no clock: the caller owns the files and the time (`mzlab_cli::flow::FileRunner`). Contract `docs/studi/workflow/contract.schema.json`, study `docs/studi/WORKFLOW_MODULARI.md`. Nothing here changes a step result or shows in the UI; the `err.flow.*` keys are not in `lang/` until a screen shows them.

## Functions
- `flow::parse(bytes) -> Plan` (`parse.rs`): `mzflow/2`; a file with no `format` is read as `mzflow/2`; the keys of `mzflow/1` (`formato`, `programma`, `operazioni`) are still read (`Plan::operations` is kept untouched); anything else is `err.flow.format`.
- `flow::check(plan, registry, platform) -> Vec<Problem>` (`check.rs`): every problem, in file order, cycles last. `Problem{instance, port, error_key, params}`; keys: `err.flow.step.unknown|version`, `instance.duplicate`, `unavailable` (no module for the platform: `Platform::Native|Wasm`), `algorithm.unknown|unavailable`, `preset.unknown`, `param.unknown|type|range`, `port.unknown|missing`, `input.unknown`, `ref.unknown`, `type`, `cardinality`, `facet`, `cycle`.
- `flow::conversion_path(registry, from, to)`: the shortest chain of registered conversion steps (`Step::converts`); the check accepts a wire that has one.
- `flow::canonical(instance, step) -> Resolved` (`canon.rs`): algorithm default, then preset, then the differences written in the file; sorted keys, shortest numbers (`Resolved::text()`); `params_sha256()` is the SHA-256 of that text without the `noresult` parameters; `flow::cache_key(input hashes, step, algorithm, params_sha256)`.
- `flow::plan_order(plan)` and `flow::levels(plan)` (`order.rs`): Kahn's sort, ties in file order; `levels` groups what can run together; a cycle is `err.flow.cycle`.
- `flow::execute(plan, registry, runner, workers) -> Run` (`run.rs`): checks, then runs level by level; the instances of a level and the items of a `per_item` step go to `workers` threads (native only), outputs gathered by index, so the record does not depend on `workers`. A failed instance gets an `error` in its activity and only its descendants are left out (`Run::skipped`). `Runner::run(&Job) -> Outcome` is the caller's.
- `flow::prov_json(agent, plan_sha256, run)` (`prov.rs`): the `prov/1` text; entities are `file:NAME`, `INSTANCE.PORT` or `INSTANCE.PORT[i]` (a `many` port); one activity per instance with `references` (citation keys of the step and of the algorithm that ran).
- `mzlab_cli::flow::{steps, table_step, FileRunner, run_plan}` (`crates/mzlab-cli/src/flow.rs`): the registry plus `features.table@1` (index, detect, align and CSV of `mzlab-cli features` as one native step), the runner that reads files from the folder of the plan, and the whole run with its `prov/1`; `examples/flow.rs` is the thin caller.

## Tests
- `cargo test -p mzlab-feat --test flow`: the four blocks of `docs/studi/workflow/PROVE_SU_CARTA.md` give exactly the problems written in the test; types, cardinality, facets, ranges and cycles; canonical form (key order, `1e5` against `100000`, an explicit default, a `noresult` parameter); 1 and 4 workers give the same record but for `ms` and `heap_peak_bytes`; an error stops only the descendants.
- `cargo test -p mzlab-core --lib flow`: parse and order.
- `tests/test_flow_determinismo.py`: a synthetic plan on `tools/dati_sintetici.py` files run twice natively (1 and 2 workers) and once as `wasm32-wasip1` under node (`tools/wasi_run.mjs`) gives the SHA-256 constants written in the test; `prov.json` validates `$defs/prov1`.

## Traps
- `convert.mzml` reads vendor files, not mzML; a plan of mzML inputs goes straight to `features.table@1`.
- A conversion a wire needs is accepted by the check but not inserted into the plan: the plan must still name it.
- The heap peak of `FileRunner` is the one of the process: exact with one worker only.
- A fact the inputs do not declare (`facets`, `type`) is not checked by the plan; the step checks it when it starts.
