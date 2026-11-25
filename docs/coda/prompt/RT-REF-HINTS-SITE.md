Read docs/coda/BRIEF.md.

# RT-REF-HINTS-SITE: show the quant.targets hints on the site

**Goal.** Show the quant.targets hints on the site.

**Why.** The CLI prints a hint on stderr (and in `--json`) when the data ask for `--rt-ref auto` or `--offset auto`; the site shows none because the `quanttargets` wasm is at +4.89 % of its base (limit 5 %).

**Read first.** `docs/agenti/quant-targets.md` (Hints), `crates/mzlab-feat/src/quant_targets/rtref.rs` (`hints`, `cfg(not(target_arch = "wasm32"))`), `docs/agenti/wasm-caps.md`. Start with `grep -n 'quant' docs/agenti/INDICE.md`, then only that area file.

**Done means.** The function in the site (`.wasm` + OPFS), the app and the CLI with a byte-for-byte parity test (the wasm leg fails, not skips, in CI), `--json` with stable keys, the evidence printed and logged, a test that the entry point is found (`commands --find`). Opt-in: no existing output byte changes. Synthetic data only.

**Out of scope.** Changing the hint wording or the CLI hints.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
