Read docs/coda/BRIEF.md.

# ION-COUNTS-LABELS: read the fill time and the Q1 scan values from `method/1`

**Goal.** Extend `mzlab_core::ionfacts` (`Facts::from_method`, one line per label in its table) so that `raw xic --ions` reads the fill time of an EMS/EPI scan (`fill_ms`) and the scan time, step and range of a Q1 scan (`scan_s`, `step_da`, `mass_lo`, `mass_hi`) from the file's own `method/1`, as it already reads the dwell of a transition.

**Why.** Only `transitions[].dwell_ms` is a named field of `method/1` today. For a trap or Q1 scan the factor still needs a typed `--set` value, and a forgotten one silently falls back to the data estimate.

**Read first.** `crates/mzlab-core/src/ionfacts.rs`, `ionscale.rs` (`ScanKind::needs`), `docs/agenti/method.md` (`.wiff` experiment fields; "Fill time and Q1 resolution are NOT decoded yet"), `filemethod.rs` (`fill_time_ms` is `unknown` for `sciex-wiff`), `docs/agenti/raw-access.md` (the `--ions` lines).

**Blocked on facts.** Ask the owner for FORMAT facts only, no names or data: for each vendor format, where the fill time and the Q1 scan time, step size and mass range sit in the method (stream, label, unit), and in which `method/1` field a reader would put them. Do not guess a label and do not read mzML `ion_injection_time` as a fill time before the owner confirms it means that.

**Done means.** 1. Each confirmed label read in `from_method` with a test on a method document per scan type and an experiment named in `Read::from`. 2. The decoder that writes the field into `method/1` (and the schema) if the reader lacks it, with its parity test. 3. CLI, app and site unchanged in shape: they already pass `ionfacts` values; the parity test of `parity_ionfacts.rs` and `tests/test_ion_facts_wasm.py` gains a case per scan type. 4. `docs/agenti/method.md` and `raw-access.md` updated.

**Out of scope.** Changing the quantum estimator; real instrument constants in this repo.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
