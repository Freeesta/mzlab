# Experimental design from the CLI (`sample design`)

`table.stats@1` and `table.drift@1` take a `design` input (`design/1`, `docs/schemi/design-1.json`) next to the `samples` input (`samples/1`). `mzlab-cli sample design` makes and checks both from the command line. Code: `crates/mzlab-cli/src/design_cmd.rs`, tests `design_cmd_tests.rs`; the engine is `crates/mzlab-core/src/design/` (`propose`, `validate`, `problems`, canonical JSON, digest). Samples: `samples.md`.

## Same bytes as the site and the app
- The three commands call `samples::route_all("design_propose" | "design_validate" | "design_canonical" | "design_digest", ...)`: the route the `.wasm` `rotte` (`samples(op, input)`, page path `api/samples/design_*`) and the app (`lr_samples`) serve. Parity tests: `design_cmd_tests.rs` (CLI against the route) and `crates/mzlab-desktop/tests/parity_routes.rs` (app against `.wasm`, `design_steps`).
- `DataType::schema()` (`mzlab-core/src/step/mod.rs`): the ports of type `samples` and `design` carry `"schema": "samples/1"` / `"design/1"` in `flow steps --json` (and in `steps-*-1.json`, the page's registry); `mzlab-cli schema design/1` prints it.

## Commands
- `sample design propose (--samples SAMPLES.json | FILE...) [--out DESIGN.json]`: `design_proposal/1` on stdout: the `design`, the automatic `choices` (conditions, reference, comparisons, batches: value and why, e.g. the reference rule that matched) and the `evidence` (files of each condition, replicates, `min_replicates`). `FILE...` are hashed like `sample table FILE...`. `--out` writes the bare `design/1` (route bytes + newline), the file a plan wires to its `design` input. The user decides: nothing is blocked.
- `sample design validate DESIGN.json [--samples SAMPLES.json]`: exit 0 `design_check/1` (`sha256`, and with `--samples` the route's `warnings`: `fewReplicates`, `batchNoQc`). Exit 3: one `error/1` (`err.design.invalid`) whose `problems[]` are `{instance:"design", port:<key path>, error_key:"err.design.problem.<code>", params:{key,...}}`. Key paths: `conditions[1].replicates`, `comparisons[0].b`, `reference`, `$` (not JSON).
  Codes: `missing`, `wrongType`, `badNumber`, `wrongSchema`, `badKind`, `duplicate`, `unknownCondition`; with `--samples` also the blocking `emptyComparison` (`comparisons`) and `refMissing` (`reference`). Not a verdict command: its exit 3 is an `error/1`. Texts: `err.design.*` in `mzlab/web/lang/en/errori-a-m.js`.
- `sample design canonical DESIGN.json [--out PATH]`: `design_canonical/1`: `design` (canonical bytes: keys in alphabetical order, factors by name, batches by SHA-256 then file), `bytes`, `sha256` (of those bytes without the newline; what `prov/1` records).
- Exit codes: 0; 2 usage; 3 invalid design or samples file; 4 file missing or unreadable; 6 `--out` not writable.

## From files to statistics
```
mzlab-cli sample table FILE... > samples.json
mzlab-cli sample design propose --samples samples.json --out design.json
mzlab-cli sample design validate design.json --samples samples.json
# plan: inputs samples.json (samples), design.json (design), the .mzfeat table -> table.stats@1
mzlab-cli flow run plan.json --in DIR --out RUN
```
End-to-end test: `a_design_proposed_from_the_names_runs_a_stats_plan_without_an_edit`. A series needs two prefixes before the time in the names (`ref_t005min_1`, `trt_t005min_1`), see `samples.md`.
