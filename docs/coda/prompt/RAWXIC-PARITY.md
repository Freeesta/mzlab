Read docs/coda/BRIEF.md.

# RAWXIC-PARITY: raw xic: the remaining equality and parity tests

**Goal.** Add the equality tests deferred in #3712: `raw xic` equals `trace xic | peak auto-lr` bytes; area, height and S/N equal to `quant targets` on the wasm leg; and the app-leg `--sample` parity step.

**Why.** Today wasm = CLI and CLI = `quant targets` are tested separately; equality on the wasm leg is only transitive.

**Read first.** `tests/test_raw_xic_wasm.py`, `crates/mzlab-cli/tests/raw_xic_cli.rs`, `crates/mzlab-desktop/tests/parity_routes/route_steps.rs`, `docs/agenti/raw-access.md`. Start with `grep -n 'raw xic\|raw-access' docs/agenti/INDICE.md`, then `grep -n 'def test' tests/test_raw_xic_wasm.py`.

**Already on main (checked 25 Nov, re-check before coding).** The app route has no sample parameter because a multi-sample file reaches it as one file per sample (stated in #3712): check whether that limit is real; if so, state it in the PR body and drop that item.

**Done means.** Tests that fail in CI when tools are missing; no product code unless a test finds a real difference (then fix the cause).

**Out of scope.** New `raw` options.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
