Read docs/coda/BRIEF.md.

# MZ-OFFSET-POLARITY: a per-polarity --mz-offset spec for trace xic and features

**Goal.** A per-polarity --mz-offset spec for trace xic and features.

**Why.** `quant targets --offset auto` measures one offset per file and polarity; `trace xic` and `features --mz-offset auto:MZ` measure one on the polarity in use, so a run with both polarities needs two calls.

**Read first.** `docs/agenti/mzfeat-lr.md`, `crates/mzlab-cli/src/mz_offset.rs`, `docs/agenti/quant-targets.md`. Start with `grep -n 'quant' docs/agenti/INDICE.md`, then only that area file.

**Done means.** The function in the site (`.wasm` + OPFS), the app and the CLI with a byte-for-byte parity test (the wasm leg fails, not skips, in CI), `--json` with stable keys, the evidence printed and logged, a test that the entry point is found (`commands --find`). Opt-in: no existing output byte changes. Synthetic data only.

**Out of scope.** Changing the offset estimator (mzcal).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
