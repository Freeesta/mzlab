Read docs/coda/BRIEF.md.

# NOMINAL-BIN-HR: `spectrum get --bin` on a high-resolution profile file

**Goal.** `spectrum get FILE --bin nominal|0.1 [--mz-offset ...]` answers on a high-resolution profile file: the bins, or one clear refusal that names why. Today the answer is `err.scan.none`, which says nothing about binning.

**Why.** The route that builds the profile window returns nothing for the single-precision index section of HR profile files, so the new flag fails with a message that points at the wrong cause.

**Read first.** `docs/agenti/data-channels.md` (the `--bin` bullet), `crates/mzlab-core/src/nominalbin.rs`, `spectrum.rs`, `profspec.rs`, `hrroutes.rs`, `crates/mzlab-cli/src/spectrum_bins.rs`. Start with `grep -n 'profile hr\|profile_hr' docs/agenti/INDICE.md`, then `grep -rn 'err.spectrum.rebin.hr' crates --include=*.rs -l`.

**Already on main (checked 25 Nov, re-check before coding).** `rebin=` and `mzoff=` of `/api/spectrum` bin unit-resolution profile data; high-resolution centroid data is refused with `err.spectrum.rebin.hr` (exit 1). An HR profile file does not reach that check: its window is empty first.

**Done means.** 1. Decide with evidence whether HR profile bins make sense (width 0.1 yes, nominal rarely) and implement them or refuse with `err.spectrum.rebin.hr`. 2. A synthetic HR profile file (`tools/dati_sintetici.py`) in a CLI test. 3. Site, app and CLI answer the same bytes (extend `crates/mzlab-desktop/tests/parity_nominal_bin.rs`).

**Out of scope.** Unit-resolution data (done); `--minus` and `--sum` (row NOMINAL-BIN-MINUS).

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
