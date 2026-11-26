Read docs/coda/BRIEF.md.

# MS2-PEAK-HR-NOISE: a net-intensity floor for `spectrum get --peak auto` at a high-resolution level

**Goal.** `spectrum get --peak auto` at a profile or high-resolution level keeps the lines of the peak and not the random noise lines of the window, with a floor the user can see and set.

**Why.** At unit resolution the nominal-mass bins add the noise of the window and of the sides on the same bin, so it cancels. At high resolution a noise line of the window has no line within the tolerance in the sides, so its whole sum is kept: the synthetic Orbitrap file of `tools/dati_sintetici.py` (`hr_dda`) gives about 1,600 lines for one parent peak, of which a handful are the ion and its isotopes. The agent then has to filter by hand.

**Read first.** `crates/mzlab-core/src/specpeak.rs` (`net_spectrum`, `at`, `Cells`), `docs/agenti/data-channels.md` (the `--peak auto` entry), `crates/mzlab-core/src/specpeak_hr_tests.rs`. Start with `grep -n 'net_spectrum' crates/mzlab-core/src/specpeak.rs`.

**Done means.** 1. A `--set` parameter (for example `min_net`, in counts per scan of the window) drops the lines under it at a profile or high-resolution level; its default and the number of lines dropped are in `evidence.peak`. 2. The default is chosen from the data (for example a multiple of the sides' noise level) and logged, or is zero with a stated reason; the unit-resolution bytes do not change without the parameter. 3. Synthetic test: a known apex over random noise lines, the ion lines kept and the noise gone. Parity site=app=CLI (`tests/test_spec_peak_wasm.py`, `route_steps.rs`); size of the lazy `specpeak` `.wasm` in the PR body (`docs/agenti/wasm-caps.md`).

**Out of scope.** Comparing two spectra (row SPECTRUM-COMPARE); a page control for `--peak auto`.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
