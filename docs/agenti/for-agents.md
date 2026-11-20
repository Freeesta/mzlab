# mzLab for external agents (`mzlab-cli` from the shell)

For an agent running on the user's computer (Claude Code, Claude Cowork or others) and using mzLab without the page: it prepares an `mzflow/2` plan, checks it, runs it and reads the results with `mzlab-cli`. Full command rules: `cli.md`; plans and `prov/1`: `rust-flow.md`; `step/1` steps: `rust-step.md`. Choices and open proposals: `llm-choices.md`.

## Installation
- Linux: every release carries static builds of `mzlab-cli` (x86_64 and aarch64) with a `SHA256SUMS` file: fetch and verify with the command in `cli-agenti.md`.
- Any other system, or a build with your own commit: from source, `cargo install --path crates/mzlab-cli --profile cli` (needs Rust; the `cli` profile is `opt-level = 3`, in `target/cli/`: `cargo build --profile cli -p mzlab-cli`, about a quarter faster than the `release` profile, which is size-optimised and stays for the `.wasm`; same output bytes), then `mzlab-cli` is in the PATH. The same CLI compiles for `wasm32-wasip1` and runs a plan under node with `tools/wasi_run.mjs`.

## Recommended sequence
0. **Learn the CLI**: `mzlab-cli commands --json` (every command, flag, output schema, exit code), `mzlab-cli schema NAME` (any JSON Schema), `mzlab-cli flow steps --compact` (the steps in a few KB).
1. **Steps and parameters**: `mzlab-cli flow steps --json [--id ID]` prints the `step/1` descriptors of the binary you will use, with `executable_native`; the same `step/1` are in `docs/schemi/steps-1.json`, `steps-attributio-1.json`, `steps-tabui-1.json`, `steps-cli-1.json` (one JSON line per step; `params` is a JSON Schema with `default`, `minimum`, `maximum`, `enum`). A step without a native executor (list `BUCHI` in `cli.md`, "Coverage") makes the plan be rejected with `err.flow.step.native`.
2. **Plan**: an `mzflow/2` file with `inputs` (file names) and `flow steps` (`id` of the instance, `step` = `id@version`, `in` = `file:NAME` or `instance.port`, optional `algorithm`, `preset`, `params` with only the differences from the default).
3. **Check without writing**: `mzlab-cli flow check PLAN.json [--set ...] --json` checks the plan without the inputs (`flowcheck/1`, code 3 with the problems); `mzlab-cli flow run PLAN.json --in FOLDER --dry-run` checks plan, `--set` and inputs and prints the inputs found and the instances in order with the canonical parameters; code 3 = invalid plan (one `instance.port:key` line per problem on stderr), 4 = missing input; with `--json` on stdout `flowdry/1`.
4. **Run**: `mzlab-cli flow run PLAN.json --in FOLDER --out RUN [--set INSTANCE.NAME=VALUE ...] [--workers N]`; progress on stderr, one line per job (`--progress jsonl`: one JSON line per job).
5. **Outcome**: with `--json` on stdout `flowrun/1` (status, times and SHA-256 of the files of each instance) or `error/1` (`error_key`, `params`, `hint`); exit code 0 all done, 1 some instance in error (the other outputs are written), 2 wrong usage, 3 invalid plan, 4 missing input or different SHA-256.
6. **Results**: in `RUN` one file per port (`<instance>.<ext>`), `plan.json` (canonical plan with the `--set`), `prov.json` (`prov/1`) and `report.json` (`run-report/1`: status, times and parameters of each instance); `mzlab-cli flow report RUN` rewrites the `run-report/1`.
7. **Fix**: rerun in the same `RUN` with other `--set`: the cache redoes only the steps that change (`cached` in `prov/1`); `--fresh` redoes everything.

## Five chains (tested)
Each chain is a test on synthetic data in `crates/mzlab-cli/src/workflow_tests.rs`: every call exits 0 and the call count below is asserted. Add `--cache DIR` (or `MZLAB_CACHE`) to the file commands of a chain that reads the same files twice; `file index FILE...` builds the indexes first (`cli-agenti.md`).
1. Unknown folder to a CSV (7 calls): `file list DIR --json`; `llm inspect DIR > inspection.json`; `llm params --inspection inspection.json`; `flow steps --compact`; `sample table FILE... > samples.json`; `flow check PLAN.json --json`; `flow run PLAN.json --in DIR --out RUN --json` (the plan ends with `export.table_csv@1`).
2. Reproducible run (5): `sample table FILE... > roles.json`; `flow check PLAN.json`; `flow run PLAN.json --in DIR --out RUN`; `flow record RUN --roles roles.json --out RUN/run.mzrun.json`; `flow verify RUN/run.mzrun.json --json`.
3. Trace to peaks (3): `file channels FILE` (take the `id`); `trace chrom FILE --id ID --format csv > trace.csv`; `peak auto-lr trace.csv`.
4. Statistics (5): `sample table FILE... > samples.json`; `sample design propose --samples samples.json --out design.json`; `sample design validate design.json --samples samples.json`; `flow check PLAN.json`; `flow run PLAN.json --in DIR --out RUN` (a `table.stats@1` plan with the `table`, `samples` and `design` inputs).
5. Vendor folder triage (5): `file list DIR --json`; `sample list FILE... --json`; `file convert FILE... --out OUT --dry-run --json`; `file convert FILE... --out OUT --json`; `file validate OUT/*.mzML --json`. For a `.wiff`, `file pair FILE --json` comes after the listing and `sample select FILE --sample LIST --json` before the conversion.

## Example (the basic chain on mzML files)
```json
{"format":"mzflow/2","inputs":[{"name":"s_t000.mzML","type":"raw"},{"name":"s_t010.mzML","type":"raw"},{"name":"s_t030.mzML","type":"raw"}],"steps":[
{"id":"idx","step":"index.mzidx@1","in":{"source":["file:s_t000.mzML","file:s_t010.mzML","file:s_t030.mzML"]}},
{"id":"det","step":"features.detect@1","in":{"index":["idx.index"]}},
{"id":"ali","step":"features.align@1","in":{"peaks":["det.peaks"]}},
{"id":"gap","step":"features.gapfill@1","in":{"table":"ali.table","index":["idx.index"]}}]}
```
It is the `BASE` plan of `crates/mzlab-cli/src/flow_run_tests.rs`. A parameter is changed without touching the file, for example `--set det.polarity=-1` or `--set det.min_height=5000`; `--set det.algorithm=centwave` chooses the wave model (parameters `ppm`, `peak_width_s`, `snr` only with that algorithm); a `planned` algorithm of the descriptor (today `threshold` of `features.detect`) gives `err.flow.algorithm.unavailable`; the name of a unique instance can be the step id.

Table by families and table by ions: two `features.align` on the same `det.peaks`, the second with `"params":{"mode":"ion"}` (or `--set ion.mode=ion`); the detect is shared. In family mode an ion absorbed into the parent's family (ISF, adduct) has no row and appears only among the members; in ion mode it has its own row with the areas per sample.
For the ion table of a series with RT drift between samples (TiO2, about 0.2 min) use `ion.mode=ion`, `ion.rt_warp=piecewise` and `ion.rt_gap=0.3`: on the 12-file series the rows in view go from 235 to 134 and the median rank from 31 to 20.5. It does not hold for the family table: with the same values it loses products (absorbed as members), there the default values stay.
The recipe is tested only on the high-resolution TiO2 series: on the flufenacet demo (default HR recipe) `rt_warp=piecewise` and `rt_gap=0.3` halve the rows (families 11 195 -> 3 537, ions 13 910 -> 4 135) but two of the three documented products move away and the view goes from 1 to 0 rows: do not apply it to another series without testing it with the expected-ion check.

## Learn the format, then check before running (tested: `flow_template_tests.rs`)
1. `mzlab-cli flow steps --compact` (ids, ports, parameter names), then `mzlab-cli flow steps --json --id features.detect` for the types, limits and defaults of one step.
2. `mzlab-cli flow steps --id features.detect@2 --template > plan.json`: a valid one-step plan; add the steps and links you need (`"in":{"index":["idx.index"]}`) and change only the parameters you want.
3. `mzlab-cli flow check plan.json --json`: exit 3 names every problem with its `path`, e.g. `det.params.min_hight` (unknown parameter), `det.params.min_height` (wrong type), `det.in.index` (missing port), `steps[1].param` (a key the format does not have). `flow run plan.json --in DIR --dry-run` repeats the check with the inputs and prints what WOULD run; it never writes.

## Several ions from a full-scan file, then a report
`trace xic` answers one m/z per call (the page's `api/xic`), or every m/z of a range in one pass with `--sweep LO:HI:STEP --format csv` (the same bytes as the list); loop it for a few ions, integrate each trace, and keep the calls as the record:
```sh
for mz in 364.4 194.2 152.2; do
  mzlab-cli trace xic sample.mzML --mz $mz --ppm 500 --rt 12,16.5 --format csv --out xic_$mz.csv   # rt_min,intensity
  mzlab-cli peak auto-lr xic_$mz.csv > peaks_$mz.json                                              # {"params","peaks":[rt, area, ...]}
done
```
At unit resolution give the window in Da (`--da 0.3`), in ppm for high resolution. There is no Flow step that extracts XICs yet, so a plan cannot do it: the plan examples above start from the index. After a plan run, `mzlab-cli flow report RUN` writes the `run-report/1` of the run.

## Rules for the agent
- The CLI uses no network and reads and writes only in `--in` and `--out`; do not delete user files: the run folder is the only place to write.
- At unit resolution an m/z is a candidate, not an identification; the identification steps require `hr` data.
- Every run is reproducible: same plan and same inputs give the same bytes with 1 or N `--workers`.
