# mzLab for external agents (`mzlab-cli` from the shell)

For an agent running on the user's computer (Claude Code, Claude Cowork or others) and using mzLab without the page: it prepares an `mzflow/2` plan, checks it, runs it and reads the results with `mzlab-cli`. Full command rules: `cli.md`; plans and `prov/1`: `rust-flow.md`; `step/1` steps: `rust-step.md`. Choices and open proposals: `llm-choices.md`.

## Installation
- Today only from source: `cargo install --path crates/mzlab-cli --profile cli` (needs Rust; the `cli` profile is `opt-level = 3`, in `target/cli/`: `cargo build --profile cli -p mzlab-cli`, about a quarter faster than the `release` profile, which is size-optimised and stays for the `.wasm`; same output bytes), then `mzlab-cli` is in the PATH. The same CLI compiles for `wasm32-wasip1` and runs a plan under node with `tools/wasi_run.mjs`.

## Recommended sequence
1. **Steps and parameters**: `mzlab-cli steps --json [--id ID]` prints the `step/1` descriptors of the binary you will use, with `executable_native`; the same `step/1` are in `docs/schemi/steps-1.json`, `steps-attributio-1.json`, `steps-tabui-1.json`, `steps-cli-1.json` (one JSON line per step; `params` is a JSON Schema with `default`, `minimum`, `maximum`, `enum`). A step without a native executor (list `BUCHI` in `cli.md`, "Coverage") makes the plan be rejected with `err.flow.step.native`.
2. **Plan**: an `mzflow/2` file with `inputs` (file names) and `steps` (`id` of the instance, `step` = `id@version`, `in` = `file:NAME` or `instance.port`, optional `algorithm`, `preset`, `params` with only the differences from the default).
3. **Check without writing**: `mzlab-cli flow check PLAN.json [--set ...] --json` checks the plan without the inputs (`flowcheck/1`, code 3 with the problems); `mzlab-cli flow run PLAN.json --in FOLDER --dry-run` checks plan, `--set` and inputs and prints the inputs found and the instances in order with the canonical parameters; code 3 = invalid plan (one `instance.port:key` line per problem on stderr), 4 = missing input; with `--json` on stdout `flowdry/1`.
4. **Run**: `mzlab-cli flow run PLAN.json --in FOLDER --out RUN [--set INSTANCE.NAME=VALUE ...] [--workers N]`; progress on stderr, one line per job (`--progress jsonl`: one JSON line per job).
5. **Outcome**: with `--json` on stdout `flowrun/1` (status, times and SHA-256 of the files of each instance) or `errore/1` (`error_key`, `params`, `hint`); exit code 0 all done, 1 some instance in error (the other outputs are written), 2 wrong usage, 3 invalid plan, 4 missing input or different SHA-256.
6. **Results**: in `RUN` one file per port (`<instance>.<ext>`), `plan.json` (canonical plan with the `--set`), `prov.json` (`prov/1`) and `corsa.json` (`corsa/1`: status, times and parameters of each instance); `mzlab-cli flow report RUN` rewrites the `corsa/1`.
7. **Fix**: rerun in the same `RUN` with other `--set`: the cache redoes only the steps that change (`cached` in `prov/1`); `--fresh` redoes everything.

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
The recipe is tested only on the high-resolution TiO2 series: on the flufenacet demo (default HR recipe) `rt_warp=piecewise` and `rt_gap=0.3` halve the rows (families 11 195 -> 3 537, ions 13 910 -> 4 135) but two of the three documented products move away and the view goes from 1 to 0 rows: do not apply it to another series without testing it with `explain-missing`.
In family mode an expected ion can turn out "absorbed": it is a member of a family row whose m/z or RT is outside the window, and `explain-missing` gives it as stage `absorbed`, with `absorbed_row`, `absorbed_mz` and `absorbed_rt` (the family row and its representative); with `ion.mode=ion` it has its own row (so T08 and T23 of the TiO2 series, and T11 with `rt_gap=0.3`). `link_rt` and `min_dependent` do not change this effect.

## Rules for the agent
- The CLI uses no network and reads and writes only in `--in` and `--out`; do not delete user files: the run folder is the only place to write.
- At unit resolution an m/z is a candidate, not an identification; the identification steps require `hr` data.
- Every run is reproducible: same plan and same inputs give the same bytes with 1 or N `--workers`.
