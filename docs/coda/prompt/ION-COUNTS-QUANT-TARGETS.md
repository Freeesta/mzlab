Read docs/coda/BRIEF.md.

# ION-COUNTS-QUANT-TARGETS: ion-scale columns for `quant targets`

**Goal.** Give `quant targets` and the Flow step `quant.targets` the opt-in ion scale: area, height and S/N in ions per target and file next to the cps ones, with the factor of each file.

**Why.** Poisson S/N only makes sense in ions, and `trace xic --ions`, `peak noise --ions` and `raw xic --ions` already give both scales from one factor (`mzlab_core::ionscale::factor`, method value, else 1 / ion quantum). A target list over many files and polarities has one factor per file; one call must keep it in one place.

**Read first.** `crates/mzlab-core/src/ionscale.rs`, `crates/mzlab-core/src/ionstat_ions.rs`, `crates/mzlab-core/src/rawxic_trace.rs`, `docs/agenti/raw-access.md` (the `--ions` lines). Start with `grep -n 'quant targets' docs/agenti/INDICE.md`, then `grep -n 'one_ion' crates/mzlab-core/src -rl`.

**Already on main (re-check before coding).** `raw xic --ions`, `trace xic --ions` (route `/api/xicions`, lazy `.wasm` `rawxic` export `xic_ions`), `peak noise --ions` (wasm `noise_floor` takes the ion values in `params`), `ionscale::choice_json` (the log text of the choice), `Routes::ion_scale_of`, `measure_ions`.

**Done means.** 1. An opt-in flag `--ions` (with `--scan-type` and the method values as `--set`), the keys or columns next to the cps ones, bytes unchanged without it, the schema updated. 2. The factor of each file and its evidence in the output, the choice logged. 3. Tests with synthetic files of known factor for each scan type (ems, epi, q1, sim, other). 4. Parity site(.wasm)=app=CLI byte for byte, the wasm leg failing, not skipping, in CI; agent catalogs and `related.rs` words updated. A Flow step parameter (`ions`, default off) needs its entry in the step catalog, `flow steps --compact` and the schemas, regenerated with their tools.

**Out of scope.** Reading more method labels than `ionfacts` knows (row ION-COUNTS-LABELS); re-deriving the quantum; `rotte` growth (use a lazy module).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
