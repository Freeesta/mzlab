Read docs/coda/BRIEF.md.

# ION-COUNTS-METHOD: read the ion-scale method values from the file

**Goal.** `raw xic --ions` takes the method values of the cps-to-ions factor (`fill_ms`, `dwell_ms`, `scan_s`, `step_da`, `mass_lo`, `mass_hi`) only as `--set` keys or route parameters. Read them from the file's own method (`method/1`) so the factor needs no typing, with the same bytes in the CLI, the app and the site.

**Why.** A typed number can be wrong or forgotten, and then the data estimate silently stands in. The method is the evidence the factor is meant to come from.

**Read first.** `crates/mzlab-core/src/ionscale.rs` (`Facts`, `factor`), `crates/mzlab-core/src/rawxic.rs` (`ions_of`), `docs/agenti/raw-access.md` (the `--ions` lines), `docs/agenti/method.md` (`ms.periods[].experiments[]`, `parameters`, `transitions[].dwell_ms`). Start with `grep -n 'ionscale\|raw xic' docs/agenti/INDICE.md`.

**Already on main (checked 25 Nov, re-check before coding).** The factor function, the `ion_scale` member, the `ions_*` row keys and the `--ions` flag. Only the dwell of an MRM or SIM transition is a named field of `method/1` today; no label of the fill time of a trap scan or of the scan time, step and range of a Q1 scan is known to the reader.

**Done means.** 1. One function `Facts::from_method(&Value, kind)` in core, tested on method documents for each scan type. 2. The CLI reads the method of each file when no value is given, logs each value and the experiment it came from in `provenance.choices`, and a value given with `--set` wins. 3. The site passes the same values as route parameters (the page already reads `/api/metodo`); the app likewise. 4. Parity CLI=app=wasm on a synthetic file with a method in its `<run>` (as `method_raw_parity.rs`).

**Ask the coordinator** for the format facts only: where the fill time and the Q1 scan time, step size and range sit in the method of each vendor format. Do not guess a label.

**Out of scope.** Changing the quantum estimator; real instrument constants.

**Follow-ups.** Closing step of the brief: delete your row and this file; name each concrete follow-up in the PR body and add its row and prompt.
