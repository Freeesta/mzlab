Read docs/coda/BRIEF.md.

# RT-REF-OTHER-STEPS: an internal-reference RT mode for peak, trace xic and the features alignment

**Goal.** An internal-reference RT mode for peak, trace xic and the features alignment.

**Why.** `quant.targets` can follow an internal reference (`rt_ref`, evidence per file); the other steps still use fixed windows or the whole-run alignment.

**Read first.** `docs/agenti/quant-targets.md`, `crates/mzlab-feat/src/quant_targets/rtref.rs`, `crates/mzlab-feat/src/rtwarp.rs`. Start with `grep -n 'quant' docs/agenti/INDICE.md`, then only that area file.

**Done means.** The function in the site (`.wasm` + OPFS), the app and the CLI with a byte-for-byte parity test (the wasm leg fails, not skips, in CI), `--json` with stable keys, the evidence printed and logged, a test that the entry point is found (`commands --find`). Opt-in: no existing output byte changes. Synthetic data only.

**Out of scope.** Changing `quant.targets` (edge, ambiguous and blank policy stay as they are).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
