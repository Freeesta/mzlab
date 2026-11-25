# mzLab for external agents (`mzlab-cli`): install `tools/install.sh` (prebuilt) or build `cargo build --profile cli -p mzlab-cli` (`target/cli/`), never `--release` (size-optimised, for the `.wasm`)

Before writing any script, ask the CLI: `mzlab-cli commands --find TASK WORDS --json` (`commands-find/1`: the top five commands and Flow steps, each with a one-line `example`, `see_also` and, where a loop of calls has a one-call equivalent, `bulk`). If you are about to call one command once per file, ion or trace, or to write Python for a table, a filter or a figure, look here first:

| task | one call |
|---|---|
| what a folder holds | `file list DIR --json`; per `.wiff` sample: `file inventory`, `sample list` |
| convert with sample names | `sample list FILE --json` (sample numbers and names), then `file convert FILE... --out DIR --sample LIST --json`: one `NAME-ID.mzML` per sample (NAME = the file name, ID = the sample number) |
| roles (blank, QC, standard, time) | `sample table FILE... > samples.json`; roles and names from a lab CSV: `sample import META.csv DIR --dry-run`, then without it; design: `sample design propose` |
| profile or centroid, ion counts, m/z grid, mass offset of a file | `raw info FILE... [--offset-ion MZ@RT]` (`raw-info/1`); index cache first: `raw cache FILE... --cache DIR` |
| is there a peak at this m/z and time, in each file (verdict, rule, ion counts, upper limit) | `raw xic FILE... --mz LIST --rt A,B` (`raw-xic/1`), `--set min_sn=,min_points=,alpha=` |
| which noise floor, minimum height, ppm or RT tolerance fit my files (before choosing a threshold) | `estimate FILE... --axis da\|ppm --emit-set PLAN.json` (`estimate/1`: median, range, evidence, the `--set` lines); limits: `flow defaults STEP` |
| one target list from the feature tables of several series (conditions, replicates, polarities) | `quant union` or Flow step `features.union` (`union/1`); then `quant targets` over every file |
| non-targeted feature table | `features FILE... --out table.csv`, or a plan `index.mzidx` → `features.detect` → `features.align` → `features.gapfill` |
| target list of SIM ions / SRM transitions, area and S/N in every file | Flow step `chrom.transitions` (parameter `transitions` = CSV `name;q1;q3;...`) |
| every peak of every channel of every file | `peak table FILE... --auto` (one call, all files); hand windows: `--windows` |
| XICs of many ions, then their peaks | `trace xic FILE --mz M1,M2,... --da 0.3 --out F.json` per file, then `peak table --traces A.json --traces B.json --auto` |
| blank threshold, replicates, QC CV; drop what a blank (carryover, contaminant, matrix blank) also shows | Flow step `table.filter` (`blank_ratio`, `blank_subtract`, ... `counts.blank` says how many it dropped); by ratio: `table compare`; one spectrum: `spectrum get --minus`; `features` says on stderr when file names look like blanks |
| correlated ions, isotopes, adducts of one compound | Flow step `features.group`; one ion: `ion pattern`, `ion match` |
| centroid profile spectra | Flow step `spectra.centroid` |
| products of a parent, unit resolution (kinetics) | Flow step `tp.dig_lr`: ONE run over the whole series (every file in `index`); other settings: rerun the same `--out` with `--set` (cached) |
| kinetic LR campaign: names, roles, targets, products per condition | the 9-call recipe and plan template of `recipes.md` (`docs/recipes/kinetic-lr.mzflow.json`) |
| hypotheses of the products | Flow step `tp.generate` |
| structure picture (SVG, PNG, CDXML, KET, molfile) | `mol draw SMILES --format svg --out a.svg`; pathway: Flow step `tp.network` |
| calibration curve, LOD, LOQ | Flow step `quant.curve` |
| export | `export.table_csv`, `export.mztab`, `export.annotations` (steps); `peak table --columns sn,fig,quality` |
| provenance | `flow record RUN --roles roles.json`, `flow verify`; `prov.json` is in every run |

A Flow step runs from the shell too: `mzlab-cli flow steps --id ID --template > plan.json`, edit, `flow check plan.json --json`, `flow run plan.json --in DIR --out RUN`.


For an agent running on the user's computer (Claude Code, Claude Cowork or others) and using mzLab without the page: it prepares an `mzflow/2` plan, checks it, runs it and reads the results with `mzlab-cli`. **Build `mzlab-cli` with `--profile cli`, not `release`** (`release` is size-optimised for the `.wasm`; `cli` is the speed profile, same output bytes, binary in `target/cli/`). Full command rules: `cli.md`; plans and `prov/1`: `rust-flow.md`; `step/1` steps: `rust-step.md`. Choices and open proposals: `llm-choices.md`.

## Installation
- Linux (x86_64, aarch64; static, any distro) and macOS on Apple silicon (arm64, unsigned): every published release carries a prebuilt `mzlab-cli` and `SHA256SUMS`. Install in one line, no sudo, checked against `SHA256SUMS` before it is copied: `curl -fsSL https://raw.githubusercontent.com/Freeesta/mzlab/main/tools/install.sh | sh` (writes `~/.local/bin/mzlab-cli`, prints its `--version`; `sh -s -- --dir DIR`, `--version X.Y.Z`, `--dry-run`; exit codes and manual fetch: `cli-agenti.md`, "Prebuilt binaries").
- Any other system (Windows, Intel Mac), no published release yet (exit 5), or a build of your own commit: from source with Rust, `cargo build --profile cli -p mzlab-cli` (binary `target/cli/mzlab-cli`; or `cargo install --path crates/mzlab-cli --profile cli` to put it in the PATH). Use the `cli` profile (`opt-level = 3`, about a quarter faster), not `release`, which is size-optimised for the `.wasm`; same output bytes. The same CLI compiles for `wasm32-wasip1` and runs a plan under node with `tools/wasi_run.mjs`.

## Recommended sequence
0. **Learn the CLI**: `mzlab-cli commands --find TASK WORDS --json` (the commands and steps for a task), `mzlab-cli commands --json` (every command, flag, output schema, exit code), `mzlab-cli schema NAME` (any JSON Schema), `mzlab-cli flow steps --compact` (the steps in a few KB).
1. **Steps and parameters**: `mzlab-cli flow steps --json [--id ID]` prints the `step/1` descriptors of the binary you will use, with `executable_native`; the same `step/1` are in `docs/schemi/steps-1.json`, `steps-attributio-1.json`, `steps-tabui-1.json`, `steps-cli-1.json` (one JSON line per step; `params` is a JSON Schema with `default`, `minimum`, `maximum`, `enum`). A step without a native executor (list `BUCHI`, see `cli-passi.md`) makes the plan be rejected with `err.flow.step.native`.
2. **Plan**: an `mzflow/2` file with `inputs` (file names) and `flow steps` (`id` of the instance, `step` = `id@version`, `in` = `file:NAME` or `instance.port`, optional `algorithm`, `preset`, `params` with only the differences from the default).
3. **Check without writing**: `mzlab-cli flow check PLAN.json [--set ...] --json` checks the plan without the inputs (`flowcheck/1`, code 3 with the problems); `mzlab-cli flow run PLAN.json --in FOLDER --dry-run` checks plan, `--set` and inputs and prints the inputs found and the instances in order with the canonical parameters; code 3 = invalid plan (one `instance.port:key` line per problem on stderr), 4 = missing input; with `--json` on stdout `flowdry/1`.
4. **Run**: `mzlab-cli flow run PLAN.json --in FOLDER --out RUN [--set INSTANCE.NAME=VALUE ...] [--workers N]`; progress on stderr, one line per job (`--progress jsonl`: one JSON line per job).
5. **Outcome**: with `--json` on stdout `flowrun/1` (status, times and SHA-256 of the files of each instance) or `error/1` (`error_key`, `params`, `hint`); exit code 0 all done, 1 some instance in error (the other outputs are written), 2 wrong usage, 3 invalid plan, 4 missing input or different SHA-256.
6. **Results**: in `RUN` one file per port (`<instance>.<ext>`), `plan.json` (canonical plan with the `--set`), `prov.json` (`prov/1`) and `report.json` (`run-report/2`: status, times and parameters of each instance); `mzlab-cli flow report RUN` rewrites the `run-report/2`.
7. **Fix**: rerun in the same `RUN` with other `--set`: the cache redoes only the steps that change (`cached` in `prov/1`); `--fresh` redoes everything.

## Six chains (tested)
Chains 1-5 follow; the sixth, a whole kinetic LR campaign from vendor files to the kinetics per condition, is `recipes.md` (test: `crates/mzlab-cli/src/workflow_kinetic_tests.rs`). Each chain is a test on synthetic data in `crates/mzlab-cli/src/workflow_tests.rs` or that file: every call exits 0 and the call count below is asserted. Add `--cache DIR` (or `MZLAB_CACHE`) to the file commands of a chain that reads the same files twice; `file index FILE...` builds the indexes first (`cli-agenti.md`).
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
1. `mzlab-cli flow steps --compact` (ids, ports, parameter names), then `mzlab-cli flow defaults features.detect --json` for the defaults, limits, units and `--set` keys of one step (a rule that is off when empty has `default: null`).
2. `mzlab-cli flow steps --id features.detect@2 --template > plan.json`: a valid one-step plan; add the steps and links you need (`"in":{"index":["idx.index"]}`) and change only the parameters you want.
3. `mzlab-cli flow check plan.json --json`: exit 3 names every problem with its `path`, e.g. `det.params.min_hight` (unknown parameter), `det.params.min_height` (wrong type), `det.in.index` (missing port), `steps[1].param` (a key the format does not have). `flow run plan.json --in DIR --dry-run` repeats the check with the inputs and prints what WOULD run; it never writes.

## Several ions from many full-scan files, then their peaks
`trace xic --mz M1,M2,..` answers every ion of the list in one call per file (one index load, the scans shared; `--sweep LO:HI:STEP` for a regular grid): `--json` (default) is a `trace-set/1`. `peak table --traces` reads those files, one sample per file and one channel per ion, and `--auto` integrates every peak of every ion in every file in ONE call (tested on the demo `FullScan_t*.mzML`, `traces_in_tests.rs`):
```sh
for f in t0 t5 t10; do mzlab-cli trace xic FullScan_$f.mzML --mz 364.4,194.2 --da 0.3 --out xic_$f.json; done
mzlab-cli peak table --traces xic_t0.json --traces xic_t5.json --traces xic_t10.json --auto > peaks.json   # {"windows","log","groups","unmatched"}
```
`peak auto-lr` is one trace (two CSV columns); never call it once per ion or file. SIM/SRM files: `peak table FILE... --auto` reads their channels directly; a target list with S/N: step `chrom.transitions`. At unit resolution give the window in Da (`--da 0.3`), in ppm for high resolution. After a plan run, `mzlab-cli flow report RUN` writes the `run-report/2` of the run.

## Rules for the agent
- The CLI uses no network and reads and writes only in `--in` and `--out`; do not delete user files: the run folder is the only place to write.
- At unit resolution an m/z is a candidate, not an identification; the identification steps require `hr` data.
- Every run is reproducible: same plan and same inputs give the same bytes with 1 or N `--workers`.
