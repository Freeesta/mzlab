Read docs/coda/BRIEF.md.

# ION-COUNTS-TRACE: ion-scale keys for the trace and peak outputs

**Goal.** Give `trace xic` and the peak outputs (`peak noise`, `peak table`, `quant targets`) the same opt-in both scales as `raw xic --ions`: the factor of `mzlab_core::ionscale::factor` (method value, else 1 / ion quantum) and ion-scale columns or keys next to the cps ones. Existing bytes unchanged without the opt-in.

**Why.** Poisson S/N only makes sense in ions, and these outputs are where users meet S/N first. One factor function keeps every command on the same scale.

**Read first.** `crates/mzlab-core/src/ionscale.rs`, `crates/mzlab-core/src/ionstat.rs` (`OneIon::Cps`, `FloorParams`), `docs/agenti/raw-access.md` (the `--ions` lines), `docs/agenti/cli-agenti.md`. Start with `grep -n 'peak noise\|quant targets\|trace xic' docs/agenti/INDICE.md`, then `grep -rn 'one_ion' crates/mzlab-core/src -l`.

**Already on main (checked 25 Nov, re-check before coding).** `raw xic --ions` with `ion_scale`, `ions_*`, `sn_ions`, `cps_net`. `ionstat` takes a one-ion cps (`OneIon::Cps`) from the user.

**Done means.** 1. An opt-in flag per command (name it like `--ions`), its keys in the schema, `--json` stable. 2. The factor and its evidence in the output, the choice logged. 3. Tests with synthetic files of known factor for each scan type. 4. Parity site=app=CLI byte for byte (the wasm leg fails, not skips, in CI), agent catalogs and `related.rs` words updated.

**Out of scope.** Reading the method automatically (row ION-COUNTS-METHOD); re-deriving the quantum.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
