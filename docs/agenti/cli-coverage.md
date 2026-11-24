# CLI coverage: step registry and native executor

Moved from `cli.md` ("Coverage"): which registries exist, which steps `FileRunner` runs natively and how their parameters are set. The command list and the rules for new commands stay in `cli.md`; the line per step is in `cli-passi.md`.

- Registries: `mzlab_feat::steps::registry()` (13, `steps-1.json`), `attributio()` (10, `steps-attributio-1.json`), `mzlab_chem::step::tabui()` (9, `steps-tabui-1.json`), steps of the crate (`features.table`, `spectra.centroid`, `spectra.ms2`, `spectra.msn_graph`, `meta.compare`, `quant.calibrate`, `quant.curve`, all in `steps-cli-1.json`). `mzlab_cli::flow::steps()` merges them all; the Flow page reads them.
- Run by `FileRunner` (`EXECUTORS` of `flow.rs`): `convert.mzml`, the BASE (`index.mzidx`, `features.detect`, `features.align`, `features.gapfill`: `flow_base.rs`), `features.estimate`, `features.table`, `table.drift`, `features.group`, `table.kmd`, `table.plausible`, `spectra.ms2` (DIA: `dia.md`), `spectra.msn_graph`, `spectra.lookup`, `spectra.match`, `formula.ms1`, `formula.ms2`, `annot.level` (rule of `levels.js`), `tp.generate`, `tp.link`, `tp.filter`, `tp.position`, `tp.network` (`flow_network.rs`), `chrom.uv_peaks`, `meta.compare`, `quant.calibrate`/`.curve`, `export.inclusion`.
  Also `chrom.transitions` (`flow_transitions.rs`, `transitions.md`) and `structure.moiety` (`flow_moiety.rs`), with their own `EXECUTORS`.
  A step without an executor is refused before starting (`err.flow.step.native`, also in the desktop's `flow/run`).
- Step by step: `cli-passi.md` (a line per registry step with a native executor).
- Parameters: every parameter of the exported steps (plus `features.table` and `meta.compare`, not exported), also of the algorithms, is set with `--set`. `features.table` uses the keys of the Pipeline panel (`ppm`, `rt_tol`, which `apply_params` splits into two fields) and has no `fill`.
