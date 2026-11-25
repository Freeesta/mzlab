Read docs/coda/BRIEF.md.

# DAD-DELAY-APPLY: apply the PDA/UV detector delay at conversion, opt-in

**Goal.** Apply the PDA/UV detector delay at conversion, opt-in.

**Why.** `dad delay` only estimates the delay between the UV trace and the MS; the converted file keeps both clocks as acquired.

**Read first.** `docs/agenti/dad.md`, `crates/mzlab-cli/src/dad_cmd.rs`, the converter docs (`grep -n 'convert' docs/agenti/INDICE.md`). Start with `grep -n 'quant' docs/agenti/INDICE.md`, then only that area file.

**Done means.** The function in the site (`.wasm` + OPFS), the app and the CLI with a byte-for-byte parity test (the wasm leg fails, not skips, in CI), `--json` with stable keys, the evidence printed and logged, a test that the entry point is found (`commands --find`). Opt-in: no existing output byte changes. Synthetic data only.

**Out of scope.** A delay applied without a switch; any change to default conversion output.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
