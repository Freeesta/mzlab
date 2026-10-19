# Ready prompts for the DEV chats W-1 and W-2

Part of `docs/studi/WORKFLOW_MODULARI.md` (§8). Each block below is pasted as the first message of a new DEV chat; it stands alone. W-2 starts only after W-1 is merged.

## W-1 · Step descriptors as Rust data

```text
Package W-1 of the modular-workflow study (docs/studi/WORKFLOW_MODULARI.md §4 and §8; contract docs/studi/workflow/contract.schema.json, rules docs/studi/workflow/COMUNICAZIONE.md). Branch claude/w1-descriptors from origin/main, one small PR, do not merge: send the Coordinator the PR and the 40-character SHA when CI is green.

Goal: each core step of the flagship chain describes itself with a step/1 descriptor written as Rust data next to the code that runs it, and exported to JSON. No result of any step changes.

Steps (id@1): import.raw, convert.mzml, index.mzidx, features.estimate, features.detect, features.align, features.gapfill, table.filter, features.ms2_link, export.table_csv. All have package "core".
Where the code is: crates/mzlab-feat-wasm/src/run.rs (recipe_for, detect, read_files, align), crates/mzlab-feat/src/recipe.rs (Recipe, fingerprint), crates/mzlab-core/src/lettori.rs (trait Lettore, apri), lettori_mzml.rs (scrivi), mzidx.rs (build_mzml_spill), crates/mzlab-core/src/campioni/ (campioni/2). Read docs/agenti/rust.md and docs/agenti/mzfeat.md first.

What to build:
1. A small module (suggested crates/mzlab-core/src/step/) with plain structs for step/1: id, version, title, doc, package, in/out ports (type, many, accepts, requires), params (type, unit, default, min, max, x-advanced), runs_in, deterministic, errors, algorithms[] (id, version, title, doc, references, applicability, params, status), default_algorithm, presets, converts, cost. Serialise with serde_json; no new dependency.
2. One descriptor per step above, in the crate that owns the step, with the parameters the code really reads today (names in English, units explicit, defaults equal to the current Recipe values). features.detect: algorithm masstrace (available); centwave and threshold (planned) as in docs/studi/workflow/step.features.detect.json.
3. A function registry() -> Vec<Step> and an example or test that writes them to JSON.
4. references: only papers whose DOI you resolved; citation_key = first author + year + first title word, lower case (e.g. tautenhahn2008highly).

Tests (external truth = the schema, not our code):
- every exported descriptor validates against contract.schema.json (a pytest with jsonschema; CI does not install it today, so add it to the test-only Python requirements where pytest is declared, never to the site);
- parameter defaults equal the values the code uses today (a Rust test that builds the Recipe from the descriptor defaults and compares Recipe::fingerprint with today's);
- step outputs byte for byte unchanged: the existing golden tests (tests/golden/feat/) stay green without regeneration.

Acceptance: the four required jobs of test.yml green; tools/controlla.py exit 0; no .wasm ceiling raised (engine 1703936 B, finder 512000, sciex 327680, chem 569344): descriptors are not compiled into the site .wasm unless their size is measured and fits; no user text outside lang/ (descriptor titles are English data for the future form; do not show them in the UI yet); docs: one line per new function in docs/agenti/rust.md (split the file if it would exceed 25 KB).
Do NOT: change any algorithm, rename existing functions, add a UI, write new calculation in Python or JS, copy code from GPL projects (patRoon).
```

## W-2 · Canonical form, `prov/1` and the `mzflow/2` checker

```text
Package W-2 of the modular-workflow study (docs/studi/WORKFLOW_MODULARI.md §4 rules 1-5, §6, §7 test 3; contract docs/studi/workflow/contract.schema.json, rules docs/studi/workflow/COMUNICAZIONE.md §2-§3). Starts after W-1 (step/1 descriptors and registry()) is merged. Branch claude/w2-prov from origin/main, one small PR, do not merge: send the Coordinator the PR and the 40-character SHA when CI is green.

Goal: in Rust (mzlab-core, next to the W-1 registry), read an mzflow/2 file, check it, resolve every instance to its canonical parameters, and write a prov/1 record of a run. The prototype crates/mzlab-cli/examples/flow.rs already does a reduced version (two steps, prov/1, identical hashes native and WASI): replace its hand-written parts with the library and keep the example as the thin caller.

What to build:
1. parse(mzflow/2 JSON) -> Plan, with the mzflow/1 keys (formato, programma, operazioni) still read.
2. check(plan, registry) -> Vec<Problem{instance, port, error_key, params}>: types and accepts, cardinality (many/one, per_item), facets (requires), cycles, unknown step or version, algorithm not available, step not runnable on this platform (err.flow.unavailable), parameters out of range. Error keys err.flow.* added to lang/it.js and lang/en.js only if a UI shows them; otherwise keys only.
3. canonical(instance, descriptor) -> resolved params: algorithm default, then preset, then the instance differences; floats written with the shortest round-trip form; keys sorted; params_sha256 = SHA-256 of that text (sha2 is already a dependency of mzlab-core).
4. plan order: topological sort (Kahn), ties by the order of steps in the file; outputs of per_item steps gathered by index.
5. prov/1 writer: entity per input and output (sha256, bytes, type), activity per instance (step, algorithm id@version, preset, params, params_sha256, used, generated, ms, heap_peak_bytes, cached, references as citation_key list), agent (name, version, commit, platform).

Tests (external truth):
- §7 test 3: a small synthetic mzflow/2 (tools/dati_sintetici.py files, never laboratory data) run twice natively and once in WASM (wasmtime or the existing WASI path of flow.rs) gives identical generated sha256, written in the test as constants;
- prov.json validates the prov1 definition of contract.schema.json; the four mzflow/2 blocks of docs/studi/workflow/PROVE_SU_CARTA.md parse, and the checker returns exactly the problems expected for each (written in the test);
- canonical form: two files that differ only in key order, float spelling (1e5 vs 100000) or an explicit default give the same params_sha256;
- order: the same plan with 1 and N workers produces the same prov/1 except ms and heap_peak_bytes.

Acceptance: the four required jobs of test.yml green; tools/controlla.py exit 0; no .wasm ceiling raised (engine 1703936 B, finder 512000, sciex 327680, chem 569344): if the checker goes into a site .wasm, measure the bytes and say so; cargo clippy clean on the new code; docs: one line per new public function in docs/agenti/rust.md, and docs/agenti/cli.md updated where flow.rs changes.
Do NOT: change any step result, add a UI or the generated form (that is W-4), add a dependency, read the clock or unseeded randomness inside a step, write new calculation in Python or JS.
```
