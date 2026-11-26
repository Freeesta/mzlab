Read docs/coda/BRIEF.md.

# CLICK-HR-GUARD: guard and evidence for the HR click

**Goal.** One click on a high-resolution trace (`autoAll`, `auto_integrate` in `peakq_auto.rs`) reports points, area, noise and S/N per peak and refuses junk windows with the same reasons, force and logging as the low-resolution click (`peakq_click.rs`, `refused`, `forced`, `params`). The S/N the guard shows is the S/N of the manual-integration table (`manint_fig`), or the difference is stated in the evidence line.

**Why.** The low-resolution click is guarded; the HR click still integrates every peak of the stretch without a verdict. The guard computes S/N as height over the edge line divided by the noise sigma of the raw trace, which can differ from the table value of the same window.

**Read first.** `docs/agenti/data-integration.md` ("Click guard", "HR method choice"), `crates/mzlab-core/src/peakq_click.rs`, `crates/mzlab-core/src/peakq_auto.rs`, `mzlab/web/integra-clic.js` (`go`), `mzlab/web/picchi.js` (`autoAll`).

**Done means.** Rust engine route with a wasm export and a CLI route (`--json` stable keys, distinct exit codes), a byte-for-byte parity test that fails, not skips, when the wasm leg is missing (pattern `tests/test_flowtab_union_wasm.py`); `lang/` keys for the messages; e2e on a synthetic HR file. Raw bytes of the module touched, before, after and %, in the PR body.

**Out of scope.** Low-resolution click (done); manual drag windows.

**Follow-ups.** Closing step of the brief.
