# First-order rate of a time series (`quant.kinetics@1`)
Files: `crates/mzlab-ops/src/run/table/kinetics.rs` (joins, UV factors, fits, choices, the `kinetics-series/1` text), `kinetics_read.rs` (parameters and the two CSVs), `crates/mzlab-cli/src/flow_kinetics.rs` (step descriptor and files), `crates/mzlab-flowkin-wasm/src/lib.rs` (`flow_kinetics` in the browser), `flow/lazy/flow-web-quant.js` (site executor). The fit is `mzlab_core::kinetics::fit_decay` (first-order decay, AICc, residual bootstrap); nothing of it is duplicated.

## What it does
One step for the three manual jobs of a kinetic campaign: the areas of fixed RT windows for an ion list, the injection check on a UV reference peak, and k with its uncertainty.
- **areas** (CSV; `file` or `sample`, `id` or `analyte`/`label`/`name`, `area`, optional `state`; `absent` = no peak): the table of `quant.targets` (fixed RT windows per ion) or any table of that shape.
- **samples** (`samples/1`): role in `kind` (`sample` or `t0`), series in `group`, time in `time_min`. A `t0` file joins every series at its time (0 when it has none, logged); `use_t0=false` turns that off.
- **uv** (optional, `chrom.uv_peaks`): the UV peak of a reference, one per file. Factor of a file = its UV area / the median UV area; `uv_mode=normalise` divides the areas by it, `check` reports the factors and leaves the areas alone. No `uv` wired = no division.
- **UV caveat**: the reference must not react. In a photolysis series the parent decays, so its UV peak is not an injection check; use an internal reference or the parent of a dark control (`uv_rt` picks the apex). A reference that itself trends (|Spearman rho| of its UV area against time >= 0.8, reported as `uv.time_spearman`) gives `warn.flow.kinetics.uvTrend`.

## Output
`kinetics-series/1` JSON, keys in this order:
- `params`: what was used. `uv`: wired, mode, reference RT and tolerances, reference area, `time_spearman`.
- `files`: file, kind, series, `time_min`, `uv_rt_min`, `uv_area`, `factor`, `uv_off`, `used`, `reason`.
- `points`: every ion and series point with `area`, `factor`, normalised `value`, `state`, `used`, `reason`.
- `fits`: `k`, `k_ci`, `half_life`, `A`, `r2`, `n`, `times`, `spearman_time`, `reliable`, `note`.
- `choices`: every automatic decision with its reason. `warnings`: keys `warn.flow.kinetics.*` (`fewPoints`, `forced`, `missing`, `noDecay`, `notFitted`, `unmatched`, `uvMissing`, `uvOff`, `uvTrend`).

A file whose factor is off by more than `uv_tol` % is flagged and warned, never dropped. Errors: `err.flow.kinetics.{areas,uv,row,param,samples}`, each with its line.

## Where it runs
- CLI/app: `quant.kinetics` in `EXECUTORS` of `flow.rs` (through `flow_curve.rs`), descriptor in `docs/schemi/steps-cli-1.json` (regenerated with `MZLAB_UPDATE_STEPS=1 cargo test -p mzlab-cli --lib flow_ms2`); parameters with `--set INSTANCE.NAME=VALUE`.
- Site: `flow_kinetics` of the `flowkin` module (bridge `flowkin/1`, `wasm-caps.md`; a module of its own because inside `flowcal` it grew that module +24 %), op `flow_kinetics` of `feat-worker.js`, executor `F.webQuant.kinetics` (`flow-live-jobs.js`).
- Tests: `kinetics_tests.rs` (synthetic series with known k: Fe 0.04, Cu 0.015, shared t0, injection factors), `flow_kinetics_tests.rs` (13 synthetic files through `quant.targets` and `quant.kinetics`, k within 3 %, golden file `crates/mzlab-cli/tests/data/kinetics/golden.json`, `MZLAB_UPDATE_GOLDEN=1` rewrites it), `tests/test_flowkin_wasm.py` (wasm in Node = `flow run`, five parameter groups), `tests_e2e/e2e_flow_web_quant.py` (site = native, `kn` step on the committed series).
