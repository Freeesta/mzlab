# How modules talk: ports, compatibility, order, parallelism, algorithms

Part of `docs/studi/WORKFLOW_MODULARI.md` (§4b). This is the core of the contract: the future «Flow» section (a canvas of draggable, wired modules) is only a way of drawing what is here. Graphics are out of scope. Marks: **V** verified in the code or measured, **I** hypothesis. The schema is `contract.schema.json` (fields in backquotes).

## 1. What travels on a wire
- **A wire carries a reference, never bytes.** The data lives in the store (OPFS in the browser, a folder in the desktop app) and the wire carries its **descriptor**: `type`, `sha256`, `bytes`, format version and a few **facets** read from the data (resolution `hr`/`lr`, polarity, MS levels present, number of samples). Reason: an `.mzidx` or an mzML weighs megabytes to gigabytes; mzML and `.mzidx` already live in OPFS and modules read them in slices (`ReadAt`, `Index<R>`; V).
- **Data are immutable and content-addressed**: same SHA-256 = same data. A step never rewrites its input (correction 7 of `PROVE_SU_CARTA.md`). Two wires carrying the same data do not copy it.
- **Format version in the descriptor**: data of a format newer than the program open read-only with `error.newerFormat` (rule already used for `.mzidx`, `.mzfeat`, `.mzfeatf`: `docs/agenti/formato-nuovo.md`, V).

## 2. Compatibility rule (when an output can be wired to an input)
A wire from `A.out` to `B.in` is valid when all of these hold:
1. **Type**: `out.type` = `in.type`, or `out.type` is in `in.accepts` (e.g. a step that reads both `mzidx` and `table`).
2. **Cardinality**: `many → many` yes (same order); `one → many` yes (a list of one); several wires into a `many` port are **gathered** in the order of `inputs` in the file, never in the order the user drew them (the result must not depend on the drawing); `many → one` only if the step has `per_item: true`, i.e. it runs once per element (§3).
3. **Facets**: `in.requires` (e.g. `{"resolution": ["hr"]}` for `tp.link` and `annot.*`: principle 1, in the LR world no TP and no formula as truth; `{"ms_levels": [2]}` for `features.ms2_link`). When the facet is already known (after import) the check is immediate; otherwise the step checks again when it starts and answers with an error key.
4. **Acyclic graph**: no wire closes a cycle.
Each violation is an entry `{instance, port, error_key, params}` with keys such as `err.flow.type`, `err.flow.cardinality`, `err.flow.facet`, `err.flow.cycle`: the canvas colours the wire and shows the text from the catalogue, never a hand-written text.

**A missing step is proposed, not inserted silently.** Conversion steps declare `converts: {from, to}`. When the user wires `raw` to an `mzidx` port, the program looks for the shortest path in the graph of registered conversions (breadth-first; ties go to the lowest `cost`, then registry order) and proposes «Insert: Build index» with one click.
Real example: `raw → mzidx` exists directly (`upload::indicizza_lettore`) and through mzML (`lettori_mzml::scrivi` + `build_mzml_spill`), and both give the same features (V, §6.1 of the main file): the direct path is shorter and wins. ONLY registered conversions enter the graph: the second mzML writer (`raw::to_mzml`, different areas downstream, V) does not.

## 3. Execution order and parallelism (derived, never written by the user)
- **Order**: topological sort of the graph (Kahn's algorithm); ties follow the order of `steps` in the file. The same file always gives the same plan.
- **Parallel branches**: an instance starts when all its ancestors are done; two instances with no mutual dependency run together (e.g. the two `tp.generate` branches of `PROVE_SU_CARTA.md` D).
- **Parallel per file**: a `per_item: true` step on a `many` port runs once per element, in parallel; outputs are gathered by INDEX, not by arrival. Detection is already per file (`run::detect` per file, then `read_files` and `align`: V in `mzlab-cli`).
- **Where**: in the browser one worker per active instance, loading the step's `.wasm` module (`runs_in.wasm`; `rust-worker.js`, `feat-worker.js`, `tpmine-worker.js` exist: V); in the desktop app native threads (`rayon` is already a dependency of `mzlab-desktop`: V).
- **Memory is the only shared resource**: no write conflicts (immutable data), so the only limit is memory. The planner admits a step only if the sum of `memory_hint_bytes` of the active steps fits the device budget (the memory contract of `mzlab-core::budget`, `docs/agenti/memoria.md`: V that it exists; use in the planner: I). The hint comes from `heap_peak_bytes` of earlier runs in `prov/1`.
- **Determinism with parallelism**: a step reads neither the clock nor unseeded randomness; the result does not depend on arrival order. Test: the same plan with 1 worker and with N workers gives the same `generated.sha256` (to add to test 3 of §7).
- **Errors and cancel**: a failing step stops only its descendants; sibling branches finish and stay cached. Cancelling a step stops it and its descendants.
- **Cache like `make`**: key = (input SHA-256, `id@version`, algorithm `id@version`, `params_sha256`). Changing a parameter of X invalidates X and its descendants, nothing else; a repeated run without changes recomputes nothing and `prov/1` writes `cached: true`.

## 4. Algorithms, presets and comparison
- **Algorithms** (`algorithms[]`, `default_algorithm`): the step (e.g. `features.detect`) is the «slot» of the canvas; **alternatives** are algorithms inside the same step, with the same ports (same names and types): they swap without rewiring. Each entry declares `id`, `version`, `title`, `doc`, `references` (`doi`, `citation_key`, `note` of the paper(s) it is written from: never copied code, §9 of the main file; the same keys appear in the help, on the canvas and in the `references` of the `prov/1` activity).
  It also declares `applicability`, its own `params` and `status` (`available` or `planned`, greyed out on the canvas).
  The `params` of the step are those common to every algorithm. Real examples in `step.features.detect.json` (`masstrace` available; `centwave`, `threshold` planned) and `step.spectra.match.json` (`cosine`, `modified_cosine`, `spectral_entropy`, all in `mzlab-lib/src/scores.rs`, V).
- **Applicability and suggestion**: `applicability` uses the facets of the input (`resolution`, `polarity`, `ms_levels`, `spectrum_mode`) plus what the method needs (`peak_fwhm_s`, `min_files`, `needs`: `internal_standards`, `ms2`, `blank`, `reference_sample`, `library`) and a `note`. The canvas lists the matching algorithms first; the others stay selectable with the reason (e.g. «needs internal standards»); the plan check rejects them only when a mandatory input is missing.
- **Default**: `default_algorithm` is the best known for the typical case and is justified by bench numbers (`tools/bench/`, test 1 of §7), not by opinion.
- **Presets** (`presets`, `default_preset`): named parameter sets, each for one `algorithm` (absent = the default). A preset may have `when` on the input facets (e.g. `hr`, `lr`): «Auto» takes the first that matches. It already exists in the code: `run::recipe_for` picks the high-resolution recipe or the unit-resolution profile from the first file, and the CLI has `--preset auto|pos|neg` (V).
- **In the file**: an instance writes `step`, optionally `algorithm` and `preset`, and in `params` only the differences. The canonical form (rule 1 of §4 of the main file) resolves algorithm + preset + differences + estimates into complete values; `prov/1` writes the algorithm (`id@version`), the preset and the resolved values; the cache key includes the algorithm.
- **Swapping algorithm**: common parameters stay; algorithm-specific ones take the preset of the new algorithm; the canvas shows the difference before confirming.
- **Comparison** (`meta.compare@1`, `step.meta.compare.json`): a meta-step that runs the step named in `params.step` with 2 or more `variants` (`algorithm`, `preset`, `params`, `label`) on the same inputs. Its input ports are those of the wrapped step (`ports_from: wrapped`); its outputs are one per variant (`cmp.A.peaks`, `cmp.B.peaks`) and one `comparison`. Identical variants are an error (`err.flow.compare.sameVariant`). Variants run in parallel under the rules of §3 (memory adds up).
- **Comparison metrics**, per output type: `peaks`/`table`: matched within `mz_tolerance_ppm` (or `mz_tolerance_da` at unit resolution) and `rt_tolerance_min`, only in A, only in B, median Δm/z in ppm and ΔRT, median |log2| of the area ratio, Spearman of matched areas; `rtmap`: RT offset per anchor; `annot` (MS2 scores, identities): rank correlation (Kendall), top-1 agreement, top-k overlap, changed identification levels. Each metric carries the list of rows behind it, so the canvas shows the differences, not just numbers.
- **Uses**: choosing the default on evidence; validating a new implementation against the old one (same inputs, same starting hash); methodology and teaching (students see what a method changes). In the LR world the comparison shows different peaks, never TPs or calibration lines (principle 1).

## 5. Canvas ↔ engine dialogue (messages)
Only small JSON; data stay in the store.
| Message | From → to | Content |
|---|---|---|
| `plan.check` | canvas → checker (Rust) | the `mzflow/2`; answer: problems `{instance, port, error_key, params}` and conversion proposals |
| `plan.run` | canvas → planner | the checked plan; the planner sends ready instances to workers |
| `step.started` / `step.progress` | worker → canvas | `{instance, done, total}`, the same shape as today's single progress bar (`progresso.js`, `PROGRESSO`: V) |
| `step.done` | worker → canvas | `{instance, generated: [descriptors], ms, heap_peak_bytes, cached}` |
| `step.failed` | worker → canvas | `{instance, error, error_key, params}` (API shape: V) |
| `plan.done` | planner → canvas | the complete `prov/1` |
Plan checking and canonical form live in Rust (rule «everything in Rust»); the browser planner only dispatches, as `feat-worker.js` already does with the Pipeline steps (V).

## 6. Example: two branches, a preset, a proposed conversion
```json
{"format":"mzflow/2","program":{"name":"mzLab","version":"x"},
 "inputs":[{"name":"t0.raw","sha256":"…","type":"raw"},{"name":"t60.raw","sha256":"…","type":"raw"},{"name":"samples.json","sha256":"…","type":"samples"}],
 "steps":[
  {"id":"idx","step":"index.mzidx@1","in":{"raw":["file:t0.raw","file:t60.raw"]},"label":"proposed: Build index"},
  {"id":"det","step":"features.detect@1","algorithm":"masstrace","preset":"hr","in":{"index":"idx.index"},"params":{"min_height":100000}},
  {"id":"aln","step":"features.align@1","in":{"peaks":"det.peaks","index":"idx.index","samples":"file:samples.json"}},
  {"id":"ms2","step":"features.ms2_link@1","in":{"table":"aln.table","index":"idx.index"}},
  {"id":"tp","step":"tp.link@1","in":{"table":"aln.table","spectra":"ms2.spectra"}},
  {"id":"csv","step":"export.table_csv@1","in":{"table":"aln.table"}}]}
```
Derived plan: `idx` (per file, 2 in parallel) → `det` (per file, 2 in parallel) → `aln` → then `ms2` and `csv` together → `tp`.

## 7. Still to test
- Plan check and conversion search in Rust: module size in bytes (I) and time on plans of 50 instances (I).
- The planner with N workers: real memory peak in the browser and the «1 worker = N workers» test (I).
- Gathering by index in today's parallel Pipeline detection: check that output order does not depend on arrival (I).
