Read docs/coda/BRIEF.md.

# MS2-PEAK-HR: `spectrum get --peak auto` for profile and high-resolution levels

**Goal.** `spectrum get --peak auto` makes the background-subtracted spectrum also for a profile level and a high-resolution centroid level, where it now answers `window_only` (apex, window and sides, no spectrum).

**Why.** Only unit-resolution centroid levels get a spectrum, because the subtraction runs per nominal mass. A profile or HR product-ion scan has the same apex and sides problem, and an agent that gets `window_only` still has to subtract by hand.

**Read first.** `crates/mzlab-core/src/specpeak.rs` (`nominal_sum`, the `has_spectrum` branch), `crates/mzlab-core/src/specdiff.rs` (`subtract_hr` by ppm, `mean_window` for the profile and HR cells), `docs/agenti/data-channels.md` (the `--peak auto` entry), `crates/mzlab-core/src/specpeak_tests.rs`. Start with `grep -n 'window_only' crates/mzlab-core/src/specpeak.rs`.

**Done means.** 1. At a profile or HR level `--peak auto` gives the window sum minus the sides' mean, with the cell and the tolerance named in `evidence.peak` (`--set` for the ppm, default printed); lines are never merged across the tolerance without saying so. 2. The verdict stays `peak`; `window_only` remains only for a request the engine cannot answer, with its reason. 3. Synthetic test on a profile file and an HR centroid file (`tools/dati_sintetici.py`: `full_scan_profile`, the HR files of `crates/mzlab-core/tests`): a known apex, a flat background, the net line kept and the flat line gone. Parity site=app=CLI (`tests/test_spec_peak_wasm.py`, `route_steps.rs`); size of the lazy `rawxic` `.wasm` in the PR body (`docs/agenti/wasm-caps.md`).

**Out of scope.** Comparing two spectra (row SPECTRUM-COMPARE); a page control for `--peak auto`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
