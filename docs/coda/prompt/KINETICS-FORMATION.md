Read docs/coda/BRIEF.md.

# KINETICS-FORMATION: Formation-decay fit of a product in `quant.kinetics`

**Goal.** An opt-in parameter of `quant.kinetics` that fits a product ion with the consecutive model (formed from the parent, then decaying) from `mzlab_core::kinetics::fit_consecutive`, taking the parent's first-order k from the same document (or a given value), and writes k2, its interval and the choice in `fits` and `choices`.

**Why.** The first version fits only first-order decay. A transformation product rises and falls; its decay constant is the number the campaign wants.

**Read first.** `docs/agenti/kinetics-series.md`, `crates/mzlab-core/src/kinetics.rs` (`fit_consecutive`), `crates/mzlab-ops/src/run/table/kinetics.rs`, `crates/mzlab-cli/src/flow_kinetics.rs`, `docs/agenti/wasm-caps.md` (`flowkin`: its cap is the measurement x 1.05).

**Already on main (checked 28 Nov, re-check before coding).** `quant.kinetics@1` (decay only); `fit_consecutive` exists in the core and is not called by the step.

**Done means.** 1. The parameter and the output keys in the schema; default bytes unchanged. 2. A synthetic parent and product series with known k1 and k2 (like the Fe and Cu series of `kinetics_tests.rs`). 3. Site = app = CLI byte parity, the wasm size of `flowkin` before and after in the PR body.

**Out of scope.** Second-order models; plots (`KINETICS-VIEW`).

**Follow-ups.** Closing step of the brief: delete your row and this file; delete your id from the `needs first` cells; name each concrete follow-up in the PR body and add its row and prompt.
