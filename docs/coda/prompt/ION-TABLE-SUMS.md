Read docs/coda/BRIEF.md.

# ION-TABLE-SUMS: the sum row of `peak table --ions`

**Goal.** Fill the ion cells of the `--sums` row of `peak table --ions`: the area and height in ions of the rows that share one factor, with the noise and S/N left empty where they do not add.

**Why.** The sum row pads the ion columns with empty cells today (`manint_csv.rs`, `Extra::width`) because the rows of one sum may come from traces with different factors, so a plain sum would mix scales.

**Read first.** `crates/mzlab-core/src/manint_csv.rs` (the sum row), `crates/mzlab-core/src/manint_ions.rs`, `docs/agenti/raw-access.md` (the `peak table --ions` lines).

**Done means.** 1. The ion cells of the sum row hold the sum of the rows with one factor and stay empty, with the reason in the log, when the factors differ. 2. Bytes unchanged without `--ions`. 3. Tests with synthetic traces of two factors. 4. Parity site (.wasm) = app = CLI, the wasm leg failing, not skipping, in CI.

**Out of scope.** New columns; `rotte` growth.

**Follow-ups.** Closing step of the brief.
